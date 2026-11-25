//! A project's feature flag, said in one of its environments:
//! the flag's variable of that environment (`FLAG_<NAME>`,
//! `kernel::flags::variable`) written, and what is said of it, as one
//! write, as every change of an environment is (`crate::kept`).
//!
//! What a flag says in an environment is kept there and nowhere else, so
//! that an app the project deploys reads it as any setting and a
//! blueprint run under the environment as `environment.vars`. The write
//! says two things: that the environment's variables changed
//! (`project.environment.changed`, by name, as any change says it), and
//! that the flag did (`project.flag.changed`), naming the flag's row so
//! that its history keeps the change, with what it was and what it is.
//! A variable of a flag keeps whatever mark for agents it had; one that
//! was not set is not handed to agents, as no variable is unless somebody
//! says so.
//!
//! **An approval never undoes a later change.** A change that waits for a
//! go-ahead is kept on the environment as it is asked ([`ask`]: its id,
//! by the flag's variable, in the write that says `project.flag.asked`
//! with what the flag said then), and is written once approved only
//! where, as that write reads the environment, it is still the change
//! asked last of that flag there and the flag says what it said then
//! ([`Expected`]). Every change of the flag written since takes it away,
//! a turning off that changes nothing among them, and a newer ask takes
//! its place, so two approvals cannot land out of order. Where it is not
//! so, nothing is written, and that is said (`project.flag.unchanged`,
//! on the flag's history).
//!
//! Whether whoever asks may, and whether it waits for somebody the
//! environment's rule names, is whoever asks's to have settled
//! (`kernel::flags::change`); what is here is the writing.

use journal::Journal;
use kernel::flags::{self, Kind};
use kernel::{Principal, Result, Source};
use serde_json::{Value, json};

use crate::held::Held;
use crate::kept::{changed_said, missing, one};
use crate::project::{Changed, Environment, NAMESPACE, Variable, key};

/// A flag of a project said in one of its environments.
pub struct Flagging<'a> {
    pub project: &'a str,
    pub environment: &'a str,
    /// The flag's name, as its row says it, and its row, by its id.
    pub flag: &'a str,
    pub record: &'a str,
    pub kind: Kind,
    /// What it is to say there, as it is kept (`kernel::flags::said`); or
    /// nothing, for its variable taken away.
    pub value: Option<Value>,
    /// Who asked, as an actor and by name, where that is not whoever
    /// writes it (a change that waited for a go-ahead is written by what
    /// looks after the project's flags), and who let it go on.
    pub asked_by: Option<(String, String)>,
    pub approved_by: Option<String>,
    /// Where it is a change that waited: what has to be so still for it to
    /// be written.
    pub expected: Option<Expected>,
}

/// What a change that waited for a go-ahead is written only where it is
/// still so: it is the change asked last of the flag there, and the flag
/// says what it said when it was asked.
pub struct Expected {
    pub asked: String,
    pub was: Option<Value>,
}

/// What it came to: the environment as it is now, and what the flag said
/// there before, where it said anything. Nothing was written where it
/// says that already and nothing waited, or where it waited and was
/// overtaken (`unchanged`).
pub struct Flagged {
    pub environment: Environment,
    pub was: Option<Value>,
    pub written: bool,
    pub unchanged: bool,
}

/// What is said of a flag in the writes here: who, what, and of what row.
fn said_of(flagging: &Flagging<'_>, variable: &str, by: &Principal) -> Value {
    let mut said = json!({
        "project": flagging.project,
        "environment": flagging.environment,
        "flag": flags::key(flagging.flag),
        "name": flagging.flag,
        "record": flagging.record,
        "variable": variable,
        "value": flagging.value,
        "shown": flags::shown(flagging.kind, flagging.value.as_ref().unwrap_or(&flags::off(flagging.kind))),
        // A key that holds less is itself, never its maker.
        "by": by.key.as_ref().map_or_else(|| by.actor(), |key| format!("key:{key}")),
        "by_name": by.name,
    });
    if let Some((actor, name)) = &flagging.asked_by {
        said["asked_by"] = json!(actor);
        said["asked_by_name"] = json!(name);
    }
    if let Some(approved) = &flagging.approved_by {
        said["approved_by"] = json!(approved);
    }
    said
}

