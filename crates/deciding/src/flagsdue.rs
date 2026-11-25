//! `flags.due`: which of a project's feature flags are past the day they
//! were to go, and whom each is told to (`std/flags`), since flags that are
//! never taken out are the cost of having them. What a flag's row comes to
//! is `kernel::flags::due`, a function of the rows and a day with its
//! table of cases; what is here reads today from the clock, and says whom:
//! the flag's owner, and whoever applied what keeps the flags where its
//! row names nobody.

use blueprint::{ActionSpec, ValueType as T};
use conductor::Registry;
use kernel::flags;
use serde_json::{Value, json};

use crate::add;

pub fn register(registry: &mut Registry) {
    add(
        registry,
        ActionSpec::new(
            "flags.due",
            "Say which of a project's feature flags are past the day they were to go, each with whom it is told to and in what words.",
        )
        .deciding()
        .arg("rows", T::List, "the rows of the project's `flags`, as `rows.read` gives them")
        .output(
            "due",
            "each flag past its day: `record` (its row), `name`, `until`, `to` (its owner, or `applier` where its row names nobody) and `says`, what they are told",
        )
        .idempotent(),
        |cx, args| async move {
            let rows: Vec<Value> = args.get("rows").and_then(Value::as_array).cloned().unwrap_or_default();
            let today = flags::day_of(cx.clock().now_ms());
            let due: Vec<Value> = flags::due(&rows, &today)
                .into_iter()
                .map(|one| {
                    let to = if one.owner.is_empty() { "applier".to_string() } else { one.owner };
                    json!({ "record": one.record, "name": one.name, "until": one.until, "to": to, "says": one.says })
                })
                .collect();
            Ok(json!({ "due": due }))
        },
    );
}
