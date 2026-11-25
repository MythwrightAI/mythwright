//! How git is asked, said once.
//!
//! The daemon asks git about a project's repository with no wall around
//! it, and what is in a repository is somebody else's: an agent wrote the
//! worktree, and whoever the project came from wrote its configuration. So
//! there is one way of asking, and everything here that asks git starts
//! from [`git`]:
//!
//! - Nothing of the repository's is run for it. A hook is a file in the
//!   repository, or runs one; so is the program a repository may name to
//!   say what has changed (`core.fsmonitor`).
//! - Nobody is prompted. A remote that wants a password fails at once and
//!   says so, where a prompt nobody sees would wait for ever.
//! - What only reads takes no lock. Left to itself git brings the index up
//!   to date as it reads it (a `status`, a diff of the files) and holds
//!   the index's lock for that moment, which is the lock a merge or a
//!   commit needs and does not wait for. What is given up is the saving:
//!   the next reading looks again at the files whose times had changed,
//!   until somebody's own git writes the index.
//!
//! - Who commits is the repository's to say, and after it whoever runs the
//!   daemon as git knows them. Git takes an author and a committer named
//!   in its environment before either, and the daemon's environment is
//!   whatever it happened to be started in: a shell that names its owner
//!   for their own commits, a hosted runner that names itself, a terminal
//!   an agent was given, which names the agent. A daemon started there
//!   merged, and committed what an agent had left, as that. So those are
//!   taken out of what git is started with ([`WHO`]). Whose work a commit
//!   is, is still said where it is made, in the command (`--author`).
//!
//! Git does not wait at a lock another git holds: it refuses at once. What
//! that looks like in git's words, how long such a lock is waited for and
//! what is said when it is still there after that, are here too
//! ([`lock_met`], [`LOCK_PATIENCE`], [`Waited`], [`lock_kept`]), for
//! whoever asks.
//!
//! What speaks to a remote may be asked as a key the factory keeps, a
//! bot's, in place of whoever runs the daemon ([`git_as_key`]).
//!
//! This builds a command and starts nothing: whoever asks runs it, with
//! whatever else its environment is to hold.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

/// The variable a key is handed to git in, and through git to the helper
/// that answers with it. Whoever starts the command puts the key there as
/// it starts; it is in no argument.
pub const KEY: &str = "FACTORY_REMOTE_KEY";

/// The variable that says which remote a key is for, by the address the
/// repository has for it. The helper gives the key to that address and to
/// nothing a remote sends git on to.
pub const REMOTE: &str = "FACTORY_REMOTE";

/// The variable in which everything the factory starts is told where the
/// product's own program is. The helper git asks is that program.
pub const PROGRAM: &str = "FACTORY_CLI";

/// What in an environment names who commits. Git goes by each of these
/// before it reads what the repository says (`user.name`, `user.email`),
/// so none of them is handed on to it from the daemon's own.
pub const WHO: &[&str] =
    &["GIT_AUTHOR_NAME", "GIT_AUTHOR_EMAIL", "GIT_COMMITTER_NAME", "GIT_COMMITTER_EMAIL"];

/// Git, asked about the repository in `dir`, with nothing said yet of
/// what is asked: the caller adds that.
pub fn git(dir: &Path) -> Command {
    let mut git = Command::new("git");
    git.arg("-C").arg(dir);
    git.args(["-c", "core.hooksPath=/dev/null", "-c", "core.fsmonitor=false"]);
    git.env("GIT_TERMINAL_PROMPT", "0");
    // Git's own word for it: no lock is taken that the asking can do without.
    git.env("GIT_OPTIONAL_LOCKS", "0");
    for name in WHO {
        git.env_remove(name);
    }
    git.stdin(Stdio::null());
    git
}

/// How long a lock that another git holds is waited for, and how often it
/// is looked for meanwhile. A git at work holds one for a moment; one that
/// is still there after this is somebody's long piece of work, or was left
/// behind by a git that was cut off.
pub const LOCK_PATIENCE: Duration = Duration::from_secs(10);
pub const LOCK_LOOK: Duration = Duration::from_millis(100);

