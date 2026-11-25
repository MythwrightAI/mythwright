//! The hub's side: which members are linked, and the steps waiting on them.
//!
//! A step for another device becomes a command on that member's link. The
//! command stays here until the member reports how it went, however many
//! times the link drops in between: when a member links again it is told
//! everything still waiting on it, and it runs a command it has already seen
//! only once. So a link that flaps costs time, not correctness.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use conductor::{ActionCx, ActionError, ActionResult, Remote};
use journal::Journal;
use kernel::{BoxFut, Error, Fields, Result, Source};
use serde::Serialize;
use serde_json::json;
use tokio::sync::{Notify, mpsc, oneshot};

use crate::wire::{Ack, Command, DeviceInfo, Report, RunCommand};

/// How long a step waits for a member that is not linked before it fails.
/// A member that is restarting is back well within this; one that is gone
/// should fail the step so that its retries and hooks can do something.
pub const DEFAULT_GRACE: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct Hub {
    inner: Arc<Inner>,
}

struct Inner {
    journal: Journal,
    state: Mutex<State>,
    /// Signalled when a link opens or closes.
    changed: Notify,
    grace: Mutex<Duration>,
    /// Links opening and closing, waiting to be written to the journal.
    announcements: OnceLock<mpsc::UnboundedSender<(&'static str, String, Option<Seen>)>>,
}

#[derive(Default)]
struct State {
    links: HashMap<String, Link>,
    pending: HashMap<String, Pending>,
    seen: HashMap<String, Seen>,
    serial: u64,
}

struct Link {
    serial: u64,
    commands: mpsc::Sender<Command>,
    since: i64,
}

struct Pending {
    device: String,
    command: RunCommand,
    cx: ActionCx,
    reply: oneshot::Sender<ActionResult>,
}

/// What is known of a member from its own reports.
#[derive(Clone, Debug, Default, Serialize, serde::Deserialize)]
pub struct Seen {
    pub last_seen: i64,
    pub info: Option<DeviceInfo>,
}

/// The commands for one member, for as long as its link is open. Dropping
/// this is what closes the link.
pub struct LinkStream {
    commands: mpsc::Receiver<Command>,
    _closed: Closed,
}

impl LinkStream {
    /// The next command for the member; `None` once the hub has closed the
    /// link. Taking commands through the stream itself (rather than a part
    /// of it) is what keeps the link open for exactly as long as it is read.
    pub async fn next(&mut self) -> Option<Command> {
        self.commands.recv().await
    }
}

struct Closed {
    hub: Hub,
    device: String,
    serial: u64,
}

impl Drop for Closed {
    fn drop(&mut self) {
        self.hub.closed(&self.device, self.serial);
    }
}

impl Hub {
    pub fn new(journal: Journal) -> Hub {
        Hub {
            inner: Arc::new(Inner {
                journal,
                state: Mutex::new(State::default()),
                changed: Notify::new(),
                grace: Mutex::new(DEFAULT_GRACE),
                announcements: OnceLock::new(),
            }),
        }
    }

    pub fn set_grace(&self, grace: Duration) {
        *self.inner.grace.lock().expect("hub lock") = grace;
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.inner.state.lock().expect("hub lock")
    }

    /// When `device` linked, if it is linked now.
    pub fn linked_since(&self, device: &str) -> Option<i64> {
        self.state().links.get(device).map(|link| link.since)
    }

    pub fn seen(&self, device: &str) -> Option<Seen> {
        self.state().seen.get(device).cloned()
    }

