//! Sign-ins the factory holds, and who has which.
//!
//! An agent's program signs in to its model's provider. Started with
//! nothing said, it does so as whoever runs the daemon, the same for every
//! slot. The factory can instead hold sign-ins of its own, each for one
//! agent package, and hand one to a slot for the length of a turn:
//!
//! - a **key** for the provider's API, and a **token**, the long-lived
//!   sign-in of a subscription: each a value, kept in a file only its owner
//!   can read, sealed with a key of this installation's ([`link::kept`]),
//!   in a folder of the data folder that no copy of the factory
//!   carries. The value is never in the database, an event, a measurement,
//!   a log or an argument: it goes from that file into the environment of
//!   the one program it is handed to.
//! - a **folder** in which the provider's program keeps and renews its own
//!   sign-in. The program is pointed at it where it is. It is never
//!   copied: a copy renews itself and puts the original out.
//!
//! Each may be held by so many turns at once (one, unless said), and is
//! given back when its turn ends, however it ends. A turn never outlives
//! the daemon, so a daemon that starts finds nothing held. A sign-in whose
//! provider says it has reached its limit is set aside until it is good
//! again; the limit is the sign-in's and not the agent's, so the work that
//! met it starts again on another.
//!
//! Which of the sign-ins a slot may draw it is handed is not decided here:
//! [`crate::rule`] is asked, with what is free and when each was last
//! used. This module is the mechanism under that: keeping, holding, giving
//! back, setting aside and handing over.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use kernel::{Cancel, Error, Principal, Result, Source};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::Notify;

use crate::handed::{Handing, Kind, Limit};
use crate::rule::{Asked, Decision, Offered};
use crate::{Agents, Slot};

/// What a slot says to draw any subscription of its package, or any key.
/// No sign-in can be called one of these, or a kind's name.
pub const SUBSCRIPTION: &str = "subscription";
pub const KEY: &str = "key";

/// A turn that holds a sign-in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Holder {
    pub slot: String,
    pub turn: String,
    /// When it was handed over, in milliseconds since 1970.
    pub since: i64,
}

/// A sign-in as it is listed. It never has the value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SignIn {
    pub name: String,
    /// The agent package whose program it signs in.
    #[serde(rename = "for")]
    pub package: String,
    /// `key`, `token` or `folder`.
    pub kind: &'static str,
    /// How many turns may hold it at once.
    pub at_once: u32,
    /// For a folder: where it is.
    pub folder: Option<String>,
    /// When it was last handed out.
    pub last_used: Option<i64>,
    pub held_by: Vec<Holder>,
    /// Set aside until then, having reached its limit.
    pub aside_until: Option<i64>,
    pub aside_why: Option<String>,
    pub created: i64,
}

/// A sign-in to add, or to put in place of the one of its name.
#[derive(Clone, Debug, Default)]
pub struct NewSignIn {
    pub name: String,
    pub package: String,
    pub kind: String,
    /// The value of a key or a token.
    pub value: Option<String>,
    /// Where a folder is.
    pub folder: Option<String>,
    pub at_once: Option<u32>,
}

struct Kept {
    kind: Kind,
    shown: SignIn,
}

impl Kept {
    fn aside(&self, now: i64) -> Option<i64> {
        self.shown.aside_until.filter(|until| *until > now)
    }

    fn free(&self, now: i64) -> bool {
        self.aside(now).is_none() && self.shown.held_by.len() < self.shown.at_once as usize
    }
}

/// Every sign-in, and what tells a waiting turn that something changed.
#[derive(Default)]
pub(crate) struct Pool {
    kept: Mutex<BTreeMap<String, Kept>>,
    changed: Notify,
}

const SIGNIN: &str =
    "name, package, kind, at_once, folder, last_used, held_by, aside_until, aside_why, created";

