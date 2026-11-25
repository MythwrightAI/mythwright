//! The parts of something a blueprint tells somebody: what a told line
//! holds beside its words, chosen by the blueprint that tells, and what
//! each comes to when it is read and pressed.
//!
//! A part is a mapping with a `kind` and the words that kind takes. Two
//! kinds are there:
//!
//! - `links`: addresses, each shown as a link, as an address in the
//!   words is;
//! - `cards`: the cards of a project's board that are in one column, or
//!   that hold one label, each shown with what was said on it since it
//!   came there, to be passed or failed where they are read. A part of a
//!   column keeps which cards were there as it was told ([`keep`]), and
//!   lists those wherever each is when it is read, so that a line goes on
//!   saying what it was told of after its cards have moved on; one a
//!   program from before kept, and one of a label, lists what is there
//!   when it is read. Failed, a
//!   card is moved to `fail_to` (`ready` unless said) with why it failed
//!   as what was said as it was moved, which is what its agent is told
//!   when it is next given the card; passed, it is approved (`card.approved`,
//!   as whoever passed it, a line of its history and no comment), and then
//!   moved to `pass_to`, or `pass_press` of its column pressed on it, or
//!   nothing more where the part says neither;
//! - `message`: one message of a channel, opened where it was said: in
//!   its thread where it is in one (what a discussion between agents
//!   came to, for whoever is to read or decide it);
//! - `rows`: rows of a table of a project's, by their ids or by what a
//!   cell says (as `rows.read`'s `saying` chooses), with the columns it
//!   names in its order, each drawn as the table draws it ([`columns`],
//!   [`chosen`]);
//! - `runs`: the runs of an instance, of the instances under a key (a
//!   pipeline applied again and again), or of what ran under an environment, each with what it is a run of,
//!   how it went, when and how long ([`run_called`]);
//! - `measures`: measurements by their names and labels, each its latest
//!   value beside the median of its last few ([`measured`]): counted, and
//!   never scored.
//!
//! Every kind may say which tab of the line's page it is in (`tab`, a
//! name): the parts that say none are on a first tab with no name, and a
//! page with one tab draws no row of tabs ([`tabbed`]).
//!
//! A part is checked and filled in when it is told ([`read`]) and kept on
//! the line as a value, so that a line says what it was told with. What
//! it comes to is asked when it is read and pressed, of the cards as they
//! are then: which it lists ([`Cards::lists`]), what a press comes to
//! ([`press`]), and what was said on a card since it came where it is
//! listed ([`thread`]). Everything here is a function of what it is
//! handed; who reads a board, moves a card and asks who may is whoever
//! keeps them.

use serde_json::{Map, Value, json};

mod read;
pub use read::{Chosen, MOST_SAID, Measured, brief, chosen, columns, in_seconds, measured, run_called, tabs};

/// The kinds a part may be, in the order they are offered.
pub const KINDS: &[&str] = &["links", "cards", "message", "rows", "runs", "measures"];
/// The most parts a line holds, and the most addresses a part of links.
pub const MOST: usize = 10;
pub const MOST_LINKS: usize = 50;
/// Where a card that fails goes, where its part says nothing.
pub const FAIL_TO: &str = "ready";
/// The most rows a part of rows lists, by ids or as chosen, and the most
/// columns it shows.
pub const MOST_ROWS: usize = 100;
pub const MOST_COLUMNS: usize = 12;
/// How many runs a part of runs lists where it says none, and the most.
pub const RUNS: usize = 10;
pub const MOST_RUNS: usize = 50;
/// The most measurements a part of measures names, how many of each's
/// latest values its median is of where it says none, and the most.
pub const MOST_MEASURES: usize = 20;
pub const LAST: usize = 10;
pub const MOST_LAST: usize = 100;
/// The longest a tab's name may be, in letters.
pub const MOST_TAB: usize = 40;

