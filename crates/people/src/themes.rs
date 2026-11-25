//! The themes somebody makes for the client, kept with their account, and
//! those an admin offers to everybody.
//!
//! **A theme** is what it is called and eight colours (`ground`, `text`,
//! `accent`, `ok`, `bad`, `info`, `agent`, `card`): the client works the
//! rest of a theme's list out from those. It may say eight more, its
//! light version, which the client wears where the system asks for
//! light, as the themes that ship do; one that says none is the same in
//! both. And it may say how the client looks beyond its colours, its
//! look ([`LOOKS`]): the letters, their size, how close things stand
//! and how round corners are, each a word of a few, and each it does not
//! say as it usually is, in both versions. It is a row of `themes`, the factory's own and never a
//! project's, under the account that made it, so that it is there in
//! any browser its owner signs in to; and which theme somebody has
//! chosen is a row of `themes_chosen`, theirs the same way. A copy of the factory keeps both, as it keeps the accounts.
//!
//! **Checked where it is written** ([`checked`], with its table of cases
//! beside the client's, which holds the same colours to the same
//! sentences): a name that is something and not long, eight colours each
//! `#rrggbb` and nothing else, text that can be read on its ground, the
//! same of its light version on its own where it says one, each setting
//! of its look one of that setting's words, and [`MOST`]
//! themes an account at most. Anything else is refused in a sentence and
//! nothing is written.
//!
//! **Whose they are.** Nothing here takes an owner from whoever asks:
//! the themes of a request are always the asker's own, a person's
//! (`Principal::actor`, which for a key somebody made is its maker's),
//! and reading and changing them take [`READ`] and [`WRITE`] of what the
//! asker holds, so a key reads its maker's only where it was given that.
//! An agent and a device keep none. A theme of somebody else's answers as
//! one that is not there.
//!
//! **Offered to everybody.** Whoever may manage the factory ([`OFFER`])
//! offers one of their own, which is then listed for everybody beside the
//! ones that ship, with who offered it, and may be chosen by anybody; and
//! whoever may manage it withdraws one, whoever offered it, after which it
//! is its owner's alone again. It stays its owner's to change and take
//! away while it is offered.
//!
//! **Who hears.** Every change is one write with what is said of it
//! (`theme.*`, the factory's for good: `journal::HERE`). What is said of
//! a theme that is somebody's alone, and of what they chose, takes being
//! them (`member:own:<actor>`, [`kernel::principal::own`]), so nobody else
//! is told it whatever they hold; what is said of one offered to
//! everybody is for whoever may read themes. Each carries the theme
//! whole, so a page follows it with nothing read again.

use journal::{Journal, optional};
use kernel::principal::own;
use kernel::{Error, Principal, PrincipalKind, Result, Source};
use rusqlite::params;
use serde::Serialize;
use serde_json::{Map, Value, json};

/// What reading one's own themes, and those offered, takes.
pub const READ: &str = "themes:read";
/// What making, changing, taking away and choosing one's own takes.
pub const WRITE: &str = "themes:write";
/// What offering one to everybody, and withdrawing one, takes beside [`WRITE`].
pub const OFFER: &str = "admin";
/// The most themes one account keeps.
pub const MOST: usize = 40;
/// The most letters a theme's name has.
pub const LONGEST: usize = 60;
/// The themes the client ships, which anybody may choose and nobody
/// keeps here. The client's list is held to this one by the table of
/// cases both are held to.
pub const SHIPPED: &[&str] = &["grid-iron", "rose-pine", "tokyo-midnight"];
/// The colours a theme is made of, in the order the client asks them.
pub const COLOURS: [&str; 8] = ["ground", "text", "accent", "ok", "bad", "info", "agent", "card"];
/// What a theme may say of how the client looks beyond its colours, each
/// setting with the words it may be, in the order the client asks them.
/// `usual` is what a setting is where nothing is said of it, and is kept
/// as nothing said. The client says what each word comes to on the page
/// (`ui/src/looks.ts`), held to these by the table of cases both read.
pub const LOOKS: [(&str, &[&str]); 4] = [
    ("letters", &["usual", "serif", "humanist", "mono"]),
    ("size", &["smaller", "usual", "larger", "largest"]),
    ("density", &["compact", "usual", "roomy"]),
    ("corners", &["square", "usual", "round"]),
];
/// The word for a setting of a look that says nothing.
pub const USUAL: &str = "usual";