/// How long a lock that another git holds has been waited for. The lock
/// is another program's, let go of in the world's time, so it is waited
/// for in that time and never on the factory's clock: where a test held
/// the factory's clock still, a merge that met somebody's `git status`
/// (which holds the index's lock while it runs) waited for ever after it
/// was let go of (`halfdone.rs` in the API suite).
#[derive(Debug, Default)]
pub struct Waited(Option<std::time::Instant>);

impl Waited {
    /// Whether nothing has been waited for yet: what is said the first time.
    pub fn first(&self) -> bool {
        self.0.is_none()
    }

    /// Whether it has been waited for as long as one is ([`LOCK_PATIENCE`]).
    pub fn spent(&self) -> bool {
        self.0.is_some_and(|began| began.elapsed() >= LOCK_PATIENCE)
    }

    /// A moment ([`LOCK_LOOK`]) for the git that holds it to be done.
    pub async fn moment(&mut self) {
        self.0.get_or_insert_with(std::time::Instant::now);
        tokio::time::sleep(LOCK_LOOK).await;
    }
}

/// The lock git said it could not take, where that is why it did not do
/// what was asked: the file, as git named it. Git refuses a lock before it
/// has done anything, so what was refused one can be asked again.
pub fn lock_met(said: &str) -> Option<String> {
    for line in said.lines() {
        if let Some(rest) = line.split("Unable to create '").nth(1)
            && let Some((file, why)) = rest.split_once("': ")
            && file.ends_with(".lock")
            && why.starts_with("File exists")
        {
            return Some(file.to_string());
        }
        if let Some(rest) = line.split("could not lock config file ").nth(1)
            && let Some((file, why)) = rest.rsplit_once(": ")
            && why.starts_with("File exists")
        {
            return Some(format!("{file}.lock"));
        }
    }
    None
}

/// What is said of a lock that was still there when it had been waited
/// for: whose it may be, and what to do.
pub fn lock_kept(lock: &str) -> String {
    format!(
        "git could not take its lock ({lock}) in {} seconds: another git is at work in this repository \
         (somebody's own in the project's folder, or an agent's program in its worktree), or one that was cut \
         off left the lock behind. If no git is running there, remove that file, and do this again",
        LOCK_PATIENCE.as_secs()
    )
}

/// The same, for what speaks to the remote at `address` as a key. Git is
/// told two things, in this order: that it has no helper to ask, and then
/// that it has one, this program. The first is what shuts out whoever runs
/// the daemon: without it git offers theirs. The key itself is handed in
/// [`KEY`] by whoever starts the command.
pub fn git_as_key(dir: &Path, address: &str) -> Command {
    let mut git = git(dir);
    git.args(["-c", "credential.helper="]);
    // Named by its variable and not written out: git hands the line to a
    // shell, and where a program is may hold anything a shell reads.
    git.arg("-c").arg(format!("credential.helper=!\"${PROGRAM}\" credential"));
    git.env(REMOTE, address);
    git
}

/// Whether an address is one a key is given to: a key is something a
/// remote reached over https asks for. Any other is reached as whoever
/// runs the daemon, and a key is never quietly left out.
pub fn takes_a_key(address: &str) -> bool {
    address.starts_with("https://") || address.starts_with("http://")
}

/// What git is asked for the branches of a repository: each local branch
/// and each as a remote holds it, with the commit it is at, when that was
/// committed and its first line, a field apart from the next by a nul so
/// that nothing a subject says is taken for another field. Read by
/// [`branches`].
pub const BRANCHES: &[&str] = &[
    "for-each-ref",
    "--format=%(refname)%00%(objectname)%00%(committerdate:unix)%00%(subject)",
    "refs/heads",
    "refs/remotes",
];

/// A branch of a repository, as a person chooses one.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Branch {
    /// What it is called where it is chosen: `dev`, or as a remote holds
    /// it, `origin/dev`.
    pub name: String,
    /// The remote that holds it, where it is a remote's; none for a local one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    /// What git calls it whole, which is what is asked of git for it.
    pub refname: String,
    /// The commit it is at, whole.
    pub commit: String,
    /// When that commit was made, in milliseconds.
    pub at: i64,
    /// Its first line.
    pub subject: String,
}

