//! Whether several items handed on together have all come through.
//!
//! Some work goes on only once every one of several items has been
//! passed, and stops where one of them is sent back: a set of branches
//! gathered into one (`place.gather`) is let go onto the project's own
//! branch only when every item whose work it holds was passed by whoever
//! looks at them, and not where one was failed. Where each item is, is a
//! row of a table (what `rows.read` gives), and what is decided is the
//! column each row's field says: the one items pass to (`passes`), the
//! ones they wait in to be looked at (`waits`), or anywhere else, which
//! is somewhere they were sent back to.
//!
//! What a set comes to: `back` where any item has gone back, or is gone
//! from the table (it can never pass); `waiting` where none has and some
//! still wait; `passed` where every one is where items pass to. Nothing
//! here knows what an item is, what the table is, or who moves them: a
//! blueprint asks this again each time one of them is moved, and does
//! what it is told.

use std::collections::BTreeMap;

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Value, json};

use crate::add;

blueprint::came! {
    /// What several items handed on together come to.
    pub enum Came {
        /// Every one is where items pass to.
        Passed => "passed",
        /// One at least went somewhere else, or is gone.
        Back => "back",
        /// None went back, and some still wait to be looked at.
        Waiting => "waiting",
    }
}

/// Where each of the items stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Passed {
    pub came: Came,
    /// Those where items pass to, in the order they were handed.
    pub passed: Vec<String>,
    /// Those that went somewhere else, each with where it is.
    pub back: Vec<(String, String)>,
    /// Those that still wait to be looked at.
    pub waiting: Vec<String>,
    /// Those the table has no row of any more.
    pub gone: Vec<String>,
}

/// What `items` come to, each being where `at` says (by its id), where
/// items pass to `passes` and wait in `waits`. An item said twice is
/// counted once.
pub fn passed(
    items: &[String],
    at: &BTreeMap<String, String>,
    passes: &str,
    waits: &[String],
) -> Result<Passed, String> {
    if items.is_empty() {
        return Err("`items` are the items handed on together, and none is handed".into());
    }
    if passes.is_empty() {
        return Err(
            "`passes` is where an item comes to once it is passed, such as `done`, and is said".into()
        );
    }
    if waits.iter().any(|one| one == passes) {
        return Err(format!(
            "`{passes}` is said both to be where items pass to and where they wait: an item there would be both"
        ));
    }
    let mut seen: Vec<&String> = Vec::new();
    let mut said = Passed { came: Came::Passed, passed: vec![], back: vec![], waiting: vec![], gone: vec![] };
    for item in items {
        if seen.contains(&item) {
            continue;
        }
        seen.push(item);
        match at.get(item) {
            None => said.gone.push(item.clone()),
            Some(here) if here == passes => said.passed.push(item.clone()),
            Some(here) if waits.contains(here) => said.waiting.push(item.clone()),
            Some(here) => said.back.push((item.clone(), here.clone())),
        }
    }
    said.came = if !said.back.is_empty() || !said.gone.is_empty() {
        Came::Back
    } else if !said.waiting.is_empty() {
        Came::Waiting
    } else {
        Came::Passed
    };
    Ok(said)
}

impl Passed {
    /// What it comes to, in a sentence.
    pub fn said(&self) -> String {
        let all = self.passed.len() + self.back.len() + self.waiting.len() + self.gone.len();
        let named =
            |names: &[String]| names.iter().map(|one| format!("`{one}`")).collect::<Vec<_>>().join(", ");
        match self.came {
            Came::Passed if all == 1 => "It was passed.".to_string(),
            Came::Passed => format!("All {all} were passed."),
            Came::Waiting => format!(
                "{} of {all} passed so far; {} still {}.",
                self.passed.len(),
                named(&self.waiting),
                if self.waiting.len() == 1 { "waits" } else { "wait" }
            ),
            Came::Back => {
                let mut parts: Vec<String> =
                    self.back.iter().map(|(item, here)| format!("`{item}` went back to {here}")).collect();
                if !self.gone.is_empty() {
                    parts.push(format!(
                        "{} {} gone",
                        named(&self.gone),
                        if self.gone.len() == 1 { "is" } else { "are" }
                    ));
                }
                format!("{}.", parts.join("; "))
            }
        }
    }
}

fn words(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|one| one.as_str().map(str::to_string))
        .collect()
}

/// Where each row is, by its id, as its `field` says.
fn placed(rows: Option<&Value>, field: &str) -> BTreeMap<String, String> {
    let rows = rows.and_then(Value::as_array).into_iter().flatten();
    rows.filter_map(|row| {
        let id = row.get("id").and_then(Value::as_str)?;
        let here = row.get(field).and_then(Value::as_str).unwrap_or_default();
        Some((id.to_string(), here.to_string()))
    })
    .collect()
}

/// Adds `items.passed` to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    add(
        registry,
        ActionSpec::new(
            "items.passed",
            "Say whether every one of several items handed on together was passed, one went back, or some still wait to be looked at.",
        )
        .example(ASKED)
        .deciding()
        .required("items", T::List, "the items handed on together, by id")
        .required("rows", T::List, "the rows of the table where they are, each with its `id`, as `rows.read` gives them")
        .required("passes", T::String, "where an item comes to once it is passed, such as `done`")
        .required("waits", T::List, "where an item waits to be looked at; anywhere else, it went back")
        .arg("field", T::String, "the field of a row that says where it is (default `status`)")
        .output("came", "`passed` where every one was; `back` where one went somewhere else, or is gone from the table; `waiting` where none went back and some still wait")
        .output("passed", "those that were passed, by id")
        .output("back", "those that went back, each with its `id` and where it is (`at`)")
        .output("waiting", "those that still wait, by id")
        .output("gone", "those the table has no row of any more, by id")
        .output("said", "what it comes to, in a sentence")
        .idempotent(),
        |_, args| async move {
            let field = args.get("field").and_then(Value::as_str).filter(|one| !one.is_empty()).unwrap_or("status");
            let passes = args.get("passes").and_then(Value::as_str).unwrap_or_default();
            let at = placed(args.get("rows"), field);
            let decided = passed(&words(args.get("items")), &at, passes, &words(args.get("waits")))
                .map_err(ActionError::new)?;
            let back: Vec<Value> =
                decided.back.iter().map(|(id, here)| json!({ "id": id, "at": here })).collect();
            Ok(json!({
                "came": decided.came.word(), "passed": decided.passed, "back": back,
                "waiting": decided.waiting, "gone": decided.gone, "said": decided.said(),
            }))
        },
    );
}

/// What `items.passed` is shown by in the Reference and the editor.
const ASKED: &str = "\
blueprint: all-passed
permissions: [records:read]

params:
  items: { type: list, description: The items handed on together. }

steps:
  where:
    do: rows.read
    with: { collection: board }

  decided:
    needs: [where]
    do: items.passed
    with:
      items: \"${params.items}\"
      rows: \"${steps.where.out.rows}\"
      passes: done
      waits: [staging]

  said:
    needs: [decided]
    do: log
    with: { message: \"${steps.decided.out.said}\" }
";
