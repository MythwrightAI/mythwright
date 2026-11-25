//! A place of its own for a piece of work, in a project's repository.
//!
//! Something an agent works on apart from everything else has a place: a
//! worktree of the project at `.lanes/<item>`, on a branch `lane/<item>`,
//! called after whatever names the work (`item`: a card's id, where it is a
//! card). Making that place, keeping what was left in it, sending the
//! branch to a remote, merging it and clearing it away are each several
//! things asked of git, where what is asked next rests on what git
//! answered. That is worked out here, one action for each, and a blueprint
//! says only when it is done and what becomes of the answer.
//!
//! Nothing here knows what the work is, who asked for it or what becomes
//! of it: that is the blueprint's (`std/lane-round`, `std/work`,
//! `std/review`).
//!
//! Git is asked the one way there is (`kernel::git`): nothing of the
//! repository's is run for it, and nobody is prompted. What speaks to a
//! remote is asked as the key the step was handed, where it was handed one
//! (`env: { FACTORY_REMOTE_KEY: { secret: <name> } }`), and the key goes
//! to that one command: nothing else asked of git here is given it.
//!
//! Each action looks at what is there before it changes anything, so one
//! that a restart interrupted is done again from its start.
//!
//! Git is not stopped in the middle of what it is doing where that can be
//! helped: a daemon that is asked to stop asks git nothing more and lets
//! what is being asked come to its end first, for a while
//! ([`Places::settle`]). Where git was cut off all the same (killed with
//! the daemon, or the machine went), what it left half done is put right
//! by the action that next comes to it, and only where it is this
//! factory's own: a worktree of the item's that git was still making is
//! taken away and made again (`place.make`), a lock left on the index of
//! the item's own place is cleared (`place.keep`), and a merge of the
//! item's own branch that was never committed is called off and made
//! again (`place.merge`). A lock in the project's own folder is not
//! touched: a person works there, and it may be theirs.
//!
//! Two gits in one repository do not wait for each other: the second to
//! want a lock is refused. So what writes takes the repository's turn
//! among the daemon's own (`Place::turn`), a lock that somebody else's git
//! holds is waited for and what was refused asked again (`ask`), and a
//! merge that fails for any reason is called off, so that the project's
//! files and index are as they were (`merge`). What that rests on is in
//! [`asking`] and `kernel::git`.
//!
//! What a project's Git page shows of its repository as it stands, and
//! being told that it may have changed, is here too ([`seen`]): it only
//! reads, and takes no turn.
//!
//! Only these make and remove such a worktree. Agents work inside one and
//! never touch the others: a repository's worktree list is shared state,
//! and one agent tidying it once unregistered every one.

use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use conductor::{ActionCx, ActionError, ActionResult};
use kernel::{Fields, git};
use serde_json::{Value, json};

mod asking;
pub mod branches;
mod follows;
mod forwarded;
mod gathered;
mod registered;
pub mod seen;
mod taken;

use asking::Asking;
pub use registered::register;

/// What the actions here are handed: where programs are started, and what
/// is being asked of git for places just now. One for a factory, held by
/// whoever registers the actions (`agents`).
#[derive(Clone)]
pub struct Places {
    hosting: hosting::Hosting,
    git: Arc<Asking>,
    /// Whose repository somebody has looked at ([`seen::Looked`]).
    looked: Arc<seen::Looked>,
    /// Whose branches something asked to follow (`branches`).
    branches: Arc<branches::Followed>,
}

impl Places {
    pub fn new(hosting: hosting::Hosting) -> Places {
        Places { hosting, git: Arc::default(), looked: Arc::default(), branches: Arc::default() }
    }

    /// Whose repository somebody has looked at, which is followed whole.
    pub fn looked(&self) -> Arc<seen::Looked> {
        self.looked.clone()
    }

    /// Whose branches something asked to follow, for whoever follows them.
    pub fn branches(&self) -> Arc<branches::Followed> {
        self.branches.clone()
    }
}

/// The folder of a project its places are under, and what their branches
/// are called before the item's name. Kept as they have always been, so
/// that work begun before goes on where it is.
const UNDER: &str = ".lanes";
const BRANCHES: &str = "lane";

/// Lines of what git said that an answer keeps; all of it is in the step's log.
const KEEP_LINES: usize = 40;

/// What git answered: how it ended, and the last of what it said.
struct Said {
    /// Nothing, where it could not be started or was ended from outside.
    code: Option<i32>,
    out: String,
    err: String,
}

impl Said {
    fn is(&self, code: i32) -> bool {
        self.code == Some(code)
    }

    fn ok(&self) -> bool {
        self.is(0)
    }

    /// Why it did not do what was asked: how it ended, and the last thing
    /// it said.
    fn why(&self) -> String {
        let last = self.err.lines().last().or_else(|| self.out.lines().last()).unwrap_or_default();
        match self.code {
            Some(code) if last.is_empty() => format!("git exited with code {code}"),
            Some(code) => format!("git exited with code {code}: {last}"),
            None if last.is_empty() => "git ended without saying how".to_string(),
            None => last.to_string(),
        }
    }

