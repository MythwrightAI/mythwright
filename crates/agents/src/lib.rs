//! Agent slots.
//!
//! A slot is the place an agent lives in the fleet: its name, the project it
//! works in, the channels it listens to, what it may do, and which adapter
//! runs it. The slot is long-lived; the agent's sessions are not. Each piece
//! of work is a *turn*: a prompt goes in, a reply comes out, and whatever the
//! turn started is stopped when it ends.
//!
//! Turns are queued per slot in the database and run one at a time, so a slot
//! never has two agents editing the same files, a turn that was in progress
//! when the conductor stopped is run again when it returns, and nothing about
//! the queue depends on the agent itself behaving.
//!
//! An agent acts on the fleet the way a person at a terminal does: with the
//! command-line client, signed in as its slot. Its credential is written to a
//! file only it is pointed at and carries only the slot's scopes.
//!
//! Its program signs in to its model's provider as whoever runs the daemon,
//! unless its slot says what it draws its sign-in from: then each turn is
//! handed one the factory holds, for the length of that turn ([`signins`]).

mod actions;
pub mod allowed;
pub mod apart;
mod asked;
pub mod assigned;
pub mod brought;
mod builtin;
/// What the package that came differs by from an installed copy, which
/// is `kernel::came`: a function of two manifests, with no agent in it.
pub use kernel::came;
pub mod copy;
mod council;
mod credential;
mod discussing;
pub mod dispatch;
pub mod docs;
mod door;
pub mod drained;
mod ensured;
pub mod events;
pub mod external;
mod filed;
pub mod forgotten;
mod heard;
mod installed;
mod kept;
pub mod keys;
mod letgo;
pub mod moved;
mod moving;
mod own;
pub mod performed;
mod picking;
pub mod present;
pub mod real;
pub mod removed;
pub mod repository;
pub mod rule;
mod schema;
pub mod shown;
pub mod signins;
mod together;
mod told;
mod topical;
pub mod tried;
mod turns;
pub mod walled;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use conductor::{Engine, InstanceFilter, InstanceInfo, Registry};
use journal::{Journal, optional};
use kernel::{Cancel, Error, Principal, PrincipalKind, Result, Source};
use serde::Serialize;
use serde_json::{Value, json};
use studio::Studio;
use tokio::sync::{mpsc, oneshot};
use tokio::task::AbortHandle;

// What a fleet's blueprints ask that uses no agent (a function of plain
// values with its table of cases, and the action that asks it) is in
// `deciding`, the crate under this one, each in a file called what it is
// called here (`agents::work` is `deciding::work`), with its cases in that
// crate's suite. They are offered here under the names they always had.
pub use deciding::{
    begun, column, counted, decided, reviews, rounds, segments, spent, tally, turn, unkept, work,
};

pub use deciding::picks::Order;
pub use external::{Invitation, Invite, Joined, Standing};
pub use filed::{TURN_LOOKED_UP, TURN_WAITED};
pub use installed::Taken;
pub use picking::Candidate;
pub use removed::{Brought, Removed};
pub use signins::{NewSignIn, SignIn};
pub use turns::{TurnInfo, TurnRequest};

// What a package says, and how a turn of its program is run, is in
// `extensions`, the crate under this one, which asks nothing of the
// agents. It is offered here under the names it always had; what of it
// needs the agents (`walled`, `shown`, `brought`, `performed`) is here
// beside what it offers.
pub use extensions::{
    Adapter, EXTERNAL, Extension, Failure, MANIFEST, Model, PERMISSIONS, Reply, Turn, adapter, declared,
    extension, handed, program, running,
};

/// Where the daemon keeps what each package's program said it offers.
const MODELS_KEPT: &str = "agent.models";
/// Where it keeps the model each agent's program last said it was using.
const MODEL_REPORTED: &str = "agent.model";

fn every_mode() -> Vec<String> {
    PERMISSIONS.iter().map(|(name, _)| name.to_string()).collect()
}