/// A sign-in's folder as its row has it, said as a whole path is said
/// now. On Windows a program from before whole paths were said plainly
/// (`kernel::whole`) wrote the system's long form of it (`\\?\C:\…`),
/// which is the same folder and other text: the folder added again
/// under another name was not seen to be a sign-in already, and a
/// program was handed the long form to keep its sign-in in. Read so, a
/// row is the folder that was written down, wherever it is compared or
/// handed on. Only there (`windows`): anywhere else a name may begin as
/// a long form does, and is left as it is, as `whole::said_plainly`
/// leaves it.
pub fn folder_kept(written: String, windows: bool) -> String {
    if windows { kernel::whole::plainly(&written) } else { written }
}

fn read_signin(row: &rusqlite::Row<'_>) -> rusqlite::Result<Kept> {
    let kind = Kind::parse(&row.get::<_, String>(2)?).unwrap_or(Kind::Key);
    Ok(Kept {
        kind,
        shown: SignIn {
            name: row.get(0)?,
            package: row.get(1)?,
            kind: kind.name(),
            at_once: row.get(3)?,
            folder: row.get::<_, Option<String>>(4)?.map(|folder| folder_kept(folder, cfg!(windows))),
            last_used: row.get(5)?,
            held_by: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
            aside_until: row.get(7)?,
            aside_why: row.get(8)?,
            created: row.get(9)?,
        },
    })
}

/// A sign-in a turn holds. Letting go of it gives it back, so that a turn
/// that is dropped where it stands does not keep one from the others.
pub(crate) struct Drawn {
    pub name: String,
    pub kind: Kind,
    folder: Option<String>,
    turn: String,
    slot: String,
    agents: Agents,
    given_back: bool,
}

/// What a program is started with for a turn that holds a sign-in.
pub(crate) struct Handed {
    /// The variables its environment gains, and those it loses.
    pub gains: BTreeMap<String, String>,
    pub loses: Vec<String>,
    /// The value of a key or a token, and the name it is said by instead.
    secret: Option<(String, String)>,
    /// The folders it was handed: the sign-in's own, where it is one, and
    /// the slot's in place of the program's usual. Behind a wall these are
    /// what it is shown to sign in with.
    pub folders: Vec<std::path::PathBuf>,
}

impl Handed {
    /// Whether its program is told a folder to keep its own things in,
    /// in place of where it usually keeps them in its home: what it reads
    /// there is then found there and not under its home.
    pub(crate) fn moves_its_folder(&self) -> bool {
        self.gains
            .values()
            .any(|value| self.folders.iter().any(|folder| folder.as_path() == std::path::Path::new(value)))
    }

    /// The same as [`Handed::unsaid`] for something that has the program's
    /// environment and not this: the variables the value is in, and what
    /// it is said as in its place. `None` where it was handed a value that
    /// is in no variable, which only what holds the value can take out.
    pub(crate) fn unsay(&self) -> Option<Vec<(String, String)>> {
        let Some((value, name)) = &self.secret else { return Some(Vec::new()) };
        let within: Vec<(String, String)> = self
            .gains
            .iter()
            .filter(|(_, held)| *held == value)
            .map(|(variable, _)| (variable.clone(), format!("[the sign-in `{name}`]")))
            .collect();
        (!within.is_empty()).then_some(within)
    }

    /// What the program said, with the value it was handed taken out. A
    /// program that repeats its key, in an answer or in the words it fails
    /// with, would otherwise have it written down with the turn.
    pub(crate) fn unsaid(&self, said: &str) -> String {
        match &self.secret {
            // Too short to be told from ordinary words is too short to
            // be a secret.
            Some((value, name)) if value.len() >= 6 && said.contains(value.as_str()) => {
                said.replace(value.as_str(), &format!("[the sign-in `{name}`]"))
            }
            _ => said.to_string(),
        }
    }
}

impl Drop for Drawn {
    fn drop(&mut self) {
        if !self.given_back {
            self.agents.let_go(&self.name, &self.turn);
        }
    }
}

