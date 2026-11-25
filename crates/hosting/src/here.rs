//! The wrapper where this program cannot be started as one
//! ([`Wraps::Here`](crate::wrapped::Wraps)): a factory that is its own
//! test, whose program is the test's.
//!
//! What stands in for the wrapper is a thread of the process, and not a
//! task of the factory's: it goes on when the factory is stopped and
//! another is started in the same process, as a wrapper goes on without
//! its daemon. It does what a wrapper does with the same code
//! ([`crate::wrapped`]): waits for the program, passes on what it prints
//! without what it was handed, and says how it ended where a daemon
//! looks. Three things are not the wrapper's, and what holds each for the
//! wrapper itself is a test that runs this program
//! (`crates/cli/tests/suite/wrapped.rs`):
//!
//! * There is no process that is the wrapper, so the program leads its
//!   own group and its number is the one written down. A wrapper is
//!   still there, saying how its program ended, in the moment after the
//!   program is gone; here the thread is ([`saying`]), and a look that
//!   falls in that moment is answered by the thread and not by the
//!   program's being gone.
//! * A program ended without being asked is said nothing of. That is
//!   what is done to a group, and a wrapper is in its program's group:
//!   it is gone with it and says nothing. (A wrapper whose program alone
//!   was ended so says 137.)
//! * Nothing waits to be told down a pipe: the moment the program began
//!   is written down once it has begun. No daemon can be gone between
//!   the two, since the daemon is this process.
//!
//! Where a group is something a process holds (Windows), this process
//! holds the program's ([`crate::group::led`]) and lets it go when the
//! thread has said how the program ended, as a wrapper's group ends with
//! the wrapper. That is run there by this crate's suite. A program whose
//! group is let go of ends there with the code 0, and no signal says it
//! was ended: so the second thing above does not hold on Windows, where
//! one ended without being asked is said to have ended well.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use tokio::sync::watch;

use crate::unsaid::Unsaid;
use crate::wrapped::{self, Asked};

/// How a program ended, once it has: the code, or none for one that was
/// ended without being asked and of which nothing was said.
pub(crate) type How = Option<Option<i32>>;

/// One program a thread here is waiting for.
struct Standing {
    /// Whether it has ended, and then whether it was ended without being
    /// asked: known from the moment the thread is told, before it has
    /// said anything.
    ended: Mutex<Option<bool>>,
    told: Condvar,
    how: watch::Receiver<How>,
    /// Where it says how it ended, which is in its factory's own folder:
    /// what says whose it is ([`end_under`]).
    exit: PathBuf,
}

/// The programs threads here are waiting for, each by its number and the
/// moment it was started.
static AT_WORK: Mutex<BTreeMap<(u32, u64), Arc<Standing>>> = Mutex::new(BTreeMap::new());

/// How long a look waits for a thread to be told that its program has
/// ended, when the program is seen to be gone. The system tells it at
/// once; this is for a machine with everything else to do.
const TOLD_WITHIN: Duration = Duration::from_secs(10);

fn standing(pid: u32, started: u64) -> Option<Arc<Standing>> {
    let at_work = AT_WORK.lock().expect("wrappers lock");
    match started {
        // Started by something that could not say when: found by number.
        0 => at_work.range((pid, 0)..=(pid, u64::MAX)).next().map(|(_, standing)| standing.clone()),
        _ => at_work.get(&(pid, started)).cloned(),
    }
}

/// What was started here.
pub(crate) struct Started {
    pub pid: u32,
    /// The moment it was started, as the system says it.
    pub started: u64,
    /// How it ended, once the thread has said so.
    pub how: watch::Receiver<How>,
}