/// Where things are for the agents on this device.
#[derive(Clone, Debug)]
pub struct Config {
    /// The daemon's address, as an agent's command-line client should use it.
    pub api_url: String,
    /// Each slot gets a directory here for its credential.
    pub home: PathBuf,
    /// A directory holding the command-line client, put first on an agent's PATH.
    pub bin: PathBuf,
    /// The adapter a slot gets when it does not name one. Left unset, that is
    /// the first installed package whose program is here, else the built-in
    /// assistant.
    pub default_adapter: Option<String>,
    /// Where extension packages are installed, a folder each.
    pub extensions: PathBuf,
    /// A folder of packages that came with the product and can be installed
    /// from it by name. None of them is in use until someone installs it.
    pub offered: Option<PathBuf>,
    /// Where the values of the sign-ins the factory holds are kept, a file
    /// each that only its owner can read.
    pub signins: PathBuf,
    /// The key of this installation each of those is sealed with
    /// ([`link::kept`]).
    pub seal: link::kept::Seal,
    /// The factory's own data folder, which nothing behind a wall is
    /// shown whole.
    pub data: PathBuf,
    /// The home of whoever runs the daemon, where it is not the one their
    /// environment says: a test's. A program behind a wall is shown a home
    /// of its own in its place.
    pub user_home: Option<PathBuf>,
}

/// Something that can run an agent, as it is offered to whoever is choosing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Provider {
    pub name: String,
    pub title: String,
    pub description: String,
    /// `extension`, `command` or `builtin`.
    pub kind: &'static str,
    /// Whether its program is on this device.
    pub available: bool,
    /// How its models are known: `asked` of the program, `listed` by the
    /// package, or `none`, in which case one is typed.
    pub models: &'static str,
    /// The permission modes an agent on it can be held to.
    pub permissions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Slot {
    pub name: String,
    pub adapter: String,
    pub model: Option<String>,
    pub project: Option<String>,
    /// Standing instructions given to the agent on every turn.
    pub instructions: String,
    pub scopes: Vec<String>,
    /// How the agent CLI treats its own permission prompts.
    pub permission_mode: String,
    /// What it draws its sign-in from, in order: `subscription`, `key`, or
    /// the name of one. Empty for an agent whose program signs in as
    /// whoever runs the daemon.
    pub signins: Vec<String>,
    /// The role it was brought up in, if any: a word of its fleet's own,
    /// by which slots that do the same kind of work are told from the rest
    /// and work is given to whichever of them is free. Nothing here knows
    /// what any role is for; what a slot does is its instructions and the
    /// blueprints that give it work.
    pub role: String,
    /// How it is ranked among those of its role: a number of its fleet's
    /// own, read only where whoever gives work says to choose by it.
    pub rank: i64,
    /// Whether its program is started behind a wall ([`walled`]): shown
    /// its work, a home of its own and what is added here, and nothing
    /// else of the device.
    pub wall: bool,
    /// What else it may read there, and what else it may change: folders
    /// or files of the device, `~` being the home of whoever runs the daemon.
    pub reads: Vec<String>,
    pub changes: Vec<String>,
    /// What its program may do with its project's repository behind its
    /// wall: `off`, `read`, `commit` or `write` ([`walled::Git`]).
    pub git: String,
    /// The keys its program is handed: each variable of its environment
    /// with the name of the secret the factory keeps for it ([`keys`]).
    pub secrets: BTreeMap<String, String>,
    /// Whether its program goes on without the daemon ([`running`]): it is
    /// started so that it does not end with the daemon, and the next
    /// daemon takes its turn up where it is.
    pub outlives: bool,
    /// Whether its program may use the web unasked: look things up and
    /// fetch pages with what it has of its own for that. What that is in
    /// a particular program is its package's to say (`turn.web`).
    pub web: bool,
    pub active: bool,
    /// It is given no more work, and is stopped when what it has in hand
    /// is finished.
    pub leaving: bool,
    pub created: i64,
    pub updated: i64,
    /// Which turn of all was the last its name had before it was made,
    /// where an agent of that name was taken away before it
    /// ([`removed`]): the turns numbered above that are its own, and
    /// nothing of the others is. Nought where nobody had the name.
    #[serde(skip)]
    pub(crate) since: i64,
    /// The environment of its project's it works under, empty for none:
    /// its program is handed what of it is marked for agents, and the
    /// keys its slot names are looked for there first ([`keys`]).
    pub environment: String,
}

