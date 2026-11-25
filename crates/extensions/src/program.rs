//! Running a package's program once, to its end, and keeping all it said.
//!
//! An action a package declares is one run of its program with a list of
//! arguments. There is no shell anywhere in that: the program is started
//! as it is named, with the arguments as they are, which is the same thing
//! on every system. All of what it prints is kept, because the answer is
//! the whole of it; that is bounded, so a program that never stops talking
//! cannot fill the daemon. It is killed, with whatever it started, when the
//! step it runs for is cancelled, times out or is otherwise let go of.

use std::process::Stdio;
use std::time::Duration;

use kernel::Cancel;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::adapter;

/// How much of what a program prints is read, on each of its two streams.
/// An answer is a status or a list, not a download.
pub const MOST: usize = 4 * 1024 * 1024;

/// How long the output of a program that has ended is still read for. It
/// ends with the program, so this is only ever waited out when something
/// the program started is still holding it open.
const LEFT_OPEN: Duration = Duration::from_secs(2);

/// How a run ended, and everything it printed.
pub struct Ran {
    /// `None` for a program that was killed, or whose status was lost.
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Why there is nothing to read an answer from.
pub enum Trouble {
    /// It could not be started: most often because there is no such
    /// program where the daemon looks for programs.
    NotStarted(std::io::Error),
    /// The step it was run for was cancelled.
    Cancelled,
    /// It printed more than is read.
    TooMuch,
    Lost(String),
}

/// One stream of a program's output as it arrives.
struct Stream<R> {
    from: Option<R>,
    kept: Vec<u8>,
    /// How much of `kept` has been handed on as whole lines.
    told: usize,
    name: char,
}

impl<R: AsyncRead + Unpin> Stream<R> {
    fn new(from: Option<R>, name: char) -> Self {
        Self { from, kept: Vec::new(), told: 0, name }
    }

    /// Waits for more. Never comes back for a stream that has ended, so
    /// that it can be waited on beside others.
    async fn more(&mut self) -> std::io::Result<usize> {
        match &mut self.from {
            Some(from) => from.read_buf(&mut self.kept).await,
            None => std::future::pending().await,
        }
    }

    /// Takes in what a read brought. `Err` when that is more than is read.
    fn took(
        &mut self,
        read: std::io::Result<usize>,
        line: &(dyn Fn(char, &str) + Send + Sync),
    ) -> Result<(), Trouble> {
        match read {
            // The end of it, or a stream that broke: either way, no more.
            Ok(0) | Err(_) => self.from = None,
            Ok(_) => {}
        }
        if self.kept.len() > MOST {
            return Err(Trouble::TooMuch);
        }
        while let Some(end) = self.kept[self.told..].iter().position(|byte| *byte == b'\n') {
            line(self.name, String::from_utf8_lossy(&self.kept[self.told..self.told + end]).trim_end());
            self.told += end + 1;
        }
        Ok(())
    }

    /// Everything, once no more is coming: the last line is handed on even
    /// if it was never ended.
    fn all(self, line: &(dyn Fn(char, &str) + Send + Sync)) -> String {
        if self.told < self.kept.len() {
            line(self.name, String::from_utf8_lossy(&self.kept[self.told..]).trim_end());
        }
        String::from_utf8_lossy(&self.kept).into_owned()
    }
}

/// Runs `command` to its end. `line` is given each line as it is printed,
/// with `o` or `e` for which stream it was on.
pub async fn run(
    mut command: tokio::process::Command,
    cancel: &Cancel,
    line: &(dyn Fn(char, &str) + Send + Sync),
) -> Result<Ran, Trouble> {
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    // A group of its own, so that what it starts can be stopped with it.
    hosting::group::leads(command.as_std_mut());
    let mut child = adapter::try_spawn(&mut command).await.map_err(Trouble::NotStarted)?;
    // Dropped on every way out of here but the last, which is the one
    // where the program has been seen to end.
    let group = adapter::Group::of(&child);
    let mut out = Stream::new(child.stdout.take(), 'o');
    let mut err = Stream::new(child.stderr.take(), 'e');
    let status = loop {
        tokio::select! {
            read = out.more() => out.took(read, line)?,
            read = err.more() => err.took(read, line)?,
            status = child.wait() => break status,
            () = cancel.cancelled() => return Err(Trouble::Cancelled),
        }
    };
    // It has ended, so all it wrote is written, and reading the rest ends
    // at once. Unless something it started and left running holds its
    // output open, which could be for ever: that is not waited for.
    let rest = async {
        while out.from.is_some() || err.from.is_some() {
            tokio::select! {
                read = out.more() => out.took(read, line)?,
                read = err.more() => err.took(read, line)?,
            }
        }
        Ok::<(), Trouble>(())
    };
    if let Ok(read) = tokio::time::timeout(LEFT_OPEN, rest).await {
        read?;
    }
    group.spare();
    let status = status.map_err(|err| Trouble::Lost(err.to_string()))?;
    Ok(Ran { code: status.code(), stdout: out.all(line), stderr: err.all(line) })
}
