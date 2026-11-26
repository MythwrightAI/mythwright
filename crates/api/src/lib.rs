//! The daemon.
//!
//! [`Node::start`] assembles everything (journal, conductor, hosting, studio,
//! agents) into one running node and puts one HTTP API in front of it. The
//! API is the only way in: the client, the command line and agents all use
//! it, with a credential, and none of them is trusted for where it connects
//! from.
//!
//! There is one way to find out what changed: the event stream at
//! `/api/events`. Nothing in the API is meant to be asked repeatedly; a
//! client reads a thing once and then follows the stream.

mod address;
pub mod commands;
mod cursor;
pub mod documented;
mod documents;
pub mod drain;
pub mod events;
mod followers;
mod following;
mod handed;
mod http;
pub mod laying;
mod letgo;
mod loaded;
mod mockups;
pub mod opened;
pub mod releases;
mod routes;
mod searched;
mod spent;
mod state;
mod trash;
mod whose;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use address::{let_go_of, listen_at};
use agents::Agents;
use conductor::{Engine, Layers, Registry};
use folders::{DATABASE_FILE, LOCK_PATIENCE, own_url};
use gateway::Gateway;
use hosting::Hosting;
use journal::{Db, Journal, Logs};
use kernel::{Clock, Error, Result, Source, SystemClock};
use serde_json::json;
use studio::Studio;
use tokio::task::AbortHandle;

// What works on the data folder as files is `folders`', offered here
// under the names it always had.
pub use folders::{
    across, arrival, contract, folder, forked, library, moved, place, profile, registry, repack, restarting,
    snapshot,
};
pub use hosting::Walls;
// A caller that signs what it sends is a kind of credential, which is `people`'s.
pub use people::callers;
// What keeps the sandbox tests under way is `sandbox`'s.
pub use sandbox::kept as sandboxes;
pub use setup::{byhand, examples, setups, usual};
pub use state::App;
// What is served beside the API's routes, and what every request is held
// to first, is `served`'s, offered here under the names it always had.
pub use served::client as ui;
pub use served::client::within as client_file;
pub use served::{sent, store};

/// How often measurements are written to disk, where any are worth a
/// write (`Journal::flush_metrics_when_due`).
const FLUSH_EVERY: Duration = Duration::from_secs(10);
/// How often this device's own load and memory are sampled.
const SAMPLE_EVERY: Duration = Duration::from_secs(30);
/// How long a node that stops waits for its databases' files to be free.
/// Each thread ends what it was in the middle of, and then the database
/// brings its log back and holds the file to the disk as it is closed:
/// on a machine with much else to write that alone was seen to take more
/// than two seconds, which is what was waited before, for each in turn
/// and saying nothing, and a node started next on the same data was told
/// the database was locked. A daemon is a process that ends, and loses
/// nothing by going on once this has run out; a node inside a process
/// that goes on (a test's) is told which were not free, and what the
/// thread of each was at.
const PUT_AWAY: Duration = Duration::from_secs(60);
/// How long a daemon that was asked to stop waits for what git is in the
/// middle of for a card's place: making a worktree, committing, merging.
const GIT_PATIENCE: Duration = Duration::from_secs(10);
/// Events older than this are dropped, though the newest are always kept.
const KEEP_EVENTS_MS: i64 = 14 * 24 * 3600 * 1000;
const KEEP_EVENTS_MIN: u64 = 50_000;
/// How often what is too old to matter is taken out.
const PRUNE_EVERY_MS: i64 = 3600 * 1000;
/// Measurements older than this are dropped.
const KEEP_METRICS_MS: i64 = 90 * 24 * 3600 * 1000;

