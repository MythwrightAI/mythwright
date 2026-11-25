//! The factory stopped and started again by a step of its own
//! (`factory.restart`), so that a program installed meanwhile is the one
//! that runs from then on.
//!
//! A daemon cannot start itself again, and does not try: what keeps it
//! running does (`up`, which the desktop app is too). So the step asks the
//! node to stop saying that it is to come back, the program ends with a
//! word of its own for that (`serve` ends with 4), and the keeper starts
//! it again at once, as it starts again a daemon that died. Where nothing
//! keeps the daemon running it would not come back, and the step is
//! refused before anything is done, saying so.
//!
//! What asked is not killed by it: a daemon that stops leaves what runs as
//! it is written, and the next takes it up. The step waits for the stop
//! with nothing done that a stop could cut in two, and is cut off with the
//! rest; the daemon that comes back finds it under way and does it again.
//! Done again, it finds that it has been done by that alone: a step taken
//! up after a start of the daemon was under way when the daemon stopped,
//! so a start came after it began, which is what it was asked for
//! (`ActionCx::again`). It answers then, from the program that runs now,
//! and nothing is asked twice: a stop anywhere from the step's beginning
//! on comes to one start after it, as a step never stopped does.
//!
//! **At rest** (`at_rest`, which is the usual) it waits first until
//! nothing is under way in the factory but what asked: no run of the
//! runner (but the asker's own, and those of what started it, which wait
//! on it) and no turn of an agent's in hand. It is told, not looking: it
//! looks again each time a run, a turn or an instance ends, and when
//! another step asking the same comes to wait beside it (so that two such
//! steps do not each wait for the other). A run that waits for something
//! nobody does keeps the factory from rest for as long as it waits; the
//! step's own `timeout` is what bounds that.
//!
//! It came out of the daemon (`api`), which offers it under the name it
//! always had (`api::restarting`) and holds what it keeps for the node:
//! the action is added beside `factory.snapshot`, and what it says
//! (`system.restarting`) is described with the folder's own events.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionCx, ActionError, ActionResult, Registry};
use journal::Journal;
use kernel::{BoxFut, Fields};
use serde_json::{Value, json};

/// What a daemon that was asked to come back ends with, for what keeps it
/// running to start it again at once. It is no failure: a daemon that
/// died ends otherwise.
pub const AGAIN: u8 = 4;

/// What is held of the factory being started again, by the node and by
/// the steps that ask.
pub struct Restarting {
    data: PathBuf,
    /// Whether what started it said it keeps it running.
    kept: bool,
    asked: tokio::sync::Notify,
    again: AtomicBool,
    /// The instances of every step waiting for rest, with those above
    /// each: none of their runs keeps the factory from it.
    waiting: Mutex<Vec<BTreeSet<String>>>,
    joined: tokio::sync::Notify,
}

impl Restarting {
    pub fn new(data: PathBuf, kept: bool) -> Arc<Restarting> {
        Arc::new(Restarting {
            data,
            kept,
            asked: tokio::sync::Notify::new(),
            again: AtomicBool::new(false),
            waiting: Mutex::default(),
            joined: tokio::sync::Notify::new(),
        })
    }

    /// Completes when a step has asked for the daemon to be started again.
    pub async fn asked(&self) {
        self.asked.notified().await;
    }

    /// Whether a step asked for it: the node is then stopped to come back.
    pub fn again(&self) -> bool {
        self.again.load(Ordering::SeqCst)
    }
}

/// One step waiting for rest, for as long as it waits.
struct Waiting<'a>(&'a Restarting, BTreeSet<String>);

impl Drop for Waiting<'_> {
    fn drop(&mut self) {
        let mut waiting = self.0.waiting.lock().expect("restart lock");
        if let Some(at) = waiting.iter().position(|one| *one == self.1) {
            waiting.remove(at);
        }
    }
}

/// Adds `factory.restart` to what a blueprint can do.
pub fn register(registry: &mut Registry, restarting: Arc<Restarting>, agents: agents::Agents) {
    let spec = ActionSpec::new(
        "factory.restart",
        "Stop this factory and have what keeps it running start it again, once nothing else is under way, so that what was installed meanwhile runs from then on.",
    )
    .example(ASKED)
    // Every run in the factory is stopped with it: the owner's to ask, as
    // a stop is.
    .capability("admin")
    .arg("at_rest", T::Bool, "wait until nothing else is under way first: no run but this one and those that wait on it, and no agent's turn in hand (default true)")
    .output("came", "`restarted`: it answers once the factory has started again")
    .output("version", "the version of the program that runs now")
    .output("runs_from", "`installed`, or `checkout` for one run from where it was built")
    // Stood in, nothing is stopped: it is as though it had been.
    .stands(json!({ "came": "restarted", "version": env!("CARGO_PKG_VERSION"), "runs_from": "installed" }))
    // Done again after the start it asked for, it finds that start.
    .idempotent();
    registry.add_fn(spec, move |cx, args| -> BoxFut<'static, ActionResult> {
        let (restarting, agents) = (restarting.clone(), agents.clone());
        Box::pin(async move { restart(&restarting, &agents, cx, args).await })
    });
}