impl Slot {
    /// The principal this slot's agent acts as.
    pub fn principal(&self) -> Principal {
        Principal {
            kind: PrincipalKind::Agent,
            id: self.name.clone(),
            name: self.name.clone(),
            scopes: self.scopes.clone(),
            key: None,
        }
    }
}

/// What to make a slot be. Fields left out keep what the slot already has.
#[derive(Clone, Debug, Default)]
pub struct SlotSpec {
    pub name: String,
    pub adapter: Option<String>,
    pub model: Option<String>,
    /// The project it works in; said empty, none (what a text applied
    /// for no project says of the agent it brings up).
    pub project: Option<String>,
    pub channel: Option<String>,
    pub instructions: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub permission_mode: Option<String>,
    /// What it draws its sign-in from, in order; an empty list for none.
    pub signins: Option<Vec<String>>,
    /// The role it is brought up in; empty for none.
    pub role: Option<String>,
    /// How it is ranked among those of its role; unsaid, it is as it was.
    pub rank: Option<i64>,
    /// Whether its program works behind a wall; unsaid, it is as it was,
    /// and a new one has a wall.
    pub wall: Option<bool>,
    /// What else it is shown behind its wall, to read and to change.
    pub reads: Option<Vec<String>>,
    pub changes: Option<Vec<String>>,
    /// What it may do with its project's repository there; unsaid, it is
    /// as it was, and a new one may read it.
    pub git: Option<String>,
    /// The keys it is handed, each written `VARIABLE=name`; an empty list
    /// for none, and unsaid, as it was.
    pub secrets: Option<Vec<String>>,
    /// Whether its program goes on without the daemon; unsaid, it is as it
    /// was, and a new one's does not.
    pub outlives: Option<bool>,
    /// Whether its program may use the web unasked; unsaid, it is as it
    /// was, and a new one's may not.
    pub web: Option<bool>,
    /// The environment of its project's it works under; unsaid, it is as
    /// it was, and a new one works under none. The empty text is none.
    pub environment: Option<String>,
}

#[derive(Clone)]
pub struct Agents {
    inner: Arc<Inner>,
}

struct Inner {
    journal: Journal,
    studio: Studio,
    /// What everything started here is told about where it runs.
    hosting: hosting::Hosting,
    config: Config,
    /// The sign-ins the factory holds, and who has which.
    pool: signins::Pool,
    /// Where the rule for choosing one is read from, once the conductor
    /// that has the blueprints is known. Not kept alive from here: what
    /// the installed packages bring is in it, and that is this.
    library: OnceLock<std::sync::Weak<dyn conductor::Library>>,
    adapters: Mutex<HashMap<String, Arc<dyn Adapter>>>,
    extensions: Mutex<BTreeMap<String, Arc<Extension>>>,
    /// The names the application's own actions are under (`board` for
    /// `board.add`), once the conductor that has them is known. A package
    /// that brings actions cannot be called one of them.
    taken: OnceLock<BTreeSet<String>>,
    /// What each package's program said it offers, the last time it was asked.
    models: Mutex<HashMap<String, Vec<Model>>>,
    /// The model each agent's program last said it was using, by slot.
    reported: Mutex<HashMap<String, String>>,
    /// One worker per slot with work to do; sending wakes it.
    workers: Mutex<HashMap<String, mpsc::UnboundedSender<()>>>,
    waiters: Mutex<HashMap<String, Vec<oneshot::Sender<()>>>>,
    /// Turns in progress, so a caller that gives up can stop one.
    running: Mutex<HashMap<String, Cancel>>,
    /// The turns an earlier daemon left in hand, as found when this one
    /// began, and whether anybody has asked for each since: whoever asks
    /// the same of the same agent again is given that turn, once, and not
    /// a second asking ([`Agents::claim`]). With them those it left
    /// waiting that a step asked for in a word of its own, which only
    /// that word finds again; and either may be let go of
    /// ([`Agents::let_go_under`]).
    left_in_hand: Mutex<HashMap<String, asked::Left>>,
    /// The daemon is draining to stop (`crate::drained`): a turn nobody's
    /// run asked for is not begun.
    draining: std::sync::atomic::AtomicBool,
    /// The turns in hand that are of a channel kept to its members.
    hand: Arc<kept::Hand>,
    /// The agents whose programs go on without the daemon, as their
    /// slots say now (`running::Outliving`).
    outliving: Arc<running::Outliving>,
    /// The turns in hand whose programs are on pipes (`crate::running`),
    /// which end with the daemon whatever their slots say.
    piped: Mutex<std::collections::HashSet<String>>,
    /// How many turns each agent was given, and its last (`picking`).
    given: Mutex<filed::Given>,
    /// Where turns are kept, and the count their numbers are given from.
    filed: filed::Filed,
    /// Held from choosing who work goes to until it is in their queue, so
    /// that two choices made together do not both find the same agent free.
    choosing: tokio::sync::Mutex<()>,
    /// What is being asked of git for places, and whether more may be:
    /// the actions of `places`, which are added with these.
    places: places::Places,
    /// A project's environments as the runner holds them, once the
    /// conductor is known: what an agent under one is handed of it.
    environs: OnceLock<Arc<environments::held::Held>>,
    tasks: Mutex<Vec<AbortHandle>>,
}