    /// What it said of why, in its own words where it said anything.
    fn words(&self) -> String {
        if self.err.is_empty() { self.why() } else { self.err.clone() }
    }
}

/// What a remote has, as far as it was asked.
#[derive(Default)]
struct Theirs {
    /// Whether it has a branch for the item already.
    has: bool,
    /// Its branch that new work is cut from, where that has everything the
    /// project's folder has.
    from: Option<String>,
}

/// One item's place, for the step that asked.
struct Place {
    cx: ActionCx,
    hosting: hosting::Hosting,
    git: Arc<Asking>,
    /// The project's folder, which is its repository.
    project: PathBuf,
    item: String,
}

impl Place {
    fn of(places: &Places, cx: ActionCx, args: &Fields) -> Result<Place, ActionError> {
        let Some(item) = named(args, "item")? else {
            return Err(ActionError::new(
                "`item` is what the work is known by, such as a card's id, `shop-1`, and is said: its folder and \
                 its branch are called after it",
            ));
        };
        let (hosting, git) = (places.hosting.clone(), places.git.clone());
        Ok(Place { hosting, git, project: cx.workdir(), cx, item })
    }

    /// Where the item is worked, from the top of the project.
    fn place(&self) -> String {
        format!("{UNDER}/{}", self.item)
    }

    fn branch(&self) -> String {
        format!("{BRANCHES}/{}", self.item)
    }

    /// The branch of another item, `other`, where the repository has one.
    async fn branch_of(&self, other: &str) -> Result<Option<String>, ActionError> {
        let branch = format!("{BRANCHES}/{other}");
        let head = format!("refs/heads/{branch}");
        Ok(self
            .ask(&self.project, &["rev-parse", "--verify", "--quiet", &head])
            .await?
            .ok()
            .then_some(branch))
    }

    /// The secret the step was handed for the remote, by its name.
    fn key(&self) -> Option<&str> {
        self.cx.secrets().get(git::KEY).map(String::as_str)
    }

    /// A folder said from the top of the project, which it must be within.
    fn within(&self, dir: &str) -> Result<PathBuf, ActionError> {
        let inside =
            Path::new(dir).components().all(|part| matches!(part, Component::Normal(_) | Component::CurDir));
        if !inside {
            return Err(ActionError::new(format!(
                "`dir` is a folder within the project, written from its top, such as `{}`; `{dir}` would be \
                 outside it",
                self.place()
            )));
        }
        Ok(self.project.join(dir))
    }

    fn note(&self, words: &str) {
        self.cx.log('s', words);
    }

    /// Asks git one thing of the repository in `dir`. Whatever git answers
    /// is an answer, one that could not be started included; only being
    /// told to stop is not.
    ///
    /// Refused a lock that another git holds, it had done nothing, and is
    /// asked again until the lock is free: a git at work holds one for a
    /// moment. Past the time such a lock is waited for, what git said is
    /// the answer, with whose the lock may be and what to do.
    async fn ask(&self, dir: &Path, asked: &[&str]) -> Result<Said, ActionError> {
        self.patiently(|| git::git(dir), asked, false).await
    }

    /// Asks once, whatever is in the way: for what has itself to put
    /// something right before it is asked again.
    async fn ask_once(&self, dir: &Path, asked: &[&str]) -> Result<Said, ActionError> {
        self.run(git::git(dir), asked, false).await
    }

    /// Asks once and keeps all that git wrote, for what is handed on whole
    /// (a patch): its lines are not each written in the step's log.
    async fn read(&self, dir: &Path, asked: &[&str]) -> Result<Said, ActionError> {
        self.run_with(git::git(dir), asked, false, true).await
    }

    async fn patiently(
        &self,
        command: impl Fn() -> std::process::Command,
        asked: &[&str],
        handed: bool,
    ) -> Result<Said, ActionError> {
        let mut waited = git::Waited::default();
        loop {
            let mut said = self.run(command(), asked, handed).await?;
            let Some(lock) = git::lock_met(&said.err) else { return Ok(said) };
            if waited.spent() {
                said.err = format!("{}\n{}", said.err, git::lock_kept(&lock));
                return Ok(said);
            }
            self.waits_for(&lock, &mut waited).await?;
        }
    }

    /// A moment for the git that holds `lock` to be done, said the first
    /// time: in the world's time, as that git keeps it (`git::Waited`).
    async fn waits_for(&self, lock: &str, waited: &mut git::Waited) -> Result<(), ActionError> {
        if waited.first() {
            self.note(&format!("another git has {lock}: this waits for it and asks again"));
        }
        tokio::select! {
            () = waited.moment() => Ok(()),
            () = self.cx.cancel().cancelled() => Err(ActionError::cancelled()),
        }
    }

