//! One test binary for the crate; add modules here.
//!
//! These run the real binary, the way a person or an agent does.

mod allowed;
mod archived;
mod asked;
mod away;
mod beside;
mod beyond;
mod bot;
mod carried;
mod compared;
mod conversed;
mod copies;
mod current;
mod cut;
mod daemon;
mod documented;
mod drained;
mod environed;
mod explained;
mod external;
mod filed;
mod first;
mod flagged;
mod fleets;
mod followed;
mod guarded;
mod heard;
mod inbox;
mod kept;
mod kept_logs;
mod keyed;
mod keys;
mod kinds;
mod laying;
mod library;
mod moved;
mod opened;
mod outlined;
mod outliving;
mod packages;
mod places;
mod played;
mod profiles;
mod records;
mod renamed;
mod researched;
mod restarted;
mod sandboxed;
mod session_kept;
mod signins;
mod spaces;
mod standin;
mod stores;
mod tells;
mod texts;
mod threads;
mod topics;
mod trashed;
mod unsaid;
mod updated;
mod usage;
mod variables;
mod wrapped;
mod written;

/// A port that is a test's own (`crates/hosting/standin/ports.rs`), for
/// a daemon that is a process of its own and has to be told where to
/// listen: one that is started again where it was, or kept running. The
/// daemons here are Unix's (`daemon.rs`).
#[cfg(unix)]
#[path = "../../../hosting/standin/ports.rs"]
mod ports;
mod pressed;

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use api::{Config, Node};

pub const BINARY: &str = env!("CARGO_BIN_EXE_factory");

/// The stand-in for a program that the tests here start
/// (`crates/hosting/standin`): this crate's own copy, built with this
/// suite. It is no part of what is installed or packaged, which take the
/// product's program by its name.
pub const STANDIN: &str = env!("CARGO_BIN_EXE_cli-standin");

pub struct Rig {
    pub node: Option<Node>,
    pub url: String,
    pub dir: PathBuf,
    /// What its node said, kept or passed on, for a test to read.
    pub order: std::sync::Arc<journal::Order>,
}

impl Drop for Rig {
    fn drop(&mut self) {
        // A factory leaves what it hosts running when it stops, on
        // purpose, and a test ends what it started, however it ended.
        // Nothing is watched from here on, so nothing ended is started
        // again.
        if let Some(node) = &self.node {
            node.app.hosting.halt();
        }
        for pid in hosting::end_under(&self.dir) {
            eprintln!(
                "the test was over with a program its factory started still running; its group {pid} was ended"
            );
        }
        let _ = std::fs::remove_dir_all(&self.dir);
        if let Some(node) = &self.node {
            held(node);
            // Given back, as the API suite's rig does: the system gives a
            // user few ways of being told of files.
            node.app.hosting.watch().close();
        }
    }
}

/// How many times what is of a folder is looked for before the test says
/// what is still there: each look is a reading of what is running, and
/// what was ended is gone in a few.
#[cfg(unix)]
const LOOKS: usize = 4_000;

/// Where a process stands among the others, as the system says it: the
/// group it is in, its session, and the terminal it has, if any.
#[cfg(unix)]
fn standing(of: &std::path::Path) -> Option<(u32, u32, i64)> {
    let stat = std::fs::read_to_string(of.join("stat")).ok()?;
    let mut rest = stat.rsplit_once(')')?.1.split_whitespace().skip(2);
    Some((rest.next()?.parse().ok()?, rest.next()?.parse().ok()?, rest.next()?.parse().ok()?))
}

