//! The wrapper: what the factory starts in place of a program, to start
//! the program and say how it ended.
//!
//! A program the factory starts has to be found again by a daemon that is
//! not its parent, and has to say how it ended with no daemon there to be
//! told. So what the daemon starts is a wrapper, at the head of a group
//! of its own ([`crate::group`]), which:
//!
//! * starts the program it was handed, from a list of arguments and with
//!   no shell, in its own group, with its own environment and output; or,
//!   told that what it was handed is a user's own script
//!   ([`Asked::script`]), has their system's shell read it
//!   ([`crate::script`]);
//! * waits for it, and writes the code it ended with where the daemon
//!   looks ([`Asked::exit`]);
//! * goes on when the group is asked to stop, until the program has
//!   ended: a process is over when its wrapper has ended, so a wrapper
//!   that went at once would have the program taken for stopped while it
//!   was still putting its things away, and nothing would say how it
//!   ended;
//! * where the program was handed something ([`Asked::unsay`]), reads
//!   what it prints and writes it down without that
//!   ([`crate::unsaid`]), so that what reaches the disk is already
//!   without it whether or not a daemon is there to read;
//! * for a program that is to be found again by the moment it was started
//!   ([`crate::kept`], [`Asked::told`]), says which process it is, and
//!   starts nothing until the daemon has written that moment down and
//!   said so down a pipe only the daemon holds. A daemon that is gone
//!   before then has let go of the pipe, and the wrapper ends having
//!   started nothing.
//!
//! The wrapper is this program, started with the word [`WRAPS`]
//! ([`wraps`] is what answers it). It was a line of shell, which is one
//! system's, and which a daemon had to find. This is found by where this
//! program is, never along a path, and is the same on every system this
//! program is built for. It has been run on Linux only.
//!
//! On Windows (compiled, and never run) the wrapper is also what holds
//! its program's group ([`crate::group::heads`]): everything it starts
//! ends when it does. It is stopped by being left word, which it looks
//! for, and ends at once: nothing there asks a program to stop.
//!
//! A factory that is its own test cannot start itself as anything: the
//! program is the test's. There the wrapper is a thread of the test's
//! process that is not the factory's ([`Wraps::Here`], [`crate::here`]).

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

use crate::unsaid::Unsaid;

/// The word this program is started with to be a wrapper.
pub const WRAPS: &str = "wraps";

/// Whether this process is the factory's own program, which answers
/// [`WRAPS`]: said by that program as it starts ([`answers`]).
static ANSWERS: AtomicBool = AtomicBool::new(false);

/// Says that this process is the factory's own program: started again
/// with [`WRAPS`], it is the wrapper. Said by the program's `main`, once,
/// before anything else. A process that has not said so is some other
/// program that holds a factory inside it (a test's), and is never
/// started as a wrapper: it would not know the word.
pub fn answers() {
    ANSWERS.store(true, Ordering::Relaxed);
}

/// Where this program is, to be started as the wrapper of what it
/// starts; or, in a sentence, why it cannot be, with what to say
/// instead. Asked before anything is started, so that a factory inside a
/// program that is not the factory's own is told at once, and not by
/// what it started never ending.
pub(crate) fn own() -> Result<&'static Path, String> {
    if !ANSWERS.load(Ordering::Relaxed) {
        return Err(format!(
            "this factory is inside a program that is not its own (a test's), which cannot be started again as \
             the wrapper of a program it starts (the word `{WRAPS}`); nothing was started. Such a factory says \
             that its own process stands in for the wrapper, before anything is started: \
             `Hosting::stands_between()`, or `stands_between: true` in a node's `Config`"
        ));
    }
    crate::proc::program().ok_or_else(|| {
        "this program could not say where it is, to start a program for itself and say how that ended; nothing \
         was started"
            .to_string()
    })
}

/// What the daemon says down the pipe of a wrapper that waits to be told.
pub(crate) const GO: &str = "go";

/// What wraps the programs a factory starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Wraps {
    /// This program, started as a wrapper ([`wraps`]): it goes on with
    /// the program when the daemon does not.
    #[default]
    Program,
    /// A thread of this process that is not the factory's
    /// ([`crate::here`]). Only for a factory that is its own test: there
    /// this program is the test's, which cannot be started as anything
    /// else. It goes on when the factory is stopped and another is
    /// started in the same process, as a wrapper goes on without its
    /// daemon, and not when the process ends.
    Here,
}

