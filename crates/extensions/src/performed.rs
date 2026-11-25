//! A package's program run for one doing of one of its actions, and what
//! came of it read with the package's own expressions
//! ([`crate::declared`]). Nothing is run through a shell: all it can run is
//! that one program, in the ways its package wrote down.
//!
//! Which actions there are is asked of the packages installed each time,
//! which is the agents' (`agents::performed::Offered`); this is what a step
//! of one, and a page that shows what a package says it can of a
//! repository (`agents::shown`), each come to.

use conductor::{ActionError, ActionResult};
use hosting::Hosting;
use serde_json::{Map, Value, json};

use crate::declared::Declared;
use crate::extension::Extension;
use crate::program::{self, MOST, Trouble};

/// How many lines of what a program printed are kept with a step's failure.
/// The whole of it is in the step's log.
const KEEP_LINES: usize = 40;

/// Where a package's program is run for one doing of an action, and what
/// it is handed: a step's, or a page's that shows what the package says it
/// can of a repository (`agents::shown`).
pub struct At<'a> {
    pub workdir: std::path::PathBuf,
    pub env: &'a std::collections::BTreeMap<String, String>,
    pub secrets: &'a std::collections::BTreeMap<String, String>,
    /// Who asks, by name, and whether they may give the program an
    /// environment of its own.
    pub asker: &'a str,
    pub may_give: bool,
    pub cancel: &'a kernel::Cancel,
    pub log: &'a (dyn Fn(char, &str) + Send + Sync),
}

/// Who the daemon runs as, as far as its environment says.
pub fn user() -> String {
    ["USER", "USERNAME"].iter().find_map(|name| std::env::var(name).ok()).unwrap_or_default()
}

/// The end of some text, a line at a time.
fn tail(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(KEEP_LINES)..].join("\n")
}

/// Runs a package's program for one action, and reads what came of it.
pub async fn once(
    extension: &Extension,
    action: &Declared,
    hosting: &Hosting,
    at: &At<'_>,
    mut scope: Map<String, Value>,
) -> ActionResult {
    let (name, program, package) = (&action.spec.name, &extension.program, &extension.name);
    let says = |problem: String| ActionError::new(format!("`{name}`: {problem}"));
    if let Some(sentence) = action.refused(&scope).map_err(says)? {
        return Err(ActionError::new(sentence));
    }
    let argv = action.argv(&scope).map_err(says)?;
    (at.log)('s', &format!("{program} {}", argv.join(" ")));

    let file = hosting::named::program(std::ffi::OsStr::new(program), None);
    let mut command = tokio::process::Command::new(&file);
    command.args(&argv);
    if at.workdir.is_dir() {
        command.current_dir(&at.workdir);
    }
    // What a step says of the environment steers the program beyond the
    // ways its package wrote down (where it looks for its settings, which
    // account it uses), so it is said by whoever may run commands.
    if !(at.env.is_empty() && at.secrets.is_empty()) && !at.may_give {
        return Err(ActionError::new(format!(
            "{} may not give `{name}` an environment of its own (`env` needs `shell`)",
            at.asker
        )));
    }
    // A key the step's environment names is the account the program acts
    // as at its service, in place of whoever runs the daemon.
    // What it was handed is taken out of everything it says before that is
    // written down, given to the package to read, or said of a failure.
    let unsaid = hosting
        .dress_handed(&mut command, at.env, at.secrets)
        .map_err(|err| ActionError::new(err.message))?;
    let ran = program::run(command, at.cancel, &|stream, line| (at.log)(stream, &unsaid.from(line))).await;
    let ran = ran.map_err(|trouble| match trouble {
        // Looked for along a path, a program that is nowhere on it is not
        // always reported as not found: a folder on the path that may not
        // be looked into is reported instead. So it is looked for again.
        Trouble::NotStarted(why)
            if why.kind() == std::io::ErrorKind::NotFound || !extension.available() =>
        {
            let more = if extension.install.is_empty() { String::new() } else { format!(" {}", extension.install) };
            ActionError::new(format!(
                "The program `{program}` is not installed on this device, or not where the daemon looks for \
                 programs. Install it and do this again: it is what the extension package `{package}` runs.{more}"
            ))
        }
        Trouble::NotStarted(why) => ActionError::new(format!(
            "The program `{program}`, which the extension package `{package}` runs, could not be started: {}",
            hosting::named::why_not(&file, &why)
        )),
        Trouble::Cancelled => ActionError::cancelled(),
        Trouble::TooMuch => ActionError::new(format!(
            "`{program}` printed more than {} MB, which is more than is read of an answer, and was stopped. \
             Nothing was taken from what it printed.",
            MOST / (1024 * 1024)
        )),
        Trouble::Lost(why) => ActionError::new(format!("`{program}` was started and then lost: {why}")),
    })?;

    let ran = program::Ran { stdout: unsaid.from(&ran.stdout), stderr: unsaid.from(&ran.stderr), ..ran };
    action.ran(&mut scope, ran.code, &ran.stdout, &ran.stderr);
    let said = || json!({ "code": ran.code, "stdout": tail(&ran.stdout), "stderr": tail(&ran.stderr) });
    if let Some(sentence) = action.failed(&scope).map_err(says)? {
        return Err(ActionError::new(sentence).with_data(said()));
    }
    match ran.code {
        Some(0) => action.gives(&scope).map_err(says),
        // The package had nothing to say about it, so the program's own
        // last words are the best explanation there is.
        Some(code) => {
            let last = ran.stderr.lines().last().or_else(|| ran.stdout.lines().last()).unwrap_or_default();
            let detail = if last.is_empty() { String::new() } else { format!(": {last}") };
            Err(ActionError::new(format!("`{program}` exited with code {code}{detail}")).with_data(said()))
        }
        None => Err(ActionError::new(format!(
            "`{program}` ended without an exit code (killed, or its status was lost)"
        ))
        .with_data(said())),
    }
}
