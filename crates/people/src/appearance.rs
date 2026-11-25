//! How what somebody reads in the client looks for them, kept with their
//! account, so that it follows them to every window and browser they
//! sign in to.
//!
//! **A setting** is five words: how big the text of what is read is
//! (`text`, the sizes a theme's look has: [`TEXT`]), how wide it is laid
//! out (`width`: the usual measure, `standard`, or `wide`, the page's
//! width up to a bound), whether the client is light, dark or as the
//! system asks (`colour`: `automatic`, `light`, `dark`), whether a long
//! article shows its contents beside it or keeps them hidden until asked
//! (`contents`: `shown`, `hidden`), and the letters an article or a
//! library's document is read in (`typeface`: `sans`, `serif`). Each not said is
//! as it usually is, the first of [`SETTINGS`]' usual words. It is one row
//! of `appearance`, the factory's own and never a project's, under the
//! account it is of; a copy of the factory keeps it, as it keeps the
//! accounts. It is beside a theme's look, not part of one: a theme is
//! chosen among others and may be shared, and what is read is one
//! reader's.
//!
//! **The daemon stores it and acts on none of it.** What each word comes
//! to on the page is the client's (`ui/src/appearance.ts`).
//!
//! **Whose it is.** Nothing here takes an owner from whoever asks: the
//! setting of a request is always the asker's own, a person's
//! (`Principal::actor`, which for a key somebody made is its maker's).
//! It is how the client looks for them, as their themes are, so reading
//! and changing it take what reading and changing those take
//! ([`READ`], [`WRITE`]). An agent and a device keep none.
//!
//! **Who hears.** Every change is one write with what is said of it
//! (`appearance.kept`, the factory's for good: `journal::HERE`), carrying
//! the setting whole, so that another open page follows it with nothing
//! read again; it takes being its owner (`member:own:<actor>`), so nobody
//! else is told it whatever they hold. One that comes to what is kept
//! already is no write, and nothing is said.

use journal::{Journal, optional};
use kernel::principal::own;
use kernel::{Error, Principal, PrincipalKind, Result, Source};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// What reading one's own setting takes: what reading one's themes does.
pub const READ: &str = crate::themes::READ;
/// What changing it takes.
pub const WRITE: &str = crate::themes::WRITE;
/// How big the text of what is read may be: the sizes a theme's look has.
pub const TEXT: &[&str] = crate::themes::LOOKS[1].1;
/// How wide what is read may be laid out.
pub const WIDTH: &[&str] = &["standard", "wide"];
/// Whether the client is light or dark: as the system asks, or always one.
pub const COLOUR: &[&str] = &["automatic", "light", "dark"];
/// Whether a long article's contents are shown beside it, or hidden until asked for.
pub const CONTENTS: &[&str] = &["shown", "hidden"];
/// The letters an article or a library's document is read in.
pub const TYPEFACE: &[&str] = &["sans", "serif"];
/// Each thing a setting says, its words, and the one it is where nothing is said.
pub const SETTINGS: [(&str, &[&str], &str); 5] = [
    ("text", TEXT, crate::themes::USUAL),
    ("width", WIDTH, "standard"),
    ("colour", COLOUR, "automatic"),
    ("contents", CONTENTS, "shown"),
    ("typeface", TYPEFACE, "sans"),
];

pub(crate) const SCHEMA: &[&str] = &["
    CREATE TABLE appearance (
        account TEXT PRIMARY KEY,
        setting TEXT NOT NULL,
        nth INTEGER NOT NULL,
        changed INTEGER NOT NULL
    ) WITHOUT ROWID;
"];

/// How what somebody reads looks for them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Setting {
    pub text: String,
    pub width: String,
    pub colour: String,
    /// Kept before it was a word of a setting, a setting says nothing of it: as usual.
    #[serde(default = "contents_usually")]
    pub contents: String,
    /// Kept before it was a word of a setting, a setting says nothing of it: as usual.
    #[serde(default = "typeface_usually")]
    pub typeface: String,
}

fn contents_usually() -> String {
    "shown".to_string()
}

fn typeface_usually() -> String {
    "sans".to_string()
}

/// What somebody is handed of their setting.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Kept {
    /// What is kept: nothing for an account that never kept one.
    pub appearance: Option<Setting>,
    /// How many times it was written, which grows with each write: what
    /// a page that hears of two tells the later by.
    pub nth: i64,
    /// When it was last written.
    pub changed: Option<i64>,
}

