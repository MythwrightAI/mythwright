//! What a project is set up with, as rows: each a blueprint applied for
//! it as whoever sets it up, with what it is called where a person
//! chooses, what it is, whether it is on unless said, what it cannot be
//! had without and why, which of its params are told the project, and
//! what it is handed beside (`with`).
//!
//! The rows are `std/project`'s (its `rows`, whose default is the usual
//! list), or a setup somebody saved, and nothing in Rust lists them. What
//! is decided of them is here, as functions of plain values with their
//! cases (the suite's `setups.rs`): whether a list of rows can be read
//! ([`rows`]); which of them a request asks for, by name or as a row of
//! its own ([`chosen`]); and what one row comes to as it is reached
//! ([`came`]): there already, left out for the lack of what it needs, or
//! to be given. The applying is `project.set_up`'s (the `setup` crate),
//! which asks [`came`] of each row in turn, as whoever applied the text.

use serde_json::{Map, Value, json};

/// A project's own fleet text, which is written for it from what a
/// project's is written from before it is applied.
pub const FLEET: &str = "fleet";

/// Offered as a project is added, in the dialog that adds one.
pub const ADDING: &str = "adding";

/// One thing a project is set up with.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// What it is asked for by, in a request and on the command line.
    pub name: String,
    /// What it is called where a person chooses.
    pub title: String,
    /// How it is said in a sentence (`a board`).
    pub said: String,
    /// What it is, in a line.
    pub says: String,
    /// The blueprint that is applied for it.
    pub blueprint: String,
    /// Whether it is given unless whoever sets the project up says not.
    pub on: bool,
    /// What it cannot be given without, by name.
    pub needs: Vec<String>,
    /// Why it cannot: said where it is left out for the lack of them.
    pub why: String,
    /// The table it gives the project, where it gives one.
    pub table: String,
    /// The params of its blueprint that are told the project.
    pub told: Vec<String>,
    /// Where a person is offered it: [`ADDING`], or the name of something
    /// it needs, on whose page it is then offered.
    pub offered: String,
    /// What its blueprint is handed beside what it is told.
    pub with: Map<String, Value>,
}

/// The words a row may say, each read by [`Row::read`].
const WORDS: &[&str] =
    &["name", "title", "said", "says", "blueprint", "on", "needs", "why", "table", "told", "offered", "with"];

impl Row {
    /// One row as it is written, or why it cannot be read. Only `name`
    /// and `blueprint` must be said: it is called its name and said as
    /// its name where nothing else is said, it is on, needs nothing, is
    /// told nothing and is offered as a project is added.
    pub fn read(value: &Value) -> Result<Row, String> {
        let Value::Object(row) = value else {
            return Err(format!(
                "a row is a mapping with at least its `name` and its `blueprint`, and not {value}"
            ));
        };
        let name = row
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or("a row says its `name`: what it is asked for by")?;
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(format!(
                "`{name}` is no name for a row: a name is small letters, digits and dashes, such as `board` or `notes-2`"
            ));
        }
        if let Some(word) = row.keys().find(|word| !WORDS.contains(&word.as_str())) {
            return Err(format!(
                "`{name}` says `{word}`, which a row does not say. It may say: {}",
                WORDS.join(", ")
            ));
        }
        let wrong = |word: &str, what: &str| format!("`{word}` of `{name}` is {what}");
        let blueprint = text(row, "blueprint", "")
            .filter(|blueprint| !blueprint.trim().is_empty() && !blueprint.contains(char::is_whitespace))
            .ok_or_else(|| wrong("blueprint", "the name of the blueprint applied for it, and is needed"))?;
        let title = text(row, "title", &name).ok_or_else(|| wrong("title", "words"))?;
        let said = text(row, "said", &title.to_lowercase()).ok_or_else(|| wrong("said", "words"))?;
        let on = match row.get("on") {
            None | Some(Value::Null) => true,
            Some(Value::Bool(on)) => *on,
            Some(_) => return Err(wrong("on", "true or false")),
        };
        let names = |word: &str| -> Result<Vec<String>, String> {
            match row.get(word) {
                None | Some(Value::Null) => Ok(Vec::new()),
                Some(Value::Array(items)) => items
                    .iter()
                    .map(|item| {
                        item.as_str().map(str::to_string).ok_or_else(|| wrong(word, "a list of names"))
                    })
                    .collect(),
                Some(_) => Err(wrong(word, "a list of names")),
            }
        };
        let with = match row.get("with") {
            None | Some(Value::Null) => Map::new(),
            Some(Value::Object(with)) => with.clone(),
            Some(_) => return Err(wrong("with", "what its blueprint is handed, each param by its name")),
        };
        let told = names("told")?;
        if let Some(told) = told.iter().find(|told| with.contains_key(told.as_str())) {
            return Err(format!(
                "`{told}` of `{name}` is the project's: it is told which project it is for, and is not said here"
            ));
        }
        Ok(Row {
            title,
            said,
            says: text(row, "says", "").ok_or_else(|| wrong("says", "words"))?,
            blueprint,
            on,
            needs: names("needs")?,
            why: text(row, "why", "").ok_or_else(|| wrong("why", "words"))?,
            table: text(row, "table", "").ok_or_else(|| wrong("table", "the name of a table"))?,
            told,
            offered: text(row, "offered", ADDING).ok_or_else(|| wrong("offered", "a word"))?,
            with,
            name,
        })
    }

    /// The row as it is written and handed out: every word said.
    pub fn value(&self) -> Value {
        json!({
            "name": self.name, "title": self.title, "said": self.said, "says": self.says,
            "blueprint": self.blueprint, "on": self.on, "needs": self.needs, "why": self.why,
            "table": self.table, "told": self.told, "offered": self.offered, "with": self.with,
        })
    }
}

