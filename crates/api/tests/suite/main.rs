//! One test binary for the crate; add modules here.
//!
//! These tests run a whole node and talk to it over HTTP, the way the
//! client, the command line and agents do.

/// The small things a stage calls as it comes up, each over in a moment:
/// whether one is still there when somebody looks is the machine's to
/// say, so nothing that lists what is running holds it either way. Here
/// and in no module, since tests of every system list what is running.
pub(crate) const PASSING: [&str; 3] = ["std/looked", "std/due", "std/resumed"];

/// The scenarios played with the daemon stopped at each place. They are
/// the longest thing the program does, a few plays at a time for all of
/// it, and a test program begins its tests in the order of their names:
/// begun where their own names put them, last, they had most of their
/// plays still to make when every other test was over. Under a name
/// that comes before the rest they are made beside the rest. A test is
/// still found by its module (`cargo test -p api --test suite simulated::`).
#[path = "."]
mod abreast {
    mod approving;
    mod climbed;
    mod contesting;
    mod cutoff;
    mod discussing;
    mod envheld;
    mod goplayed;
    mod landing;
    mod ordering;
    mod pressing;
    mod rolling;
    mod rounded;
    mod seated;
    mod segmented;
    pub mod simulated;
    mod slowing;
    mod tagging;
    mod takenout;
    mod telling;
    mod triggering;
}
use abreast::simulated;
mod about;
mod accepted;
mod access;
mod actions;
mod added;
mod addresses;
mod aftermath;
mod agented;
mod allowed;
mod alone;
mod anchored;
mod apart;
mod appearance;
mod archived;
mod around;
mod asked;
mod assigned;
mod authority;
mod awaited;
mod away;
mod beyond;
mod blueprinted;
mod botdone;
mod brought;
mod byhand;
mod carried;
mod chained;
mod client;
mod clocked;
mod columned;
mod comings;
mod commanded;
mod commented;
mod compared;
mod concluded;
mod conflicted;
mod contested;
mod contract;
mod conversed;
mod copies;
mod costed;
mod councils;
mod current;
mod cut;
mod deploys;
mod deputised;
mod devices;
mod differed;
mod discussed;
mod docsed;
mod documented;
mod edited;
mod either;
mod environs;
mod envrules;
mod exampled;
mod examples;
mod experiments;
mod extensions;
mod external;
mod featured;
mod fetched;
mod fieldsaid;
mod finishing;
mod flags;
mod fleets;
mod folders;
mod followed;
mod followers;
mod following;
mod forgotten;
mod forked;
mod found;
mod gathered;
mod gitpage;
mod given;
mod given_back;
mod goahead;
mod grouped;
mod guarded;
mod halfdone;
mod held;
mod icons;
mod inbox;
mod inside;
mod journals;
mod kept;
mod kept_agents;
mod kept_columns;
mod kept_door;
mod keyed;
mod labels;
mod laid;
mod lanes;
mod late;
mod letgo;
mod library;
mod lines;
mod listed;
mod listening;
mod loaded;
mod looked;
mod made;
mod managed;
mod marked;
mod mockups;
mod models;
mod moment;
mod moved;
mod named;
mod noted;
mod observed;
mod opened;
mod ordered;
mod out_of_hand;
mod outlined;
mod outside;
mod ownprompt;
mod passed;
mod pieces;
mod pipelined;
mod pipelines;
mod planned;
mod played;
mod pooled;
/// A port that is a test's own (`crates/hosting/standin/ports.rs`), for
/// what is told where to listen before it is there, or comes back there.
#[path = "../../../hosting/standin/ports.rs"]
mod ports;
mod posted;
mod posting;
mod pressed;
mod prioritised;
mod private;
mod profiles;
mod prompted;
mod quiet;
mod reached;
mod readme;
mod reapplied;
mod recorded;
mod recordfiles;
mod records;
mod redeclared;
mod references;
mod refused;
mod releases;
mod relocated;
mod removed;
mod renamed;
mod repacked;
mod rescanned;
mod restarted;
mod restarting;
mod retitled;
mod reviewed;
mod roles;
mod rolledout;
mod rounds;
mod rows;
mod running;
mod sandboxed;
mod sandboxes;
mod scenarios;
mod schema;
mod scoped;
mod searched;
mod secrets;
mod segments;
mod sent;
mod sentback;
mod settled;
mod setups;
mod several;
mod shells;
mod shown;
mod shutting;
mod sidebyside;
mod sidekept;
mod signins;
mod simulation;
mod site;
mod slotted;
mod slowed;
mod snapshots;
mod spaces;
mod spent;
mod spoken;
mod staged;
mod standin;
mod stands;
mod steering;
mod steps;
mod stopped;
mod stores;
mod summarised;
mod tablemade;
mod tagged;
mod taken;
mod tells;
mod texts;
mod themed;
mod threaded;
mod tidied;
mod timed;
mod told;
mod toldtabs;
mod topiclinks;
mod topicpictures;
mod topics;
mod topicstatus;
mod trashed;
mod twice;
mod under;
mod updated;
mod used;
mod usual;
mod variables;
mod versions;
mod vouched;
mod waited;
mod walkthrough;
mod walled;
mod watches;
mod whole;
mod wholes;
mod whose;
mod wikis;
mod wikitrash;
mod windowed;
mod withdrawn;
mod wrote;

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use api::{Config, Node};
use kernel::Event;
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

