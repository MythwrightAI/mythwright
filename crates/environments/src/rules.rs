//! What an environment holds to (`Environment::rules`), kept whole with it, and what is made under it comes to
//! by them, as plain values.
//!
//! Two rules, each saying nothing where it is empty:
//! - `approve`: the people (`user:<account>`) of whom one approves what
//!   is started under it by somebody who is not one of them. Such a thing
//!   is started only by a run that holds a go-ahead naming the
//!   environment, which one of them gave (`approval.ask`, `kernel::go`);
//!   one of them starts what they like under it. Somebody who presses Run
//!   and is not one of them, with no run to wait in, is refused.
//! - `branches`: the branches what is started under it may be of, where
//!   it is handed one (the param `branch`, as an environment's Run hands
//!   its deploy): each a branch's whole name as it is chosen (`main`, or as
//!   a remote holds it, `origin/main`), or a beginning of names ended by
//!   `*` (`release/*`).
//!
//! They are held where something comes to run under the environment (an
//! apply that names it, from outside or by a step's `environment:`), in
//! the turn that holds the environment there (`Held::holding`); what is
//! started by what already runs under it is under it already, and is not
//! held again. Saying them takes what changing the environment's secrets
//! takes.

use serde::{Deserialize, Serialize};

/// What an environment holds to.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rules {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub approve: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branches: Vec<String>,
}

/// The most people an environment's rule names as approving.
pub const MOST_APPROVING: usize = 50;

/// The most branches an environment's rule names.
pub const MOST_BRANCHES: usize = 50;

/// What is to come to run under an environment, as its rules ask of it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Entering<'a> {
    /// Whom it acts for, as an actor (`user:<account>`, `agent:<name>`).
    pub by: &'a str,
    /// The branch it is handed, where it is handed one.
    pub branch: Option<&'a str>,
    /// The run that asks for it, where a step of one does.
    pub run: Option<&'a str>,
}

impl<'a> Entering<'a> {
    /// What acts for `by`, handed `params`, comes to: the branch is its
    /// param `branch`, as an environment's Run hands its deploy one, and
    /// none where that is empty. `run` is the run of the step that asks.
    pub fn of(by: &'a str, params: &'a kernel::Fields, run: Option<&'a str>) -> Entering<'a> {
        let branch =
            params.get("branch").and_then(serde_json::Value::as_str).filter(|branch| !branch.is_empty());
        Entering { by, branch, run }
    }
}

/// What the rules say of something entering, before any go-ahead is read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Comes {
    /// It may.
    Goes,
    /// Only with a go-ahead of the run that asks, given by one of these.
    Waits(Vec<String>),
    /// It may not, and why.
    Refused(String),
}

impl Rules {
    pub fn is_empty(&self) -> bool {
        self.approve.is_empty() && self.branches.is_empty()
    }

    /// Whether `branch` is one the rule lets run (`*` at the end of one
    /// it names is any branch beginning so); every branch where it names
    /// none.
    pub fn lets(&self, branch: &str) -> bool {
        self.branches.is_empty()
            || self.branches.iter().any(|named| match named.strip_suffix('*') {
                Some(beginning) => branch.starts_with(beginning),
                None => named == branch,
            })
    }

    /// What `entering` comes to under the environment `name` of `project`.
    pub fn comes(&self, project: &str, name: &str, entering: &Entering<'_>) -> Comes {
        if let Some(branch) = entering.branch
            && !self.lets(branch)
        {
            return Comes::Refused(format!(
                "`{branch}` may not be run under `{name}` of `{project}`, which lets {} alone: say one of those, \
                 or whoever may change its secrets says another (`{} project environment {project} {name} \
                 branches …`)",
                self.branches.join(", "),
                brand::CLI
            ));
        }
        match self.approve.is_empty() || self.approve.iter().any(|one| one == entering.by) {
            true => Comes::Goes,
            false => Comes::Waits(self.approve.clone()),
        }
    }
}

/// What is said of something started under `name` of `project` by
/// somebody its rule does not name, with no go-ahead: by a run (`run`),
/// or by whoever asked with no run to wait in.
pub fn unapproved(project: &str, name: &str, run: bool) -> String {
    let way = match run {
        true => format!(
            "this run holds no go-ahead for it: a step of `approval.ask` with `environment: {name}` before the \
             one that starts it waits for one of them"
        ),
        false => {
            "and you are not one of them: one of them runs it, or a run that waits for their go-ahead does \
                  (`approval.ask`)"
                .to_string()
        }
    };
    format!("what is started under `{name}` of `{project}` is approved by those its rule names, {way}")
}

/// Why `branch` cannot be a branch an environment's rule names, if it
/// cannot: a branch's name as git takes one, with `*` only at its end.
pub fn branch_refused(branch: &str) -> Option<String> {
    let name = branch.strip_suffix('*').unwrap_or(branch);
    let shaped = !branch.is_empty()
        && branch.len() <= 200
        && !name.contains(['*', ' ', '~', '^', ':', '?', '[', '\\'])
        && !name.contains("..")
        && !name.starts_with(['/', '-'])
        && !name.chars().any(char::is_control);
    (!shaped).then(|| {
        format!(
            "`{branch}` cannot be a branch an environment lets run: a branch is named as git names one (`main`, \
             `origin/main`), and `*` at its end is any branch beginning so (`release/*`)"
        )
    })
}
