//! A change somebody asks for that they may not make themselves, as plain
//! values (`docs/approvals.md`): which cells of a record it changes, what
//! making it takes, what an asker lacks of that, and what the asking then
//! comes to. Whoever keeps the record works out what each thing takes,
//! from the table's fields as they are declared, and hands it here; who
//! holds what is asked by the one rule ([`crate::principal::covers`]).
//!
//! Nothing here keeps or makes anything. How an ask stands is worked out
//! where it is read or acted on, and never by a timer ([`stands`]): it
//! lasts [`LASTS_MS`], and is stale once its record was changed at all
//! since it was asked.

use serde_json::{Map, Value};

use crate::principal::covers;

/// The kinds of thing that can be asked for: a record's cells, and a run
/// let go on (`crate::go`); another guarded act is a kind of its own,
/// with a door of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Cells,
    Go,
}

impl Kind {
    pub const ALL: &[Kind] = &[Kind::Cells, Kind::Go];

    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Cells => "cells",
            Kind::Go => "go",
        }
    }

    /// The kind written `word`, or the sentence that says which there are.
    pub fn read(word: &str) -> Result<Kind, String> {
        Kind::ALL.iter().copied().find(|kind| kind.as_str() == word).ok_or_else(|| {
            let kinds: Vec<&str> = Kind::ALL.iter().map(|kind| kind.as_str()).collect();
            format!("there is no kind of change called `{word}`: it is one of {}", kinds.join(", "))
        })
    }
}

/// One cell a change sets: what it holds now and what it is to hold.
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    pub name: String,
    pub before: Value,
    pub after: Value,
}

/// Whether a cell's value says nothing: none at all, or empty text, which
/// is what a cell emptied in the client is kept as.
pub fn says_nothing(value: &Value) -> bool {
    value.is_null() || value.as_str().is_some_and(str::is_empty)
}

/// The cells `asked` changes of a record whose cells are `kept`, by their
/// names (a mapping is kept in that order): one set to what it holds already, or from
/// nothing to nothing, is no change. It is the one reading of what a
/// change of a record's cells changes, for whoever makes one and for
/// whoever is shown one.
pub fn changed(kept: &Map<String, Value>, asked: &Map<String, Value>) -> Vec<Cell> {
    asked
        .iter()
        .filter_map(|(name, after)| {
            let before = kept.get(name).unwrap_or(&Value::Null);
            let same = before == after || (says_nothing(before) && says_nothing(after));
            (!same).then(|| Cell { name: name.clone(), before: before.clone(), after: after.clone() })
        })
        .collect()
}

/// What a thing a change is asked of is: whoever makes it is asked each.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Of {
    /// Writing a project's records at all.
    Records,
    /// What the table asks of whoever changes its rows.
    Table,
    /// What giving one cell its value takes.
    Cell(String),
    /// What writing each row of a table this one steers takes, by its name.
    Steered(String),
    /// Reading the record a cell names, by the cell.
    Named(String),
    /// What a cell the change leaves as it is took, while it says so
    /// (`stands`), by the cell: asked of changing anything of the row.
    Standing(String),
}

impl Of {
    /// What it is, in words, for whoever is shown what a change takes.
    pub fn said(&self) -> String {
        match self {
            Of::Records => "writing records".into(),
            Of::Table => "changing this table's rows".into(),
            Of::Cell(name) => format!("setting `{name}`"),
            Of::Steered(table) => format!("writing the rows of `{table}`, which this table steers"),
            Of::Named(name) => format!("reading what `{name}` names"),
            Of::Standing(name) => format!("changing anything of a row whose `{name}` says what it does"),
        }
    }
}

/// One thing a change is asked of, and every scope it takes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Need {
    pub of: Of,
    pub scopes: Vec<String>,
}

impl Need {
    pub fn new(of: Of, scopes: &[&str]) -> Need {
        Need { of, scopes: scopes.iter().map(|scope| scope.to_string()).collect() }
    }
}