    /// Opens the link for `device`, replacing any it already had: a member
    /// that links again has given up on its old link, whatever we believe.
    pub(crate) async fn open_link(&self, device: &str) -> LinkStream {
        let (commands, stream) = mpsc::channel(256);
        let now = self.inner.journal.now_ms();
        let (serial, waiting) = {
            let mut state = self.state();
            state.serial += 1;
            let serial = state.serial;
            state.links.insert(device.to_string(), Link { serial, commands: commands.clone(), since: now });
            state.seen.entry(device.to_string()).or_default().last_seen = now;
            let waiting: Vec<RunCommand> = state
                .pending
                .values()
                .filter(|pending| pending.device == device)
                .map(|pending| pending.command.clone())
                .collect();
            (serial, waiting)
        };
        let welcome =
            Command::Welcome { hub: self.inner.journal.device().to_string(), device: device.to_string() };
        // There is room for it: nothing else has been put in yet.
        let _ = commands.try_send(welcome);
        // Everything still waiting on this member, in case it never got it.
        // Put in as it is read, and not here: nobody reads until this has
        // returned, and with more waiting than there is room for it never
        // would.
        if !waiting.is_empty() {
            tokio::spawn(async move {
                for command in waiting {
                    if commands.send(Command::Run(Box::new(command))).await.is_err() {
                        return;
                    }
                }
            });
        }
        self.inner.changed.notify_waiters();
        self.announce("device.connected", device, None);
        LinkStream {
            commands: stream,
            _closed: Closed { hub: self.clone(), device: device.to_string(), serial },
        }
    }

    fn closed(&self, device: &str, serial: u64) {
        let seen = {
            let mut state = self.state();
            if state.links.get(device).is_none_or(|link| link.serial != serial) {
                // Already replaced by a newer link.
                return;
            }
            state.links.remove(device);
            let now = self.inner.journal.now_ms();
            let seen = state.seen.entry(device.to_string()).or_default();
            seen.last_seen = now;
            seen.clone()
        };
        self.inner.changed.notify_waiters();
        self.announce("device.disconnected", device, Some(seen));
    }

