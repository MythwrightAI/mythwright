//! A table read as what its rows come to (`rows.read`, which the studio
//! offers as `studio::rows`): a function of the table's cells, its rows
//! and what is asked, reading and writing nothing.
//!
//! A row of a table a person edits says as much as its writer cared to: a
//! cell never filled is nothing, and one emptied in the client is empty
//! text. Both say nothing. Each row is given with what it leaves unsaid
//! filled in: by what whoever asks hands it (`fill`), and otherwise by the
//! kind of cell the table declares (nothing for a number, no for a flag,
//! none for a list, empty text for the rest).
//!
//! With it comes what can be said of rows together and nothing more: which
//! say in one cell what an earlier row says (`once`), which were there and
//! are no longer (`had`), what a number comes to over all of them
//! (`adding`), for each row what a number comes to over those rows of
//! another list that say the same in a cell (`counting`), and which rows
//! say what is asked of some of their cells (`saying`).
//!
//! It knows nothing of what any table is for. What a figure means, and
//! what is done about a row that says what another says, is whoever asks.
//! The kernel stands on nothing here, the language included, so the two
//! things of the language it goes by are written out: a value as text
//! ([`display`]) and a name as it is written ([`slug`]), each held to the
//! language's own by a test of the studio's suite.

use serde_json::{Map, Value, json};

/// A cell a table declares: its name and the kind of thing it holds.
#[derive(Clone, Copy, Debug)]
pub struct Cell<'a> {
    pub name: &'a str,
    pub kind: &'a str,
}

/// A value as text, as the language writes one out: text as it is,
/// nothing as nothing, and anything else as JSON writes it
/// (`blueprint::expr::display`).
pub fn display(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// A name as it is written: lowercase ASCII words joined by single dashes
/// (`blueprint::expr::slug`).
pub fn slug(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

/// A cell that says something: one never filled says nothing, and neither
/// does one that was emptied.
fn said(cell: Option<&Value>) -> Option<&Value> {
    match cell {
        None | Some(Value::Null) => None,
        Some(Value::String(text)) if text.trim().is_empty() => None,
        Some(value) => Some(value),
    }
}

/// The words a form begins with: what is handed for a cell where that is
/// worked out from the row's other cells and is not a value.
const FORMS: &[&str] = &["like", "slug_of", "by", "share", "least"];

/// Whether what is handed for a cell is a form ([`comes_to`]) and not a
/// value: a mapping that says one of the words a form begins with.
fn is_form(handed: &Value) -> bool {
    handed.as_object().is_some_and(|form| FORMS.iter().any(|word| form.contains_key(*word)))
}

/// Whether a cell says something a form can go by: not nothing, not empty
/// text or an empty list, and not a number that is nothing or less.
fn something(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::String(text)) => !text.trim().is_empty(),
        Some(Value::Array(items)) => !items.is_empty(),
        Some(Value::Number(number)) => number.as_f64().is_some_and(|number| number > 0.0),
        Some(_) => true,
    }
}

/// How deep a form may hold forms: more is a mistake, not a rule.
const DEEPEST: usize = 6;