#[derive(Clone)]
pub struct Config {
    /// Everything this node keeps lives under here.
    pub data_dir: PathBuf,
    /// Where new projects are created. Defaults to `<data_dir>/projects`.
    pub projects_dir: Option<PathBuf>,
    /// The address to listen on. Port `0` picks a free one.
    pub listen: SocketAddr,
    /// This device's name. Defaults to its host name.
    pub device: Option<String>,
    /// The client's files, when they are not beside the binary.
    pub ui_dir: Option<PathBuf>,
    /// An extra directory of blueprints that shadows the built-in standard
    /// library; in a development checkout, the repository's own.
    pub blueprints_dir: Option<PathBuf>,
    /// A folder of extension packages that can be installed from here by
    /// name; in a development checkout, the repository's own.
    pub packages_dir: Option<PathBuf>,
    /// The adapter new agents get. Left unset, that is the first installed
    /// extension whose program is here, else the built-in assistant.
    pub default_adapter: Option<String>,
    /// Other agent programs, as `(name, command line)`: anything that reads
    /// a prompt on standard input and prints a reply can be an agent.
    pub adapters: Vec<(String, Vec<String>)>,
    /// What whoever started the daemon has to say about why, kept with its
    /// start in the journal. A person at a terminal has nothing to say; a
    /// window that found no daemon and started one does.
    pub note: Option<String>,
    /// Whether every step writes down what it was handed, at the head of
    /// its log: for a factory to one side that somebody plays a blueprint
    /// in to see what its steps are given.
    pub handed: bool,
    /// What every page says across its top in place of what it would
    /// (`folders::across`), said as it was started and kept from then on.
    pub banner: Option<String>,
    /// Whether something keeps this daemon running that starts it again
    /// when it ends saying it is to come back (`up`, which says so as it
    /// starts it): what a step that has the factory started again asks
    /// (`factory.restart`).
    pub kept: bool,
    /// The clock to run on; a manual one in tests.
    pub clock: Option<Arc<dyn Clock>>,
    /// What this factory's profile is called (`prod`, `dev1`). The
    /// data folder keeps the first one it is given and refuses another;
    /// left unset, the factory is whatever its data folder says, which for
    /// a folder that was never given one is nothing.
    pub profile: Option<String>,
    /// For a factory with a profile: listen where its data folder says
    /// it last did, or on a free port of this device the first time, and
    /// not on `listen`. A named factory then has the same address every
    /// time without being told it, and never lands on another's.
    pub keep_address: bool,
    /// The file that lists the factories on this device, by which they are
    /// found by name and kept out of each other's directories. Left unset,
    /// this factory is in no list and asks none.
    pub registry: Option<PathBuf>,
    /// A snapshot of another factory to start this one from: a folder that
    /// factory wrote a copy of itself to. Only for a factory that does not
    /// exist yet, and that has a profile of its own; see `arrival`.
    pub from: Option<PathBuf>,
    /// How walls are held on this device: by the system's own program,
    /// if left unset. A factory told to hold none starts every agent's
    /// program seeing the device as whoever runs the daemon does.
    pub walls: Option<hosting::Walls>,
    /// The factory is its own test: its program is a test's, and cannot
    /// be started again as the wrapper of a program the factory starts.
    /// A thread of the test's process then stands in for the wrapper
    /// (`hosting::wrapped::Wraps::Here`). A factory inside a program that
    /// is not its own which does not say this starts nothing, and says
    /// so at once. Never said for a factory that is run.
    pub stands_between: bool,
    /// Where the order in which events are written is written down,
    /// whichever journal keeps each (`journal::Order`): for a factory
    /// that is its own test, whose test has to tell what came after
    /// what across its journals. Never said for a factory that is run.
    pub order: Option<Arc<journal::Order>>,
    /// What is told, and waited for, as each project's journal is about
    /// to be written into a snapshot of this factory
    /// (`snapshot::Copying`): for a factory that is its own test, whose
    /// test holds a copy there while the factory goes on writing. Never
    /// said for a factory that is run.
    pub copying: Option<snapshot::Copying>,
    /// Whether what its databases write is waited for on the disk
    /// (`journal::disk`): nowhere, for a factory that is its own test,
    /// which no loss of power is meant to outlive and whose tests are
    /// mostly that waiting otherwise. Never said for a factory that is
    /// run, which always waits.
    pub disk: journal::Disk,
    /// How hard what is kept of a password is made to guess from
    /// (`people::Hashing`): the least the hash takes, for a factory that
    /// is its own test, whose passwords stand against nobody and which
    /// registers its owner as each test begins. Never said for a factory
    /// that is run, which hashes as hard as the hash makes it.
    pub hashing: people::Hashing,
    /// The home of whoever runs the daemon, where it is not the one
    /// their profile says: for a factory that is its own test and
    /// holds walls, whose agents' packages say what their programs keep
    /// there. Never said for a factory that is run.
    pub user_home: Option<PathBuf>,
    /// Whether the factory makes its owner as it first starts, before it
    /// answers anybody (`people::opened`): said by every program that runs
    /// one. Unsaid, a factory with no owner is given one by whoever first
    /// registers from this machine, which is how a test's own node is.
    pub makes_owner: bool,
}

impl Config {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            projects_dir: None,
            listen: SocketAddr::from(([127, 0, 0, 1], 7700)),
            device: None,
            ui_dir: None,
            blueprints_dir: None,
            packages_dir: None,
            default_adapter: None,
            adapters: Vec::new(),
            note: None,
            handed: false,
            banner: None,
            kept: false,
            clock: None,
            profile: None,
            keep_address: false,
            registry: None,
            from: None,
            walls: None,
            stands_between: false,
            order: None,
            copying: None,
            disk: journal::Disk::Waited,
            hashing: people::Hashing::Hard,
            user_home: None,
            makes_owner: false,
        }
    }
}