/// The branches in what git answered [`BRANCHES`] with: local ones first,
/// then each remote's, each by name. What a remote calls the branch it
/// points at (`origin/HEAD`) is no branch of its own and is left out, and
/// so is a line that does not read as one.
pub fn branches(said: &str) -> Vec<Branch> {
    let mut found: Vec<Branch> = said
        .lines()
        .filter_map(|line| {
            let mut fields = line.splitn(4, '\0');
            let refname = fields.next()?;
            let commit = fields.next()?;
            let at = fields.next()?.trim().parse::<i64>().ok()?;
            let subject = fields.next().unwrap_or_default();
            let (name, remote) = if let Some(local) = refname.strip_prefix("refs/heads/") {
                (local.to_string(), None)
            } else {
                let theirs = refname.strip_prefix("refs/remotes/")?;
                let (remote, branch) = theirs.split_once('/')?;
                if branch == "HEAD" || branch.is_empty() {
                    return None;
                }
                (theirs.to_string(), Some(remote.to_string()))
            };
            if name.is_empty() || commit.is_empty() {
                return None;
            }
            Some(Branch {
                name,
                remote,
                refname: refname.to_string(),
                commit: commit.to_string(),
                at: at * 1000,
                subject: subject.to_string(),
            })
        })
        .collect();
    found.sort_by(|one, other| (one.remote.is_some(), &one.name).cmp(&(other.remote.is_some(), &other.name)));
    found
}

/// What a fetch brought, by the remotes' branches before it and after:
/// those that are new, those at another commit than they were, and those
/// that are gone, each by name. A local branch is never a fetch's doing.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Fetched {
    pub new: Vec<String>,
    pub moved: Vec<String>,
    pub gone: Vec<String>,
}

pub fn fetched(before: &[Branch], after: &[Branch]) -> Fetched {
    let theirs = |branches: &[Branch]| -> std::collections::BTreeMap<String, String> {
        branches
            .iter()
            .filter(|branch| branch.remote.is_some())
            .map(|branch| (branch.name.clone(), branch.commit.clone()))
            .collect()
    };
    let (was, is) = (theirs(before), theirs(after));
    let mut fetched = Fetched::default();
    for (name, commit) in &is {
        match was.get(name) {
            None => fetched.new.push(name.clone()),
            Some(before) if before != commit => fetched.moved.push(name.clone()),
            Some(_) => {}
        }
    }
    fetched.gone = was.keys().filter(|name| !is.contains_key(*name)).cloned().collect();
    fetched
}

impl Fetched {
    /// What it comes to, in a sentence.
    pub fn said(&self) -> String {
        let named =
            |names: &[String]| names.iter().map(|name| format!("`{name}`")).collect::<Vec<_>>().join(", ");
        let mut parts = Vec::new();
        if !self.new.is_empty() {
            parts.push(format!("new: {}", named(&self.new)));
        }
        if !self.moved.is_empty() {
            parts.push(format!("moved on: {}", named(&self.moved)));
        }
        if !self.gone.is_empty() {
            parts.push(format!("gone: {}", named(&self.gone)));
        }
        match parts.is_empty() {
            true => "Fetched: every remote's branches are as they were.".to_string(),
            false => format!("Fetched. {}.", parts.join("; ")),
        }
    }
}

// ---- a repository as it stands -----------------------------------------------------
//
// What a project's Git page shows, read from what git answers: the branch
// the folder is on and how it stands against its upstream, what is changed
// in the working tree, the commits of a branch and the remotes. Each is a
// question asked as git asks it and a function of what it answered, so that
// what it comes to is held without a repository.

/// What git is asked for the branch the folder is on and what is changed
/// there: the porcelain that does not change between versions, each entry
/// ended by a nul so that no path is taken apart. Read by [`status`].
pub const STATUS: &[&str] = &["status", "--porcelain=v2", "--branch", "-z", "--untracked-files=normal"];