/// What a wrapper is asked beside the program it starts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Asked {
    /// Where it says how the program ended: the code, as text.
    pub exit: PathBuf,
    /// For one that waits to be told before it starts its program: where
    /// it says which process it is.
    pub told: Option<PathBuf>,
    /// What the program reads on its standard input, where that is a file.
    pub reads: Option<PathBuf>,
    /// What its output is written without: each variable of its
    /// environment that holds something it was handed, and what that is
    /// written as in its place. The values are not here: they are read
    /// from the environment, which is the program's.
    pub unsay: Vec<(String, String)>,
    /// What it is handed is no program with its arguments but one word,
    /// a script of its user's own, which their system's shell is to
    /// read. It reaches the wrapper as an argument, whole; how it is
    /// then given to the shell is the system's ([`crate::script`]), and
    /// on Windows is not as an argument.
    pub script: bool,
}

/// The word that says what follows `--` is a user's own script.
const SCRIPT: &str = "--script";

impl Asked {
    /// The words a wrapper is started with: [`WRAPS`], what it is asked,
    /// and `--`. The program and its arguments follow, or the script.
    pub fn words(&self) -> Vec<OsString> {
        let mut words: Vec<OsString> = vec![WRAPS.into(), "--exit".into(), self.exit.clone().into()];
        for (flag, file) in [("--told", &self.told), ("--reads", &self.reads)] {
            if let Some(file) = file {
                words.extend([flag.into(), file.clone().into()]);
            }
        }
        for (variable, said) in &self.unsay {
            words.extend(["--without".into(), format!("{variable}={said}").into()]);
        }
        if self.script {
            words.push(SCRIPT.into());
        }
        words.push("--".into());
        words
    }

    /// The same read back, with the program and its arguments: `None`
    /// for words no daemon wrote.
    fn read(words: &[OsString]) -> Option<(Asked, &[OsString])> {
        let mut asked = Asked::default();
        let mut words = words;
        loop {
            match words {
                [end, program @ ..] if end == "--" => {
                    // A script is one word, and nothing after it.
                    let fits = if asked.script { program.len() == 1 } else { !program.is_empty() };
                    let whole = !asked.exit.as_os_str().is_empty() && fits;
                    return whole.then_some((asked, program));
                }
                [script, rest @ ..] if script == SCRIPT => {
                    asked.script = true;
                    words = rest;
                }
                [flag, value, rest @ ..] => {
                    match flag.to_str()? {
                        "--exit" => asked.exit = value.into(),
                        "--told" => asked.told = Some(value.into()),
                        "--reads" => asked.reads = Some(value.into()),
                        "--without" => {
                            let (variable, said) = value.to_str()?.split_once('=')?;
                            asked.unsay.push((variable.to_string(), said.to_string()));
                        }
                        _ => return None,
                    }
                    words = rest;
                }
                _ => return None,
            }
        }
    }

    /// What the program's output is written without, with the values
    /// `value_of` gives for the variables named.
    pub(crate) fn unsaid(&self, value_of: impl Fn(&str) -> Option<String>) -> Unsaid {
        Unsaid::of(self.unsay.iter().filter_map(|(variable, said)| Some((value_of(variable)?, said.clone()))))
    }
}

/// The command that starts `program` wrapped, by the copy of this program
/// at `own`: at the head of a group of its own, with what it reads being
/// nothing, or for one that waits to be told the pipe it is told down.
/// Its environment, where it runs and where its output goes are the
/// program's, and are whoever calls this to say.
pub fn command(own: &Path, asked: &Asked, program: &[impl AsRef<OsStr>]) -> tokio::process::Command {
    let mut command = tokio::process::Command::new(own);
    command.args(asked.words()).args(program);
    command.stdin(if asked.told.is_some() { Stdio::piped() } else { Stdio::null() });
    crate::group::leads(command.as_std_mut());
    command
}