/// A running node.
pub struct Node {
    pub app: App,
    pub addr: SocketAddr,
    /// What this factory was started from, when it was started from a copy
    /// of another and has not said so to anyone yet.
    pub arrived: Option<arrival::Arrived>,
    tasks: Vec<AbortHandle>,
    /// What answers at `addr`. Waited for when the node stops, so that the
    /// address is free by then: a factory with a profile is started
    /// again where it was.
    serving: Option<tokio::task::JoinHandle<()>>,
    /// The databases it opened, each a thread and a file, closed when it stops.
    opened: Vec<Db>,
    /// Its data folder, had for as long as the node runs, so a second one
    /// cannot start on the same data and is told where this one is.
    had: folder::Had,
    /// Asked by a step of its own to be started again (`factory.restart`).
    restarting: std::sync::Arc<restarting::Restarting>,
    /// The owner this start made, signed in for whoever started it: once,
    /// where the factory had nobody and was told to make its owner.
    pub made: Option<people::SignedIn>,
}

/// Makes the command-line client available to agents under the product's
/// command name, whatever the binary itself is called.
fn link_client(bin: &std::path::Path) {
    let Ok(exe) = std::env::current_exe() else { return };
    if std::fs::create_dir_all(bin).is_err() {
        return;
    }
    let link = bin.join(brand::CLI);
    let _ = std::fs::remove_file(&link);
    #[cfg(unix)]
    let _ = std::os::unix::fs::symlink(&exe, &link);
    #[cfg(not(unix))]
    let _ = std::fs::copy(&exe, link.with_extension("exe"));
}

