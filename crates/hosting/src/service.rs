//! Long-lived services.
//!
//! A service is a process the fleet keeps running: started from a blueprint,
//! watched for health, restarted when it dies (within a budget, so a crash
//! loop stops itself), and stopped when the instance that owns it stops.
//! Services survive the conductor: on the way back up they are found again
//! by pid and start time rather than started twice.
//!
//! Everything a service does is an event (`service.started`, `.healthy`,
//! `.unhealthy`, `.exited`, `.restarting`, `.stopped`, `.failed`), so a
//! blueprint can react to a service's life with an ordinary `on` handler,
//! and every health check is a measurement.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use blueprint::duration;
use conductor::ActionError;
use kernel::{Cancel, Fields, Source};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::watch;

use crate::Hosting;
use crate::ports::{self, Spoken};
use crate::proc::{Launch, Proc, Tail};
use crate::said::Said;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServiceSpec {
    pub run: Option<String>,
    pub cmd: Vec<String>,
    /// Arguments for this program's own command line, in place of `run` or
    /// `cmd`. Where the program is, is found each time the service is
    /// started and never kept: it may have been moved or replaced since.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cli: Option<Vec<String>>,
    pub env: BTreeMap<String, String>,
    /// The port the service listens on; `0` means "pick a free one".
    pub port: u16,
    /// The environment variable the port is passed in.
    pub port_env: String,
    pub health: Option<Health>,
    pub restart: Restart,
    /// Restarts allowed within an hour before the service is given up on.
    pub max_restarts: u32,
    /// The wait before the first restart; it doubles each time, up to 30s.
    pub restart_backoff_ms: u64,
    pub stop_grace_ms: u64,
    /// The labels the step that started it carries: what it measures of
    /// its own is kept with them ([`crate::said`]).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,
    /// The secrets it is handed, each by the variable it is handed in: the
    /// names, read again every time it is started ([`crate::secrets`]).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub secrets: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Health {
    pub probe: Probe,
    pub interval_ms: u64,
    pub timeout_ms: u64,
    /// Consecutive failures before a healthy service is called unhealthy.
    pub unhealthy_after: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Probe {
    /// The port accepts a connection.
    Tcp,
    /// An HTTP GET of this path answers with a 2xx or 3xx.
    Http(String),
    /// This command exits zero. It is given the port in the port variable.
    Cmd(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Restart {
    Always,
    OnFailure,
    Never,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Starting,
    Healthy,
    Unhealthy,
    /// Ended on its own and was not restarted.
    Exited,
    Stopped,
    /// Given up on after restarting too often.
    Failed,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Starting => "starting",
            Status::Healthy => "healthy",
            Status::Unhealthy => "unhealthy",
            Status::Exited => "exited",
            Status::Stopped => "stopped",
            Status::Failed => "failed",
        }
    }

    fn parse(text: &str) -> Status {
        match text {
            "starting" => Status::Starting,
            "healthy" => Status::Healthy,
            "unhealthy" => Status::Unhealthy,
            "exited" => Status::Exited,
            "stopped" => Status::Stopped,
            _ => Status::Failed,
        }
    }

    /// Whether the service has a process that should be running.
    pub fn is_up(self) -> bool {
        matches!(self, Status::Starting | Status::Healthy | Status::Unhealthy)
    }
}

/// A service as the API and the client see it.
#[derive(Clone, Debug, Serialize)]
pub struct ServiceInfo {
    pub name: String,
    /// The instance that owns it.
    pub instance: String,
    /// The project that instance was applied for, when it was for one.
    pub project: Option<String>,
    pub status: Status,
    pub pid: u32,
    pub port: u16,
    /// Times it has been restarted since it was started.
    pub restarts: u32,
    pub last_exit: Option<i32>,
    /// Why it is in its current status, when that needs saying.
    pub reason: Option<String>,
    /// When it entered its current status.
    pub since: i64,
    pub command: String,
    /// The file its output is written to.
    pub log: String,
}

#[derive(Clone, Debug)]
pub(crate) struct State {
    pub status: Status,
    pub pid: u32,
    pub restarts: u32,
    /// Times its process has ended under this conductor. One whose process
    /// has ended and is yet to be started again is still what it was
    /// (`status`) and has not restarted (`restarts`): this is what moves
    /// when it ends, for whoever watches it (`router.observe`).
    pub exits: u32,
    pub last_exit: Option<i32>,
    pub reason: Option<String>,
    pub since: i64,
}

pub(crate) struct Service {
    pub name: String,
    pub instance: String,
    pub project: Option<String>,
    pub spec: ServiceSpec,
    pub cwd: PathBuf,
    pub port: u16,
    pub state: watch::Sender<State>,
    pub stop: Cancel,
    /// Set once nobody looks after it any more, and so once the last
    /// thing said of it has been said: whoever stops it waits for this,
    /// since its status says it is stopped before that is said.
    pub over: Cancel,
    /// Where its output is written.
    pub log: String,
    /// Its port spoken for, where it was given one and for as long as
    /// this factory looks after it ([`ports`]): its program takes the
    /// port a while after it is given, and again each time it is started
    /// again.
    spoken: Mutex<Option<Spoken>>,
}

impl Service {
    /// The port it was given, where this factory speaks for it.
    pub fn spoken_for(&self) -> Option<u16> {
        self.spoken.lock().expect("service lock").as_ref().map(Spoken::port)
    }

    /// Speaks for the port it was given, where nothing here does: for a
    /// service found again, which whoever started it spoke for until it
    /// went. One that named its port is its owner's to keep free.
    fn speak(&self) {
        let mut spoken = self.spoken.lock().expect("service lock");
        if self.spec.port == 0 && spoken.is_none() {
            *spoken = ports::speak_for(self.port);
        }
    }

    /// It is looked after here no longer: its port is somebody's to give.
    pub fn let_go(&self) {
        self.spoken.lock().expect("service lock").take();
    }

    pub fn info(&self) -> ServiceInfo {
        let state = self.state.borrow().clone();
        ServiceInfo {
            name: self.name.clone(),
            instance: self.instance.clone(),
            project: self.project.clone(),
            status: state.status,
            pid: state.pid,
            port: self.port,
            restarts: state.restarts,
            last_exit: state.last_exit,
            reason: state.reason,
            since: state.since,
            command: self.spec.said(),
            log: self.log.clone(),
        }
    }
}

pub(crate) const SCHEMA: &[&str] = &[
    "
    CREATE TABLE services (
        name TEXT PRIMARY KEY,
        instance TEXT NOT NULL,
        spec TEXT NOT NULL,
        cwd TEXT NOT NULL,
        port INTEGER NOT NULL,
        status TEXT NOT NULL,
        proc TEXT NOT NULL,
        restarts INTEGER NOT NULL DEFAULT 0,
        updated INTEGER NOT NULL
    ) WITHOUT ROWID;

    CREATE TABLE routers (
        name TEXT PRIMARY KEY,
        instance TEXT NOT NULL,
        host TEXT NOT NULL,
        listen INTEGER NOT NULL,
        backends TEXT NOT NULL,
        updated INTEGER NOT NULL
    ) WITHOUT ROWID;
",
    "
    ALTER TABLE services ADD COLUMN project TEXT;
    ALTER TABLE routers ADD COLUMN project TEXT;
",
];

impl ServiceSpec {
    /// Reads a spec as a blueprint writes it, with durations like `2s`.
    pub fn from_args(args: &Fields) -> Result<ServiceSpec, String> {
        let text = |key: &str| args.get(key).and_then(Value::as_str).map(str::to_string);
        let millis = |source: &Fields, key: &str, default: u64| -> Result<u64, String> {
            match source.get(key) {
                None | Some(Value::Null) => Ok(default),
                Some(Value::String(text)) => duration::parse(text).map_err(|err| format!("`{key}`: {err}")),
                Some(other) => Err(format!("`{key}` must be a duration such as `2s`, got {other}")),
            }
        };
        let list = |key: &str| -> Option<Vec<String>> {
            args.get(key)
                .and_then(Value::as_array)
                .map(|items| items.iter().map(blueprint::expr::display).collect())
        };
        let (cmd, cli) = (list("cmd").unwrap_or_default(), list("cli"));
        let run = text("run");
        match (run.is_some(), !cmd.is_empty(), cli.is_some()) {
            (true, false, false) | (false, true, false) | (false, false, true) => {}
            (false, false, false) => {
                return Err(
                    "a service needs `run: <script>`, `cmd: [program, args...]` or `cli: [args...]`".into()
                );
            }
            _ => return Err("give one of `run`, `cmd` and `cli`, not more".into()),
        }
        let health = match args.get("health") {
            None | Some(Value::Null) | Some(Value::Bool(false)) => None,
            Some(Value::Object(health)) => {
                let probe = match (health.get("http"), health.get("cmd"), health.get("tcp")) {
                    (Some(Value::String(path)), _, _) => Probe::Http(path.clone()),
                    (_, Some(Value::String(cmd)), _) => Probe::Cmd(cmd.clone()),
                    (_, _, Some(Value::Bool(true))) => Probe::Tcp,
                    _ => {
                        return Err(
                            "`health` needs one of `tcp: true`, `http: <path>` or `cmd: <command>`".into()
                        );
                    }
                };
                Some(Health {
                    probe,
                    interval_ms: millis(health, "interval", 2_000)?.max(100),
                    timeout_ms: millis(health, "timeout", 2_000)?,
                    unhealthy_after: health.get("unhealthy_after").and_then(Value::as_u64).unwrap_or(3)
                        as u32,
                })
            }
            Some(other) => return Err(format!("`health` must be a mapping, got {other}")),
        };
        let restart = match text("restart").as_deref() {
            None | Some("on-failure") => Restart::OnFailure,
            Some("always") => Restart::Always,
            Some("never") => Restart::Never,
            Some(other) => {
                return Err(format!("`restart` must be always, on-failure or never, got `{other}`"));
            }
        };
        Ok(ServiceSpec {
            run,
            cmd,
            cli,
            env: args
                .get("env")
                .and_then(Value::as_object)
                .map(|env| env.iter().map(|(k, v)| (k.clone(), blueprint::expr::display(v))).collect())
                .unwrap_or_default(),
            port: args.get("port").and_then(Value::as_u64).unwrap_or(0) as u16,
            port_env: text("port_env").unwrap_or_else(|| "PORT".into()),
            health,
            restart,
            max_restarts: args.get("max_restarts").and_then(Value::as_u64).unwrap_or(6) as u32,
            restart_backoff_ms: millis(args, "restart_backoff", 1_000)?,
            stop_grace_ms: millis(args, "stop_grace", 5_000)?,
            labels: BTreeMap::new(),
            secrets: BTreeMap::new(),
        })
    }
}

impl ServiceSpec {
    /// The command as it is shown to someone reading about the service.
    fn said(&self) -> String {
        match (&self.run, &self.cli) {
            (Some(run), _) => run.clone(),
            (None, Some(cli)) => format!("{} {}", brand::CLI, cli.join(" ")),
            (None, None) => self.cmd.join(" "),
        }
    }
}

impl Hosting {
    fn launch(
        &self,
        name: &str,
        spec: &ServiceSpec,
        cwd: &std::path::Path,
        port: u16,
        life: u32,
    ) -> kernel::Result<Launch> {
        let mut env = spec.env.clone();
        if port > 0 {
            env.insert(spec.port_env.clone(), port.to_string());
        }
        // Read now and kept nowhere: one that has been taken away since it
        // was last started is a start that fails, saying so.
        self.hand_over(&spec.secrets, &mut env)?;
        Ok(Launch {
            script: spec.run.clone(),
            // A program that cannot say where it is leaves nothing to
            // start, and the launch says so.
            argv: match &spec.cli {
                Some(cli) => crate::proc::own(cli).unwrap_or_default(),
                // A program that takes its port as an argument, or has to
                // be told what it is called here, is told in the argument:
                // there is no shell to read a variable for it.
                None => spec
                    .cmd
                    .iter()
                    .map(|argument| argument.replace("{port}", &port.to_string()).replace("{name}", name))
                    .collect(),
            },
            cwd: cwd.to_path_buf(),
            env,
            clear_env: false,
            address: self.address(),
            profile: self.profile(),
            dir: self.run_dir(),
            tag: format!("svc.{name}.{life}"),
            // What it writes is its log, read by whoever may read it: it
            // is written without what it was handed.
            handed: spec.secrets.clone(),
            wraps: self.wraps(),
        })
    }

    async fn save(&self, service: &Service, proc: &Value) {
        let state = service.state.borrow().clone();
        let row = (
            service.name.clone(),
            service.instance.clone(),
            serde_json::to_string(&service.spec).unwrap_or_default(),
            service.cwd.to_string_lossy().into_owned(),
            service.port,
            state.status.as_str(),
            proc.to_string(),
            state.restarts,
            self.inner.journal.now_ms(),
            service.project.clone(),
        );
        let _ = self
            .inner
            .journal
            .db()
            .call(move |connection| {
                connection.execute(
                    "INSERT OR REPLACE INTO services
                     (name, instance, spec, cwd, port, status, proc, restarts, updated, project)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![row.0, row.1, row.2, row.3, row.4, row.5, row.6, row.7, row.8, row.9],
                )
            })
            .await;
    }

    /// Starts a service, or returns the running one if it is the same.
    pub(crate) async fn start_service(
        &self,
        name: &str,
        instance: &str,
        project: Option<&str>,
        spec: ServiceSpec,
        cwd: PathBuf,
    ) -> Result<Arc<Service>, ActionError> {
        if let Some(existing) = self.service(name)
            && existing.state.borrow().status.is_up()
        {
            if existing.spec == spec {
                return Ok(existing);
            }
            return Err(ActionError::new(format!(
                "a different service called `{name}` is already running; stop it first or use another name"
            )));
        }
        if !cwd.is_dir() {
            return Err(ActionError::new(format!("the working directory {} does not exist", cwd.display())));
        }
        // One it is given is spoken for from here, before its program is
        // there to take it, until it is looked after no longer.
        let (port, spoken) = match spec.port {
            0 => {
                let spoken = ports::free().map_err(|err| ActionError::new(format!("no free port: {err}")))?;
                (spoken.port(), Some(spoken))
            }
            port => (port, None),
        };
        let proc = self.launch(name, &spec, &cwd, port, 0).and_then(|launch| launch.start())?;
        let state = State {
            status: Status::Starting,
            pid: proc.pid,
            restarts: 0,
            exits: 0,
            last_exit: None,
            reason: None,
            since: self.inner.journal.now_ms(),
        };
        let service = Arc::new(Service {
            name: name.to_string(),
            instance: instance.to_string(),
            project: project.map(str::to_string),
            spec,
            cwd,
            port,
            state: watch::Sender::new(state),
            stop: Cancel::new(),
            over: Cancel::new(),
            log: self.inner.logs.path("services", name).to_string_lossy().into_owned(),
            spoken: Mutex::new(spoken),
        });
        self.save(&service, &proc.saved()).await;
        self.inner.services.lock().expect("hosting lock").insert(name.to_string(), service.clone());
        self.supervise(service.clone(), proc, false);
        Ok(service)
    }

    pub(crate) fn service(&self, name: &str) -> Option<Arc<Service>> {
        self.inner.services.lock().expect("hosting lock").get(name).cloned()
    }

    /// The services that are handed the secret kept as `name` each time
    /// they are started, each with the variable it reads it from: what
    /// would not start again if the secret were taken away.
    pub fn services_handed(&self, name: &str) -> Vec<(ServiceInfo, String)> {
        let services = self.inner.services.lock().expect("hosting lock");
        let mut handed: Vec<(ServiceInfo, String)> = services
            .values()
            .flat_map(|service| {
                let named = service.spec.secrets.iter().filter(move |(_, kept)| kept.as_str() == name);
                named.map(|(variable, _)| (service.info(), variable.clone()))
            })
            .collect();
        handed.sort_by(|one, other| one.0.name.cmp(&other.0.name));
        handed
    }

    /// The ports this factory speaks for: one for each service it gave a
    /// port and still looks after ([`ports`]).
    pub fn spoken_for(&self) -> Vec<u16> {
        let services = self.inner.services.lock().expect("hosting lock");
        let mut ports: Vec<u16> = services.values().filter_map(|service| service.spoken_for()).collect();
        ports.sort_unstable();
        ports
    }

    /// Every service this device knows about.
    pub fn services(&self) -> Vec<ServiceInfo> {
        let mut services: Vec<ServiceInfo> = self
            .inner
            .services
            .lock()
            .expect("hosting lock")
            .values()
            .map(|service| service.info())
            .collect();
        services.sort_by(|a, b| a.name.cmp(&b.name));
        services
    }

    /// Stops a service and waits for it to be gone.
    pub async fn stop_service(&self, name: &str) -> bool {
        let Some(service) = self.service(name) else { return false };
        service.stop.cancel();
        let mut state = service.state.subscribe();
        // The supervisor is the one that stops the process; wait for it to
        // say so, and for it to have said the last it says of the service:
        // its status is written first, and whoever stopped it for an
        // instance goes on to say that the instance is over, which was
        // said before `service.stopped` where that came second.
        let gone = async {
            while state.borrow_and_update().status.is_up() {
                if state.changed().await.is_err() {
                    break;
                }
            }
            service.over.cancelled().await;
        };
        let limit = Duration::from_millis(service.spec.stop_grace_ms + 5_000);
        let _ = tokio::time::timeout(limit, gone).await;
        true
    }

    fn supervise(&self, service: Arc<Service>, proc: Proc, adopted: bool) {
        let hosting = self.clone();
        let task = tokio::spawn(async move {
            hosting.run_service(service.clone(), proc, adopted).await;
            // Stopped, ended for good or given up on: nothing starts its
            // program again, and its port is not kept from anybody.
            service.let_go();
            service.over.cancel();
        });
        self.inner.tasks.lock().expect("hosting lock").push(task.abort_handle());
    }

    async fn announce(&self, service: &Service, topic: &str, data: Value) {
        let mut data = data;
        data["service"] = json!(service.name);
        let _ = self.said_of(&service.project).emit(topic, Source::instance(&service.instance), data).await;
    }

    fn set(&self, service: &Service, change: impl FnOnce(&mut State)) {
        let now = self.inner.journal.now_ms();
        service.state.send_modify(|state| {
            let before = state.status;
            change(state);
            if state.status != before {
                state.since = now;
            }
        });
    }

    /// Takes what a line a service printed says to the factory, if it says
    /// anything ([`crate::said`]). A measurement is kept here and now; an
    /// event is handed back to be said; one that is wrong is answered where
    /// the service's output is read, as a line of the system's.
    fn take(&self, service: &Service, line: &str) -> Option<(String, Value)> {
        let journal = &self.inner.journal;
        match crate::said::said(line)? {
            Said::Measure { name, value, ok, labels } => {
                let of = service.project.as_deref().unwrap_or("");
                // Which service it is and what its step said of it come
                // first: a program does not get to say it is another.
                let mut all = vec![("service", service.name.as_str()), ("project", of)];
                let given = service.spec.labels.iter().chain(&labels);
                for (label, value) in given {
                    if !all.iter().any(|(taken, _)| taken == label) {
                        all.push((label.as_str(), value.as_str()));
                    }
                }
                journal.measure(&name, &all, value, ok);
                None
            }
            Said::Event { topic, data } => Some((topic, data)),
            Said::Wrong(why) => {
                let told = format!("not taken: {why}");
                self.inner.logs.append("services", &service.name, journal.now_ms(), 's', &told);
                None
            }
        }
    }

    /// Says the events a service said of its own, for the instance that
    /// owns it, in the order it printed them.
    async fn speak(&self, service: &Service, events: Vec<(String, Value)>) {
        let own = self.said_of(&service.project);
        for (topic, data) in events {
            // One that names a project is kept where that project's are,
            // as any event that names one is: asked of the factory's own.
            let named = data.get("project").and_then(Value::as_str).is_some_and(|named| !named.is_empty());
            let journal = if named { &self.inner.journal } else { &own };
            let _ = journal.emit(&topic, Source::instance(&service.instance), data).await;
        }
    }

    /// Looks after one service for as long as it should run.
    async fn run_service(&self, service: Arc<Service>, mut proc: Proc, adopted: bool) {
        let journal = &self.inner.journal;
        let said_in = self.said_of(&service.project);
        let mut restarts: Vec<Instant> = Vec::new();
        let mut first = !adopted;
        loop {
            if first {
                self.announce(&service, "service.started", json!({ "pid": proc.pid, "port": service.port }))
                    .await;
            }
            first = true;
            let mut tails = [Tail::new(&proc.out, 'o'), Tail::new(&proc.err, 'e')];
            // What it printed, passed on; and the events it said of its own
            // in doing so, for whoever reads to say ([`Hosting::speak`]).
            let relay = |tails: &mut [Tail; 2], flush: bool| {
                let mut events = Vec::new();
                for tail in tails.iter_mut() {
                    tail.read(flush, &mut |stream, line| {
                        self.inner.logs.append("services", &service.name, journal.now_ms(), stream, line);
                        let data = json!({ "service": service.name, "stream": stream, "text": line });
                        said_in.emit_ephemeral("log.line", Source::instance(&service.instance), data);
                        if let Some(event) = self.take(&service, line) {
                            events.push(event);
                        }
                    });
                }
                events
            };

            let mut failures = 0u32;
            let mut next_check = Instant::now();
            if service.spec.health.is_none() && service.state.borrow().status != Status::Healthy {
                self.set(&service, |state| state.status = Status::Healthy);
                self.save(&service, &proc.saved()).await;
                self.announce(&service, "service.healthy", json!({})).await;
            }

            // Told when the output grows, then one read for whatever was
            // written before asking: in that order nothing falls between.
            let mut output = self.inner.watch.files(&[&proc.out, &proc.err]).or_looking();
            self.speak(&service, relay(&mut tails, false)).await;

            let code = loop {
                // Nothing here wakes up to look. The process ending, its
                // output growing, its next health check coming due and being
                // told to stop are each something that happens; a service
                // that is doing nothing costs nothing between its checks.
                let check_due = async {
                    match &service.spec.health {
                        Some(_) => tokio::time::sleep_until(next_check.into()).await,
                        None => std::future::pending().await,
                    }
                };
                tokio::select! {
                    code = proc.wait() => break code,
                    () = output.next() => self.speak(&service, relay(&mut tails, false)).await,
                    () = check_due => {
                        // Also for output the system did not report.
                        self.speak(&service, relay(&mut tails, false)).await;
                        let Some(health) = service.spec.health.as_ref() else { continue };
                        next_check = Instant::now() + Duration::from_millis(health.interval_ms);
                        let began = Instant::now();
                        let result = probe(health, &service).await;
                        let took = began.elapsed().as_secs_f64() * 1000.0;
                        let of = service.project.as_deref().unwrap_or("");
                        let labels = [("service", service.name.as_str()), ("project", of)];
                        journal.measure("service.health", &labels, took, result.is_ok());
                        let was = service.state.borrow().status;
                        match result {
                            Ok(()) => {
                                failures = 0;
                                if was != Status::Healthy {
                                    self.set(&service, |state| {
                                        state.status = Status::Healthy;
                                        state.reason = None;
                                    });
                                    self.save(&service, &proc.saved()).await;
                                    self.announce(&service, "service.healthy", json!({})).await;
                                }
                            }
                            Err(reason) => {
                                failures += 1;
                                // A service that has never been healthy is still starting.
                                if was == Status::Healthy && failures >= health.unhealthy_after {
                                    self.set(&service, |state| {
                                        state.status = Status::Unhealthy;
                                        state.reason = Some(reason.clone());
                                    });
                                    self.save(&service, &proc.saved()).await;
                                    self.announce(&service, "service.unhealthy", json!({ "reason": reason })).await;
                                }
                            }
                        }
                    }
                    () = service.stop.cancelled() => {
                        proc.stop(Duration::from_millis(service.spec.stop_grace_ms)).await;
                        self.speak(&service, relay(&mut tails, true)).await;
                        proc.clean();
                        self.set(&service, |state| state.status = Status::Stopped);
                        self.save(&service, &Value::Null).await;
                        self.announce(&service, "service.stopped", json!({})).await;
                        return;
                    }
                }
            };
            self.speak(&service, relay(&mut tails, true)).await;
            proc.clean();
            let wanted = match service.spec.restart {
                Restart::Always => true,
                Restart::OnFailure => code != Some(0),
                Restart::Never => false,
            };
            // What it was is over with its process: one to be started again
            // is starting from here, and was never healthy while it waited,
            // which whoever waits for it to be healthy would have taken.
            self.set(&service, |state| {
                state.last_exit = code;
                state.exits += 1;
                state.status = if wanted { Status::Starting } else { Status::Exited };
            });
            self.announce(&service, "service.exited", json!({ "code": code })).await;
            if !wanted {
                self.save(&service, &Value::Null).await;
                return;
            }
            restarts.retain(|at| at.elapsed() < Duration::from_secs(3600));
            if restarts.len() as u32 >= service.spec.max_restarts {
                let reason = format!("restarted {} times within an hour; giving up", restarts.len());
                self.set(&service, |state| {
                    state.status = Status::Failed;
                    state.reason = Some(reason.clone());
                });
                self.save(&service, &Value::Null).await;
                self.announce(&service, "service.failed", json!({ "reason": reason })).await;
                return;
            }
            restarts.push(Instant::now());
            let doubled = service.spec.restart_backoff_ms << (restarts.len() - 1).min(6);
            let wait = Duration::from_millis(doubled.min(30_000));
            let attempt = service.state.borrow().restarts + 1;
            self.announce(
                &service,
                "service.restarting",
                json!({ "attempt": attempt, "in_ms": wait.as_millis() as u64 }),
            )
            .await;
            tokio::select! {
                () = tokio::time::sleep(wait) => {}
                () = service.stop.cancelled() => {
                    self.set(&service, |state| state.status = Status::Stopped);
                    self.save(&service, &Value::Null).await;
                    self.announce(&service, "service.stopped", json!({})).await;
                    return;
                }
            }
            // Found again by a conductor that could not speak for its
            // port just then, it is spoken for before its program is to
            // take the port once more.
            service.speak();
            let launched = self.launch(&service.name, &service.spec, &service.cwd, service.port, attempt);
            match launched.and_then(|launch| launch.start()) {
                Ok(next) => {
                    proc = next;
                    self.set(&service, |state| {
                        state.status = Status::Starting;
                        state.pid = proc.pid;
                        state.restarts = attempt;
                    });
                    self.save(&service, &proc.saved()).await;
                }
                Err(err) => {
                    self.set(&service, |state| {
                        state.status = Status::Failed;
                        state.reason = Some(err.message.clone());
                    });
                    self.save(&service, &Value::Null).await;
                    self.announce(&service, "service.failed", json!({ "reason": err.message })).await;
                    return;
                }
            }
        }
    }

    /// Finds the services that were running when the conductor last stopped.
    /// One whose process is still there is adopted as it is; one whose
    /// process is gone is treated as having exited.
    pub(crate) async fn resume_services(&self) -> kernel::Result<usize> {
        type Row = (String, String, String, String, u16, String, String, u32, Option<String>);
        let rows: Vec<Row> = self
            .inner
            .journal
            .db()
            .call(|connection| {
                let mut statement = connection.prepare(
                    "SELECT name, instance, spec, cwd, port, status, proc, restarts, project FROM services",
                )?;
                let rows = statement.query_map([], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get::<_, i64>(4)? as u16,
                        row.get(5)?,
                        row.get(6)?,
                        row.get::<_, i64>(7)? as u32,
                        row.get(8)?,
                    ))
                })?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await?;
        let mut adopted = 0;
        for (name, instance, spec, cwd, port, status, proc, restarts, project) in rows {
            let status = Status::parse(&status);
            let Ok(spec) = serde_json::from_str::<ServiceSpec>(&spec) else { continue };
            let saved: Value = serde_json::from_str(&proc).unwrap_or(Value::Null);
            let alive =
                Proc::adopt(&saved, &self.inner.watch).filter(|proc| status.is_up() && proc.is_alive());
            let state = State {
                status: if alive.is_some() {
                    status
                } else if status.is_up() {
                    Status::Exited
                } else {
                    status
                },
                pid: alive.as_ref().map_or(0, |proc| proc.pid),
                restarts,
                exits: 0,
                last_exit: None,
                reason: (alive.is_none() && status.is_up())
                    .then(|| "its process was gone when the conductor came back".into()),
                since: self.inner.journal.now_ms(),
            };
            let service = Arc::new(Service {
                name: name.clone(),
                instance,
                project,
                spec,
                cwd: PathBuf::from(cwd),
                port,
                state: watch::Sender::new(state),
                stop: Cancel::new(),
                over: Cancel::new(),
                log: self.inner.logs.path("services", &name).to_string_lossy().into_owned(),
                spoken: Mutex::default(),
            });
            self.inner.services.lock().expect("hosting lock").insert(name, service.clone());
            match alive {
                Some(proc) => {
                    adopted += 1;
                    // Whoever spoke for its port is gone, and its
                    // program may yet end and be started again.
                    service.speak();
                    self.supervise(service, proc, true);
                }
                None if status.is_up() => {
                    self.save(&service, &Value::Null).await;
                    self.announce(
                        &service,
                        "service.exited",
                        json!({ "code": null, "while": "the conductor was down" }),
                    )
                    .await;
                    service.over.cancel();
                }
                None => service.over.cancel(),
            }
        }
        Ok(adopted)
    }

    pub(crate) async fn forget_service(&self, name: &str) {
        self.inner.services.lock().expect("hosting lock").remove(name);
        let name = name.to_string();
        let _ = self
            .inner
            .journal
            .db()
            .call(move |connection| connection.execute("DELETE FROM services WHERE name = ?1", [&name]))
            .await;
    }
}