/// What is written inside a step's `parts`: each part, as one of them is
/// said (`blueprint::inside`), for the editor, the notes and the client.
pub const INSIDE: &[(&str, &str, &str)] = &[
    ("kind", "string", "what the part is"),
    (
        "tab",
        "string",
        "the tab of the line's page it is in, by its name; the parts that say none are on a first tab with no name",
    ),
    ("kind.links", "word", "addresses, each shown as a link"),
    (
        "kind.cards",
        "word",
        "the cards of a board in one column as it is told, each listed where it is when it is read, or the \
         cards holding one label; each to be passed or failed where it is read while it is where the part says",
    ),
    (
        "kind.message",
        "word",
        "one message of a channel, opened where it was said: in its thread where it is in one",
    ),
    (
        "kind.rows",
        "word",
        "rows of a table of a project's, by their ids or by what a cell says, with the columns named, each drawn \
         as its table draws it, as whoever reads the line may read the table",
    ),
    (
        "kind.runs",
        "word",
        "the runs of an instance, of what ran under a key, or of what ran under an environment: what each is a \
         run of, how it went, when and how long, each opening its run's page",
    ),
    (
        "kind.measures",
        "word",
        "measurements by their names and labels, each its latest value beside the median of its last few",
    ),
    ("links", "list", "of `links`: the addresses, each a page on the web (http or https)"),
    ("channel", "string", "of `message`: the channel's id"),
    ("message", "string", "of `message`: the message's id"),
    (
        "project",
        "string",
        "of `cards`, `rows` and `runs`: whose board, table or runs; the project the text is applied for where it says none",
    ),
    ("table", "string", "of `rows`: the table, by its name"),
    ("ids", "list", "of `rows`: the rows, each by its id, in the order listed; or say `saying`"),
    (
        "saying",
        "object",
        "of `rows`: what a row is to say, by cell, to be listed, as `rows.read`'s `saying` is written; or say `ids`",
    ),
    (
        "columns",
        "list",
        "of `rows`: the columns shown, in order, each by its name (default: the table's own)",
    ),
    (
        "instance",
        "string",
        "of `runs`: the instance whose runs are listed, by its id; or say `key` or `environment`",
    ),
    (
        "key",
        "string",
        "of `runs`: the key whose instances' runs are listed, of the project: a pipeline's, applied again and again",
    ),
    ("environment", "string", "of `runs`: the environment of the project whose runs are listed"),
    ("most", "int", "of `runs`: how many of the latest are listed (default 10, at most 50)"),
    ("measures", "list", "of `measures`: the measurements, each by its name"),
    (
        "labels",
        "object",
        "of `measures`: the labels each is to be measured by, each with its value as a word",
    ),
    (
        "last",
        "int",
        "of `measures`: how many of each's latest values its median is of (default 10, at most 100)",
    ),
    ("column", "string", "of `cards`: the column whose cards are listed; or say `label`"),
    ("label", "string", "of `cards`: the label the cards listed hold; or say `column`"),
    ("fail_to", "string", "of `cards`: the column a card that fails is moved to, with why (default `ready`)"),
    (
        "pass_to",
        "string",
        "of `cards`: the column a card that passes is moved to, once it is approved; or say `pass_press`, or neither, and it stays",
    ),
    (
        "pass_press",
        "string",
        "of `cards`: what its column offers to be pressed that is pressed on a card that passes, once it is approved",
    ),
    ("reason", "bool", "of `cards`: a card is failed only with why (default true)"),
];

/// One part, as a line keeps it and reads it back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Part {
    Links(Vec<String>),
    Cards(Cards),
    /// A channel's id, and the id of one of its messages.
    Message {
        channel: String,
        message: String,
    },
    /// A change somebody asked for, by its project and its id
    /// (`kernel::approvals`): made by the daemon as it tells whoever
    /// could make it, and never by a step, which [`read`] refuses.
    Change {
        project: String,
        ask: String,
    },
    /// A run that waits for somebody to let it go on, by its project (none
    /// for a run of no project's) and the ask's id (`kernel::go`): made by
    /// the daemon as an approval step tells whoever may decide it, and
    /// never by a step, which [`read`] refuses.
    Go {
        project: String,
        ask: String,
    },
    Rows(Rows),
    Runs(Runs),
    Measures(Measures),
}