    /// Records that a link opened or closed. A link closes wherever its
    /// stream happens to be dropped, which is no place to wait on the
    /// journal, so both kinds go through one queue: they are recorded in the
    /// order they happened, which is the order anyone reading them needs.
    fn announce(&self, topic: &'static str, device: &str, seen: Option<Seen>) {
        let queue = self.inner.announcements.get_or_init(|| {
            let (queue, mut waiting) = mpsc::unbounded_channel::<(&'static str, String, Option<Seen>)>();
            let journal = self.inner.journal.clone();
            if let Ok(runtime) = tokio::runtime::Handle::try_current() {
                runtime.spawn(async move {
                    while let Some((topic, device, seen)) = waiting.recv().await {
                        if let Some(seen) = seen {
                            // Kept, so "last seen" survives this hub restarting.
                            let _ = journal.kv_set(crate::copy::SEEN, &device, &json!(seen)).await;
                        }
                        let _ =
                            journal.emit(topic, Source::actor("system"), json!({ "device": device })).await;
                    }
                });
            }
            queue
        });
        let _ = queue.send((topic, device.to_string(), seen));
    }

    /// Closes `device`'s link, if it has one. Its member will try again,
    /// and get in only if its credential is still good.
    pub fn unlink(&self, device: &str) {
        if self.state().links.remove(device).is_some() {
            self.inner.changed.notify_waiters();
        }
    }

    /// Closes the link of a device that has been taken out of the fleet,
    /// and ends every step that was waiting on it, as failed and saying
    /// so: nothing will come back from it, and waiting out how long a
    /// device may be away would only say that later. What it was doing it
    /// is told to stop by the hub's own answer when it next asks to be let
    /// in (`Welcome::unknown`): its session is over, so nothing more can
    /// be said to it down the link it had.
    pub fn taken_out(&self, device: &str) {
        let waited: Vec<Pending> = {
            let mut state = self.state();
            state.links.remove(device);
            let ids: Vec<String> = state
                .pending
                .iter()
                .filter(|(_, pending)| pending.device == device)
                .map(|(id, _)| id.clone())
                .collect();
            ids.iter().filter_map(|id| state.pending.remove(id)).collect()
        };
        self.inner.changed.notify_waiters();
        for pending in waited {
            let why = format!(
                "`{device}` was taken out of this fleet while this step was waiting on it: nothing it was doing \
                 for this hub is taken from it, and it stops when it is next told so"
            );
            let _ = pending.reply.send(Err(conductor::ActionError::new(why)));
        }
    }

    /// Closes every link and forgets what was waiting, as when the daemon
    /// stops. Steps that were waiting are resumed by the next daemon.
    pub fn halt(&self) {
        let mut state = self.state();
        state.links.clear();
        state.pending.clear();
    }

    /// Takes a member's report: its log lines go to the steps they belong
    /// to, its results end the steps that were waiting for them.
    pub async fn report(&self, device: &str, report: Report) -> Ack {
        let now = self.inner.journal.now_ms();
        let mut lines = Vec::new();
        let mut finished = Vec::new();
        let mut lost = Vec::new();
        let mut ids = Vec::new();
        let mut changed = false;
        let commands = {
            let mut state = self.state();
            let seen = state.seen.entry(device.to_string()).or_default();
            seen.last_seen = now;
            let heartbeat = report.info.is_some();
            if let Some(info) = report.info {
                // What a member is running having changed is news; that it
                // is still there, with a slightly different load, is not.
                let was = seen.info.as_ref().map(|old| (&old.services, old.instances, &old.version));
                changed = was != Some((&info.services, info.instances, &info.version));
                seen.info = Some(info);
            }
            for line in report.logs {
                if let Some(pending) = state.pending.get(&line.id).filter(|pending| pending.device == device)
                {
                    lines.push((pending.cx.clone(), line.stream, line.text));
                }
            }
            for outcome in report.results {
                // Only a result that reached the step waiting for it is
                // acknowledged. One that nothing is waiting for may belong to
                // a step this hub has not got back to yet (it has just
                // restarted and is still resuming its runs), so the member
                // is left holding it and offers it again.
                if state.pending.get(&outcome.id).is_some_and(|pending| pending.device == device)
                    && let Some(pending) = state.pending.remove(&outcome.id)
                {
                    ids.push(outcome.id.clone());
                    finished.push((pending.reply, outcome));
                }
            }
            if heartbeat {
                // What we are waiting for that the member does not have was
                // lost with an earlier link, or with the member's memory when
                // it restarted. Sending it again is safe either way.
                lost = state
                    .pending
                    .values()
                    .filter(|pending| {
                        pending.device == device && !report.running.contains(&pending.command.id)
                    })
                    .map(|pending| pending.command.clone())
                    .collect();
            }
            state.links.get(device).map(|link| link.commands.clone())
        };
        if changed {
            self.announce("device.changed", device, None);
        }
        for (cx, stream, text) in lines {
            cx.log(stream, &text);
        }
        for (reply, outcome) in finished {
            let _ = reply.send(outcome.into_result());
        }
        if let Some(commands) = commands {
            for command in lost {
                let _ = commands.send(Command::Run(Box::new(command))).await;
            }
        }
        Ack { ids }
    }

    /// Ends the step waiting for command `id` as failed, saying `why`: it
    /// could not be sent at all, and never will be.
    pub(crate) fn refuse(&self, id: &str, why: &str) {
        let pending = self.state().pending.remove(id);
        if let Some(pending) = pending {
            let _ = pending.reply.send(Err(conductor::ActionError::new(why)));
        }
    }

    /// Stops waiting for a command and tells the member to stop doing it.
    fn abandon(&self, id: &str) {
        let mut state = self.state();
        let Some(pending) = state.pending.remove(id) else { return };
        if let Some(link) = state.links.get(&pending.device) {
            // Not delivered if the link is choked or gone; then the member
            // finishes the work and its result is taken and dropped.
            let _ = link.commands.try_send(Command::Cancel { id: id.to_string() });
        }
    }

    async fn run(&self, device: String, action: String, args: Fields, cx: ActionCx) -> ActionResult {
        let id = format!("{}/{}/{}", cx.run(), cx.step(), cx.attempt());
        let command = RunCommand {
            id: id.clone(),
            action,
            args,
            principal: cx.principal().clone(),
            project: cx.project().map(str::to_string),
            // What the step prints is said on the device that does it, too,
            // and takes there what it takes here ([`ActionCx::about`]).
            origin: Source { device: Some(self.inner.journal.device().to_string()), ..cx.about() },
            attempt: cx.attempt(),
            env: cx.env().clone(),
            secrets: cx.secrets().clone(),
        };
        // If this daemon restarts while the member is working, the step is
        // picked up again instead of being failed as unsafe to repeat: the
        // same command is sent, and the member, still busy with it or holding
        // its result, answers without doing it twice.
        let _ = cx.checkpoint(json!({ "device": device, "command": id })).await;

        let (reply, mut answer) = oneshot::channel();
        let commands = {
            let mut state = self.state();
            state.pending.insert(
                id.clone(),
                Pending { device: device.clone(), command: command.clone(), cx: cx.clone(), reply },
            );
            state.links.get(&device).map(|link| link.commands.clone())
        };
        match commands {
            Some(commands) => {
                let _ = commands.send(Command::Run(Box::new(command))).await;
            }
            None => cx.log('s', &format!("device `{device}` is not linked; waiting for it")),
        }

        // However this stops waiting (an answer, a cancel, or the step's
        // timeout simply dropping it), a command nobody is waiting for is
        // called off.
        struct Waiting<'a>(&'a Hub, &'a str);
        impl Drop for Waiting<'_> {
            fn drop(&mut self) {
                self.0.abandon(self.1);
            }
        }
        let _waiting = Waiting(self, &id);

        let grace = *self.inner.grace.lock().expect("hub lock");
        loop {
            // Listen for changes before looking, so one that happens in
            // between is not missed.
            let changed = self.inner.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            let linked = self.linked_since(&device).is_some();
            tokio::select! {
                result = &mut answer => {
                    return result.unwrap_or_else(|_| Err(ActionError::new("the gateway stopped before the device answered")));
                }
                () = cx.cancel().cancelled() => return Err(ActionError::cancelled()),
                () = &mut changed => {}
                () = cx.clock().sleep(grace), if !linked => {
                    let last = self.seen(&device).map(|seen| seen.last_seen).filter(|at| *at > 0);
                    let since = match last {
                        Some(at) => format!("last heard from {}", kernel::clock::rfc3339(at)),
                        None => "never heard from".to_string(),
                    };
                    return Err(ActionError::new(format!(
                        "device `{device}` has not been linked for {}; {since}",
                        blueprint::duration::format(grace.as_millis() as u64)
                    ))
                    .with_data(json!({ "device": device, "unreachable": true })));
                }
            }
        }
    }

    pub(crate) fn require_known(known: &[String], device: &str) -> Result<()> {
        if known.iter().any(|name| name == device) {
            return Ok(());
        }
        let list = if known.is_empty() {
            "no devices have been added to it".to_string()
        } else {
            format!("it has: {}", known.join(", "))
        };
        Err(Error::not_found(format!("there is no device called `{device}` in this fleet; {list}")))
    }
}

/// The hub as the conductor sees it: something that can run an action on a
/// named device.
pub(crate) struct Dispatch {
    pub hub: Hub,
    pub known: Arc<dyn Fn() -> BoxFut<'static, Vec<String>> + Send + Sync>,
}

impl Remote for Dispatch {
    fn run_action(
        &self,
        device: &str,
        action: &str,
        args: Fields,
        cx: ActionCx,
    ) -> BoxFut<'static, ActionResult> {
        let (hub, known) = (self.hub.clone(), self.known.clone());
        let (device, action) = (device.to_string(), action.to_string());
        Box::pin(async move {
            // A name nobody has is a mistake in the blueprint, not a device
            // that is slow to link: say so now rather than after the grace.
            Hub::require_known(&known().await, &device)?;
            hub.run(device, action, args, cx).await
        })
    }
}
