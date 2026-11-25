//! A program and everything it started, as one thing that can be asked
//! to stop.
//!
//! Everything the factory starts is started at the head of a group of its
//! own ([`leads`]), and is stopped by its group ([`stop`]): asked, which a
//! program may take its time over, or ended without being asked. That is
//! a call to the system and never a program that is run: a daemon started
//! with a path that had no such program on it stopped nothing, and said
//! nothing of it.
//!
//! What a group is, is the system's:
//!
//! * On Unix it is a process group, which the wrapper leads
//!   ([`crate::wrapped`]) and a signal reaches whole. It has been run on
//!   Linux only.
//! * On Windows it is a job object that ends everything in it when the
//!   last hold on it is let go: see `system` below, which says who holds
//!   each and what a stop comes to. The half of it a process holds for
//!   a program it started with no wrapper ([`led`], [`stop`], [`ended`])
//!   is run on Windows by this crate's suite, whose programs are held
//!   so; a wrapper's half has been run there by hand, once.
//!
//! The doors a system has no use for do nothing there ([`heads`],
//! [`led`], [`wrapped`] and [`ended`] on Unix), so whoever starts or stops
//! something says the same on every system.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Has what `command` starts lead a group of its own: one asking then
/// reaches it and everything it starts, and an interrupt typed at the
/// daemon's terminal does not.
pub fn leads(command: &mut std::process::Command) {
    system::leads(command);
}

/// Whether a group can be stopped on this system, and in a sentence why
/// not. Asked before anything is started: a program that could not be
/// stopped afterwards is not started.
pub fn can() -> Result<(), String> {
    system::can()
}

/// Asks the group `leader` leads to stop; with `force`, ends it without
/// asking. A group that is gone already was stopped.
///
/// On Windows nothing is asked: a stop is an ending, whatever `force`
/// says (`system` below says why).
pub fn stop(leader: u32, force: bool) -> Result<(), String> {
    system::stop(leader, force)
}

/// Asks one process to stop, and nothing it started: for a program that
/// stops what it started itself (a daemon, asked by what watches over it).
pub fn ask(pid: u32) -> Result<(), String> {
    system::ask(pid)
}

/// Makes this process the head of a group that ends with it, for as long
/// as what comes back is held: said by the wrapper before it starts its
/// program, so that everything it starts is in the group from the first.
/// Where a group is a process group the wrapper was started at the head
/// of one ([`leads`]), and this does nothing.
pub fn heads() -> Result<impl Sized, String> {
    system::heads()
}

/// What was just started with [`leads`] and no wrapper: a program the
/// daemon speaks to over pipes.
pub trait Started {
    /// Its number, while it has not been waited for.
    fn number(&self) -> Option<u32>;
    /// The system's hold on it.
    #[cfg(windows)]
    fn handle(&self) -> Option<std::os::windows::io::RawHandle>;
}

impl Started for std::process::Child {
    fn number(&self) -> Option<u32> {
        Some(self.id())
    }

    #[cfg(windows)]
    fn handle(&self) -> Option<std::os::windows::io::RawHandle> {
        Some(std::os::windows::io::AsRawHandle::as_raw_handle(self))
    }
}

impl Started for tokio::process::Child {
    fn number(&self) -> Option<u32> {
        self.id()
    }

    #[cfg(windows)]
    fn handle(&self) -> Option<std::os::windows::io::RawHandle> {
        self.raw_handle()
    }
}

/// Says that `child` was started to lead a group with no wrapper, right
/// after it was started. Where a group is something this process has to
/// hold (Windows), it is made and held here until the group is stopped
/// ([`stop`]) or its leader seen to end ([`ended`]); `Err` where that
/// could not be done, and then the program is not left at work: whoever
/// started it ends it. Where a group is a process group, [`leads`] was
/// all there is to it.
pub fn led(child: &impl Started) -> Result<(), String> {
    system::led(child)
}

/// Says that the group `leader` leads is a wrapper's, which says how its
/// program ended at `exit`: by whoever started the wrapper or found it
/// again. Where a group cannot be reached by its leader's number
/// (Windows), that is where word is left for the wrapper to end
/// ([`word`]).
pub fn wrapped(leader: u32, exit: &Path) {
    system::wrapped(leader, exit);
}

/// Says that `leader` has been seen to end, and that its group is not to
/// be stopped: nothing more is kept of it. Where a group is held by this
/// process (Windows) it ends here, with whatever its leader left running:
/// there a group does not outlive what holds it.
pub fn ended(leader: u32) {
    system::ended(leader);
}