    /// The repository's turn among the daemon's own gits, for an action
    /// that writes there: held until the action is over.
    async fn turn(&self) -> Result<tokio::sync::OwnedMutexGuard<()>, ActionError> {
        // Its worktrees share one folder, which is what the turn is of.
        // Where there is no repository, the project's folder stands for it.
        let common = self.ask(&self.project, &["rev-parse", "--git-common-dir"]).await?;
        let folder = if common.ok() { self.project.join(common.out.trim()) } else { self.project.clone() };
        let turn = self.git.turn(&kernel::whole::whole(&folder).unwrap_or(folder));
        if let Ok(free) = turn.clone().try_lock_owned() {
            return Ok(free);
        }
        self.note("another of the daemon's own gits is at work in this repository: this waits its turn");
        tokio::select! {
            turn = turn.lock_owned() => Ok(turn),
            () = self.cx.cancel().cancelled() => Err(ActionError::cancelled()),
        }
    }

    /// The same of what speaks to a remote: asked as the key the step was
    /// handed, where `address` is said, and otherwise as whoever runs the
    /// daemon. This is the one command the key is in the environment of.
    async fn ask_remote(
        &self,
        dir: &Path,
        address: Option<&str>,
        asked: &[&str],
    ) -> Result<Said, ActionError> {
        match address {
            Some(address) => self.patiently(|| git::git_as_key(dir, address), asked, true).await,
            None => self.ask(dir, asked).await,
        }
    }

    /// Asks, and takes anything but success as why the action fails.
    async fn does(&self, dir: &Path, asked: &[&str]) -> Result<Said, ActionError> {
        let said = self.ask(dir, asked).await?;
        if said.ok() { Ok(said) } else { Err(ActionError::new(said.why())) }
    }

    async fn run(
        &self,
        git: std::process::Command,
        asked: &[&str],
        handed: bool,
    ) -> Result<Said, ActionError> {
        self.run_with(git, asked, handed, false).await
    }

    async fn run_with(
        &self,
        mut git: std::process::Command,
        asked: &[&str],
        handed: bool,
        whole: bool,
    ) -> Result<Said, ActionError> {
        git.args(asked);
        let mut git = tokio::process::Command::from(git);
        git.stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
        self.note(&format!("git {}", asked.join(" ")));
        let unasked = |why: String| Said { code: None, out: String::new(), err: why };
        // What git says of a remote it was refused at may hold what it was
        // refused with: the key is taken out of every line before it is
        // written down or given back.
        let unsaid = if handed {
            // A key that is not kept is why this is not asked at all.
            match self.hosting.dress_handed(&mut git, self.cx.env(), self.cx.secrets()) {
                Ok(unsaid) => unsaid,
                Err(err) => return Ok(unasked(err.message)),
            }
        } else {
            self.hosting.dress(&mut git, self.cx.env());
            hosting::unsaid::Unsaid::default()
        };
        // A daemon that is stopping asks git nothing more: what would be
        // begun now would be cut off. This waits, and goes with the rest.
        let Some(_in_flight) = self.git.begin() else { return std::future::pending().await };
        // Dropped unanswered, the command is ended: a step that is stopped
        // leaves no git running.
        let answered = tokio::select! {
            answered = git.output() => answered,
            () = self.cx.cancel().cancelled() => return Err(ActionError::cancelled()),
        };
        let answered = match answered {
            Ok(answered) => answered,
            Err(err) => return Ok(unasked(format!("git could not be started: {err}"))),
        };
        let kept = |stream: char, bytes: &[u8]| {
            let text = unsaid.from(&String::from_utf8_lossy(bytes));
            if whole {
                self.cx.log(stream, &format!("{} bytes", text.len()));
                return text;
            }
            let lines: Vec<&str> = text.lines().collect();
            for line in &lines {
                self.cx.log(stream, line);
            }
            lines[lines.len().saturating_sub(KEEP_LINES)..].join("\n")
        };
        Ok(Said {
            code: answered.status.code(),
            out: kept('o', &answered.stdout),
            err: kept('e', &answered.stderr),
        })
    }