/// How many changed paths are kept of a working tree. A folder nothing
/// ignores (a build's output) is one untracked line, but a tree with many
/// changes is said by how many more there are.
pub const MOST_CHANGES: usize = 500;

/// What git is asked for the commits of a branch, newest first: a commit a
/// line, its fields apart by a nul. The branch is put after it, after
/// `--end-of-options`. Read by [`commits`].
pub const COMMITS: &[&str] =
    &["log", "--max-count=50", "--format=%H%x00%an%x00%ae%x00%at%x00%P%x00%s", "--end-of-options"];

/// What git is asked for a repository's remotes: each with where it is
/// fetched from and pushed to. Read by [`remotes`].
pub const REMOTES: &[&str] = &["remote", "-v"];

/// The branch a working tree is on, and how it stands against the branch
/// it follows where it follows one.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Head {
    /// The branch, by name; none where the folder is on a commit and no
    /// branch (detached).
    pub branch: Option<String>,
    /// The commit it is at, whole; none in a repository with no commit yet.
    pub commit: Option<String>,
    /// The branch it follows, as `origin/main`, where it follows one.
    pub upstream: Option<String>,
    /// How many commits it has that the one it follows has not, and the
    /// other way round, where git said.
    pub ahead: Option<u32>,
    pub behind: Option<u32>,
}

/// A path that is changed in the working tree, or in what is staged.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Changed {
    /// Where it is, from the repository's top.
    pub path: String,
    /// Where it was, for one moved or copied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub was: Option<String>,
    /// What is staged of it, in a word (`modified`, `added`, `deleted`,
    /// `renamed`, `copied`, `type changed`, `unmerged`); empty for nothing.
    pub staged: &'static str,
    /// What is changed of it beside what is staged, in a word, as `staged`,
    /// with `untracked` for a file git keeps nothing of.
    pub unstaged: &'static str,
}

/// The working tree as [`STATUS`] says it.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Status {
    pub head: Head,
    /// The changed paths, as many as [`MOST_CHANGES`].
    pub changes: Vec<Changed>,
    /// How many more there were.
    pub more: usize,
}

/// What a letter of a porcelain status means, in a word.
fn changed_as(letter: char) -> &'static str {
    match letter {
        'M' => "modified",
        'A' => "added",
        'D' => "deleted",
        'R' => "renamed",
        'C' => "copied",
        'T' => "type changed",
        'U' => "unmerged",
        _ => "",
    }
}

/// What git answered [`STATUS`] with. An entry that does not read as one
/// is left out; so is one git says it ignores.
pub fn status(said: &str) -> Status {
    let mut found = Status::default();
    let mut entries = said.split('\0');
    let keep = |found: &mut Status, changed: Changed| {
        if found.changes.len() < MOST_CHANGES {
            found.changes.push(changed);
        } else {
            found.more += 1;
        }
    };
    while let Some(entry) = entries.next() {
        if let Some(header) = entry.strip_prefix("# ") {
            let (key, value) = header.split_once(' ').unwrap_or((header, ""));
            match key {
                "branch.oid" if value != "(initial)" && !value.is_empty() => {
                    found.head.commit = Some(value.to_string())
                }
                "branch.head" if value != "(detached)" && !value.is_empty() => {
                    found.head.branch = Some(value.to_string())
                }
                "branch.upstream" if !value.is_empty() => found.head.upstream = Some(value.to_string()),
                "branch.ab" => {
                    let mut counts = value.split(' ');
                    found.head.ahead =
                        counts.next().and_then(|ahead| ahead.trim_start_matches('+').parse().ok());
                    found.head.behind =
                        counts.next().and_then(|behind| behind.trim_start_matches('-').parse().ok());
                }
                _ => {}
            }
            continue;
        }
        let kind = entry.chars().next();
        // Fields before the path, by kind: an ordinary change has eight, a
        // move or copy nine (its path is followed by where it was, as the
        // next entry), and one in the middle of a merge ten.
        let (fields, moved) = match kind {
            Some('1') => (8, false),
            Some('2') => (9, true),
            Some('u') => (10, false),
            Some('?') => {
                let path = entry[1..].trim_start();
                if !path.is_empty() {
                    keep(
                        &mut found,
                        Changed { path: path.to_string(), was: None, staged: "", unstaged: "untracked" },
                    );
                }
                continue;
            }
            _ => continue,
        };
        let parts: Vec<&str> = entry.splitn(fields + 1, ' ').collect();
        let (Some(xy), Some(path)) = (parts.get(1), parts.get(fields)) else { continue };
        let mut letters = xy.chars();
        let (x, y) = (letters.next().unwrap_or('.'), letters.next().unwrap_or('.'));
        let was = if moved { entries.next().filter(|was| !was.is_empty()).map(str::to_string) } else { None };
        if path.is_empty() {
            continue;
        }
        let (staged, unstaged) =
            if kind == Some('u') { ("unmerged", "unmerged") } else { (changed_as(x), changed_as(y)) };
        keep(&mut found, Changed { path: path.to_string(), was, staged, unstaged });
    }
    found
}

