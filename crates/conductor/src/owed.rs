//! Whether what the runner says of its own steps and runs has to be kept.
//!
//! What the runner says of a step and a run (that it started, finished,
//! failed, was tried again, waits, was passed over or cancelled; that a
//! run started, was taken up, failed or finished) is said for every step
//! that is run, two and more each, and is half of everything a factory
//! says. Nothing reads it back from the journal: what a step and a run
//! came to is in their rows, which every page and command reads, and
//! what a restart needs to know of where a step or a run began is
//! written on its row ([`runs::Say::begins_step`], `Store::begin_run`).
//! So each is passed on to whoever is watching, in its order and with
//! its number, and has a place in the journal only where something could
//! come to need it there: a handler that hears it, or a step that waits
//! for it, which reads from its place in the journal when it is next
//! listening.
//!
//! That is decided by what is certain and never by who happens to be
//! listening at the instant something is said. A handler is not on
//! the index while its instance is being taken up after a restart or
//! given another text, and a `wait` is not counted until its step has
//! begun again, yet each is owed everything said since its place. So it
//! is read from the texts in force: while any instance there is runs a
//! text with a handler or a `wait` that a topic could be for, every one
//! on that topic is kept. A topic written with a hole in it cannot be
//! read off the text and is taken to be for all of them. And while
//! everything is being taken up again at a start, when not every text is
//! in force yet, all are kept.

use kernel::Source;
use kernel::event::topic_matches_any;
use serde_json::{Value, json};

/// What a step that was passed over says.
pub(crate) const SKIPPED: &str = "step.skipped";

/// What the runner says of its own steps and runs: each kept only where
/// a text in force could be owed it, and otherwise passed on.
pub const OWN: [&str; 11] = [
    "run.started",
    "run.resumed",
    "run.failed",
    "run.finished",
    "step.started",
    "step.finished",
    "step.failed",
    "step.retrying",
    SKIPPED,
    "step.cancelled",
    "step.waiting",
];

pub use texts::ran::Keeping;
/// How many instances there are whose text could be owed each of
/// [`OWN`], and whether everything is being taken up again, is counted
/// as texts are taken ([`texts::ran::Owed`]): here under the names it
/// always had, made with [`OWN`].
pub(crate) use texts::ran::{Hearing, Owed, heard_by};

impl crate::Engine {
    /// Has what is said on `topics` (patterns, as a handler's are) that is
    /// one of [`OWN`] kept for as long as what is given back is held: for
    /// whatever reads it back from the journal from a place of its own (a
    /// comparison's count), which is owed it as a handler is.
    pub fn keeps_said(&self, topics: &[String]) -> Keeping {
        let named = |own: &str| {
            topics.iter().any(|pattern| pattern.contains("${") || topic_matches_any(pattern, own))
        };
        self.inner.owed.keeping(named)
    }

    /// Something the runner says of its own steps and runs, made ready to
    /// be said with what is written: kept where a text in force could be
    /// owed it, and otherwise passed on in its order ([`Owed`]).
    pub(crate) fn own_say(&self, topic: &'static str, source: Source, data: Value) -> runs::Say {
        runs::to_say(topic, source, data).kept(self.inner.owed.keeps(topic))
    }

    /// Has this conductor keep everything it says of its own steps and
    /// runs, whatever hears it: for a factory whose journal is read back
    /// as what a person is shown of what ran (a sandbox), and for no
    /// factory that is run.
    pub fn keeps_all_it_says(&self) {
        self.inner.owed.keep_all();
    }

    /// Says that a run began (`run.started`) or was taken up
    /// (`run.resumed`). Where a run's beginning is only passed on, its
    /// row is written as begun with it, since that it began is what tells
    /// a run taken up with nothing else written from one that never began
    /// (`run::said_started`); kept, the journal says so.
    pub(crate) async fn begin_run(&self, cx: &crate::run::RunCx, topic: &'static str) {
        let data = json!({ "flow": cx.flow_id, "blueprint": cx.blueprint.name });
        let say = self.own_say(topic, cx.about(None), data);
        match !say.is_kept() && topic == "run.started" && cx.kept() {
            true => {
                let _ = self.inner.store.begin_run(&cx.run_id, vec![say]).await;
            }
            false => say.tell(self.own()).await,
        }
    }
}