fn read_slot(row: &rusqlite::Row<'_>) -> rusqlite::Result<Slot> {
    Ok(Slot {
        name: row.get(0)?,
        adapter: row.get(1)?,
        model: row.get(2)?,
        project: row.get(3)?,
        instructions: row.get(4)?,
        scopes: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
        permission_mode: row.get(6)?,
        active: row.get::<_, i64>(7)? != 0,
        created: row.get(8)?,
        updated: row.get(9)?,
        signins: serde_json::from_str(&row.get::<_, String>(10)?).unwrap_or_default(),
        role: row.get(11)?,
        leaving: row.get::<_, i64>(12)? != 0,
        rank: row.get(13)?,
        wall: row.get::<_, i64>(14)? != 0,
        reads: serde_json::from_str(&row.get::<_, String>(15)?).unwrap_or_default(),
        changes: serde_json::from_str(&row.get::<_, String>(16)?).unwrap_or_default(),
        git: row.get(17)?,
        secrets: serde_json::from_str(&row.get::<_, String>(18)?).unwrap_or_default(),
        outlives: row.get::<_, i64>(19)? != 0,
        web: row.get::<_, i64>(20)? != 0,
        since: row.get(21)?,
        environment: row.get(22)?,
    })
}

const SLOT: &str = "name, adapter, model, project, instructions, scopes, permission_mode, active, created, updated, \
                    signins, role, leaving, rank, wall, reads, changes, git, secrets, outlives, web, since, environment";

