//! A program started so that it can go on without the daemon, for whoever
//! else starts programs here: every agent's turn, whether or not it is to
//! go on, which is decided as the daemon stops ([`Process::cut`] ends one
//! that is not, and says so).
//!
//! It is started as everything in [`crate::proc`] is: in a group of its
//! own, writing to files and not to pipes, leaving the code it ended with
//! in a file, and known by its number and the moment it was started. What
//! is added here is what a turn needs and a service does not: what it is
//! asked goes in from a file, so that nothing is lost of it if the daemon
//! stops as it is being handed over, and what it prints is read from the
//! first line each time it is taken up, since whoever reads it keeps what
//! it has understood so far in memory and nowhere else.
//!
//! Such a program is found again by its number together with the moment
//! it was started, which only the daemon can write down, and only once it
//! is started. So what is started first is a wrapper that waits
//! ([`crate::wrapped`], this program): it starts its program when the
//! daemon has written the moment down and said so, down a pipe that only
//! the daemon holds. A daemon that is gone before then has let go of the
//! pipe, and the wrapper ends having started nothing: there is no moment
//! in which a program is at work that the next daemon could not find.
//!
//! The files are in the daemon's own folder for what it runs, and are
//! found again by the name they were given there, never by a path that was
//! written down: a copy of a factory holds no such files, and so takes up
//! nothing that another factory started.

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::json;

use crate::proc::Proc;
use crate::watch::{Changes, Watch};
use crate::wrapped::{Asked, Wraps};

/// Where one such program's files are: in `dir`, each called `tag` and
/// what it is.
#[derive(Clone, Debug)]
pub struct Files {
    dir: PathBuf,
    tag: String,
    /// What wraps the program that is started under them.
    wraps: Wraps,
}

impl Files {
    pub fn new(dir: impl Into<PathBuf>, tag: &str) -> Files {
        Files { dir: dir.into(), tag: tag.replace(['/', '\\', ':'], "_"), wraps: Wraps::Program }
    }

    /// The same, for a program that is wrapped as `wraps` says.
    pub fn wrapped(self, wraps: Wraps) -> Files {
        Files { wraps, ..self }
    }

    /// What the wrapper of the program started under these is asked.
    fn asked(&self, unsay: &[(String, String)]) -> Asked {
        Asked {
            exit: self.path("exit"),
            told: Some(self.path("pid")),
            reads: Some(self.path("in")),
            unsay: unsay.to_vec(),
            script: false,
        }
    }

    fn path(&self, what: &str) -> PathBuf {
        self.dir.join(format!("{}.{what}", self.tag))
    }

    fn all(&self) -> [PathBuf; 4] {
        [self.path("in"), self.path("out"), self.path("err"), self.path("exit")]
    }
}

/// Whether a program can be kept so on this system: it takes a wrapper
/// that outlives the daemon and says how the program ended, and a group
/// that a later daemon can stop.
///
/// On Windows that is so as far as it is written, and none of it has
/// been run there: the wrapper holds its program's group itself and is
/// started with a console of its own, so nothing of the daemon's ending
/// reaches it, and a later daemon stops it by the word it leaves where
/// these files are ([`crate::group`]).
pub fn can() -> bool {
    crate::group::can().is_ok()
}