pub(crate) const SCHEMA: &[&str] = &[
    "
    CREATE TABLE themes (
        id TEXT PRIMARY KEY,
        account TEXT NOT NULL,
        name TEXT NOT NULL,
        colours TEXT NOT NULL,
        made INTEGER NOT NULL,
        changed INTEGER NOT NULL,
        offered INTEGER,
        offered_by TEXT
    );
    CREATE INDEX themes_of ON themes (account);
    CREATE INDEX themes_offered ON themes (offered) WHERE offered IS NOT NULL;

    CREATE TABLE themes_chosen (
        account TEXT PRIMARY KEY,
        theme TEXT NOT NULL,
        chosen INTEGER NOT NULL
    ) WITHOUT ROWID;
",
    // A theme's light version, where it says one: its eight colours as
    // `colours` keeps them, or nothing.
    "ALTER TABLE themes ADD COLUMN light TEXT;",
    // What a theme says of how the client looks beyond its colours: a
    // mapping of the settings it says, each a word, or nothing for a
    // theme that says none.
    "ALTER TABLE themes ADD COLUMN look TEXT;",
];

/// A theme as it is handed out.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Theme {
    pub id: String,
    /// Whose it is, as an actor (`user:<account>`).
    pub owner: String,
    /// What its owner is called.
    pub owner_name: String,
    pub name: String,
    /// The eight colours, by what each is for.
    pub colours: Map<String, Value>,
    /// Its light version's eight, worn where the system asks for light:
    /// nothing for a theme that is the same in both.
    pub light: Option<Map<String, Value>>,
    /// What it says of how the client looks beyond its colours, by
    /// setting: empty for a theme that says nothing of it.
    pub look: Map<String, Value>,
    pub made: i64,
    pub changed: i64,
    /// Who offered it to everybody and when, while it is offered.
    pub offered: Option<Offered>,
}

/// That a theme is offered to everybody.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Offered {
    /// Who offered it, as an actor.
    pub by: String,
    /// What they are called.
    pub name: String,
    pub at: i64,
}

/// What somebody is handed of themes: their own, those offered to
/// everybody that are not theirs, and the one they chose, where it is one
/// that may still be chosen.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Kept {
    pub own: Vec<Theme>,
    pub offered: Vec<Theme>,
    pub chosen: Option<String>,
}