impl Agents {
    pub async fn open(
        journal: Journal,
        studio: Studio,
        hosting: hosting::Hosting,
        config: Config,
    ) -> Result<Agents> {
        apart::make(journal.db()).await?;
        let pool = signins::Pool::open(&journal).await?;
        std::fs::create_dir_all(&config.home)?;
        std::fs::create_dir_all(&config.extensions)?;
        // A project's journal that keeps its agents' turns has the same
        // tables. Then, before anything asks for work again: what was
        // in hand, what each was given, and where turns are kept.
        apart::makes(&journal).await?;
        let (left, waiting, given, filed) = filed::read(&journal).await?;
        // Of those left in hand, the ones of a kept channel are what an
        // agent's key opens that channel for, from here: asked by the
        // channels each time an agent reads one (`kept`).
        let hand = Arc::new(kept::Hand::default());
        for (turn, slot, kept) in &left {
            if let Some(kept) = kept {
                hand.takes(turn, slot, kept);
            }
        }
        let asked = hand.clone();
        studio.channels.asked_with(Arc::new(move |channel, actor| asked.has(channel, actor)));
        assigned::install(&journal, &studio);
        // And which agents are started by nobody here, each of a kept
        // channel it is in as a person is: asked the same way.
        let outside = journal.db().call(|connection| {
            let mut statement = connection.prepare("SELECT name FROM slots WHERE adapter = ?1")?;
            let names = statement.query_map([external::EXTERNAL], |row| row.get::<_, String>(0))?;
            names.collect::<rusqlite::Result<Vec<String>>>()
        });
        for name in outside.await? {
            hand.is_outside(&name, true);
        }
        let asked = hand.clone();
        studio.channels.outside_with(Arc::new(move |actor| asked.outside(actor)));
        // And whose programs go on without the daemon, asked as it stops.
        let outliving = Arc::new(running::Outliving::default());
        let outlive = journal.db().call(|connection| {
            let mut statement = connection.prepare("SELECT name FROM slots WHERE outlives = 1")?;
            let names = statement.query_map([], |row| row.get::<_, String>(0))?;
            names.collect::<rusqlite::Result<Vec<String>>>()
        });
        for name in outlive.await? {
            outliving.is(&name, true);
        }
        let in_hand = left.into_iter().map(|(turn, ..)| (turn, asked::Left::Open));
        let left_in_hand =
            in_hand.chain(waiting.into_iter().map(|turn| (turn, asked::Left::Waiting))).collect();
        let (extensions, problems) = extension::load_all(&config.extensions);
        for problem in problems {
            let data = json!({ "problem": problem });
            let _ = journal.emit("extension.broken", Source::actor("system"), data).await;
        }
        let agents = Agents {
            inner: Arc::new(Inner {
                journal,
                studio: studio.clone(),
                hosting: hosting.clone(),
                config,
                pool,
                library: OnceLock::new(),
                environs: OnceLock::new(),
                adapters: Mutex::default(),
                extensions: Mutex::new(extensions.clone()),
                taken: OnceLock::new(),
                models: Mutex::default(),
                reported: Mutex::default(),
                workers: Mutex::default(),
                waiters: Mutex::default(),
                running: Mutex::default(),
                left_in_hand: Mutex::new(left_in_hand),
                draining: Default::default(),
                hand,
                outliving,
                piped: Mutex::default(),
                given: Mutex::new(given),
                filed,
                choosing: tokio::sync::Mutex::default(),
                places: places::Places::new(hosting.clone()),
                tasks: Mutex::default(),
            }),
        };
        agents.add_adapter("builtin", Arc::new(builtin::Builtin::new(studio)));
        // A package that brings only blueprints runs no agent.
        for (name, extension) in extensions.into_iter().filter(|(_, extension)| extension.agent) {
            // What its program said it offers the last time it was asked is
            // still the best there is to go on, until it is asked again.
            if let Ok(Some(known)) = agents.inner.journal.kv_get(MODELS_KEPT, &name).await
                && let Ok(known) = serde_json::from_value::<Vec<Model>>(known)
            {
                agents.inner.models.lock().expect("agents lock").insert(name.clone(), known);
            }
            agents.add_adapter(&name, Arc::new(extension::Packaged(extension)));
        }
        for slot in agents.slots().await.unwrap_or_default() {
            if let Ok(Some(Value::String(model))) =
                agents.inner.journal.kv_get(MODEL_REPORTED, &slot.name).await
            {
                agents.inner.reported.lock().expect("agents lock").insert(slot.name, model);
            }
        }
        Ok(agents)
    }

    /// Makes an adapter available to slots under `name`.
    ///
    /// Never under the word for an agent this factory does not start
    /// ([`EXTERNAL`]): nothing runs one, and whatever makes an adapter
    /// says so before it comes here (a package's name is refused as it
    /// is read, a program set up on this device as the daemon starts).
    pub fn add_adapter(&self, name: &str, adapter: Arc<dyn Adapter>) {
        if name == EXTERNAL {
            return;
        }
        self.inner.adapters.lock().expect("agents lock").insert(name.to_string(), adapter);
    }

    pub fn adapters(&self) -> Vec<String> {
        let mut names: Vec<String> =
            self.inner.adapters.lock().expect("agents lock").keys().cloned().collect();
        names.sort();
        names
    }

    pub fn default_adapter(&self) -> String {
        if let Some(configured) = &self.inner.config.default_adapter {
            return configured.clone();
        }
        let extensions = self.inner.extensions.lock().expect("agents lock");
        let usable = extensions.values().find(|extension| extension.agent && extension.available());
        usable.map_or_else(|| "builtin".to_string(), |extension| extension.name.clone())
    }