impl Node {
    pub async fn start(config: Config) -> Result<Node> {
        // The port is bound before anything slow happens, so "is it up?" has
        // an answer as early as possible and a second node fails at once.
        // What the data folder is comes first of all: a folder that is
        // another profile's is refused before anything is touched.
        let settled =
            profile::settle(&config.data_dir, config.profile.as_deref(), config.listen, config.keep_address)?;
        // And whether it may be started from a copy of another, if it was
        // asked to be: refused here, nothing has been written anywhere.
        let arriving = arrival::check(&config.data_dir, config.from.as_deref(), config.profile.as_deref())?;
        let listen = settled.listen;
        let listener = listen_at(listen, settled.remembered, &config.data_dir).await?;
        let addr = listener.local_addr()?;

        std::fs::create_dir_all(&config.data_dir)?;
        // Everything the factory keeps is here, every credential among it:
        // the folder is its owner's alone, whatever it was made as.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&config.data_dir, std::fs::Permissions::from_mode(0o700));
        }
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(config.data_dir.join(folder::LOCK))?;
        // A lock outlives its holder for an instant in any child that was
        // being started as the holder exited, so a node taking over from one
        // that has just stopped waits briefly before concluding that another
        // really is running. What it concludes is read once it has waited:
        // a daemon that is stopping has said by then that it serves no more.
        let mut waited = Duration::ZERO;
        while lock.try_lock().is_err() {
            if waited >= LOCK_PATIENCE {
                return Err(match folder::served(&config.data_dir) {
                    Some(at) => folder::running(&config.data_dir, at),
                    None => folder::busy(&config.data_dir),
                });
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
            waited += Duration::from_millis(25);
        }
        // The folder is this node's alone from here, and says where it is
        // served: whoever comes to it meanwhile is told.
        let had = folder::Had::serves(&config.data_dir, lock, addr)?;

        let clock: Arc<dyn Clock> = config.clock.clone().unwrap_or_else(|| Arc::new(SystemClock));
        // A copy of another factory is put into it now, before it is a factory: the copying
        // is files, and is done off the threads everything else runs on.
        if let Some(plan) = arriving {
            let (data, projects, now) =
                (config.data_dir.clone(), config.projects_dir.clone(), clock.now_ms());
            tokio::task::spawn_blocking(move || plan.seed(&data, projects.as_deref(), now))
                .await
                .map_err(|err| Error::internal(format!("putting the copy in place crashed: {err}")))??;
        }

        // What it is, and where it now listens, are written into it.
        let stamp = settled.stamp.map(|stamp| profile::Stamp { address: Some(addr), ..stamp });
        if let Some(stamp) = &stamp {
            stamp.write(&config.data_dir)?;
        }
        let name = stamp.as_ref().map(|stamp| stamp.profile.clone());

        let data = &config.data_dir;
        let device = config.device.clone().unwrap_or_else(|| {
            hosting::resources(data)
                .ok()
                .map(|found| found.host)
                .filter(|host| !host.is_empty())
                .unwrap_or_else(|| "device".into())
        });
        // What a `repack` that was cut off left beside the database goes
        // before the database is opened: the folder is held, as it was
        // while that was being written.
        let cleared = repack::clear(data);
        // And what a move of a project to a journal of its own left,
        // cut off before or after it was decided: a file that no
        // project's journal is kept in.
        let unnamed = moved::clear(data);
        // And what a sandbox test left, where a daemon was killed in the
        // middle of one: a folder nothing will ever read.
        sandboxes::clear(data);
        // Opened waiting for the disk or not, as was said: the journals
        // of its projects are then opened as this one was.
        let open = |file: &std::path::Path| match config.disk {
            journal::Disk::Waited => Db::open(file),
            journal::Disk::Unwaited => Db::open_unwaited(file),
        };
        let journal = Journal::open(open(&data.join(DATABASE_FILE))?, clock, device).await?;
        if let Some(order) = &config.order {
            journal.orders_by(order.clone());
        }
        // The journals of its projects are opened with it, before
        // anything is said: each a file and a thread of its own.
        journal.keeps_projects_in(data.join(snapshot::JOURNALS)).await?;
        // A factory started from a copy of another says so first of all;
        // and one whose database was written out again while it was
        // stopped, that it was.
        let arrived = arrival::announce(&journal).await;
        repack::announce_cleared(&journal, cleared).await;
        repack::announce_reindexed(&journal).await;
        repack::announce(&journal).await;
        moved::announce_cleared(&journal, unnamed).await;
        moved::announce(&journal).await;
        if let Some(says) = &config.banner {
            // Told what to say across every page, it says it, though it was told it may stop.
            let kept = folders::across::Kept { quiet: false, says: Some(says.clone()) };
            folders::across::keep(&journal, &kept).await?;
        }
        if arrived.is_some() {
            // The note that a snapshot was arriving, if a start was killed
            // between putting the database in place and taking it away.
            let _ = std::fs::remove_file(data.join(arrival::MARKER));
        }
        let logs = Logs::new(data.join("logs"));
        logs.say_what_is_handed(config.handed);
        // Where the factory's owner said runs' steps write, and wrote before.
        journal.tell_logs(&logs).await;
        let projects_dir = config.projects_dir.clone().unwrap_or_else(|| data.join("projects"));
        // What is searched is kept in a database of its own, and measured the same way.
        let searched = open(&data.join("search.db"))?;
        journal.measures(&searched, "search");
        // The databases this node opened beside its journals', which it
        // asks for when it stops: closed again then.
        let opened = vec![searched.clone()];
        let studio =
            Studio::open(journal.clone(), searched, projects_dir.clone(), data.join(snapshot::ATTACHMENTS))
                .await?;
        studio.projects.keeps_out_of(data);
        studio.identity.hashes(config.hashing);
        // Every write of a topic writes what goes with it, from before anything is written.
        topics::Topics::open(&journal, &studio).await?;
        let hosting = Hosting::new(journal.clone(), logs.clone(), data.join("run")).await?;
        hosting.answers_at(own_url(addr));
        // Sealed with a key of this installation's, which no copy has.
        let own = gateway::own::Own::open(data).map_err(|err| Error::internal(err.message))?;
        hosting.keeps_secrets_in(data.join(snapshot::SECRETS), own.at_rest());
        hosting.in_profile(name.clone().unwrap_or_default());
        hosting.holds_walls(config.walls.clone().unwrap_or_else(hosting::Walls::system));
        if config.stands_between {
            hosting.stands_between();
        }
        let agent_config = agents::Config {
            api_url: own_url(addr),
            home: data.join(snapshot::SLOTS),
            bin: data.join("bin"),
            default_adapter: config.default_adapter.clone(),
            extensions: data.join(snapshot::EXTENSIONS),
            offered: config.packages_dir.clone(),
            signins: data.join(snapshot::SIGNINS),
            seal: own.at_rest(),
            data: data.clone(),
            user_home: config.user_home.clone(),
        };
        link_client(&agent_config.bin);
        let agents = Agents::open(journal.clone(), studio.clone(), hosting.clone(), agent_config).await?;
        for (name, argv) in &config.adapters {
            agents.add_adapter(name, Arc::new(agents::adapter::Command { argv: argv.clone() }));
        }

        let gateway = Gateway::new(journal.clone(), studio.identity.clone(), hosting.clone(), data.clone())?;

        let mut registry = Registry::new();
        hosting.register(&mut registry);
        studio.register(&mut registry);
        agents.register(&mut registry);
        gateway.register(&mut registry);
        comparisons::register(&mut registry);
        blocks::register(&mut registry);
        let copied = snapshot::Source {
            data: data.clone(),
            projects: projects_dir,
            profile: name.clone(),
            copying: config.copying.clone(),
        };
        folders::register(&mut registry, copied, studio.projects.clone());
        told::register(&mut registry, &journal, &studio);
        outline::register(&mut registry, outline::Outlines::new(journal.clone(), studio.clone()));
        topics::register(&mut registry);
        let names = Arc::new(served::Names::open(journal.clone()).await);
        served::register(&mut registry, names.clone());
        let restarting = restarting::Restarting::new(data.clone(), config.kept);
        restarting::register(&mut registry, restarting.clone(), agents.clone());
        documented::register(&mut registry, studio.clone());
        setup::register(&mut registry, studio.clone());
        // After everything that is fixed: what the installed packages
        // declare, asked for each time, since packages come and go.
        registry.also(agents.actions(&hosting));
        // Every crate has said what it emits. From here a handler on a
        // topic of this program's that is not among them is a mistake,
        // and (where this is run to be tested) so is an event that is.
        registry.emits(events::events()).emits_no_more();
        // What comes and goes with a package is asked of the engine's
        // registry as it is then, held weakly. The journal holds what
        // events are held to, and a copy of the registry held there held
        // every action and, through them, the journal: nothing of a node
        // was given back, some two megabytes for every node the API
        // suite started, seven gigabytes by its end.
        let described: Arc<std::sync::OnceLock<conductor::Downgraded>> = Arc::default();
        if cfg!(debug_assertions) {
            let own = blueprint::EventList::new(registry.own_events().to_vec());
            let engine = described.clone();
            // And an event of a project's slot or channel says which
            // project, which is what whose it is, is read from.
            let whose = whose::Whose::default();
            journal.hold(Arc::new(move |topic, data| {
                let engine = engine.get().and_then(conductor::Downgraded::upgrade);
                let described = match (own.get(topic), engine) {
                    // One that comes and goes with a package, or none at all.
                    (None, Some(engine)) => engine.registry().events().fault(topic, data),
                    _ => own.fault(topic, data),
                };
                described.or_else(|| whose.fault(topic, data))
            }));
        }

        // Nearest first: the project's own blueprints, then a development
        // checkout's, then what the installed packages bring, then what
        // ships with the product. The last two cannot meet: a package's
        // are under its own name, and no package may be called `std`.
        let mut shipped = Layers::new();
        let checkout: Option<Arc<dyn conductor::Library>> = config.blueprints_dir.as_ref().map(|dir| {
            Arc::new(conductor::DirLibrary::new(dir, brand::BLUEPRINT_EXT)) as Arc<dyn conductor::Library>
        });
        if let Some(checkout) = &checkout {
            shipped = shipped.then(checkout.clone());
        }
        let shipped: Arc<dyn conductor::Library> =
            Arc::new(shipped.then(agents.library()).then(Arc::new(conductor::standard())));
        // A project's own come first, and are held to what a project that
        // is not trusted may put in place of what ships (`library`).
        // Where each project keeps its own: read once, and told what is
        // said of it from then on (`routes::kept_in`).
        let folders = studio.projects.folders().clone();
        folders.read(&journal).await;
        let own_blueprints = library::Own { projects: studio.projects.clone(), folders };
        let own = library::ProjectLibrary::new(own_blueprints.clone(), shipped.clone());
        // A sandbox test reads the blueprints that ship, and is handed
        // nothing that is this factory's: what its packages bring is
        // read as texts as each sandbox is made.
        let sandboxes = sandboxes::Sandboxes::new(data, journal.clock().clone(), checkout, agents.library());
        let layers = Layers::new().then(Arc::new(own)).then(shipped);
        let engine = Engine::new(journal.clone(), logs.clone(), registry, Arc::new(layers)).await?;
        let _ = described.set(engine.downgrade());
        engine.in_profile(name.clone().unwrap_or_default());
        let place = state::Place {
            registry: join(&config, &studio, &journal, name.as_deref().unwrap_or_default(), addr).await,
            profile: name,
            data: kernel::whole::whole(data).unwrap_or_else(|_| data.clone()),
            cookie: state::cookie_for(stamp.as_ref()),
        };
        hosting.attach(&engine);
        studio.attach(&engine);
        agents.attach(&engine);
        gateway.attach(&engine);
        let comparisons = comparisons::Comparisons::attach(&engine);

        // An owner before anybody is answered, where there is none and the
        // factory makes one; and so their workspace, below.
        let made = match config.makes_owner {
            true => studio.identity.make_owner().await?,
            false => None,
        };
        // A factory always has a workspace and every project is in one:
        // brought about here for a factory an earlier program made, and
        // for whatever a stop left undone, before anybody is answered.
        // Where that is so already nothing is written and nothing said.
        studio.settle().await?;
        // And every topic is somewhere there is (`topics::Topics::settle`).
        topics::Topics::new(journal.clone(), studio.clone(), None).settle(None).await?;
        // Whether a run holds a go-ahead to start something under an
        // environment, asked where it is started: before anything is taken up.
        engine.environments().answers(told::goes::GoAheads(journal.clone()));
        // What an ask owed when the daemon stopped: lines not yet told, what came of one not yet told back.
        told::Asks::new(&journal, &studio).settle().await?;

        // What was running is found before anything is resumed, so resumed
        // steps find their services where they left them.
        hosting.resume().await?;
        let resumed = engine.resume().await?;
        agents.start();
        // An agent added by hand before agents had a text of their own
        // is given its text, once ([`byhand::take_up`]).
        byhand::take_up(&engine, &agents).await;
        // What a factory from before vouched for of a folder its projects no longer read, once.
        library::settle(&own_blueprints, &engine, &journal).await;
        // Last: resumed steps for other devices are waiting by the time the
        // links come back, and this device's own hub finds it ready.
        gateway.start().await?;

        let app = App::new(
            journal.clone(),
            engine,
            comparisons,
            hosting,
            studio.clone(),
            agents,
            gateway,
            served::client::Client::open(&journal, config.ui_dir.clone()).await,
            addr,
            names,
            place,
            sandboxes,
            own_blueprints,
        );
        commands::listen(&app);
        // What reads the tables for the index, before it reads anything.
        searched::install(&studio);
        let mut tasks =
            vec![studio.search.start().abort_handle(), studio.watch_runs(&app.engine).abort_handle()];
        tasks.push(tokio::spawn(housekeeping(journal.clone())).abort_handle());
        tasks.push(tokio::spawn(sample(journal.clone(), data.clone())).abort_handle());
        tasks.push(tokio::spawn(letgo::keep(app.clone())).abort_handle());
        tasks.push(tokio::spawn(trash::keep(app.clone())).abort_handle());
        tasks.push(tokio::spawn(loaded::keep(app.clone())).abort_handle());
        let watching = app.clone();
        let watched = async move {
            served::client::watch(&watching.client, &watching.journal, watching.hosting.watch()).await
        };
        tasks.push(tokio::spawn(watched).abort_handle());
        tasks.push(tokio::spawn(documents::follow(app.clone())).abort_handle());
        tasks.push(tokio::spawn(routes::git::follow(app.clone())).abort_handle());
        tasks.push(tokio::spawn(routes::topics::follow_places(app.clone())).abort_handle());
        tasks.push(tokio::spawn(followers::tell(app.clone())).abort_handle());
        tasks.push(tokio::spawn(followers::digests(app.clone())).abort_handle());
        tasks
            .push(tokio::spawn(following::follow(app.clone(), config.blueprints_dir.clone())).abort_handle());
        // Each request with where it came from, which is what says whether
        // the first account is being made on this machine.
        let router = routes::router(app.clone()).into_make_service_with_connect_info::<SocketAddr>();
        let serving = tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });

        let started = json!({
            "address": addr.to_string(), "version": place::version(), "resumed": resumed,
            "note": config.note, "profile": app.place.profile,
            // Whether this is somebody's installed copy or a program run
            // from the checkout it was built in, which reads that checkout.
            "runs_from": place::runs_from(),
        });
        let _ = journal.emit("system.started", Source::actor("system"), started).await;
        Ok(Node { app, addr, arrived, tasks, serving: Some(serving), opened, had, restarting, made })
    }

    /// A link that signs the owner in once, for the program this node is in.
    pub async fn link_for_owner(&self) -> Result<people::opened::Link> {
        self.app.studio.identity.link_for_owner().await
    }

    /// What this factory's profile is called, when it has one.
    pub fn profile(&self) -> Option<&str> {
        self.app.place.profile.as_deref()
    }

    /// `http://<address>` of this node.
    /// What this node offers a blueprint, written as the kept contract is
    /// (`contract`): the actions built into it and the events it emits.
    pub fn offers(&self) -> String {
        contract::offered(self.app.engine.registry())
    }

    /// What this node emitted that is not as it says its events are.
    /// Looked for only in a build made to be tested, and empty in any
    /// other: a test's node is asked this as it is put away.
    pub fn undeclared(&self) -> Vec<String> {
        self.app.journal.faults()
    }

    /// Where this node is reached from the device it is on: what whoever
    /// started it is told to open. One that listens on every address is
    /// said at this device's own, and never at the address that means
    /// every one (`0.0.0.0`), which is nowhere to go: Linux takes it for
    /// the device itself, Windows refuses it, and a browser may.
    pub fn url(&self) -> String {
        own_url(self.addr)
    }

    /// Completes when the owner has asked, through the API, for the daemon
    /// to stop. Whoever runs the node decides what that means; `stop` below
    /// is the usual answer.
    pub async fn asked_to_stop(&self) {
        tokio::select! {
            () = self.app.stop.notified() => {}
            () = self.restarting.asked() => {}
        }
    }

    /// Whether the stop was asked for by a step that wants the daemon back
    /// (`factory.restart`): whoever runs the node has what keeps it running
    /// start it again ([`restarting::AGAIN`]).
    pub fn again(&self) -> bool {
        self.restarting.again()
    }

    /// Completes when a step has asked for the daemon to be started again:
    /// for whoever stands in for what keeps it running (a test).
    pub async fn asked_again(&self) {
        self.restarting.asked().await;
    }

    /// Stops the node without stopping what it runs: services and commands
    /// carry on, and the next node picks them up.
    ///
    /// It has stopped when the files of its databases are free, every one
    /// of them, and nothing answers at its address any more (where a
    /// program it was starting has a copy of what it listened with, for
    /// two seconds at most: `let_go_of`). It gives what it could not put
    /// away, which is nothing as a rule: each database whose file was not
    /// free when the wait for them ran out ([`PUT_AWAY`]), with what its
    /// thread was at.
    pub async fn stop(self) -> Vec<String> {
        self.stop_within(PUT_AWAY).await
    }

    /// Like [`Node::stop`], waiting only so long for the databases' files
    /// to be free: for whoever has to be told at once what holds one.
    pub async fn stop_within(self, patience: Duration) -> Vec<String> {
        self.stop_telling(patience, |_| {}).await
    }

    /// Like [`Node::stop`], telling `told` what the stop waits for that
    /// is not its own: a daemon says it where it is run.
    pub async fn stop_saying(self, told: impl Fn(&str)) -> Vec<String> {
        self.stop_telling(PUT_AWAY, told).await
    }

    async fn stop_telling(mut self, patience: Duration, told: impl Fn(&str)) -> Vec<String> {
        for task in &self.tasks {
            task.abort();
        }
        // Stopped is when nothing listens here any more, not a moment before.
        // The folder says so first: it is this node's until its files are
        // put away, and whoever comes to it meanwhile waits for that and
        // is not told that it is being served.
        self.had.serves_no_more();
        if let Some(serving) = self.serving.take() {
            serving.abort();
            let _ = serving.await;
        }
        // What git is in the middle of is let come to its end first, and
        // nothing more is asked of it: stopped half way it leaves a
        // worktree half made or a lock that the next daemon meets.
        self.app.agents.settle(GIT_PATIENCE).await;
        // Every sandbox test is over: its thread, its databases and its folder.
        self.app.sandboxes.end_all().await;
        self.app.engine.halt();
        self.app.gateway.halt();
        self.app.agents.halt();
        self.app.hosting.halt();
        // A journal that shut at a place it was told keeps one thing more,
        // that the daemon stopped, and nothing else: not what was measured
        // since, which a daemon that went at that place would not have
        // written, and nothing of what may still be about of this one.
        // Nor where it shuts while that is being written: a journal that
        // has shut answers nothing, and a stop that looked a moment before
        // it shut, and then asked, waited for ever.
        let _ = self.app.journal.unless_shut(self.app.journal.flush_metrics()).await;
        let _ = self.app.journal.emit_last("system.stopped", Source::actor("system"), json!({})).await;
        // Nothing of this node asks its databases anything again. Their
        // threads and files are given back now, and not when the last thing
        // that still holds the node lets go of it, which may be never.
        let opened = self.databases();
        for db in &opened {
            db.close();
        }
        // And stopped is when their files are free: a daemon started on
        // this data the moment this one has stopped opens them at once.
        // Every one of them is waited for, under one bound for them all,
        // each having been asked first: a project's whose thread waits on
        // the factory's for a run of numbers is answered by the factory's
        // ending. One that is not free when the bound runs out is said,
        // with what its thread was at.
        let put_away = async {
            for db in &opened {
                db.closed().await;
            }
        };
        let left = match tokio::time::timeout(patience, put_away).await {
            Ok(()) => Vec::new(),
            Err(_) => opened.iter().filter(|db| !db.is_closed()).map(Db::standing).collect(),
        };
        // And stopped is when its address is free, with the folder still
        // its own meanwhile (`had` goes with this): whoever comes to it
        // then waits, as a starting node waits for the lock.
        let_go_of(self.addr, &told).await;
        left
    }

    /// Every database this node has open, in the order they are closed
    /// in: each project's journal, the factory's own, which keeps the
    /// giver the others draw from, and then what else it opened.
    fn databases(&self) -> Vec<Db> {
        let mut all = self.app.journal.dbs();
        all.extend(self.opened.iter().cloned());
        all
    }
}

