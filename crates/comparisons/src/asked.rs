//! A comparison started and ended by a blueprint: `compare.start` and
//! `compare.end`; a thing written down as one of its ways' with nothing
//! started for it, `compare.went`; and the shares of one that shares its
//! things out said again, `compare.share`.
//!
//! What a blueprint starts is a comparison whose ways are named by the
//! start (`kernel::compared::named`): it speaks only for a start whose
//! params say `compared` (its name) and `way` (one of its ways), and so
//! takes nothing from any other start of its blueprint, and any number of
//! them may be in force for one blueprint. Each is the same done twice:
//! asked again it is answered as it is, with the ways asked for that are
//! new added, so a blueprint that keeps nothing asks for it each time it
//! is done. It is started as whoever applied the instance and in the
//! instance's project, holding what they hold, as a comparison started
//! from outside is held. Nothing here knows what is compared.

use blueprint::inside::Row;
use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, ActionResult, Registry};
use kernel::{BoxFut, Error};
use serde_json::{Value, json};

use crate::Comparisons;
use crate::compared::{Compare, covers};

/// What each way is written with.
const WAYS: &[Row] = &[
    ("name", "string", "what the way is called: a word, such as `otter`, which a start names as its `way`"),
    (
        "share",
        "int",
        "its part of the things where they are shared out; a start that names its way is given it whatever the shares (default 1)",
    ),
    ("blueprint", "string", "a blueprint started in place of the one compared"),
    ("version", "string", "the version of what it starts that it is held to"),
    ("with", "object", "what it hands, over what the start hands"),
    ("prompts", "object", "for a file a step links, by its name in the library, the file it reads instead"),
];

/// What each count is written with.
const COUNTS: &[Row] = &[
    ("name", "string", "what the count is called: a lowercase word, such as `judged`"),
    ("topic", "string", "the event's topic, whole"),
    (
        "thing",
        "string",
        "the field of the event that says which thing it is about; a dotted path reads within it",
    ),
    (
        "by",
        "list",
        "the fields whose values say what it came to, each read as `thing` is; the last word of the topic where none",
    ),
    (
        "accepts",
        "list",
        "what it came to that says the thing was accepted (the last of several fields where several say); a thing is counted as accepted once",
    ),
];