/// How a program ended.
pub(crate) struct Ended {
    /// The code it ended with; for one a signal ended, what a shell
    /// would say of it (128 and the signal's number).
    pub code: i32,
    /// It was ended without being asked.
    pub killed: bool,
}

/// How a program that was waited for ended.
pub(crate) fn how(waited: std::io::Result<ExitStatus>) -> Ended {
    let Ok(status) = waited else { return Ended { code: 1, killed: false } };
    #[cfg(unix)]
    let signal = std::os::unix::process::ExitStatusExt::signal(&status);
    #[cfg(not(unix))]
    let signal: Option<i32> = None;
    Ended {
        code: status.code().or(signal.map(|signal| 128 + signal)).unwrap_or(1),
        killed: signal == Some(9),
    }
}

/// Starts a program.
///
/// One that is being replaced at this very moment (an agent's
/// command-line tool updating itself) cannot be started for an instant;
/// that is tried again rather than reported.
pub(crate) fn start(command: &mut Command) -> std::io::Result<Child> {
    let mut tries = 0;
    loop {
        match command.spawn() {
            Err(err) if err.kind() == std::io::ErrorKind::ExecutableFileBusy && tries < 20 => {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            started => return started,
        }
    }
}

/// The code a program that could not be started is said to have ended
/// with, as a shell says it: 127 for one that is not there, 126 for one
/// that is and cannot be run.
pub(crate) fn unstarted(err: &std::io::Error) -> i32 {
    if err.kind() == std::io::ErrorKind::NotFound { 127 } else { 126 }
}

/// Passes on what a program prints, a line at a time, without what it
/// was handed.
fn pass(from: impl Read, mut to: impl Write, unsaid: &Unsaid) {
    let mut from = BufReader::new(from);
    let mut line = Vec::new();
    while from.read_until(b'\n', &mut line).is_ok_and(|read| read > 0) {
        let said = unsaid.from(&String::from_utf8_lossy(&line));
        let _ = to.write_all(said.as_bytes()).and_then(|()| to.flush());
        line.clear();
    }
}

/// Begins passing on what `child` prints, where it was started to print
/// into pipes: what comes back is over when the program, and whatever it
/// left holding its output, has said all it will.
pub(crate) fn passing(
    child: &mut Child,
    unsaid: &Unsaid,
    out: impl Write + Send + 'static,
    err: impl Write + Send + 'static,
) -> Vec<JoinHandle<()>> {
    let mut passing = Vec::new();
    if let Some(said) = child.stdout.take() {
        let unsaid = unsaid.clone();
        passing.push(std::thread::spawn(move || pass(said, out, &unsaid)));
    }
    if let Some(said) = child.stderr.take() {
        let unsaid = unsaid.clone();
        passing.push(std::thread::spawn(move || pass(said, err, &unsaid)));
    }
    passing
}

/// Says how a program ended, where the daemon looks. Written beside and
/// then given its name, so that whoever reads it reads the whole of the
/// code or nothing: half a number is another number.
pub(crate) fn say(exit: &Path, code: i32) {
    let mut beside = exit.as_os_str().to_owned();
    beside.push(".saying");
    let beside = PathBuf::from(beside);
    if std::fs::write(&beside, code.to_string()).and_then(|()| std::fs::rename(&beside, exit)).is_err() {
        let _ = std::fs::write(exit, code.to_string());
    }
}

/// Has this process go on when it is asked to stop, for as long as what
/// comes back is held. Whoever asks a program to stop asks its whole
/// group, and the wrapper leads it. A handler that does nothing is not
/// handed on to the program, as ignoring would be: the program hears the
/// asking as it always did.
#[cfg(unix)]
fn stays() -> impl Sized {
    use tokio::signal::unix::{SignalKind, signal};
    // Hearing a signal takes a runtime to hear it in: the one this is
    // called in where there is one, and otherwise one of its own.
    let own = match tokio::runtime::Handle::try_current() {
        Ok(_) => None,
        Err(_) => tokio::runtime::Builder::new_current_thread().enable_io().build().ok(),
    };
    let heard: Vec<_> = {
        let _within = own.as_ref().map(|runtime| runtime.enter());
        [SignalKind::terminate(), SignalKind::interrupt(), SignalKind::hangup()]
            .into_iter()
            .filter_map(|kind| signal(kind).ok())
            .collect()
    };
    (heard, own)
}

/// Where the system is not Unix there is no asking to go on through:
/// on Windows a group is not asked to stop but ended, by its wrapper
/// ending ([`stopped`]), and the wrapper is started where an interrupt
/// typed at the daemon's terminal does not reach it
/// ([`crate::group::leads`]). Compiled, and never run.
#[cfg(not(unix))]
fn stays() -> impl Sized {}

/// The code a wrapper ends with when it is left word to end, where that
/// is how its group is stopped: what a shell says of a program that was
/// ended without being asked. It says nothing where the daemon looks,
/// as a wrapper ended with its group says nothing.
pub const STOPPED: i32 = 137;

/// Has this process end, and with it the group it holds, when word is
/// left for it at the file beside `exit` ([`crate::group::word`]): how a
/// group is stopped where it cannot be reached by its leader's number.
/// Only there: where a group is a process group, a signal does it.
#[cfg(windows)]
fn stopped(exit: &Path) {
    let word = crate::group::word(exit, std::process::id());
    std::thread::spawn(move || {
        crate::group::hears(&word, crate::group::LOOKS);
        std::process::exit(STOPPED);
    });
}

#[cfg(not(windows))]
fn stopped(_exit: &Path) {}

/// Is the wrapper: `words` are what [`Asked::words`] wrote, without the
/// first, and then the program with its arguments. What comes back is
/// the code this process ends with: the program's own.
pub fn wraps(words: &[OsString]) -> i32 {
    let Some((asked, program)) = Asked::read(words) else {
        eprintln!("`{WRAPS}` is how this program starts another for itself, and is not a command to give it");
        return 2;
    };
    let _stays = stays();
    // Before anything is said or started: what this starts is in its
    // group from the first, and a wrapper that could hold none starts
    // nothing, since nothing could stop what it started.
    let _heads = match crate::group::heads() {
        Ok(heads) => heads,
        Err(why) => {
            eprintln!("could not start `{}`: {why}", program[0].to_string_lossy());
            say(&asked.exit, 126);
            return 126;
        }
    };
    stopped(&asked.exit);
    if let Some(told) = &asked.told {
        // Which process this is, before anything else: with the moment it
        // began, which the daemon writes down, it is what a later daemon
        // finds the program by.
        if std::fs::write(told, std::process::id().to_string()).is_err() {
            return 2;
        }
        // Nothing is started until the daemon has written that down and
        // said so. One that is gone first has let go of the pipe, and
        // nothing was started that nobody could find.
        let mut word = String::new();
        let heard = std::io::stdin().lock().read_line(&mut word);
        if heard.is_err() || word.trim_end() != GO {
            return 0;
        }
    }
    let unsaid = asked.unsaid(|variable| std::env::var(variable).ok());
    let made = match asked.script {
        // A user's own script, which their system's shell reads.
        true => crate::script::command(&program[0].to_string_lossy()),
        // By the file its name comes to, where the system does not find
        // a program by its name alone.
        false => {
            let mut command = Command::new(crate::named::program(&program[0], None));
            command.args(&program[1..]);
            Ok(command)
        }
    };
    let mut command = match made {
        Ok(command) => command,
        Err(why) => {
            eprintln!("{why}");
            say(&asked.exit, 126);
            return 126;
        }
    };
    let file = command.get_program().to_owned();
    if !unsaid.is_empty() {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
    }
    let reads = asked.reads.as_ref().map(File::open).transpose();
    let started = reads.and_then(|reads| {
        if let Some(reads) = reads {
            command.stdin(reads);
        }
        start(&mut command)
    });
    let code = match started {
        Ok(mut child) => {
            let passing = passing(&mut child, &unsaid, std::io::stdout(), std::io::stderr());
            let ended = how(child.wait());
            // Before it is said to be over, what it said last is where
            // it is read: by this, or by whatever it left holding its
            // output.
            passing.into_iter().for_each(|passed| drop(passed.join()));
            ended.code
        }
        Err(err) => {
            eprintln!("{}", crate::named::not_started(&file, &err));
            unstarted(&err)
        }
    };
    say(&asked.exit, code);
    code
}
