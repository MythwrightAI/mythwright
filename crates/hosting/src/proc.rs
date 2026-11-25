//! Processes that outlive the conductor.
//!
//! Everything started here is started the same way, for one reason: the
//! conductor must be able to stop and come back without taking the fleet with
//! it. So a process:
//!
//! * runs in a group of its own ([`crate::group`]), which makes "stop it
//!   and everything it started" one asking and keeps a Ctrl-C in the
//!   conductor's terminal away from it;
//! * writes its output to files, not pipes, so it does not die of a broken
//!   pipe when its parent does;
//! * leaves its exit code in a file, so it can be collected by a conductor
//!   that is not its parent;
//! * is known by pid *and* start time, because pids are reused and "is pid
//!   4242 alive" is not the same question as "is my process alive".
//!
//! What does the first and the third is the wrapper ([`crate::wrapped`]):
//! this program, started in the program's place to start it, wait for it
//! and say how it ended. The process that is known, waited on and found
//! again is the wrapper, which outlives what it started.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use kernel::{Error, Result};
use serde_json::{Value, json};

use crate::watch::{Changes, Watch};
use crate::wrapped::{Asked, Wraps};

/// How often a process started by an earlier conductor is looked for when
/// nothing has been heard of it.
///
/// A process of this conductor's own is waited on, and the system says when
/// it ends. One it merely inherited is nobody's child here, and there is no
/// way that works on every system of being told that it died. Almost always
/// its files say so (the exit code is written, or its output is let go of);
/// this is for one that was killed outright where the system does not report
/// that.
const ADOPTED_LOOK: Duration = Duration::from_secs(5);
/// The same, for the moments after such a process was told to stop, when its
/// end is expected and someone is waiting for it.
const STOPPING_LOOK: Duration = Duration::from_millis(100);

pub(crate) struct Launch {
    /// A script of its user's own, for their system's shell
    /// ([`crate::script`]), or
    pub script: Option<String>,
    /// a program and its arguments, run without a shell's interpretation.
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    pub env: BTreeMap<String, String>,
    /// Start from an almost empty environment instead of the conductor's.
    pub clear_env: bool,
    /// Where the daemon is, when it has said: told to the process as
    /// [`ADDRESS_ENV`].
    pub address: Option<String>,
    /// What the factory's profile is called, when it has been said: told
    /// to the process as [`PROFILE_ENV`].
    pub profile: Option<String>,
    /// Where the output and exit files go.
    pub dir: PathBuf,
    /// File stem for them.
    pub tag: String,
    /// The variables of `env` that hold a secret, each with the name it is
    /// kept under. With any, what the program prints goes through its
    /// wrapper on its way to the files ([`crate::unsaid`]), so that what
    /// lies there, and everything read from there, is without the value.
    pub handed: BTreeMap<String, String>,
    /// What wraps the program.
    pub wraps: Wraps,
}

pub(crate) struct Proc {
    pub pid: u32,
    /// Start time as the operating system reports it; with `pid`, an identity.
    pub started: u64,
    pub out: PathBuf,
    pub err: PathBuf,
    pub exit: PathBuf,
    held: Held,
    /// For an adopted process: changes to its files, which are how its end
    /// is heard of.
    changes: Option<Changes>,
    /// It has been told to stop.
    stopping: bool,
}

/// How a process is waited for.
enum Held {
    /// It is this conductor's child, and the system says when it ends.
    Child(Box<tokio::process::Child>),
    /// A thread of this process waits for it, and says
    /// ([`Wraps::Here`]).
    Here(tokio::sync::watch::Receiver<crate::here::How>),
    /// An earlier conductor started it: its files say.
    Adopted,
}

/// Environment variables passed through even with `clear_env`.
const BASE_ENV: &[&str] = &["PATH", "HOME", "USER", "LANG", "TERM", "TMPDIR", "TEMP", "SystemRoot"];

/// The same on Windows, beside those: without them a program there does
/// not find its user's folders or the system's, the endings a program's
/// file has, or the system's shell. Only there: a variable of one of
/// these names on another system is somebody's own, and is cleared.
const WINDOWS_ENV: &[&str] = &[
    "USERPROFILE",
    "USERNAME",
    "HOMEDRIVE",
    "HOMEPATH",
    "APPDATA",
    "LOCALAPPDATA",
    "PATHEXT",
    "ComSpec",
    "TMP",
    "SystemDrive",
    "ProgramData",
    "ProgramFiles",
    "windir",
];

