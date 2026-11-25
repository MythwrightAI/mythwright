//! A run as it is handed to somebody.
//!
//! A run keeps the whole event that set its handler off, and what each
//! of its steps gave. The handler heard that event as whoever applied
//! its text, who may read more than whoever now reads the run: a record
//! of a table kept private is in the runs of a handler that heard it
//! change. So a run is handed out by the question an event is
//! ([`Inner::may_hear`]): whole to whoever may be told what set it off,
//! and to anybody else without what it kept of that: what the event
//! carried, what the run and each step gave and why any of them failed.
//! That it ran, when, for which handler, by which steps and how each
//! ended is still theirs to read, with a word that says the rest is
//! kept from them ([`KEPT_FROM`]).
//!
//! Every route that hands a run out hands it out through [`run`] or
//! [`runs`], and a step's log is read only where [`Inner::tells_of`]
//! says so; a test of the suite reads the routes for one that does not.
//!
//! What the runner says of such a run as it goes is held where it is
//! said: it takes what the event took (`RunCx::about` in the runner),
//! and is handed out as any event is.
//!
//! What is not held here is what a blueprint does with what it heard: an
//! instance's state, what it started and with which params, and what its
//! own events say (an `emit` step, an action's) are its applier's to
//! have said.

use conductor::RunInfo;
use kernel::{Event, Principal, Source};
use serde_json::{Value, json};

use crate::state::Inner;

/// What a run says where something of it is kept from whoever reads it.
pub const KEPT_FROM: &str = "kept_from";

/// What `run` keeps as the event that set it off, as far as who may be
/// told of it goes: its topic and where it came from. Nothing for a run
/// no event set off: the run of an instance's steps, and one a handler
/// began for what it read itself on a timer, which was said to nobody
/// and has no place in any journal. And no answer where what it keeps
/// cannot be read as where an event came from.
fn set_off_by(run: &RunInfo) -> Result<Option<Event>, ()> {
    let Some(seq) = run.event.get("seq").and_then(Value::as_u64).filter(|seq| *seq != 0) else {
        return Ok(None);
    };
    let topic = run.event.get("topic").and_then(Value::as_str).unwrap_or_default().to_string();
    let source = match run.event.get("source") {
        None | Some(Value::Null) => Source::default(),
        Some(said) => serde_json::from_value(said.clone()).map_err(|_| ())?,
    };
    Ok(Some(Event { seq, ts: 0, topic, source, data: Value::Null }))
}

impl Inner {
    /// Whether `who` reads all of `run`: whether they may be told the
    /// event that set it off, where one did. What cannot be read is not
    /// taken for nothing having been said: nobody is told it.
    pub fn tells_of(&self, who: &Principal, run: &RunInfo) -> bool {
        match set_off_by(run) {
            Ok(None) => true,
            Ok(Some(event)) => self.may_hear(who, &event),
            Err(()) => false,
        }
    }
}

/// `run` as `who` is handed it.
pub fn run(app: &Inner, who: &Principal, mut run: RunInfo) -> Value {
    let mut handed = whole_or_not(app, who, &mut run);
    // What its steps printed may have been let go after so many days.
    if let Some(said) = app.engine.let_go_said(run.finished) {
        handed["let_go"] = json!(said);
    }
    handed
}

fn whole_or_not(app: &Inner, who: &Principal, run: &mut RunInfo) -> Value {
    if app.tells_of(who, run) {
        return json!(run);
    }
    if let Some(event) = run.event.as_object_mut() {
        event.remove("data");
    }
    (run.outputs, run.error) = (Value::Null, None);
    for step in &mut run.steps {
        (step.output, step.error) = (Value::Null, None);
    }
    let mut handed = json!(run);
    handed[KEPT_FROM] = json!(true);
    handed
}

/// Each of `runs` as `who` is handed it.
pub fn runs(app: &Inner, who: &Principal, runs: Vec<RunInfo>) -> Vec<Value> {
    runs.into_iter().map(|one| run(app, who, one)).collect()
}