    /// Everything that can run an agent on this device.
    pub fn providers(&self) -> Vec<Provider> {
        let extensions = self.inner.extensions.lock().expect("agents lock").clone();
        let mut providers: Vec<Provider> = self
            .adapters()
            .into_iter()
            .map(|name| match extensions.get(&name) {
                Some(extension) => Provider {
                    title: if extension.title.is_empty() { name.clone() } else { extension.title.clone() },
                    description: extension.description.clone(),
                    kind: "extension",
                    available: extension.available(),
                    models: extension.models_kind(),
                    permissions: extension.permissions(),
                    name,
                },
                None if name == "builtin" => Provider {
                    title: "Built-in assistant".into(),
                    description: "Needs no model and no network. It understands a few requests; it is for trying things out.".into(),
                    kind: "builtin",
                    available: true,
                    models: "none",
                    permissions: every_mode(),
                    name,
                },
                None => Provider {
                    title: name.clone(),
                    description: "A program configured on this device.".into(),
                    kind: "command",
                    available: true,
                    models: "none",
                    permissions: every_mode(),
                    name,
                },
            })
            .collect();
        providers.sort_by(|a, b| (a.kind == "builtin", &a.name).cmp(&(b.kind == "builtin", &b.name)));
        providers
    }

    /// The installed packages.
    pub fn extensions(&self) -> Vec<Arc<Extension>> {
        self.inner.extensions.lock().expect("agents lock").values().cloned().collect()
    }

    /// The installed package of this name.
    pub fn extension(&self, name: &str) -> Option<Arc<Extension>> {
        self.inner.extensions.lock().expect("agents lock").get(name).cloned()
    }

    /// The blueprints the installed packages bring, as a layer of the
    /// library: there while a package is installed, gone when it is removed.
    pub fn library(&self) -> Arc<dyn conductor::Library> {
        Arc::new(brought::Brought::new(self.clone()))
    }

    /// The actions the installed packages declare, for the conductor to ask
    /// for each time one is done: there while a package is installed, gone
    /// when it is removed. `hosting` says what a program started here is
    /// told about where it runs.
    pub fn actions(&self, hosting: &hosting::Hosting) -> Arc<dyn conductor::Brought> {
        Arc::new(performed::Offered::new(self.clone(), hosting.clone()))
    }

    /// Whether the application's own actions are under this name.
    pub(crate) fn is_taken(&self, name: &str) -> bool {
        self.inner.taken.get().is_some_and(|taken| taken.contains(name))
    }

    /// Whether asking for `provider`'s models now would run no program:
    /// it has been asked, or it is nothing that is asked.
    pub fn knows_models(&self, provider: &str) -> bool {
        !self.inner.extensions.lock().expect("agents lock").contains_key(provider)
            || self.inner.models.lock().expect("agents lock").contains_key(provider)
    }

    /// The models something that runs agents offers. A package that asks its
    /// program is asked once and remembered; `fresh` asks again.
    pub async fn models(&self, provider: &str, fresh: bool) -> Result<Vec<Model>> {
        let extension = self.inner.extensions.lock().expect("agents lock").get(provider).cloned();
        let Some(extension) = extension else {
            return if self.inner.adapters.lock().expect("agents lock").contains_key(provider) {
                Ok(Vec::new())
            } else {
                Err(Error::not_found(format!("nothing called `{provider}` runs agents here")))
            };
        };
        if !fresh && let Some(known) = self.inner.models.lock().expect("agents lock").get(provider) {
            return Ok(known.clone());
        }
        let started = std::time::Instant::now();
        let asked = extension.ask_models(Some(&self.inner.hosting)).await;
        let labels = [("extension", provider)];
        self.inner.journal.measure(
            "agent.models",
            &labels,
            started.elapsed().as_secs_f64() * 1000.0,
            asked.is_ok(),
        );
        let models = asked.map_err(Error::unavailable)?;
        self.inner.models.lock().expect("agents lock").insert(provider.to_string(), models.clone());
        let _ = self.inner.journal.kv_set(MODELS_KEPT, provider, &json!(models)).await;
        Ok(models)
    }

