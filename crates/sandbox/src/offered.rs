//! The actions a sandbox has: a registry built for it, never the daemon's.
//!
//! Three kinds, and nothing else is ever in it ([`Kind`]). Which kind an
//! action is, is said where it is registered and by nothing here: what
//! is added by an entrance that is handed nothing (`blocks`, `deciding`,
//! the two blocks of a rollout) is a function of what a step hands it;
//! what the studio adds through the entrance for its tables
//! (`Studio::register_kept`) is the studio's own code over the journal
//! the sandbox made; and every other action there is on the device, which
//! this is told of only as what it says of itself, is given a stand-in
//! under that description. So an action somebody adds to the daemon is
//! here as a stand-in from the moment it is described, and is the real
//! one only where its crate adds it through one of those entrances.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use blueprint::{ActionSpec, EventSpec};
use conductor::{Action, ActionCx, ActionResult, Registry};
use journal::Order;
use kernel::{BoxFut, Fields};
use serde::Serialize;
use serde_json::Value;
use studio::Studio;

use crate::outside::Chosen;

/// What an action of a sandbox is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// The real one, which holds nothing: a word of the language, or a
    /// block that works something out from what it is handed and what
    /// the sandbox's own conductor keeps.
    Own,
    /// The real one, over the sandbox's own journal and folder: what
    /// keeps rows in the tables a factory has.
    Kept,
    /// A stand-in: it writes down what it was asked and answers what it
    /// was told to. The real one reaches outside the tables, and is
    /// not here.
    Stood,
}

/// A sandbox's registry, and what kind each action in it is.
pub struct Offered {
    pub registry: Registry,
    pub kinds: BTreeMap<String, Kind>,
}

/// Writes down as `kind` every action of `registry` that has none yet.
fn note(registry: &Registry, kinds: &mut BTreeMap<String, Kind>, kind: Kind) {
    for spec in registry.specs().specs() {
        kinds.entry(spec.name.clone()).or_insert(kind);
    }
}

/// How many actions of a sandbox's own are in the middle of their work.
pub(crate) type Working = Arc<AtomicUsize>;

/// One doing of an action of the sandbox's own: the run it was done
/// for, what the action is called, and where the order of what was said
/// stood as it began and as it ended (nothing yet, while it is at work).
#[derive(Clone, Debug)]
pub(crate) struct Done {
    pub run: String,
    pub action: String,
    pub from: u64,
    pub to: Option<u64>,
}

/// Every doing of an action of the sandbox's own, in the order they
/// began. What such an action says of a table or a channel names no
/// run, and often no instance: it is said by the studio as whoever
/// applied the text. Which run it was said for is known all the same,
/// by where it came in the order of what was said (`crate::shown`).
pub(crate) struct Doing {
    order: Arc<Order>,
    done: Mutex<Vec<Done>>,
}

impl Doing {
    pub fn new(order: Arc<Order>) -> Arc<Doing> {
        Arc::new(Doing { order, done: Mutex::default() })
    }

    /// Every doing so far.
    pub fn so_far(&self) -> Vec<Done> {
        self.done.lock().expect("doing lock").clone()
    }

    fn begun(&self, run: String, action: String) -> usize {
        let mut done = self.done.lock().expect("doing lock");
        done.push(Done { run, action, from: self.order.so_far(), to: None });
        done.len() - 1
    }

    fn ended(&self, which: usize) {
        if let Some(one) = self.done.lock().expect("doing lock").get_mut(which) {
            one.to = Some(self.order.so_far());
        }
    }
}

/// An action of the sandbox's own, which counts itself in as it begins
/// and out as it ends, however it ends: what says whether a sandbox is
/// at rest asks how many are at work (`crate::rest`). And it writes down
/// which run it was done for ([`Doing`]).
struct Counted {
    real: Arc<dyn Action>,
    working: Working,
    doing: Arc<Doing>,
}

/// Counts one out when it is let go of: at its end, or where its step
/// is called off in the middle of it.
struct AtWork(Working, Arc<Doing>, usize);

impl Drop for AtWork {
    fn drop(&mut self) {
        self.1.ended(self.2);
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl Counted {
    fn counted(&self, run: String, work: BoxFut<'static, ActionResult>) -> BoxFut<'static, ActionResult> {
        let (working, doing) = (self.working.clone(), self.doing.clone());
        let action = self.real.spec().name.clone();
        Box::pin(async move {
            working.fetch_add(1, Ordering::SeqCst);
            let which = doing.begun(run, action);
            let _at_work = AtWork(working, doing, which);
            work.await
        })
    }
}

impl Action for Counted {
    fn spec(&self) -> &ActionSpec {
        self.real.spec()
    }

    fn run(&self, cx: ActionCx, args: Fields) -> BoxFut<'static, ActionResult> {
        self.counted(cx.run().to_string(), self.real.run(cx, args))
    }

    fn resume(&self, cx: ActionCx, args: Fields, saved: Value) -> BoxFut<'static, ActionResult> {
        self.counted(cx.run().to_string(), self.real.resume(cx, args, saved))
    }
}

/// The registry of a sandbox whose tables are `studio`'s, on a device
/// whose actions say `described` of themselves and whose events say
/// `events` (all of them, where `whole`), with `chosen` as what answers
/// for each action that reaches outside the tables, and `working` and
/// `doing` as where each action that is the real one counts itself at
/// work and writes down which run it was done for.
pub(crate) fn offered(
    studio: &Studio,
    described: &[ActionSpec],
    events: &[EventSpec],
    whole: bool,
    chosen: &Chosen,
    (working, doing): (&Working, &Arc<Doing>),
) -> Offered {
    // The real ones are registered as their crates register them, and
    // then each is put in the sandbox's registry counted.
    let mut real = Registry::new();
    let mut kinds: BTreeMap<String, Kind> =
        blueprint::intrinsics().into_iter().map(|spec| (spec.name, Kind::Own)).collect();
    blocks::register(&mut real);
    deciding::register(&mut real, |_| {});
    hosting::register_blocks(&mut real);
    note(&real, &mut kinds, Kind::Own);
    studio.register_kept(&mut real);
    // What reads a card's history and asks a block of it reads the sandbox's own.
    deciding::timed::register_reading(&mut real, studio);
    note(&real, &mut kinds, Kind::Kept);
    let mut registry = Registry::new();
    for spec in real.specs().specs() {
        if let Some(action) = real.get(&spec.name) {
            registry.add(Counted { real: action, working: working.clone(), doing: doing.clone() });
        }
    }
    for spec in described {
        if kinds.contains_key(&spec.name) {
            continue;
        }
        // What answers for it is chosen in one place (`crate::outside`).
        let (kind, answers) = chosen.answers_for(spec);
        kinds.insert(spec.name.clone(), kind);
        let described = Arc::new(spec.clone());
        registry.add_fn(spec.clone(), move |cx, args| -> BoxFut<'static, ActionResult> {
            answers.clone().ask(described.clone(), cx, args)
        });
    }
    // What is heard here is what is heard on the device, so that a
    // handler which checks there checks here: the runner's own events
    // are in a new registry already.
    let known: BTreeSet<String> = registry.own_events().iter().map(|event| event.topic.clone()).collect();
    registry.emits(events.iter().filter(|event| !known.contains(&event.topic)).cloned().collect());
    if whole {
        registry.emits_no_more();
    }
    Offered { registry, kinds }
}