    /// Where git keeps what it knows of the worktree at `place`, if git
    /// was cut off while it was still making it. Git locks a worktree it
    /// is making, in a word of its own, until it is whole: so one that is
    /// locked so has had nobody at work in it, and whatever is there is
    /// part of a checkout. One that somebody locked for a reason of their
    /// own is theirs.
    async fn half_made(&self, place: &str) -> Result<Option<PathBuf>, ActionError> {
        let common = self.ask(&self.project, &["rev-parse", "--git-common-dir"]).await?;
        if !common.ok() {
            return Ok(None);
        }
        let kept = self.project.join(common.out.trim()).join("worktrees");
        // Git writes down where each worktree's own `.git` is: this one's.
        let here = [Some(self.project.clone()), kernel::whole::whole(&self.project).ok()];
        let ours = |written: &str| {
            here.iter().flatten().any(|project| Path::new(written.trim()) == project.join(place).join(".git"))
        };
        let Ok(mut entries) = tokio::fs::read_dir(&kept).await else { return Ok(None) };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let of = entry.path();
            let (at, locked) = (
                tokio::fs::read_to_string(of.join("gitdir")).await,
                tokio::fs::read_to_string(of.join("locked")).await,
            );
            if at.is_ok_and(|at| ours(&at)) && locked.is_ok_and(|why| why.trim() == "initializing") {
                return Ok(Some(of));
            }
        }
        Ok(None)
    }

    /// Takes away a worktree of the item's own that git never finished
    /// making, so that it is made again: by git where git will, and
    /// otherwise its folder and what git kept of it.
    async fn retake(&self, place: &str, kept: &Path) -> Result<(), ActionError> {
        self.note(&format!("{place} was being made when git was cut off; it is taken away and made again"));
        self.ask(&self.project, &["worktree", "unlock", place]).await?;
        if !self.ask(&self.project, &["worktree", "remove", "--force", place]).await?.ok() {
            let _ = tokio::fs::remove_dir_all(self.project.join(place)).await;
            let _ = tokio::fs::remove_dir_all(kept).await;
        }
        Ok(())
    }

    /// Clears a lock left on the index of the item's own place. Only what
    /// is asked of git here works in that place, one thing at a time, and
    /// its agent has finished: a lock that is there now was left by a git
    /// that was cut off, and nothing would ever take it away.
    async fn unlocked(&self, at: &Path) -> Result<(), ActionError> {
        let dir = self.ask(at, &["rev-parse", "--git-dir"]).await?;
        if !dir.ok() {
            return Ok(());
        }
        let lock = at.join(dir.out.trim()).join("index.lock");
        if tokio::fs::try_exists(&lock).await.unwrap_or(false) && tokio::fs::remove_file(&lock).await.is_ok()
        {
            self.note("a lock was left on its index by a git that was cut off; it is cleared");
        }
        Ok(())
    }

    /// The repository is told that the lanes are not the project's files,
    /// in a list that is the lanes' own and is never committed, since it
    /// leaves itself out too.
    async fn ignored(&self) {
        let lanes = self.project.join(".lanes");
        let list = lanes.join(".gitignore");
        if tokio::fs::try_exists(&list).await.unwrap_or(false) {
            return;
        }
        let written = async {
            tokio::fs::create_dir_all(&lanes).await?;
            tokio::fs::write(&list, "*\n").await
        };
        if let Err(err) = written.await {
            self.note(&format!("{} could not be written: {err}", list.display()));
        }
    }

    /// What the remote has, asked now; or why it could not be asked.
    async fn theirs(
        &self,
        remote: &str,
        base: Option<String>,
    ) -> Result<Result<Theirs, String>, ActionError> {
        let project = &self.project;
        // Where the remote is, when it is asked with a key, which is given
        // there and nowhere else.
        let address = match self.key() {
            Some(key) => {
                let said = self.ask(project, &["remote", "get-url", remote]).await?;
                if !said.ok() {
                    return Ok(Err(said.why()));
                }
                let address = said.out.trim().to_string();
                if !git::takes_a_key(&address) {
                    return Ok(Err(format!(
                        "{remote} is not reached over https, and the key {key} is only given to a remote that \
                         is. Give it an https address (git remote set-url {remote} https://...), or take `key` \
                         away and it is asked as whoever runs the daemon"
                    )));
                }
                Some(address)
            }
            None => None,
        };
        let mut fetch = vec!["fetch", "--quiet"];
        // What the repository has inside it from somewhere else is not
        // asked for with the key.
        if address.is_some() {
            fetch.push("--no-recurse-submodules");
        }
        fetch.push(remote);
        let fetched = self.ask_remote(project, address.as_deref(), &fetch).await?;
        if !fetched.ok() {
            return Ok(Err(fetched.why()));
        }
        // The branch new work is cut from: the one the project's folder has
        // checked out, when none was said.
        let base = match base {
            Some(base) => base,
            None => {
                let here = self.ask(project, &["rev-parse", "--abbrev-ref", "HEAD"]).await?;
                if !here.ok() {
                    return Ok(Err(here.why()));
                }
                here.out.trim().to_string()
            }
        };
        let of_theirs = |name: &str| format!("refs/remotes/{remote}/{name}");
        let known = |said: &Said| said.is(0) || said.is(1);
        let has =
            self.ask(project, &["rev-parse", "--verify", "--quiet", &of_theirs(&self.branch())]).await?;
        let based = self.ask(project, &["rev-parse", "--verify", "--quiet", &of_theirs(&base)]).await?;
        for said in [&has, &based] {
            if !known(said) {
                return Ok(Err(said.why()));
            }
        }
        // Whether that branch has everything the project's folder has.
        // Where the folder has work the remote has not (an item merged here
        // and not sent), new work is cut from the folder, or it would
        // begin without it.
        let mut from = None;
        if based.ok() {
            let ahead =
                self.ask(project, &["merge-base", "--is-ancestor", "HEAD", &of_theirs(&base)]).await?;
            if !known(&ahead) {
                return Ok(Err(ahead.why()));
            }
            from = ahead.ok().then(|| format!("{remote}/{base}"));
        }
        Ok(Ok(Theirs { has: has.ok(), from }))
    }
}

