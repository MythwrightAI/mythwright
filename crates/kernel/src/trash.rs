//! The Trash, as plain values: how long what is deleted is kept, when a
//! thing in it goes for good, and where something given back goes when
//! its place is taken. What is kept, and the keeping, are the studio's
//! (`studio::trash`); this is what is worked out of times, numbers and
//! names alone.
//!
//! What somebody deletes goes to the Trash and is given back from there,
//! and goes for good once it has been there for as many days as the
//! factory's owner says, thirty unless they say. Nought days is deleting
//! at once, as it was before there was a Trash; and they may say that
//! what is deleted is kept for ever.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{Error, Result};

/// How many days what is deleted is kept where nobody said.
pub const USUAL_DAYS: u32 = 30;
/// The most days that may be said: a hundred years.
pub const MOST_DAYS: u32 = 36_500;
/// A day, in milliseconds.
pub const DAY_MS: i64 = 86_400_000;

/// How long what is deleted is kept, as the factory's owner said it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keeping {
    /// For how many days; for ever where there is none. Nought is
    /// deleting at once.
    pub days: Option<u32>,
    /// Who last said, by name, and when; nobody where it is as usual.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<i64>,
}

impl Default for Keeping {
    fn default() -> Keeping {
        Keeping { days: Some(USUAL_DAYS), by: None, at: None }
    }
}

impl Keeping {
    /// Whether what is deleted goes at once, with nothing kept.
    pub fn at_once(&self) -> bool {
        self.days == Some(0)
    }
}

/// What may be said of how long: so many days, from nought to
/// [`MOST_DAYS`], or for ever with nothing. Refused in a sentence
/// otherwise.
pub fn days(days: Option<u32>) -> Result<Option<u32>> {
    match days {
        Some(days) if days > MOST_DAYS => Err(Error::invalid(format!(
            "what is deleted is kept for {MOST_DAYS} days at most, and {days} is more; say `0` to delete at \
             once, or `always` to keep it until somebody deletes it for good"
        ))),
        days => Ok(days),
    }
}

/// When something put in the Trash at `at` goes for good, kept for
/// `days`: never where it is kept for ever.
pub fn goes(at: i64, days: Option<u32>) -> Option<i64> {
    days.map(|days| at.saturating_add(i64::from(days).saturating_mul(DAY_MS)))
}

/// Whether something put in the Trash at `at`, kept for `days`, is to
/// go for good by `now`.
pub fn due(at: i64, days: Option<u32>, now: i64) -> bool {
    goes(at, days).is_some_and(|goes| goes <= now)
}

/// Where a record goes back to: the place it had, where nothing else of
/// what it is shown with has that place now, and otherwise after the last
/// of them (`highest`), as a card put at the bottom of its column is.
pub fn back_at(position: f64, taken: bool, highest: Option<f64>) -> f64 {
    match taken {
        false => position,
        true => highest.map_or(0.0, |highest| highest.max(0.0)) + 1.0,
    }
}

/// What a file given back beside whatever is at its place now is called:
/// `notes (restored).md` for `notes.md`, then `notes (restored 2).md` and
/// so on, the first that `taken` does not say is there. A folder, or a
/// file whose name has no ending, is called the same with nothing after.
pub fn beside(path: &str, taken: impl Fn(&str) -> bool) -> String {
    let (folder, name) = match path.rsplit_once('/') {
        Some((folder, name)) => (format!("{folder}/"), name),
        None => (String::new(), path),
    };
    let (stem, ending) = match name.rsplit_once('.') {
        Some((stem, ending)) if !stem.is_empty() => (stem, format!(".{ending}")),
        _ => (name, String::new()),
    };
    let mut nth = 1;
    loop {
        let said = match nth {
            1 => format!("{folder}{stem} (restored){ending}"),
            nth => format!("{folder}{stem} (restored {nth}){ending}"),
        };
        if !taken(&said) {
            return said;
        }
        nth += 1;
    }
}