/// What whoever holds `held` lacks of `needs`: each thing with the scopes
/// of it they do not hold, in the order needed, and nothing where they
/// hold all of it. A thing that takes nothing is never lacking.
pub fn lacking(needs: &[Need], held: &[String]) -> Vec<Need> {
    needs
        .iter()
        .filter_map(|need| {
            let missing: Vec<String> =
                need.scopes.iter().filter(|scope| !covers(held, scope)).cloned().collect();
            (!missing.is_empty()).then(|| Need { of: need.of.clone(), scopes: missing })
        })
        .collect()
}

/// Every scope `needs` take, each once, in the order first needed.
pub fn scopes_of(needs: &[Need]) -> Vec<String> {
    let mut all: Vec<String> = Vec::new();
    for scope in needs.iter().flat_map(|need| &need.scopes) {
        if !all.contains(scope) {
            all.push(scope.clone());
        }
    }
    all
}

/// What asking for a change comes to, for whoever asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Comes {
    /// Nothing changes: nothing was asked that is not so already.
    Nothing,
    /// They hold all it takes, and it is made as anybody's change is.
    Make,
    /// They lack these, and it is kept for somebody who holds them.
    Ask(Vec<Need>),
}

/// What asking for `cells` comes to, which need `needs`, for whoever holds
/// `held`.
pub fn comes(cells: &[Cell], needs: &[Need], held: &[String]) -> Comes {
    if cells.is_empty() {
        return Comes::Nothing;
    }
    match lacking(needs, held) {
        lacks if lacks.is_empty() => Comes::Make,
        lacks => Comes::Ask(lacks),
    }
}

/// The words of the line whoever could make a change is told by: who
/// asks, the record by what it is known by and its table, and the cells
/// by their names.
pub fn told(asker: &str, record: &str, table: &str, cells: &[Cell]) -> String {
    let names: Vec<&str> = cells.iter().map(|cell| cell.name.as_str()).collect();
    format!("{asker} asks to change {record} ({table}): {}", names.join(", "))
}

/// How long an ask waits to be decided: seven days.
pub const LASTS_MS: i64 = 7 * 24 * 60 * 60 * 1000;

/// How many asks one asker may have waiting at once.
pub const MOST_WAITING: usize = 20;

/// How an ask stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Waiting,
    Approved,
    Declined,
    Stale,
    Withdrawn,
    Expired,
}

impl Status {
    pub const ALL: &[Status] = &[
        Status::Waiting,
        Status::Approved,
        Status::Declined,
        Status::Stale,
        Status::Withdrawn,
        Status::Expired,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Status::Waiting => "waiting",
            Status::Approved => "approved",
            Status::Declined => "declined",
            Status::Stale => "stale",
            Status::Withdrawn => "withdrawn",
            Status::Expired => "expired",
        }
    }

    /// The status written `word`; one this program does not know is
    /// taken for decided, so that nothing acts on it.
    pub fn read(word: &str) -> Status {
        Status::ALL.iter().copied().find(|status| status.as_str() == word).unwrap_or(Status::Withdrawn)
    }
}

/// How an ask kept as `status`, asked at `at` of a record that was changed
/// last at `asked_of` then and is changed last at `now_of` now, stands at
/// `now`: a waiting one that has lasted its time is expired, and one whose
/// record was changed since is stale; anything decided is as it was.
pub fn stands(status: Status, at: i64, (asked_of, now_of): (i64, i64), now: i64) -> Status {
    match status {
        Status::Waiting if now - at >= LASTS_MS => Status::Expired,
        Status::Waiting if now_of != asked_of => Status::Stale,
        other => other,
    }
}

/// What an asker is told an ask came to, in a sentence, where it is not
/// waiting: `by` is whoever decided it, by name, and `why` what they said.
pub fn decided(status: Status, record: &str, by: &str, why: &str) -> String {
    let because = if why.trim().is_empty() { String::new() } else { format!(": {}", why.trim()) };
    match status {
        Status::Waiting => format!("Your change of {record} waits to be approved."),
        Status::Approved => format!("{by} approved your change of {record}, and it is made."),
        Status::Declined => format!("{by} declined your change of {record}{because}."),
        Status::Stale => format!(
            "Your change of {record} was not made: the record was changed since you asked. Ask again if it is still wanted."
        ),
        Status::Withdrawn => format!("Your change of {record} was withdrawn."),
        Status::Expired => format!(
            "Your change of {record} was not decided in seven days and goes no further. Ask again if it is still wanted."
        ),
    }
}