/// What a program started from an almost empty environment (`clear_env`)
/// keeps of the daemon's: each variable every program needs to run at
/// all that `value_of` has, with its value. `windows` says whether the
/// system is that one, whose programs need more.
pub fn cleared_env(windows: bool, value_of: impl Fn(&str) -> Option<String>) -> Vec<(&'static str, String)> {
    let more: &[&str] = if windows { WINDOWS_ENV } else { &[] };
    BASE_ENV.iter().chain(more).filter_map(|name| Some((*name, value_of(name)?))).collect()
}

/// The variable in which everything started here is told where the
/// product's own program is: the daemon is also the command line, and a
/// blueprint that needs to run it (`"$FACTORY_CLI" store serve ...`) has no
/// other way to name it that is true on every device, installed or run
/// from a checkout. It is the program only: no credential comes with it,
/// so a command that talks to the daemon still has to sign in as someone.
pub const PROGRAM_ENV: &str = kernel::git::PROGRAM;

/// The variable in which everything started here is told where the daemon
/// itself answers, as `http://127.0.0.1:7700`: a blueprint that puts the
/// daemon within reach of something else (another device, a proxy) has to
/// say what is to be reached, and the address is whatever the daemon was
/// started with. It is the address only. Like the program, it comes with
/// no credential: whatever is sent there still has to sign in as someone.
pub const ADDRESS_ENV: &str = "FACTORY_URL";

/// The variable in which everything started here is told what the factory's
/// profile is called (`prod`, `dev1`), and that it has none by the
/// variable being empty: a command can then do in one profile what it
/// does not do in another, as an expression can with `profile.name`.
/// It is a name and nothing more: it reaches no other factory.
pub const PROFILE_ENV: &str = "FACTORY_PROFILE";

/// The variable that says the environment of its project's an instance
/// was applied under, as an expression's `environment.name`. It said the
/// profile before 0.1.0, so it is taken out of what a program inherits
/// and set only by the runner, for a step of an instance that runs under
/// one (`environments::held::ENV_VAR`): a program started here never
/// reads the old meaning.
pub const ENVIRONMENT_ENV: &str = environments::held::ENV_VAR;

/// Where this program is, found once and kept. It is asked for as the
/// conductor starts, while the file it was started from is certainly still
/// the one on disk: a program that is rebuilt while it runs is afterwards
/// reported by some systems under a name that leads nowhere.
pub(crate) fn program() -> Option<&'static Path> {
    static PROGRAM: OnceLock<Option<PathBuf>> = OnceLock::new();
    PROGRAM.get_or_init(|| std::env::current_exe().ok()).as_deref()
}

/// This program with `args` after it, as a command run without a shell:
/// what `cli: [...]` on a `shell` step or a service means. A blueprint we
/// ship that needs this program (`std/store`) names it this way and not
/// as `"$FACTORY_CLI"`, which only a shell expands and each shell its own
/// way. It is found afresh each time something is started, so that a
/// service outlives the program being moved or replaced.
pub(crate) fn own(args: &[String]) -> std::result::Result<Vec<String>, String> {
    let program = program().ok_or_else(|| {
        format!("this program could not say where it is, so `cli: [{}]` cannot be run", args.join(", "))
    })?;
    let mut argv = vec![program.to_string_lossy().into_owned()];
    argv.extend(args.iter().cloned());
    Ok(argv)
}

/// Gives a command the environment of everything started from here: the
/// conductor's own without its credentials, where the product's program and
/// the daemon are, what the factory's profile is called, and then
/// whatever the blueprint and the step said, which is theirs to say.
pub(crate) fn dress(
    command: &mut tokio::process::Command,
    address: Option<&str>,
    profile: Option<&str>,
    env: &BTreeMap<String, String>,
) {
    // Never hand the conductor's own credentials to what it runs.
    let own = format!("{}_", brand::ENV_PREFIX);
    for (name, _) in std::env::vars() {
        if name.starts_with(&own) {
            command.env_remove(name);
        }
    }
    if let Some(program) = program() {
        command.env(PROGRAM_ENV, program);
    }
    if let Some(address) = address {
        command.env(ADDRESS_ENV, address);
    }
    command.env_remove(ENVIRONMENT_ENV);
    if let Some(profile) = profile {
        command.env(PROFILE_ENV, profile);
    }
    command.envs(env);
}

