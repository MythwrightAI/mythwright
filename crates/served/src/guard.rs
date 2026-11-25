//! Whether a request is one this daemon answers at all, decided before any
//! route is looked at.
//!
//! A daemon that listens on this machine can be reached by more than the
//! people it is for. A page somebody merely visits can make their browser
//! ask it things, and has two ways of being believed. It can give its own
//! name an address of this machine (the name it was loaded under then leads
//! here, and the browser takes what it finds here for that page's own), and
//! it can ask from where it is, with the session a browser keeps for this
//! machine going along because the browser keeps sessions by machine and
//! not by port. Neither needs a credential to be stolen.
//!
//! So two things are held here, for every request, whoever it says it is:
//!
//! - **It was addressed to this daemon.** What it was asked at (`Host`) is
//!   an address, `localhost`, or a name the daemon has been told is its own
//!   ([`Names`]). A name that merely leads here is refused.
//! - **If it changes something, it does not come from somewhere else.** A
//!   browser says where a request that changes something comes from
//!   (`Origin`, `Sec-Fetch-Site`). One that says it comes from another
//!   origin is refused; one that says nothing of where it comes from (the
//!   command line, an agent, a script) is as it always was: it has a
//!   credential or it has not.

use std::collections::BTreeSet;
use std::net::IpAddr;
use std::sync::{Arc, RwLock};

use axum::Json;
use axum::extract::{Request, State};
use axum::http::header::{HOST, ORIGIN};
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionResult, Registry};
use journal::Journal;
use kernel::{BoxFut, Error, Principal, Result, Source};
use serde_json::{Value, json};

/// Where the names a daemon has been told are its own are kept.
const NAMESPACE: &str = "names";

/// The names this daemon answers to besides the addresses it listens on.
///
/// Something put in front of the daemon (a tunnel, a proxy, another name
/// for this machine on a network of one's own) asks it under a name of its
/// choosing. The daemon does not know that name until somebody who may
/// says it: which is the point, since a name nobody vouched for is what a
/// page that has pointed its own name here would be using.
pub struct Names {
    journal: Journal,
    told: RwLock<BTreeSet<String>>,
}

/// A name as it is compared: without a port, without the dot a full name
/// may end in, in small letters.
fn plain(name: &str) -> String {
    name.trim().trim_end_matches('.').to_ascii_lowercase()
}

/// The machine a `Host` (or the like) names, without its port: `[::1]:7700`
/// is `::1`, `workshop:7700` is `workshop`.
pub fn machine(host: &str) -> String {
    let host = host.trim();
    if let Some(rest) = host.strip_prefix('[') {
        return rest.split(']').next().unwrap_or_default().to_ascii_lowercase();
    }
    match host.rsplit_once(':') {
        Some((name, port)) if !port.is_empty() && port.bytes().all(|byte| byte.is_ascii_digit()) => {
            plain(name)
        }
        _ => plain(host),
    }
}

impl Names {
    pub async fn open(journal: Journal) -> Names {
        let told = journal.kv_list(NAMESPACE).await.unwrap_or_default().into_iter().map(|(name, _)| name);
        Names { told: RwLock::new(told.collect()), journal }
    }

    /// The names it has been told, in order.
    pub fn told(&self) -> Vec<String> {
        self.told.read().expect("names lock").iter().cloned().collect()
    }

    /// Whether a request asked at `host` was addressed to this daemon.
    pub fn answers(&self, host: &str) -> bool {
        let machine = machine(host);
        if machine.parse::<IpAddr>().is_ok() {
            // Asked at an address, it was reached at that address: by a
            // route of this machine's, a forwarded port, a tunnel. No page
            // can make its own name an address; it is names that are made
            // to lead here.
            return true;
        }
        machine == "localhost"
            || machine.ends_with(".localhost")
            || self.told.read().expect("names lock").contains(&machine)
    }