/// The text of `word`, `or` where it is not said, and nothing where it
/// says something that is no text.
fn text(row: &Map<String, Value>, word: &str, or: &str) -> Option<String> {
    match row.get(word) {
        None | Some(Value::Null) => Some(or.to_string()),
        Some(Value::String(text)) => Some(text.clone()),
        Some(_) => None,
    }
}

/// A list of rows as it is written (a setup, or `std/project`'s `rows`),
/// or why it cannot be read: each row as [`Row::read`] reads it, no name
/// twice, and nothing a row needs coming after it, since the rows are
/// given in order and it would never be there first.
pub fn rows(value: &Value) -> Result<Vec<Row>, String> {
    let Value::Array(items) = value else {
        return Err(format!(
            "the rows are a list, each a mapping with its `name` and `blueprint`, and not {value}"
        ));
    };
    let mut rows: Vec<Row> = Vec::new();
    for item in items {
        let row = Row::read(item)?;
        if rows.iter().any(|one| one.name == row.name) {
            return Err(format!("`{}` is the name of two rows: a row is asked for by its name", row.name));
        }
        rows.push(row);
    }
    for (at, row) in rows.iter().enumerate() {
        if let Some(need) = row.needs.iter().find(|need| rows[at..].iter().any(|later| &later.name == *need))
        {
            return Err(format!(
                "`{}` needs `{need}`, which is itself or comes after it: what a row needs is given before it",
                row.name
            ));
        }
    }
    Ok(rows)
}

/// `rows` as a request asks for them (`asked`, under the word it was
/// said by): each named by its name, by its name with what its blueprint
/// is handed (`{ "name": "board", "with": {...} }`, over what the row says
/// it is handed), or a row of its own (one that says its `blueprint`).
/// What is asked is on and the rest of `rows` off, in their order, and
/// then the rows of its own in the order they were said, each once: so
/// that whoever goes through them can say what a row needs that was not
/// asked for. A name that is no row is refused, saying under which word
/// it was named and what may be; so is a row of its own called what a
/// row of `rows` is, and one handed what it is told of the project.
pub fn chosen(rows: &[Row], word: &str, asked: &[Value]) -> Result<Vec<Row>, String> {
    let mut named: Vec<Row> = Vec::new();
    let mut own: Vec<Row> = Vec::new();
    for item in asked {
        let (said, with) = match item {
            Value::Object(one) if one.contains_key("blueprint") => {
                let row = Row { on: true, ..Row::read(item)? };
                if rows.iter().any(|one| one.name == row.name) {
                    return Err(format!(
                        "`{}` is a row already, which is asked for by its name: a row of your own is called something else",
                        row.name
                    ));
                }
                if !own.iter().any(|one| one.name == row.name) {
                    own.push(row);
                }
                continue;
            }
            Value::Object(one) => (one.get("name").and_then(Value::as_str), one.get("with")),
            _ => (item.as_str(), None),
        };
        let found = said.and_then(|said| rows.iter().find(|row| row.name == said)).ok_or_else(|| {
            let known: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
            let named = said.map_or_else(|| item.to_string(), |said| format!("\"{said}\""));
            format!(
                "`{word}` names {named}, which is nothing a project starts with. It may name: {}",
                known.join(", ")
            )
        })?;
        let mut row = Row { on: true, ..found.clone() };
        match with {
            None | Some(Value::Null) => {}
            Some(Value::Object(with)) => {
                if let Some(told) = row.told.iter().find(|told| with.contains_key(told.as_str())) {
                    return Err(format!(
                        "`{told}` of `{}` is the project's: it is told which project it is for, and is not said here",
                        row.name
                    ));
                }
                row.with.extend(with.clone());
            }
            Some(other) => {
                return Err(format!(
                    "`with` of `{}` is what its blueprint is handed, each param by its name, and not {other}",
                    row.name
                ));
            }
        }
        if !named.iter().any(|one| one.name == row.name) {
            named.push(row);
        }
    }
    let mut all: Vec<Row> = rows
        .iter()
        .map(|row| {
            named.iter().find(|one| one.name == row.name).cloned().unwrap_or(Row { on: false, ..row.clone() })
        })
        .collect();
    all.extend(own);
    Ok(all)
}

