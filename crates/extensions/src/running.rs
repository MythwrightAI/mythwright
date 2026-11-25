//! The program of one turn while it runs: started by its wrapper and read
//! from its files (`hosting::kept`), or, where that cannot be, on pipes
//! held by the daemon.
//!
//! An adapter reads what the program prints a line at a time, waits for it
//! to end and asks how. It does not care which of the two this is. Every
//! turn's program is started by its wrapper, so that
//! whether it goes on without the daemon is decided when the daemon stops
//! and not when it was started: a slot that `outlives` the daemon, as it
//! says then, has its program left to go on and taken up by the next
//! daemon, which reads what it said from the first line and waits for it
//! where it is; any other has it ended as the daemon stops, with its
//! group, as one on pipes always was, and is begun again by the next (or
//! left, as whoever asked for it said). So a slot changed while its turn
//! is in hand holds for that turn. Only a program that cannot write to its
//! files is on pipes: one handed something in no variable, which only the
//! daemon can take out of what it prints, and one started where there is
//! no daemon around it; it ends with the daemon whatever its slot says.
//!
//! A turn that ends, however it ends, leaves nothing it started running,
//! and what a program wrote is taken away only once the turn is kept as
//! ended. One that is let go of without having ended, because the daemon
//! itself is stopping, is left as it is where its slot outlives the
//! daemon, and ended otherwise ([`Running`]'s `drop`).

use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, RwLock};

use hosting::kept::{self, Files, Lines, Process};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::adapter::{self, Group, Turn};

/// How a turn's program goes on without the daemon, when it does.
#[derive(Clone)]
pub struct Kept {
    /// Where its files are.
    pub files: Files,
    /// An earlier daemon started it, and it is to be taken up where it is.
    pub taken_up: bool,
    /// What its files are written without: each variable of its
    /// environment that holds something it was handed (a key, a sign-in),
    /// and what that is said as in its place. With any, the program's
    /// words go through its wrapper on their way to the files
    /// (`hosting::wrapped`), so that what lies there while no daemon is
    /// reading is what a daemon would have written down, and never the
    /// value.
    pub unsay: Vec<(String, String)>,
    /// Whether its slot outlives the daemon, asked as the daemon stops.
    pub outlives: Outlives,
}

/// Whether a slot's program goes on without the daemon, as its slot says
/// now: asked as the daemon stops, where nothing can be waited for.
pub type Outlives = Arc<dyn Fn() -> bool + Send + Sync>;

/// The agents whose programs go on without the daemon, as their slots say
/// now: what a program that is let go of as the daemon stops is asked of,
/// where the slots cannot be read. Filled as agents are opened, and told
/// each time a slot is written or taken away.
#[derive(Default)]
pub struct Outliving(RwLock<HashSet<String>>);

impl Outliving {
    pub fn of(&self, slot: &str) -> bool {
        self.0.read().expect("agents lock").contains(slot)
    }

    pub fn is(&self, slot: &str, outlives: bool) {
        let mut known = self.0.write().expect("agents lock");
        match outlives {
            true => known.insert(slot.to_string()),
            false => known.remove(slot),
        };
    }
}

/// Whether the program of a turn an earlier daemon left in hand under
/// `files` is there to be taken up: still at work where its slot
/// `outlives` the daemon, or ended having said how. One the daemon ended
/// as it stopped ([`kept::Process::was_cut`]), and one still at work for a
/// slot that does not outlive it (the daemon went without stopping, and
/// could not end it), is ended now and its files taken away: what becomes
/// of the turn is what becomes of one that was cut off. One that is gone
/// without a word (killed outright with its wrapper, or the machine was
/// started again) was cut off after all.
pub async fn there_to_take_up(files: &Files, watch: &hosting::Watch, outlives: bool) -> bool {
    let Some(mut process) = Process::found(files, watch) else { return false };
    if process.was_cut() || (!outlives && !process.has_ended() && process.is_there()) {
        process.stop().await;
        process.forget();
        kept::clean(files);
        return false;
    }
    process.is_there()
}