/// A part of rows: which table, which of its rows, and which columns.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rows {
    pub project: String,
    pub table: String,
    /// The rows by their ids, in that order; nothing, where `saying` chooses them.
    pub ids: Option<Vec<String>>,
    /// What a row is to say, by cell, to be listed (`kernel::rows`).
    pub saying: Option<Map<String, Value>>,
    /// The columns shown, in order; none, where the table's own are.
    pub columns: Vec<String>,
}

/// A part of runs: whose runs, and how many of the latest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Runs {
    /// The project, or nothing where an instance of no project's is named.
    pub project: String,
    pub of: RunsOf,
    pub most: usize,
}

/// Whose runs a part of runs lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunsOf {
    Instance(String),
    Key(String),
    Environment(String),
}

/// A part of measures: which measurements, by which labels, and how many
/// of each's latest values its median is of.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Measures {
    pub names: Vec<String>,
    pub labels: Vec<(String, String)>,
    pub last: usize,
}

/// A part of cards: which it lists, and what passing and failing do.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cards {
    pub project: String,
    /// The column whose cards it lists, or nothing where it is a label's.
    pub column: String,
    /// The label whose cards it lists, or nothing where it is a column's.
    pub label: String,
    pub fail_to: String,
    /// Where a card that passes is moved; nothing, where it is not.
    pub pass_to: String,
    /// What of its column is pressed on a card that passes; nothing, where nothing is.
    pub pass_press: String,
    /// Whether a card is failed only with why.
    pub reason: bool,
    /// The cards its column held as it was told, in the column's order;
    /// nothing, where the part keeps none (one of a label, one a program
    /// from before kept) and lists what is there when it is read.
    pub kept: Option<Vec<String>>,
}

/// The key a part keeps its cards under: added as it is told, and never
/// written by a step (a step's part that says it is refused, [`read`]).
const KEPT: &str = "kept";

/// The project and the column whose cards a part kept as it is told
/// would keep, or nothing where it keeps none: a part of cards that says
/// a column.
pub fn keeps(part: &Value) -> Option<(String, String)> {
    let column = part["column"].as_str().filter(|column| !column.is_empty());
    match (part["kind"].as_str(), part["project"].as_str(), column) {
        (Some("cards"), Some(project), Some(column)) => Some((project.to_string(), column.to_string())),
        _ => None,
    }
}

/// A part as it is kept, with the cards its column held as it was told,
/// by their ids in the column's order.
pub fn keep(part: &mut Value, cards: Vec<String>) {
    if let Value::Object(part) = part {
        part.insert(KEPT.into(), json!(cards));
    }
}

/// The parts a line keeps, as they are handed out with the line: without
/// the cards a part kept, which are read for whoever reads the line only
/// as they may read the board.
pub fn shown(kept: &Value) -> Value {
    let mut shown = kept.clone();
    for part in shown.as_array_mut().into_iter().flatten() {
        if let Value::Object(part) = part {
            part.remove(KEPT);
        }
    }
    shown
}

fn word<'a>(part: &'a Map<String, Value>, key: &str) -> Result<&'a str, String> {
    match part.get(key) {
        None | Some(Value::Null) => Ok(""),
        Some(Value::String(said)) => Ok(said.trim()),
        Some(other) => Err(format!("`{key}` of a part is a word; it was given {other}")),
    }
}

/// Whether `address` is one a link may lead to: the web, and one word.
pub fn is_address(address: &str) -> bool {
    let rest = address.strip_prefix("https://").or_else(|| address.strip_prefix("http://"));
    rest.is_some_and(|rest| !rest.is_empty() && !address.chars().any(char::is_whitespace))
}

/// The parts a step asked to tell with, checked, each as it is kept:
/// what it says filled in where it says nothing, `project` being the
/// project the text is applied for. Why not, in a sentence.
pub fn read(parts: &Value, project: Option<&str>) -> Result<Vec<Value>, String> {
    let listed = match parts {
        Value::Null => return Ok(Vec::new()),
        Value::Array(listed) => listed,
        other => return Err(format!("`parts` is a list of parts, each with a `kind`; it was given {other}")),
    };
    if listed.len() > MOST {
        return Err(format!("a line holds {MOST} parts at most; this was given {}", listed.len()));
    }
    listed
        .iter()
        .enumerate()
        .map(|(n, part)| one(part, project).map_err(|why| format!("part {}: {why}", n + 1)))
        .collect()
}