/// Ends everything that is running and is of the folder `dir`, a test's
/// own: a daemon started on data there, what keeps one running, the
/// wrapper of each thing a daemon there started (which says how its
/// program ended in a file there) with the program under it, and
/// whatever is in the folder. Each is ended with its group where it
/// leads one, without being asked.
///
/// A daemon leaves what it hosts running when it stops, on purpose, and
/// ending a daemon ends nothing it started; so whatever in a test owns a
/// daemon's folder asks this as it is put away, however the test ended.
/// It is looked for until none of it is there, since what is ended first
/// may have been starting something: nothing is left for whoever runs the
/// tests next. What is running is read from `/proc`; where there is none
/// nothing is found, and nothing ended.
///
/// What is ended is the tests' and nobody else's. Something is of the
/// folder by being in it, or by being this suite's program (or a copy of
/// it kept in the folder) told of the folder: somebody's own command that
/// only names the folder, to read what a test left there, is not. And
/// what has a terminal that is not the tests' own is somebody's at
/// another terminal, in the folder to look: it is left as it is.
#[cfg(unix)]
pub fn ended_under(dir: &std::path::Path) {
    let Some(named) = dir.to_str().map(str::as_bytes) else { return };
    // A file that was taken away under what runs it, or a folder under
    // what is in it, is said with a word after it.
    let link = |of: &std::path::Path, name: &str| {
        let path = std::fs::read_link(of.join(name)).ok()?.to_string_lossy().into_owned();
        Some(PathBuf::from(path.strip_suffix(" (deleted)").unwrap_or(&path)))
    };
    let program = std::fs::canonicalize(BINARY).unwrap_or_else(|_| PathBuf::from(BINARY));
    let own = standing(std::path::Path::new("/proc/self")).map(|(_, session, _)| session);
    let mut told = std::collections::BTreeSet::new();
    let mut there = Vec::new();
    for _ in 0..LOOKS {
        there.clear();
        for entry in std::fs::read_dir("/proc").into_iter().flatten().flatten() {
            let of = entry.path();
            let pid = entry.file_name().to_str().and_then(|name| name.parse::<u32>().ok());
            let Some(pid) = pid.filter(|pid| *pid != std::process::id()) else { continue };
            // One that is over and not yet taken off the list says
            // none of this, and is not found.
            let within = link(&of, "cwd").is_some_and(|folder| folder.starts_with(dir));
            let told_of = || {
                let said = std::fs::read(of.join("cmdline")).unwrap_or_default();
                said.windows(named.len()).any(|part| part == named)
                    && link(&of, "exe").is_some_and(|file| file == program || file.starts_with(dir))
            };
            if !(within || told_of()) {
                continue;
            }
            let Some((group, session, terminal)) = standing(&of) else { continue };
            if terminal != 0 && Some(session) != own {
                continue;
            }
            there.push(pid);
            if !told.insert(pid) {
                continue;
            }
            match group == pid {
                true => drop(hosting::group::stop(pid, true)),
                false => daemon::signal(pid, "-KILL"),
            }
        }
        if there.is_empty() {
            return;
        }
        std::thread::yield_now();
    }
    eprintln!("still running, though each was ended, and of {}: {there:?}", dir.display());
}

/// The same where a group is something a process holds and not something
/// it is in (`hosting::group`): nothing a daemon started there outlives
/// what holds its group, and nothing is read of what is running. Nothing
/// is done.
#[cfg(not(unix))]
pub fn ended_under(_: &std::path::Path) {}

/// Ends what is of a test's folder when the test is over, however it
/// ended (`ended_under`): for a test that starts a wrapper or a daemon
/// itself, with no rig that owns the folder.
#[cfg(unix)]
pub struct Ended<'a>(pub &'a std::path::Path);

#[cfg(unix)]
impl Drop for Ended<'_> {
    fn drop(&mut self) {
        ended_under(self.0);
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

/// What a command printed and how it exited.
pub struct Ran {
    pub code: i32,
    pub out: String,
    pub err: String,
}

impl Ran {
    /// Standard output of a command that must have succeeded.
    pub fn ok(self) -> String {
        assert_eq!(self.code, 0, "the command failed.\nstdout: {}\nstderr: {}", self.out, self.err);
        self.out
    }
}

pub async fn soon<T>(what: &str, work: impl Future<Output = T>) -> T {
    match tokio::time::timeout(Duration::from_secs(40), work).await {
        Ok(value) => value,
        Err(_) => panic!("timed out waiting for: {what}"),
    }
}

pub fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("cli-{name}-{}", &kernel::id::secret()[..12]));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// What a program started with an emptied environment still has to be
/// handed on Windows to run at all, and nothing anywhere else: without
/// where the system is (`SystemRoot`) a program there cannot open a
/// socket, and every command of the suite said that nothing was
/// answering at an address where its own node was. It is what the daemon
/// keeps for a program it starts that way (`hosting::cleared_env`).
pub fn kept_on_windows() -> Vec<(&'static str, String)> {
    match cfg!(windows) {
        true => hosting::cleared_env(true, |name| std::env::var(name).ok()),
        false => Vec::new(),
    }
}

