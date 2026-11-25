//! How somebody arranged the client's side panel for themselves, kept
//! with their account, so that it follows them to every window and
//! browser they sign in to.
//!
//! **An arrangement** is three things, each a list of text and nothing
//! more: the groups of channels somebody made (each an id, what they
//! called it and which channels are in it, in the order they were made),
//! the channels they pinned to the top, and what they pinned at the top
//! of Resources (each the address that opens it, what follows the `#`).
//! It is one row of `arranged`, the factory's own and never a project's,
//! under the account it is of. A copy of the factory keeps it, as it
//! keeps the accounts. What is folded, the open tabs and what was lately
//! opened are not in it: those are each window's own.
//!
//! **The daemon stores it and acts on none of it.** Nothing here, or
//! anywhere in the daemon, reads a group or a pin to decide anything:
//! who may read a channel, what a workspace lists, what is told to whom.
//! An id is text: one that names a channel or a thing that is gone, or
//! that its owner may not read, is not refused, since a channel may come
//! back from the Trash; the client passes it over where it is shown.
//!
//! **Checked where it is written** ([`checked`], with its table of cases
//! beside the client's): every id text of one line and not long, every
//! name something and not long, nothing twice (a group, a group's name
//! whatever its capitals, a channel in two groups, a pin), a bounded
//! number of each, and nothing beside the three. Anything else is
//! refused in a sentence and nothing is written.
//!
//! **Whose it is.** Nothing here takes an owner from whoever asks: the
//! arrangement of a request is always the asker's own, a person's
//! (`Principal::actor`, which for a key somebody made is its maker's),
//! and reading and changing it take [`READ`] and [`WRITE`] of what the
//! asker holds, so a key reads its maker's only where it was given that.
//! An agent and a device keep none.
//!
//! **Who hears.** Every change is one write with what is said of it
//! (`arranged.kept`, the factory's for good: `journal::HERE`), carrying
//! the arrangement whole, so that another open page follows it with
//! nothing read again; it takes being its owner (`member:own:<actor>`,
//! [`kernel::principal::own`]), so nobody else is told it whatever they
//! hold. One that comes to what is kept already is no write, and nothing
//! is said.

use journal::{Journal, optional};
use kernel::principal::own;
use kernel::{Error, Principal, PrincipalKind, Result, Source};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// What reading one's own arrangement takes.
pub const READ: &str = "arranged:read";
/// What changing it takes.
pub const WRITE: &str = "arranged:write";
/// The most groups one account keeps.
pub const GROUPS: usize = 100;
/// The most channels in groups, all groups together.
pub const GROUPED: usize = 2000;
/// The most channels pinned to the top.
pub const PINNED: usize = 200;
/// The most things pinned in Resources.
pub const THINGS: usize = 200;
/// The most letters a group's name has.
pub const LONGEST_NAME: usize = 60;
/// The most letters an id has: a channel's, a group's or a thing's address.
pub const LONGEST_ID: usize = 2048;

pub(crate) const SCHEMA: &[&str] = &["
    CREATE TABLE arranged (
        account TEXT PRIMARY KEY,
        arrangement TEXT NOT NULL,
        nth INTEGER NOT NULL,
        changed INTEGER NOT NULL
    ) WITHOUT ROWID;
"];

/// A group of channels somebody made.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    /// The channels in it, by id.
    pub channels: Vec<String>,
}

/// How somebody arranged the side panel: what is kept of it, and what is
/// written.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Arrangement {
    /// The groups of channels, in the order they were made.
    pub groups: Vec<Group>,
    /// The channels pinned to the top, by id, in the order they were pinned.
    pub pinned: Vec<String>,
    /// What is pinned in Resources, each by the address that opens it,
    /// in the order it was pinned.
    pub things: Vec<String>,
}

/// What somebody is handed of their arrangement.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Kept {
    /// What is kept: nothing for an account that never kept one.
    pub arrangement: Option<Arrangement>,
    /// How many times it was written, which grows with each write: what
    /// a page that hears of two tells the later by.
    pub nth: i64,
    /// When it was last written.
    pub changed: Option<i64>,
}