/// A node that is let go of gives back what it held, stopped or not: what
/// it started is ended, and its logs, its databases and what it followed
/// are closed. A daemon that ends is a process that ends, and the system
/// takes all of it back. A node inside a process that goes on (a test's)
/// is held for ever by what it started, which holds it in turn, and with
/// it a file for each of its databases and logs and what its tasks were
/// run by. Nothing is said or written here: stopping in order is
/// [`Node::stop`], which comes here last.
impl Drop for Node {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
        if let Some(serving) = &self.serving {
            serving.abort();
        }
        self.app.sandboxes.let_go();
        self.app.engine.halt();
        self.app.gateway.halt();
        self.app.agents.halt();
        self.app.hosting.halt();
        self.app.studio.halt();
        self.app.engine.logs().close();
        for db in self.databases() {
            db.close();
        }
    }
}

/// Puts a starting factory among the factories on its device: where it is
/// now, and the directories its projects are in, are written to the list,
/// and from here on the list is asked before a directory becomes a project.
///
/// A list that cannot be written does not stop a factory. It is then not
/// found by name and not kept out of another's directories, and says so in
/// its own log. A factory with no profile that has adopted nothing has
/// nothing to say, and writes nothing.
async fn join(
    config: &Config,
    studio: &Studio,
    journal: &Journal,
    profile: &str,
    addr: SocketAddr,
) -> Option<registry::Registry> {
    let registry = registry::Registry::at(config.registry.as_ref()?);
    let held = registry::Held::new(registry.clone(), &config.data_dir, profile);
    studio.projects.held_through(Arc::new(held));
    let holds: Vec<PathBuf> = studio
        .projects
        .list()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|project| project.path.into())
        .collect();
    if let Err(err) = registry.started(&config.data_dir, profile, addr, &holds) {
        let effect = "this factory is not found by its profile's name, and a directory it holds is not \
                      refused to a factory of another profile";
        let data = json!({ "what": "the list of factories on this device", "reason": err.message, "effect": effect });
        let _ = journal.emit("system.degraded", Source::actor("system"), data).await;
    }
    Some(registry)
}