fn one(part: &Value, project: Option<&str>) -> Result<Value, String> {
    let Value::Object(part) = part else {
        return Err(format!("a part is a mapping with a `kind`; it was given {part}"));
    };
    let kind = word(part, "kind")?;
    let takes: &[&str] = match kind {
        "links" => &["kind", "tab", "links"],
        "cards" => {
            &["kind", "tab", "project", "column", "label", "fail_to", "pass_to", "pass_press", "reason"]
        }
        "message" => &["kind", "tab", "channel", "message"],
        "rows" => &["kind", "tab", "project", "table", "ids", "saying", "columns"],
        "runs" => &["kind", "tab", "project", "instance", "key", "environment", "most"],
        "measures" => &["kind", "tab", "measures", "labels", "last"],
        "" => return Err(format!("a part says its `kind`: {}", KINDS.join(" or "))),
        other => return Err(format!("`{other}` is no kind of part; a part is {}", KINDS.join(" or "))),
    };
    if let Some(key) = part.keys().find(|key| !takes.contains(&key.as_str())) {
        return Err(format!("a part of `{kind}` takes {}, and not `{key}`", takes[1..].join(", ")));
    }
    let tab = word(part, "tab")?;
    if tab.chars().count() > MOST_TAB || tab.contains('\n') {
        return Err(format!("`tab` is the name of a tab, on one line of {MOST_TAB} letters at most"));
    }
    let mut kept = filled(kind, part, project)?;
    if !tab.is_empty()
        && let Value::Object(kept) = &mut kept
    {
        kept.insert("tab".into(), json!(tab));
    }
    Ok(kept)
}

/// A list of words a part says under `key`, at most `most`: nothing where
/// it says none.
fn words(part: &Map<String, Value>, key: &str, most: usize) -> Result<Vec<String>, String> {
    let listed = match part.get(key) {
        None | Some(Value::Null) => return Ok(Vec::new()),
        Some(Value::Array(listed)) => listed,
        Some(other) => return Err(format!("`{key}` of a part is a list of words; it was given {other}")),
    };
    if listed.len() > most {
        return Err(format!("`{key}` of a part lists {most} at most; it was given {}", listed.len()));
    }
    let each = |one: &Value| match one.as_str().map(str::trim) {
        Some(said) if !said.is_empty() => Ok(said.to_string()),
        _ => Err(format!("`{key}` of a part is a list of words; it was given {one}")),
    };
    listed.iter().map(each).collect()
}

/// A whole number a part says under `key`, from one to `most`: `usual`
/// where it says none.
fn count(part: &Map<String, Value>, key: &str, usual: usize, most: usize) -> Result<usize, String> {
    match part.get(key) {
        None | Some(Value::Null) => Ok(usual),
        Some(said) => match said.as_u64().map(|n| n as usize) {
            Some(n) if (1..=most).contains(&n) => Ok(n),
            _ => Err(format!("`{key}` of a part is a whole number from 1 to {most}; it was given {said}")),
        },
    }
}

/// The project a part is of: what it says, or the one the text is applied
/// for; nothing where neither says one.
fn of_project<'a>(part: &'a Map<String, Value>, project: Option<&'a str>) -> Result<&'a str, String> {
    Ok(match word(part, "project")? {
        "" => project.unwrap_or_default(),
        said => said,
    })
}