impl Launch {
    pub fn start(&self) -> Result<Proc> {
        if self.script.is_none() && self.argv.is_empty() {
            return Err(Error::internal(
                "there is nothing to start: this program could not say where it is, to run its own command line",
            ));
        }
        // What could not be stopped afterwards is not started.
        crate::group::can().map_err(Error::unavailable)?;
        fs::create_dir_all(&self.dir)?;
        let path = |suffix: &str| self.dir.join(format!("{}.{suffix}", self.tag));
        let (out, err, exit) = (path("out"), path("err"), path("exit"));
        for stale in [&out, &err, &exit, &offset_path(&out), &offset_path(&err)] {
            let _ = fs::remove_file(stale);
        }
        let open = |path: &Path| OpenOptions::new().create(true).append(true).open(path);

        // A user's own script is one word, which whatever starts it has
        // their system's shell read ([`crate::script`]): one that shell
        // cannot be given is refused before anything is started.
        if let Some(why) = self.script.as_deref().and_then(crate::script::refused) {
            return Err(Error::invalid(why));
        }
        let argv = match &self.script {
            Some(script) => vec![script.clone()],
            None => self.argv.clone(),
        };
        let unsay = self.handed.iter().map(|(variable, name)| (variable.clone(), crate::unsaid::mark(name)));
        let script = self.script.is_some();
        let asked = Asked { exit: exit.clone(), told: None, reads: None, unsay: unsay.collect(), script };
        let mut command = match (self.wraps, &self.script) {
            // Where this program cannot say where it is, nothing is
            // started: there would be nothing to say how the program
            // ended, and one handed a secret would write it down as it
            // liked.
            (Wraps::Program, _) => {
                let own = crate::wrapped::own().map_err(Error::unavailable)?;
                crate::wrapped::command(own, &asked, &argv)
            }
            // As the wrapper starts it.
            (Wraps::Here, Some(script)) => crate::script::command(script).map_err(Error::invalid)?.into(),
            (Wraps::Here, None) => {
                // As the wrapper finds it: by the file its name comes to.
                let path = self.env.get("PATH").map(std::ffi::OsStr::new);
                let file = crate::named::program(std::ffi::OsStr::new(&argv[0]), path);
                let mut command = tokio::process::Command::new(file);
                command.args(&argv[1..]);
                command
            }
        };
        command.current_dir(&self.cwd);
        if self.clear_env {
            command.env_clear();
            command.envs(cleared_env(cfg!(windows), |name| std::env::var(name).ok()));
        }
        dress(&mut command, self.address.as_deref(), self.profile.as_deref(), &self.env);
        let not_started = |err: std::io::Error| {
            Error::from(err).context(format!("could not start a process in {}", self.cwd.display()))
        };
        let (pid, started, held) = match self.wraps {
            Wraps::Program => {
                command.stdout(open(&out)?).stderr(open(&err)?);
                let child = command.spawn().map_err(not_started)?;
                let pid = child
                    .id()
                    .ok_or_else(|| Error::internal("the process exited before it could be tracked"))?;
                crate::group::wrapped(pid, &exit);
                (pid, identity(pid).unwrap_or(0), Held::Child(Box::new(child)))
            }
            Wraps::Here => {
                let unsaid = asked.unsaid(|variable| self.env.get(variable).cloned());
                let started = crate::here::start(command.as_std_mut(), &asked, &out, &err, unsaid)
                    .map_err(not_started)?;
                (started.pid, started.started, Held::Here(started.how))
            }
        };
        Ok(Proc { pid, started, out, err, exit, held, changes: None, stopping: false })
    }
}

impl Proc {
    /// One this conductor has just started some other way than
    /// [`Launch::start`], to be followed as one of those is. Only a program
    /// that goes on without the daemon is started so (`kept::Process`).
    pub(crate) fn of(
        pid: u32,
        started: u64,
        out: PathBuf,
        err: PathBuf,
        exit: PathBuf,
        child: tokio::process::Child,
    ) -> Proc {
        Proc {
            pid,
            started,
            out,
            err,
            exit,
            held: Held::Child(Box::new(child)),
            changes: None,
            stopping: false,
        }
    }