impl Pool {
    /// Reads what the factory holds. Nothing is held by anyone when a
    /// daemon starts: a turn's program ends with the daemon that ran it.
    pub(crate) async fn open(journal: &journal::Journal) -> Result<Pool> {
        let kept = journal
            .db()
            .call(|connection| {
                connection.execute("UPDATE signins SET held_by = '[]' WHERE held_by != '[]'", [])?;
                let mut statement = connection.prepare(&format!("SELECT {SIGNIN} FROM signins"))?;
                let rows = statement.query_map([], read_signin)?;
                rows.collect::<rusqlite::Result<Vec<Kept>>>()
            })
            .await?;
        let kept = kept.into_iter().map(|kept| (kept.shown.name.clone(), kept)).collect();
        Ok(Pool { kept: Mutex::new(kept), changed: Notify::new() })
    }
}

/// Makes a file that only its owner can read from the moment it exists.
fn write_private(path: &std::path::Path, value: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
        }
    }
    // Never there with what it had and another's leave to read it.
    let _ = std::fs::remove_file(path);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)?.write_all(value)
}

impl Agents {
    fn value_path(&self, name: &str) -> PathBuf {
        self.inner.config.signins.join(name)
    }

    /// Keeps the value of the sign-in `name`: sealed with this
    /// installation's key, in a file only its owner can read.
    fn keep_value(&self, name: &str, value: &str) -> Result<()> {
        let path = self.value_path(name);
        let sealed = self
            .inner
            .config
            .seal
            .seal(name, value.as_bytes())
            .map_err(|err| Error::internal(err.message))?;
        write_private(&path, &sealed).map_err(|err| Error::from(err).context(path.display()))
    }

    /// The value kept for the sign-in `name`, if one is and it opens. One
    /// written as it is, by a build that did not seal, is read as it is
    /// and sealed there and then.
    fn kept_value(&self, name: &str) -> Option<String> {
        let kept = std::fs::read(self.value_path(name)).ok()?;
        let plain = match link::kept::Seal::is_sealed(&kept) {
            true => self.inner.config.seal.open(name, &kept).ok()?,
            false => {
                let value = String::from_utf8(kept.clone()).ok()?;
                let _ = self.keep_value(name, &value);
                kept
            }
        };
        String::from_utf8(plain).ok().map(|value| value.trim().to_string())
    }

