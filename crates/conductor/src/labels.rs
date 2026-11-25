//! Labels, carried: how what a blueprint says of the work it starts reaches
//! every measurement of that work.
//!
//! A step that is a `call`, a `spawn` or a task may say `labels:`. They are
//! worked out when the step starts and added to the labels the step is
//! itself under, and from there they are carried down:
//!
//! - into the instance a `call` or `spawn` makes, where they are kept with
//!   it. Everything the instance does is measured with them for as long as
//!   it lives, however long after the step that made it finished and
//!   whatever restarted in between: its steps, its runs, its handlers'
//!   runs, its polls, and the children it starts in turn;
//! - into the steps of a task, for as long as the task runs;
//! - to every action ([`crate::ActionCx::labels`]), so that what an action
//!   has done elsewhere, such as an agent's turn, is measured with them too.
//!
//! The step that says `labels:` is not itself under them: it is measured as
//! a step of the blueprint it is written in.

pub(crate) use blueprint::labels::beside;
use blueprint::{Labels, Step};
use kernel::{Fields, Result};
use runs::InstanceInfo;

use crate::action::ActionError;
use crate::engine::Engine;
use crate::run::RunCx;

/// The labels everything under `step` carries: those the run is under,
/// and the step's own, worked out in `scope`.
pub(crate) fn under(cx: &RunCx, step: &Step, scope: &Fields) -> std::result::Result<Labels, ActionError> {
    blueprint::labels::under(&cx.labels, &step.labels, scope)
        .map_err(|err| ActionError::new(format!("`labels` could not be filled in: {err}")))
}

/// Measures a step that has ended, with the labels it is under.
pub(crate) fn measure(cx: &RunCx, step: &Step, labels: &[(&str, String); 3], duration_ms: i64, ok: bool) {
    // Whose it is comes first, so that it is on both measurements below:
    // "how is this project doing" is then the same question of each.
    let project = cx.live.meta().project.unwrap_or_default();
    let mut all: Vec<(&str, &str)> = vec![("project", project.as_str())];
    all.extend(labels.iter().map(|(key, value)| (*key, value.as_str())));
    let journal = cx.engine.own();
    journal.measure("step", &beside(&all, &cx.labels), duration_ms as f64, ok);
    // `measure: build` reports the step a second time under its own name, so
    // "how long do builds take" is one query whatever the step is called.
    if let Some(name) = &step.measure {
        journal.measure(name, &beside(&all[..3], &cx.labels), duration_ms as f64, ok);
    }
}

impl Engine {
    /// Gives an instance that is applied again the labels it is applied
    /// with this time, whether or not anything else about it changed: what
    /// it does from here on is measured with them.
    pub(crate) async fn relabel(&self, existing: &mut InstanceInfo, labels: Labels) -> Result<()> {
        if existing.labels == labels {
            return Ok(());
        }
        self.inner.store.keep_labels(&existing.id, &labels, self.inner.own.now_ms()).await?;
        if let Some(live) = self.live(&existing.id) {
            live.meta.lock().expect("live lock").labels = labels.clone();
        }
        existing.labels = labels;
        Ok(())
    }
}