/// Whether an agent's program is to go on without the daemon, as it is
/// said in a blueprint, a row or a request: `on` or `off`, true or false.
/// Nothing, or the empty text, says nothing of it.
pub fn said(outlives: Option<&serde_json::Value>) -> kernel::Result<Option<bool>> {
    on_or_off(outlives, "whether an agent `outlives` the daemon")
}

/// Whether an agent's program may use the web unasked, said the same way.
pub fn said_of_the_web(web: Option<&serde_json::Value>) -> kernel::Result<Option<bool>> {
    on_or_off(web, "whether an agent may use the `web` unasked")
}

/// Something of a slot that is on or off, as a blueprint, a row or a
/// request says it; `what` is how a refusal names it.
fn on_or_off(said: Option<&serde_json::Value>, what: &str) -> kernel::Result<Option<bool>> {
    use serde_json::Value;
    match said {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(said)) => Ok(Some(*said)),
        Some(Value::String(said)) if said.is_empty() => Ok(None),
        Some(Value::String(said)) if said == "on" => Ok(Some(true)),
        Some(Value::String(said)) if said == "off" => Ok(Some(false)),
        Some(other) => Err(kernel::Error::invalid(format!(
            "{what} is `on` or `off`, and not `{}`",
            blueprint::expr::display(other)
        ))),
    }
}

pub enum Running {
    Piped {
        child: Box<tokio::process::Child>,
        group: Option<Group>,
        lines: tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
        stderr: Option<tokio::process::ChildStderr>,
    },
    Kept {
        process: Box<Process>,
        said: Lines,
        changes: hosting::Changes,
        heard: VecDeque<String>,
        /// How it ended, once it has: the code, or none where it left none.
        ended: Option<Option<i32>>,
        outlives: Outlives,
        /// Stopped, or done with: nothing is left to do as it is let go of.
        settled: bool,
    },
}

/// How a program ended.
pub struct Ended {
    pub code: Option<i32>,
    /// The same as a person reads it, for a sentence.
    pub said: String,
}

impl Ended {
    pub fn well(&self) -> bool {
        self.code == Some(0)
    }
}

/// Why a kept program cannot be taken up, as a sentence.
const GONE: &str = "its program was to go on without the daemon, and was gone when the daemon came back, \
                    leaving no word of how it ended";

/// The program of `turn`: taken up where an earlier daemon left it, or
/// started now. `more` adds what the adapter passes it and gives what it
/// is to read on its standard input.
pub async fn begin(
    turn: &Turn,
    program: &str,
    more: impl FnOnce(&mut tokio::process::Command) -> String,
) -> Result<Running, String> {
    let watch = turn.hosting.as_ref().map(|hosting| hosting.watch().clone());
    match (&turn.kept, watch) {
        (Some(kept), Some(watch)) if kept.taken_up => {
            let process = Process::found(&kept.files, &watch).ok_or(GONE)?;
            if !process.is_there() {
                process.forget();
                return Err(GONE.to_string());
            }
            Ok(kept_running(process, &watch, kept.outlives.clone()))
        }
        (Some(kept), Some(watch)) => {
            let mut command = adapter::command(program, turn)?;
            let input = more(&mut command);
            kept::hand_in(&kept.files, &input)
                .map_err(|err| format!("could not hand `{program}` what it is asked: {err}"))?;
            let process = kept::begin(&mut command, &kept.files, &kept.unsay)
                .map_err(|err| format!("could not start `{program}`: {err}"))?;
            Ok(kept_running(process, &watch, kept.outlives.clone()))
        }
        _ => {
            let mut command = adapter::command(program, turn)?;
            let input = more(&mut command);
            let mut child = adapter::start(command, program, &input).await?;
            let group = Some(Group::of(&child));
            let stdout = child.stdout.take().ok_or("the agent has no output")?;
            let stderr = child.stderr.take();
            let lines = BufReader::new(stdout).lines();
            Ok(Running::Piped { child: Box::new(child), group, lines, stderr })
        }
    }
}

