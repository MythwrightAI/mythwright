//! What of a text will hear nothing where it is applied: a handler or a
//! `wait` that says no `scope`, and so hears its own project's events and
//! what is nobody's, applied where what it listens for can never be either.
//!
//! It is a function of the text, the project it is applied for and what
//! each event says of itself, and is said to whoever applies the text
//! (`unheard`, a warning of an apply and of a check), since nothing else
//! would say so until somebody wondered why nothing ran. The runner offers
//! it where it always was (`conductor::whose::unheard`).

use serde_json::Value;

use crate::EventSpec;
use crate::events::matches;
use crate::model::{Blueprint, Flow, Step, StepKind, TriggerScope, TriggerSource};
use crate::syntax::Span;

/// The word a `wait` is given for hearing its own project, which is what
/// it hears where it says none.
const OWN: &str = "project";

/// Something of a text that will hear nothing where the text is applied.
pub struct Unheard {
    /// The handler, or the step that waits.
    pub name: String,
    /// Where it is written.
    pub span: Span,
    /// Why, and what to do about it, for whoever applied the text.
    pub said: String,
}

/// Why a handler or a `wait` of a text applied for `project` will hear
/// nothing of what it listens for, where it says no `scope` and so hears
/// its own project's and what is nobody's: said to whoever applies it,
/// since nothing else would say so until somebody wondered why nothing
/// ran.
///
/// Two ways it comes about. The text is applied for no project and the
/// topic is always some project's (every described event it matches
/// always carries `project`): what is in no project hears what is
/// nobody's. Or a handler names, written out in `for:`, a project that
/// is not the one the text is applied for: naming another project is
/// not saying that it is heard.
pub fn unheard(blueprint: &Blueprint, project: Option<&str>, described: &[EventSpec]) -> Vec<Unheard> {
    let cure = "Apply it for that project (`--project`), or have the handler say `scope: fleet`, which hears \
                every project's: a handler that names another project in `for:` hears it only by saying so";
    let mut found = Vec::new();
    for trigger in &blueprint.triggers {
        let TriggerSource::Event(pattern) = &trigger.source else { continue };
        if trigger.scope != TriggerScope::Project {
            continue;
        }
        let named = trigger
            .address
            .get("project")
            .and_then(Value::as_str)
            .filter(|named| !named.is_empty() && !named.contains("${"));
        let why = match (project, named) {
            (own, Some(named)) if own != Some(named) => Some(format!(
                "`{}` names the project `{named}` in `for:`, and this is applied for {}",
                trigger.name,
                own.map_or("no project".to_string(), |own| format!("`{own}`")),
            )),
            (None, _) if always_a_projects(pattern, described) => Some(format!(
                "this is applied for no project, and `{pattern}`, which `{}` listens for, is always some \
                 project's",
                trigger.name
            )),
            _ => None,
        };
        if let Some(why) = why {
            let said = format!(
                "{why}. A handler that says no `scope` hears its own project's events and what is no \
                 project's, so this one will hear nothing. {cure}"
            );
            found.push(Unheard { name: trigger.name.clone(), span: trigger.span, said });
        }
    }
    // A step that waits for an event hears as a handler does, and has no
    // `for:` to name a project in.
    if project.is_none() {
        let mut waits = Vec::new();
        waiting(&blueprint.flow, &mut waits);
        blueprint.triggers.iter().for_each(|trigger| waiting(&trigger.flow, &mut waits));
        for step in waits {
            let text = |key: &str| step.with.get(key).and_then(Value::as_str);
            let Some(topic) = text("event").filter(|topic| !topic.contains("${")) else { continue };
            if text("scope").is_some_and(|scope| scope != OWN) || !always_a_projects(topic, described) {
                continue;
            }
            let said = format!(
                "this is applied for no project, and `{topic}`, which `{}` waits for, is always some \
                 project's. A `wait` that says no `scope` hears its own project's events and what is no \
                 project's, so this one will hear nothing. Apply it for that project (`--project`), or \
                 have the wait say `scope: fleet` among what it is given, which hears every project's",
                step.id
            );
            found.push(Unheard { name: step.id.clone(), span: step.span, said });
        }
    }
    found
}

/// The steps of a flow that wait, those inside steps done as one among them.
fn waiting<'a>(flow: &'a Flow, found: &mut Vec<&'a Step>) {
    for step in &flow.steps {
        if matches!(&step.kind, StepKind::Do(action) if action == "wait") {
            found.push(step);
        }
        waiting(&step.inner, found);
    }
}

/// Whether everything a pattern can match is always some project's: it
/// matches a described event, and every one it matches always carries
/// `project`. One a blueprint says, which nobody described, may be
/// anybody's.
fn always_a_projects(pattern: &str, described: &[EventSpec]) -> bool {
    let any =
        |topic: &str| pattern.split(',').map(str::trim).any(|one| !one.is_empty() && matches(one, topic));
    let mut matched = described.iter().filter(|spec| any(&spec.topic)).peekable();
    matched.peek().is_some()
        && matched.all(|spec| spec.fields.iter().any(|field| field.name == "project" && field.always))
}