pub struct Rig {
    pub node: Option<Node>,
    pub url: String,
    pub dir: PathBuf,
    pub token: String,
    /// What the node was started with, and is started with again.
    config: Config,
    client: reqwest::Client,
}

impl Drop for Rig {
    fn drop(&mut self) {
        // A factory leaves what it hosts running when it stops, on
        // purpose, and a test ends what it started, however it ended: one
        // that failed before it stopped its service, one stopped between
        // starting a program and keeping that it had (which the
        // simulation plays), one that never thought to. Nothing is
        // watched from here on, so nothing ended is started again.
        if let Some(node) = &self.node {
            node.app.hosting.halt();
        }
        ended_with(&self.dir);
        let _ = std::fs::remove_dir_all(&self.dir);
        // A node that is put away without being stopped gives back what
        // it held as it is let go of (`Drop for Node`): the suite starts
        // hundreds.
        if let Some(node) = &self.node {
            held(node);
        }
    }
}

/// Ends what a test's factory in `dir` started that is still running, and
/// says so, so that what a failed test printed shows what was ended for
/// it (`hosting::end_under`: every program a factory in this process
/// started says how it ended in its factory's folder).
pub fn ended_with(dir: &Path) {
    for pid in hosting::end_under(dir) {
        eprintln!(
            "the test was over with a program its factory started still running; its group {pid} was ended"
        );
    }
}

/// Fails the test whose node emitted an event that is not as this program
/// says its events are (`Node::undeclared`): every node a test starts is
/// asked as it is put away, so that the whole suite holds the program to
/// what blueprints are checked against.
fn held(node: &Node) {
    let undeclared = node.undeclared();
    if !std::thread::panicking() && !undeclared.is_empty() {
        panic!(
            "this node emitted events that are not as it describes them; put the description right where \
             the crate lists its events (`events.rs`), or the event:\n{}",
            undeclared.join("\n")
        );
    }
}

/// How long anything is waited for before the test says it never came.
pub const PATIENCE: Duration = Duration::from_secs(30);

/// How long a node is given to answer a request before the test says it
/// never did. A request may wait for what it asked for to be done (an
/// apply that says `wait`), so this is longer than the wait for an event.
const ANSWERED: Duration = Duration::from_secs(180);

pub async fn soon<T>(what: &str, work: impl Future<Output = T>) -> T {
    match tokio::time::timeout(PATIENCE, work).await {
        Ok(value) => value,
        Err(_) => panic!("timed out waiting for: {what}"),
    }
}

/// Like [`soon`], for what takes as long as a request may: a node
/// started, or stopped.
pub async fn in_time<T>(what: &str, work: impl Future<Output = T>) -> T {
    match tokio::time::timeout(ANSWERED, work).await {
        Ok(value) => value,
        Err(_) => panic!("timed out waiting for: {what}"),
    }
}

// A stand-in is held, and says it began, at a file the test makes
// (`crates/hosting/standin/holds.rs`): what a pipe with a name was,
// where there are such pipes, and the same on every system.
pub use standin::{Listening, hold, let_go, pass};

/// Writes down how many places the test this is asked from passed and
/// how many plays it made of them, where whatever runs the suite to time
/// it asked to be told (`PLAYED_TO`, a file: the gates and `cargo xtask
/// build-times`, which keep it beside what each test took). Told nothing,
/// as a program run by anybody else is, it writes nothing. The test is
/// known by its thread, which is called what it is.
pub fn played(places: usize, plays: usize) {
    let Ok(to) = std::env::var("PLAYED_TO") else { return };
    let test = std::thread::current().name().unwrap_or_default().to_string();
    let line = format!("{}\t{places}\t{plays}\t{test}\n", env!("CARGO_PKG_NAME"));
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(to) {
        let _ = std::io::Write::write_all(&mut file, line.as_bytes());
    }
}