/// Where a row is reached, what is so of the project.
pub struct Reached<'a> {
    /// The project, by its id.
    pub project: &'a str,
    /// What it was given so far as it is set up now, by name.
    pub given: &'a [String],
    /// What it had before, by name: of a project that was there already,
    /// and nothing of one only just added.
    pub had: &'a [String],
    /// The instance of the row's blueprint alive in it, where one is.
    pub alive: Option<&'a str>,
    /// The rows it is set up with, for what a need is said as.
    pub rows: &'a [Row],
    /// What this program is run as on the command line.
    pub cli: &'a str,
}

/// What one row comes to as it is reached.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Came {
    /// The project has it: this instance, which is not applied again.
    There(String),
    /// Left out, with the sentence that says why.
    Left(String),
    /// Applied for the project.
    Give,
}

/// What `row` comes to where it is reached: there already where its
/// blueprint is alive in the project; left out where something it needs
/// was neither given now nor had before; and otherwise given.
pub fn came(row: &Row, reached: &Reached<'_>) -> Came {
    if let Some(alive) = reached.alive {
        return Came::There(alive.to_string());
    }
    let has = |need: &String| reached.given.contains(need) || reached.had.contains(need);
    let Some(lacks) = row.needs.iter().find(|need| !has(need)) else { return Came::Give };
    let lacked =
        reached.rows.iter().find(|one| &one.name == lacks).map_or(lacks.as_str(), |one| one.said.as_str());
    let why = if row.why.is_empty() { format!("it needs {lacked}") } else { row.why.clone() };
    Came::Left(format!(
        "`{}` was not given {}: {why}, and it has not got {lacked}. With one: {}",
        reached.project,
        row.said,
        by_hand(row, reached.project, reached.cli)
    ))
}

/// How `row` is given `project` by hand, for whoever is told it was not.
pub fn by_hand(row: &Row, project: &str, cli: &str) -> String {
    if row.blueprint == FLEET {
        return format!("`{cli} fleet init {project}`, then `{cli} apply {FLEET} --project {project}`");
    }
    let told: String = row.told.iter().map(|param| format!(" --param {param}={project}")).collect();
    format!("`{cli} apply {} --project {project}{told}`", row.blueprint)
}

/// What is said where applying `row` failed, `why`.
pub fn failed(row: &Row, project: &str, why: &str, cli: &str) -> String {
    format!("`{project}` was not given {}: {why}. By hand: {}", row.said, by_hand(row, project, cli))
}

/// What is said where none of `rows` was given, since whoever set the
/// project up (`who`) may not apply a blueprint.
pub fn refused(rows: &[Row], who: &str, project: &str, cli: &str) -> String {
    let said: Vec<String> = rows.iter().map(|row| row.said.clone()).collect();
    let hands: Vec<String> = rows.iter().map(|row| by_hand(row, project, cli)).collect();
    format!(
        "`{project}` was not given {}: each is made by applying a blueprint, which takes `blueprints:run`, and {who} \
         does not hold it. Someone who does can give it them: {}",
        said_together(&said),
        hands.join(", then "),
    )
}

/// What is said where a project came with a fleet text of its own
/// (`file`), which is left for whoever added it to read.
pub fn theirs(project: &str, file: &str, cli: &str) -> String {
    format!(
        "`{project}` came with a fleet blueprint of its own (`{file}`), which was not applied: read it, and then \
         `{cli} apply {FLEET} --project {project}`"
    )
}

/// `a board`, `a board and a fleet`, `a board, a fleet and docs`.
pub fn said_together(said: &[String]) -> String {
    match said {
        [] => String::new(),
        [one] => one.clone(),
        [first @ .., last] => format!("{} and {last}", first.join(", ")),
    }
}