/// Says `flagging`'s value in its environment, as `by`, with what is said
/// of it, as one write; and holds it for what runs there. A change that
/// waited is written only where what it expects is still so, and is said
/// otherwise to have written nothing.
pub async fn set(
    (journal, held): (&Journal, &Held),
    flagging: Flagging<'_>,
    by: &Principal,
    source: Source,
) -> Result<Flagged> {
    let _turn = held.changing().await;
    let (project, name) = (flagging.project, flagging.environment);
    let mut environment = one(journal, project, name).await?.ok_or_else(|| missing(project, name))?;
    let variable = flags::variable(flagging.flag);
    let was = environment.vars.get(&variable).map(|var| var.value.clone());
    let waiting = environment.asked.get(&variable).cloned();
    if let Some(expected) = &flagging.expected {
        let still = waiting.as_deref() == Some(expected.asked.as_str()) && was == expected.was;
        if !still {
            let mut said = said_of(&flagging, &variable, by);
            said["was"] = json!(expected.was);
            said["now"] = json!(was);
            said["overtaken"] = json!(match waiting.as_deref() {
                Some(other) if other != expected.asked => "asked again",
                _ => "changed",
            });
            journal.write(move |writing| writing.say("project.flag.unchanged", source, said)).await?;
            return Ok(Flagged { environment, was, written: false, unchanged: true });
        }
    }
    if was == flagging.value && waiting.is_none() {
        return Ok(Flagged { environment, was, written: false, unchanged: false });
    }
    // Whatever is written of the flag takes away the change that waited:
    // an approval of it would undo this.
    environment.asked.remove(&variable);
    let mut changed = Changed::default();
    match (&flagging.value, environment.vars.get_mut(&variable)) {
        (None, Some(_)) => {
            environment.vars.remove(&variable);
            changed.removed.push(variable.clone());
        }
        (None, None) => {}
        (Some(now), Some(var)) => {
            if var.value != *now {
                var.value = now.clone();
                changed.changed.push(variable.clone());
            }
        }
        (Some(now), None) => {
            environment.vars.insert(variable.clone(), Variable { value: now.clone(), agents: false });
            changed.set.push(variable.clone());
        }
    }
    let mut said = said_of(&flagging, &variable, by);
    said["was"] = json!(was);
    let (at, kept) = (key(project, name), environment.kept());
    let changed_said = (!changed.is_empty()).then(|| changed_said(project, name, &changed, by));
    let flag_source = source.clone();
    let write = journal.write(move |writing| {
        writing.keep(NAMESPACE, &at, &kept)?;
        if let Some(changed_said) = changed_said {
            writing.say("project.environment.changed", source, changed_said)?;
        }
        writing.say("project.flag.changed", flag_source, said)
    });
    write.await?;
    held.put(project, &environment);
    Ok(Flagged { environment, was, written: true, unchanged: false })
}

/// Keeps a change of a flag that waits for a go-ahead as the one asked
/// last of it there, by its id (`asked`), and says it is asked
/// (`project.flag.asked`, with what the flag says now), as one write.
/// It takes the place of any asked before it.
pub async fn ask(
    (journal, held): (&Journal, &Held),
    flagging: Flagging<'_>,
    asked: &str,
    by: &Principal,
    source: Source,
) -> Result<Flagged> {
    let _turn = held.changing().await;
    let (project, name) = (flagging.project, flagging.environment);
    let mut environment = one(journal, project, name).await?.ok_or_else(|| missing(project, name))?;
    let variable = flags::variable(flagging.flag);
    let was = environment.vars.get(&variable).map(|var| var.value.clone());
    environment.asked.insert(variable.clone(), asked.to_string());
    let mut said = said_of(&flagging, &variable, by);
    said["was"] = json!(was);
    said["asked"] = json!(asked);
    said["kind"] = json!(flagging.kind.word());
    // Who asked is whoever writes this: said once, as `by`.
    if let Some(object) = said.as_object_mut() {
        object.remove("asked_by");
        object.remove("asked_by_name");
        object.remove("approved_by");
    }
    let (at, kept) = (key(project, name), environment.kept());
    let write = journal.write(move |writing| {
        writing.keep(NAMESPACE, &at, &kept)?;
        writing.say("project.flag.asked", source, said)
    });
    write.await?;
    held.put(project, &environment);
    Ok(Flagged { environment, was, written: true, unchanged: false })
}