    /// Adds a sign-in, or puts this one in place of the one of its name:
    /// when it was last used and whether it is set aside stay as they were.
    pub async fn signin_add(&self, new: NewSignIn, by: &Principal) -> Result<SignIn> {
        by.require("admin")?;
        let cli = brand::CLI;
        let name = new.name.trim().to_string();
        let taken = [SUBSCRIPTION, KEY].into_iter().chain(Kind::ALL.map(Kind::name)).any(|word| word == name);
        if name.is_empty() || name != blueprint::expr::slug(&name) || taken {
            return Err(Error::invalid(format!(
                "a sign-in is called by a short name of lowercase letters, digits and dashes, such as `work-1`, \
                 and not by a word a slot draws by (`{SUBSCRIPTION}`, `{KEY}`, `token`, `folder`); `{}` is not one",
                new.name
            )));
        }
        let Some(kind) = Kind::parse(new.kind.trim()) else {
            return Err(Error::invalid(format!(
                "a sign-in is a `key` (a key for the provider's API), a `token` (a subscription's long-lived \
                 token) or a `folder` (where the provider's program keeps its own sign-in); `{}` is none of them",
                new.kind
            )));
        };
        let package = new.package.trim().to_string();
        let extension = self.handing(&package).map_err(Error::invalid)?;
        let handing: &Handing = extension.signin.as_ref().expect("checked by `handing`");
        if !handing.takes(kind) {
            let takes: Vec<&str> = handing.kinds().into_iter().map(Kind::name).collect();
            return Err(Error::invalid(format!(
                "the extension package `{package}` does not say how its program is handed a {}; it can be \
                 handed: {}",
                kind.name(),
                takes.join(", ")
            )));
        }
        let at_once = new.at_once.unwrap_or(1);
        if at_once == 0 {
            return Err(Error::invalid("a sign-in that nobody may hold is no use; `at_once` is 1 or more"));
        }
        let (value, folder) = match kind {
            Kind::Folder => {
                let asked = new.folder.as_deref().unwrap_or_default().trim();
                let path = std::path::Path::new(asked);
                let settled =
                    kernel::whole::whole(path).ok().filter(|settled| path.is_absolute() && settled.is_dir());
                let Some(settled) = settled else {
                    return Err(Error::invalid(format!(
                        "a `folder` sign-in is the directory the provider's program keeps its own sign-in in, \
                         said by its whole path; `{asked}` is not a directory that is there"
                    )));
                };
                let settled = settled.to_string_lossy().into_owned();
                // One folder under two names would be handed to two
                // programs at once, each renewing it under the other.
                let twice = {
                    let kept = self.inner.pool.kept.lock().expect("signins lock");
                    kept.values()
                        .find(|kept| kept.shown.name != name && kept.shown.folder.as_ref() == Some(&settled))
                        .map(|kept| kept.shown.name.clone())
                };
                if let Some(other) = twice {
                    return Err(Error::conflict(format!(
                        "{settled} is already the sign-in `{other}`; one folder is one sign-in, so that it is \
                         held by as many programs as it says and no more"
                    )));
                }
                (None, Some(settled))
            }
            Kind::Key | Kind::Token => {
                let value = new.value.as_deref().unwrap_or_default().trim().to_string();
                if value.is_empty() || value.contains(['\n', '\r', '\0']) {
                    return Err(Error::invalid(format!(
                        "a {} is one line of text, given on standard input or in a file \
                         (`{cli} signin add {name} --for {package} --kind {} --from-file <file>`), never as an \
                         argument; nothing that reads as one was given",
                        kind.name(),
                        kind.name()
                    )));
                }
                (Some(value), None)
            }
        };
        // The value first: a row that speaks of a value that is not there
        // would be handed out and fail.
        if let Some(value) = &value {
            self.keep_value(&name, value)?;
        } else {
            let _ = std::fs::remove_file(self.value_path(&name));
        }
        let now = self.inner.journal.now_ms();
        let row = (name.clone(), package.clone(), kind.name(), at_once, folder.clone(), now);
        self.inner
            .journal
            .db()
            .call(move |connection| {
                connection.execute(
                    "INSERT INTO signins (name, package, kind, at_once, folder, created)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                     ON CONFLICT (name) DO UPDATE SET package = ?2, kind = ?3, at_once = ?4, folder = ?5",
                    params![row.0, row.1, row.2, row.3, row.4, row.5],
                )
            })
            .await?;
        let shown = {
            let mut kept = self.inner.pool.kept.lock().expect("signins lock");
            let entry = kept.entry(name.clone()).or_insert_with(|| Kept {
                kind,
                shown: SignIn {
                    name: name.clone(),
                    package: package.clone(),
                    kind: kind.name(),
                    at_once,
                    folder: None,
                    last_used: None,
                    held_by: Vec::new(),
                    aside_until: None,
                    aside_why: None,
                    created: now,
                },
            });
            entry.kind = kind;
            entry.shown.kind = kind.name();
            entry.shown.package = package.clone();
            entry.shown.at_once = at_once;
            entry.shown.folder = folder;
            entry.shown.clone()
        };
        self.inner.pool.changed.notify_waiters();
        let data = json!({ "signin": name, "for": package, "kind": kind.name(), "at_once": at_once });
        let _ = self.inner.journal.emit("agent.signin.added", Source::actor(by.actor()), data).await;
        // A folder that an agent in a kept channel may now be handed.
        self.settle_kept().await;
        Ok(shown)
    }