/// Starts what `command` says at the head of a group of its own, with a
/// thread that waits for it and says how it ended. What it prints goes
/// to the files `out` and `err`: through the thread, where it was handed
/// something.
pub(crate) fn start(
    command: &mut Command,
    asked: &Asked,
    out: &Path,
    err: &Path,
    unsaid: Unsaid,
) -> std::io::Result<Started> {
    fn open(path: &Path) -> std::io::Result<File> {
        OpenOptions::new().create(true).append(true).open(path)
    }
    let (out, errors) = (open(out)?, open(err)?);
    crate::group::leads(command);
    match &asked.reads {
        Some(reads) => command.stdin(File::open(reads)?),
        None => command.stdin(Stdio::null()),
    };
    let held = match unsaid.is_empty() {
        // Its files are its own: nothing here holds them once it has them.
        true => {
            command.stdout(out).stderr(errors);
            None
        }
        false => {
            command.stdout(Stdio::piped()).stderr(Stdio::piped());
            Some((out, errors))
        }
    };
    let (says, how) = watch::channel(None);
    let started = wrapped::start(command);
    // What the command held of the files is the program's now.
    command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    // Where this process has to hold the program's group to stop it, one
    // that could not be held is not left at work.
    let started = started.and_then(|mut child| match crate::group::led(&child) {
        Ok(()) => Ok(child),
        Err(why) => {
            let _ = child.kill();
            let _ = child.wait();
            Err(std::io::Error::other(why))
        }
    });
    let mut child = match started {
        Ok(child) => child,
        // A wrapper that could not start its program says so where the
        // program would have spoken, and says how that ended: there is a
        // wrapper whatever becomes of the program.
        Err(why) => {
            let said = format!("could not start `{}`: {why}\n", command.get_program().to_string_lossy());
            let _ = open(err).and_then(|mut errors| errors.write_all(said.as_bytes()));
            let code = wrapped::unstarted(&why);
            wrapped::say(&asked.exit, code);
            let _ = says.send(Some(Some(code)));
            return Ok(Started { pid: 0, started: 0, how });
        }
    };
    let pid = child.id();
    let started = crate::proc::identity(pid).unwrap_or(0);
    let standing = Arc::new(Standing {
        ended: Mutex::new(None),
        told: Condvar::new(),
        how: how.clone(),
        exit: asked.exit.clone(),
    });
    AT_WORK.lock().expect("wrappers lock").insert((pid, started), standing.clone());
    let exit = asked.exit.clone();
    std::thread::spawn(move || {
        let passing = held.map(|(out, err)| wrapped::passing(&mut child, &unsaid, out, err));
        let ended = wrapped::how(child.wait());
        *standing.ended.lock().expect("wrapper lock") = Some(ended.killed);
        standing.told.notify_all();
        let code = (!ended.killed).then(|| {
            passing.into_iter().flatten().for_each(|passed| drop(passed.join()));
            wrapped::say(&exit, ended.code);
            ended.code
        });
        // Its number may be another's from here on, and by now that may
        // be another program a thread here waits for.
        let mut at_work = AT_WORK.lock().expect("wrappers lock");
        if at_work.get(&(pid, started)).is_some_and(|there| Arc::ptr_eq(there, &standing)) {
            at_work.remove(&(pid, started));
        }
        drop(at_work);
        crate::group::ended(pid);
        let _ = says.send(Some(code));
    });
    Ok(Started { pid, started, how })
}

/// Whether a program that is itself gone still has a thread here that is
/// saying how it ended: what a wrapper is, in the moment after its
/// program has ended and before it has itself. One that was ended
/// without being asked has none: its wrapper went with it.
///
/// Asked only of a program seen to be gone, so the thread is told at
/// once, and is waited for until it has been: answered from what the
/// thread knew a moment before, a program that had ended well was taken
/// for one cut off, or one cut off for one still at work.
pub(crate) fn saying(pid: u32, started: u64) -> bool {
    let Some(standing) = standing(pid, started) else { return false };
    let ended = standing.ended.lock().expect("wrapper lock");
    let (ended, _) =
        standing.told.wait_timeout_while(ended, TOLD_WITHIN, |ended| ended.is_none()).expect("wrapper lock");
    // Not told in all that time, it is taken to be at work still.
    *ended != Some(true)
}

/// How a program a thread here still waits for is heard to end, for
/// whoever finds it again by its number and the moment it was started.
pub(crate) fn found(pid: u32, started: u64) -> Option<watch::Receiver<How>> {
    standing(pid, started).map(|standing| standing.how.clone())
}

/// Ends every program a thread here still waits for that says how it
/// ended under `folder`, and gives the number of each once its thread
/// has been told it is over: for a test's own factory, as the test is
/// over.
///
/// A factory leaves what it hosts running when it stops, on purpose, and
/// one stopped between starting a program and keeping that it had knows
/// nothing of the program. So a test that is over cannot ask its factory
/// what is still running; the threads here know every program this
/// process started for one, and where each says how it ended is in its
/// factory's folder. What is ended is ended as a group is
/// ([`crate::group::stop`]), without being asked: a test's program has
/// nothing to put away.
pub fn end_under(folder: &Path) -> Vec<u32> {
    let theirs: Vec<(u32, Arc<Standing>)> = AT_WORK
        .lock()
        .expect("wrappers lock")
        .iter()
        .filter(|(_, standing)| standing.exit.starts_with(folder))
        .map(|((pid, _), standing)| (*pid, standing.clone()))
        .collect();
    let mut over = Vec::new();
    for (pid, standing) in theirs {
        // One whose thread has been told already is over, and its number
        // may be another's by now.
        if standing.ended.lock().expect("wrapper lock").is_some() || crate::group::stop(pid, true).is_err() {
            continue;
        }
        let ended = standing.ended.lock().expect("wrapper lock");
        let (ended, _) = standing
            .told
            .wait_timeout_while(ended, TOLD_WITHIN, |ended| ended.is_none())
            .expect("wrapper lock");
        if ended.is_some() {
            over.push(pid);
        }
    }
    over
}