/// What a form comes to for a row, as the row is: a value is itself, and
///
/// - `{ like: <cell> }` is what another cell says, and with `else: <form>`
///   that form where the cell says nothing (or a number not above nothing);
/// - `{ slug_of: <cell> }` is what another cell says, as a name is written;
/// - `{ by: <cell>, <said>: <form>, ..., otherwise: <form> }` is the form
///   beside what the cell says, as it is written out (`work`, `true`, `0`),
///   and `otherwise` where none is beside it;
/// - `{ share: <form>, each: <form> }` is how many times `each` goes into
///   `share`, and never fewer than one: an `each` of nothing takes all of
///   it, which is once;
/// - `{ least: [<form>, ...] }` is the least of what the forms come to
///   that is a number above nothing: one that says nothing, or a number
///   not above nothing, is passed over, and where every one is, nothing.
///
/// A table is what its blueprint hands: nothing here knows what any cell
/// means. A form this cannot read is refused, saying which row.
fn comes_to(
    table: &str,
    id: &str,
    form: &Value,
    row: &Map<String, Value>,
    depth: usize,
) -> Result<Value, String> {
    if !is_form(form) {
        return Ok(form.clone());
    }
    let refused = |why: &str| {
        format!(
            "what the row {id} of `{table}` is given cannot be worked out from {form}: {why}. A cell is given a              value, `{{ like: <cell> }}` (with `else` for where that says nothing), `{{ slug_of: <cell> }}`,              `{{ by: <cell>, <what it says>: ..., otherwise: ... }}`, `{{ share: ..., each: ... }}` or `{{ least: [...] }}`"
        )
    };
    if depth > DEEPEST {
        return Err(refused("it holds forms too deep to be meant"));
    }
    let said = form.as_object().expect("a form is a mapping");
    let only = |allowed: &[&str]| said.keys().all(|key| allowed.contains(&key.as_str()));
    if let Some(cell) = said.get("like") {
        let cell = cell
            .as_str()
            .filter(|_| only(&["like", "else"]))
            .ok_or_else(|| refused("`like` names one cell"))?;
        return match (something(row.get(cell)), said.get("else")) {
            (false, Some(other)) => comes_to(table, id, other, row, depth + 1),
            _ => Ok(row.get(cell).cloned().unwrap_or_else(|| json!(""))),
        };
    }
    if let Some(cell) = said.get("slug_of") {
        let cell = cell
            .as_str()
            .filter(|_| only(&["slug_of"]))
            .ok_or_else(|| refused("`slug_of` names one cell"))?;
        return Ok(json!(row.get(cell).and_then(Value::as_str).map(slug).unwrap_or_default()));
    }
    if let Some(cell) = said.get("by") {
        let cell = cell.as_str().ok_or_else(|| refused("`by` names one cell"))?;
        let says = row.get(cell).map(display).unwrap_or_default();
        let beside = said.get(&says).filter(|_| says != "by" && says != "otherwise");
        return match beside.or_else(|| said.get("otherwise")) {
            Some(chosen) => comes_to(table, id, chosen, row, depth + 1),
            None => Err(refused(&format!(
                "`{cell}` says `{says}`, which it has nothing beside, and it says no `otherwise`"
            ))),
        };
    }
    if let Some(forms) = said.get("least") {
        let forms = forms
            .as_array()
            .filter(|_| only(&["least"]))
            .ok_or_else(|| refused("`least` is a list of forms, and goes with nothing else"))?;
        let mut least: Option<f64> = None;
        for form in forms {
            let came = comes_to(table, id, form, row, depth + 1)?;
            if let Some(number) = came.as_f64().filter(|number| *number > 0.0) {
                least = Some(least.map_or(number, |least| least.min(number)));
            }
        }
        return Ok(match least {
            Some(least) if least.fract() == 0.0 => json!(least as i64),
            Some(least) => json!(least),
            None => Value::Null,
        });
    }
    let (Some(share), Some(each)) = (said.get("share"), said.get("each")) else {
        return Err(refused("`share` goes with `each`"));
    };
    if !only(&["share", "each"]) {
        return Err(refused("`share` goes with `each` and nothing else"));
    }
    let whole = |form: &Value| -> Result<i64, String> {
        let came = comes_to(table, id, form, row, depth + 1)?;
        Ok(came.as_i64().or_else(|| came.as_f64().map(|number| number as i64)).unwrap_or(0))
    };
    let (share, each) = (whole(share)?, whole(each)?);
    let each = if each > 0 { each } else { share };
    Ok(json!((share / each.max(1)).max(1)))
}