fn kept_running(process: Process, watch: &hosting::Watch, outlives: Outlives) -> Running {
    let (said, changes) = (process.said(), process.changes(watch));
    let heard = VecDeque::new();
    Running::Kept { process: Box::new(process), said, changes, heard, ended: None, outlives, settled: false }
}

impl Running {
    /// The next line it printed, or nothing once it has printed its last.
    pub async fn line(&mut self) -> Result<Option<String>, String> {
        match self {
            Running::Piped { lines, .. } => {
                lines.next_line().await.map_err(|err| format!("reading the agent's output: {err}"))
            }
            Running::Kept { process, said, changes, heard, ended, .. } => loop {
                if let Some(line) = heard.pop_front() {
                    return Ok(Some(line));
                }
                if ended.is_some() {
                    return Ok(None);
                }
                heard.extend(said.more(false));
                if !heard.is_empty() {
                    continue;
                }
                tokio::select! {
                    code = process.wait() => {
                        *ended = Some(code);
                        heard.extend(said.more(true));
                    }
                    () = changes.next() => {}
                }
            },
        }
    }

    /// How it ended, once it has printed its last.
    pub async fn ended(&mut self) -> Result<Ended, String> {
        match self {
            Running::Piped { child, .. } => {
                let status = child.wait().await.map_err(|err| format!("waiting for the agent: {err}"))?;
                Ok(Ended { code: status.code(), said: status.to_string() })
            }
            Running::Kept { process, ended, .. } => {
                let code = match ended {
                    Some(code) => *code,
                    None => process.wait().await,
                };
                *ended = Some(code);
                let said = match code {
                    Some(code) => format!("exit status: {code}"),
                    None => "killed, or how it ended was lost".to_string(),
                };
                Ok(Ended { code, said })
            }
        }
    }

    /// The end of what it printed on standard error. Whatever it left
    /// running is stopped first, so that all it had to say is there.
    pub async fn errors(&mut self) -> String {
        match self {
            Running::Piped { group, stderr, .. } => {
                drop(group.take());
                adapter::errors(stderr.take()).await
            }
            Running::Kept { process, .. } => {
                process.sweep();
                process.errors(adapter::KEPT_ERRORS)
            }
        }
    }

    /// Stops it, and everything it started: the turn was cancelled.
    /// What it wrote for a later daemon stays until the turn is kept as
    /// ended (`Agents::conclude`).
    pub async fn stop(mut self) {
        if let Running::Kept { process, settled, .. } = &mut self {
            process.stop().await;
            process.forget();
            *settled = true;
        }
        // One on pipes is stopped by being let go of.
    }

    /// It has ended and been read: what it left running is ended. What
    /// it wrote for the next daemon is not taken away here: that is the
    /// answer a daemon that goes before it has kept the turn's end leaves
    /// the next to take up, and it goes once the end is kept
    /// (`Agents::conclude`).
    pub fn done(mut self) {
        if let Running::Kept { process, settled, .. } = &mut self {
            process.sweep();
            process.forget();
            *settled = true;
        }
    }
}

/// A program let go of before it was done with: the daemon is stopping
/// (its turn's task is let go of as the daemon halts), or reading it
/// failed. Where its slot outlives the daemon, as the slot says now, it
/// is left to go on and is taken up by the next daemon; otherwise it is
/// ended with all it started, as one on pipes is, and is said to have
/// been ended so ([`kept::Process::cut`]). One that has ended already is
/// left as it is, its answer for whoever takes it up.
impl Drop for Running {
    fn drop(&mut self) {
        if let Running::Kept { process, outlives, settled: false, .. } = self
            && !process.has_ended()
            && !outlives()
        {
            process.cut();
        }
    }
}