/// The binary, about to be run with a home of its own, so nothing is read
/// from or written to the real user's settings.
pub fn command_in(dir: &std::path::Path, url: &str, args: &[&str]) -> tokio::process::Command {
    let mut command = tokio::process::Command::new(BINARY);
    command
        .args(args)
        .env_clear()
        .envs(crate::kept_on_windows())
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("HOME", dir)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_DATA_HOME", dir.join("share"))
        .env(brand::env_var("URL"), url)
        .stdin(Stdio::null());
    // How the journal gathers work is said to a suite by one word
    // (`journal::gather`), and the journal here is the daemon's: the word
    // goes on to it, or this suite is played one way whatever it is told.
    if let Ok(gathers) = std::env::var("JOURNAL_GATHERS") {
        command.env("JOURNAL_GATHERS", gathers);
    }
    // And so does the word for whether a project that is added has a
    // journal of its own (`journal::apart`).
    if let Ok(apart) = std::env::var("PROJECT_JOURNALS") {
        command.env("PROJECT_JOURNALS", apart);
    }
    command
}

/// Runs the binary that way, and waits for it.
pub async fn run_in(dir: &std::path::Path, url: &str, env: &[(&str, String)], args: &[&str]) -> Ran {
    let mut command = command_in(dir, url, args);
    for (name, value) in env {
        command.env(name, value);
    }
    let output = soon(&format!("`{}`", args.join(" ")), command.output()).await.unwrap();
    Ran {
        code: output.status.code().unwrap_or(-1),
        out: String::from_utf8_lossy(&output.stdout).into_owned(),
        err: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

impl Rig {
    /// A node in this process, with nobody registered.
    pub async fn new() -> Rig {
        let dir = scratch("node");
        let order = std::sync::Arc::new(journal::Order::default());
        let config = Config {
            listen: "127.0.0.1:0".parse().unwrap(),
            device: Some("test-device".into()),
            default_adapter: Some("builtin".into()),
            walls: Some(api::Walls::Open),
            // This node's program is the test's: it cannot be what stands between.
            stands_between: true,
            // And no test takes the power away: the disk is not waited for.
            disk: journal::Disk::Unwaited,
            order: Some(order.clone()),
            ..Config::new(dir.join("data"))
        };
        let node = Node::start(config).await.unwrap();
        Rig { url: node.url(), node: Some(node), dir, order }
    }

    /// A node whose owner, Ada, has signed in from this rig's home.
    pub async fn signed_in() -> Rig {
        let rig = Rig::new().await;
        rig.run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
            .await
            .ok();
        rig
    }

    pub async fn run(&self, args: &[&str]) -> Ran {
        run_in(&self.dir, &self.url, &[], args).await
    }

    pub async fn run_with(&self, env: &[(&str, String)], args: &[&str]) -> Ran {
        run_in(&self.dir, &self.url, env, args).await
    }

    /// Where a project's files are.
    pub fn project(&self, id: &str) -> PathBuf {
        self.dir.join("data/projects").join(id)
    }

    /// Where the factory keeps `id`'s own blueprints, in its data folder
    /// (`@blueprints/` of the project's): a test that writes one as an
    /// editor would writes it here.
    pub fn placed(&self, id: &str) -> PathBuf {
        self.dir.join("data").join("blueprints").join("projects").join(id)
    }
}
