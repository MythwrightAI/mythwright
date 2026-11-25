//! The rows a table starts with, added so that it is the same done twice.
//!
//! Something that makes a table gives it its first rows once, and adding
//! a record is not something done twice: a step stopped after one row was
//! added and before it had kept that it was is refused when the daemon is
//! back, and the table is left with the rows it had got to. `rows.start`
//! is handed all of them and adds those the table has not got, each found
//! again by what it is called. Done a second time it adds what was still
//! to come and nothing else.
//!
//! It adds, and never changes or takes away: a row somebody has changed
//! since is the row of that name, and holds what they gave it. Whether a
//! table is to be given its first rows at all is whoever asks (a table
//! somebody emptied is theirs): this is asked while a table is being
//! made, and not again.

use serde_json::{Map, Value};

/// A cell as text, as the language says one: words as they are, nothing
/// as nothing, and anything else as it is written.
fn display(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// What a row says it is called, in the cell rows are known by; nothing,
/// where it says nothing there.
pub fn called<'a>(row: &'a Map<String, Value>, by: &str) -> Option<&'a Value> {
    match row.get(by) {
        None | Some(Value::Null) => None,
        Some(Value::String(text)) if text.trim().is_empty() => None,
        Some(value) => Some(value),
    }
}

/// Which of the rows a table is to start with are still to be added,
/// given what each row it has is called (`there`): by their places in
/// `rows`, in order.
///
/// A row is there when the table has a row called what it is called.
/// Where several of `rows` are called the same, as many of them are
/// there as the table has rows of that name, the earlier ones first: so
/// two rows of one name are both added to a table that has neither, and
/// one of them to a table that has one. A row that says nothing of what
/// it is called could not be found again, and is refused.
pub fn owed(there: &[Value], rows: &[Map<String, Value>], by: &str) -> Result<Vec<usize>, String> {
    let mut there: Vec<String> = there.iter().map(display).collect();
    let mut owed = Vec::new();
    for (place, row) in rows.iter().enumerate() {
        let Some(name) = called(row, by) else {
            return Err(format!(
                "a row a table starts with is found again by what it says in `{by}`, and row {} says nothing there: \
                 give it one, or say with `by` which cell its rows are known by",
                place + 1
            ));
        };
        let name = display(name);
        match there.iter().position(|one| *one == name) {
            // That row of the table is this one, and no later row's.
            Some(found) => drop(there.swap_remove(found)),
            None => owed.push(place),
        }
    }
    Ok(owed)
}
