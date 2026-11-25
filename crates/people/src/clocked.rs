//! How somebody has a time written for them in the client, kept with
//! their account, so that it follows them to every window and browser
//! they sign in to.
//!
//! **A setting** is two words: the zone their times are written in, by
//! its name in the zones' own list (`America/Denver`), and whether a time
//! is written on a clock of twelve hours with AM or PM after it or of
//! twenty-four (`12h`, `24h`; twelve where nothing is said). It is one
//! row of `clocked`, the factory's own and never a project's, under the
//! account it is of; a copy of the factory keeps it, as it keeps the
//! accounts.
//!
//! **The daemon keeps no list of zones and writes no time in one.** A
//! zone is text to it: it is held to the form a zone's name has where it
//! is written ([`checked`]: one line, not long, the letters, digits and
//! marks such a name is made of, in parts between slashes), and looked
//! up nowhere. The client, which has the zones its browser knows, holds
//! a zone to those before it sends it and writes every time it shows
//! in it; whether a blueprint may write a time in somebody's zone is
//! later work, and would bring a list of zones into the program.
//!
//! **Whose it is.** Nothing here takes an owner from whoever asks: the
//! setting of a request is always the asker's own, a person's
//! (`Principal::actor`, which for a key somebody made is its maker's),
//! and reading and changing it take [`READ`] and [`WRITE`] of what the
//! asker holds. An agent and a device keep none.
//!
//! **Who hears.** Every change is one write with what is said of it
//! (`clock.kept`, the factory's for good: `journal::HERE`), carrying the
//! setting whole, so that another open page follows it with nothing read
//! again; it takes being its owner (`member:own:<actor>`), so nobody
//! else is told it whatever they hold. One that comes to what is kept
//! already is no write, and nothing is said.

use journal::{Journal, optional};
use kernel::principal::own;
use kernel::{Error, Principal, PrincipalKind, Result, Source};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// What reading one's own setting takes.
pub const READ: &str = "clock:read";
/// What changing it takes.
pub const WRITE: &str = "clock:write";
/// The most letters a zone's name has: the longest in the zones' own list
/// is some thirty.
pub const LONGEST_ZONE: usize = 64;
/// The two ways a time is written.
pub const HOURS: [&str; 2] = ["12h", "24h"];
/// How a time is written where nothing is said.
pub const USUAL_HOURS: &str = "12h";

pub(crate) const SCHEMA: &[&str] = &["
    CREATE TABLE clocked (
        account TEXT PRIMARY KEY,
        setting TEXT NOT NULL,
        nth INTEGER NOT NULL,
        changed INTEGER NOT NULL
    ) WITHOUT ROWID;
"];

/// How somebody has a time written for them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Setting {
    /// The zone, by its name in the zones' own list.
    pub zone: String,
    /// `12h` or `24h`.
    pub hours: String,
}

/// What somebody is handed of their setting.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Kept {
    /// What is kept: nothing for an account that never kept one.
    pub clock: Option<Setting>,
    /// How many times it was written, which grows with each write: what
    /// a page that hears of two tells the later by.
    pub nth: i64,
    /// When it was last written.
    pub changed: Option<i64>,
}

/// Why `zone` is not the name of a zone as such a name is written, if it
/// is not. Whether a zone of that name is known is the client's to say.
fn unfit_zone(zone: &Value) -> Option<String> {
    let Some(zone) = zone.as_str() else {
        return Some("A zone is said by its name, such as `America/Denver`, which is text.".to_string());
    };
    if zone.is_empty() {
        return Some("Say which zone, by its name, such as `America/Denver`.".to_string());
    }
    let letters = zone.chars().count();
    if letters > LONGEST_ZONE {
        return Some(format!("A zone's name is {LONGEST_ZONE} letters at most, and this one is {letters}."));
    }
    if zone.chars().any(char::is_control) {
        return Some("A zone's name is one line.".to_string());
    }
    if let Some(other) =
        zone.chars().find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-' | '+')))
    {
        return Some(format!(
            "A zone's name is made of letters, digits, `_`, `-` and `+`, in parts between `/`, and `{other}` is \
             none of them."
        ));
    }
    if zone.split('/').any(|part| !part.chars().next().is_some_and(|c| c.is_ascii_alphabetic())) {
        return Some(format!(
            "`{zone}` is no zone's name: each of its parts between `/` begins with a letter, such as \
             `America/Denver`."
        ));
    }
    None
}