/// Why `id` is not an id of `what` (said as a sentence begins), if it is not.
fn unfit_id(id: &Value, what: &str) -> Option<String> {
    let Some(text) = id.as_str() else {
        return Some(format!("{what} is named by its id, which is text, and {id} is not."));
    };
    if text.trim().is_empty() {
        return Some(format!("{what} is named by its id, and this one is empty."));
    }
    let letters = text.chars().count();
    if letters > LONGEST_ID {
        return Some(format!("An id is {LONGEST_ID} letters at most, and this one is {letters}."));
    }
    if text.chars().any(char::is_control) {
        return Some("An id is one line of text, and this one is several.".to_string());
    }
    None
}

/// A list of ids of `what`, each once, or why it is not one.
fn ids(list: Option<&Value>, what: &str, of: &str, most: usize) -> std::result::Result<Vec<String>, String> {
    let list = match list {
        None | Some(Value::Null) => return Ok(Vec::new()),
        Some(Value::Array(list)) => list,
        Some(_) => return Err(format!("`{of}` is a list of ids.")),
    };
    if list.len() > most {
        return Err(format!("{most} {what} at most are kept, and this says {}.", list.len()));
    }
    let mut kept: Vec<String> = Vec::new();
    for id in list {
        if let Some(wrong) = unfit_id(id, "A channel or a thing") {
            return Err(wrong);
        }
        let id = id.as_str().unwrap_or_default().to_string();
        if kept.contains(&id) {
            return Err(format!("`{id}` is said twice in `{of}`: each is there once."));
        }
        kept.push(id);
    }
    Ok(kept)
}

/// The arrangement `said` is, as it is kept, or why it is not one.
pub fn checked(said: &Value) -> std::result::Result<Arrangement, String> {
    let Some(said) = said.as_object() else {
        return Err("An arrangement is a mapping of `groups`, `pinned` and `things`.".to_string());
    };
    if let Some(other) = said.keys().find(|key| !["groups", "pinned", "things"].contains(&key.as_str())) {
        return Err(format!(
            "`{other}` is nothing an arrangement says: it says `groups`, `pinned` and `things`."
        ));
    }
    let groups = match said.get("groups") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(groups)) => groups.clone(),
        Some(_) => {
            return Err("`groups` is a list of groups, each an id, a name and its channels.".to_string());
        }
    };
    if groups.len() > GROUPS {
        return Err(format!("{GROUPS} groups at most are kept, and this says {}.", groups.len()));
    }
    let mut kept: Vec<Group> = Vec::new();
    let mut grouped = 0;
    for group in &groups {
        let Some(group) = group.as_object() else {
            return Err("A group is a mapping of its `id`, its `name` and its `channels`.".to_string());
        };
        if let Some(other) = group.keys().find(|key| !["id", "name", "channels"].contains(&key.as_str())) {
            return Err(format!("`{other}` is nothing a group says: it says `id`, `name` and `channels`."));
        }
        let id = group.get("id").unwrap_or(&Value::Null);
        if let Some(wrong) = unfit_id(id, "A group") {
            return Err(wrong);
        }
        let id = id.as_str().unwrap_or_default().to_string();
        let name = group.get("name").and_then(Value::as_str).unwrap_or_default().trim().to_string();
        if name.is_empty() {
            return Err("A group is called something: say what this one is called.".to_string());
        }
        let letters = name.chars().count();
        if letters > LONGEST_NAME {
            return Err(format!(
                "A group's name is {LONGEST_NAME} letters at most, and this one is {letters}."
            ));
        }
        if name.chars().any(char::is_control) {
            return Err("A group's name is one line of words.".to_string());
        }
        if kept.iter().any(|one| one.id == id) {
            return Err(format!("Two groups are `{id}`: each group has an id of its own."));
        }
        if kept.iter().any(|one| one.name.to_lowercase() == name.to_lowercase()) {
            return Err(format!("Two groups are called {name}: each is called something of its own."));
        }
        let channels = ids(group.get("channels"), "channels in a group", "channels", GROUPED)?;
        grouped += channels.len();
        if grouped > GROUPED {
            return Err(format!("{GROUPED} channels at most are kept in groups, all the groups together."));
        }
        if let Some(twice) =
            channels.iter().find(|channel| kept.iter().any(|one| one.channels.contains(channel)))
        {
            return Err(format!("`{twice}` is in two groups: a channel is in one group or in none."));
        }
        kept.push(Group { id, name, channels });
    }
    let pinned = ids(said.get("pinned"), "channels pinned", "pinned", PINNED)?;
    let things = ids(said.get("things"), "things pinned", "things", THINGS)?;
    Ok(Arrangement { groups: kept, pinned, things })
}