    /// Tells the daemon a name it is asked under. The owner's to say.
    pub async fn tell(&self, name: &str, by: &Principal) -> Result<String> {
        by.require("admin")?;
        let name = machine(name);
        let fits = !name.is_empty()
            && name.len() <= 253
            && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'.')
            && name.parse::<IpAddr>().is_err();
        if !fits {
            return Err(Error::invalid(
                "a name the daemon answers to is a host's name, such as `box.example.net`: letters, digits, \
                 dashes and dots, with no port and no address before it",
            ));
        }
        self.journal
            .kv_set(NAMESPACE, &name, &json!({ "by": by.actor(), "at": self.journal.now_ms() }))
            .await?;
        let new = self.told.write().expect("names lock").insert(name.clone());
        if new {
            let _ =
                self.journal.emit("system.named", Source::actor(by.actor()), json!({ "name": name })).await;
        }
        Ok(name)
    }

    /// Takes a name back: the daemon no longer answers when asked under it.
    pub async fn forget(&self, name: &str, by: &Principal) -> Result<String> {
        by.require("admin")?;
        let name = machine(name);
        if !self.told.write().expect("names lock").remove(&name) {
            return Err(Error::not_found(format!("`{name}` is not a name this daemon was told")));
        }
        self.journal.kv_set(NAMESPACE, &name, &serde_json::Value::Null).await?;
        let _ = self.journal.emit("system.unnamed", Source::actor(by.actor()), json!({ "name": name })).await;
        Ok(name)
    }
}

fn refused(message: String) -> Response {
    let body = json!({ "error": { "kind": "denied", "message": message } });
    (StatusCode::FORBIDDEN, Json(body)).into_response()
}

/// Refuses a request that was not addressed to this daemon, or that changes
/// something and says it comes from another origin; lets every other
/// through to be judged by its credential.
pub async fn guard(State(names): State<Arc<Names>>, request: Request, next: Next) -> Response {
    if let Some(why) = turned_away(&names, &request) {
        return refused(why);
    }
    next.run(request).await
}

/// Why a request is not answered, when it is not.
fn turned_away(names: &Names, request: &Request) -> Option<String> {
    let header = |name: &str| request.headers().get(name).and_then(|value| value.to_str().ok());
    let host = header(HOST.as_str()).unwrap_or_default();
    if !names.answers(host) {
        let asked = if host.is_empty() { "with no name".to_string() } else { format!("as `{host}`") };
        return Some(format!(
            "this daemon was asked {asked}, which is not an address it listens on or a name it has been \
             told is its own, so it does not answer. If that name is how this device is reached (through a \
             tunnel, or by its name on a network of your own), tell the daemon so on its own machine: \
             `{cli} names add {name}`",
            cli = brand::CLI,
            name = if host.is_empty() { "<name>".to_string() } else { machine(host) },
        ));
    }
    if matches!(*request.method(), Method::GET | Method::HEAD | Method::OPTIONS) {
        return None;
    }
    let elsewhere = match (header(ORIGIN.as_str()), header("sec-fetch-site")) {
        (_, Some(site)) if !matches!(site, "same-origin" | "none") => Some(site.to_string()),
        (Some(origin), _) => {
            let from = origin.split_once("://").map_or(origin, |(_, rest)| rest);
            (!from.eq_ignore_ascii_case(host)).then(|| origin.to_string())
        }
        _ => None,
    };
    elsewhere.map(|from| {
        format!(
            "this request changes something and says it comes from elsewhere (`{from}`), not from this \
             daemon's own pages, so it is refused whatever credential came with it"
        )
    })
}

/// The action by which a blueprint tells the daemon a name it is asked
/// under: what puts the daemon behind another address says so as it does.
pub(crate) fn register(registry: &mut Registry, names: Arc<Names>) {
    let spec = ActionSpec::new(
        "system.answer_as",
        "Tell the daemon a name it is asked under, so that it answers when it is: the name of whatever was put in front of it.",
    )
    .required("name", T::String, "the name, such as `box.example.net`, with no port")
    // A name the daemon answers to is one it takes a browser's word under.
    .capability("admin")
    .output("name", "the name, as it is kept")
    .idempotent();
    registry.add_fn(spec, move |cx, args| -> BoxFut<'static, ActionResult> {
        let names = names.clone();
        Box::pin(async move {
            let name = args.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
            Ok(json!({ "name": names.tell(&name, cx.principal()).await? }))
        })
    });
}