/// The kind of thing a cell holds: as its table declares it, and where the
/// table says nothing of it, as what was handed for it is.
fn kind<'a>(fields: &'a [Cell<'a>], fill: &Map<String, Value>, cell: &str) -> &'a str {
    if let Some(field) = fields.iter().find(|field| field.name == cell) {
        return field.kind;
    }
    match fill.get(cell) {
        Some(Value::Number(_)) => "number",
        Some(Value::Bool(_)) => "flag",
        Some(Value::Array(_)) => "list",
        _ => "text",
    }
}

/// A number as a cell may hold it: written as one, or as text that is one.
fn number(table: &str, id: &str, cell: &str, written: &Value) -> Result<Value, String> {
    let wrong = |written: &str| {
        format!(
            "`{cell}` of the row {id} of `{table}` is a whole number, and `{written}` is not one. Put a number \
             there, or empty it"
        )
    };
    match written {
        Value::Number(number) => Ok(number
            .as_i64()
            .map_or_else(|| json!(number.as_f64().unwrap_or(0.0) as i64), |whole| json!(whole))),
        Value::String(text) => text.trim().parse::<i64>().map(|whole| json!(whole)).map_err(|_| wrong(text)),
        other => Err(wrong(&other.to_string())),
    }
}

/// What a row counts for in a sum: the number in the cell, and nothing
/// where it says none.
fn counted(row: &Map<String, Value>, cell: &str) -> i64 {
    row.get(cell).and_then(Value::as_i64).unwrap_or(0)
}

fn so(row: &Map<String, Value>, cell: &str) -> bool {
    row.get(cell).and_then(Value::as_bool).unwrap_or(false)
}

/// What is asked of the rows together.
#[derive(Default)]
pub struct Asked<'a> {
    /// What a cell that says nothing is taken to say.
    pub fill: Option<&'a Map<String, Value>>,
    /// The cell in which each row is to say something no earlier row says.
    pub once: Option<&'a str>,
    /// The rows there were before, by id.
    pub had: Option<&'a Value>,
    /// The cell whose numbers are added up over the rows.
    pub adding: Option<&'a str>,
    /// For each row, figures over the rows of other lists that say what it
    /// says in a cell: by the name each is given.
    pub counting: Option<&'a Map<String, Value>>,
    /// What rows are to say, by cell, to be among `those`.
    pub saying: Option<&'a Map<String, Value>>,
    /// What each row is given, by cell, once it is filled, marked and
    /// counted: a value or a form, worked out from the row as it then is.
    pub giving: Option<&'a Map<String, Value>>,
    /// What one cell says over the rows that say something, by the name
    /// each list is given: `{ of: <cell>, saying: { ... } }`.
    pub gathering: Option<&'a Map<String, Value>>,
}

/// What a row is to say in a cell to be one of those asked for.
enum Wanted<'a> {
    /// Exactly this.
    Is(&'a Value),
    /// Anything but this.
    Not(&'a Value),
    /// Any one of these.
    In(&'a [Value]),
    /// Something, or nothing.
    Said(bool),
}

/// What `saying` asks of one cell: a value, `{ not: <value> }`,
/// `{ in: [<value>, ...] }` for any one of those, or `{ said: true }` (or
/// `false`). Anything else that looks like one of those and is not is
/// refused: nothing is guessed at.
fn wanted<'a>(cell: &str, asked: &'a Value) -> Result<Wanted<'a>, String> {
    let Some(form) = asked.as_object() else { return Ok(Wanted::Is(asked)) };
    match (form.len(), form.get("not"), form.get("said"), form.get("in")) {
        (1, Some(value), None, None) => Ok(Wanted::Not(value)),
        (1, None, Some(Value::Bool(flag)), None) => Ok(Wanted::Said(*flag)),
        (1, None, None, Some(Value::Array(any))) => Ok(Wanted::In(any)),
        _ => Err(format!(
            "`saying` says of a cell what a row is to say in it: a value, `{{ not: <value> }}` for anything \
             else, `{{ in: [<value>, ...] }}` for any one of those, or `{{ said: true }}` for something and \
             `{{ said: false }}` for nothing; of `{cell}` it says {asked}"
        )),
    }
}

/// Whether what `saying` asks, cell by cell, can be asked: each a value or
/// one of the forms [`read`] takes. Why not, in a sentence.
pub fn saying_fits(saying: &Map<String, Value>) -> Result<(), String> {
    saying.iter().try_for_each(|(cell, asked)| wanted(cell, asked).map(|_| ()))
}