/// A part of `kind` as it is kept, with what it leaves unsaid filled in.
fn filled(kind: &str, part: &Map<String, Value>, project: Option<&str>) -> Result<Value, String> {
    match kind {
        "links" => return links(part),
        "message" => {
            let (channel, message) = (word(part, "channel")?, word(part, "message")?);
            if channel.is_empty() || message.is_empty() {
                return Err("a part of `message` says the `channel` and the `message`, each by its id".into());
            }
            return Ok(json!({ "kind": "message", "channel": channel, "message": message }));
        }
        "rows" => return rows(part, project),
        "runs" => return runs(part, project),
        "measures" => return measures(part),
        _ => {}
    }
    let (column, label) = (word(part, "column")?, word(part, "label")?);
    if column.is_empty() == label.is_empty() {
        return Err(
            "a part of `cards` lists the cards of one `column` or of one `label`: say one of the two".into(),
        );
    }
    let project = of_project(part, project)?;
    if project.is_empty() {
        return Err(
            "a part of `cards` is of a project's board: say `project`, or apply the text for one".into()
        );
    }
    let reason = match part.get("reason") {
        None | Some(Value::Null) => true,
        Some(Value::Bool(reason)) => *reason,
        Some(other) => return Err(format!("`reason` is true or false; it was given {other}")),
    };
    let fail_to = match word(part, "fail_to")? {
        "" => FAIL_TO,
        said => said,
    };
    let (pass_to, pass_press) = (word(part, "pass_to")?, word(part, "pass_press")?);
    if !pass_to.is_empty() && !pass_press.is_empty() {
        return Err(
            "a card that passes is moved (`pass_to`) or has something of its column pressed (`pass_press`): say one of the two".into(),
        );
    }
    if !column.is_empty() && (fail_to == column || pass_to == column) {
        return Err(format!(
            "a card passed or failed leaves `{column}`: `fail_to` and `pass_to` say another column"
        ));
    }
    Ok(json!({
        "kind": "cards", "project": project, "column": column, "label": label,
        "fail_to": fail_to, "pass_to": pass_to, "pass_press": pass_press, "reason": reason,
    }))
}

fn links(part: &Map<String, Value>) -> Result<Value, String> {
    let links = match part.get("links") {
        Some(Value::Array(links)) if !links.is_empty() => links,
        _ => return Err("a part of `links` says `links`: a list of addresses".into()),
    };
    if links.len() > MOST_LINKS {
        return Err(format!("a part of `links` holds {MOST_LINKS} addresses at most"));
    }
    for link in links {
        if !link.as_str().is_some_and(is_address) {
            return Err(format!("{link} is no address a link leads to: it begins `https://` or `http://`"));
        }
    }
    Ok(json!({ "kind": "links", "links": links }))
}

fn rows(part: &Map<String, Value>, project: Option<&str>) -> Result<Value, String> {
    let table = word(part, "table")?;
    if table.is_empty() {
        return Err("a part of `rows` says the `table` its rows are of, by its name".into());
    }
    let project = of_project(part, project)?;
    if project.is_empty() {
        return Err(
            "a part of `rows` is of a project's table: say `project`, or apply the text for one".into()
        );
    }
    let ids = words(part, "ids", MOST_ROWS)?;
    let saying = match part.get("saying") {
        None | Some(Value::Null) => None,
        Some(Value::Object(saying)) => {
            crate::rows::saying_fits(saying)?;
            Some(saying)
        }
        Some(other) => {
            return Err(format!("`saying` of a part is what a row is to say, by cell; it was given {other}"));
        }
    };
    if ids.is_empty() == saying.is_none() {
        return Err(
            "a part of `rows` lists rows by their `ids` or by what a cell says (`saying`): say one of the two"
                .into(),
        );
    }
    let mut kept = json!({
        "kind": "rows", "project": project, "table": table,
        "columns": words(part, "columns", MOST_COLUMNS)?,
    });
    match saying {
        Some(saying) => kept["saying"] = json!(saying),
        None => kept["ids"] = json!(ids),
    }
    Ok(kept)
}

fn runs(part: &Map<String, Value>, project: Option<&str>) -> Result<Value, String> {
    let said = [
        ("instance", word(part, "instance")?),
        ("key", word(part, "key")?),
        ("environment", word(part, "environment")?),
    ];
    let named: Vec<&(&str, &str)> = said.iter().filter(|(_, said)| !said.is_empty()).collect();
    let [(by, of)] = named.as_slice() else {
        return Err(
            "a part of `runs` lists the runs of one `instance`, of what ran under a `key`, or of what ran under an \
             `environment`: say one of the three"
                .into(),
        );
    };
    let project = of_project(part, project)?;
    if project.is_empty() && *by != "instance" {
        return Err(format!(
            "a part of `runs` by its `{by}` is of a project's: say `project`, or apply the text for one"
        ));
    }
    let mut kept =
        json!({ "kind": "runs", "project": project, "most": count(part, "most", RUNS, MOST_RUNS)? });
    kept[*by] = json!(of);
    Ok(kept)
}

