//! Adapters: how one turn of an agent is actually run.
//!
//! A slot does not care what the agent is. An adapter takes a prompt and
//! returns a reply; behind it may be an agent's command-line program, a
//! script, or the built-in assistant that needs no model at all.
//!
//! None of the adapters here is for a particular agent. A real one comes as
//! an extension package (see `extension`), which describes its program to
//! the one adapter that runs any of them; anything that just reads a prompt
//! and prints a reply needs only the `command` adapter and no package.

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use hosting::Hosting;
use kernel::{BoxFut, Cancel};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Everything an adapter is given for one turn.
pub struct Turn {
    pub slot: String,
    /// Who the agent acts as: its slot, with the slot's scopes.
    pub principal: kernel::Principal,
    /// What the agent is told about itself and the fleet, ahead of the prompt.
    pub system: String,
    pub prompt: String,
    /// The adapter's own handle on the conversation so far, if there is one.
    pub session: Option<String>,
    pub model: Option<String>,
    pub workdir: PathBuf,
    /// Where the fleet's tooling and this slot's credential are, and the
    /// sign-in it was handed for this turn, if its slot draws one.
    pub env: BTreeMap<String, String>,
    /// Variables the program is started without, whatever the daemon has
    /// them as: those it would sign in by other than with what it was
    /// handed.
    pub without: Vec<String>,
    /// What everything the daemon starts is told about where it runs, and
    /// what of the daemon's own it is not handed. `None` only where an
    /// adapter is run by itself, with no daemon around it.
    pub hosting: Option<Hosting>,
    /// The wall its program is started behind, and what it is shown
    /// there; `None` for one that has none.
    pub behind: Option<crate::walled::Behind>,
    /// How much the agent may do on its own: `read`, `edit` or `full`. What
    /// that means to a particular agent's program is its package's business.
    pub permission_mode: String,
    /// The factory's own command as this slot's program finds it: the
    /// whole path of what is first on its `PATH` under the command's
    /// name. A package whose program asks leave for each command it runs
    /// says by this which one it need not ask about: what the agent may
    /// do with it is what its slot's key holds. `None` where an adapter
    /// is run by itself, with no daemon around it.
    pub cli: Option<PathBuf>,
    /// Whether its program may use the web unasked. What of a program's
    /// own that lets it do is its package's business.
    pub web: bool,
    pub cancel: Cancel,
    /// Called with text as the agent produces it, for whoever is watching.
    pub progress: Arc<dyn Fn(&str) + Send + Sync>,
    /// Where its program goes on without the daemon, for an agent whose
    /// turns do; `None` for one whose program is the daemon's to hold.
    pub kept: Option<crate::running::Kept>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reply {
    pub text: String,
    pub session: Option<String>,
    /// The model that wrote it, as the program itself named it, if it did.
    pub model: Option<String>,
    /// What its program said the turn cost, in dollars; nothing where it
    /// said nothing of it (a package that reports only tokens), which is
    /// never taken for its having cost nothing.
    pub cost_usd: Option<f64>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    /// What its program said it was refused as it went (a tool, a
    /// command), each by a word, where its package says how it tells.
    pub refused: Vec<String>,
}

/// Why a turn has no reply, and what its program had given by then: a
/// package reads that to say whether it was the work that failed or the
/// sign-in the program was handed, which had reached its limit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Failure {
    /// The sentence for whoever asked for the turn.
    pub message: String,
    /// What the agent had answered, if it had.
    pub reply: String,
    /// The code the program ended with, where it was seen to end.
    pub code: Option<i32>,
    /// The end of what it printed on standard error.
    pub stderr: String,
}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Failure { message, ..Failure::default() }
    }
}

impl From<&str> for Failure {
    fn from(message: &str) -> Self {
        Failure::from(message.to_string())
    }
}

pub trait Adapter: Send + Sync + 'static {
    fn run(&self, turn: Turn) -> BoxFut<'static, Result<Reply, Failure>>;
}

/// Kills a process group when dropped. A turn that ends, however it ends,
/// leaves nothing it started running.
pub struct Group(Option<u32>);

impl Group {
    pub fn of(child: &tokio::process::Child) -> Group {
        Group(child.id())
    }

    /// Lets go of a group whose process has been seen to end: there is
    /// nothing left to kill, and its number may by now be another's.
    /// (Where the daemon itself holds the group, which is Windows, what
    /// the program left running ends here all the same: a group does not
    /// outlive what holds it there.)
    pub fn spare(mut self) {
        if let Some(pid) = self.0.take() {
            hosting::group::ended(pid);
        }
    }
}

impl Drop for Group {
    fn drop(&mut self) {
        if let Some(pid) = self.0 {
            let _ = hosting::group::stop(pid, true);
        }
    }
}

