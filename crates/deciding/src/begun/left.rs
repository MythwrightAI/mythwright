//! Which items were left in hand when whatever had them was stopped.
//!
//! Whoever works the items of a column takes each out of it into the
//! column where items are in hand, and its row says whose it is (`taken`,
//! the column it was taken from) and that a round of it was begun
//! (`again`: `heard`, with which of its moves). A round that ends hands
//! the item on or writes why it did not; so a row that still says only
//! that a round was begun, with nothing at work on it, was left by a
//! round that was stopped. Whoever is first there for a column has such
//! items to take up again: [`left`] says which, from the rows as they
//! stand. It is handed the columns, and knows nothing of what is done in
//! any of them.

use blueprint::{ActionSpec, ValueType as T};
use conductor::Registry;
use serde_json::{Value, json};

use crate::add;

/// What [`left`] is asked.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Asked {
    /// The items, each a row: its `id`, the column it is in (`status`),
    /// the column it was taken from (`taken`) and what its row keeps of
    /// its rounds (`again`).
    pub items: Vec<Value>,
    /// The column an item is in while it is in hand.
    pub doing: String,
    /// The columns whose items are worked.
    pub worked: Vec<String>,
}

/// What was left: the items in hand that are given again, by their ids;
/// empty where there are none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Left {
    pub given: Vec<String>,
}

pub fn left(asked: &Asked) -> Left {
    let text = |row: &Value, cell: &str| row[cell].as_str().unwrap_or_default().to_string();
    let begun = |row: &Value| row["again"].as_array().and_then(|kept| kept.first()) == Some(&json!("heard"));
    let ids = |such: &dyn Fn(&Value) -> bool| -> Vec<String> {
        asked
            .items
            .iter()
            .filter(|row| such(row))
            .map(|row| text(row, "id"))
            .filter(|id| !id.is_empty())
            .collect()
    };
    let given = ids(&|row| {
        !asked.doing.is_empty()
            && text(row, "status") == asked.doing
            && asked.worked.contains(&text(row, "taken"))
            && begun(row)
    });
    Left { given }
}

/// Adds what is asked here to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    add(
        registry,
        ActionSpec::new(
            "work.left",
            "Say which items were left in hand when whatever had them was stopped: those a round was begun for \
             and neither handed on nor said anything of.",
        )
        .deciding()
        .arg("items", T::List, "the items, each a row as `rows.read` gives it: `id`, `status`, `taken`, `again`")
        .required("doing", T::String, "the column an item is in while it is in hand")
        .arg("worked", T::List, "the columns whose items are worked")
        .output("given", "the items left in hand, which are given again: their ids")
        .idempotent(),
        move |_, args| async move {
            let list = |key: &str| args.get(key).and_then(Value::as_array).cloned().unwrap_or_default();
            let words = |key: &str| list(key).iter().map(blueprint::expr::display).collect::<Vec<_>>();
            let asked = Asked {
                items: list("items"),
                doing: args.get("doing").and_then(Value::as_str).unwrap_or_default().to_string(),
                worked: words("worked"),
            };
            let came = left(&asked);
            Ok(json!({ "given": came.given }))
        },
    );
}