fn measures(part: &Map<String, Value>) -> Result<Value, String> {
    let names = words(part, "measures", MOST_MEASURES)?;
    if names.is_empty() {
        return Err(
            "a part of `measures` says `measures`: a list of the measurements, each by its name".into()
        );
    }
    let labels = match part.get("labels") {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(labels)) => {
            let mut said = Map::new();
            for (label, value) in labels {
                let value = match value {
                    Value::String(word) => word.trim().to_string(),
                    Value::Number(_) | Value::Bool(_) => value.to_string(),
                    other => {
                        return Err(format!(
                            "`labels` of a part says each label's value as a word; `{label}` is {other}"
                        ));
                    }
                };
                said.insert(label.clone(), json!(value));
            }
            said
        }
        Some(other) => {
            return Err(format!(
                "`labels` of a part is a mapping of each label to its value; it was given {other}"
            ));
        }
    };
    Ok(json!({
        "kind": "measures", "measures": names, "labels": labels,
        "last": count(part, "last", LAST, MOST_LAST)?,
    }))
}

/// The parts a line keeps, read back: what is not one is passed over, as
/// a line a program from before kept says none.
pub fn of(kept: &Value) -> Vec<Part> {
    tabbed(kept).into_iter().map(|(_, part)| part).collect()
}

/// The parts a line keeps, read back, each with the tab it is in: the
/// empty text for the first tab, which has no name.
pub fn tabbed(kept: &Value) -> Vec<(String, Part)> {
    let text = |part: &Value, key: &str| part[key].as_str().unwrap_or_default().to_string();
    let listed = kept.as_array().map(Vec::as_slice).unwrap_or_default();
    let words = |said: &Value| -> Vec<String> {
        said.as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_string).collect()
    };
    listed
        .iter()
        .filter_map(|part| {
            let read = match part["kind"].as_str()? {
                "links" => {
                    let links = part["links"].as_array()?.iter().filter_map(Value::as_str);
                    Part::Links(links.map(str::to_string).collect())
                }
                "message" => Part::Message { channel: text(part, "channel"), message: text(part, "message") },
                "change" => Part::Change { project: text(part, "project"), ask: text(part, "ask") },
                "go" => Part::Go { project: text(part, "project"), ask: text(part, "ask") },
                "cards" => Part::Cards(Cards {
                    project: text(part, "project"),
                    column: text(part, "column"),
                    label: text(part, "label"),
                    fail_to: text(part, "fail_to"),
                    pass_to: text(part, "pass_to"),
                    pass_press: text(part, "pass_press"),
                    reason: part["reason"].as_bool().unwrap_or(true),
                    kept: part[KEPT].as_array().map(|_| words(&part[KEPT])),
                }),
                "rows" => Part::Rows(Rows {
                    project: text(part, "project"),
                    table: text(part, "table"),
                    ids: part["ids"].as_array().map(|_| words(&part["ids"])),
                    saying: part["saying"].as_object().cloned(),
                    columns: words(&part["columns"]),
                }),
                "runs" => {
                    let of =
                        match (part["instance"].as_str(), part["key"].as_str(), part["environment"].as_str())
                        {
                            (Some(instance), _, _) => RunsOf::Instance(instance.to_string()),
                            (_, Some(key), _) => RunsOf::Key(key.to_string()),
                            (_, _, Some(environment)) => RunsOf::Environment(environment.to_string()),
                            _ => return None,
                        };
                    let most = part["most"].as_u64().map_or(RUNS, |most| (most as usize).clamp(1, MOST_RUNS));
                    Part::Runs(Runs { project: text(part, "project"), of, most })
                }
                "measures" => Part::Measures(Measures {
                    names: words(&part["measures"]),
                    labels: part["labels"]
                        .as_object()
                        .into_iter()
                        .flatten()
                        .map(|(label, value)| (label.clone(), crate::rows::display(value)))
                        .collect(),
                    last: part["last"].as_u64().map_or(LAST, |last| (last as usize).clamp(1, MOST_LAST)),
                }),
                _ => return None,
            };
            Some((text(part, "tab"), read))
        })
        .collect()
}