    /// What to call the model an agent runs on, as far as is known without
    /// asking anything: the one chosen for it, by the name its provider
    /// gives it, or else the one its provider uses when none is chosen.
    /// `None` when its provider has not said, or has only said "the usual".
    pub fn model_name(&self, slot: &Slot) -> Option<String> {
        let known = self.inner.models.lock().expect("agents lock").get(&slot.adapter).cloned();
        match &slot.model {
            Some(chosen) => Some(
                known
                    .into_iter()
                    .flatten()
                    .find(|model| &model.id == chosen)
                    .map(|model| model.label)
                    .filter(|label| !label.is_empty())
                    .unwrap_or_else(|| chosen.clone()),
            ),
            // Left to its provider's choice: what its program last said it
            // was using, which is the fact of the matter; failing that, the
            // one its provider says it uses. A provider's "default" is a
            // choice, not the name of a model.
            None => self.inner.reported.lock().expect("agents lock").get(&slot.name).cloned().or_else(|| {
                known
                    .into_iter()
                    .flatten()
                    .find(|model| model.default && model.id != "default")
                    .map(|model| if model.label.is_empty() { model.id } else { model.label })
            }),
        }
    }

    /// Notes which model an agent's program said it used for a turn.
    pub(crate) async fn note_model(&self, slot: &str, model: &str) {
        let known =
            self.inner.reported.lock().expect("agents lock").insert(slot.to_string(), model.to_string());
        if known.as_deref() != Some(model) {
            let _ = self.inner.journal.kv_set(MODEL_REPORTED, slot, &json!(model)).await;
            // Whoever is showing the agent now has something better to show.
            let data = self.of_slot(slot, json!({ "slot": slot, "model": model })).await;
            let _ = self.inner.journal.emit("agent.slot.updated", Source::actor("system"), data).await;
        }
    }

    /// What is said of a slot, with the project the slot is in where it
    /// is in one: an event of an agent's names no instance, so this is
    /// what says whose it is to whatever hears events by project
    /// (`conductor::whose`). Without it an agent's turn in one project
    /// was nobody's, and so everybody's to hear.
    pub(crate) async fn of_slot(&self, slot: &str, mut data: Value) -> Value {
        if let Ok(Some(Slot { project: Some(project), .. })) = self.find(slot).await {
            data["project"] = json!(project);
        }
        data
    }

    pub async fn find(&self, name: &str) -> Result<Option<Slot>> {
        let name = name.to_string();
        self.inner
            .journal
            .db()
            .call(move |c| {
                optional(c.query_row(
                    &format!("SELECT {SLOT} FROM slots WHERE name = ?1"),
                    [&name],
                    read_slot,
                ))
            })
            .await
    }

    pub async fn slot(&self, name: &str) -> Result<Slot> {
        self.find(name).await?.ok_or_else(|| Error::not_found(format!("there is no agent called `{name}`")))
    }

    pub async fn slots(&self) -> Result<Vec<Slot>> {
        self.inner
            .journal
            .db()
            .call(|connection| {
                let mut statement =
                    connection.prepare(&format!("SELECT {SLOT} FROM slots ORDER BY created, name"))?;
                let rows = statement.query_map([], read_slot)?;
                rows.collect()
            })
            .await
    }

    /// The environment an agent's process is given: where the daemon is, who
    /// it is, and the command-line client on its PATH. The credential itself
    /// is in a file, never in the environment or on a command line.
    pub(crate) fn environment(&self, slot: &Slot) -> BTreeMap<String, String> {
        let config = &self.inner.config;
        let mut env = BTreeMap::new();
        env.insert(brand::env_var("URL"), config.api_url.clone());
        env.insert(brand::env_var("TOKEN_FILE"), self.token_path(&slot.name).to_string_lossy().into_owned());
        env.insert(brand::env_var("SLOT"), slot.name.clone());
        if let Some(project) = &slot.project {
            env.insert(brand::env_var("PROJECT"), project.clone());
        }
        let path = std::env::var("PATH").unwrap_or_default();
        let separator = if cfg!(windows) { ';' } else { ':' };
        env.insert("PATH".to_string(), format!("{}{separator}{path}", config.bin.display()));
        env
    }