async fn restart(
    restarting: &Restarting,
    agents: &agents::Agents,
    cx: ActionCx,
    args: Fields,
) -> ActionResult {
    // Under way when the daemon stopped: a start came after it began.
    if cx.again() {
        cx.log('s', "the factory was started again since this step began");
        return Ok(json!({
            "came": "restarted", "version": env!("CARGO_PKG_VERSION"), "runs_from": crate::place::runs_from(),
        }));
    }
    let kept = || {
        (!restarting.kept && !crate::folder::kept(&restarting.data)).then(|| {
            ActionError::new(format!(
                "nothing keeps this factory running, so stopped it would not come back: it is started again only \
                 where it was started with `{} up` (or the desktop app). Start it that way, and do this again",
                brand::CLI
            ))
        })
    };
    if let Some(refused) = kept() {
        return Err(refused);
    }
    let at_rest = args.get("at_rest").and_then(Value::as_bool).unwrap_or(true);
    if at_rest {
        waited(restarting, agents, &cx).await?;
        // What keeps it may have gone while this waited.
        if let Some(refused) = kept() {
            return Err(refused);
        }
    }
    let _ = cx.emit("system.restarting", json!({ "at_rest": at_rest })).await;
    cx.log('s', "the factory is stopped, and what keeps it running starts it again");
    restarting.again.store(true, Ordering::SeqCst);
    restarting.asked.notify_one();
    // Cut off with everything else as the daemon stops, and done again by
    // the one that comes back.
    cx.cancel().cancelled().await;
    Err(ActionError::cancelled())
}

/// Waits until nothing is under way but what asked.
async fn waited(restarting: &Restarting, agents: &agents::Agents, cx: &ActionCx) -> Result<(), ActionError> {
    let engine = cx.engine();
    // The instance that asked, and each above it: theirs wait on this.
    let mut chain = BTreeSet::new();
    let mut at = Some(cx.instance().to_string());
    while let Some(id) = at.take() {
        if !chain.insert(id.clone()) {
            break;
        }
        at = engine.instance(&id).await.ok().and_then(|instance| instance.parent);
    }
    restarting.waiting.lock().expect("restart lock").push(chain.clone());
    let _waiting = Waiting(restarting, chain);
    restarting.joined.notify_waiters();
    let journal: &Journal = engine.journal();
    // Listening before looking, so that an ending in between is heard.
    let mut ended = journal.follow_every(journal.now(), "run.finished,instance.finished,agent.turn.*");
    let mut said = Vec::new();
    loop {
        let joined = restarting.joined.notified();
        let busy = under_way(restarting, agents, cx).await?;
        if busy.is_empty() {
            return Ok(());
        }
        if busy != said {
            cx.log('s', &format!("waiting for what is under way to end: {}", busy.join(", ")));
            said = busy;
        }
        tokio::select! {
            next = ended.next_or_behind() => { next.map_err(|err| ActionError::new(err.message))?; }
            () = joined => {}
            () = cx.cancel().cancelled() => return Err(ActionError::cancelled()),
        }
    }
}

/// What is under way that the factory waits for, in words: each run of
/// the runner that is no waiting step's or of what is above one, and each
/// agent with a turn in hand.
async fn under_way(
    restarting: &Restarting,
    agents: &agents::Agents,
    cx: &ActionCx,
) -> Result<Vec<String>, ActionError> {
    let engine = cx.engine();
    let mine: BTreeSet<String> =
        restarting.waiting.lock().expect("restart lock").iter().flatten().cloned().collect();
    let mut busy = Vec::new();
    for journal in engine.journal().every_keeping(journal::keeps::RUNS) {
        let store = runs::Store::open(&journal).await?;
        for run in store.unfinished_runs().await? {
            if run.status != runs::RunStatus::Running || mine.contains(&run.instance) {
                continue;
            }
            let of = engine.instance(&run.instance).await.map(|one| one.blueprint);
            busy.push(format!("a run of {}", of.unwrap_or_else(|_| run.instance.clone())));
        }
    }
    let slots = agents.slots().await?;
    let in_hand = agents.in_hand(slots.iter().map(|slot| slot.name.as_str())).await?;
    let mut turns: Vec<&String> = in_hand.keys().collect();
    turns.sort();
    busy.extend(turns.into_iter().map(|slot| format!("a turn of {slot}")));
    Ok(busy)
}

/// What `factory.restart` is shown by in the Reference and the editor.
const ASKED: &str = "\
blueprint: take-what-was-installed
permissions: [admin, shell]

params:
  install: { type: string, description: The command that installs the new program. }

steps:
  installed:
    do: shell
    with: { run: \"${params.install}\" }

  restarted:
    needs: [installed]
    do: factory.restart
    with: { at_rest: true }
";