    /// The sign-ins the factory holds: who has which, and what is set
    /// aside. Never a value.
    pub fn signins(&self, by: &Principal) -> Result<Vec<SignIn>> {
        by.require("admin")?;
        let now = self.inner.journal.now_ms();
        let kept = self.inner.pool.kept.lock().expect("signins lock");
        Ok(kept
            .values()
            .map(|kept| {
                let aside = kept.aside(now);
                SignIn {
                    aside_until: aside,
                    aside_why: kept.shown.aside_why.clone().filter(|_| aside.is_some()),
                    ..kept.shown.clone()
                }
            })
            .collect())
    }

    /// Takes a sign-in away, and its value with it. A program that was
    /// handed it keeps what it was handed until its turn ends.
    pub async fn signin_remove(&self, name: &str, by: &Principal) -> Result<()> {
        by.require("admin")?;
        if self.inner.pool.kept.lock().expect("signins lock").remove(name).is_none() {
            return Err(Error::not_found(format!("there is no sign-in called `{name}`")));
        }
        let _ = std::fs::remove_file(self.value_path(name));
        let gone = name.to_string();
        self.inner
            .journal
            .db()
            .call(move |connection| connection.execute("DELETE FROM signins WHERE name = ?1", [&gone]))
            .await?;
        self.inner.pool.changed.notify_waiters();
        let data = json!({ "signin": name });
        let _ = self.inner.journal.emit("agent.signin.removed", Source::actor(by.actor()), data).await;
        Ok(())
    }

    /// Makes a sign-in that was set aside good again, before its time.
    pub async fn signin_restore(&self, name: &str, by: &Principal) -> Result<()> {
        by.require("admin")?;
        {
            let mut kept = self.inner.pool.kept.lock().expect("signins lock");
            let Some(kept) = kept.get_mut(name) else {
                return Err(Error::not_found(format!("there is no sign-in called `{name}`")));
            };
            kept.shown.aside_until = None;
            kept.shown.aside_why = None;
        }
        self.save(name).await;
        self.inner.pool.changed.notify_waiters();
        let data = json!({ "signin": name });
        let _ = self.inner.journal.emit("agent.signin.restored", Source::actor(by.actor()), data).await;
        Ok(())
    }

    /// How the program of `package` is handed a sign-in, or the sentence
    /// that says why it cannot be.
    fn handing(&self, package: &str) -> std::result::Result<std::sync::Arc<crate::Extension>, String> {
        let cli = brand::CLI;
        let Some(extension) = self.extension(package).filter(|extension| extension.agent) else {
            return Err(format!(
                "a sign-in is for the program of an extension package that runs agents, and no such package \
                 called `{package}` is installed here; `{cli} extension list` says which are"
            ));
        };
        if extension.signin.is_none() {
            return Err(format!(
                "the extension package `{package}` does not say how its program is handed a sign-in \
                 (`agent.signin` in its manifest), so it cannot be handed one: an agent on it signs in as \
                 whoever runs the daemon"
            ));
        }
        Ok(extension)
    }