/// The command that runs one turn of `program`, started as everything the
/// daemon starts is: with the daemon's environment less its own
/// credentials, told where the daemon and its program are and what the
/// factory's profile is called, and then given what the turn says,
/// which is the slot's own.
///
/// Behind a wall, what is started is the program that holds the wall,
/// which starts this one where it sees what it was shown and no more; and
/// it inherits of the daemon's environment only what any program needs to
/// run at all. A wall that cannot be held is why the turn fails: the
/// program is never started without it instead.
pub fn command(program: &str, turn: &Turn) -> Result<tokio::process::Command, String> {
    let start = Start {
        slot: &turn.slot,
        workdir: &turn.workdir,
        env: &turn.env,
        without: &turn.without,
        hosting: turn.hosting.as_ref(),
        behind: turn.behind.as_ref(),
        kept: turn.kept.as_ref().map(|kept| &kept.files),
        unsay: turn.kept.as_ref().map(|kept| kept.unsay.as_slice()).unwrap_or_default(),
    };
    started(program, &start)
}

/// What a slot's program is started with, for a turn or for anything
/// else that starts it as a turn would.
pub struct Start<'a> {
    pub slot: &'a str,
    pub workdir: &'a std::path::Path,
    pub env: &'a BTreeMap<String, String>,
    pub without: &'a [String],
    pub hosting: Option<&'a Hosting>,
    pub behind: Option<&'a crate::walled::Behind>,
    /// Where it writes what it says, when it is to go on without the
    /// daemon; `None` for one that speaks to the daemon over pipes.
    pub kept: Option<&'a hosting::kept::Files>,
    /// For one that is kept: what its files are written without
    /// ([`crate::running::Kept::unsay`]).
    pub unsay: &'a [(String, String)],
}

/// The command that starts `program` with `turn`, as [`command`] says.
pub fn started(program: &str, turn: &Start<'_>) -> Result<tokio::process::Command, String> {
    // A turn that ends leaves nothing it started running: where that
    // cannot be seen to, nothing is started.
    hosting::group::can().map_err(|why| format!("`{}` was not started: {why}", turn.slot))?;
    // One that goes on without the daemon is started by a wrapper that
    // outlives it (`hosting::kept`), and everything below is said of the
    // wrapper: what it is given, the program it starts has. Handed
    // something, what the program prints is written to its files by the
    // wrapper without that.
    let wrapped = |argv: &[OsString]| -> Result<tokio::process::Command, String> {
        let not =
            |why: String| format!("`{}` could not be started to go on without the daemon: {why}", turn.slot);
        match turn.kept {
            Some(files) => {
                hosting::kept::wrapper(files, turn.unsay, argv).map_err(|err| not(err.to_string()))
            }
            None => {
                // By the file its name comes to along the path it is
                // started with, where the system does not find a program
                // by its name alone (a kept one is found so by its
                // wrapper).
                let path = turn.env.get("PATH").map(OsStr::new);
                let mut command = tokio::process::Command::new(hosting::named::program(&argv[0], path));
                command.args(&argv[1..]);
                Ok(command)
            }
        }
    };
    let mut command = match turn.behind {
        None => wrapped(&[program.into()])?,
        Some(behind) => {
            let mut shown = behind.shown.clone();
            let path = turn.env.get("PATH").map(String::as_str).unwrap_or_default();
            shown.program(program, path, &behind.beside);
            let mut held = behind.walls.behind(&shown).map_err(|why| {
                format!(
                    "`{slot}` works behind a wall, and none can be held here: {why}. Nothing was started. To let \
                     it work with none, seeing this device as whoever runs the daemon does: \
                     `{cli} agent add {slot} --no-wall`",
                    slot = turn.slot,
                    cli = brand::CLI
                )
            })?;
            // The wrapper is outside the wall, and what holds the wall is
            // what it starts.
            held.push(program.into());
            let mut command = wrapped(&held)?;
            command.env_clear();
            let kept =
                hosting::wall::INHERITED.iter().copied().chain(behind.inherits.iter().map(String::as_str));
            for name in kept {
                if let Some(value) = std::env::var_os(name) {
                    command.env(name, value);
                }
            }
            if let Some(home) = &behind.home {
                command.env("HOME", home);
            }
            command.envs(behind.sets.iter().map(|(name, value)| (name, value)));
            command
        }
    };
    command.current_dir(turn.workdir);
    // What a kept one prints goes to its files, which the wrapper was given.
    if turn.kept.is_none() {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
    }
    match turn.hosting {
        Some(hosting) => hosting.dress(&mut command, turn.env),
        None => {
            command.envs(turn.env);
        }
    }
    for variable in turn.without {
        command.env_remove(variable);
    }
    hosting::group::leads(command.as_std_mut());
    Ok(command)
}

