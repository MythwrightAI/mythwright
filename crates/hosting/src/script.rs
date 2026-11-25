//! A user's own script, and the command that has their system's shell
//! read it.
//!
//! What a user writes for a shell (`run:` of a step or a service, a
//! health check's `cmd:`) is the one thing here a shell reads, and
//! nothing of the factory's own is ever handed to one. The shell is the
//! system's, found where the system keeps it and not along the daemon's
//! path: whether a user's command can be started must not turn on what
//! whoever started the daemon had on theirs.
//!
//! On every system but Windows that is `/bin/sh -c <script>`: the script
//! is an argument of its own, and is read whole, as many lines as it has.
//!
//! On Windows the shell (`cmd`) takes no arguments as other programs do.
//! It reads the one line it was started with, in its own way, and what
//! the standard library writes for an argument is written for programs
//! that read theirs the usual way (a backslash before a quotation mark),
//! which that shell does not: `echo "a b"` handed over as an argument was
//! read as `echo \"a b\"`. So there the script is put on the shell's line
//! as it is ([`line`], through the door the standard library has for
//! that, `raw_arg`), after `/D /S /C` and between two quotation marks.
//! What that rests on is the shell's own description of itself
//! (`cmd /?`): what follows `/C` is read as a command line, and where
//! `/S` was said and its first character is a quotation mark, that one
//! and the last quotation mark of the line are taken off and the rest is
//! kept as it was written. So what is between the two is read as it
//! would be typed at that shell's prompt: its quotation marks, `&`, `|`,
//! `>`, `^` and `%NAME%` are the shell's to read. (`/D` has it run
//! nothing a registry says to run first.) Read as a line typed, which
//! is not in every way a line of a file of that shell's: a variable a
//! `for` counts by is `%i` there and not `%%i`.
//!
//! One thing follows from that shell reading its line that way, and is
//! the user's to know: it puts what a `%NAME%` holds into the line
//! before it reads the line, so what the variable holds is read as part
//! of the command. A line that writes `%NAME%` of a variable holding
//! what somebody else said (what was said to a bot, `%MESSAGE%`) has it
//! run: `echo heard %MESSAGE%`, told `this& del that`, deletes. `sh`
//! reads a line first and puts a `$NAME` in afterwards, and runs
//! nothing of it. So on Windows such a line names a program that reads
//! the variable itself. Seen there, and held by this crate's suite.
//!
//! One line, since one is all that shell is started with: a script of
//! several is refused before anything is started, in a sentence that
//! says what to write instead ([`line`]). Joining them with `&` would be
//! read, and would not mean the same: the whole line is read for its
//! `%NAME%`s before any of it is run, so that what one command set the
//! next would not see.
//!
//! What is one system's is a function of text and of a folder
//! ([`line`], [`shell`]), with a table of cases played on Linux. On
//! Windows it is run by this crate's suite: that shell is handed lines
//! and what it makes of each is read, and a step and a health check
//! each have it read a line with quotation marks in it.

use std::path::{Path, PathBuf};
use std::process::Command;

/// What `run:` is said to be, wherever an action takes one.
pub(crate) const RUN_ARG: &str =
    "a script of your own for the system's shell: `sh`, or on Windows `cmd`, which is given one line";

/// What the Windows shell is called.
pub const SHELL: &str = "cmd.exe";

/// What that shell is told before its line: to run nothing of the
/// registry's first, to take off the two quotation marks about its line
/// and nothing else, and to read the line and end.
pub const READS: [&str; 3] = ["/D", "/S", "/C"];

/// Where the Windows shell is: in the folder the system keeps its own
/// programs in, under `system` (the system's own folder, which every
/// program there is told as `SystemRoot`), where there is such a file.
/// Where that is not said or holds none, by its name, which the standard
/// library looks for where the system keeps its programs before it
/// looks along any path of the daemon's.
pub fn shell(system: Option<&Path>) -> PathBuf {
    let kept = system.map(|system| system.join("System32").join(SHELL));
    kept.filter(|file| file.is_file()).unwrap_or_else(|| PathBuf::from(SHELL))
}

/// The line the Windows shell is given after [`READS`] for `script`: the
/// script as it was written, between the two quotation marks that shell
/// takes off. Or why there is none, as a sentence.
///
/// * A script of one line is that line, with nothing in it written
///   another way: whatever it holds is the shell's to read.
/// * Lines with nothing on them are not lines of it, so a line with a
///   line's end after it (how a block of a blueprint is read) is one
///   line; and an empty script is an empty line, which does nothing and
///   ends well, as it does for every other shell.
/// * A script of several lines is refused: the shell is started with
///   one.
pub fn line(script: &str) -> Result<String, String> {
    let mut lines =
        script.lines().map(|line| line.trim_end_matches('\r')).filter(|line| !line.trim().is_empty());
    let first = lines.next().unwrap_or_default();
    match lines.count() {
        0 => Ok(format!("\"{first}\"")),
        more => Err(format!(
            "a command of several lines cannot be given to the Windows shell, which reads the one line it is \
             started with (`cmd /C`), and this one has {}. Nothing was started. Put the lines in a file of that \
             shell's in the project and name it (`run: build.cmd`); or write them as one line with `&` between \
             them, which means the same only where no line reads with `%NAME%` what a line before it set",
            more + 1
        )),
    }
}

/// Why `script` cannot be read by this system's shell, where it cannot:
/// asked before anything is started for it.
pub fn refused(script: &str) -> Option<String> {
    if cfg!(windows) { line(script).err() } else { None }
}

/// The command that has this system's shell read `script`: for a command
/// that is its user's own, and for nothing of the factory's. Its
/// environment, where it runs and where its output goes are whoever
/// calls this to say.
pub fn command(script: &str) -> Result<Command, String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let line = line(script)?;
        let system = std::env::var_os("SystemRoot").map(PathBuf::from);
        let mut command = Command::new(shell(system.as_deref()));
        command.args(READS).raw_arg(line);
        Ok(command)
    }
    #[cfg(not(windows))]
    {
        let mut command = Command::new("/bin/sh");
        command.arg("-c").arg(script);
        Ok(command)
    }
}