impl Cards {
    /// Whether a card in `status`, holding `labels`, is one it lists.
    pub fn lists(&self, status: &str, labels: &[String]) -> bool {
        match self.column.is_empty() {
            false => status == self.column,
            true => labels.contains(&self.label),
        }
    }

    /// Where its cards are, in words: `in staging`, `labelled try`.
    pub fn listed(&self) -> String {
        match self.column.is_empty() {
            false => format!("in {}", self.column),
            true => format!("labelled {}", self.label),
        }
    }
}

/// Whether a card is passed or failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Fail,
}

impl Verdict {
    pub fn read(word: &str) -> Result<Verdict, String> {
        match word {
            "pass" => Ok(Verdict::Pass),
            "fail" => Ok(Verdict::Fail),
            other => Err(format!("a card is passed or failed: say `pass` or `fail`, and not `{other}`")),
        }
    }
}

/// A card being passed or failed: what is pressed, by whom, and the card
/// as it is.
#[derive(Clone, Debug, Default)]
pub struct Pressing {
    pub id: String,
    pub verdict: Option<Verdict>,
    /// Why, as whoever pressed said it.
    pub reason: String,
    /// Who presses, as an actor (`user:<id>`, `agent:<name>`, `key:<id>`).
    pub by: String,
    pub status: String,
    pub labels: Vec<String>,
    pub assignee: Option<String>,
    /// The agents that worked it, as actors.
    pub worked: Vec<String>,
}

/// What a press comes to. Failed, where the card is moved, with why.
/// Passed, it is approved by whoever passed it, and then moved where the
/// part says, or has what the part says of its column pressed on it, or
/// nothing more: no words are said on it for passing, which are said only
/// where somebody writes some.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Done {
    Moved { to: String, said: String },
    Passed { then: Then },
}

/// What is done with a card once it is approved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Then {
    Nothing,
    Move(String),
    Press(String),
}

/// Whether the card may be passed or failed by whoever presses, whatever
/// is pressed: why not, in a sentence. Whether they may move a card at
/// all is the board's to ask.
pub fn may(cards: &Cards, pressing: &Pressing) -> Result<(), String> {
    let id = &pressing.id;
    if cards.kept.as_ref().is_some_and(|kept| !kept.contains(id)) {
        return Err(format!(
            "{id} is not one of the cards this line was told of: they are those {} as it was told",
            cards.listed()
        ));
    }
    if !cards.lists(&pressing.status, &pressing.labels) {
        return Err(format!(
            "{id} is not one of the cards {} any more: it is in {}, and is passed or failed where it is now",
            cards.listed(),
            pressing.status
        ));
    }
    if pressing.by.starts_with("agent:") {
        let own = pressing.assignee.as_deref() == Some(pressing.by.as_str())
            || pressing.worked.contains(&pressing.by);
        if own {
            let name = pressing.by.trim_start_matches("agent:");
            return Err(format!(
                "{name} worked on {id}, and an agent does not pass or fail its own work: a person, or another agent, does"
            ));
        }
    }
    Ok(())
}

/// What passing or failing a card that a part of cards lists comes to,
/// or why it is refused, in a sentence.
pub fn press(cards: &Cards, pressing: &Pressing) -> Result<Done, String> {
    may(cards, pressing)?;
    let id = &pressing.id;
    let said = pressing.reason.trim().to_string();
    match pressing.verdict {
        None => Err("say whether the card passes or fails: `pass` or `fail`".into()),
        Some(Verdict::Fail) if said.is_empty() && cards.reason => Err(format!(
            "{id} is failed here only with why: say it, and that is what its agent is told when it next has the card"
        )),
        Some(Verdict::Fail) => Ok(Done::Moved { to: cards.fail_to.clone(), said }),
        Some(Verdict::Pass) => Ok(Done::Passed {
            then: match (cards.pass_to.as_str(), cards.pass_press.as_str()) {
                ("", "") => Then::Nothing,
                ("", press) => Then::Press(press.to_string()),
                (to, _) => Then::Move(to.to_string()),
            },
        }),
    }
}

