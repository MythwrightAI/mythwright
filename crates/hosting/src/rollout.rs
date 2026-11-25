//! What a rollout works out: what its next version is called, and what
//! watching that version came to.
//!
//! `std/slot` hosts a service behind an address that does not change, a
//! new version beside the old and then in its place. It says what is
//! started, shifted and stopped, and when. Two things it would otherwise
//! have to count for itself are functions here: which generation the next
//! version is and what it is called for that ([`next`]), and through how
//! many stages it was watched with how many of them showing nothing
//! ([`watched`]). Neither starts or stops anything.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use kernel::BoxFut;
use serde_json::{Value, json};

/// The version after generation `generation` of what is called `name`:
/// which generation it is, and what its service is called for it.
pub fn next(name: &str, generation: i64) -> Result<(i64, String), String> {
    if name.is_empty() {
        return Err("`name` is what is being rolled out, and it has none".to_string());
    }
    if generation < 0 {
        return Err(format!(
            "`generation` is how many versions of `{name}` there have been, and is not less than none; it is \
             {generation}"
        ));
    }
    let next = generation + 1;
    Ok((next, format!("{name}-g{next}")))
}

blueprint::came! {
    /// What watching a new version came to.
    pub enum Came {
        /// It was watched through no stage: put in place at once.
        None => "none",
        /// Connections reached it in at least one stage it was watched through.
        Watched => "watched",
        /// It was watched, and no connection reached it in any stage: that
        /// it was not rolled back says nothing of whether it works.
        Unwatched => "unwatched",
    }
}

/// What the stages a version was watched through come to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Watched {
    pub came: Came,
    /// How many stages it was watched through.
    pub stages: usize,
    /// In how many of them no connection reached it.
    pub unwatched: usize,
}

/// What watching came to, from what each stage gave: a mapping that says
/// how many `connections` the version was sent while it was watched.
pub fn watched(stages: &Value) -> Result<Watched, String> {
    let none = Vec::new();
    let stages = match stages {
        Value::Null => &none,
        Value::Array(stages) => stages,
        other => {
            return Err(format!(
                "`shifts` is a list of what each shift gave, or nothing where there were none; it is {other}"
            ));
        }
    };
    let mut unwatched = 0;
    for (place, stage) in stages.iter().enumerate() {
        let Some(connections) = stage.get("connections").and_then(Value::as_u64) else {
            return Err(format!(
                "shift {} of `shifts` does not say how many `connections` reached the version in it: {stage}",
                place + 1
            ));
        };
        unwatched += usize::from(connections == 0);
    }
    let came = match (stages.len(), unwatched) {
        (0, _) => Came::None,
        (all, none) if all == none => Came::Unwatched,
        _ => Came::Watched,
    };
    Ok(Watched { came, stages: stages.len(), unwatched })
}

/// Adds `rollout.next` and `rollout.watched`, which are [`next`] and
/// [`watched`] for a blueprint.
pub(crate) fn register(registry: &mut Registry) {
    let spec = ActionSpec::new(
        "rollout.next",
        "Say which generation the next version of something is, and what it is called.",
    )
    .required("name", T::String, "what is being rolled out")
    .arg("generation", T::Int, "the generation there is now; 0, and the default, where there is none yet")
    .output("generation", "the generation the next version is: one more")
    .output("name", "what its service is called: `<name>-g<generation>`")
    .idempotent();
    registry.add_fn(spec, |_cx, args| -> BoxFut<'static, conductor::ActionResult> {
        Box::pin(async move {
            let name = args.get("name").and_then(Value::as_str).unwrap_or_default();
            let generation = match args.get("generation") {
                None | Some(Value::Null) => 0,
                Some(said) => said.as_i64().ok_or_else(|| {
                    ActionError::new(format!("`generation` is a whole number, and was given {said}"))
                })?,
            };
            let (generation, name) = next(name, generation).map_err(ActionError::new)?;
            Ok(json!({ "generation": generation, "name": name }))
        })
    });

    let spec = ActionSpec::new(
        "rollout.watched",
        "Say what watching a new version through its stages came to: how many, and in how many nothing reached it.",
    )
    .arg("shifts", T::List, "what each shift of its share gave, each saying its `connections`; nothing where there were none")
    .output("came", &format!("{}: no stage; connections reached it in some stage; or in none", Came::words()))
    .output("shifts", "how many shifts it was watched through")
    .output("unwatched", "in how many of them no connection reached it")
    .idempotent();
    registry.add_fn(spec, |_cx, args| -> BoxFut<'static, conductor::ActionResult> {
        Box::pin(async move {
            let came = watched(args.get("shifts").unwrap_or(&Value::Null)).map_err(ActionError::new)?;
            Ok(json!({ "came": came.came.word(), "shifts": came.stages, "unwatched": came.unwatched }))
        })
    });
}