/// The account whose arrangement `by`'s is: a person's, or a key's maker's.
fn account_of(by: &Principal) -> Result<String> {
    match by.kind {
        PrincipalKind::User => Ok(by.id.clone()),
        _ => Err(Error::invalid(
            "How the side panel is arranged is kept with a person's account, and an agent or a device has none: \
             sign in as a person.",
        )),
    }
}

fn kept_of(connection: &rusqlite::Connection, account: &str) -> rusqlite::Result<Kept> {
    let row: Option<(String, i64, i64)> = optional(connection.query_row(
        "SELECT arrangement, nth, changed FROM arranged WHERE account = ?1",
        [account],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ))?;
    Ok(match row {
        Some((arrangement, nth, changed)) => Kept {
            arrangement: Some(serde_json::from_str(&arrangement).unwrap_or_default()),
            nth,
            changed: Some(changed),
        },
        None => Kept { arrangement: None, nth: 0, changed: None },
    })
}

/// The keeper of how everybody arranged the side panel.
#[derive(Clone)]
pub struct Arranged {
    journal: Journal,
}

impl Arranged {
    pub async fn open(journal: Journal) -> Result<Arranged> {
        journal.db().migrate("arranged", SCHEMA).await?;
        Ok(Arranged { journal })
    }

    /// What `by` is handed of their arrangement: what is kept, or
    /// nothing where they never kept one.
    pub async fn of(&self, by: &Principal) -> Result<Kept> {
        by.require(READ)?;
        let account = account_of(by)?;
        self.journal.db().call(move |connection| kept_of(connection, &account)).await
    }

    /// Keeps `said` as `by`'s arrangement, whole. Where `first` is so it
    /// is kept only where `by` keeps none yet, which is how what a
    /// browser kept before is taken up: two windows that each hand theirs
    /// over have the first's kept and are both answered with it. One that
    /// comes to what is kept is as it was, and nothing is said. Either way
    /// the answer is what is kept from then.
    pub async fn keep(&self, by: &Principal, said: &Value, first: bool) -> Result<Kept> {
        by.require(WRITE)?;
        let account = account_of(by)?;
        let arrangement = checked(said).map_err(Error::invalid)?;
        let (by, now) = (by.clone(), self.journal.now_ms());
        let wrote = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            let was = kept_of(transaction, &account)?;
            if (first && was.arrangement.is_some()) || was.arrangement.as_ref() == Some(&arrangement) {
                return Ok(was);
            }
            let written = serde_json::to_string(&arrangement).unwrap_or_default();
            transaction.execute(
                "INSERT INTO arranged (account, arrangement, nth, changed) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (account) DO UPDATE
                 SET arrangement = excluded.arrangement, nth = excluded.nth, changed = excluded.changed",
                params![account, written, was.nth + 1, now],
            )?;
            let kept = kept_of(transaction, &account)?;
            let owner = format!("user:{account}");
            let source = Source::actor(by.actor()).taking(Some(&own(&owner)));
            let data = json!({ "owner": owner, "arrangement": kept.arrangement, "nth": kept.nth });
            writing.say_here("arranged.kept", source, data)?;
            Ok(kept)
        });
        Ok(wrote.await?.0)
    }
}