/// What has a file of `dir` open, for a failure to say: this process,
/// and each process it started that is alive, with the files of the
/// folder each has open. A program started as its holder went away holds
/// what it inherited for as long as it lives.
pub fn holding(dir: &std::path::Path) -> String {
    let opens = |pid: u32| -> Vec<String> {
        let fds = std::fs::read_dir(format!("/proc/{pid}/fd")).into_iter().flatten().flatten();
        let mut open: Vec<String> = fds
            .filter_map(|fd| std::fs::read_link(fd.path()).ok())
            .filter_map(|file| file.strip_prefix(dir).ok().map(|within| within.display().to_string()))
            .collect();
        open.sort();
        open
    };
    // Every process there is, with what started it and what it is.
    let mut all: Vec<(u32, u32, String)> = Vec::new();
    for entry in std::fs::read_dir("/proc").into_iter().flatten().flatten() {
        let Some(pid) = entry.file_name().to_str().and_then(|name| name.parse::<u32>().ok()) else {
            continue;
        };
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
        let parent = stat.rsplit_once(')').and_then(|(_, rest)| rest.split_whitespace().nth(1)?.parse().ok());
        let is = std::fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
        let is = String::from_utf8_lossy(&is).replace('\0', " ");
        all.push((pid, parent.unwrap_or(0), is.chars().take(160).collect()));
    }
    let me = std::process::id();
    let mut mine = vec![me];
    while let Some((pid, ..)) =
        all.iter().find(|(pid, parent, _)| mine.contains(parent) && !mine.contains(pid))
    {
        mine.push(*pid);
    }
    let mut said = vec![format!("this process ({me}) has open of {}: {:?}", dir.display(), opens(me))];
    for (pid, parent, is) in all.iter().filter(|(pid, ..)| *pid != me && mine.contains(pid)) {
        said.push(format!(
            "alive, started by {parent}: {pid} `{}`, which has open of it: {:?}",
            is.trim(),
            opens(*pid)
        ));
    }
    said.join("\n")
}

/// The stand-in for a program, which the tests here start where they
/// need one that says, reads, writes, waits and ends as they tell it
/// (`crates/hosting/standin`): this crate's own copy, built with this
/// suite, and found by what cargo says of where.
pub const STANDIN: &str = env!("CARGO_BIN_EXE_api-standin");

fn config(dir: &std::path::Path) -> Config {
    Config {
        listen: SocketAddr::from(([127, 0, 0, 1], 0)),
        device: Some("test-device".into()),
        // No model in tests: agents are the built-in assistant.
        default_adapter: Some("builtin".into()),
        // A stand-in for a real agent: it reads the work and says it did it.
        adapters: vec![("worker".into(), standin::argv(&["swallow", "say", "I did the work."]))],
        walls: Some(api::Walls::Open),
        // This program is the test's: it cannot be what stands between.
        stands_between: true,
        // What came after what is told across its journals.
        order: Some(Arc::default()),
        // No test here takes the power away, and waiting for the disk
        // was most of what a node's start and stop took.
        disk: journal::Disk::Unwaited,
        // Nor guesses a password: hashed as hard, the owner each test
        // registers was a twentieth of what the suite took.
        hashing: people::Hashing::Least,
        ..Config::new(dir.join("data"))
    }
}

impl Rig {
    /// A node with nobody registered yet.
    pub async fn fresh() -> Rig {
        Rig::with("api", |_, _| {}).await
    }