/// A commit, as a list of them shows one.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Commit {
    /// Whole.
    pub commit: String,
    pub author: String,
    pub email: String,
    /// When it was authored, in milliseconds.
    pub at: i64,
    /// The commits it was made on, whole: two or more for a merge.
    pub parents: Vec<String>,
    /// Its first line.
    pub subject: String,
}

/// The commits in what git answered [`COMMITS`] with, in the order it
/// said them. A line that does not read as one is left out.
pub fn commits(said: &str) -> Vec<Commit> {
    said.lines()
        .filter_map(|line| {
            let mut fields = line.splitn(6, '\0');
            let commit = fields.next()?.trim();
            let author = fields.next()?;
            let email = fields.next()?;
            let at = fields.next()?.trim().parse::<i64>().ok()?;
            let parents = fields.next()?.split_whitespace().map(str::to_string).collect();
            let subject = fields.next().unwrap_or_default();
            (!commit.is_empty()).then(|| Commit {
                commit: commit.to_string(),
                author: author.to_string(),
                email: email.to_string(),
                at: at * 1000,
                parents,
                subject: subject.to_string(),
            })
        })
        .collect()
}

/// A remote of a repository.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Remote {
    pub name: String,
    /// Where it is fetched from, as [`shown`] says an address.
    pub fetch: String,
    /// Where it is pushed to, where that is another address than `fetch`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub push: Option<String>,
}

/// An address of a remote as it may be shown and handed on: without what
/// goes before an `@` in one written with `://`, which is where a name and
/// a password, or a token, are written into an address. One written as
/// `user@host:path` names an account and no secret, and is as it was.
pub fn shown(address: &str) -> String {
    let Some((scheme, rest)) = address.split_once("://") else { return address.to_string() };
    let end = rest.find('/').unwrap_or(rest.len());
    match rest[..end].rfind('@') {
        Some(at) => format!("{scheme}://{}", &rest[at + 1..]),
        None => address.to_string(),
    }
}

/// The remotes in what git answered [`REMOTES`] with, in the order it
/// said them, each address as [`shown`] says it.
pub fn remotes(said: &str) -> Vec<Remote> {
    let mut found: Vec<Remote> = Vec::new();
    for line in said.lines() {
        let Some((name, rest)) = line.split_once('\t') else { continue };
        let (address, which) = match rest.rsplit_once(' ') {
            Some((address, which)) => (address.trim(), which.trim()),
            None => (rest.trim(), "(fetch)"),
        };
        if name.is_empty() || address.is_empty() {
            continue;
        }
        let address = shown(address);
        let at = match found.iter().position(|remote| remote.name == name) {
            Some(at) => at,
            None => {
                found.push(Remote { name: name.to_string(), fetch: String::new(), push: None });
                found.len() - 1
            }
        };
        match which {
            "(push)" => found[at].push = Some(address),
            _ => found[at].fetch = address,
        }
    }
    for remote in &mut found {
        if remote.fetch.is_empty() {
            remote.fetch = remote.push.take().unwrap_or_default();
        } else if remote.push.as_deref() == Some(remote.fetch.as_str()) {
            remote.push = None;
        }
    }
    found
}
