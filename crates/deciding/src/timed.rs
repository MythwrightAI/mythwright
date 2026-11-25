//! How long a card took, read from its history.
//!
//! A card's history keeps, as they happen, its being added to a column,
//! each move with where it was and where it came to, its being taken away
//! and given back, and what an agent said of working on it. That is
//! everything there is to say how long it took, so nothing more is kept
//! for it: what the history comes to is this function of its lines, with
//! a table of cases, and a table beside the board (`std/timed`) keeps a
//! row of it for each card that reaches the board's last column.
//!
//! What a history comes to: when the card was added and when it last came
//! to the last column, the time between them, how long it was in each
//! column on the way (a column it came back to counted each time), how
//! many times it was moved to a column before the one it was in (sent
//! back), how many times it came to the last column, and who worked it
//! (the agents that said they worked on it, and whoever moved it into the
//! column cards are worked in by hand). Time while it was in the Trash is
//! in no column. A history that does not say when the card was added
//! (one from before its board kept one, or a board that keeps none of its
//! cards' moves) begins at the first move it says, and says so (`whole`).
//!
//! It counts and never scores: nothing here says a card, a column or
//! whoever worked it did well.

use blueprint::{ActionSpec, ValueType as T};
use conductor::Registry;
use serde_json::{Map, Value, json};

use crate::add;

/// A line of a card's history, as far as how long it took goes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Line {
    pub topic: String,
    pub ts: i64,
    /// Who did it, as the journal has them (`agent:shop-worker-1`), and by
    /// name.
    pub actor: String,
    pub who: String,
    pub data: Value,
}

/// What a card's history comes to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Timed {
    /// `done` where it is in the last column, `on` where it is anywhere
    /// else, `unread` where its history says nothing of where it was.
    pub came: &'static str,
    /// Whether it says when the card was added; else it begins at the
    /// first move it says.
    pub whole: bool,
    /// When it was added (or first said to be moved), and when it last came
    /// to the last column, in milliseconds of the clock.
    pub added: i64,
    pub done: i64,
    /// How long it was in each column, the board's in their order and then
    /// any other it was in, in milliseconds.
    pub spent: Vec<(String, i64)>,
    /// How many times it was sent back, and came to the last column.
    pub back: i64,
    pub reached: i64,
    /// Who worked it, in the order they first did.
    pub worked: Vec<String>,
}

/// Every word [`timed`] can answer.
pub const TIMED_WORDS: &[&str] = &["done", "on", "unread"];