/// The item's place to work in, made or found: a worktree on a branch of
/// its own, cut from the project as it is now, so that an item which waited
/// on others begins from what they left.
async fn make(it: &Place, args: &Fields) -> ActionResult {
    let came =
        |dir: &str, how: &str, unasked: &str| Ok(json!({ "dir": dir, "came": how, "unasked": unasked }));
    // A place the item already has, said by whoever made it, is used.
    if let Some(dir) = text(args, "dir") {
        return came(&dir, "given", "");
    }
    let project = &it.project;
    // Where there is no repository, or none with history to branch from,
    // the item is worked in the project itself.
    if !flag(args, "worktree", true)
        || !it.ask(project, &["rev-parse", "--verify", "--quiet", "HEAD"]).await?.ok()
    {
        return came("", "none", "");
    }
    it.ignored().await;
    let (place, branch) = (it.place(), it.branch());
    let remote = text(args, "remote").unwrap_or_else(|| "origin".to_string());
    // Another item whose branch new work is cut from, where it has one: a
    // piece of work done several ways at once begins each from what the
    // item had so far.
    let from = match named(args, "from")? {
        Some(from) if from == it.item => {
            return Err(ActionError::new(format!(
                "`from` names {from} itself: a place is cut from another's"
            )));
        }
        Some(from) => it.branch_of(&from).await?,
        None => None,
    };
    // Where that was asked for, the remote is asked what it has, before
    // the item's place is made. A remote that cannot be asked is said, and
    // the item is begun from what is here.
    let (theirs, unasked) = match flag(args, "fetch", false) {
        true => match it.theirs(&remote, text(args, "base")).await? {
            Ok(theirs) => (theirs, String::new()),
            Err(why) => (Theirs::default(), why),
        },
        false => (Theirs::default(), String::new()),
    };
    // A place git was cut off in the middle of making is not yet one.
    // From here it writes in the repository, and has its turn there. The
    // remote was asked before: nothing waits on a remote for another item.
    let _turn = it.turn().await?;
    if let Some(kept) = it.half_made(&place).await? {
        it.retake(&place, &kept).await?;
    }
    // Every round after an item's first finds its place where the last left it.
    let there = it.ask(&project.join(&place), &["rev-parse", "--show-prefix"]).await?;
    let how = if there.ok() {
        "there"
    } else {
        // It is not there (no such directory), or what is there cannot be
        // asked at all, as a file in its way cannot: it is made, and what
        // is in the way is what git says of that.
        let head = format!("refs/heads/{branch}");
        let known = it.ask(project, &["rev-parse", "--verify", "--quiet", &head]).await?;
        let (how, made) = match known.code {
            // A worktree on the branch the item already has.
            Some(0) => ("rejoined", it.ask(project, &["worktree", "add", "--quiet", &place, &branch]).await?),
            // A branch the remote already has for the item is pulled down,
            // and the item is taken up where that branch is.
            Some(1) if theirs.has => {
                let from = format!("{remote}/{branch}");
                (
                    "pulled",
                    it.ask(project, &["worktree", "add", "--quiet", "-b", &branch, &place, &from]).await?,
                )
            }
            // A worktree on a branch of its own, both new. Cut from the
            // project as it is here, or from the remote's branch where
            // that was asked and has everything here and more.
            Some(1) => {
                let mut asked = vec!["worktree", "add", "--quiet", "--no-track", "-b", &branch, &place];
                asked.extend(from.as_deref().or(theirs.from.as_deref()));
                ("fresh", it.ask(project, &asked).await?)
            }
            _ => ("none", known),
        };
        // A place that could not be made is not made up for by the
        // project's own folder: what was left there would lie among the
        // project's files with nothing to say whose it was.
        if !made.ok() {
            return Err(ActionError::new(format!("its place {place} could not be made ({})", made.words())));
        }
        how
    };
    // A place that was there already takes in what the remote has had
    // added to the item's branch since, where that follows on from what is
    // here. Where it does not, both are left as they are.
    if matches!(how, "there" | "rejoined") && theirs.has {
        let from = format!("{remote}/{branch}");
        it.ask(&project.join(&place), &["merge", "--quiet", "--ff-only", &from]).await?;
    }
    came(if how == "none" { "" } else { &place }, how, &unasked)
}