/// One health check.
async fn probe(health: &Health, service: &Service) -> Result<(), String> {
    let limit = Duration::from_millis(health.timeout_ms);
    let address = ("127.0.0.1", service.port);
    let check = async {
        match &health.probe {
            Probe::Tcp => {
                TcpStream::connect(address).await.map(|_| ()).map_err(|err| format!("connect: {err}"))
            }
            Probe::Http(path) => {
                let mut stream =
                    TcpStream::connect(address).await.map_err(|err| format!("connect: {err}"))?;
                let request = format!("GET {path} HTTP/1.0\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
                stream.write_all(request.as_bytes()).await.map_err(|err| format!("send: {err}"))?;
                let mut head = [0u8; 64];
                let read = stream.read(&mut head).await.map_err(|err| format!("read: {err}"))?;
                let line = String::from_utf8_lossy(&head[..read]);
                let code: u16 =
                    line.split_whitespace().nth(1).and_then(|code| code.parse().ok()).unwrap_or(0);
                if (200..400).contains(&code) { Ok(()) } else { Err(format!("GET {path} answered {code}")) }
            }
            Probe::Cmd(command) => {
                // The check is its user's own command, written for a shell.
                let status = tokio::process::Command::from(crate::script::command(command)?)
                    .env(&service.spec.port_env, service.port.to_string())
                    .current_dir(&service.cwd)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .kill_on_drop(true)
                    .status()
                    .await
                    .map_err(|err| format!("could not run the check: {err}"))?;
                if status.success() { Ok(()) } else { Err(format!("the check exited with {status}")) }
            }
        }
    };
    match tokio::time::timeout(limit, check).await {
        Ok(result) => result,
        Err(_) => Err(format!("no answer within {}", duration::format(health.timeout_ms))),
    }
}