fn said(data: &Value, key: &str) -> String {
    data.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

/// What `lines`, a card's history oldest first, come to on a board whose
/// columns are `columns`, cards being worked in `working`.
pub fn timed(lines: &[Line], columns: &[String], working: &str) -> Result<Timed, String> {
    let Some(last) = columns.last() else {
        return Err("how long a card took is read against its board's columns, and none is handed".into());
    };
    let at = |column: &str| columns.iter().position(|one| one == column);
    let mut timed = Timed::default();
    let mut spent: Vec<(String, i64)> = columns.iter().map(|column| (column.clone(), 0)).collect();
    // Where the card is, and since when; nowhere while it is in the Trash.
    let mut here: Option<(String, i64)> = None;
    let mut begun = false;
    let add = |spent: &mut Vec<(String, i64)>, column: &str, ms: i64| match spent
        .iter_mut()
        .find(|one| one.0 == column)
    {
        Some(one) => one.1 += ms.max(0),
        None => spent.push((column.to_string(), ms.max(0))),
    };
    let worked = |who: String, timed: &mut Timed| {
        if !who.is_empty() && !timed.worked.contains(&who) {
            timed.worked.push(who);
        }
    };
    for line in lines {
        let name = if line.who.is_empty() { line.actor.clone() } else { line.who.clone() };
        match line.topic.as_str() {
            "board.item.created" => {
                let status = said(&line.data, "status");
                let restored = line.data.get("restored").and_then(Value::as_bool) == Some(true);
                if !begun && !restored {
                    (timed.added, timed.whole, begun) = (line.ts, true, true);
                }
                // Added straight to the last column, it came there as it was added.
                if &status == last && here.is_none() {
                    (timed.reached, timed.done) = (timed.reached + 1, line.ts);
                }
                if here.is_none() && !status.is_empty() {
                    here = Some((status, line.ts));
                }
            }
            "board.item.moved" => {
                let (from, to) = (said(&line.data, "from"), said(&line.data, "to"));
                if !begun {
                    (timed.added, timed.whole, begun) = (line.ts, false, true);
                }
                if let Some((column, since)) = here.take() {
                    add(&mut spent, &column, line.ts - since);
                }
                if let (Some(was), Some(now)) = (at(&from), at(&to))
                    && now < was
                {
                    timed.back += 1;
                }
                if &to == last {
                    (timed.reached, timed.done) = (timed.reached + 1, line.ts);
                }
                // A person who took it to be worked worked it; a blueprint's
                // move is said as whoever applied it, who did not.
                let by_hand =
                    line.data.get("through").is_none_or(Value::is_null) && !line.actor.starts_with("agent:");
                if to == working && by_hand {
                    worked(name, &mut timed);
                }
                here = Some((to, line.ts));
            }
            "board.item.removed" => {
                if let Some((column, since)) = here.take() {
                    add(&mut spent, &column, line.ts - since);
                }
            }
            "card.worked" => {
                let agent = said(&line.data, "agent");
                worked(if agent.is_empty() { name } else { agent }, &mut timed);
            }
            _ => {}
        }
    }
    if !begun {
        return Ok(Timed { came: "unread", ..Default::default() });
    }
    // The time it has been in the last column since it last came there is
    // no part of how long it took.
    timed.came = match &here {
        Some((column, _)) if column == last && timed.reached > 0 => "done",
        _ => "on",
    };
    timed.spent = spent;
    Ok(timed)
}

/// What a column's time is kept as in a table: `in_` and the column's name,
/// lowercase, with anything but a letter, a digit or `_` as `_`.
pub fn field(column: &str) -> String {
    let named: String = column
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' { c } else { '_' })
        .collect();
    format!("in_{named}")
}

/// The fields of a table of how long cards took on a board whose columns
/// are `columns`, by name with what each is: one for each column but the
/// last, where a card that came there is done.
pub fn fields(columns: &[String]) -> Result<Map<String, Value>, String> {
    if columns.len() < 2 {
        return Err(format!(
            "how long a card took is kept for a board of two columns or more, from where cards begin to where they \
             are done; this one has {}",
            columns.len()
        ));
    }
    let mut all = Map::new();
    all.insert("card".into(), json!({ "type": "record", "of": "board", "required": true }));
    all.insert("title".into(), json!({ "type": "text" }));
    all.insert("added".into(), json!({ "type": "date" }));
    all.insert("done".into(), json!({ "type": "date" }));
    all.insert("total".into(), json!({ "type": "number" }));
    for column in &columns[..columns.len() - 1] {
        let name = field(column);
        if all.contains_key(&name) {
            return Err(format!(
                "the columns `{column}` and another are both kept as `{name}`; call one something else"
            ));
        }
        all.insert(name, json!({ "type": "number" }));
    }
    all.insert("back".into(), json!({ "type": "number" }));
    all.insert("reached".into(), json!({ "type": "number" }));
    all.insert("worked".into(), json!({ "type": "list" }));
    all.insert("whole".into(), json!({ "type": "flag" }));
    Ok(all)
}

/// Milliseconds as minutes, to a tenth.
fn minutes(ms: i64) -> f64 {
    (ms as f64 / 6_000.0).round() / 10.0
}

/// The day of `ms` of the clock, written year-month-day as a table's day
/// is, in the clock's own reckoning (UTC).
pub fn day(ms: i64) -> String {
    let days = ms.div_euclid(86_400_000);
    // Days since 1970-01-01 to a day of the calendar (Howard Hinnant's way).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The row a card's history comes to, of the columns its table keeps.
pub fn row(timed: &Timed, card: &str, title: &str, kept: &[String]) -> Map<String, Value> {
    let mut row = Map::new();
    row.insert("card".into(), json!(card));
    row.insert("title".into(), json!(title));
    row.insert("added".into(), json!(day(timed.added)));
    row.insert("done".into(), json!(day(timed.done)));
    row.insert("total".into(), json!(minutes(timed.done - timed.added)));
    for column in kept.iter().take(kept.len().saturating_sub(1)) {
        let ms = timed.spent.iter().find(|one| &one.0 == column).map_or(0, |one| one.1);
        row.insert(field(column), json!(minutes(ms)));
    }
    row.insert("back".into(), json!(timed.back));
    row.insert("reached".into(), json!(timed.reached));
    row.insert("worked".into(), json!(timed.worked));
    row.insert("whole".into(), json!(timed.whole));
    row
}

fn words(args: &kernel::Fields, key: &str) -> Vec<String> {
    let each = args.get(key).and_then(Value::as_array).cloned().unwrap_or_default();
    each.iter().filter_map(|one| one.as_str().map(str::to_string)).collect()
}

/// Adds `timed.fields` to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    add(
        registry,
        ActionSpec::new(
            "timed.fields",
            "Say the fields of a table of how long cards took on a board with these columns: the card, when it was added and done, the time in all and in each column, how often it was sent back and came to the last column, and who worked it.",
        )
        .deciding()
        .required("columns", T::List, "the board's columns, left to right")
        .output("fields", "the fields, by name, as `collection.ensure` takes them; a column's time is `in_` and its name")
        .output("last", "the last of the columns, where a card is done")
        .idempotent(),
        |_, args| async move {
            let columns = words(&args, "columns");
            let fields = fields(&columns).map_err(conductor::ActionError::new)?;
            Ok(json!({ "fields": fields, "last": columns.last() }))
        },
    );
}

