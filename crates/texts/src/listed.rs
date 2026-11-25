//! Which of the blueprints there are runs which, for whoever lists them.
//!
//! Most blueprints that ship are never applied by a person: they are
//! parts, run by something a person applies. A list that shows every
//! one alike is forty names long with a dozen to choose from in it. So
//! what is here says, of the blueprints the factory has that are no
//! project's (what ships, what the packages bring, a checkout's own),
//! what each names ([`blueprint::names`]: worked out from the texts and
//! from what each action says of itself, never written down), from which
//! follows what somebody applies and what is a part of what.
//!
//! One thing no text says: that a blueprint which another runs is
//! applied by people too. Those are the few lines of [`APPLIED`], each
//! with why. Everything else a person applies is what nothing names.
//!
//! Nothing is kept: it is a function of the texts as the library gives
//! them when it is asked, so a package added or a text changed is in the
//! next answer. Nothing is renamed or moved by any of it; a name is what
//! everything that runs finds a blueprint by.

use blueprint::ActionSpec;
use blueprint::names::{self, Runs};

use crate::library::Library;

/// The blueprints that ship which somebody applies, or starts from a
/// text of their own, though another that ships runs them too: each by
/// its name, with why it is listed among what is applied. One that
/// nothing names needs no line here, and a test fails for such a line.
pub const APPLIED: &[(&str, &str)] = &[
    (
        "std/agent",
        "applied for one agent added by hand, keyed by its name; what brings up a role's agents calls it for \
         each of them",
    ),
    (
        "std/plan",
        "applied by hand for one document, whose tickets it then adds; what runs it besides does the same for \
         every document of a folder",
    ),
    (
        "std/pool",
        "started from a text of somebody's own, for whatever is too heavy to do all at once: it says how many \
         go at a time",
    ),
    (
        "std/slot",
        "what hosts a service behind an address that stays, called from a text of somebody's own; the two that \
         run it besides each host one service of their own with it",
    ),
];

/// What each blueprint that is no project's runs, and which of them
/// somebody applies though something runs it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Listed {
    /// What each names, by name, each name once and in order.
    pub runs: Runs,
    /// Those of [`APPLIED`] that are there.
    pub applied: Vec<String>,
}

impl Listed {
    /// What somebody applies, by name.
    pub fn tops(&self) -> Vec<String> {
        names::tops(&self.runs, &self.applied)
    }

    /// Every part of `name`, all the way down.
    pub fn parts(&self, name: &str) -> Vec<String> {
        names::parts(name, &self.runs)
    }

    /// What `name` is a part of: each blueprint somebody applies that
    /// it is reached from.
    pub fn part_of(&self, name: &str) -> Vec<String> {
        names::part_of(name, &self.runs, &self.applied)
    }
}

/// What the blueprints `library` has for no project run, read from their
/// texts as they are now and from what `actions` say of themselves. One
/// that does not read is listed and names nothing.
pub fn listed(library: &dyn Library, actions: &[ActionSpec]) -> Listed {
    let all = library.names(None);
    let known = |name: &str| all.iter().any(|one| one == name);
    let mut runs = Runs::new();
    for name in &all {
        let text = library.load(name, None).unwrap_or_default();
        let mut named: Vec<String> =
            names::of(&text, &known, actions).into_iter().map(|one| one.name).collect();
        named.sort();
        named.dedup();
        runs.insert(name.clone(), named);
    }
    let applied = APPLIED.iter().map(|(name, _)| (*name).to_string()).filter(|name| runs.contains_key(name));
    Listed { applied: applied.collect(), runs }
}