/// A command that asks a program something outside any turn: started as
/// the above is, and given nothing of any slot's.
pub fn asking(program: &str, hosting: Option<&Hosting>) -> tokio::process::Command {
    let mut command = tokio::process::Command::new(hosting::named::program(OsStr::new(program), None));
    if let Some(hosting) = hosting {
        hosting.dress(&mut command, &BTreeMap::new());
    }
    hosting::group::leads(command.as_std_mut());
    command
}

/// Starts a program.
///
/// A program that is being replaced at this very moment (an agent's
/// command-line tool updating itself) cannot be started for an instant; that
/// is tried again rather than reported.
pub async fn spawn(
    command: &mut tokio::process::Command,
    program: &str,
) -> Result<tokio::process::Child, String> {
    try_spawn(command).await.map_err(|err| {
        let why = hosting::named::why_not(command.as_std().get_program(), &err);
        format!("could not start `{program}`: {why}")
    })
}

/// The same, for a caller that has something of its own to say about why
/// it did not start.
pub async fn try_spawn(command: &mut tokio::process::Command) -> std::io::Result<tokio::process::Child> {
    let mut tries = 0;
    loop {
        match command.spawn() {
            Ok(mut child) => {
                // What is started here is the daemon's to stop with all
                // it starts, with no wrapper between. Where the daemon
                // has to hold its group for that (`hosting::group::led`)
                // and cannot, the program is not left at work.
                return match hosting::group::led(&child) {
                    Ok(()) => Ok(child),
                    Err(why) => {
                        let _ = child.start_kill();
                        Err(std::io::Error::other(why))
                    }
                };
            }
            Err(err) if err.kind() == std::io::ErrorKind::ExecutableFileBusy && tries < 20 => {
                tries += 1;
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            Err(err) => return Err(err),
        }
    }
}

/// Starts the program, giving it the prompt on standard input.
///
/// A program that exits without reading its input is not an error here: how
/// it ended says what went wrong, and that is what is reported.
pub async fn start(
    mut command: tokio::process::Command,
    program: &str,
    input: &str,
) -> Result<tokio::process::Child, String> {
    command.stdin(Stdio::piped());
    let mut child = spawn(&mut command, program).await?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input.as_bytes()).await;
    }
    Ok(child)
}

/// How much of what a program printed on standard error is kept.
pub const KEPT_ERRORS: usize = 16 * 1024;

/// The end of what a process that has ended wrote to standard error.
pub async fn errors(stderr: Option<tokio::process::ChildStderr>) -> String {
    let mut bytes = Vec::new();
    if let Some(mut stderr) = stderr {
        let _ = stderr.read_to_end(&mut bytes).await;
    }
    let from = bytes.len().saturating_sub(KEPT_ERRORS);
    String::from_utf8_lossy(&bytes[from..]).into_owned()
}

/// The last lines of what a process wrote to standard error, for an error message.
pub fn tail(text: &str) -> String {
    let lines: Vec<&str> = text.lines().rev().take(4).collect();
    lines.into_iter().rev().collect::<Vec<_>>().join(" / ")
}

/// Runs turns with any program that reads a prompt on standard input and
/// prints its reply. `{model}` in the arguments is replaced by the slot's.
pub struct Command {
    pub argv: Vec<String>,
}

impl Adapter for Command {
    fn run(&self, turn: Turn) -> BoxFut<'static, Result<Reply, Failure>> {
        let argv = self.argv.clone();
        Box::pin(async move {
            let Some((program, args)) = argv.split_first() else {
                return Err("this adapter has no command configured".into());
            };
            let model = turn.model.clone().unwrap_or_default();
            let input = format!("{}\n\n{}", turn.system, turn.prompt);
            let mut running = crate::running::begin(&turn, program, |command| {
                command.args(args.iter().map(|arg| arg.replace("{model}", &model)));
                input
            })
            .await?;
            let mut said = Vec::new();
            loop {
                let line = tokio::select! {
                    line = running.line() => line?,
                    () = turn.cancel.cancelled() => {
                        running.stop().await;
                        return Err("the turn was cancelled".into());
                    }
                };
                let Some(line) = line else { break };
                said.push(line);
            }
            let text = said.join("\n");
            let ended = running.ended().await?;
            if !ended.well() {
                let stderr = running.errors().await;
                running.done();
                let message = format!("`{program}` failed ({}): {}", ended.said, tail(&stderr));
                return Err(Failure { message, reply: text, code: ended.code, stderr });
            }
            running.done();
            let text = text.trim().to_string();
            (turn.progress)(&text);
            Ok(Reply { text, ..Reply::default() })
        })
    }
}