    /// A node started with something other than the usual configuration;
    /// `change` is given the configuration and the rig's own directory.
    pub async fn with(label: &str, change: impl FnOnce(&mut Config, &std::path::Path)) -> Rig {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("{label}-{}", &kernel::id::secret()[..12]));
        std::fs::create_dir_all(&dir).unwrap();
        let mut config = config(&dir);
        change(&mut config, &dir);
        let node = in_time("a node to start", Node::start(config.clone())).await.unwrap();
        Rig {
            url: node.url(),
            node: Some(node),
            dir,
            token: String::new(),
            config,
            client: reqwest::Client::new(),
        }
    }

    /// A node with its owner, Ada, signed in.
    pub async fn new() -> Rig {
        Rig::owned(Rig::fresh().await).await
    }

    /// Another device: a node that calls itself `name`, with its own owner.
    pub async fn device(name: &str) -> Rig {
        let rig = Rig::with(&format!("api-{name}"), |config, _| config.device = Some(name.into())).await;
        Rig::owned(rig).await
    }

    pub async fn owned(mut rig: Rig) -> Rig {
        let (status, body) = rig
            .post(
                "/api/auth/register",
                json!({ "email": "ada@example.com", "name": "Ada", "password": "correct horse" }),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        rig.token = body["token"].as_str().unwrap().to_string();
        rig
    }

    pub fn node(&self) -> &Node {
        self.node.as_ref().expect("the node is running")
    }

    /// The file of the database that keeps `project`'s tables, their
    /// records and what was said of each: its own journal's where that
    /// keeps them, and the factory's otherwise. Asked while the node
    /// runs, by a test that then writes a row there as a daemon that is
    /// away would find it.
    pub fn kept_in(&self, project: &str) -> std::path::PathBuf {
        let journal = &self.node().app.journal;
        match journal.records_of(Some(project)).db().is(journal.db()) {
            true => self.config.data_dir.join("journal.db"),
            false => self.config.data_dir.join("journals").join(format!("{project}.db")),
        }
    }

    /// Where the factory keeps `project`'s own blueprints on the disk
    /// (`@blueprints/` of the project's): a test that writes one as an
    /// editor or a program from before would writes it here.
    pub fn placed(&self, project: &str) -> std::path::PathBuf {
        conductor::library::folders::kept_at(&self.config.data_dir, project)
    }

    /// Stops the node and starts another on the same data.
    pub async fn restart(&mut self) {
        self.stop().await;
        self.start().await;
    }

    /// Stops the node; what it was running is left running. It has
    /// stopped when every file of its databases is free: one that is not
    /// is what a node started next on this data is refused by, so a stop
    /// that left one, or never answered, says which and what its thread
    /// was at.
    pub async fn stop(&mut self) {
        let node = self.node.take().expect("the node is running");
        held(&node);
        let journals = node.app.journal.dbs();
        match tokio::time::timeout(ANSWERED, node.stop()).await {
            Ok(left) => {
                assert!(
                    left.is_empty(),
                    "the node stopped with a database not put away:\n{}",
                    left.join("\n")
                );
            }
            Err(_) => {
                let at: Vec<String> = journals.iter().map(journal::Db::standing).collect();
                panic!("timed out waiting for: the node to stop. Its journals' databases:\n{}", at.join("\n"))
            }
        }
    }

    /// Starts a node on the data of the one that was stopped.
    pub async fn start(&mut self) {
        if let Err(why) = self.try_start().await {
            panic!("{why}");
        }
    }

    /// The same, for whoever has more to say where it cannot be: why it
    /// could not, and what has the folder's files open at that moment,
    /// this process and whatever it started that is still alive.
    pub async fn try_start(&mut self) -> Result<(), String> {
        let what = "a node to start on the data of the one that was stopped";
        match in_time(what, Node::start(self.config.clone())).await {
            Ok(node) => {
                self.url = node.url();
                self.node = Some(node);
                Ok(())
            }
            Err(why) => Err(format!(
                "a node could not be started on the data of the one that was stopped: {why}\n{}",
                holding(&self.dir)
            )),
        }
    }

    pub async fn call(
        &self,
        method: Method,
        path: &str,
        token: Option<&str>,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut request = self.client.request(method.clone(), format!("{}{path}", self.url));
        if let Some(token) = token.filter(|token| !token.is_empty()) {
            request = request.bearer_auth(token);
        }
        if let Some(body) = body {
            request = request.json(&body);
        }
        // A node that never answers is a test that says so and which
        // request it was, not one that waits for ever and its gate with
        // it: what a route still has to ask a journal that has shut at a
        // place it was told is never done.
        let answered = tokio::time::timeout(ANSWERED, async {
            let response = request.send().await.unwrap();
            let status = response.status();
            (status, response.text().await.unwrap())
        });
        let Ok((status, text)) = answered.await else {
            panic!("timed out waiting for: the node's answer to {method} {path}")
        };
        (status, serde_json::from_str(&text).unwrap_or(Value::String(text)))
    }

    pub async fn get(&self, path: &str) -> (StatusCode, Value) {
        self.call(Method::GET, path, Some(&self.token), None).await
    }

    pub async fn post(&self, path: &str, body: Value) -> (StatusCode, Value) {
        self.call(Method::POST, path, Some(&self.token), Some(body)).await
    }

    /// A successful GET's body.
    pub async fn read(&self, path: &str) -> Value {
        let (status, body) = self.get(path).await;
        assert_eq!(status, StatusCode::OK, "GET {path}: {body}");
        body
    }

    /// A successful POST's body.
    /// Gives a project a board with the columns the tests speak of, and
    /// nothing on it: what a person does by applying `std/board`.
    pub async fn board(&self, project: &str) {
        let params = serde_json::json!({ "columns": ["draft", "ready", "doing", "review", "done"] });
        let body = serde_json::json!({ "blueprint": "std/board", "project": project, "params": params, "wait": true });
        let applied = self.send("/api/apply", body).await;
        assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    }

    pub async fn send(&self, path: &str, body: Value) -> Value {
        let (status, answer) = self.post(path, body).await;
        assert_eq!(status, StatusCode::OK, "POST {path}: {answer}");
        answer
    }

    /// The first stored event on `topic` that satisfies `wanted`, waiting
    /// for it if it has not happened yet.
    ///
    /// **Its `seq` is where it came among everything that was written**,
    /// whichever journal keeps it, and not its number (`journal::Order`):
    /// so that one event is after another, or after a [`Rig::mark`], where
    /// its `seq` is the greater, though a project's journal keeps the one
    /// and the factory's own the other. So are those of everything the
    /// rig hands over. The number an event has in its journal, which is
    /// what the node itself answers with, is [`Rig::number`] of that.
    pub async fn event(&self, topic: &str, wanted: impl Fn(&Event) -> bool) -> Arc<Event> {
        let mut follow = self.follow(topic);
        let waited = tokio::time::timeout(PATIENCE, async {
            loop {
                let event = follow.next().await.unwrap();
                if wanted(&event) {
                    return event;
                }
            }
        })
        .await;
        match waited {
            Ok(event) => event,
            // What did happen is said with what did not: a wait that fails
            // only now and then, on a machine that cannot be looked at, has
            // nothing else to go by.
            Err(_) => panic!(
                "timed out waiting for: an event on `{topic}`\nwhat happened last, the latest first:\n{}",
                self.lately().await
            ),
        }
    }

    fn order(&self) -> &Arc<journal::Order> {
        self.config.order.as_ref().expect("a rig's node keeps the order things were written in")
    }

    /// Where the last thing written came: everything said from here on,
    /// in whichever journal, is handed over with a greater `seq`.
    pub fn mark(&self) -> u64 {
        self.order().so_far()
    }

    /// The number an event has in its journal, given the `seq` the rig
    /// handed it over with.
    pub fn number(&self, seq: u64) -> u64 {
        self.order().number(seq).unwrap_or(seq)
    }

    /// Where the event numbered `number` came: the `seq` the rig hands
    /// it over with.
    pub fn came(&self, number: u64) -> u64 {
        self.order().of(number).unwrap_or(0)
    }

    /// Follows what every journal says on `topics`, from the beginning.
    pub fn follow(&self, topics: &str) -> Followed {
        self.follow_after(0, topics)
    }

    /// Follows what every journal says on `topics` that was written
    /// after `mark`: a [`Rig::mark`], or the `seq` of an event the rig
    /// handed over.
    pub fn follow_after(&self, mark: u64, topics: &str) -> Followed {
        let follow = self.order().follow_every(&self.node().app.journal, topics);
        Followed { follow, order: self.order().clone(), after: mark }
    }

    /// Every stored event `query` asks for, of every journal, in the
    /// order they were written (the newest first where that is asked),
    /// each with where it came as its `seq`; `after` and `before` are
    /// marks, as [`Rig::mark`] gives them.
    pub async fn said(&self, query: journal::EventQuery) -> Vec<Event> {
        let limit = if query.limit == 0 { 500 } else { query.limit };
        let all =
            journal::EventQuery { after: 0, before: 0, limit: 100_000, newest_first: false, ..query.clone() };
        let mut events: Vec<(u64, Event)> = Vec::new();
        // What was passed on and not kept with what was kept: what was
        // said, which is what a test holds the daemon to.
        for event in self.order().read_every(&self.node().app.journal, all).await.unwrap() {
            let placed = placed(self.order(), &event);
            if placed.seq > query.after && (query.before == 0 || placed.seq < query.before) {
                events.push((event.seq, placed));
            }
        }
        events.sort_by_key(|(number, event)| (event.seq, *number));
        if query.newest_first {
            events.reverse();
        }
        events.truncate(limit);
        events.into_iter().map(|(_, event)| event).collect()
    }

    /// The last events there were, the latest first, a line each.
    pub async fn lately(&self) -> String {
        let query = journal::EventQuery { limit: 60, newest_first: true, ..Default::default() };
        // A journal that has shut at a place it was told answers nothing:
        // a wait that ran out on such a node says so, and is not kept
        // from failing by asking what happened last.
        let read = tokio::time::timeout(
            Duration::from_secs(5),
            self.order().read_every(&self.node().app.journal, query),
        )
        .await;
        let Ok(events) = read else {
            return "  (the journal answered nothing in five seconds: it has shut, or is held up)"
                .to_string();
        };
        let events = events.unwrap_or_default();
        let lines: Vec<String> = events
            .iter()
            .map(|event| {
                let mut data = event.data.to_string();
                if data.len() > 160 {
                    let cut = (0..=160).rev().find(|at| data.is_char_boundary(*at)).unwrap_or(0);
                    data.truncate(cut);
                    data.push('…');
                }
                format!("  {} {} {}", event.seq, event.topic, data)
            })
            .collect();
        lines.join("\n")
    }

    /// How many events on `topic` there have been so far.
    pub async fn count_of(&self, topic: &str) -> usize {
        let query = journal::EventQuery { topics: topic.to_string(), limit: 10_000, ..Default::default() };
        self.order().read_every(&self.node().app.journal, query).await.unwrap().len()
    }

    /// Waits until `count` events on `topic` have happened.
    pub async fn count(&self, topic: &str, count: usize) {
        let mut follow = self.order().follow_every(&self.node().app.journal, topic);
        soon(&format!("{count} events on `{topic}`"), async {
            for _ in 0..count {
                follow.next().await.unwrap();
            }
        })
        .await;
    }
}