/// The setting `said` is, as it is kept, or why it is not one.
pub fn checked(said: &Value) -> std::result::Result<Setting, String> {
    let Some(said) = said.as_object() else {
        return Err(
            "How what one reads looks is a mapping of `text`, `width`, `colour`, `contents` and `typeface`."
                .to_string(),
        );
    };
    if let Some(other) = said.keys().find(|key| !SETTINGS.iter().any(|(name, ..)| name == key)) {
        return Err(format!(
            "`{other}` is nothing said of how what one reads looks: it says `text`, `width`, `colour`, `contents` and \
             `typeface`."
        ));
    }
    let mut chosen = Vec::with_capacity(SETTINGS.len());
    for (name, words, usual) in SETTINGS {
        let word = match said.get(name) {
            None | Some(Value::Null) => usual,
            Some(word) => match word.as_str().filter(|word| words.contains(word)) {
                Some(word) => word,
                None => {
                    return Err(format!(
                        "`{name}` is one of {}, and {word} is none of them.",
                        words.join(", ")
                    ));
                }
            },
        };
        chosen.push(word.to_string());
    }
    let [text, width, colour, contents, typeface]: [String; 5] = chosen.try_into().unwrap_or_default();
    Ok(Setting { text, width, colour, contents, typeface })
}

/// The account whose setting `by`'s is: a person's, or a key's maker's.
fn account_of(by: &Principal) -> Result<String> {
    match by.kind {
        PrincipalKind::User => Ok(by.id.clone()),
        _ => Err(Error::invalid(
            "How what one reads looks is kept with a person's account, and an agent or a device has none: sign \
             in as a person.",
        )),
    }
}

fn kept_of(connection: &rusqlite::Connection, account: &str) -> rusqlite::Result<Kept> {
    let row: Option<(String, i64, i64)> = optional(connection.query_row(
        "SELECT setting, nth, changed FROM appearance WHERE account = ?1",
        [account],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ))?;
    Ok(match row {
        Some((setting, nth, changed)) => {
            Kept { appearance: serde_json::from_str(&setting).ok(), nth, changed: Some(changed) }
        }
        None => Kept { appearance: None, nth: 0, changed: None },
    })
}

/// The keeper of how what everybody reads looks for them.
#[derive(Clone)]
pub struct Appearance {
    journal: Journal,
}

impl Appearance {
    pub async fn open(journal: Journal) -> Result<Appearance> {
        journal.db().migrate("appearance", SCHEMA).await?;
        Ok(Appearance { journal })
    }

    /// What `by` is handed of their setting: what is kept, or nothing
    /// where they never kept one.
    pub async fn of(&self, by: &Principal) -> Result<Kept> {
        by.require(READ)?;
        let account = account_of(by)?;
        self.journal.db().call(move |connection| kept_of(connection, &account)).await
    }

    /// Keeps `said` as `by`'s setting, whole. One that comes to what is
    /// kept is as it was, and nothing is said. Either way the answer is
    /// what is kept from then.
    pub async fn keep(&self, by: &Principal, said: &Value) -> Result<Kept> {
        by.require(WRITE)?;
        let account = account_of(by)?;
        let setting = checked(said).map_err(Error::invalid)?;
        let (by, now) = (by.clone(), self.journal.now_ms());
        let wrote = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            let was = kept_of(transaction, &account)?;
            if was.appearance.as_ref() == Some(&setting) {
                return Ok(was);
            }
            let written = serde_json::to_string(&setting).unwrap_or_default();
            transaction.execute(
                "INSERT INTO appearance (account, setting, nth, changed) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (account) DO UPDATE
                 SET setting = excluded.setting, nth = excluded.nth, changed = excluded.changed",
                params![account, written, was.nth + 1, now],
            )?;
            let kept = kept_of(transaction, &account)?;
            let owner = format!("user:{account}");
            let source = Source::actor(by.actor()).taking(Some(&own(&owner)));
            let data = json!({ "owner": owner, "appearance": kept.appearance, "nth": kept.nth });
            writing.say_here("appearance.kept", source, data)?;
            Ok(kept)
        });
        Ok(wrote.await?.0)
    }
}