    /// Where the command-line client is for an agent's program: under its
    /// name in the folder that is first on the program's `PATH`
    /// ([`Agents::environment`]), which is shown where it is behind a wall.
    pub(crate) fn client(&self) -> PathBuf {
        let link = self.inner.config.bin.join(brand::CLI);
        if cfg!(windows) { link.with_extension("exe") } else { link }
    }

    /// Adds this crate's actions to `registry`, and those of `deciding`
    /// each where it always was among them: what is written out of the
    /// language for editors lists the actions in the order they are added.
    pub fn register(&self, registry: &mut Registry) {
        actions::register(registry, self);
        letgo::register(registry, self);
        places::register(registry, &self.inner.places);
        deciding::register(registry, |registry| present::register(registry, self));
        deciding::timed::register_reading(registry, &self.inner.studio);
        registry.emits(events::events());
    }

    /// Ties slots to the instances that hold them: when a fleet stops, the
    /// agents it brought in stop taking work.
    pub fn attach(&self, engine: &Engine) {
        // What this device's own actions are called is settled by now.
        let own = engine.registry().specs().specs().iter().map(|spec| spec.name.clone());
        let intrinsic = blueprint::intrinsics().into_iter().map(|spec| spec.name);
        let _ = self.inner.taken.set(
            own.chain(intrinsic).map(|name| name.split('.').next().unwrap_or_default().to_string()).collect(),
        );
        let _ = self.inner.library.set(Arc::downgrade(engine.library()));
        let _ = self.inner.environs.set(engine.environments().clone());
        own::install(&self.inner.studio, engine.downgrade());
        let (agents, weak) = (self.clone(), engine.downgrade());
        engine.on_stop(Arc::new(move |info: InstanceInfo| {
            let (agents, engine) = (agents.clone(), weak.upgrade());
            Box::pin(async move {
                if !info.holds.iter().any(|hold| hold.starts_with("agent:")) {
                    return;
                }
                let filter = InstanceFilter { live_only: true, limit: 100_000, ..InstanceFilter::default() };
                let live = match &engine {
                    Some(engine) => engine.instances(filter).await.unwrap_or_default(),
                    None => Vec::new(),
                };
                for hold in &info.holds {
                    // An agent goes with the last thing that holds it: its
                    // text moved to another project holds it before the
                    // one it had is stopped, and a role from before each
                    // agent had a text of its own holds it beside that.
                    let others = live.iter().any(|one| one.id != info.id && one.holds.contains(hold));
                    if let Some(name) = hold.strip_prefix("agent:")
                        && !others
                        && agents.find(name).await.ok().flatten().is_some_and(|slot| slot.active)
                    {
                        let _ = agents.deactivate(name).await;
                    }
                }
            })
        }));
    }

    /// Stops running turns and listening, without recording anything: turns
    /// in progress are run again by the next conductor.
    pub fn halt(&self) {
        for task in self.inner.tasks.lock().expect("agents lock").drain(..) {
            task.abort();
        }
        self.inner.workers.lock().expect("agents lock").clear();
    }

    fn spawn(&self, work: impl Future<Output = ()> + Send + 'static) {
        let handle = tokio::spawn(work).abort_handle();
        let mut tasks = self.inner.tasks.lock().expect("agents lock");
        tasks.retain(|task| !task.is_finished());
        tasks.push(handle);
    }
}

impl Agents {
    /// For a daemon that was asked to stop, before it stops what runs:
    /// nothing more is asked of git for a place, and what is being asked
    /// is waited for, `most` at the longest (`places::Places::settle`).
    pub async fn settle(&self, most: std::time::Duration) {
        self.inner.places.settle(most).await;
    }

    /// Whose repository somebody has looked at, which is followed whole
    /// (`places::seen::Looked`).
    pub fn looked(&self) -> Arc<places::seen::Looked> {
        self.inner.places.looked()
    }

    /// Whose branches something asked to follow (`branches.follow`), for
    /// whoever follows them (`places::branches`).
    pub fn branches(&self) -> std::sync::Arc<places::branches::Followed> {
        self.inner.places.branches()
    }
}