/// Adds `timed.read` to what a blueprint can do: it reads a card's history
/// from `studio`, as whoever asks may read the card, and says what that
/// comes to.
pub fn register_reading(registry: &mut Registry, studio: &studio::Studio) {
    let studio = studio.clone();
    registry.add_fn(
        ActionSpec::new(
            "timed.read",
            "Read a card's history and say how long it took: from being added to coming to the board's last column, in each column, how often it was sent back, and who worked it, as the row a table of those keeps.",
        )
        .deciding()
        .required("id", T::String, "the card")
        .required("columns", T::List, "the columns its table keeps a time for, the board's left to right")
        .arg("working", T::String, "the column cards are worked in, where whoever moves a card by hand worked it; `doing` if not said")
        .capability("board:read")
        .output("came", "`done` where it is in the last column, `on` where it is not, `unread` where its history says nothing of where it was")
        .output("row", "the row it comes to, by field, as `timed.fields` says them")
        .output("number", "the card's own number, which its row is kept under")
        .output("total", "how long it took, in minutes")
        .output("back", "how many times it was sent back")
        .output("reached", "how many times it came to the last column")
        .output("worked", "who worked it")
        .idempotent(),
        move |cx, args| -> kernel::BoxFut<'static, conductor::ActionResult> {
            let studio = studio.clone();
            Box::pin(async move {
                let id = args.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
                // Read as anything of the card is, by whoever asks.
                let record = studio.records.get(&id, cx.principal()).await?;
                let lines: Vec<Line> = studio
                    .collections
                    .history(&id)
                    .await?
                    .into_iter()
                    .map(|line| Line {
                        topic: line.topic,
                        ts: line.ts,
                        actor: line.actor.unwrap_or_default(),
                        who: line.who,
                        data: line.data,
                    })
                    .collect();
                let columns = words(&args, "columns");
                let working = args.get("working").and_then(Value::as_str).unwrap_or("doing");
                let timed = timed(&lines, &columns, working).map_err(conductor::ActionError::new)?;
                let title = record.data.get("title").and_then(Value::as_str).unwrap_or_default();
                Ok(json!({
                    "came": timed.came, "row": row(&timed, &id, title, &columns), "number": record.number,
                    "total": minutes(timed.done - timed.added), "back": timed.back, "reached": timed.reached,
                    "worked": timed.worked,
                }))
            })
        },
    );
}