/// What kind of thing is in the Trash.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// A row of a table, a card included.
    Record,
    /// A file or a folder of a project.
    File,
    /// A project, forgotten.
    Project,
    /// A channel, taken away with what was said in it.
    Channel,
    /// Something attached to a record, taken away from it: its record is
    /// what it is known by, and what is kept of it says which it was.
    Attachment,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Record => "record",
            Kind::File => "file",
            Kind::Project => "project",
            Kind::Channel => "channel",
            Kind::Attachment => "attachment",
        }
    }

    pub fn parse(text: &str) -> Kind {
        match text {
            "file" => Kind::File,
            "project" => Kind::Project,
            "channel" => Kind::Channel,
            "attachment" => Kind::Attachment,
            _ => Kind::Record,
        }
    }
}

fn went_alone(with: &usize) -> bool {
    *with == 0
}

/// Something in the Trash, as whoever reads it is told of it.
#[derive(Clone, Debug, Serialize)]
pub struct Trashed {
    pub id: String,
    pub kind: Kind,
    /// What it is known by: a record's id, a file's path in its project, a
    /// project's id, a channel's id.
    pub thing: String,
    /// The project it is of; for a project, itself; for a channel, which
    /// is the factory's whatever project it says, nothing.
    pub project: String,
    /// The table, for a row of one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    /// What it was called.
    pub called: String,
    pub by: String,
    /// Who that is, by name, as whoever reads the Trash is told.
    pub who: String,
    pub at: i64,
    /// When it goes for good, as the factory keeps what is deleted now;
    /// never, where it is kept for ever.
    pub goes: Option<i64>,
    /// Whether whoever reads this may give it back, and delete it for
    /// good: the one thing, whoever could have deleted it.
    pub may: bool,
    /// How many records went into the Trash with it, and come back with
    /// it: the pages under a page deleted.
    #[serde(skip_serializing_if = "went_alone")]
    pub with: usize,
    /// What puts it back where it was, which is handed to nobody.
    #[serde(skip)]
    pub kept: Value,
    /// What reading it takes beside what its kind does: the scope a
    /// table it was of keeps its rows private under; for a channel, what
    /// hearing of the channel takes, which names who was in a kept one.
    #[serde(skip)]
    pub reads: Option<String>,
}

impl Trashed {
    /// Something of `kind` deleted by `by` at `at`, called by what it is
    /// known by until it is said otherwise.
    pub fn new(id: String, kind: Kind, thing: &str, project: &str, by: &str, at: i64) -> Trashed {
        Trashed {
            id,
            kind,
            thing: thing.to_string(),
            project: project.to_string(),
            table: None,
            called: thing.to_string(),
            by: by.to_string(),
            who: String::new(),
            at,
            goes: None,
            may: false,
            with: 0,
            kept: Value::Null,
            reads: None,
        }
    }

    /// What is said as it goes in, comes out or goes for good: which, of
    /// what kind, and whose. A project's says no project, and is the
    /// factory's, as its row is, and so is a channel's.
    pub fn about(&self) -> Value {
        let mut data = json!({ "id": self.id, "kind": self.kind, "thing": self.thing });
        if !matches!(self.kind, Kind::Project | Kind::Channel) {
            data["project"] = json!(self.project);
        }
        data
    }

    /// What hearing of it takes beside what the Trash's own events do:
    /// what reading the thing takes. `board` is what a board's table is
    /// called, whose cards are read as cards.
    pub fn taking(&self, board: &str) -> Option<&str> {
        let kind = match (self.kind, self.table.as_deref()) {
            (Kind::Record | Kind::Attachment, Some(table)) if table == board => Some("board:read"),
            (Kind::Record | Kind::Attachment, _) => Some("records:read"),
            _ => None,
        };
        self.reads.as_deref().or(kind)
    }
}

/// What is written beside a file kept in the Trash: everything its row
/// holds, so that a daemon stopped between the two finds the row owed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Whence {
    pub id: String,
    pub project: String,
    pub path: String,
    pub by: String,
    pub at: i64,
    pub reads: Option<String>,
}

impl Whence {
    /// The row it says is owed.
    pub fn entry(&self) -> Trashed {
        let mut entry =
            Trashed::new(self.id.clone(), Kind::File, &self.path, &self.project, &self.by, self.at);
        entry.called = self.path.rsplit('/').next().unwrap_or(&self.path).to_string();
        entry.kept = json!({ "path": self.path });
        entry.reads = self.reads.clone();
        entry
    }
}