/// Where the wrapper with the number `leader`, which says how its
/// program ended at `exit`, is left word to end, on a system where its
/// group cannot be reached by a number: a file beside that one, called
/// by the wrapper's number as well. So a word is one wrapper's: the next
/// to be started under the same files has another number and does not
/// end on it, and nothing has to take a word away before a wrapper is
/// started, which would take it from one that had not yet heard it.
pub fn word(exit: &Path, leader: u32) -> PathBuf {
    let mut word = exit.as_os_str().to_owned();
    word.push(format!(".{leader}.stop"));
    PathBuf::from(word)
}

/// How often a wrapper looks for its word. Nothing tells it: what could
/// (an event of a console) is not offered without `unsafe` where this is
/// used.
pub const LOOKS: Duration = Duration::from_millis(100);

/// Waits until word is left at `word`, looking every `looks`, and takes
/// it away: it was heard, and is not there for whatever has this
/// wrapper's number next.
pub fn hears(word: &Path, looks: Duration) {
    while !word.exists() {
        std::thread::sleep(looks);
    }
    let _ = std::fs::remove_file(word);
}

/// What a process knows of the groups it may be asked to stop, where a
/// group cannot be reached by its leader's number and so has to be
/// remembered: each by that number, which is all [`stop`] is handed.
///
/// A number is one group's at a time. It is the system's to give again
/// once its process has ended, so what is known of a number is what was
/// last said of it.
///
/// `Job` is the hold on a group, which ends it when let go. This is a
/// table and nothing of any system's, so that it is tested where the
/// system it is for is not.
pub struct Known<Job> {
    /// The groups this process holds: programs it started with no wrapper.
    held: BTreeMap<u32, Job>,
    /// The wrappers it started or found again, each with where word is
    /// left for it.
    wrappers: BTreeMap<u32, PathBuf>,
}

/// What stopping a group comes to.
#[derive(Debug, PartialEq, Eq)]
pub enum Stops<Job> {
    /// The hold on it is let go, and it ends: by whoever is handed this
    /// letting go of it.
    Ends(Job),
    /// Word is left here for its wrapper, which ends with everything it
    /// started.
    Leaves(PathBuf),
    /// Nothing is known under that number: it was never this process's
    /// to stop, or it has been seen to end. It was stopped.
    Nothing,
}

impl<Job> Known<Job> {
    pub const fn new() -> Known<Job> {
        Known { held: BTreeMap::new(), wrappers: BTreeMap::new() }
    }

    /// A program started with no wrapper leads a group held by `job`.
    /// What comes back is the hold on a group that had its number
    /// before, whose leader is therefore gone: to be let go.
    pub fn led(&mut self, leader: u32, job: Job) -> Option<Job> {
        self.wrappers.remove(&leader);
        self.held.insert(leader, job)
    }

    /// A wrapper leads a group, and hears at `word`. What comes back is
    /// as for [`Known::led`].
    pub fn wrapped(&mut self, leader: u32, word: PathBuf) -> Option<Job> {
        self.wrappers.insert(leader, word);
        self.held.remove(&leader)
    }

    /// What stopping the group `leader` leads comes to. A group that is
    /// held is known no longer; a wrapper may be left word again.
    pub fn stop(&mut self, leader: u32) -> Stops<Job> {
        if let Some(job) = self.held.remove(&leader) {
            return Stops::Ends(job);
        }
        match self.wrappers.get(&leader) {
            Some(word) => Stops::Leaves(word.clone()),
            None => Stops::Nothing,
        }
    }

    /// `leader` was seen to end. What comes back is the hold on its
    /// group, to be let go.
    pub fn ended(&mut self, leader: u32) -> Option<Job> {
        self.wrappers.remove(&leader);
        self.held.remove(&leader)
    }
}

#[cfg(unix)]
mod system {
    use std::os::unix::process::CommandExt;
    use std::path::Path;

    use rustix::io::Errno;
    use rustix::process::{Pid, Signal, kill_process, kill_process_group};

    pub fn leads(command: &mut std::process::Command) {
        command.process_group(0);
    }

    pub fn can() -> Result<(), String> {
        Ok(())
    }

    pub fn heads() -> Result<(), String> {
        Ok(())
    }

