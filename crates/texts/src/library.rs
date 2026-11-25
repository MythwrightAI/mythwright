//! Where blueprints come from.
//!
//! A blueprint is looked up by name, like `fleet` or `std/slot`. Lookups go
//! through layers, nearest first: a project's own blueprints shadow the shared
//! ones, which shadow the standard library that ships with the product. That
//! is what lets a user change how a canary rollout works by editing a text
//! file in their project, with no rebuild and without touching anyone else's.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub mod folders;

pub trait Library: Send + Sync + 'static {
    /// The text of the blueprint called `name`, as seen from `project`.
    fn load(&self, name: &str, project: Option<&str>) -> Option<String>;

    /// Every name this library can load for `project`.
    fn names(&self, project: Option<&str>) -> Vec<String>;

    /// Where `project` has a text of its own for `name` that is not the
    /// one `load` gives: a sentence saying which file, why it is passed
    /// over, and what would have it used. Nothing, for a library that
    /// passes nothing over.
    fn passed_over(&self, _name: &str, _project: Option<&str>) -> Option<String> {
        None
    }

    /// Whether the text `load` gives for `name` is a file of the project's
    /// own: one that whoever may change the project's files can change,
    /// which is not whoever applied what calls it ([`crate::vouched`]).
    fn is_own(&self, _name: &str, _project: Option<&str>) -> bool {
        false
    }

    /// Every text of the project's own that `load` would give, by name.
    fn own(&self, _project: Option<&str>) -> Vec<(String, String)> {
        Vec::new()
    }

    /// The text there would be for `name` if the project had none of its
    /// own: the one that ships, or nothing.
    fn shipped(&self, name: &str, project: Option<&str>) -> Option<String> {
        self.load(name, project)
    }

    /// Whether the text `load` gives for `name` came with the program or
    /// with a package: one that is another when the program is, without
    /// anybody having written it. A running tree goes on with such a text
    /// as it reached it ([`crate::pinned`]). A file somebody can write
    /// where the factory runs (a project's own, a checkout's) is not one.
    fn ships(&self, _name: &str, _project: Option<&str>) -> bool {
        false
    }

    /// What the file `load` reads for `name` is called, where the text is
    /// a file and the library knows which: for saying where in it
    /// something is wrong. A blueprint's file has more than one name it
    /// may go by, and whoever is told of line 3 looks in the one they have.
    fn called(&self, _name: &str, _project: Option<&str>) -> Option<String> {
        None
    }

    /// Why `load` gives nothing for a name there is a file for: a sentence
    /// for whoever asked for it by that name. A blueprint's file may be
    /// called more than one way ([`named`]), and a folder that holds two
    /// of them for one name has said which is meant with neither. Nothing,
    /// for a library that refuses nothing.
    fn refused(&self, _name: &str, _project: Option<&str>) -> Option<String> {
        None
    }

    /// Where `project` keeps blueprints of its own ([`folders`]), for a
    /// library that reads them; nothing for one that knows no project's.
    fn folder(&self, _project: Option<&str>) -> Option<folders::Folder> {
        None
    }
}

/// Where `project` keeps blueprints of its own, as the nearest library
/// that reads them says: the usual where none does, or no project is
/// asked of.
pub fn folder_of(library: &dyn Library, project: Option<&str>) -> folders::Folder {
    project.and_then(|_| library.folder(project)).unwrap_or_default()
}

/// What there is under a folder for a blueprint's name.
#[derive(Debug, PartialEq)]
pub enum Named {
    /// No file of any of the names it may have.
    Nothing,
    /// The one file, as it is called from the folder.
    File(String),
    /// More than one, each as it is called from the folder.
    Several(Vec<String>),
}

/// The file under `root` that is the blueprint called `name`, whichever
/// of the names a blueprint's file may have it goes by, where its usual
/// ending is `ending` (`fleet.bp`, and `fleet.bp.yml` for an editor's
/// sake).
pub fn named(root: &Path, name: &str, ending: &str) -> Named {
    let mut there: Vec<String> = brand::endings_of(ending)
        .iter()
        .map(|ending| format!("{name}.{ending}"))
        .filter(|file| root.join(file).is_file())
        .collect();
    match there.len() {
        0 => Named::Nothing,
        1 => Named::File(there.remove(0)),
        _ => Named::Several(there),
    }
}

