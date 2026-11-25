//! The member's side: keep a link to the hub open, do what it asks, and say
//! how it went.
//!
//! The link is expected to drop. When it does the member carries on with
//! whatever it was doing, tries again after a pause that grows (so a hub
//! that is down is not hammered by every member at once), and when it gets
//! through tells the hub what it finished in the meantime. A result is kept
//! until the hub says it has it.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use conductor::{ActionError, Engine, InstanceFilter, RemoteCall};
use hosting::Hosting;
use kernel::{Cancel, Principal};
use serde::Serialize;
use tokio::sync::Notify;

use crate::wire::{
    Command, DeviceInfo, LinkError, LogEntry, Outcome, Report, RunCommand, ServiceBrief, Uplink,
};

/// How often the member says it is still there when it has nothing to say.
pub const HEARTBEAT: Duration = Duration::from_secs(15);
/// The longest pause between attempts to reach the hub.
pub const MAX_BACKOFF: Duration = Duration::from_secs(60);
/// Log lines kept for the hub while the link is down; older ones are dropped.
const MAX_HELD_LINES: usize = 5_000;
/// How many answered commands are remembered, to recognise late copies.
const REMEMBERED: usize = 2_048;
/// How long an answer the hub has not taken is kept for it.
const KEEP_UNCLAIMED: Duration = Duration::from_secs(3600);

/// How much goes in one report: this many lines, this many answers.
struct Most {
    lines: usize,
    answers: usize,
}

/// An answer waiting for the hub to take it.
struct Held {
    /// When the work finished.
    at: i64,
    outcome: Outcome,
}

/// How the link to the hub stands.
#[derive(Clone, Debug, Default, Serialize)]
pub struct LinkStatus {
    pub connected: bool,
    /// When it entered this state.
    pub since: i64,
    /// What the hub calls itself, once it has said.
    pub hub: Option<String>,
    /// What the hub calls this device.
    pub device: Option<String>,
    /// Why it is not connected, when it is not.
    pub error: Option<String>,
    /// Failed attempts since it was last connected.
    pub failures: u32,
    /// When the next attempt will be made, when one is waiting.
    pub retry_at: Option<i64>,
}

#[derive(Clone)]
pub(crate) struct Member {
    inner: Arc<Inner>,
}

struct Inner {
    engine: Engine,
    hosting: Hosting,
    data_dir: PathBuf,
    /// What the hub may do here: a principal holding the scopes this device
    /// agreed to when it joined.
    allow: Principal,
    state: Mutex<State>,
    /// Signalled when there is something to report.
    wake: Notify,
    /// What the hub was last told about this device is out of date.
    stale: AtomicBool,
}

#[derive(Default)]
struct State {
    running: HashMap<String, Cancel>,
    /// Finished, and not yet acknowledged by the hub.
    results: Vec<Held>,
    /// Running commands the hub has stopped waiting for.
    called_off: HashSet<String>,
    /// Commands whose results the hub has taken, most recent last. A copy
    /// of one of these that turns up late must not be run again.
    delivered: VecDeque<String>,
    logs: Vec<LogEntry>,
    status: LinkStatus,
    /// The daemon is stopping: what is called off now is not an answer.
    halted: bool,
}

impl State {
    /// Notes that a command has been answered and the answer taken.
    fn remember(&mut self, id: String) {
        if self.delivered.len() >= REMEMBERED {
            self.delivered.pop_front();
        }
        self.delivered.push_back(id);
    }
}

/// The pause before attempt `failures + 1`: one second, doubling, capped.
pub fn backoff(failures: u32) -> Duration {
    let seconds = 1u64 << failures.saturating_sub(1).min(6);
    Duration::from_secs(seconds).min(MAX_BACKOFF)
}

impl Member {
    pub fn new(engine: Engine, hosting: Hosting, data_dir: PathBuf, allow: Vec<String>) -> Member {
        let allow = Principal {
            kind: kernel::PrincipalKind::Device,
            id: "hub".into(),
            name: "the hub".into(),
            scopes: allow,
            key: None,
        };
        Member {
            inner: Arc::new(Inner {
                engine,
                hosting,
                data_dir,
                allow,
                state: Mutex::new(State::default()),
                wake: Notify::new(),
                stale: AtomicBool::new(false),
            }),
        }
    }