    /// Checks what a slot on `adapter` is told to draw from, in order.
    /// What it says is kept as it was said, trimmed.
    pub(crate) fn check_draws(&self, adapter: &str, draws: &[String]) -> Result<Vec<String>> {
        let extension = self.handing(adapter).map_err(|why| {
            Error::invalid(format!(
                "{why}. Tell an agent on `{adapter}` to draw none: `signins: []`, or `--no-signin` on the \
                 command line"
            ))
        })?;
        let handing: &Handing = extension.signin.as_ref().expect("checked by `handing`");
        let takes: Vec<&str> = handing.kinds().into_iter().map(Kind::name).collect();
        let kept = self.inner.pool.kept.lock().expect("signins lock");
        let mut checked = Vec::new();
        for draw in draws {
            let draw = draw.trim();
            let fits = match draw {
                SUBSCRIPTION => handing.takes(Kind::Token) || handing.takes(Kind::Folder),
                KEY => handing.takes(Kind::Key),
                name if name.is_empty() || name != blueprint::expr::slug(name) => {
                    return Err(Error::invalid(format!(
                        "`{draw}` is not something to draw: say `{SUBSCRIPTION}` (any subscription of the \
                         agent's package), `{KEY}` (any key of it), or the name of one sign-in"
                    )));
                }
                // A name may be said before the sign-in is added.
                name => match kept.get(name) {
                    Some(kept) if kept.shown.package != adapter => {
                        return Err(Error::invalid(format!(
                            "the sign-in `{name}` is for `{}`, and this agent runs on `{adapter}`",
                            kept.shown.package
                        )));
                    }
                    Some(kept) => handing.takes(kept.kind),
                    None => true,
                },
            };
            if !fits {
                return Err(Error::invalid(format!(
                    "an agent on `{adapter}` cannot draw `{draw}`: its package does not say how its program \
                     is handed one; it can be handed: {}",
                    takes.join(", ")
                )));
            }
            if !checked.iter().any(|said| said == draw) {
                checked.push(draw.to_string());
            }
        }
        Ok(checked)
    }

    /// What a slot may draw as things stand at `now`: each sign-in of its
    /// package that one of its choices names, under the first that does.
    fn asked(&self, slot: &Slot, turn: &str, conversation: Option<&str>, now: i64) -> Asked {
        let kept = self.inner.pool.kept.lock().expect("signins lock");
        let mut signins = Vec::new();
        for kept in kept.values().filter(|kept| kept.shown.package == slot.adapter) {
            let choice = slot.signins.iter().position(|draw| match draw.as_str() {
                SUBSCRIPTION => kept.kind.is_subscription(),
                KEY => kept.kind == Kind::Key,
                name => name == kept.shown.name,
            });
            let Some(choice) = choice else { continue };
            signins.push(Offered {
                name: kept.shown.name.clone(),
                kind: kept.kind.name(),
                choice,
                at_once: kept.shown.at_once,
                held: kept.shown.held_by.len(),
                free: kept.free(now),
                last_used: kept.shown.last_used.unwrap_or(0),
                aside_until: kept.aside(now),
            });
        }
        Asked {
            slot: slot.name.clone(),
            package: slot.adapter.clone(),
            turn: turn.to_string(),
            conversation: conversation.map(str::to_string),
            now,
            signins,
        }
    }

    /// How many sign-ins a slot could be handed, held and set aside among
    /// them: how many times a turn of its can begin again at a limit.
    pub(crate) fn could_draw(&self, slot: &Slot) -> usize {
        self.asked(slot, "", None, self.inner.journal.now_ms()).signins.len()
    }

    /// Takes hold of `name` for a turn, if it is still free.
    fn hold(&self, name: &str, slot: &Slot, turn: &str, now: i64) -> Option<Drawn> {
        let mut kept = self.inner.pool.kept.lock().expect("signins lock");
        let kept = kept.get_mut(name).filter(|kept| kept.free(now))?;
        kept.shown.held_by.push(Holder { slot: slot.name.clone(), turn: turn.to_string(), since: now });
        kept.shown.last_used = Some(now);
        Some(Drawn {
            name: name.to_string(),
            kind: kept.kind,
            folder: kept.shown.folder.clone(),
            turn: turn.to_string(),
            slot: slot.name.clone(),
            agents: self.clone(),
            given_back: false,
        })
    }