    pub fn led(_child: &impl super::Started) -> Result<(), String> {
        Ok(())
    }

    pub fn wrapped(_leader: u32, _exit: &Path) {}

    pub fn ended(_leader: u32) {}

    /// The process a number names. Not the first two: to the system those
    /// mean every process there is and the one that started them all, and
    /// nothing the factory started has either number.
    fn named(pid: u32) -> Result<Pid, String> {
        i32::try_from(pid)
            .ok()
            .filter(|pid| *pid > 1)
            .and_then(Pid::from_raw)
            .ok_or_else(|| format!("{pid} is not the number of a process the factory started"))
    }

    fn asked(pid: u32, answer: Result<(), Errno>) -> Result<(), String> {
        match answer {
            // Nothing there to stop: it had ended.
            Ok(()) | Err(Errno::SRCH) => Ok(()),
            Err(err) => Err(format!("the system would not stop process {pid}: {err}")),
        }
    }

    pub fn stop(leader: u32, force: bool) -> Result<(), String> {
        let signal = if force { Signal::KILL } else { Signal::TERM };
        asked(leader, kill_process_group(named(leader)?, signal))
    }

    pub fn ask(pid: u32) -> Result<(), String> {
        asked(pid, kill_process(named(pid)?, Signal::TERM))
    }
}

/// Windows. What a process holds for a program it started with no
/// wrapper (`led`, `stop`, `ended`) is run there by this crate's suite,
/// through the thread that stands in for a wrapper in a test
/// ([`crate::here`]): a step that timed out, an instance stopped and a
/// service taken down each ended with what it had started. What a
/// wrapper does for itself (`heads`, the word it is left) has no test
/// of its own run there (the wrapper's are the command line's, and
/// still marked for Unix), and was seen done by hand once: of that,
/// every sentence here is what the code is written to do.
///
/// One thing seen there that nothing here says otherwise: a program
/// ended by its group being let go of ends with the code 0, as one that
/// ended well does. Nothing tells the two apart by the code.
///
/// The system's own interface takes `unsafe`, which this workspace
/// denies, so a job object is the `win32job` crate's: a job made, a
/// process put in it, and everything in it ended when the last hold on
/// it is let go (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`). That crate
/// offers no job by a name, no hold on a job another process made and no
/// ending of one. And nothing here ends a process by its number: the one
/// safe door to that (`sysinfo`'s `Process::kill`) runs `taskkill` as a
/// program there, which is what this module is for not doing. So a
/// group is ended by whoever holds it letting go:
///
/// * **A wrapper** ([`crate::wrapped`]) makes a job and puts itself in
///   it before it starts its program ([`heads`]), and holds it for as
///   long as it lives: everything it starts is in the job with no moment
///   in between, and ends when the wrapper does, however that is. It is
///   stopped by being left word ([`word`], a file beside the one it says
///   how its program ended in, called by its number), which it looks
///   for ([`hears`]) and ends on. Whoever starts or finds a wrapper says
///   where that is ([`wrapped`]), since `stop` is handed a number and
///   nothing else. A wrapper that is not looking (one that is stuck) is
///   not stopped so; one this process started is then ended by the hold
///   it has on it (`Proc::stop`), and one it found again cannot be.
/// * **A program started with no wrapper** (an agent's on pipes, a
///   package's) is put in a job this process holds ([`led`]), right
///   after it is started. What it started in the instant before is not
///   in the job. The job is let go by `stop`, by its leader being seen
///   to end ([`ended`]), and by this process ending.
///
/// What that comes to, and is other than on Unix:
///
/// * **Nothing is asked.** A stop is an ending whatever `force` says:
///   a program is given no chance to put its things away, and a wrapper
///   that is stopped says nothing of how its program ended. Asking would
///   be an event of the console (`GenerateConsoleCtrlEvent`), which no
///   crate here offers safely.
/// * **A group does not outlive what holds it.** What a program left
///   running ends when its wrapper does, where on Unix it goes on until
///   its group is stopped; and a program on pipes ends with the daemon,
///   with what it started.
/// * **A wrapper is not reached by its number.** One that this process
///   neither started nor found again ([`wrapped`]) is not stopped by
///   `stop`, which then says it was (as of a group that is gone): a
///   number alone may be anybody's by now.
/// * **One process cannot be asked** ([`ask`]): what keeps a daemon
///   running leaves the asking to the console they share, or to `down`.
/// * What is started has a console of its own with no window
///   (`CREATE_NO_WINDOW`), so that it does not end with the daemon's
///   and puts nothing on anybody's screen.
#[cfg(windows)]
mod system {
    use std::os::windows::process::CommandExt;
    use std::path::Path;
    use std::sync::Mutex;