    /// Whether this is that member, and not another made for a later link.
    pub(crate) fn is(&self, other: &Member) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.inner.state.lock().expect("member lock")
    }

    pub fn status(&self) -> LinkStatus {
        self.state().status.clone()
    }

    /// Calls off whatever is being done for the hub, as the daemon stopping
    /// does, and says nothing more to the hub. Work stopped this way has no
    /// result: the hub still wants it done, and will ask the next daemon.
    pub fn halt(&self) {
        let mut state = self.state();
        state.halted = true;
        for cancel in state.running.values() {
            cancel.cancel();
        }
    }

    fn set_status(&self, change: impl FnOnce(&mut LinkStatus)) {
        let now = self.inner.engine.journal().now_ms();
        let mut state = self.state();
        let was = state.status.connected;
        change(&mut state.status);
        if state.status.connected != was || state.status.since == 0 {
            state.status.since = now;
        }
    }

    /// Stops everything this device was doing for its hub and keeps no
    /// answer for it: the hub has said, sealed, that this device is no
    /// longer one of its fleet, so nothing done here for it will be taken.
    /// The token is the authority, and it has been withdrawn.
    fn taken_out(&self) {
        let mut state = self.state();
        let running: Vec<String> = state.running.keys().cloned().collect();
        for id in running {
            if let Some(cancel) = state.running.get(&id) {
                cancel.cancel();
            }
            state.called_off.insert(id);
        }
        state.results.clear();
        state.logs.clear();
    }

    /// Keeps the link to the hub up until `stop` is raised, or until the
    /// hub itself says this device is no longer one of its fleet: then
    /// what was being done for it is stopped, nothing more is asked of it,
    /// and what the hub said is handed back for whoever keeps the
    /// membership to forget it.
    pub async fn run(&self, uplink: Arc<dyn Uplink>, stop: Cancel) -> Option<String> {
        let clock = self.inner.engine.journal().clock().clone();
        let mut failures = 0u32;
        // Whether it has been said that what answers is not the hub.
        let mut unproven = false;
        loop {
            // Trying now: no next attempt is waiting.
            self.set_status(|status| status.retry_at = None);
            // Asking for the link is given the silence a link is allowed
            // and no longer, and is given up at once when this device is
            // told to stop: whatever takes the asking and never answers
            // holds nothing up for good.
            let opened = tokio::select! {
                opened = uplink.open() => opened,
                () = clock.sleep(crate::http::SILENCE) => Err(LinkError::new(
                    "the hub took this device's asking for its link and did not answer in time",
                )),
                () = stop.cancelled() => {
                    self.set_status(|status| status.connected = false);
                    return None;
                }
            };
            let error = match opened {
                Ok(mut commands) => {
                    failures = 0;
                    unproven = false;
                    self.set_status(|status| {
                        *status = LinkStatus {
                            connected: true,
                            hub: status.hub.take(),
                            device: status.device.take(),
                            ..LinkStatus::default()
                        }
                    });
                    let link = stop.child();
                    let reporter = tokio::spawn(self.clone().report(uplink.clone(), link.clone()));
                    tokio::spawn(self.clone().notice(link.clone()));
                    loop {
                        tokio::select! {
                            command = commands.recv() => match command {
                                Some(command) => self.handle(command),
                                None => break,
                            },
                            () = stop.cancelled() => break,
                        }
                    }
                    link.cancel();
                    let _ = reporter.await;
                    LinkError::new("the link to the hub was lost")
                }
                Err(err) => err,
            };
            if stop.is_cancelled() {
                self.set_status(|status| status.connected = false);
                return None;
            }
            failures += 1;
            if error.out {
                // The hub itself says this device is out of its fleet:
                // sealed with the key it proved, so nobody else said it.
                // What was being done for it is stopped, and it is asked
                // nothing more: asking again every few minutes for ever
                // would be told the same, and written down in two journals.
                self.taken_out();
                self.set_status(|status| {
                    status.connected = false;
                    status.error = Some(error.message.clone());
                });
                return Some(error.message);
            }
            if error.unproven && !std::mem::replace(&mut unproven, true) {
                // Said once for as long as it goes on: what answers is not
                // the hub this device joined, and nothing it asks is done.
                let why = serde_json::json!({ "why": error.message });
                let _ = self
                    .inner
                    .engine
                    .journal()
                    .emit("device.unproven", kernel::Source::actor("system"), why)
                    .await;
            }
            // A hub that will not let this device in, and something that is
            // not the hub at all, will not change in a second; one that
            // asks for a pause gets it.
            // What is asked for comes in the clear, from whatever answered:
            // it can make this device wait longer, within reason, and never
            // less than it would have waited anyway.
            let pause = match (error.retry_after, error.refused || error.unproven) {
                (Some(asked), _) => asked.max(backoff(failures)),
                (None, true) => MAX_BACKOFF,
                (None, false) => backoff(failures),
            };
            // Not all members at the same instant: up to a fifth longer.
            let jitter = kernel::id::random_below((pause.as_millis() as u64 / 5).max(1));
            let pause = pause + Duration::from_millis(jitter);
            let retry_at = self.inner.engine.journal().now_ms() + pause.as_millis() as i64;
            self.set_status(|status| {
                status.connected = false;
                status.error = Some(error.message.clone());
                status.failures = failures;
                status.retry_at = Some(retry_at);
            });
            tokio::select! {
                () = clock.sleep(pause) => {}
                () = stop.cancelled() => return None,
            }
        }
    }

    fn handle(&self, command: Command) {
        match command {
            Command::Welcome { hub, device } => self.set_status(|status| {
                status.hub = Some(hub);
                status.device = Some(device);
            }),
            Command::Cancel { id } => {
                // The hub has stopped waiting: stop the work, and do not
                // keep an answer nobody will ask for.
                let mut state = self.state();
                if let Some(cancel) = state.running.get(&id) {
                    cancel.cancel();
                    state.called_off.insert(id);
                } else {
                    state.results.retain(|held| held.outcome.id != id);
                }
            }
            Command::Run(command) => self.start(*command),
        }
    }

    /// Runs a command, unless it is one this member already has: the hub
    /// sends again whatever it is still waiting for whenever the link comes
    /// back, and must get one run and one answer for it.
    fn start(&self, command: RunCommand) {
        let cancel = Cancel::new();
        {
            let mut state = self.state();
            if state.running.contains_key(&command.id) {
                return;
            }
            if state.results.iter().any(|held| held.outcome.id == command.id) {
                // Done already; the answer is waiting to be taken.
                drop(state);
                self.inner.wake.notify_one();
                return;
            }
            if state.delivered.contains(&command.id) {
                // Done, answered, and the answer taken: this is a copy that
                // was on its way while that happened.
                return;
            }
            state.running.insert(command.id.clone(), cancel.clone());
        }
        let member = self.clone();
        tokio::spawn(async move {
            let id = command.id.clone();
            // The one thing a hub's step leaves running here is an
            // instance it applies: kept, by the instance, so that what the
            // hub started here can be named if the hub ever takes this
            // device out.
            let applies = command.action == "blueprint.apply";
            let work = tokio::spawn(member.clone().perform(command, cancel));
            let result =
                work.await.unwrap_or_else(|_| Err(ActionError::new("the action stopped unexpectedly")));
            let applied = result.as_ref().ok().filter(|_| applies).and_then(|out| out["instance"].as_str());
            if let Some(instance) = applied {
                let journal = member.inner.engine.journal();
                let at = serde_json::json!({ "at": journal.now_ms() });
                let _ = journal.kv_set(crate::copy::STARTED, instance, &at).await;
            }
            {
                let mut state = member.state();
                state.running.remove(&id);
                if state.halted {
                    return;
                }
                if state.called_off.remove(&id) {
                    // Nobody is waiting for how this went. Remembered as
                    // answered, so a late copy of the command does nothing.
                    state.remember(id);
                    return;
                }
                let at = member.inner.engine.journal().now_ms();
                state.results.push(Held { at, outcome: Outcome::of(&id, &result) });
            }
            member.inner.wake.notify_one();
        });
    }

    /// `sent`, holding only what this device lets its hub do: every scope
    /// that is both in what was sent and in what is allowed. A hub can ask
    /// for work to be done as someone who may do anything; here that is
    /// anything this device agreed to, and nothing more.
    fn within(&self, sent: Principal) -> Principal {
        let allow = &self.inner.allow;
        let mut scopes = Vec::new();
        for scope in &sent.scopes {
            if scope == "*" {
                scopes.extend(allow.scopes.iter().cloned());
            } else if allow.can(scope) {
                scopes.push(scope.clone());
            }
        }
        // The narrower of the two, where one names a whole area and the
        // other a part of it.
        scopes.extend(allow.scopes.iter().filter(|scope| *scope != "*" && sent.can(scope)).cloned());
        scopes.sort();
        scopes.dedup();
        // Which key of the hub's own whoever asked came with means nothing
        // among this device's keys, and a key here is withdrawn by its id
        // alone: what the hub sends is done as nobody's key.
        Principal { scopes, key: None, ..sent }
    }

    async fn perform(self, command: RunCommand, cancel: Cancel) -> conductor::ActionResult {
        // What this device agreed to when it joined is checked here, on the
        // device, whatever the hub believes it may ask for.
        let needs = self
            .inner
            .engine
            .registry()
            .get(&command.action)
            .and_then(|handler| handler.spec().capability.clone());
        if let Some(capability) = needs
            && !self.inner.allow.can(&capability)
        {
            return Err(ActionError::new(format!(
                "device `{}` does not let its hub use `{capability}`; it allows: {}",
                self.status().device.unwrap_or_else(|| self.inner.engine.journal().device().to_string()),
                self.inner.allow.scopes.join(", ")
            )));
        }
        // And it is done as no more than this device agreed to, whatever
        // the hub said to do it as: for this action, and for everything
        // that comes of it here (a blueprint it applies, and that
        // blueprint's steps), which are held to the same principal.
        let principal = self.within(command.principal);
        let (member, id) = (self.clone(), command.id.clone());
        let on_log = Arc::new(move |stream: char, text: &str| {
            let mut state = member.state();
            if state.logs.len() >= MAX_HELD_LINES {
                state.logs.remove(0);
            }
            state.logs.push(LogEntry { id: id.clone(), stream, text: text.to_string() });
            drop(state);
            member.inner.wake.notify_one();
        });
        // An environment decides what a program does as surely as its
        // arguments do (which programs it finds, what it loads). The hub's
        // reaches a program here only where the hub may run commands here;
        // otherwise it is left out, and the step's own log says so.
        let env = if command.env.is_empty() || principal.can("shell") {
            command.env
        } else {
            on_log(
                'e',
                &format!(
                    "the environment this step was sent with ({}) was left out: this device does not let its hub run commands (`shell`)",
                    command.env.keys().cloned().collect::<Vec<_>>().join(", ")
                ),
            );
            Default::default()
        };
        // A secret this device keeps is handed to a program here only
        // where the hub may use what is kept secret here, and may run
        // commands here: which variable it goes in is the hub's to say, as
        // the rest of the environment is. Unlike the environment it is not
        // left out, since a program started without what it was to be
        // handed fails further on, saying less.
        if !command.secrets.is_empty()
            && let Some(lacks) = ["secrets:use", "shell"].into_iter().find(|scope| !principal.can(scope))
        {
            return Err(ActionError::new(format!(
                "device `{}` does not let its hub hand a secret it keeps to a program (`{lacks}`); it allows: {}",
                self.status().device.unwrap_or_else(|| self.inner.engine.journal().device().to_string()),
                self.inner.allow.scopes.join(", ")
            )));
        }
        // A project's environment's secret is kept where its project's
        // environments are, by the hub, and sealed for that installation:
        // this device keeps none, and does not hand one of its own of the
        // same name in its place.
        if let Some(name) = command.secrets.values().find(|name| name.contains('/')) {
            let parts: Vec<&str> = name.split('/').collect();
            let (environment, name) =
                (parts.get(1).copied().unwrap_or_default(), parts.last().copied().unwrap_or(name));
            return Err(ActionError::new(format!(
                "device `{}` keeps none of a project's environments' secrets: `{name}` of the `{environment}` \
                 environment is handed only to a program run on the device that keeps it, the hub's. Run this \
                 step there, or name a secret this device keeps",
                self.status().device.unwrap_or_else(|| self.inner.engine.journal().device().to_string())
            )));
        }
        self.inner
            .engine
            .run_remote(RemoteCall {
                action: command.action,
                args: command.args,
                principal,
                project: command.project,
                origin: command.origin,
                attempt: command.attempt,
                env,
                secrets: command.secrets,
                cancel,
                on_log,
            })
            .await
    }

    /// Watches what this device is running, so the hub hears of a change
    /// when it happens and not at the next heartbeat.
    async fn notice(self, link: Cancel) {
        let journal = self.inner.engine.journal().clone();
        let mut changes = journal.follow_every(journal.now(), "service.*,instance.*");
        loop {
            tokio::select! {
                event = changes.next() => {
                    if event.is_err() {
                        return;
                    }
                    self.inner.stale.store(true, Ordering::SeqCst);
                    self.inner.wake.notify_one();
                }
                () = link.cancelled() => return,
            }
        }
    }

    /// Sends reports for as long as the link is up: at once when there is
    /// news, and every [`HEARTBEAT`] when there is none.
    async fn report(self, uplink: Arc<dyn Uplink>, link: Cancel) {
        let clock = self.inner.engine.journal().clock().clone();
        // The first report says who this is and what it still holds.
        let mut introduce = true;
        // How many lines and how many answers go in one report: fewer
        // after one was too much to be said at once.
        let mut most = Most { lines: usize::MAX, answers: usize::MAX };
        loop {
            // Looked at each time round, so that nothing here goes on
            // after the link it reports over is gone, whatever else it
            // finds to do.
            if link.is_cancelled() {
                return;
            }
            introduce |= self.inner.stale.swap(false, Ordering::SeqCst);
            let (logs, report) = {
                let now = self.inner.engine.journal().now_ms();
                let mut state = self.state();
                if state.halted {
                    return;
                }
                // An answer nobody claimed in its time is not said again.
                state.results.retain(|held| now - held.at < KEEP_UNCLAIMED.as_millis() as i64);
                let take = most.lines.min(state.logs.len());
                let logs: Vec<LogEntry> = state.logs.drain(..take).collect();
                let report = Report {
                    info: None,
                    running: state.running.keys().cloned().collect(),
                    results: state
                        .results
                        .iter()
                        .take(most.answers)
                        .map(|held| held.outcome.clone())
                        .collect(),
                    logs: logs.clone(),
                };
                (logs, report)
            };
            let said: Vec<String> = report.results.iter().map(|outcome| outcome.id.clone()).collect();
            let report = Report { info: if introduce { Some(self.info().await) } else { None }, ..report };
            if !report.is_empty() {
                match uplink.report(report).await {
                    Ok(ack) => {
                        introduce = false;
                        // More at once next time, up to all of it: back to
                        // that by doubling, so that what was too much is
                        // not tried whole after every report that went.
                        most = Most {
                            lines: most.lines.saturating_mul(2),
                            answers: most.answers.saturating_mul(2),
                        };
                        let mut state = self.state();
                        // An answer the hub did not take is kept and offered
                        // again: the hub may not be ready for it yet. It
                        // goes behind the ones not yet said, so that where
                        // only some fit in a report each has its turn.
                        state.results.retain(|held| !ack.ids.contains(&held.outcome.id));
                        let (untaken, unsaid): (Vec<Held>, Vec<Held>) = std::mem::take(&mut state.results)
                            .into_iter()
                            .partition(|held| said.contains(&held.outcome.id));
                        state.results = unsaid;
                        state.results.extend(untaken);
                        for id in ack.ids {
                            state.remember(id);
                        }
                    }
                    Err(err) => {
                        // Not delivered: the lines go back where they were,
                        // ahead of any written since. Results were never
                        // removed. The link loop notices the outage itself.
                        let sent = Most { lines: logs.len(), answers: said.len() };
                        let made_less = {
                            let mut state = self.state();
                            let newer = std::mem::replace(&mut state.logs, logs);
                            state.logs.extend(newer);
                            err.too_much && Self::say_less(&mut state, &sent, &mut most)
                        };
                        if made_less {
                            // Too much to say at once, and now there is
                            // less of it: said again as it is, not for ever.
                            continue;
                        }
                        tokio::select! {
                            () = clock.sleep(Duration::from_secs(1)) => continue,
                            () = link.cancelled() => return,
                        }
                    }
                }
            }
            // While a report was on its way more may have been written; that
            // is the batching, and it needs no timer. And where only some
            // of the answers went in it, the rest go now.
            let more = {
                let state = self.state();
                !state.logs.is_empty() || state.results.iter().any(|held| !said.contains(&held.outcome.id))
            };
            if more {
                continue;
            }
            tokio::select! {
                () = self.inner.wake.notified() => {}
                () = clock.sleep(HEARTBEAT) => introduce = true,
                () = link.cancelled() => return,
            }
        }
    }

    /// Makes the next report smaller than one that was too much to say at
    /// once, by the first of these that makes it any smaller: half the
    /// lines; no line longer than a line should be; half the answers, the
    /// rest in the reports after; the one answer that is itself too much
    /// replaced by its failure, which says why. One thing at a time, so
    /// that nothing is cut that did not have to be. Says whether anything
    /// was made less: where nothing could be, sending again would be the
    /// same again.
    fn say_less(state: &mut State, sent: &Most, most: &mut Most) -> bool {
        /// The longest a line or an answer is left when a report must shrink.
        const KEPT: usize = 64 * 1024;
        /// What is put after a line that was cut. The line is cut so that
        /// it and this are no longer than a line is left: cut once, it is
        /// not found too long again.
        const CUT: &str = " [the rest of this line was more than a link carries]";
        if sent.lines > 1 {
            most.lines = sent.lines / 2;
            return true;
        }
        let mut less = false;
        for line in &mut state.logs {
            if line.text.len() > KEPT {
                let room = KEPT - CUT.len();
                let cut = (0..=room).rev().find(|at| line.text.is_char_boundary(*at)).unwrap_or(0);
                line.text.truncate(cut);
                line.text.push_str(CUT);
                less = true;
            }
        }
        if less {
            return true;
        }
        if sent.answers > 1 {
            most.answers = sent.answers / 2;
            return true;
        }
        let size = |held: &Held| serde_json::to_vec(&held.outcome).map_or(0, |written| written.len());
        // The one answer that was said: the first, where they are said a
        // few at a time.
        if let Some(largest) = state.results.iter_mut().take(sent.answers).max_by_key(|held| size(held))
            && size(largest) > KEPT
        {
            let failed = Err(ActionError::new(format!(
                "what this step gave out on its device is {} bytes, and the most that is said at once over a link is {}. \
                 Have it give out less: keep what is large in a file there, and give out where it is",
                size(largest),
                crate::sealed::MAX_FRAME
            )));
            largest.outcome = Outcome::of(&largest.outcome.id.clone(), &failed);
            less = true;
        }
        less
    }

    /// This device's account of itself.
    async fn info(&self) -> DeviceInfo {
        let dir = self.inner.data_dir.clone();
        let found =
            tokio::task::spawn_blocking(move || hosting::resources(&dir)).await.ok().and_then(Result::ok);
        let instances = self
            .inner
            .engine
            .instances(InstanceFilter { live_only: true, limit: 10_000, ..InstanceFilter::default() })
            .await
            .map_or(0, |instances| instances.len());
        let services = self
            .inner
            .hosting
            .services()
            .into_iter()
            .filter(|service| service.status != hosting::Status::Stopped)
            .map(|service| ServiceBrief {
                name: service.name,
                status: serde_json::to_value(service.status)
                    .ok()
                    .and_then(|status| status.as_str().map(str::to_string))
                    .unwrap_or_default(),
                port: service.port,
            })
            .collect();
        let found = found.as_ref();
        DeviceInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            os: found.map(|f| f.os.clone()).unwrap_or_default(),
            arch: found.map(|f| f.arch.clone()).unwrap_or_default(),
            cpus: found.map_or(0, |f| f.cpus),
            load1: found.and_then(|f| f.load1),
            free_mem_mb: found.map_or(0, |f| f.free_mem_mb),
            free_disk_mb: found.and_then(|f| f.free_disk_mb),
            instances,
            services,
        }
    }
}