/// Whether a row's cell says what is wanted of it. A cell that was filled
/// in for saying nothing (empty text, no list) still says nothing.
fn says(value: Option<&Value>, wanted: &Wanted<'_>) -> bool {
    let nothing = match value {
        None | Some(Value::Null) => true,
        Some(Value::String(text)) => text.trim().is_empty(),
        Some(Value::Array(items)) => items.is_empty(),
        Some(_) => false,
    };
    match wanted {
        Wanted::Is(this) => value == Some(*this),
        Wanted::Not(this) => value != Some(*this),
        Wanted::In(any) => value.is_some_and(|value| any.contains(value)),
        Wanted::Said(something) => *something != nothing,
    }
}

/// What `saying` asks, cell by cell, held to the cells the rows have.
fn asked_of<'a>(
    table: &str,
    rows: &[Map<String, Value>],
    saying: &'a Map<String, Value>,
) -> Result<Vec<(&'a str, Wanted<'a>)>, String> {
    let mut asked_of = Vec::with_capacity(saying.len());
    for (cell, what) in saying {
        if cell != "id" && !rows.is_empty() && rows.iter().all(|row| !row.contains_key(cell)) {
            let mut has: Vec<&str> = rows[0].keys().map(String::as_str).collect();
            has.sort_unstable();
            return Err(format!(
                "`saying` names the cell `{cell}`, which no row of `{table}` has; its rows have {}",
                has.join(", ")
            ));
        }
        asked_of.push((cell.as_str(), wanted(cell, what)?));
    }
    Ok(asked_of)
}

