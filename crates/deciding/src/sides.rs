//! Something kept in two places brought together, asked as actions:
//! `sides.cross` and `sides.owed`.
//!
//! What each works out is a function of plain values and is the kernel's
//! ([`kernel::sides`]), offered here under the names it always had; this
//! is how a blueprint asks it.
//!
//! It was the studio's (`studio::sides`), and came here with nothing of a
//! table in it: a fleet's blueprints ask it as they ask the rest here.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use kernel::BoxFut;
use serde_json::{Map, Value, json};

pub use kernel::sides::*;

pub(crate) fn register(registry: &mut Registry) {
    let spec = ActionSpec::new(
        "sides.cross",
        "Say what is to cross between two sides of one thing that is kept in step, each field held against what \
         the two last agreed on: what goes out, what comes in, and what they agree on once that is done.",
    )
    .required("here", T::Object, "this side: each field by its name")
    .required("there", T::Object, "the other side: the same fields by the same names")
    .arg("last", T::Object, "what the two last agreed on; nothing, where they never have")
    .required(
        "prefer",
        T::String,
        "which side is taken where a field moved on both, or was never agreed and differs: `here` or `there`",
    )
    .arg("takes", T::String, "whose changes on the other side are taken: `everyone`, which it is unless said; `trusted`, only where the other side as it stands is said to be; or `nobody`, so that this side only goes out and what moves there alone is left as it is")
    .arg("trusted", T::Bool, "whether the other side as it stands was written by somebody trusted; needed where `takes` is `trusted`")
    .arg("held", T::String, "what was said of a change that is held: nothing; `waiting`; `take`, the other side stands; or `keep`, this one does")
    .output("came", "`same`, nothing is to cross; `out`; `in`; `both`, something goes each way; or `held`, a change on the other side is not taken and nothing crosses either way")
    .output("goes", "the fields that go out, each as this side says it")
    .output("comes", "the fields that come in, each as the other side says it")
    .output("sends", "the names of the fields that go out")
    .output("takes", "the names of the fields that come in")
    .output("agreed", "what the two agree on once that is done, for every field of either; what they agreed on before, where nothing crossed")
    .output("holds", "what `held` is once that is done: `waiting`, or nothing")
    .output("newly", "whether a change came to be held just now, so that it is said once")
    .idempotent();
    registry.add_fn(spec, |_cx, args| -> BoxFut<'static, conductor::ActionResult> {
        Box::pin(async move {
            let crossing = told(&args).map_err(ActionError::new)?;
            let names = |fields: &Map<String, Value>| fields.keys().cloned().collect::<Vec<_>>();
            Ok(json!({
                "came": crossing.came.word(),
                "sends": names(&crossing.out),
                "takes": names(&crossing.take),
                "goes": crossing.out,
                "comes": crossing.take,
                "agreed": crossing.agreed,
                "holds": crossing.holds.then_some("waiting"),
                "newly": crossing.newly,
            }))
        })
    });
    let spec = ActionSpec::new(
        "sides.owed",
        "Say which of the things said on one side have not crossed to the other yet, held against the names of \
         those that have.",
    )
    .arg("things", T::List, "what was said, each a mapping that says its own name")
    .arg("by", T::String, "the field each says its name in; `id` unless said")
    .arg("had", T::List, "the names of those that have crossed")
    .arg("also", T::List, "more names that are not to cross: those that crossed the other way")
    .arg("except", T::String, "a field that leaves a thing out where it says `true` in it")
    .output("came", "`some` have not crossed; or `none`")
    .output(
        "those",
        "each that has not, in order, with `kept` beside what it says: the names of all that have crossed once \
         it has",
    )
    .output("ids", "their names, in order")
    .output("kept", "the names of all that have crossed once each of those has")
    .idempotent();
    registry.add_fn(spec, |_cx, args| -> BoxFut<'static, conductor::ActionResult> {
        Box::pin(async move {
            let owed = owed_told(&args).map_err(ActionError::new)?;
            Ok(json!({ "came": owed.word(), "those": owed.those, "ids": owed.names, "kept": owed.kept }))
        })
    });
}
