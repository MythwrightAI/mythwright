//! `flag.set`: a project's feature flag said in one of its environments by
//! a blueprint (`std/flags`, once a change that waited for a go-ahead was
//! let go on, and when a flag's row is taken away), as the route that
//! changes a flag says it for whoever asks.
//!
//! What a flag says is kept in the environment's variables, written with
//! what is said of it as one write (`Engine::set_flag`, over
//! `environments::flagged`). This is only the asking, for the project the
//! instance is applied for and no other: what it is handed is read as a
//! value of the flag's kind (`kernel::flags::said`), so that nothing but
//! `on`, `off`, a percentage or a plain word is ever written as a flag.
//! That is why writing one takes writing the project and no more, where
//! a variable of the environment written by itself takes `shell` and
//! `secrets:use` too: what a flag says reaches a command only as one of
//! those.

use blueprint::{ActionSpec, ValueType};
use conductor::environs::Flagging;
use conductor::{ActionError, ActionResult, Registry};
use kernel::flags::{self, Kind};
use kernel::{BoxFut, Error};
use serde_json::{Value, json};

/// Adds `flag.set` to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    let spec = ActionSpec::new(
        "flag.set",
        "Say what a feature flag of the project this is applied for says in one of its environments, or take it \
         away there: the environment's variable of the flag, written with what is said of it.",
    )
    .example(SET_ASKED)
    .required("flag", ValueType::String, "the flag, by its name, such as `new-checkout`")
    .required("record", ValueType::String, "the flag's row of the project's `flags`, by its id")
    .arg("kind", ValueType::String, "what kind of value it says: `switch`, `percentage` or `word` (default: switch)")
    .arg(
        "environment",
        ValueType::String,
        "the environment it is said in; where it is empty, every environment the project has, which is only \
         for taking it away",
    )
    .arg(
        "value",
        ValueType::Any,
        "what it says: `on` or `off`, a whole number from 0 to 100, or a word; nothing to take it away",
    )
    .arg("asked_by", ValueType::String, "who asked for it, as an actor, where that is not whoever applied this")
    .arg("asked_by_name", ValueType::String, "who asked for it, by name")
    .arg("approved_by", ValueType::String, "who let it go on, as an actor, where it waited for a go-ahead")
    .arg(
        "asked",
        ValueType::String,
        "where it waited for a go-ahead, the change asked, by its id: it is written only while it is the change \
         asked last of the flag there and the flag says `was` still, and otherwise nothing is written",
    )
    .arg("was", ValueType::Any, "with `asked`, what the flag said when it was asked; nothing, where it said nothing")
    .capability("projects:write")
    .output("written", "the environments it was written in now")
    .output("there", "the environments that said it already")
    .output("unchanged", "whether a change asked for (`asked`) was overtaken, by a later change of the flag or a later ask, and wrote nothing")
    .idempotent();
    registry.add_fn(spec, |cx, args| -> BoxFut<'static, ActionResult> {
        Box::pin(async move {
            let project = cx.project().map(str::to_string).ok_or_else(|| {
                Error::invalid(
                    "`flag.set` says a project's flag, and this is applied for no project: apply it with `--project`",
                )
            })?;
            let text = |key: &str| args.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
            let (flag, record) = (text("flag"), text("record"));
            if let Some(why) = flags::refused_name(&flag) {
                return Err(ActionError::from(Error::invalid(why)));
            }
            let kind = Kind::read(&text("kind"));
            let value = match args.get("value") {
                None | Some(Value::Null) => None,
                Some(asked) => Some(flags::said(kind, asked).map_err(|why| ActionError::from(Error::invalid(why)))?),
            };
            let engine = cx.engine();
            let environments = match text("environment") {
                every if every.is_empty() && value.is_none() => {
                    engine.environments_of(&project).await?.into_iter().map(|one| one.name).collect()
                }
                every if every.is_empty() => {
                    return Err(ActionError::from(Error::invalid(
                        "`flag.set` says a flag in one environment: only taking it away is done in every one at once",
                    )));
                }
                one => vec![one],
            };
            let asked_by = Some(text("asked_by")).filter(|by| !by.is_empty()).map(|by| (by, text("asked_by_name")));
            let approved_by = Some(text("approved_by")).filter(|by| !by.is_empty());
            let was = args.get("was").filter(|was| !was.is_null()).cloned();
            let expected = Some(text("asked")).filter(|asked| !asked.is_empty());
            let mut unchanged = false;
            let (mut written, mut there) = (Vec::new(), Vec::new());
            for environment in environments {
                let flagging = Flagging {
                    project: &project,
                    environment: &environment,
                    flag: &flag,
                    record: &record,
                    kind,
                    value: value.clone(),
                    asked_by: asked_by.clone(),
                    approved_by: approved_by.clone(),
                    expected: expected.clone().map(|asked| conductor::environs::Expected { asked, was: was.clone() }),
                };
                let flagged = engine.set_flag(flagging, cx.principal(), cx.source()).await?;
                unchanged |= flagged.unchanged;
                match flagged.written {
                    true => written.push(environment),
                    false => there.push(environment),
                }
            }
            Ok(json!({ "written": written, "there": there, "unchanged": unchanged }))
        })
    });
}

// What the action is shown by in the Reference and the editor: a whole
// blueprint that asks it, held to checking with no warning.
const SET_ASKED: &str = "\
blueprint: dark-mode-on
permissions: [projects]

steps:
  dark:
    do: flag.set
    with: { flag: dark-mode, record: shop-flags-1, environment: dev, value: \"on\" }
";
