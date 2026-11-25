//! A task: a step whose work is a graph of steps of its own.
//!
//! Nothing about a step is different for being inside a task. Each is run
//! by the scheduler that runs a blueprint's own steps, is recorded under the
//! task's name and its own (`build.compile`), and says when it starts and
//! how it ends. What the task adds is that they are one thing to whatever is
//! around them: the task is a step like any other, so its guard, timeout,
//! retries and hooks are a step's, said once for all of them, and it has
//! failed when one of them has.

use std::sync::Arc;

use blueprint::{Step, within};
use kernel::{BoxFut, Cancel, Fields};
use serde_json::Value;

use crate::action::ActionError;
use crate::labels;
use crate::run::{Halt, Place, RunCx, run_steps};
use crate::step::Done;

/// Runs the steps of the task `step` once through. What it gives out is
/// what each of them gave out, by name.
///
/// The future is boxed because a task is run by a step and runs steps:
/// without it the type of "running a step" would contain itself.
pub(crate) fn run<'a>(
    cx: &'a RunCx,
    step: &'a Step,
    scope: &'a Fields,
    cancel: Cancel,
    resuming: bool,
    attempt: u32,
    element: Option<usize>,
) -> BoxFut<'a, Result<Done, ActionError>> {
    Box::pin(async move {
        let prefix = within::task(&step.id);
        let store = &cx.engine.inner.store;
        let prior = if resuming {
            store.run(&cx.run_id).await?.map(|run| run.steps).unwrap_or_default()
        } else {
            // A task that is run again starts from its first step. What its
            // steps did last time is not how this time is going, and after
            // a restart would be taken for it.
            cx.forget_steps(&prefix).await?;
            Vec::new()
        };
        let outside = match scope.get("steps") {
            Some(Value::Object(steps)) => steps.clone(),
            _ => Fields::new(),
        };
        let each = ["item", "index"]
            .into_iter()
            .filter_map(|name| Some((name.to_string(), scope.get(name)?.clone())));
        let place = Place { prefix, outside, each: each.collect() };
        // One step failing stops the others by cancelling this, and that
        // must not reach the rest of the run: the task may be tried again.
        let mut inside = cx.within(cancel.child());
        inside.labels = labels::under(cx, step, scope)?;
        // Its steps have the same names each time it is done: which try
        // of it this is, and for which of its list, is what tells them.
        let of = element.map_or(String::new(), |element| format!(" of {element}"));
        inside.under = format!("{}{} try {attempt}{of} > ", cx.under, step.id);
        let inside = Arc::new(inside);
        let steps = run_steps(&inside, &step.inner, &place, prior).await;

        let mut given = Fields::new();
        for inner in &step.inner.steps {
            if let Some(result) = steps.settled.get(&inner.id) {
                given.insert(inner.id.clone(), result.output.clone());
            }
        }
        match steps.halt {
            Some(Halt::Ended(end)) => Ok(Done::End(end)),
            // What the steps before the failure gave out is still worth
            // reading, by a hook or by what follows a tolerated failure.
            Some(Halt::Failed(error)) => Err(ActionError::new(error).with_data(Value::Object(given))),
            None if inside.cancel.is_cancelled() => Err(ActionError::cancelled()),
            None => Ok(Done::Out(Value::Object(given))),
        }
    })
}