/// What is said of a blueprint's name that has more than one file, `files`
/// as they are called where whoever reads this will look for them.
pub fn several(name: &str, files: &[String]) -> String {
    let files: Vec<String> = files.iter().map(|file| format!("`{file}`")).collect();
    format!(
        "there is more than one file for the blueprint `{name}` ({}), and a blueprint is one file: which of \
         them is meant cannot be told, so neither is read. Take all but one away",
        files.join(" and ")
    )
}

/// Whether `name` is a file a text links (a prompt, in `prompts/` beside
/// the blueprints of its place) and no blueprint. Such a one is a text of
/// the library like any other, found by its name as it is, vouched for and
/// stood in for by the same rules; it is not listed among the blueprints,
/// since it is nothing anybody applies.
pub fn is_linked(name: &str) -> bool {
    blueprint::parts::is_linked(name)
}

/// Whether two texts kept under `name` are one to whatever runs them. A
/// blueprint laid out another way, or with other comments, is the
/// blueprint it was. Words a blueprint links are what an agent is told,
/// every one of them: they are the same only where they are the same
/// words.
pub fn same(name: &str, was: &str, now: &str) -> bool {
    match is_linked(name) {
        true => was == now,
        false => blueprint::revision(was) == blueprint::revision(now) || blueprint::means_the_same(was, now),
    }
}

/// Whether `name` is safe to turn into a path below a root.
pub fn is_safe_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('/')
        && name.split('/').all(|part| !part.is_empty() && part != "." && part != "..")
        && !name.contains('\\')
}

/// Blueprints held in memory. Used for the built-in standard library and in
/// tests.
#[derive(Default)]
pub struct MemoryLibrary {
    entries: Mutex<BTreeMap<String, String>>,
    /// Whether what it holds came with the program ([`Library::ships`]).
    ships: std::sync::atomic::AtomicBool,
}

impl MemoryLibrary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(entries: &[(&str, &str)]) -> Self {
        let library = Self::new();
        for (name, source) in entries {
            library.put(name, source);
        }
        library
    }

    pub fn put(&self, name: &str, source: &str) {
        self.entries.lock().expect("library lock").insert(name.to_string(), source.to_string());
    }

    /// Says that what this holds came with the program.
    pub fn ship(&self) {
        self.ships.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

impl Library for MemoryLibrary {
    fn load(&self, name: &str, _project: Option<&str>) -> Option<String> {
        self.entries.lock().expect("library lock").get(name).cloned()
    }

    fn names(&self, _project: Option<&str>) -> Vec<String> {
        self.entries.lock().expect("library lock").keys().filter(|name| !is_linked(name)).cloned().collect()
    }

    fn ships(&self, name: &str, _project: Option<&str>) -> bool {
        self.ships.load(std::sync::atomic::Ordering::SeqCst)
            && self.entries.lock().expect("library lock").contains_key(name)
    }
}

/// Blueprints as files under a directory: `std/slot` is `<root>/std/slot.<ext>`.
pub struct DirLibrary {
    root: PathBuf,
    extension: String,
}

impl DirLibrary {
    pub fn new(root: impl Into<PathBuf>, extension: &str) -> Self {
        Self { root: root.into(), extension: extension.to_string() }
    }

    fn collect(&self, dir: &Path, prefix: &str, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else { continue };
            if name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                self.collect(&path, &format!("{prefix}{name}/"), out);
            } else if let Some(stem) = brand::endings_of(&self.extension)
                .iter()
                .rev()
                .find_map(|ending| brand::without_ending(name, ending))
            {
                out.push(format!("{prefix}{stem}"));
            }
        }
    }
}

impl Library for DirLibrary {
    fn load(&self, name: &str, _project: Option<&str>) -> Option<String> {
        if !is_safe_name(name) {
            return None;
        }
        // A file a text links is called what its file is.
        if is_linked(name) {
            return std::fs::read_to_string(self.root.join(name)).ok();
        }
        match named(&self.root, name, &self.extension) {
            Named::File(file) => std::fs::read_to_string(self.root.join(file)).ok(),
            Named::Nothing | Named::Several(_) => None,
        }
    }