fn is_hex(colour: &str) -> bool {
    colour.len() == 7 && colour.starts_with('#') && colour[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// How light a colour is to the eye, from 0 to 1, as the client says it.
fn lightness(colour: &str) -> f64 {
    let part = |at: usize| {
        let share = f64::from(u8::from_str_radix(&colour[at..at + 2], 16).unwrap_or(0)) / 255.0;
        if share <= 0.03928 { share / 12.92 } else { ((share + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * part(1) + 0.7152 * part(3) + 0.0722 * part(5)
}

/// Why these colours would not make a theme that can be read, if they
/// would not: the client's `unreadable`, sentence for sentence.
pub fn unreadable(colours: &Value) -> Option<String> {
    let Some(colours) = colours.as_object() else {
        return Some(format!("A theme's colours are a mapping of eight: {}.", COLOURS.join(", ")));
    };
    for name in COLOURS {
        if !colours.get(name).and_then(Value::as_str).is_some_and(is_hex) {
            return Some(format!("{name} is not a colour"));
        }
    }
    let of = |name: &str| lightness(colours[name].as_str().unwrap_or_default());
    let (ground, text) = (of("ground"), of("text"));
    let (low, high) = (ground.min(text), ground.max(text));
    ((high + 0.05) / (low + 0.05) < 4.5).then(|| {
        "The text is too near the ground to be read on it: make one lighter or the other darker.".to_string()
    })
}

/// What is said where a theme's light version is not one: the sentence
/// for its dark one, after the words that say which.
pub const LIGHT: &str = "In the light version: ";

/// Why these colours, with a light version where there is one, would not
/// make a theme that can be read: the dark one's first, then the light
/// one's on its own. The client's `unreadableEither`, sentence for
/// sentence.
pub fn unreadable_with(colours: &Value, light: Option<&Value>) -> Option<String> {
    let light = light.filter(|light| !light.is_null());
    unreadable(colours).or_else(|| light.and_then(unreadable).map(|wrong| format!("{LIGHT}{wrong}")))
}

/// Eight colours as they are kept, or the first that is no colour of a
/// theme.
fn kept_colours(colours: &Value) -> std::result::Result<Map<String, Value>, String> {
    let given = colours.as_object().cloned().unwrap_or_default();
    if let Some(other) = given.keys().find(|key| !COLOURS.contains(&key.as_str())) {
        return Err(format!(
            "`{other}` is no colour of a theme: a theme's colours are {}.",
            COLOURS.join(", ")
        ));
    }
    Ok(COLOURS
        .iter()
        .map(|name| (name.to_string(), json!(given[*name].as_str().unwrap_or_default().to_ascii_lowercase())))
        .collect())
}

/// Why this would not be a theme's look, if it would not: the client's
/// `unfit`, sentence for sentence. Nothing (`null`) is a look that says
/// nothing.
pub fn unfit(look: &Value) -> Option<String> {
    let settings = || LOOKS.iter().map(|(setting, _)| *setting).collect::<Vec<_>>().join(", ");
    let look = match look {
        Value::Null => return None,
        Value::Object(look) => look,
        _ => return Some(format!("A theme's look is a mapping of what it says of {}.", settings())),
    };
    for (key, said) in look {
        let Some((setting, words)) = LOOKS.iter().find(|(setting, _)| setting == key) else {
            return Some(format!("`{key}` is nothing a theme's look says: it says {}.", settings()));
        };
        if !said.as_str().is_some_and(|word| words.contains(&word)) {
            return Some(format!(
                "A theme's {setting} is one of {}, and {said} is none of them.",
                words.join(", ")
            ));
        }
    }
    None
}

/// A look as it is kept: the settings it says, leaving out each that is
/// the usual one.
fn kept_look(look: &Value) -> Map<String, Value> {
    LOOKS
        .iter()
        .filter_map(|(setting, _)| {
            let word = look.get(setting).and_then(Value::as_str)?;
            (word != USUAL).then(|| (setting.to_string(), json!(word)))
        })
        .collect()
}

/// A theme as it is checked: its name, its colours, its light version
/// and its look.
pub type Checked = (String, Map<String, Value>, Option<Map<String, Value>>, Map<String, Value>);

/// What a theme is called, its colours, its light version and its look
/// as they are kept, or why they are not a theme: the name trimmed, each
/// version the eight, each written small, with nothing beside them, and
/// the look what it says that is not as usual. A light version or a look
/// that is nothing (`null`) is none.
pub fn checked(
    name: &str,
    colours: &Value,
    light: Option<&Value>,
    look: Option<&Value>,
) -> std::result::Result<Checked, String> {
    let light = light.filter(|light| !light.is_null());
    let look = look.unwrap_or(&Value::Null);
    let name = name.trim();
    if name.is_empty() {
        return Err("A theme is called something: say what this one is called.".to_string());
    }
    let letters = name.chars().count();
    if letters > LONGEST {
        return Err(format!("A theme's name is {LONGEST} letters at most, and this one is {letters}."));
    }
    if name.chars().any(char::is_control) {
        return Err("A theme's name is one line of words.".to_string());
    }
    if let Some(wrong) = unreadable_with(colours, light) {
        return Err(wrong);
    }
    let kept = kept_colours(colours)?;
    let light = match light {
        Some(light) => Some(kept_colours(light).map_err(|wrong| format!("{LIGHT}{wrong}"))?),
        None => None,
    };
    if let Some(wrong) = unfit(look) {
        return Err(wrong);
    }
    Ok((name.to_string(), kept, light, kept_look(look)))
}

/// A look as its column keeps it: nothing for one that says nothing.
fn look_written(look: &Map<String, Value>) -> Option<String> {
    (!look.is_empty()).then(|| serde_json::to_string(look).unwrap_or_default())
}

/// What reading a theme reads, in [`read_theme`]'s order.
const THEME: &str = "t.id, t.account, COALESCE(a.name, ''), t.name, t.colours, t.made, t.changed, t.offered,
                     t.offered_by, COALESCE(b.name, ''), t.light, t.look
                     FROM themes t
                     LEFT JOIN accounts a ON a.id = t.account
                     LEFT JOIN accounts b ON b.id = t.offered_by";

fn read_theme(row: &rusqlite::Row<'_>) -> rusqlite::Result<Theme> {
    let colours: String = row.get(4)?;
    let offered: Option<i64> = row.get(7)?;
    let by: Option<String> = row.get(8)?;
    let light: Option<String> = row.get(10)?;
    let look: Option<String> = row.get(11)?;
    Ok(Theme {
        id: row.get(0)?,
        owner: format!("user:{}", row.get::<_, String>(1)?),
        owner_name: row.get(2)?,
        name: row.get(3)?,
        colours: serde_json::from_str(&colours).unwrap_or_default(),
        light: light.map(|light| serde_json::from_str(&light).unwrap_or_default()),
        look: look.map(|look| serde_json::from_str(&look).unwrap_or_default()).unwrap_or_default(),
        made: row.get(5)?,
        changed: row.get(6)?,
        offered: offered.map(|at| Offered {
            by: format!("user:{}", by.unwrap_or_default()),
            name: row.get(9).unwrap_or_default(),
            at,
        }),
    })
}

fn theme_of(connection: &rusqlite::Connection, id: &str) -> rusqlite::Result<Option<Theme>> {
    optional(connection.query_row(&format!("SELECT {THEME} WHERE t.id = ?1"), [id], read_theme))
}

/// Whether `account` may choose `theme`: one that ships, one of theirs,
/// or one offered to everybody.
fn choosable(connection: &rusqlite::Connection, account: &str, theme: &str) -> rusqlite::Result<bool> {
    if SHIPPED.contains(&theme) {
        return Ok(true);
    }
    let found: Option<i64> = optional(connection.query_row(
        "SELECT 1 FROM themes WHERE id = ?1 AND (account = ?2 OR offered IS NOT NULL)",
        [theme, account],
        |row| row.get(0),
    ))?;
    Ok(found.is_some())
}

/// The account whose themes `by`'s are: a person's, or a key's maker's.
fn account_of(by: &Principal) -> Result<String> {
    match by.kind {
        PrincipalKind::User => Ok(by.id.clone()),
        _ => Err(Error::invalid(
            "Themes are kept with a person's account, and an agent or a device has none: sign in as a person.",
        )),
    }
}

fn no_theme(id: &str) -> Error {
    Error::not_found(format!("You have no theme `{id}`."))
}

/// How a theme is said: to its owner alone, or to everybody while it is
/// offered.
fn said_of(theme: &Theme, by: &Principal) -> Source {
    let source = Source::actor(by.actor());
    match theme.offered {
        Some(_) => source,
        None => source.taking(Some(&own(&theme.owner))),
    }
}

/// What a write comes to, where it may be refused in a sentence.
enum Came<T> {
    Did(T),
    Refused(Error),
}

fn came<T>(wrote: Came<T>) -> Result<T> {
    match wrote {
        Came::Did(did) => Ok(did),
        Came::Refused(error) => Err(error),
    }
}

/// The keeper of everybody's themes.
#[derive(Clone)]
pub struct Themes {
    journal: Journal,
}

impl Themes {
    pub async fn open(journal: Journal) -> Result<Themes> {
        journal.db().migrate("themes", SCHEMA).await?;
        Ok(Themes { journal })
    }

    /// What `by` is handed of themes: their own, those offered that are
    /// not theirs, and what they chose.
    pub async fn of(&self, by: &Principal) -> Result<Kept> {
        by.require(READ)?;
        let account = account_of(by)?;
        self.journal
            .db()
            .call(move |connection| {
                let mut theirs =
                    connection.prepare(&format!("SELECT {THEME} WHERE t.account = ?1 ORDER BY t.made, t.rowid"))?;
                let own = theirs.query_map([&account], read_theme)?.collect::<rusqlite::Result<Vec<_>>>()?;
                let mut offered = connection.prepare(&format!(
                    "SELECT {THEME} WHERE t.offered IS NOT NULL AND t.account <> ?1 ORDER BY t.offered, t.rowid"
                ))?;
                let offered = offered.query_map([&account], read_theme)?.collect::<rusqlite::Result<Vec<_>>>()?;
                let chosen: Option<String> = optional(connection.query_row(
                    "SELECT theme FROM themes_chosen WHERE account = ?1",
                    [&account],
                    |row| row.get(0),
                ))?;
                // One that went, or was withdrawn, is chosen no longer.
                let chosen = match chosen {
                    Some(theme) if choosable(connection, &account, &theme)? => Some(theme),
                    _ => None,
                };
                Ok(Kept { own, offered, chosen })
            })
            .await
    }

    /// Makes a theme of `by`'s own. One that is the same as one of
    /// theirs, by its name, its colours, its light version and its look,
    /// is that one, and nothing is written or said: a browser that hands
    /// over what it kept twice, or a file read in twice, hands it over
    /// once.
    pub async fn make(
        &self,
        by: &Principal,
        name: &str,
        colours: &Value,
        light: Option<&Value>,
        look: Option<&Value>,
    ) -> Result<Theme> {
        by.require(WRITE)?;
        let account = account_of(by)?;
        let (name, colours, light, look) = checked(name, colours, light, look).map_err(Error::invalid)?;
        let (by, now) = (by.clone(), self.journal.now_ms());
        let written = serde_json::to_string(&colours).unwrap_or_default();
        let light = light.map(|light| serde_json::to_string(&light).unwrap_or_default());
        let look = look_written(&look);
        let id = kernel::id::new("thm", now);
        let wrote = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            let same: Option<String> = optional(transaction.query_row(
                "SELECT id FROM themes
                 WHERE account = ?1 AND name = ?2 AND colours = ?3 AND light IS ?4 AND look IS ?5",
                params![account, name, written, light, look],
                |row| row.get(0),
            ))?;
            if let Some(same) = same {
                return Ok(Came::Did(
                    theme_of(transaction, &same)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?,
                ));
            }
            let kept: i64 = transaction.query_row(
                "SELECT COUNT(*) FROM themes WHERE account = ?1",
                [&account],
                |row| row.get(0),
            )?;
            if kept as usize >= MOST {
                return Ok(Came::Refused(Error::invalid(format!(
                    "You have {MOST} themes, which is as many as one person keeps: take one away first."
                ))));
            }
            transaction.execute(
                "INSERT INTO themes (id, account, name, colours, light, look, made, changed)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params![id, account, name, written, light, look, now],
            )?;
            let theme = theme_of(transaction, &id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            let data = json!({ "owner": theme.owner, "theme": theme });
            writing.say_here("theme.kept", said_of(&theme, &by), data)?;
            Ok(Came::Did(theme))
        });
        came(wrote.await?.0)
    }

    /// Changes what one of `by`'s own themes is called, its colours, its
    /// light version and its look, whole: one said with no light version
    /// has none from then, and one said with no look says nothing of it.
    /// One that comes to what it was is as it was, and nothing is said.
    pub async fn change(
        &self,
        by: &Principal,
        id: &str,
        name: &str,
        colours: &Value,
        light: Option<&Value>,
        look: Option<&Value>,
    ) -> Result<Theme> {
        by.require(WRITE)?;
        let account = account_of(by)?;
        let (name, colours, light, look) = checked(name, colours, light, look).map_err(Error::invalid)?;
        let (by, now, id) = (by.clone(), self.journal.now_ms(), id.to_string());
        let written = serde_json::to_string(&colours).unwrap_or_default();
        let light_written = light.as_ref().map(|light| serde_json::to_string(light).unwrap_or_default());
        let look_kept = look_written(&look);
        let wrote = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            let Some(was) =
                theme_of(transaction, &id)?.filter(|theme| theme.owner == format!("user:{account}"))
            else {
                return Ok(Came::Refused(no_theme(&id)));
            };
            if was.name == name && was.colours == colours && was.light == light && was.look == look {
                return Ok(Came::Did(was));
            }
            transaction.execute(
                "UPDATE themes SET name = ?2, colours = ?3, light = ?4, look = ?5, changed = ?6 WHERE id = ?1",
                params![id, name, written, light_written, look_kept, now],
            )?;
            let theme = theme_of(transaction, &id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            let data = json!({ "owner": theme.owner, "theme": theme });
            writing.say_here("theme.kept", said_of(&theme, &by), data)?;
            Ok(Came::Did(theme))
        });
        came(wrote.await?.0)
    }

    /// Takes one of `by`'s own themes away, from everybody where it was
    /// offered. Whoever had it chosen is in the one that is usual.
    pub async fn take_away(&self, by: &Principal, id: &str) -> Result<()> {
        by.require(WRITE)?;
        let account = account_of(by)?;
        let (by, id) = (by.clone(), id.to_string());
        let wrote = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            let Some(was) =
                theme_of(transaction, &id)?.filter(|theme| theme.owner == format!("user:{account}"))
            else {
                return Ok(Came::Refused(no_theme(&id)));
            };
            transaction.execute("DELETE FROM themes WHERE id = ?1", [&id])?;
            transaction.execute("DELETE FROM themes_chosen WHERE theme = ?1", [&id])?;
            let data = json!({ "owner": was.owner, "theme": id, "offered": was.offered.is_some() });
            writing.say_here("theme.gone", said_of(&was, &by), data)?;
            Ok(Came::Did(()))
        });
        came(wrote.await?.0)
    }

    /// Says which theme `by` has chosen: one that ships, one of their
    /// own or one offered to everybody; nothing for none, which is the
    /// usual one. One chosen already is as it was, and nothing is said.
    pub async fn choose(&self, by: &Principal, theme: Option<&str>) -> Result<Option<String>> {
        by.require(WRITE)?;
        let account = account_of(by)?;
        let (by, now, theme) = (by.clone(), self.journal.now_ms(), theme.map(str::to_string));
        let wrote = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            if let Some(theme) = &theme
                && !choosable(transaction, &account, theme)?
            {
                return Ok(Came::Refused(Error::not_found(format!(
                    "There is no theme `{theme}` for you to choose: it is one that ships, one of your own or one \
                     offered to everybody."
                ))));
            }
            let was: Option<String> = optional(transaction.query_row(
                "SELECT theme FROM themes_chosen WHERE account = ?1",
                [&account],
                |row| row.get(0),
            ))?;
            if was == theme {
                return Ok(Came::Did(theme));
            }
            match &theme {
                Some(chosen) => transaction.execute(
                    "INSERT INTO themes_chosen (account, theme, chosen) VALUES (?1, ?2, ?3)
                     ON CONFLICT (account) DO UPDATE SET theme = excluded.theme, chosen = excluded.chosen",
                    params![account, chosen, now],
                )?,
                None => transaction.execute("DELETE FROM themes_chosen WHERE account = ?1", [&account])?,
            };
            let owner = format!("user:{account}");
            let source = Source::actor(by.actor()).taking(Some(&own(&owner)));
            writing.say_here("theme.chosen", source, json!({ "owner": owner, "theme": theme }))?;
            Ok(Came::Did(theme))
        });
        came(wrote.await?.0)
    }

    /// Offers one of `by`'s own themes to everybody. One offered already
    /// is as it was, and nothing is said.
    pub async fn offer(&self, by: &Principal, id: &str) -> Result<Theme> {
        by.require(WRITE)?;
        by.require(OFFER)?;
        let account = account_of(by)?;
        let (by, now, id) = (by.clone(), self.journal.now_ms(), id.to_string());
        let wrote = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            let Some(was) =
                theme_of(transaction, &id)?.filter(|theme| theme.owner == format!("user:{account}"))
            else {
                return Ok(Came::Refused(no_theme(&id)));
            };
            if was.offered.is_some() {
                return Ok(Came::Did(was));
            }
            transaction.execute(
                "UPDATE themes SET offered = ?2, offered_by = ?3 WHERE id = ?1",
                params![id, now, account],
            )?;
            let theme = theme_of(transaction, &id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            let data = json!({ "owner": theme.owner, "theme": theme, "by": by.actor() });
            writing.say_here("theme.offered", said_of(&theme, &by), data)?;
            Ok(Came::Did(theme))
        });
        came(wrote.await?.0)
    }

    /// Withdraws a theme offered to everybody, whoever offered it: it is
    /// its owner's alone again, and whoever else had it chosen is in the
    /// one that is usual.
    pub async fn withdraw(&self, by: &Principal, id: &str) -> Result<Theme> {
        by.require(WRITE)?;
        by.require(OFFER)?;
        let (by, id) = (by.clone(), id.to_string());
        let wrote = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            let Some(was) = theme_of(transaction, &id)?.filter(|theme| theme.offered.is_some()) else {
                return Ok(Came::Refused(Error::not_found(format!(
                    "There is no theme `{id}` offered to everybody."
                ))));
            };
            transaction
                .execute("UPDATE themes SET offered = NULL, offered_by = NULL WHERE id = ?1", [&id])?;
            transaction.execute(
                "DELETE FROM themes_chosen WHERE theme = ?1 AND account <> ?2",
                params![id, was.owner.trim_start_matches("user:")],
            )?;
            let theme = theme_of(transaction, &id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            // Said to everybody, who are to stop listing it: the theme
            // itself goes to its owner alone, in what is said after.
            let data = json!({ "owner": theme.owner, "theme": id, "by": by.actor() });
            writing.say_here("theme.withdrawn", Source::actor(by.actor()), data)?;
            let data = json!({ "owner": theme.owner, "theme": theme });
            writing.say_here("theme.kept", said_of(&theme, &by), data)?;
            Ok(Came::Did(theme))
        });
        came(wrote.await?.0)
    }
}