    /// One a thread of this process has just started and waits for
    /// ([`crate::here`]).
    pub(crate) fn here(started: crate::here::Started, out: PathBuf, err: PathBuf, exit: PathBuf) -> Proc {
        let held = Held::Here(started.how);
        Proc {
            pid: started.pid,
            started: started.started,
            out,
            err,
            exit,
            held,
            changes: None,
            stopping: false,
        }
    }

    /// What is needed to find this process again after a restart.
    pub fn saved(&self) -> Value {
        json!({ "pid": self.pid, "started": self.started, "out": self.out, "err": self.err, "exit": self.exit })
    }

    /// A process started by an earlier conductor.
    pub fn adopt(saved: &Value, watch: &Watch) -> Option<Proc> {
        let path = |key: &str| saved[key].as_str().map(PathBuf::from);
        let (out, err, exit) = (path("out")?, path("err")?, path("exit")?);
        let changes = watch.files(&[&out, &err, &exit]);
        let (pid, started) = (saved["pid"].as_u64()? as u32, saved["started"].as_u64()?);
        // Where a thread of this process waits for it, the thread says
        // when it has ended, as it did to the conductor before.
        let held = crate::here::found(pid, started).map_or(Held::Adopted, Held::Here);
        if matches!(held, Held::Adopted) {
            // A wrapper, which this conductor may be asked to stop.
            crate::group::wrapped(pid, &exit);
        }
        Some(Proc { pid, started, out, err, exit, held, changes: Some(changes), stopping: false })
    }

    /// Whether the process (this one, not a later one with its pid) is
    /// running: the wrapper, which is there until it has said how its
    /// program ended.
    pub fn is_alive(&self) -> bool {
        identity(self.pid).is_some_and(|started| self.started == 0 || started == self.started)
            || crate::here::saying(self.pid, self.started)
    }

    /// Waits for the process to end. The exit code is `None` when it was
    /// killed by a signal or its status could not be recovered: unknown, which
    /// is not the same as zero.
    pub async fn wait(&mut self) -> Option<i32> {
        match &mut self.held {
            // The wrapper ends with the code its program ended with, and
            // only once what the program said last is in its files.
            Held::Child(child) => match child.wait().await {
                Ok(status) => status.code().or_else(|| self.recorded_exit()),
                Err(_) => self.recorded_exit(),
            },
            Held::Here(how) => {
                let said = how.wait_for(|how| how.is_some()).await.map(|how| how.flatten());
                said.unwrap_or_else(|_| self.recorded_exit())
            }
            Held::Adopted => loop {
                // The exit code is written last, so once it can be read the
                // process is over, whatever is still winding down.
                if let Some(code) = self.recorded_exit() {
                    return Some(code);
                }
                if !self.is_alive() {
                    return self.recorded_exit();
                }
                let heard = async {
                    match &mut self.changes {
                        Some(changes) => changes.next().await,
                        None => std::future::pending().await,
                    }
                };
                let look = if self.stopping { STOPPING_LOOK } else { ADOPTED_LOOK };
                tokio::select! {
                    () = heard => {}
                    () = tokio::time::sleep(look) => {}
                }
            },
        }
    }

    fn recorded_exit(&self) -> Option<i32> {
        fs::read_to_string(&self.exit).ok()?.trim().parse().ok()
    }

    /// Asks the process and everything it started to stop; `force` does not ask.
    pub fn kill(&self, force: bool) {
        // One the system will not stop is found still there by whoever
        // waits for it to end.
        let _ = crate::group::stop(self.pid, force);
    }

    /// Stops the process: politely, then after `grace` without asking.
    pub async fn stop(&mut self, grace: Duration) {
        self.stopping = true;
        self.kill(false);
        if tokio::time::timeout(grace, self.wait()).await.is_err() {
            self.kill(true);
            // Where a wrapper is stopped by a word it looks for, one that
            // did not end on it is no longer looking. One this conductor
            // started is ended by the hold it has on it, and its group
            // with it; one it found again cannot be. Compiled, never run.
            #[cfg(windows)]
            if let Held::Child(child) = &mut self.held {
                let _ = child.start_kill();
            }
            let _ = tokio::time::timeout(Duration::from_secs(2), self.wait()).await;
        }
    }

