//! The rows a table starts with, asked as an action (`rows.start`): which
//! are still to be added is a function of plain values and the kernel's
//! ([`kernel::first`]), offered here under the name it always had.

use blueprint::expr::slug;
use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Map, Value, json};

use crate::Studio;
use crate::actions::{add, project_of, text, through};
use kernel::first::called;
pub use kernel::first::owed;

pub(crate) fn register(registry: &mut Registry, studio: &Studio) {
    add(
        registry,
        studio,
        ActionSpec::new(
            "rows.start",
            "Give a table the rows it starts with: each is added unless the table has a row called what it is \
             called, so that done twice it is the same, and a row somebody changed is left as they have it.",
        )
.example(kernel::asked::ROWS_START)
        .making("the first rows of the table", "collection")
        .required("collection", T::String, "the table's name")
        .required("rows", T::List, "the rows, in order: each what a row holds, by cell")
        .arg("project", T::String, "which project (default: this instance's)")
        .arg("by", T::String, "the cell a row is known by, which each of `rows` says something in (default `title`)")
        .capability("records:write")
        .output("added", "the rows added just now, by id, in the order they were added")
        .output("count", "how many those are")
        .output("there", "how many of `rows` the table had already")
        .idempotent(),
        |studio, cx, args| async move {
            let project = project_of(&cx, &args)?;
            let name = slug(&text(&args, "collection").unwrap_or_default());
            let by = text(&args, "by").unwrap_or_else(|| "title".to_string());
            let rows: Vec<Map<String, Value>> = args
                .get("rows")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|row| row.as_object().cloned())
                .collect::<Option<_>>()
                .ok_or_else(|| ActionError::new("each of `rows` is what a row holds, by cell: a mapping"))?;
            let records = studio.records.list(&project, &name, cx.principal()).await?;
            let there: Vec<Value> =
                records.iter().filter_map(|record| called(&record.data, &by).cloned()).collect();
            let owed = owed(&there, &rows, &by).map_err(ActionError::new)?;
            let through = through(&cx);
            let mut added = Vec::new();
            for place in &owed {
                let fields = rows[*place].clone();
                added.push(studio.records.add(&project, &name, fields, cx.principal(), Some(&through)).await?.id);
            }
            Ok(json!({ "added": added, "count": added.len(), "there": rows.len() - owed.len() }))
        },
    );
}