/// A line of a card's history, as much of it as is gone by here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Line {
    pub topic: String,
    /// Who did it, as an actor.
    pub actor: String,
    /// Who did it, by name.
    pub who: String,
    pub ts: i64,
    /// Where a move or an adding put it.
    pub to: String,
    /// What was said: of something said on it, its words; of a question, the question.
    pub words: String,
    /// Whose turn it was, of a line that says one ended: the agent's name.
    pub agent: String,
}

impl Line {
    /// A line as a history keeps it: its topic, who, when, and what the
    /// event said of itself.
    pub fn of(topic: &str, actor: &str, who: &str, ts: i64, data: &Value) -> Line {
        let text =
            |keys: &[&str]| keys.iter().find_map(|key| data[*key].as_str()).unwrap_or_default().to_string();
        Line {
            topic: topic.to_string(),
            actor: actor.to_string(),
            who: who.to_string(),
            ts,
            to: text(&["to", "status"]),
            words: text(&["body", "question"]),
            agent: text(&["agent", "asker"]),
        }
    }

    fn said(&self) -> bool {
        self.topic == COMMENTED || self.topic == ASKED
    }
}

const COMMENTED: &str = "record.commented";
const ASKED: &str = "card.asked";

/// What was said on a card since it last came to `column`: what anyone
/// said on it, and what its agent asked, in order. Where nothing says it
/// came there (a part of a label's, or a card added there before its
/// history was kept), everything said on it.
pub fn thread(lines: &[Line], column: &str) -> Vec<Line> {
    let came = |line: &Line| {
        !column.is_empty()
            && line.to == column
            && (line.topic == "board.item.moved" || line.topic == "board.item.created")
    };
    let from = lines.iter().rposition(came).map_or(0, |at| at + 1);
    let said = lines[from..].iter().filter(|line| line.said());
    // A question is its agent's, whoever's blueprint said it.
    said.map(|line| match line.topic == ASKED && !line.agent.is_empty() {
        true => Line { actor: format!("agent:{}", line.agent), who: line.agent.clone(), ..line.clone() },
        false => line.clone(),
    })
    .collect()
}

/// What was said on a card since its agent last had it, for the agent
/// that has it next: what anyone but that agent said on it after the
/// last line of a turn (one of `turns`, whose `agent` says whose), and
/// nothing that is `note`, which it is told already. Before any turn,
/// everything said on it.
pub fn discussed(lines: &[Line], turns: &[String], note: &str) -> Vec<Line> {
    let last = lines.iter().rposition(|line| turns.contains(&line.topic));
    let worker = last.map(|at| format!("agent:{}", lines[at].agent)).filter(|worker| worker != "agent:");
    let note = note.trim();
    lines[last.map_or(0, |at| at + 1)..]
        .iter()
        .filter(|line| line.topic == COMMENTED)
        .filter(|line| Some(&line.actor) != worker.as_ref())
        .filter(|line| note.is_empty() || line.words.trim() != note)
        .cloned()
        .collect()
}

/// Lines said, each as `- who: words`, a line of words after the first
/// indented under it.
pub fn said_as_text(lines: &[Line]) -> String {
    let each = lines.iter().map(|line| {
        let who = if line.who.is_empty() { &line.actor } else { &line.who };
        format!("- {who}: {}", line.words.trim().replace('\n', "\n  "))
    });
    each.collect::<Vec<_>>().join("\n")
}

/// The agents whose turns a card's history says ended in work on it, as
/// actors: what passing or failing it is not theirs to do.
pub fn worked(lines: &[Line]) -> Vec<String> {
    let mut agents: Vec<String> = Vec::new();
    for line in lines.iter().filter(|line| line.topic == "card.worked" && !line.agent.is_empty()) {
        let agent = format!("agent:{}", line.agent);
        if !agents.contains(&agent) {
            agents.push(agent);
        }
    }
    agents
}