    use win32job::{ExtendedLimitInfo, Job};

    use super::{Known, Stops};

    /// A process started at the head of a group of its own, which an
    /// interrupt typed at the console of what started it does not reach.
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    /// A console of its own with no window: not the daemon's, which may
    /// be closed, and nothing put on anybody's screen.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    /// The groups this process may be asked to stop.
    static KNOWN: Mutex<Known<Job>> = Mutex::new(Known::new());

    fn known() -> std::sync::MutexGuard<'static, Known<Job>> {
        KNOWN.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// A job that ends everything in it when the last hold on it is let go.
    fn job() -> Result<Job, String> {
        let mut limits = ExtendedLimitInfo::new();
        limits.limit_kill_on_job_close();
        Job::create_with_limit_info(&limits).map_err(|err| {
            format!("the system gave no job object to hold a program and what it starts in: {err}")
        })
    }

    pub fn leads(command: &mut std::process::Command) {
        command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
    }

    pub fn can() -> Result<(), String> {
        Ok(())
    }

    pub fn heads() -> Result<(), String> {
        let job = job()?;
        job.assign_current_process()
            .map_err(|err| format!("this process could not be put in a job object of its own: {err}"))?;
        // Never let go of while this process lives. Letting go ends
        // everything in the job, this process first and with no code of
        // its own: a wrapper that let go as it came to its end was ended
        // there by the system, and its program was taken to have ended
        // well whatever it had said (seen on Windows: `exit 3`, read as
        // nought). The system lets go for it once it has ended with the
        // code it chose, and what it started ends then.
        std::mem::forget(job);
        Ok(())
    }

    pub fn led(child: &impl super::Started) -> Result<(), String> {
        let (Some(leader), Some(handle)) = (child.number(), child.handle()) else {
            // It has ended and been waited for: there is nothing to hold.
            return Ok(());
        };
        let job = job()?;
        // The system's hold as the number it is: no pointer is followed.
        job.assign_process(handle as isize)
            .map_err(|err| format!("process {leader} could not be put in a job object: {err}"))?;
        // A job that had its number before is let go: its leader is gone.
        drop(known().led(leader, job));
        Ok(())
    }

    pub fn wrapped(leader: u32, exit: &Path) {
        drop(known().wrapped(leader, super::word(exit, leader)));
    }

    pub fn ended(leader: u32) {
        drop(known().ended(leader));
    }

    pub fn stop(leader: u32, _force: bool) -> Result<(), String> {
        let stops = known().stop(leader);
        match stops {
            Stops::Ends(job) => drop(job),
            Stops::Leaves(word) => std::fs::write(&word, "").map_err(|err| {
                format!("process {leader} could not be left word to end, at {}: {err}", word.display())
            })?,
            Stops::Nothing => {}
        }
        Ok(())
    }

    pub fn ask(pid: u32) -> Result<(), String> {
        Err(format!(
            "process {pid} cannot be asked to stop by its number on this system. A daemon hears the interrupt \
             typed at the terminal it shares with what started it, and `{} down` asks it from anywhere",
            brand::CLI
        ))
    }
}

/// Neither: nothing is started that could not be stopped.
#[cfg(not(any(unix, windows)))]
mod system {
    use std::path::Path;

    const NOT_WRITTEN: &str = "stopping a program and everything it started is not written for this system, so \
                               nothing is started here that would have to be stopped";

    pub fn leads(_command: &mut std::process::Command) {}

    pub fn can() -> Result<(), String> {
        Err(NOT_WRITTEN.to_string())
    }

    pub fn heads() -> Result<(), String> {
        Err(NOT_WRITTEN.to_string())
    }

    pub fn led(_child: &impl super::Started) -> Result<(), String> {
        Err(NOT_WRITTEN.to_string())
    }

    pub fn wrapped(_leader: u32, _exit: &Path) {}

    pub fn ended(_leader: u32) {}

    pub fn stop(_leader: u32, _force: bool) -> Result<(), String> {
        Err(NOT_WRITTEN.to_string())
    }

    pub fn ask(_pid: u32) -> Result<(), String> {
        Err(NOT_WRITTEN.to_string())
    }
}