/// Leave to read and write any channel, for what a test here does to
/// the channels its own rig holds.
pub fn own() -> studio::Reader {
    studio::Channels::daemon("a test reads and writes what its own rig holds")
}

/// Whether the suite is played with every project kept in the factory's
/// own journal (`PROJECT_JOURNALS=one`, `journal::apart`), as a factory
/// from before keeps them. A test of a project's own journal holds
/// nothing then, and is not played: it asks first, and says so.
pub fn one_journal() -> bool {
    let one = journal::apart() == Some(false);
    if one {
        println!("not played: it is of a project's own journal, and the suite is played with one journal");
    }
    one
}

/// Whether a card of `project`, and what is said of it, are kept by one
/// journal, and so are one write: so of what is kept in the factory's
/// own, and of a project whose own journal keeps its records. A project
/// whose journal keeps what is run for it and not its records has the
/// row in one file and what is said of it in another: the row is
/// written and then said. A test of the one write asks first, and says
/// so where it holds nothing.
pub fn one_write(rig: &Rig, project: &str) -> bool {
    let journal = &rig.node().app.journal;
    let (row, said) =
        (journal.records_of(Some(project)), journal.kept_at(Some(project), journal::keeps::RUNS));
    let one = row.db().is(said.db());
    if !one {
        println!(
            "not played: a card of `{project}` is kept in one journal and what is said of it in another, \
             so the two are not one write"
        );
    }
    one
}

/// An event as the rig hands it over: with where it came among
/// everything written as its `seq`. One that was not kept has none, and
/// one written before any order was kept comes before everything.
fn placed(order: &journal::Order, event: &Event) -> Event {
    let mut placed = event.clone();
    placed.seq = order.of(event.seq).unwrap_or(0);
    placed
}

/// What every journal says, followed, each event with where it came as
/// its `seq` ([`Rig::event`]).
pub struct Followed {
    follow: journal::Following,
    order: Arc<journal::Order>,
    after: u64,
}

impl Followed {
    /// The next matching event of any journal that was written after
    /// what this follows from. Waits if there is none yet.
    pub async fn next(&mut self) -> kernel::Result<Arc<Event>> {
        loop {
            let event = self.follow.next().await?;
            let placed = placed(&self.order, &event);
            if placed.seq > self.after || !event.is_durable() {
                return Ok(Arc::new(placed));
            }
        }
    }
}
