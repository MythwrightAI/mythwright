//! What the steps of a run left under way come to, where nothing will
//! take the run up: its instance is over.
//!
//! The runner ends such a run as it would have ended itself, in one
//! write (`Engine::end_left` there): each step that had not settled is
//! written as cancelled, with what is said of it, and so is each step it
//! never came to. Which steps those are, and what each is written as, is
//! a function of the text and of the steps written, and is here; it came
//! out of the runner, which asks it under the names it always gave
//! (`crate::left` there).

use std::collections::{BTreeSet, HashMap};

use blueprint::{Blueprint, Flow, within};
use kernel::Source;
use serde_json::{Value, json};

use crate::{RunInfo, Say, StepInfo, StepStatus, to_say};

/// The steps of the run called `flow` in a text: its own, or a handler's.
/// Nothing, for what is done in place of steps as a text is taken.
pub fn flow_of<'a>(text: &'a Blueprint, flow: &str) -> Option<&'a Flow> {
    match flow.strip_prefix("on.") {
        Some(name) => text.triggers.iter().find(|trigger| trigger.name == name).map(|trigger| &trigger.flow),
        None if flow == "main" => Some(&text.flow),
        None => None,
    }
}

/// What each step of a run that had not settled is written as, and what
/// is said of it. A step the run had passed over, whose record was held
/// and not yet written when the conductor stopped, is among those it
/// never came to: what was held said nothing that a conductor stopped a
/// moment sooner would have known, and it reads as cancelled.
///
/// Of each: one that had begun is cancelled and says so, and that
/// it finished; one the run never came to is cancelled and says no more.
/// A step inside steps done as one is looked at only where those had
/// begun: what is inside steps that never began was never there.
pub fn cancelled(
    run: &RunInfo,
    flow: Option<&Flow>,
    now: i64,
    source: &dyn Fn(Option<&str>) -> Source,
) -> (Vec<StepInfo>, Vec<Say>) {
    let had: HashMap<&str, &StepInfo> = run.steps.iter().map(|step| (step.step.as_str(), step)).collect();
    let mut never = Vec::new();
    if let Some(flow) = flow {
        unbegun(flow, "", &had, &mut never);
    }
    let (mut steps, mut says) = (Vec::new(), Vec::new());
    // Those that had begun, as they were written: a text taken since may
    // no longer have one of them, and it was under way all the same.
    for step in run.steps.iter().filter(|step| !step.status.is_settled()) {
        let said = json!({
            "attempt": step.attempt,
            "duration_ms": now - step.started,
            "status": StepStatus::Cancelled.as_str(),
        });
        says.push(to_say("step.cancelled", source(Some(&step.step)), json!({ "attempt": step.attempt })));
        says.push(to_say("step.finished", source(Some(&step.step)), said));
        steps.push(StepInfo {
            status: StepStatus::Cancelled,
            output: Value::Null,
            error: None,
            saved: Value::Null,
            finished: Some(now),
            ..step.clone()
        });
    }
    let written: BTreeSet<&str> = run.steps.iter().map(|step| step.step.as_str()).collect();
    for name in never.into_iter().filter(|name| !written.contains(name.as_str())) {
        says.push(to_say("step.cancelled", source(Some(&name)), json!({})));
        steps.push(StepInfo {
            run: run.id.clone(),
            step: name,
            status: StepStatus::Cancelled,
            attempt: 0,
            tolerated: false,
            output: Value::Null,
            error: None,
            saved: Value::Null,
            started: now,
            finished: Some(now),
        });
    }
    (steps, says)
}

/// The names of the steps of `flow` that the run never came to, in the
/// order they are written; and inside steps done as one that had begun
/// and not settled, those of theirs it never came to.
pub fn unbegun(flow: &Flow, before: &str, had: &HashMap<&str, &StepInfo>, never: &mut Vec<String>) {
    for step in &flow.steps {
        let name = within::name(before, &step.id);
        match had.get(name.as_str()) {
            None => never.push(name),
            Some(written) if !written.status.is_settled() && !step.inner.steps.is_empty() => {
                unbegun(&step.inner, &within::task(&name), had, never);
            }
            Some(_) => {}
        }
    }
}
