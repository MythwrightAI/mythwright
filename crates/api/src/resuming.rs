//! A daemon answers while it takes up what it was doing.
//!
//! A daemon that starts again takes up what the one before it left: the
//! services and commands that were running, each instance and its runs,
//! the agents and the links to other devices. That took a fifth to half a
//! second at a hundred cards in flight, and longer with more, and nothing
//! was answered meanwhile. So it listens first ([`crate::Node::start`]),
//! and while it takes up:
//!
//! - What is read is answered, from what is written. What reads a
//!   conductor that is taking up what it keeps waits for that conductor,
//!   as it would for one started again by itself.
//! - What changes anything waits until it is done, and is then answered
//!   as it would have been had it come a moment later: any write may be
//!   heard by a handler of an instance not yet in hand, and a daemon that
//!   took up before answering anybody is what every play of a restart
//!   was played against. Waited for rather than refused, since a client
//!   that is refused has to ask again and one that waits a moment does
//!   not; and a request sent just before is answered the same way.
//! - `/api/meta` and `/api/fleet` say that it is taking up and how far
//!   ([`Resuming::said`]), which is what `status` prints.
//! - `system.started` is emitted once it is done, before anything that
//!   waited is let through.
//!
//! A start that fails part way answers what waited with why, and stops.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use axum::Json;
use axum::extract::{Request, State};
use axum::http::{Method, StatusCode, header::UPGRADE};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use conductor::Engine;
use serde_json::{Value, json};

use crate::state::App;

/// What a daemon taking up is doing now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Doing {
    /// Not yet begun: it was started a moment ago.
    #[default]
    Beginning,
    /// Finding the services and commands that were running.
    Services,
    /// Taking up each instance and its runs.
    Instances,
    /// Starting the agents and the links to other devices.
    Agents,
}

/// How far a daemon has come in taking up what it was doing, for as long
/// as it is.
#[derive(Default)]
pub struct Resuming {
    doing: Mutex<Doing>,
    /// Once it is over: what it came to, an error where it failed.
    over: Mutex<Option<Result<(), String>>>,
    ended: AtomicBool,
    told: tokio::sync::Notify,
    /// Where a test holds the start, told of each request held.
    held: OnceLock<Arc<Held>>,
}

impl Resuming {
    /// Has a test that holds the start told of each request held.
    pub(crate) fn tells(&self, held: Arc<Held>) {
        let _ = self.held.set(held);
    }

    /// Says what it is doing now.
    pub(crate) fn now(&self, doing: Doing) {
        *self.doing.lock().expect("resuming lock") = doing;
    }

    /// Says it is over, as it came out, and lets through what waited.
    pub(crate) fn ends(&self, came: Result<(), String>) {
        *self.over.lock().expect("resuming lock") = Some(came);
        self.ended.store(true, Ordering::SeqCst);
        self.told.notify_waiters();
    }

    /// Whether it is over.
    pub fn is_over(&self) -> bool {
        self.ended.load(Ordering::SeqCst)
    }

    /// Waits until it is over, and says how it came out.
    pub async fn over(&self) -> Result<(), String> {
        loop {
            // Listening before looking, so that its ending in between is heard.
            let told = self.told.notified();
            tokio::pin!(told);
            told.as_mut().enable();
            if let Some(came) = self.over.lock().expect("resuming lock").clone() {
                return came;
            }
            told.await;
        }
    }

    /// What is said of it: nothing once it is over, and otherwise what it
    /// is doing, how far it has come and a sentence that says so.
    pub fn said(&self, engine: &Engine) -> Value {
        if self.is_over() {
            return Value::Null;
        }
        let doing = *self.doing.lock().expect("resuming lock");
        let far = engine.how_far();
        let (what, sentence) = match doing {
            Doing::Beginning => ("beginning", "beginning".to_string()),
            Doing::Services => {
                ("services", "finding the services and commands that were running".to_string())
            }
            Doing::Instances => (
                "instances",
                format!(
                    "taking up instances: {} so far, from {} of {} journals",
                    far.instances, far.up, far.conductors
                ),
            ),
            Doing::Agents => ("agents", "starting its agents and its links to other devices".to_string()),
        };
        json!({
            "doing": what,
            "instances": far.instances,
            "journals": far.conductors,
            "journals_done": far.up,
            "says": format!(
                "It is still taking up what it was doing when it stopped ({sentence}). What is read is \
                 answered now; what changes anything waits until it is done."
            ),
        })
    }
}

/// Whether a request may change something: anything but a read, and a
/// read that would become something else.
fn changes(request: &Request) -> bool {
    !matches!(*request.method(), Method::GET | Method::HEAD | Method::OPTIONS)
        || request.headers().contains_key(UPGRADE)
}

/// Holds what may change anything until the daemon has taken up what it
/// was doing; lets everything else through at once.
pub async fn holds(State(app): State<App>, request: Request, next: Next) -> Response {
    if app.resuming.is_over() || !changes(&request) {
        return next.run(request).await;
    }
    if let Some(held) = app.resuming.held.get() {
        held.waiting.add_permits(1);
    }
    match app.resuming.over().await {
        Ok(()) => next.run(request).await,
        Err(why) => {
            let message = format!(
                "the daemon could not take up what it was doing when it stopped, and is stopping: {why}. \
                 Its own terminal, or its log, says the rest; `{} up` starts it again.",
                brand::CLI
            );
            let body = json!({ "error": { "kind": "unavailable", "message": message } });
            (StatusCode::SERVICE_UNAVAILABLE, Json(body)).into_response()
        }
    }
}

/// A start held as it is about to take up what it was doing, already
/// answering, for a test to ask it things meanwhile and then let it go
/// on. Never said for a factory that is run.
pub struct Held {
    at: Mutex<Option<SocketAddr>>,
    reached: tokio::sync::Notify,
    go: tokio::sync::Notify,
    /// A permit for each request held until the start is over.
    waiting: tokio::sync::Semaphore,
}

impl Default for Held {
    fn default() -> Self {
        Held {
            at: Mutex::default(),
            reached: tokio::sync::Notify::new(),
            go: tokio::sync::Notify::new(),
            waiting: tokio::sync::Semaphore::new(0),
        }
    }
}

impl Held {
    /// Waits until another request is held until the start is over.
    pub async fn holds_one(&self) {
        self.waiting.acquire().await.expect("never closed").forget();
    }

    /// Waits until a start has reached the place and answers there: the
    /// address it answers at.
    pub async fn reached(&self) -> SocketAddr {
        self.reached.notified().await;
        self.at.lock().expect("held lock").expect("said where it answers before it said it reached the place")
    }

    /// Lets the start go on.
    pub fn go(&self) {
        self.go.notify_one();
    }

    /// For the start: says it is here, at `addr`, and waits to be let go.
    pub(crate) async fn here(&self, addr: SocketAddr) {
        *self.at.lock().expect("held lock") = Some(addr);
        self.reached.notify_one();
        self.go.notified().await;
    }
}