/// What the agent changed is kept as a commit on its branch, if it
/// left any of it uncommitted. The commit is the agent's; whoever git is
/// configured as committed it.
async fn keep(it: &Place, args: &Fields) -> ActionResult {
    let nowhere = || {
        it.note("no worktree of its own; nothing to commit for it");
        Ok(json!({ "placed": false, "committed": false, "at": "", "unkept": "", "said": "", "named": true }))
    };
    let Some(dir) = text(args, "dir") else { return nowhere() };
    let at = it.within(&dir)?;
    let placed = it.ask(&at, &["rev-parse", "--show-prefix"]).await?;
    // 128 is no such directory.
    if placed.is(128) {
        return nowhere();
    }
    let _turn = it.turn().await?;
    if !placed.ok() {
        return Err(ActionError::new(placed.why()));
    }
    if dir == it.place() {
        it.unlocked(&at).await?;
    }
    // Everything the agent left is put forward; whether that is anything
    // is what git answers with: 1 is that there is something to commit.
    it.does(&at, &["add", "-A"]).await?;
    let staged = it.ask(&at, &["diff", "--cached", "--quiet"]).await?;
    let committed = match staged.code {
        Some(0) => {
            it.note("nothing left uncommitted");
            false
        }
        Some(1) => {
            // The item's title and the agent's name are an argument each,
            // so that nothing written on an item is run. The address is
            // nobody's: `.invalid` can never be a real one.
            let by = text(args, "by").unwrap_or_else(|| "agent".to_string());
            let author = format!("--author={by} <{by}@agents.invalid>");
            let message = text(args, "message").unwrap_or_else(|| it.item.clone());
            let made = it.ask(&at, &["commit", "--quiet", &author, "-m", &message]).await?;
            if !made.ok() {
                if !flag(args, "unkept_ok", false) {
                    return Err(ActionError::new(made.why()));
                }
                // Whoever did the work can put right what is in the work's
                // way, and cannot tell git who commits here: which it is,
                // is said, so that nobody is asked for what is not theirs.
                let named = it.ask(&at, &["var", "GIT_COMMITTER_IDENT"]).await?.ok();
                it.note("what was left could not be committed");
                return Ok(json!({
                    "placed": true, "committed": false, "at": "", "unkept": made.why(),
                    "said": made.words().trim(), "named": named,
                }));
            }
            it.note("committed what was left uncommitted");
            true
        }
        _ => return Err(ActionError::new(staged.why())),
    };
    let head = it.does(&at, &["rev-parse", "--short", "HEAD"]).await?;
    it.note(&format!("{} is at {}", it.branch(), head.out.trim()));
    Ok(
        json!({ "placed": true, "committed": committed, "at": head.out.trim(), "unkept": "", "said": "", "named": true }),
    )
}

/// Its branch is sent to the remote: as the key the step was
/// handed, and never as whoever runs the daemon where a key was to be used.
async fn send(it: &Place, args: &Fields) -> ActionResult {
    // What goes wrong here is said in words of our own.
    let nowhere = || Err(ActionError::new("no worktree of its own, so no branch to send"));
    let Some(dir) = text(args, "dir") else { return nowhere() };
    let at = it.within(&dir)?;
    if !it.ask(&at, &["rev-parse", "--show-prefix"]).await?.ok() {
        return nowhere();
    }
    let remote = text(args, "remote").unwrap_or_else(|| "origin".to_string());
    // Whether the repository knows a remote by that name, and where what
    // is sent to it goes.
    let known = it.ask(&at, &["remote", "get-url", "--push", &remote]).await?;
    if !known.ok() {
        return Err(ActionError::new(format!("the repository has no remote called {remote}")));
    }
    let address = known.out.trim();
    let key = it.key();
    if let Some(key) = key
        && !git::takes_a_key(address)
    {
        return Err(ActionError::new(format!(
            "{remote} is not reached over https, and the key {key} is only given to a remote that is. Give it an \
             https address (git remote set-url {remote} https://...), or take `key` away and the branch is sent \
             as whoever runs the daemon"
        )));
    }
    let branch = it.branch();
    let sent = it.ask_remote(&at, key.map(|_| address), &["push", "--quiet", "-u", &remote, &branch]).await?;
    if !sent.ok() {
        return Err(ActionError::new(sent.why()));
    }
    let with = key.map(|key| format!(", sent with the key {key}")).unwrap_or_default();
    it.note(&format!("{branch} is on {remote}{with}"));
    Ok(json!({ "branch": branch, "remote": remote, "key": key.unwrap_or_default() }))
}