    fn names(&self, _project: Option<&str>) -> Vec<String> {
        let mut names = Vec::new();
        self.collect(&self.root, "", &mut names);
        names.sort();
        // One that has two files is one name, and is refused when it is asked for.
        names.dedup();
        names
    }

    fn refused(&self, name: &str, _project: Option<&str>) -> Option<String> {
        match named(&self.root, name, &self.extension) {
            Named::Several(files) if is_safe_name(name) => Some(several(name, &files)),
            _ => None,
        }
    }

    fn called(&self, name: &str, _project: Option<&str>) -> Option<String> {
        match named(&self.root, name, &self.extension) {
            Named::File(file) if is_safe_name(name) => Some(file),
            _ => None,
        }
    }
}

/// Several libraries, searched in order.
#[derive(Default, Clone)]
pub struct Layers {
    layers: Vec<Arc<dyn Library>>,
}

impl Layers {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a layer that is searched after the ones already added.
    pub fn then(mut self, library: Arc<dyn Library>) -> Self {
        self.layers.push(library);
        self
    }
}

impl Library for Layers {
    fn load(&self, name: &str, project: Option<&str>) -> Option<String> {
        // A layer that refuses the name is where it would have been found:
        // what is behind it does not stand in for a file nobody can read
        // the meaning of.
        for layer in &self.layers {
            if layer.refused(name, project).is_some() {
                return None;
            }
            if let Some(text) = layer.load(name, project) {
                return Some(text);
            }
        }
        None
    }

    fn refused(&self, name: &str, project: Option<&str>) -> Option<String> {
        for layer in &self.layers {
            if let Some(why) = layer.refused(name, project) {
                return Some(why);
            }
            if layer.load(name, project).is_some() {
                return None;
            }
        }
        None
    }

    fn called(&self, name: &str, project: Option<&str>) -> Option<String> {
        // The layer that gives the text is the one that says what its file is called.
        let gives = self.layers.iter().find(|layer| layer.load(name, project).is_some());
        gives.and_then(|layer| layer.called(name, project))
    }

    fn names(&self, project: Option<&str>) -> Vec<String> {
        let mut names: Vec<String> = self.layers.iter().flat_map(|layer| layer.names(project)).collect();
        names.sort();
        names.dedup();
        names
    }

    fn passed_over(&self, name: &str, project: Option<&str>) -> Option<String> {
        self.layers.iter().find_map(|layer| layer.passed_over(name, project))
    }

    fn is_own(&self, name: &str, project: Option<&str>) -> bool {
        // The layer that gives the text is the one that says whose it is.
        let gives = self.layers.iter().find(|layer| layer.load(name, project).is_some());
        gives.is_some_and(|layer| layer.is_own(name, project))
    }

    fn own(&self, project: Option<&str>) -> Vec<(String, String)> {
        self.layers.iter().flat_map(|layer| layer.own(project)).collect()
    }

    fn shipped(&self, name: &str, project: Option<&str>) -> Option<String> {
        self.layers.iter().find_map(|layer| layer.shipped(name, project))
    }

    fn ships(&self, name: &str, project: Option<&str>) -> bool {
        let gives = self.layers.iter().find(|layer| layer.load(name, project).is_some());
        gives.is_some_and(|layer| layer.ships(name, project))
    }

    fn folder(&self, project: Option<&str>) -> Option<folders::Folder> {
        self.layers.iter().find_map(|layer| layer.folder(project))
    }
}