/// A file only whoever runs the daemon reads, made empty.
fn private(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

/// The command that starts `program` so, to be begun with [`begin`].
/// `unsay` is what its files are written without: each variable of its
/// environment that holds something it was handed, with what that is
/// written as in its place. What else the program is started with (more
/// arguments, its environment, where it is run) is added by the caller to
/// what this hands back. What it reads on its standard input is handed in
/// with [`hand_in`] before it is begun.
///
/// Where all this is written is not the program's to know: it is said to
/// the wrapper, in the wrapper's own arguments, and nowhere in what the
/// program is started with.
pub fn wrapper(
    files: &Files,
    unsay: &[(String, String)],
    program: &[OsString],
) -> std::io::Result<tokio::process::Command> {
    let own = match files.wraps {
        Wraps::Program => Some(crate::wrapped::own().map_err(std::io::Error::other)?),
        Wraps::Here => None,
    };
    made(own, files, unsay, program)
}

/// The same, wrapped by the copy of the factory's program at `own`,
/// whatever program this is: for whoever is handed that program (a test
/// of the wrapper itself).
pub fn wrapper_by(
    own: &Path,
    files: &Files,
    unsay: &[(String, String)],
    program: &[OsString],
) -> std::io::Result<tokio::process::Command> {
    made(Some(own), &files.clone().wrapped(Wraps::Program), unsay, program)
}

fn made(
    own: Option<&Path>,
    files: &Files,
    unsay: &[(String, String)],
    program: &[OsString],
) -> std::io::Result<tokio::process::Command> {
    if !can() {
        return Err(std::io::Error::other(
            "a program cannot be kept going without the daemon on this system",
        ));
    }
    let Some(first) = program.first() else {
        return Err(std::io::Error::other("there is no program to start"));
    };
    fs::create_dir_all(&files.dir)?;
    for stale in files.all().into_iter().chain([files.path("pid"), files.path("at"), files.path("cut")]) {
        let _ = fs::remove_file(stale);
    }
    let (out, err) = (private(&files.path("out"))?, private(&files.path("err"))?);
    // What the program reads is there from the first, and is nothing
    // until something is handed in.
    private(&files.path("in"))?;
    match own {
        Some(own) => {
            let mut command = crate::wrapped::command(own, &files.asked(unsay), program);
            command.stdout(out).stderr(err);
            Ok(command)
        }
        // The thread that stands in for the wrapper is given the program
        // as it is begun ([`begin`]).
        None => {
            let mut command = tokio::process::Command::new(first);
            command.args(&program[1..]);
            Ok(command)
        }
    }
}

/// Begins what [`wrapper`] made: the wrapper is started, the moment it
/// began is written down, and only then is it told to start its program
/// ([`Process::started`]).
pub fn begin(
    command: &mut tokio::process::Command,
    files: &Files,
    unsay: &[(String, String)],
) -> std::io::Result<Process> {
    match files.wraps {
        Wraps::Program => {
            let child = command.spawn()?;
            Process::started(child, files)
                .ok_or_else(|| std::io::Error::other("its wrapper ended before it could be followed"))
        }
        Wraps::Here => {
            let asked = files.asked(unsay);
            let unsaid = {
                let said = |variable: &str| {
                    let set = command.as_std().get_envs().find(|(name, _)| *name == variable);
                    match set {
                        Some((_, value)) => value.map(|value| value.to_string_lossy().into_owned()),
                        None => std::env::var(variable).ok(),
                    }
                };
                asked.unsaid(said)
            };
            let (out, err, exit) = (files.path("out"), files.path("err"), files.path("exit"));
            let started = crate::here::start(command.as_std_mut(), &asked, &out, &err, unsaid)?;
            // By what a later factory in this process finds it.
            fs::write(files.path("pid"), started.pid.to_string())?;
            fs::write(files.path("at"), started.started.to_string())?;
            Ok(Process { proc: Proc::here(started, out, err, exit), files: files.clone() })
        }
    }
}

/// Gives the program a [`wrapper`] starts what it reads on its standard
/// input: from a file, so that none of it is lost if the daemon stops as
/// it is being handed over. Before the wrapper is started.
pub fn hand_in(files: &Files, input: &str) -> std::io::Result<()> {
    private(&files.path("in"))?.write_all(input.as_bytes())
}

/// The pipe a wrapper waits at, as a file to write to here and now: a
/// hold of this function's own on it, let go when the word is written.
#[cfg(unix)]
fn ours(waits: &tokio::process::ChildStdin) -> Option<File> {
    use std::os::fd::AsFd;
    Some(File::from(waits.as_fd().try_clone_to_owned().ok()?))
}

/// The same where a pipe is held by a handle. Compiled, and never run.
#[cfg(windows)]
fn ours(waits: &tokio::process::ChildStdin) -> Option<File> {
    use std::os::windows::io::AsHandle;
    Some(File::from(waits.as_handle().try_clone_to_owned().ok()?))
}

/// Nothing is kept so where a group cannot be stopped ([`can`]).
#[cfg(not(any(unix, windows)))]
fn ours(_waits: &tokio::process::ChildStdin) -> Option<File> {
    None
}

/// One such program: started by this daemon, or found again by it.
pub struct Process {
    proc: Proc,
    files: Files,
}

impl Process {
    /// What was just started with [`wrapper`]: the moment it began is
    /// written down, and only then is the wrapper told to start its
    /// program. `None` where either could not be done, and then nothing
    /// was started: the wrapper ends when the pipe it waits at is let go.
    pub fn started(mut child: tokio::process::Child, files: &Files) -> Option<Process> {
        let pid = child.id()?;
        let at = crate::proc::identity(pid).unwrap_or(0);
        // With its number, the moment it was started is what says a
        // process found later is this one and not another with its number.
        fs::write(files.path("at"), at.to_string()).ok()?;
        // Said at once and not waited on: a word into a pipe nobody has
        // written to. Whatever stops the daemon between the two leaves the
        // pipe closed with nothing said.
        let waits = child.stdin.take()?;
        ours(&waits)?.write_all(format!("{}\n", crate::wrapped::GO).as_bytes()).ok()?;
        drop(waits);
        let (out, err, exit) = (files.path("out"), files.path("err"), files.path("exit"));
        // A wrapper, which this daemon may be asked to stop.
        crate::group::wrapped(pid, &exit);
        Some(Process { proc: Proc::of(pid, at, out, err, exit, child), files: files.clone() })
    }

    /// One an earlier daemon started under these files. `None` where it
    /// left nothing to be found by: it was never this factory's, or it
    /// never came to be started, or the daemon stopped before it had
    /// written down the moment it was, and then its wrapper started
    /// nothing. A number alone is not gone by: by the time it is read it
    /// may be some other program's, and what is found here may be stopped.
    pub fn found(files: &Files, watch: &Watch) -> Option<Process> {
        let number = |what: &str| fs::read_to_string(files.path(what)).ok()?.trim().parse::<u64>().ok();
        let (out, err, exit) = (files.path("out"), files.path("err"), files.path("exit"));
        let saved = json!({
            "pid": number("pid")?, "started": number("at")?, "out": out, "err": err, "exit": exit,
        });
        Some(Process { proc: Proc::adopt(&saved, watch)?, files: files.clone() })
    }

    pub fn pid(&self) -> u32 {
        self.proc.pid
    }

    /// Whether it has said how it ended.
    pub fn has_ended(&self) -> bool {
        fs::read_to_string(self.files.path("exit")).is_ok_and(|code| code.trim().parse::<i32>().is_ok())
    }

    /// Whether it is still there: this one, and not another that has its
    /// number now.
    fn is_alive(&self) -> bool {
        self.proc.is_alive()
    }

    /// Whether it is there to be taken up: still at work, or ended having
    /// said how. One that is gone without a word is neither.
    ///
    /// Whether it is alive is looked at first, and what it said of its
    /// ending after. It says how it ended before it goes: so one that was
    /// alive at the first look is there whatever the second finds, and one
    /// that was not has said by the second all it ever will. Looked at the
    /// other way round, a program that ended between the two looks had
    /// said nothing at the first and was gone at the second, and was taken
    /// for one that was cut off, with its answer lying there.
    pub fn is_there(&self) -> bool {
        let alive = self.is_alive();
        self.has_ended() || alive
    }

    /// Waits for it to end: the code it ended with, or `None` where it was
    /// killed outright and left none.
    pub async fn wait(&mut self) -> Option<i32> {
        self.proc.wait().await
    }

    /// Stops it and everything it started: asked, and then not asked.
    pub async fn stop(&mut self) {
        self.proc.stop(Duration::from_secs(2)).await;
    }

    /// Ends it and everything it started, at once and with nothing asked,
    /// because the daemon is stopping and it is not to go on without the
    /// daemon; and says so beside its files, so that the next daemon does
    /// not take what it left for an answer ([`Process::was_cut`]). Asked
    /// where nothing can be waited for (whatever held it is being let go
    /// of), so its wrapper is not waited for: whoever takes it up waits.
    pub fn cut(&self) {
        let _ = fs::write(self.files.path("cut"), self.proc.pid.to_string());
        let _ = crate::group::stop(self.proc.pid, true);
    }

    /// Whether a daemon that stopped ended it ([`Process::cut`]): said of
    /// this process, by its number, and not of another that a later turn
    /// under the same files has.
    pub fn was_cut(&self) -> bool {
        fs::read_to_string(self.files.path("cut")).is_ok_and(|pid| pid.trim() == self.proc.pid.to_string())
    }

    /// Ends whatever it started and left behind it, once it has itself ended.
    pub fn sweep(&self) {
        let _ = crate::group::stop(self.proc.pid, true);
    }

    /// It has ended, or was stopped: nothing more is kept of its group,
    /// and a word that was left for its wrapper is taken away. Its files
    /// stay, for [`clean`] once whoever it was for has kept that it is
    /// over: they are what a later daemon takes it up by.
    pub fn forget(&self) {
        let _ = fs::remove_file(crate::group::word(&self.files.path("exit"), self.proc.pid));
        crate::group::ended(self.proc.pid);
    }

    /// What it has printed, read from the first line.
    pub fn said(&self) -> Lines {
        Lines { path: self.files.path("out"), offset: 0, partial: Vec::new() }
    }

    /// The end of what it printed on standard error, as text.
    pub fn errors(&self, most: usize) -> String {
        let bytes = fs::read(self.files.path("err")).unwrap_or_default();
        String::from_utf8_lossy(&bytes[bytes.len().saturating_sub(most)..]).into_owned()
    }

    /// Told when it prints more or ends; looked for as well, where the
    /// system will not say.
    pub fn changes(&self, watch: &Watch) -> Changes {
        let [_, out, _, exit] = self.files.all();
        watch.files(&[&out, &exit]).or_looking()
    }
}

/// Takes the files of a program that is gone, by the name they had, and
/// a word left for its wrapper by its number: only once whoever it was
/// for has kept that it is over, since the files are the one way a later
/// daemon has of taking it up ([`Process::found`]).
pub fn clean(files: &Files) {
    if let Some(pid) =
        fs::read_to_string(files.path("pid")).ok().and_then(|pid| pid.trim().parse::<u32>().ok())
    {
        let _ = fs::remove_file(crate::group::word(&files.path("exit"), pid));
    }
    for path in files.all().into_iter().chain([files.path("pid"), files.path("at"), files.path("cut")]) {
        let _ = fs::remove_file(path);
    }
}

/// A growing file read line by line from its start. Unlike a service's
/// output it is never emptied and keeps no place on disk: it is one turn's,
/// and is read whole again by whoever takes the turn up.
pub struct Lines {
    path: PathBuf,
    offset: u64,
    partial: Vec<u8>,
}

impl Lines {
    /// Every whole line since the last asking. With `all`, also a last
    /// line that was never ended.
    pub fn more(&mut self, all: bool) -> Vec<String> {
        let mut lines = Vec::new();
        if let Ok(mut file) = File::open(&self.path)
            && file.seek(SeekFrom::Start(self.offset)).is_ok()
        {
            let mut chunk = Vec::new();
            if file.read_to_end(&mut chunk).is_ok() {
                self.offset += chunk.len() as u64;
                self.partial.extend_from_slice(&chunk);
            }
        }
        while let Some(at) = self.partial.iter().position(|byte| *byte == b'\n') {
            let rest = self.partial.split_off(at + 1);
            lines.push(String::from_utf8_lossy(&self.partial[..at]).trim_end_matches('\r').to_string());
            self.partial = rest;
        }
        if all && !self.partial.is_empty() {
            lines.push(String::from_utf8_lossy(&self.partial).into_owned());
            self.partial.clear();
        }
        lines
    }
}