/// Writes measurements out regularly and drops what is too old to matter.
async fn housekeeping(journal: Journal) {
    let clock = journal.clock().clone();
    // By the clock, which is what says an hour has gone by.
    let mut pruned = journal.now_ms();
    let mut said = false;
    loop {
        clock.sleep(FLUSH_EVERY).await;
        let _ = journal.flush_metrics_when_due().await;
        let now = journal.now_ms();
        if now - pruned >= PRUNE_EVERY_MS {
            pruned = now;
            // One that fails would fail every hour, with the journal
            // growing and nobody told: it is said, once for a daemon.
            // Each journal's own: a project's keeps its newest, however
            // old, as the factory's does.
            let mut taken = Ok(0);
            for each in journal.every() {
                let done = each.prune_events(now - KEEP_EVENTS_MS, KEEP_EVENTS_MIN).await;
                taken = taken.and(done);
            }
            if let Err(err) = taken
                && !said
            {
                said = true;
                let effect = "events are kept however old they are, and the journal goes on growing";
                let data = json!({
                    "what": "taking old events out of the journal", "reason": err.message, "effect": effect,
                });
                let _ = journal.emit("system.degraded", Source::actor("system"), data).await;
            }
            let _ = journal.prune_metrics(now - KEEP_METRICS_MS).await;
        }
    }
}

/// Measures this device: how loaded it is and how much memory is free.
/// Without this, "is the hub struggling?" is a guess.
async fn sample(journal: Journal, data: PathBuf) {
    let clock = journal.clock().clone();
    loop {
        let path = data.clone();
        if let Ok(Ok(found)) = tokio::task::spawn_blocking(move || hosting::resources(&path)).await {
            let device = journal.device().to_string();
            let labels = [("device", device.as_str())];
            // Quietly: the device is sampled whether or not anything is
            // happening, and a factory at rest is not written to for it.
            journal.measure_quietly("device.free_mem_mb", &labels, found.free_mem_mb as f64, true);
            if let Some(load) = found.load1 {
                journal.measure_quietly(
                    "device.load_per_cpu",
                    &labels,
                    load / found.cpus.max(1) as f64,
                    true,
                );
            }
            if let Some(disk) = found.free_disk_mb {
                journal.measure_quietly("device.free_disk_mb", &labels, disk as f64, true);
            }
            let data = serde_json::to_value(&found).unwrap_or_default();
            journal.emit_ephemeral("device.sample", Source::actor("system"), data);
        }
        clock.sleep(SAMPLE_EVERY).await;
    }
}