    /// A sign-in for a turn of `slot` that is about to start: the one the
    /// rule chooses of those that are free, waiting for one to be given
    /// back or to be good again when the rule says to wait. The sentence
    /// says why there is none to be had.
    pub(crate) async fn draw(
        &self,
        slot: &Slot,
        turn: &str,
        conversation: Option<&str>,
        cancel: &Cancel,
    ) -> std::result::Result<Drawn, String> {
        let (journal, pool) = (&self.inner.journal, &self.inner.pool);
        let source = Source::actor(format!("agent:{}", slot.name));
        let began = journal.now_ms();
        let mut said_waiting = false;
        let drawn = loop {
            // Listening from before the look, so that one given back
            // between the look and the wait is not missed.
            let changed = pool.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            let now = journal.now_ms();
            let asked = self.asked(slot, turn, conversation, now);
            if asked.signins.is_empty() {
                break Err(format!(
                    "`{}` draws its sign-in from {}, and the factory holds none of those for `{}`. Add one \
                     (`{cli} signin add <name> --for {} --kind <key|token|folder>`), or say what else it may \
                     draw (`{cli} agent add {} --signin ...`)",
                    slot.name,
                    slot.signins.join(", then "),
                    slot.adapter,
                    slot.adapter,
                    slot.name,
                    cli = brand::CLI
                ));
            }
            match self.decide(slot, &asked) {
                Decision::Take(name) => match self.hold(&name, slot, turn, now) {
                    Some(drawn) => break Ok(drawn),
                    // It was free when the rule was asked and has been
                    // taken since: the rule is asked again.
                    None if asked.signins.iter().any(|offered| offered.name == name && offered.free) => {}
                    None => {
                        break Err(format!(
                            "the rule for choosing a sign-in chose `{name}` for `{}`, which is not one of \
                             the sign-ins it was told are free",
                            slot.name
                        ));
                    }
                },
                Decision::GiveUp(why) => break Err(why),
                Decision::Wait => {
                    // One timer, for the earliest moment a sign-in that is
                    // set aside is good again; a sign-in given back is told.
                    let until = asked.signins.iter().filter_map(|offered| offered.aside_until).min();
                    if !said_waiting {
                        said_waiting = true;
                        let data = json!({ "slot": slot.name, "turn": turn, "until": until });
                        let _ = journal.emit("agent.signin.waiting", source.clone(), data).await;
                    }
                    let good_again = async {
                        match until {
                            Some(until) => {
                                journal
                                    .clock()
                                    .sleep(Duration::from_millis((until - now).max(1) as u64))
                                    .await;
                            }
                            None => std::future::pending().await,
                        }
                    };
                    tokio::select! {
                        () = &mut changed => {}
                        () = good_again => {}
                        () = cancel.cancelled() => break Err("the turn was cancelled".to_string()),
                    }
                }
            }
        };
        let labels = [("slot", slot.name.as_str()), ("adapter", slot.adapter.as_str())];
        journal.measure("agent.signin.wait", &labels, (journal.now_ms() - began) as f64, drawn.is_ok());
        let drawn = drawn?;
        self.save(&drawn.name).await;
        let data =
            json!({ "slot": slot.name, "turn": turn, "signin": drawn.name, "kind": drawn.kind.name() });
        let _ = journal.emit("agent.signin.held", source, data).await;
        Ok(drawn)
    }

    /// Whether a sign-in `slot` may be handed, as things stand, is a
    /// folder: where its program then keeps everything, whatever it is
    /// asked and wherever.
    pub(crate) fn may_draw_a_folder(&self, slot: &Slot) -> bool {
        let offered = self.asked(slot, "", None, self.inner.journal.now_ms()).signins;
        offered.iter().any(|offered| offered.kind == Kind::Folder.name())
    }

    /// What the program's environment gains and loses for a turn that
    /// holds `drawn`, as the slot's package says. `home` is the folder
    /// that is the slot's own for this turn (`crate::kept`).
    pub(crate) fn hand(
        &self,
        drawn: &Drawn,
        slot: &Slot,
        home: std::path::PathBuf,
    ) -> std::result::Result<Handed, String> {
        let extension = self.handing(&slot.adapter)?;
        let handing = extension.signin.as_ref().expect("checked by `handing`");
        let held = match (drawn.kind, &drawn.folder) {
            (Kind::Folder, Some(folder)) => folder.clone(),
            _ => self.kept_value(&drawn.name).ok_or_else(|| {
                format!(
                    "the value of the sign-in `{name}` is not where the factory keeps it; add it again \
                         (`{cli} signin add {name} --for {package} --kind {kind}`)",
                    name = drawn.name,
                    cli = brand::CLI,
                    package = slot.adapter,
                    kind = drawn.kind.name()
                )
            })?,
        };
        std::fs::create_dir_all(&home).map_err(|err| format!("{}: {err}", home.display()))?;
        let secret = (drawn.kind != Kind::Folder).then(|| (held.clone(), drawn.name.clone()));
        let mut folders = vec![home.clone()];
        if drawn.kind == Kind::Folder {
            folders.push(std::path::PathBuf::from(&held));
        }
        let (gains, loses) = handing.dress(drawn.kind, held, &home);
        Ok(Handed { gains, loses, secret, folders })
    }