/// The setting `said` is, as it is kept, or why it is not one.
pub fn checked(said: &Value) -> std::result::Result<Setting, String> {
    let Some(said) = said.as_object() else {
        return Err("How a time is written is a mapping of `zone` and `hours`.".to_string());
    };
    if let Some(other) = said.keys().find(|key| !["zone", "hours"].contains(&key.as_str())) {
        return Err(format!(
            "`{other}` is nothing said of how a time is written: it says `zone` and `hours`."
        ));
    }
    let zone = said.get("zone").unwrap_or(&Value::Null);
    if let Some(wrong) = unfit_zone(zone) {
        return Err(wrong);
    }
    let hours = match said.get("hours") {
        None | Some(Value::Null) => USUAL_HOURS,
        Some(hours) => match hours.as_str().filter(|hours| HOURS.contains(hours)) {
            Some(hours) => hours,
            None => return Err(format!("`hours` is `12h` or `24h`, and {hours} is neither.")),
        },
    };
    Ok(Setting { zone: zone.as_str().unwrap_or_default().to_string(), hours: hours.to_string() })
}

/// The account whose setting `by`'s is: a person's, or a key's maker's.
fn account_of(by: &Principal) -> Result<String> {
    match by.kind {
        PrincipalKind::User => Ok(by.id.clone()),
        _ => Err(Error::invalid(
            "How a time is written is kept with a person's account, and an agent or a device has none: sign in \
             as a person.",
        )),
    }
}

fn kept_of(connection: &rusqlite::Connection, account: &str) -> rusqlite::Result<Kept> {
    let row: Option<(String, i64, i64)> = optional(connection.query_row(
        "SELECT setting, nth, changed FROM clocked WHERE account = ?1",
        [account],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ))?;
    Ok(match row {
        Some((setting, nth, changed)) => {
            Kept { clock: serde_json::from_str(&setting).ok(), nth, changed: Some(changed) }
        }
        None => Kept { clock: None, nth: 0, changed: None },
    })
}

/// The keeper of how everybody has a time written for them.
#[derive(Clone)]
pub struct Clocked {
    journal: Journal,
}

impl Clocked {
    pub async fn open(journal: Journal) -> Result<Clocked> {
        journal.db().migrate("clocked", SCHEMA).await?;
        Ok(Clocked { journal })
    }

    /// What `by` is handed of their setting: what is kept, or nothing
    /// where they never kept one.
    pub async fn of(&self, by: &Principal) -> Result<Kept> {
        by.require(READ)?;
        let account = account_of(by)?;
        self.journal.db().call(move |connection| kept_of(connection, &account)).await
    }

    /// The zone the person who is `actor` (`user:<account>`) keeps, where
    /// they keep one: for what comes to them at a time of their day, which
    /// is in no request of theirs.
    pub async fn zone_of(&self, actor: &str) -> Option<String> {
        let account = actor.strip_prefix("user:")?.to_string();
        let kept = self.journal.db().call(move |connection| kept_of(connection, &account)).await.ok()?;
        kept.clock.map(|clock| clock.zone)
    }

    /// Keeps `said` as `by`'s setting, whole. Where `first` is so it is
    /// kept only where `by` keeps none yet, which is how the zone a
    /// browser says it is in is taken up: two windows that each hand
    /// theirs over have the first's kept and are both answered with it.
    /// One that comes to what is kept is as it was, and nothing is said.
    /// Either way the answer is what is kept from then.
    pub async fn keep(&self, by: &Principal, said: &Value, first: bool) -> Result<Kept> {
        by.require(WRITE)?;
        let account = account_of(by)?;
        let setting = checked(said).map_err(Error::invalid)?;
        let (by, now) = (by.clone(), self.journal.now_ms());
        let wrote = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            let was = kept_of(transaction, &account)?;
            if (first && was.clock.is_some()) || was.clock.as_ref() == Some(&setting) {
                return Ok(was);
            }
            let written = serde_json::to_string(&setting).unwrap_or_default();
            transaction.execute(
                "INSERT INTO clocked (account, setting, nth, changed) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (account) DO UPDATE
                 SET setting = excluded.setting, nth = excluded.nth, changed = excluded.changed",
                params![account, written, was.nth + 1, now],
            )?;
            let kept = kept_of(transaction, &account)?;
            let owner = format!("user:{account}");
            let source = Source::actor(by.actor()).taking(Some(&own(&owner)));
            let data = json!({ "owner": owner, "clock": kept.clock, "nth": kept.nth });
            writing.say_here("clock.kept", source, data)?;
            Ok(kept)
        });
        Ok(wrote.await?.0)
    }
}