    /// Removes the files the process wrote, and a word that was left for
    /// its wrapper: it has ended, and nothing more is kept of its group.
    pub fn clean(&self) {
        let word = crate::group::word(&self.exit, self.pid);
        for path in
            [&self.out, &self.err, &self.exit, &offset_path(&self.out), &offset_path(&self.err), &word]
        {
            let _ = fs::remove_file(path);
        }
        crate::group::ended(self.pid);
    }
}

/// The start time of the live process with this pid, or `None` if there is
/// none (a zombie counts as none).
#[cfg(target_os = "linux")]
pub(crate) fn identity(pid: u32) -> Option<u64> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // The command name is in parentheses and may contain anything, so parse
    // from the last `)`.
    let rest = stat.rsplit_once(')')?.1;
    let mut fields = rest.split_whitespace();
    let state = fields.next()?;
    if state == "Z" || state == "X" {
        return None;
    }
    // `starttime` is the 22nd field overall: the 20th after the state.
    fields.nth(18)?.parse().ok()
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn identity(pid: u32) -> Option<u64> {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
    let mut system = System::new().ok()?;
    let pid = Pid::from_u32(pid);
    system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, ProcessRefreshKind::nothing());
    system.process(pid).map(|process| process.start_time())
}

fn offset_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".off");
    PathBuf::from(name)
}

/// Reads a growing file line by line, remembering how far it got on disk so a
/// later conductor carries on from the same place.
pub(crate) struct Tail {
    path: PathBuf,
    stream: char,
    offset: u64,
    partial: Vec<u8>,
}

/// The most that is read from a file in one go.
const READ_LIMIT: u64 = 1024 * 1024;
/// Output files of long-lived processes are emptied once they pass this size
/// and everything in them has been read.
const TRUNCATE_AT: u64 = 16 * 1024 * 1024;

impl Tail {
    pub fn new(path: &Path, stream: char) -> Tail {
        let offset =
            fs::read_to_string(offset_path(path)).ok().and_then(|text| text.trim().parse().ok()).unwrap_or(0);
        Tail { path: path.to_path_buf(), stream, offset, partial: Vec::new() }
    }

    /// Hands every complete new line to `line`. With `flush`, also a last
    /// line that has no newline yet.
    pub fn read(&mut self, flush: bool, line: &mut dyn FnMut(char, &str)) {
        let Ok(mut file) = File::open(&self.path) else { return };
        let length = file.metadata().map_or(0, |meta| meta.len());
        if length < self.offset {
            // The file was emptied under us.
            self.offset = 0;
        }
        if length > self.offset && file.seek(SeekFrom::Start(self.offset)).is_ok() {
            let mut chunk = Vec::new();
            if file.take(READ_LIMIT).read_to_end(&mut chunk).is_ok() {
                self.offset += chunk.len() as u64;
                self.partial.extend_from_slice(&chunk);
                while let Some(at) = self.partial.iter().position(|byte| *byte == b'\n') {
                    let rest = self.partial.split_off(at + 1);
                    let text = String::from_utf8_lossy(&self.partial[..at]).into_owned();
                    self.partial = rest;
                    line(self.stream, text.trim_end_matches('\r'));
                }
                let _ = fs::write(offset_path(&self.path), self.offset.to_string());
            }
        }
        if flush && !self.partial.is_empty() {
            let text = String::from_utf8_lossy(&self.partial).into_owned();
            self.partial.clear();
            line(self.stream, &text);
        }
        // The writer appends, so emptying a fully read file is safe: its next
        // write lands at the new end.
        if self.offset >= TRUNCATE_AT
            && self.partial.is_empty()
            && self.offset == length
            && OpenOptions::new().write(true).open(&self.path).and_then(|file| file.set_len(0)).is_ok()
        {
            self.offset = 0;
            let _ = fs::write(offset_path(&self.path), "0");
        }
    }
}

/// Kills a process tree when dropped, unless disarmed. This is what makes a
/// cancelled or timed-out step leave nothing behind. When the conductor
/// itself is shutting down the process is left running on purpose, for the
/// next conductor to adopt.
pub(crate) struct KillOnDrop {
    pid: u32,
    engine: conductor::Engine,
    armed: bool,
}

impl KillOnDrop {
    pub fn new(pid: u32, engine: conductor::Engine) -> Self {
        Self { pid, engine, armed: true }
    }

    pub fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        if self.armed && !self.engine.is_halting() {
            let _ = crate::group::stop(self.pid, true);
        }
    }
}