/// The item's work is put into the branch the project's folder has checked
/// out, as a commit of its own that says which item: in a last line of its
/// own (`Item: shop-12`), as git writes `Signed-off-by:`.
async fn merge(it: &Place, args: &Fields) -> ActionResult {
    let came = |how: &str, said: &str, why: &str| Ok(json!({ "came": how, "said": said, "why": why }));
    let (project, branch) = (&it.project, it.branch());
    let under_way = ["rev-parse", "--verify", "--quiet", "MERGE_HEAD"];
    // One worked in the project itself has no branch of its own, and its
    // work is there already.
    if !it.ask(project, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")]).await?.ok() {
        return came("nothing", "", "");
    }
    let _turn = it.turn().await?;
    // A merge already under way in the project's folder is somebody
    // else's, and is not this one's to call off. But for one of this
    // item's own branch: nobody else merges that, so it is this merge,
    // cut off before it was committed. It is called off, and made again.
    let theirs = it.ask(project, &under_way).await?;
    if theirs.ok() {
        let own =
            it.ask(project, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")]).await?;
        let cut_off = own.ok() && own.out.trim() == theirs.out.trim();
        if !cut_off || !it.ask(project, &["merge", "--abort"]).await?.ok() {
            let why = "a merge is already under way in the project's folder: finish it, or call it off with \
                       `git merge --abort`";
            return came("stuck", "", why);
        }
        it.note(
            "a merge of this branch was cut off before it was committed; it was called off and is made again",
        );
    }
    // What somebody has put forward in the project's folder and not
    // committed is theirs, and git would refuse to merge over it. It is
    // said in words of our own, and is also what tells, after a merge
    // that failed, that whatever is then put forward is that merge's.
    let staged = ["diff", "--cached", "--quiet"];
    if it.ask(project, &staged).await?.is(1) {
        let why = "changes are put forward in the project's folder and not committed (`git status` shows them): \
                   commit them, or take them back with `git restore --staged .`";
        return came("stuck", "", why);
    }
    let message = text(args, "message").unwrap_or_else(|| branch.clone());
    // Its last line says which item, for whatever follows its work up
    // later (`place.follows`): long after the branch is gone.
    let item = format!("{}: {}", follows::ITEM, it.item);
    let mut waited = git::Waited::default();
    loop {
        let asked = ["merge", "--no-ff", "--no-edit", "-m", &message, "-m", &item, &branch];
        let mut merged = it.ask_once(project, &asked).await?;
        if merged.ok() {
            return came("merged", &merged.out, "");
        }
        // It stopped with the merge under way. Where that is on a conflict
        // (something is left unmerged) the two do not go together, and it
        // is called off for whoever worked the item to put right.
        let conflict = it.ask(project, &under_way).await?.ok()
            && !it.ask(project, &["ls-files", "--unmerged"]).await?.out.trim().is_empty();
        // Whatever stopped it, the project's files and its index are put
        // back as they were. Git writes the merged files and then makes the
        // commit, and may be refused between the two (a lock another git
        // took just then, a commit it cannot sign): left so, the next
        // merge here would be refused for changes nobody made.
        if it.ask(project, &under_way).await?.ok() {
            it.ask(project, &["merge", "--abort"]).await?;
        } else if it.ask(project, &staged).await?.is(1) {
            it.note("the merge that failed had changed the project's files: they are put back as they were");
            it.ask(project, &["reset", "--quiet", "--merge", "HEAD"]).await?;
        }
        if conflict {
            return came("conflict", &merged.out, "");
        }
        // Refused a lock that another git holds, it is made again when
        // that git has had a moment, for as long as such a lock is waited
        // for. Anything else in the way is a person's to put right.
        match git::lock_met(&merged.err) {
            Some(lock) if !waited.spent() => it.waits_for(&lock, &mut waited).await?,
            Some(lock) => {
                merged.err = format!("{}\n{}", merged.err, git::lock_kept(&lock));
                return came("stuck", &merged.out, &merged.words());
            }
            None => return came("stuck", &merged.out, &merged.words()),
        }
    }
}

/// Its worktree is removed, and its branch with it when nothing
/// would be lost.
async fn clear(it: &Place, args: &Fields) -> ActionResult {
    let came = |how: &str, why: &str| Ok(json!({ "came": how, "why": why }));
    let (project, place, branch) = (&it.project, it.place(), it.branch());
    let remote = text(args, "remote").unwrap_or_else(|| "origin".to_string());
    // 128 is no such directory. Where git cannot be asked at all there was
    // nothing of its making to tidy.
    let there = it.ask(&project.join(&place), &["rev-parse", "--show-prefix"]).await?;
    if !there.ok() && !there.is(128) {
        return came("nothing", "");
    }
    // Said to be thrown away, what it holds goes, its branch with it.
    let discard = flag(args, "discard", false);
    let _turn = it.turn().await?;
    if there.ok() {
        // Not forced: a worktree holding changes that were never committed
        // is the only place they are, unless they are to be thrown away.
        let mut remove = vec!["worktree", "remove", &place];
        if discard {
            remove.push("--force");
        }
        let removed = it.ask(project, &remove).await?;
        if !removed.ok() {
            return came("place_kept", &removed.words());
        }
    }
    let cleared = if there.ok() { "cleared" } else { "nothing" };
    // 1 is no such branch, 128 no repository: there is nothing to remove.
    let known = |said: &Said| said.is(0) || said.is(1) || said.is(128);
    let head =
        it.ask(project, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")]).await?;
    if !known(&head) {
        return came("untidied", &head.why());
    }
    if !head.ok() {
        return came(cleared, "");
    }
    if discard {
        let dropped = it.ask(project, &["branch", "--quiet", "-D", &branch]).await?;
        return if dropped.ok() { came("cleared", "") } else { came("untidied", &dropped.why()) };
    }
    // Whether everything on the branch is in the project's own branch; or
    // is on the remote, as far as this repository knows. Nothing is asked
    // of the remote, and nothing is done to it: the branch there is the
    // hosting service's business.
    let merged = it.ask(project, &["merge-base", "--is-ancestor", &branch, "HEAD"]).await?;
    if !known(&merged) {
        return came("untidied", &merged.why());
    }
    if !merged.ok() {
        let theirs = format!("refs/remotes/{remote}/{branch}");
        let sent = it.ask(project, &["merge-base", "--is-ancestor", &branch, &theirs]).await?;
        if !known(&sent) {
            return came("untidied", &sent.why());
        }
        if !sent.ok() {
            return came("branch_kept", "");
        }
    }
    // The branch is removed, its work being somewhere else.
    let dropped = it.ask(project, &["branch", "--quiet", "-D", &branch]).await?;
    if !dropped.ok() {
        return came("untidied", &dropped.why());
    }
    came("cleared", "")
}

/// Where the item was worked and what was changed there, as git has it
/// against the project's own branch, for those who look at it to be shown.
/// One held to reading may not be able to ask git itself.
async fn changed(it: &Place, args: &Fields) -> ActionResult {
    let (project, place) = (&it.project, it.place());
    // No such directory, or no repository: it was worked in the project
    // itself; or, in a repository with history to branch from, it has no
    // place yet or no longer, and that is said.
    if !it.ask(&project.join(&place), &["rev-parse", "--show-prefix"]).await?.ok() {
        let could = it.ask(project, &["rev-parse", "--verify", "--quiet", "HEAD"]).await?.ok();
        return Ok(json!({ "dir": "", "changed": "", "there": !could, "patch": "", "cut": false }));
    }
    // Against another item's branch where one is said and it has one: what
    // one of several pieces of work on an item did beside what the item had.
    let base = match named(args, "against")? {
        Some(other) => it.branch_of(&other).await?.unwrap_or_else(|| "HEAD".to_string()),
        None => "HEAD".to_string(),
    };
    let against = format!("{base}...{}", it.branch());
    let stat = it.ask(project, &["diff", "--stat", &against]).await?;
    let most = args.get("patch").and_then(Value::as_u64).unwrap_or(0).min(PATCH as u64) as usize;
    let (patch, cut) = match most {
        0 => (String::new(), false),
        most => {
            let read = it.read(project, &["diff", &against]).await?;
            if read.ok() { bounded(read.out, most) } else { (String::new(), false) }
        }
    };
    let changed = if stat.ok() { stat.out } else { String::new() };
    Ok(json!({ "dir": place, "changed": changed, "there": true, "patch": patch, "cut": cut }))
}

/// The most of a patch that is handed on, whatever is asked: what somebody
/// is shown to read, and not a copy of the work, which is the branch's.
const PATCH: usize = 1024 * 1024;

/// `text` up to `most` bytes, cut at the end of a line, and whether it was cut.
fn bounded(text: String, most: usize) -> (String, bool) {
    if text.len() <= most {
        return (text, false);
    }
    let mut end = most;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let end = text[..end].rfind('\n').map_or(end, |at| at + 1);
    (text[..end].to_string(), true)
}

/// An argument that names an item, where it is said: one name of a folder
/// and of a branch, never a way out of the project and never something
/// git reads as an option.
fn named(args: &Fields, key: &str) -> Result<Option<String>, ActionError> {
    let Some(item) = text(args, key) else { return Ok(None) };
    if !is_a_name(&item) {
        let what = match key {
            "item" => "`item` is what the work is known by".to_string(),
            other => format!("`{other}` names another piece of work by what it is known by"),
        };
        return Err(ActionError::new(format!(
            "{what}, such as a card's id, `shop-1`: its folder and its branch are called after it, and `{item}` \
             cannot be the name of either"
        )));
    }
    Ok(Some(item))
}

/// Whether `item` can name a piece of work: one name of a folder and of a
/// branch, never a way out of the project and never something git reads
/// as an option.
fn is_a_name(item: &str) -> bool {
    let one = matches!(Path::new(item).components().collect::<Vec<_>>().as_slice(), [Component::Normal(_)]);
    one && !item.starts_with('-')
}

/// The commit a folder of the project is at, where it is in a repository
/// with history: nothing where it is in none, or nothing is committed.
async fn head(it: &Place, args: &Fields) -> ActionResult {
    let at = it.within(&text(args, "dir").unwrap_or_default())?;
    let head = it.ask(&at, &["rev-parse", "--verify", "--quiet", "HEAD"]).await?;
    Ok(json!({ "commit": if head.ok() { head.out.trim() } else { "" } }))
}

fn text(args: &Fields, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::to_string).filter(|text| !text.is_empty())
}

/// Whether something was said to be so; a param handed on as text counts.
fn flag(args: &Fields, key: &str, otherwise: bool) -> bool {
    match args.get(key) {
        Some(Value::Bool(said)) => *said,
        Some(Value::String(said)) if said == "true" => true,
        Some(Value::String(said)) if said == "false" => false,
        _ => otherwise,
    }
}