    /// Whether the package of `slot` says a turn that failed had reached
    /// its sign-in's limit.
    pub(crate) fn limited(&self, slot: &Slot, failure: &crate::adapter::Failure) -> Option<Limit> {
        let extension = self.extension(&slot.adapter)?;
        extension.signin.as_ref()?.limited(&crate::handed::Said {
            error: &failure.message,
            reply: &failure.reply,
            code: failure.code,
            stderr: &failure.stderr,
            now_ms: self.inner.journal.now_ms(),
        })
    }

    /// Lets go of a sign-in in what is held here, and tells whoever waits.
    fn let_go(&self, name: &str, turn: &str) {
        if let Some(kept) = self.inner.pool.kept.lock().expect("signins lock").get_mut(name) {
            kept.shown.held_by.retain(|holder| holder.turn != turn);
        }
        self.inner.pool.changed.notify_waiters();
    }

    /// Gives a sign-in back at the end of its turn.
    pub(crate) async fn give_back(&self, mut drawn: Drawn) {
        drawn.given_back = true;
        self.let_go(&drawn.name, &drawn.turn);
        self.save(&drawn.name).await;
        let data = json!({ "slot": drawn.slot, "turn": drawn.turn, "signin": drawn.name });
        let source = Source::actor(format!("agent:{}", drawn.slot));
        let _ = self.inner.journal.emit("agent.signin.released", source, data).await;
    }

    /// Sets a sign-in aside until it is good again, and says so by its
    /// name. The turn that met the limit still holds it, and gives it back.
    pub(crate) async fn set_aside(&self, drawn: &Drawn, limit: Limit, why: &str) {
        {
            let mut kept = self.inner.pool.kept.lock().expect("signins lock");
            let Some(kept) = kept.get_mut(&drawn.name) else { return };
            kept.shown.aside_until = Some(kept.shown.aside_until.unwrap_or(0).max(limit.until_ms));
            kept.shown.aside_why = Some(why.to_string());
        }
        self.save(&drawn.name).await;
        self.inner.pool.changed.notify_waiters();
        let data = json!({
            "signin": drawn.name, "until": limit.until_ms, "slot": drawn.slot, "turn": drawn.turn, "why": why,
        });
        let source = Source::actor(format!("agent:{}", drawn.slot));
        let _ = self.inner.journal.emit("agent.signin.set_aside", source, data).await;
    }

    /// Writes down what is held here of one sign-in.
    async fn save(&self, name: &str) {
        let row = {
            let kept = self.inner.pool.kept.lock().expect("signins lock");
            let Some(kept) = kept.get(name) else { return };
            (
                name.to_string(),
                kept.shown.last_used,
                serde_json::to_string(&kept.shown.held_by).unwrap_or_else(|_| "[]".into()),
                kept.shown.aside_until,
                kept.shown.aside_why.clone(),
            )
        };
        let _ = self
            .inner
            .journal
            .db()
            .call(move |connection| {
                connection.execute(
                    "UPDATE signins SET last_used = ?2, held_by = ?3, aside_until = ?4, aside_why = ?5
                     WHERE name = ?1",
                    params![row.0, row.1, row.2, row.3, row.4],
                )
            })
            .await;
    }
}