/// Adds `compare.start`, `compare.end`, `compare.went` and
/// `compare.share` to what a blueprint can do.
pub(crate) fn register(registry: &mut Registry) {
    let start = ActionSpec::new(
        "compare.start",
        "Start a comparison whose ways are named by the start, or answer the one there is of that name with \
         the ways asked for added: from now, a start of `of` here whose params say `compared` and `way` goes \
         that way and is measured with it.",
    )
    .example(STARTED)
    .required("name", T::String, "what the comparison is called: a word, such as `models`")
    .required("of", T::String, "the blueprint whose starts name it")
    .required(
        "ways",
        T::List,
        "its ways, each a mapping (or a word, for a way that is the work as it is written)",
    )
    .within("ways", WAYS)
    .arg("per", T::String, "the param that says which thing a start is for; else its place in the tree")
    .arg("also", T::List, "blueprints started for the same things that are measured with each thing's way")
    .arg(
        "also_per",
        T::String,
        "the param that says which thing a start of one of `also` is for, where it is another than `per`",
    )
    .arg("counts", T::List, "the events about its things that are counted")
    .within("counts", COUNTS)
    .capability("blueprints:run")
    .output("name", "what the comparison is called")
    .output("ways", "the names of its ways, as it now has them")
    .output("ended", "true where it had ended, and was answered as it is")
    .idempotent();
    registry.add_fn(start, |cx, args| -> BoxFut<'static, ActionResult> {
        Box::pin(async move {
            let compared = Comparisons::of(cx.engine())?;
            let ways: Vec<Value> = args
                .get("ways")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|way| match way {
                    Value::String(name) => json!({ "name": name }),
                    other => other.clone(),
                })
                .collect();
            let asked = json!({
                "name": args.get("name"), "project": cx.project(), "of": args.get("of"),
                "per": args.get("per"), "also": args.get("also").cloned().unwrap_or(json!([])),
                "also_per": args.get("also_per"),
                "ways": ways, "counts": args.get("counts").cloned().unwrap_or(json!([])), "asked": true,
            });
            let asked: Compare = serde_json::from_value(asked).map_err(|wrong| {
                ActionError::new(format!("that is not how a comparison and its ways are written: {wrong}"))
            })?;
            let one = compared.compare(asked, cx.principal()).await?;
            let ways: Vec<&str> = one.ways.iter().map(|way| way.name.as_str()).collect();
            Ok(json!({ "name": one.name, "ways": ways, "ended": one.ended.is_some() }))
        })
    });

    let end = ActionSpec::new(
        "compare.end",
        "End a comparison whose ways are named by the start, in this project: from then a start that names it \
         goes as it is written.",
    )
    .example(ENDED)
    .required("name", T::String, "what the comparison is called")
    .capability("blueprints:run")
    .output("name", "what the comparison is called")
    .output("ended", "when it ended")
    .idempotent();
    registry.add_fn(end, |cx, args| -> BoxFut<'static, ActionResult> {
        Box::pin(async move {
            let compared = Comparisons::of(cx.engine())?;
            let name = args.get("name").and_then(Value::as_str).unwrap_or_default();
            let one = compared.comparison(name).await?;
            let here = one.project.as_deref() == cx.project();
            if !one.asked || !here {
                return Err(Error::invalid(format!(
                    "`{name}` is not a comparison whose ways are named by the start in this project, and a \
                     blueprint ends only one of those; `compare end {name}` ends any"
                ))
                .into());
            }
            if one.ended.is_none() && !covers(&cx.principal().scopes, &one.held) {
                return Err(Error::denied(format!(
                    "{} started `{name}` holding more than whoever applied this does, so it is not this \
                     blueprint's to end",
                    one.by
                ))
                .into());
            }
            let one = match one.ended {
                Some(_) => one,
                None => compared.end_comparison(name, None).await?,
            };
            Ok(json!({ "name": one.name, "ended": one.ended }))
        })
    });

    let went = ActionSpec::new(
        "compare.went",
        "Write a thing down as one of a way's, with nothing started for it, in a comparison whose ways are named \
         by the start in this project: what is said of the thing from then is counted under that way, as the \
         work it goes on with.",
    )
    .example(WENT)
    .required("name", T::String, "what the comparison is called")
    .required("thing", T::String, "the thing, as the events it counts say which thing they are about")
    .required("way", T::String, "the way it is written down as")
    .capability("blueprints:run")
    .output("name", "what the comparison is called")
    .output("written", "true where it was written down now, false where it was already")
    .idempotent();
    registry.add_fn(went, |cx, args| -> BoxFut<'static, ActionResult> {
        Box::pin(async move {
            let compared = Comparisons::of(cx.engine())?;
            let said = |key: &str| args.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
            let (name, thing, way) = (said("name"), said("thing"), said("way"));
            let written = compared.write_down(&name, cx.project(), &thing, &way, cx.principal()).await?;
            Ok(json!({ "name": name, "written": written }))
        })
    });

    let share = ActionSpec::new(
        "compare.share",
        "Say again the shares of a comparison in this project that shares its things out, with why: from then \
         each way is given its part of what comes next by them.",
    )
    .example(SHARED)
    .required("name", T::String, "what the comparison is called")
    .required(
        "shares",
        T::Object,
        "for each way to change, by its name, its share: a whole number, 1 or more; the ways not named keep theirs",
    )
    .arg("why", T::String, "why, in a sentence, which is said with the change")
    .capability("blueprints:run")
    .output("name", "what the comparison is called")
    .output("shares", "each way's share, as it now is")
    .idempotent();
    registry.add_fn(share, |cx, args| -> BoxFut<'static, ActionResult> {
        Box::pin(async move {
            let compared = Comparisons::of(cx.engine())?;
            let name = args.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
            let one = compared.comparison(&name).await?;
            if one.asked || one.project.as_deref() != cx.project() {
                return Err(Error::invalid(format!(
                    "`{name}` is not a comparison in this project that shares its things out, and a blueprint \
                     changes the shares only of one of those; `compare share {name}` changes any"
                ))
                .into());
            }
            let mut shares = std::collections::BTreeMap::new();
            for (way, share) in args.get("shares").and_then(Value::as_object).into_iter().flatten() {
                let share = share.as_u64().and_then(|share| u32::try_from(share).ok()).ok_or_else(|| {
                    ActionError::new(format!("the share of `{way}` is a whole number, 1 or more; it is {share}"))
                })?;
                shares.insert(way.clone(), share);
            }
            let why = args.get("why").and_then(Value::as_str).map(str::to_string);
            let one = compared.share(&name, shares, why, cx.principal()).await?;
            let shares: serde_json::Map<String, Value> =
                one.ways.iter().map(|way| (way.name.clone(), json!(way.share))).collect();
            Ok(json!({ "name": one.name, "shares": shares }))
        })
    });
}

// What the actions here are shown by in the Reference and the editor: each a whole
// blueprint that asks it, held to checking with no warning (the API suite's `examples.rs`).
const STARTED: &str = "\
blueprint: two-ways
permissions: [blueprints:run]

steps:
  compared:
    do: compare.start
    with: { name: tones, of: std/run, ways: [loud, quiet] }
";

const ENDED: &str = "\
blueprint: done-comparing
permissions: [blueprints:run]

steps:
  ended:
    do: compare.end
    with: { name: tones }
";

const SHARED: &str = "\
blueprint: given-more
permissions: [blueprints:run]

steps:
  more:
    do: compare.share
    with: { name: tones, shares: { quiet: 1, loud: 3 }, why: Quiet was asked for less often. }
";

const WENT: &str = "\
blueprint: kept-the-loud-one
permissions: [blueprints:run]
params: { item: '' }

steps:
  went:
    do: compare.went
    with: { name: tones, thing: \"${params.item}\", way: loud }
";
