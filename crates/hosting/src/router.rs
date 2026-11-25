//! The router: one stable address in front of changing backends.
//!
//! A router listens on a port and forwards each connection to one of its
//! backends, chosen by weight. Changing the weights is instant and does not
//! disturb connections already open, which is the whole mechanism behind a
//! canary rollout: start the new version beside the old, send it a tenth of
//! the traffic, watch, send it more.
//!
//! It works at the level of TCP connections, so it is indifferent to what is
//! spoken over them. Every connection is counted and timed per backend; a
//! rollout decides on those numbers, and so can anything else.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use conductor::ActionError;
use kernel::{Cancel, Source};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::net::{TcpListener, TcpStream};

use crate::Hosting;

/// How long a backend has to accept a connection.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
/// How long a router that is coming back after a restart waits for its own
/// address to be free again.
const TAKEOVER_PATIENCE: Duration = Duration::from_secs(3);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Backend {
    /// A service on this device, by name; or
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    /// any `host:port`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// Share of new connections, relative to the other backends. Zero means
    /// none: the backend keeps what it has and gets nothing new.
    pub weight: u32,
}

impl Backend {
    /// The name the backend is counted under.
    pub fn key(&self) -> String {
        self.service.clone().or_else(|| self.address.clone()).unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Stats {
    pub connections: u64,
    /// Connections the backend refused or did not accept in time.
    pub failures: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct RouterInfo {
    pub name: String,
    pub instance: String,
    /// The project that instance was applied for, when it was for one.
    pub project: Option<String>,
    pub host: String,
    pub listen: u16,
    pub backends: Vec<BackendInfo>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BackendInfo {
    pub target: String,
    pub weight: u32,
    pub connections: u64,
    pub failures: u64,
}

pub(crate) struct Router {
    pub name: String,
    pub instance: String,
    pub project: Option<String>,
    pub host: String,
    pub listen: u16,
    table: Mutex<Table>,
    pub stop: Cancel,
}

#[derive(Default)]
struct Table {
    backends: Vec<Backend>,
    /// Smooth weighted round-robin state, one per backend.
    credit: Vec<i64>,
    stats: HashMap<String, Stats>,
}

impl Router {
    pub fn info(&self) -> RouterInfo {
        let table = self.table.lock().expect("router lock");
        RouterInfo {
            name: self.name.clone(),
            instance: self.instance.clone(),
            project: self.project.clone(),
            host: self.host.clone(),
            listen: self.listen,
            backends: table
                .backends
                .iter()
                .map(|backend| {
                    let stats = table.stats.get(&backend.key()).copied().unwrap_or_default();
                    BackendInfo {
                        target: backend.key(),
                        weight: backend.weight,
                        connections: stats.connections,
                        failures: stats.failures,
                    }
                })
                .collect(),
        }
    }

    pub fn stats(&self, key: &str) -> Stats {
        self.table.lock().expect("router lock").stats.get(key).copied().unwrap_or_default()
    }

    pub fn backends(&self) -> Vec<Backend> {
        self.table.lock().expect("router lock").backends.clone()
    }

    fn set_backends(&self, backends: Vec<Backend>) {
        let mut table = self.table.lock().expect("router lock");
        table.credit = vec![0; backends.len()];
        table.backends = backends;
    }

    /// The next backend by smooth weighted round-robin, skipping `tried`.
    /// Smooth, so a 1-in-10 backend gets every tenth connection rather than
    /// a burst of them.
    fn pick(&self, tried: &[usize]) -> Option<(usize, Backend)> {
        let mut table = self.table.lock().expect("router lock");
        let table = &mut *table;
        let mut total = 0i64;
        let mut best: Option<usize> = None;
        for (index, backend) in table.backends.iter().enumerate() {
            if backend.weight == 0 || tried.contains(&index) {
                continue;
            }
            table.credit[index] += i64::from(backend.weight);
            total += i64::from(backend.weight);
            if best.is_none_or(|best| table.credit[index] > table.credit[best]) {
                best = Some(index);
            }
        }
        let best = best?;
        table.credit[best] -= total;
        Some((best, table.backends[best].clone()))
    }

    fn count(&self, key: &str, connected: bool) {
        let mut table = self.table.lock().expect("router lock");
        let stats = table.stats.entry(key.to_string()).or_default();
        stats.connections += 1;
        stats.failures += u64::from(!connected);
    }
}

impl Hosting {
    pub(crate) fn router(&self, name: &str) -> Option<Arc<Router>> {
        self.inner.routers.lock().expect("hosting lock").get(name).cloned()
    }

    /// Every router on this device.
    pub fn routers(&self) -> Vec<RouterInfo> {
        let mut routers: Vec<RouterInfo> =
            self.inner.routers.lock().expect("hosting lock").values().map(|router| router.info()).collect();
        routers.sort_by(|a, b| a.name.cmp(&b.name));
        routers
    }

    async fn save_router(&self, router: &Router) {
        let row = (
            router.name.clone(),
            router.instance.clone(),
            router.host.clone(),
            router.listen,
            serde_json::to_string(&router.backends()).unwrap_or_default(),
            self.inner.journal.now_ms(),
            router.project.clone(),
        );
        let _ = self
            .inner
            .journal
            .db()
            .call(move |connection| {
                connection.execute(
                    "INSERT OR REPLACE INTO routers (name, instance, host, listen, backends, updated, project)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![row.0, row.1, row.2, row.3, row.4, row.5, row.6],
                )
            })
            .await;
    }

    /// Makes sure a router called `name` is listening. `listen` of zero picks
    /// a free port.
    pub(crate) async fn ensure_router(
        &self,
        name: &str,
        owner: (&str, Option<&str>),
        host: &str,
        listen: u16,
        backends: Vec<Backend>,
    ) -> Result<Arc<Router>, ActionError> {
        self.router_listening(name, owner, host, listen, backends, Duration::ZERO).await
    }

    /// The same, waiting up to `patience` for an address that is in use.
    async fn router_listening(
        &self,
        name: &str,
        (instance, project): (&str, Option<&str>),
        host: &str,
        listen: u16,
        backends: Vec<Backend>,
        patience: Duration,
    ) -> Result<Arc<Router>, ActionError> {
        if let Some(existing) = self.router(name) {
            if listen != 0 && existing.listen != listen {
                return Err(ActionError::new(format!(
                    "router `{name}` is already listening on port {}; remove it to move it to {listen}",
                    existing.listen
                )));
            }
            return Ok(existing);
        }
        let began = Instant::now();
        let mut pause = Duration::from_millis(5);
        let listener = loop {
            match TcpListener::bind((host, listen)).await {
                Ok(listener) => break listener,
                Err(err) if err.kind() == std::io::ErrorKind::AddrInUse && began.elapsed() < patience => {
                    tokio::time::sleep(pause).await;
                    pause = (pause * 2).min(Duration::from_millis(200));
                }
                Err(err) => {
                    return Err(ActionError::new(format!(
                        "router `{name}` cannot listen on {host}:{listen}: {err}"
                    )));
                }
            }
        };
        let listen = listener.local_addr().map(|address| address.port()).unwrap_or(listen);
        let router = Arc::new(Router {
            name: name.to_string(),
            instance: instance.to_string(),
            project: project.map(str::to_string),
            host: host.to_string(),
            listen,
            table: Mutex::default(),
            stop: Cancel::new(),
        });
        router.set_backends(backends);
        self.inner.routers.lock().expect("hosting lock").insert(name.to_string(), router.clone());
        self.save_router(&router).await;

        let (hosting, accepting) = (self.clone(), router.clone());
        let task = tokio::spawn(async move {
            loop {
                let inbound = tokio::select! {
                    accepted = listener.accept() => accepted,
                    () = accepting.stop.cancelled() => return,
                };
                // A failed accept (out of descriptors, say) is not a reason
                // to stop listening.
                let Ok((inbound, _)) = inbound else { continue };
                tokio::spawn(hosting.clone().forward(accepting.clone(), inbound));
            }
        });
        self.inner.tasks.lock().expect("hosting lock").push(task.abort_handle());
        let listening = json!({ "router": name, "host": host, "port": listen });
        let source = Source::instance(instance);
        let _ = self.said_of(&router.project).emit("router.listening", source, listening).await;
        Ok(router)
    }

    /// Changes where a router sends new connections.
    pub(crate) async fn route(&self, name: &str, backends: Vec<Backend>) -> Result<Arc<Router>, ActionError> {
        let router = self
            .router(name)
            .ok_or_else(|| ActionError::new(format!("there is no router called `{name}`")))?;
        for backend in &backends {
            match (&backend.service, &backend.address) {
                (Some(service), None) if self.service(service).is_none() => {
                    return Err(ActionError::new(format!(
                        "there is no service called `{service}` to route to"
                    )));
                }
                (Some(_), None) | (None, Some(_)) => {}
                _ => {
                    return Err(ActionError::new("each backend needs exactly one of `service` or `address`"));
                }
            }
        }
        router.set_backends(backends.clone());
        self.save_router(&router).await;
        let routed = json!({ "router": name, "backends": backends });
        let source = Source::instance(&router.instance);
        let _ = self.said_of(&router.project).emit("router.routed", source, routed).await;
        Ok(router)
    }

    pub(crate) async fn remove_router(&self, name: &str) -> bool {
        let Some(router) = self.inner.routers.lock().expect("hosting lock").remove(name) else {
            return false;
        };
        router.stop.cancel();
        let name = name.to_string();
        let _ = self
            .inner
            .journal
            .db()
            .call(move |connection| connection.execute("DELETE FROM routers WHERE name = ?1", [&name]))
            .await;
        true
    }

    fn address_of(&self, backend: &Backend) -> Option<String> {
        match (&backend.service, &backend.address) {
            (Some(name), _) => {
                let service = self.service(name)?;
                let up = service.state.borrow().status.is_up();
                up.then(|| format!("127.0.0.1:{}", service.port))
            }
            (None, Some(address)) => Some(address.clone()),
            (None, None) => None,
        }
    }

    /// Carries one connection. If the chosen backend will not take it, one
    /// other is tried, so a canary that is down costs the client nothing.
    async fn forward(self, router: Arc<Router>, mut inbound: TcpStream) {
        let started = Instant::now();
        let mut tried = Vec::new();
        while tried.len() < 2 {
            let Some((index, backend)) = router.pick(&tried) else { return };
            tried.push(index);
            let key = backend.key();
            let of = router.project.as_deref().unwrap_or("");
            let labels = [("router", router.name.as_str()), ("backend", key.as_str()), ("project", of)];
            let outbound = match self.address_of(&backend) {
                Some(address) => {
                    tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(address)).await.ok()
                }
                None => None,
            };
            let Some(Ok(mut outbound)) = outbound else {
                router.count(&key, false);
                self.inner.journal.measure(
                    "router.connection",
                    &labels,
                    started.elapsed().as_secs_f64() * 1000.0,
                    false,
                );
                continue;
            };
            router.count(&key, true);
            let carried = tokio::io::copy_bidirectional(&mut inbound, &mut outbound).await;
            let took = started.elapsed().as_secs_f64() * 1000.0;
            self.inner.journal.measure("router.connection", &labels, took, carried.is_ok());
            return;
        }
    }

    /// Brings back the routers that were listening when the conductor last
    /// stopped.
    pub(crate) async fn resume_routers(&self) -> kernel::Result<usize> {
        type Row = (String, String, String, u16, String, Option<String>);
        let rows: Vec<Row> = self
            .inner
            .journal
            .db()
            .call(|connection| {
                let mut statement = connection
                    .prepare("SELECT name, instance, host, listen, backends, project FROM routers")?;
                let rows = statement.query_map([], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get::<_, i64>(3)? as u16,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                })?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await?;
        let mut restored = 0;
        for (name, instance, host, listen, backends, project) in rows {
            let backends: Vec<Backend> = serde_json::from_str(&backends).unwrap_or_default();
            // The address was this router's a moment ago. Whatever had it (the
            // daemon before this one, or a process that daemon was in the
            // middle of starting, which holds a copy of everything its
            // parent had open until it is properly under way) lets go within
            // moments, so it is worth a short wait rather than a failure.
            let owner = (instance.as_str(), project.as_deref());
            match self.router_listening(&name, owner, &host, listen, backends, TAKEOVER_PATIENCE).await {
                Ok(_) => restored += 1,
                Err(err) => {
                    let data = json!({ "router": name, "error": err.message });
                    let _ =
                        self.said_of(&project).emit("router.failed", Source::instance(&instance), data).await;
                }
            }
        }
        Ok(restored)
    }
}
