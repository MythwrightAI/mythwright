//! `profile.set`: the variables of the profile, the copy of the factory a
//! blueprint runs in, as a blueprint sets them.
//!
//! What the variables are and what an expression reads of them is the
//! runner's (`conductor::profile`), and how they are kept is
//! `environments::factory`: they are the scope every expression is worked
//! out in. This is the one way a
//! blueprint changes them, which is the owner's to use: what it changes
//! is what every blueprint in the factory reads.

use blueprint::{ActionSpec, ValueType};
use conductor::{ActionResult, Registry};
use kernel::BoxFut;
use serde_json::{Value, json};

/// Adds `profile.set` to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    let spec = ActionSpec::new(
        "profile.set",
        "Say what is true of this profile, the copy of the factory it runs in, for every blueprint here to read as \
         `profile.vars.<name>`. The whole set is replaced. Not for secrets: the values are shown and kept as they are.",
    )
.example(SET_ASKED)
    .required(
        "vars",
        ValueType::Object,
        "the variables, each a name and a plain value (text, a number, true or false); one left out is no longer set, and `{}` clears them all",
    )
    // What every blueprint in the factory reads, and so what each of them
    // does: the owner's to say, and nobody else's.
    .capability("admin")
    .output("set", "the names that were not set before")
    .output("changed", "the names that have another value than they had")
    .output("removed", "the names that are no longer set")
    .output("names", "every name that is set now")
    .idempotent();
    registry.add_fn(spec, |cx, args| -> BoxFut<'static, ActionResult> {
        Box::pin(async move {
            let vars = args.get("vars").and_then(Value::as_object).cloned().unwrap_or_default();
            let names: Vec<String> = vars.keys().cloned().collect();
            let changes = cx.engine().set_variables(vars, cx.source(), &cx.principal().name).await?;
            Ok(json!({
                "set": changes.set, "changed": changes.changed, "removed": changes.removed, "names": names,
            }))
        })
    });
}

// What the actions here are shown by in the Reference and the editor: each a whole
// blueprint that asks it, held to checking with no warning (the API suite's `examples.rs`).
const SET_ASKED: &str = "\
blueprint: where-the-api-is
permissions: [admin]

steps:
  said:
    do: profile.set
    with: { vars: { api: \"http://localhost:8080\", workers: 4 } }
";