/// The blueprints that ship with the product, under `std/`. They are plain
/// files in `blueprints/std`, compiled in so a single binary is complete; a
/// project (or a development checkout) can shadow any of them by putting a
/// file of the same name in a layer searched earlier.
pub fn standard() -> MemoryLibrary {
    let library = MemoryLibrary::with(&[
        ("std/slot", include_str!("../../../blueprints/std/slot.bp")),
        ("std/canary-stage", include_str!("../../../blueprints/std/canary-stage.bp")),
        ("std/cards", include_str!("../../../blueprints/std/cards.bp")),
        ("std/lane-round", include_str!("../../../blueprints/std/lane-round.bp")),
        ("std/pass-by-hand", include_str!("../../../blueprints/std/pass-by-hand.bp")),
        ("std/pipeline", include_str!("../../../blueprints/std/pipeline.bp")),
        ("std/pipeline-at", include_str!("../../../blueprints/std/pipeline-at.bp")),
        ("std/pipeline-timed", include_str!("../../../blueprints/std/pipeline-timed.bp")),
        ("std/pool", include_str!("../../../blueprints/std/pool.bp")),
        ("std/topics", include_str!("../../../blueprints/std/topics.bp")),
        ("std/owed", include_str!("../../../blueprints/std/owed.bp")),
        ("std/role", include_str!("../../../blueprints/std/role.bp")),
        ("std/agent", include_str!("../../../blueprints/std/agent.bp")),
        ("std/review", include_str!("../../../blueprints/std/review.bp")),
        ("std/run", include_str!("../../../blueprints/std/run.bp")),
        ("std/work", include_str!("../../../blueprints/std/work.bp")),
        ("std/turn", include_str!("../../../blueprints/std/turn.bp")),
        ("std/keep", include_str!("../../../blueprints/std/keep.bp")),
        ("std/send", include_str!("../../../blueprints/std/send.bp")),
        ("std/merge", include_str!("../../../blueprints/std/merge.bp")),
        ("std/cleared", include_str!("../../../blueprints/std/cleared.bp")),
        ("std/onward", include_str!("../../../blueprints/std/onward.bp")),
        ("std/due", include_str!("../../../blueprints/std/due.bp")),
        ("std/segments", include_str!("../../../blueprints/std/segments.bp")),
        ("std/through", include_str!("../../../blueprints/std/through.bp")),
        ("std/fleet", include_str!("../../../blueprints/std/fleet.bp")),
        ("std/told", include_str!("../../../blueprints/std/told.bp")),
        ("std/discuss", include_str!("../../../blueprints/std/discuss.bp")),
        ("std/discussion", include_str!("../../../blueprints/std/discussion.bp")),
        ("std/discussion-turn", include_str!("../../../blueprints/std/discussion-turn.bp")),
        ("std/chain", include_str!("../../../blueprints/std/chain.bp")),
        ("std/chain-wait", include_str!("../../../blueprints/std/chain-wait.bp")),
        ("std/later", include_str!("../../../blueprints/std/later.bp")),
        ("std/arrangement", include_str!("../../../blueprints/std/arrangement.bp")),
        ("std/working", include_str!("../../../blueprints/std/working.bp")),
        ("std/reviewing", include_str!("../../../blueprints/std/reviewing.bp")),
        ("std/planning", include_str!("../../../blueprints/std/planning.bp")),
        ("std/looked", include_str!("../../../blueprints/std/looked.bp")),
        ("std/resumed", include_str!("../../../blueprints/std/resumed.bp")),
        ("std/dispatch", include_str!("../../../blueprints/std/dispatch.bp")),
        ("std/features", include_str!("../../../blueprints/std/features.bp")),
        ("std/feature", include_str!("../../../blueprints/std/feature.bp")),
        ("std/feature-round", include_str!("../../../blueprints/std/feature-round.bp")),
        ("std/plan", include_str!("../../../blueprints/std/plan.bp")),
        ("std/proposal", include_str!("../../../blueprints/std/proposal.bp")),
        ("std/proposal-round", include_str!("../../../blueprints/std/proposal-round.bp")),
        ("std/log-feed", include_str!("../../../blueprints/std/log-feed.bp")),
        ("std/board", include_str!("../../../blueprints/std/board.bp")),
        ("std/docs", include_str!("../../../blueprints/std/docs.bp")),
        ("std/environments", include_str!("../../../blueprints/std/environments.bp")),
        ("std/flags", include_str!("../../../blueprints/std/flags.bp")),
        ("std/flags-due", include_str!("../../../blueprints/std/flags-due.bp")),
        ("std/project", include_str!("../../../blueprints/std/project.bp")),
        ("std/mockups", include_str!("../../../blueprints/std/mockups.bp")),
        ("std/table", include_str!("../../../blueprints/std/table.bp")),
        ("std/site", include_str!("../../../blueprints/std/site.bp")),
        ("std/store", include_str!("../../../blueprints/std/store.bp")),
        ("std/bot", include_str!("../../../blueprints/std/bot.bp")),
        ("std/rolling", include_str!("../../../blueprints/std/rolling.bp")),
        ("std/rolling-step", include_str!("../../../blueprints/std/rolling-step.bp")),
        ("std/snapshot", include_str!("../../../blueprints/std/snapshot.bp")),
        ("std/profile", include_str!("../../../blueprints/std/profile.bp")),
        ("std/signin", include_str!("../../../blueprints/std/signin.bp")),
        ("std/watch", include_str!("../../../blueprints/std/watch.bp")),
        ("std/timed", include_str!("../../../blueprints/std/timed.bp")),
        ("std/timed-card", include_str!("../../../blueprints/std/timed-card.bp")),
        ("std/timed-table", include_str!("../../../blueprints/std/timed-table.bp")),
        ("std/ab", include_str!("../../../blueprints/std/ab.bp")),
        ("std/contest", include_str!("../../../blueprints/std/contest.bp")),
        ("std/contended", include_str!("../../../blueprints/std/contended.bp")),
        ("std/entry", include_str!("../../../blueprints/std/entry.bp")),
        ("std/judge", include_str!("../../../blueprints/std/judge.bp")),
        ("std/judge-scores", include_str!("../../../blueprints/std/judge-scores.bp")),
        ("std/judge-seat", include_str!("../../../blueprints/std/judge-seat.bp")),
        ("std/judge-talk", include_str!("../../../blueprints/std/judge-talk.bp")),
        // What the texts above link: what an agent is told, as files of
        // `prompts/` beside them.
        ("std/prompts/feature-round.md", include_str!("../../../blueprints/std/prompts/feature-round.md")),
        (
            "std/prompts/feature-round/back.md",
            include_str!("../../../blueprints/std/prompts/feature-round/back.md"),
        ),
        (
            "std/prompts/feature-round/changed.md",
            include_str!("../../../blueprints/std/prompts/feature-round/changed.md"),
        ),
        (
            "std/prompts/feature-round/new.md",
            include_str!("../../../blueprints/std/prompts/feature-round/new.md"),
        ),
        (
            "std/prompts/feature-round/gathers.md",
            include_str!("../../../blueprints/std/prompts/feature-round/gathers.md"),
        ),
        (
            "std/prompts/feature-round/planned.md",
            include_str!("../../../blueprints/std/prompts/feature-round/planned.md"),
        ),
        (
            "std/prompts/feature-round/said.md",
            include_str!("../../../blueprints/std/prompts/feature-round/said.md"),
        ),
        ("std/prompts/judge.md", include_str!("../../../blueprints/std/prompts/judge.md")),
        ("std/prompts/judge/pick.md", include_str!("../../../blueprints/std/prompts/judge/pick.md")),
        ("std/prompts/judge/scores.md", include_str!("../../../blueprints/std/prompts/judge/scores.md")),
        ("std/prompts/judge/talk.md", include_str!("../../../blueprints/std/prompts/judge/talk.md")),
        ("std/prompts/judge/talked.md", include_str!("../../../blueprints/std/prompts/judge/talked.md")),
        ("std/prompts/lane-round.md", include_str!("../../../blueprints/std/prompts/lane-round.md")),
        (
            "std/prompts/lane-round/answered.md",
            include_str!("../../../blueprints/std/prompts/lane-round/answered.md"),
        ),
        (
            "std/prompts/lane-round/back-said.md",
            include_str!("../../../blueprints/std/prompts/lane-round/back-said.md"),
        ),
        (
            "std/prompts/lane-round/discussed.md",
            include_str!("../../../blueprints/std/prompts/lane-round/discussed.md"),
        ),
        (
            "std/prompts/lane-round/back.md",
            include_str!("../../../blueprints/std/prompts/lane-round/back.md"),
        ),
        (
            "std/prompts/lane-round/blocked.md",
            include_str!("../../../blueprints/std/prompts/lane-round/blocked.md"),
        ),
        ("std/prompts/chain.md", include_str!("../../../blueprints/std/prompts/chain.md")),
        ("std/prompts/chain/answer.md", include_str!("../../../blueprints/std/prompts/chain/answer.md")),
        (
            "std/prompts/lane-round/resumed.md",
            include_str!("../../../blueprints/std/prompts/lane-round/resumed.md"),
        ),
        (
            "std/prompts/lane-round/resumed-said.md",
            include_str!("../../../blueprints/std/prompts/lane-round/resumed-said.md"),
        ),
        (
            "std/prompts/lane-round/in-project.md",
            include_str!("../../../blueprints/std/prompts/lane-round/in-project.md"),
        ),
        (
            "std/prompts/lane-round/in-worktree.md",
            include_str!("../../../blueprints/std/prompts/lane-round/in-worktree.md"),
        ),
        (
            "std/prompts/lane-round/new-said.md",
            include_str!("../../../blueprints/std/prompts/lane-round/new-said.md"),
        ),
        ("std/prompts/lane-round/new.md", include_str!("../../../blueprints/std/prompts/lane-round/new.md")),
        (
            "std/prompts/lane-round/planned.md",
            include_str!("../../../blueprints/std/prompts/lane-round/planned.md"),
        ),
        (
            "std/prompts/lane-round/posted.md",
            include_str!("../../../blueprints/std/prompts/lane-round/posted.md"),
        ),
        (
            "std/prompts/lane-round/unkept.md",
            include_str!("../../../blueprints/std/prompts/lane-round/unkept.md"),
        ),
        ("std/prompts/proposal-round.md", include_str!("../../../blueprints/std/prompts/proposal-round.md")),
        (
            "std/prompts/proposal-round/back-said.md",
            include_str!("../../../blueprints/std/prompts/proposal-round/back-said.md"),
        ),
        (
            "std/prompts/proposal-round/back.md",
            include_str!("../../../blueprints/std/prompts/proposal-round/back.md"),
        ),
        (
            "std/prompts/proposal-round/changed.md",
            include_str!("../../../blueprints/std/prompts/proposal-round/changed.md"),
        ),
        (
            "std/prompts/proposal-round/new.md",
            include_str!("../../../blueprints/std/prompts/proposal-round/new.md"),
        ),
        (
            "std/prompts/proposal-round/planned.md",
            include_str!("../../../blueprints/std/prompts/proposal-round/planned.md"),
        ),
        (
            "std/prompts/proposal-round/said.md",
            include_str!("../../../blueprints/std/prompts/proposal-round/said.md"),
        ),
        ("std/prompts/review-seat.md", include_str!("../../../blueprints/std/prompts/review-seat.md")),
        (
            "std/prompts/review-seat/among.md",
            include_str!("../../../blueprints/std/prompts/review-seat/among.md"),
        ),
        (
            "std/prompts/review-seat/changed.md",
            include_str!("../../../blueprints/std/prompts/review-seat/changed.md"),
        ),
        (
            "std/prompts/review-seat/in-project.md",
            include_str!("../../../blueprints/std/prompts/review-seat/in-project.md"),
        ),
        (
            "std/prompts/review-seat/in-worktree.md",
            include_str!("../../../blueprints/std/prompts/review-seat/in-worktree.md"),
        ),
        ("std/prompts/review/asks.md", include_str!("../../../blueprints/std/prompts/review/asks.md")),
        ("std/prompts/review/planned.md", include_str!("../../../blueprints/std/prompts/review/planned.md")),
        ("std/prompts/turn.md", include_str!("../../../blueprints/std/prompts/turn.md")),
        ("std/prompts/turn/looked.md", include_str!("../../../blueprints/std/prompts/turn/looked.md")),
        ("std/prompts/turn/nearly.md", include_str!("../../../blueprints/std/prompts/turn/nearly.md")),
        ("std/prompts/turn/said.md", include_str!("../../../blueprints/std/prompts/turn/said.md")),
        ("std/prompts/turn/told.md", include_str!("../../../blueprints/std/prompts/turn/told.md")),
    ]);
    library.ship();
    library
}