/// One row as it is taken: its id, and each cell with what it leaves unsaid
/// filled in.
fn row(
    table: &str,
    fields: &[Cell<'_>],
    fill: &Map<String, Value>,
    id: &str,
    data: &Map<String, Value>,
) -> Result<Map<String, Value>, String> {
    let mut row = Map::new();
    row.insert("id".into(), json!(id));
    let declared = fields.iter().map(|field| field.name);
    let handed =
        fill.keys().map(String::as_str).filter(|cell| !fields.iter().any(|field| field.name == *cell));
    let cells: Vec<&str> = declared.chain(handed).collect();
    // What the row says of a cell the table declares; of one only `fill`
    // names, nothing: anything the record holds under that name is a
    // column somebody added.
    let written = |cell: &str| match fields.iter().any(|field| field.name == cell) {
        true => said(data.get(cell)),
        false => None,
    };
    for cell in &cells {
        let kind = kind(fields, fill, cell);
        let value = match (written(cell), kind) {
            (Some(written), "number") => number(table, id, cell, written)?,
            (Some(written), "flag") => json!(written.as_bool().unwrap_or(false)),
            (Some(written), "list" | "records") if !written.is_array() => json!([]),
            (Some(written), _) => written.clone(),
            (None, kind) => match fill.get(*cell) {
                // Filled in from other cells once every cell has what it says.
                Some(handed) if is_form(handed) => Value::Null,
                Some(handed) => handed.clone(),
                None => match kind {
                    "number" => Value::Null,
                    "flag" => json!(false),
                    "list" | "records" => json!([]),
                    _ => json!(""),
                },
            },
        };
        row.insert((*cell).to_string(), value);
    }
    for cell in &cells {
        if written(cell).is_some() {
            continue;
        }
        let Some(form) = fill.get(*cell).filter(|handed| is_form(handed)) else { continue };
        let taken = comes_to(table, id, form, &row, 0)?;
        row.insert((*cell).to_string(), taken);
    }
    Ok(row)
}

/// The rows of a table as they are taken, and what is asked of them together.
pub fn read(
    table: &str,
    fields: &[Cell<'_>],
    records: &[(String, Map<String, Value>)],
    asked: &Asked<'_>,
) -> Result<Value, String> {
    let nothing = Map::new();
    let fill = asked.fill.unwrap_or(&nothing);
    let mut rows = Vec::with_capacity(records.len());
    for (id, data) in records {
        rows.push(row(table, fields, fill, id, data)?);
    }
    // A row that says in one cell what an earlier row says: which, and the
    // row it says it after.
    let mut again = Vec::new();
    if let Some(cell) = asked.once {
        for at in 0..rows.len() {
            let says = rows[at].get(cell).cloned().unwrap_or(Value::Null);
            let first = rows[..at].iter().find(|earlier| earlier.get(cell) == Some(&says)).cloned();
            rows[at].insert("again".into(), json!(first.is_some()));
            if let Some(first) = first {
                let mut later = rows[at].clone();
                later.insert("first".into(), Value::Object(first));
                again.push(Value::Object(later));
            }
        }
    }
    for (name, of) in asked.counting.into_iter().flatten() {
        let (others, by, sum) = (
            of.get("of").and_then(Value::as_array),
            of.get("by").and_then(Value::as_str),
            of.get("sum").and_then(Value::as_str),
        );
        let (Some(others), Some(by)) = (others, by) else {
            return Err(format!(
                "`counting` says, for each figure, the rows it is counted over (`of`) and the cell in which they \
                 say what the row says (`by`), with `sum` for a number added up; `{name}` says less"
            ));
        };
        let unless = of.get("unless").and_then(Value::as_str);
        for row in &mut rows {
            let same: Vec<&Map<String, Value>> = others
                .iter()
                .filter_map(Value::as_object)
                .filter(|other| other.get(by).is_some() && other.get(by) == row.get(by))
                .collect();
            let counts = |other: &&&Map<String, Value>| unless.is_none_or(|cell| !so(other, cell));
            let comes_to: i64 = match sum {
                Some(cell) => same.iter().filter(counts).map(|other| counted(other, cell)).sum(),
                None => same.iter().filter(counts).count() as i64,
            };
            row.insert(name.clone(), json!(comes_to));
            row.insert(format!("{name}_rows"), json!(same.len()));
        }
    }
    // What each row is given, from the row as it is before any of it is given.
    if let Some(giving) = asked.giving {
        for (row, (id, _)) in rows.iter_mut().zip(records) {
            let mut given = Vec::with_capacity(giving.len());
            for (cell, form) in giving {
                given.push((cell.clone(), comes_to(table, id, form, row, 0)?));
            }
            row.extend(given);
        }
    }
    let sum: i64 = asked
        .adding
        .map(|cell| rows.iter().filter(|row| !so(row, "again")).map(|row| counted(row, cell)).sum())
        .unwrap_or(0);
    let ids: Vec<&str> = records.iter().map(|(id, _)| id.as_str()).collect();
    let had = asked.had.and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str);
    let gone: Vec<&str> = had.filter(|id| !ids.contains(id)).collect();
    // The rows that say what is asked of each cell named, and the rest.
    let (mut those, mut others, mut chosen) = (Vec::new(), Vec::new(), Vec::new());
    if let Some(saying) = asked.saying {
        let asked_of = asked_of(table, &rows, saying)?;
        for (row, id) in rows.iter().zip(&ids) {
            match asked_of.iter().all(|(cell, wanted)| says(row.get(*cell), wanted)) {
                true => {
                    those.push(*id);
                    chosen.push(row);
                }
                false => others.push(*id),
            }
        }
    }
    // What one cell says over the rows that say something, list by list.
    let mut gathered = Map::new();
    for (name, of) in asked.gathering.into_iter().flatten() {
        let (Some(cell), saying) = (of.get("of").and_then(Value::as_str), of.get("saying")) else {
            return Err(format!(
                "`gathering` says, for each list, the cell whose values it is (`of`) and what a row is to say                  to be in it (`saying`, as `saying` is written); `{name}` says {of}"
            ));
        };
        let nothing = Map::new();
        let asked_of = asked_of(table, &rows, saying.and_then(Value::as_object).unwrap_or(&nothing))?;
        let values: Vec<&Value> = rows
            .iter()
            .filter(|row| asked_of.iter().all(|(cell, wanted)| says(row.get(*cell), wanted)))
            .filter_map(|row| row.get(cell))
            .collect();
        gathered.insert(name.clone(), json!(values));
    }
    Ok(json!({
        "rows": rows, "ids": ids, "count": ids.len(), "gone": gone, "went": gone.len(), "again": again,
        "sum": sum, "those": those, "others": others, "chosen": chosen, "gathered": gathered,
    }))
}
