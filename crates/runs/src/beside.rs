//! What the runner keeps beside its rows, and whose each thing is.
//!
//! Not everything a conductor has to find again is a row. Some of it is
//! a small value under a name in the journal's small durable state
//! (`Journal::kv_get`): which text of each blueprint that ships a
//! running tree reached, what was vouched for it of its project's own
//! files, what a `poll:` last read. A project has a journal of its own
//! and a conductor that runs what was applied for it, and its rows are
//! in the project's file ([`crate::apart`]). What is kept beside them
//! has to be there too: or a project that is copied, moved or taken
//! away leaves a part of itself in the factory's file, and whatever a
//! busy project keeps is another write on the one thread every project
//! shares.
//!
//! So each name says whose what is kept under it is ([`KEPT`]), and a
//! conductor keeps what is an instance's, and what is its project's, in
//! the journal its rows are in. What is the factory's own, or is one
//! thing for every project, stays in the factory's journal, each with
//! the reason. A test reads the names every file holds after a play and
//! fails for one that is not on the list, or is in a file its line does
//! not allow.
//!
//! A journal a program from before this made may hold instances whose
//! pins and vouches that program kept in the factory's journal. The step
//! of the tables that came with this notes it ([`OWED`]), in the journal
//! itself, and the first conductor to start on that journal brings what
//! is its own over once and takes the note off.

/// Whose what is kept under a name is, and so which journal keeps it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Whose {
    /// An instance's: the key is the instance's id, or begins with it
    /// and a `/` or a line break. Kept in the journal that keeps the
    /// instance's rows.
    Instance,
    /// A project's: the key begins with the project and a line break
    /// (nothing before the break for what is nobody's). Kept in the
    /// journal of the conductor that runs what is applied for it.
    Project,
    /// The conductor's own, with no key that says whose: kept once in
    /// each journal a conductor keeps its rows in, for what is there.
    Conductor,
    /// The factory's: one for every project, in the factory's journal.
    Factory,
}

/// Which text of each name that ships an instance reached, by instance.
pub const PINNED: &str = "pinned";
/// The texts themselves, each once, by revision.
pub const TEXTS: &str = "pinned.texts";
/// What was vouched for an instance of its project's own files.
pub const VOUCHED: &str = "vouched";
/// What a `poll:` handler last read, by its instance and its name.
pub const POLL: &str = "poll";
/// When an `at:` handler last fired, with its time as written, by its
/// instance and its name.
pub const AT: &str = "at";
/// The run a step's call began, by its parent instance and what the
/// event that set the step's run off named its asking (a press of Run:
/// `kernel::asking`), each `<parent>` and a line break and the name.
pub const ASKED: &str = "asked";
/// The last text of each blueprint that checked, as a project saw it.
pub const FITS: &str = "fits";
/// What a project's fork of a blueprint that ships was forked from, by
/// the project and the blueprint's name.
pub const FORKED: &str = "forked";
/// A project's environments, each with its variables, by the project
/// and the environment's name.
pub const PROJECT_ENVIRONMENTS: &str = "project.environments";
/// How a running tree comes by new texts.
pub const TAKING: &str = "taking";
/// The variables of the profile, the copy of the factory: kept under the
/// name they had before 0.1.0, when a profile was called an environment.
pub const ENVIRONMENT: &str = "environment";

/// Where a journal says that what is its own may still be kept in the
/// factory's, by a program from before: written by the step of the
/// tables that came with this, where the journal held any instance, and
/// taken off by the first conductor to start on it.
pub const OWED: (&str, &str) = ("beside", "adopt");

/// Every name the runner keeps something under, with whose it is and
/// why. A name added to what the runner keeps is a line here.
pub const KEPT: &[(&str, Whose, &str)] = &[
    (PINNED, Whose::Instance, "what a tree reached is the tree's, and goes where its instances go"),
    (
        TEXTS,
        Whose::Conductor,
        "a text is kept for the trees that are held to it, with them: by its revision, once in each journal",
    ),
    (
        VOUCHED,
        Whose::Instance,
        "what was vouched for an instance is authority it runs with, and is kept with it",
    ),
    (POLL, Whose::Instance, "what a handler last read is its instance's, and is gone with the instance"),
    (AT, Whose::Instance, "when a handler last fired is its instance's, and is gone with the instance"),
    (
        ASKED,
        Whose::Instance,
        "the run an asking began is found by the instance whose step asked for it, and is gone with the instance",
    ),
    (FITS, Whose::Project, "the last text that checked is what a project's running trees go on with"),
    (
        FORKED,
        Whose::Project,
        "what a fork was made from is said of a file of the project's, and goes where the project goes",
    ),
    (
        PROJECT_ENVIRONMENTS,
        Whose::Project,
        "what a project's environments say is read by every instance of the project, and goes where the project goes",
    ),
    (OWED.0, Whose::Conductor, "a note of the journal's own, of what it may still have in another"),
    (
        TAKING,
        Whose::Factory,
        "how texts are come by is one setting of the factory, which every conductor is handed as it is made",
    ),
    (
        ENVIRONMENT,
        Whose::Factory,
        "where a factory's services answer is said of the factory, and read by every instance in it",
    ),
];

/// Whose what the runner keeps under `name` is; nothing for a name that
/// is not the runner's.
pub fn whose(name: &str) -> Option<Whose> {
    KEPT.iter().find(|(kept, ..)| *kept == name).map(|(_, whose, _)| *whose)
}

/// The instance a key of something kept by instance is of.
pub fn instance_of(key: &str) -> &str {
    key.split_once(['/', '\n']).map_or(key, |(instance, _)| instance)
}

/// The project a key of something kept by project is of: nothing for
/// what is nobody's.
pub fn project_of(key: &str) -> Option<&str> {
    key.split_once('\n').map(|(project, _)| project).filter(|project| !project.is_empty())
}
