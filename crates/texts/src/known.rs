//! What a text is checked against here: this device's actions and events,
//! and the blueprints its library has.
//!
//! Which actions and events are here is the registry's to say, which is
//! the runner's, and it is handed in as the language's own description of
//! them ([`Catalog`]): this knows no action, and where blueprints come
//! from ([`crate::library`]) knows none either. It came out of the runner
//! (`crates/conductor/src/known.rs`), which builds one for each check
//! ([`crate::Runner::check_own`]).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use blueprint::{ActionList, ActionSpec, Catalog, EventSpec, NoCatalog, Said, events, template};

use crate::compiled::{self, Asked};
use crate::library::Library;

/// Lets a blueprint be checked against this device's actions and library.
pub struct Known<'a> {
    /// This device's own actions and events: the registry, as a catalog.
    pub registry: &'a dyn Catalog,
    /// Why an action that is not here is not, where something says: an
    /// action a package brought, and took with it (`absent` of the
    /// registry, asked of what was never here).
    pub absent: &'a dyn Fn(&str) -> Option<String>,
    /// The actions that come and go, as they were when the check began.
    pub brought: &'a ActionList,
    pub library: &'a dyn Library,
    pub project: Option<&'a str>,
    /// The events that come and go, as they were when the check began.
    pub brought_events: &'a [EventSpec],
    /// What the blueprints that can be seen from here say to each other,
    /// read the first time it is asked for: most blueprints never ask.
    pub said: std::sync::OnceLock<compiled::Sayings>,
}

/// How many texts are kept with what each says, at most: more than a
/// library has, and no more of what a project's files once were than
/// that, however often they are written again.
const KEPT: usize = 512;

/// What one blueprint's text says: each topic written out, and what is
/// said on it.
type Sayings = Arc<[(String, Said)]>;

/// Every `emit` with a written topic in one blueprint's text, and what is
/// said on it: read off the text alone, with nothing of this device.
///
/// So it is a function of the text, and is kept by the text. A check of a
/// blueprint that hears something asks it of every blueprint its library
/// has ([`Known::all_said`]), and what applies one blueprint checks every
/// text that one reaches, each for itself: the same fifty texts were read
/// and checked a score of times over, 19 ms each time, which was a
/// quarter to two fifths of what applying such a blueprint and running
/// what it starts took. A text that does not read says nothing.
fn said_by(text: &str) -> Sayings {
    static SAID: Mutex<BTreeMap<String, Sayings>> = Mutex::new(BTreeMap::new());
    if let Some(said) = SAID.lock().expect("said lock").get(text) {
        return said.clone();
    }
    // Read with the lock let go: two that come to one text together both
    // read it, and keep the same.
    let read = blueprint::check(text, &NoCatalog).blueprint;
    let said: Sayings = read.map(|blueprint| events::emitted(&blueprint)).unwrap_or_default().into();
    let mut kept = SAID.lock().expect("said lock");
    if kept.len() >= KEPT {
        kept.clear();
    }
    kept.insert(text.to_string(), said.clone());
    said
}

impl Known<'_> {
    /// Every `emit` with a written topic in every blueprint the library
    /// has for this project.
    fn all_said(&self) -> &[(String, Said)] {
        self.said.get_or_init(|| {
            // Once for a start, whatever it compiles.
            compiled::said_for(self.project, || {
                let texts = self.library.names(self.project).into_iter();
                texts
                    .filter_map(|name| self.library.load(&name, self.project))
                    .flat_map(|text| said_by(&text).to_vec())
                    .collect()
            })
        })
    }

    /// Whether the library answers each of `asked` as it was answered:
    /// what a compile asked of it for one project, asked for this one.
    pub fn answers(&self, asked: &[Asked]) -> bool {
        asked.iter().all(|one| match one {
            Asked::Blueprint(name, was) => self.blueprint_there(name) == *was,
            Asked::Said(pattern, was) => self.said_on(pattern) == *was,
            Asked::Topics(were) => self.topics() == *were,
        })
    }

    fn blueprint_there(&self, name: &str) -> Option<bool> {
        if template::has_holes(name) {
            return None;
        }
        // One with more than one file is there, and is said to be so by
        // what reaches it (`reach`): not that nothing has the name.
        let library = self.library;
        Some(library.load(name, self.project).is_some() || library.refused(name, self.project).is_some())
    }

    fn said_on(&self, pattern: &str) -> Option<Said> {
        let mut said = Said::default();
        for (topic, one) in self.all_said() {
            if events::matches(pattern, topic) {
                said.add(one);
            }
        }
        Some(said)
    }

    fn topics(&self) -> Vec<String> {
        let mut topics: Vec<String> = self.all_said().iter().map(|(topic, _)| topic.clone()).collect();
        topics.sort();
        topics.dedup();
        topics
    }
}

impl Catalog for Known<'_> {
    fn action(&self, name: &str) -> Option<&ActionSpec> {
        self.registry.action(name).or_else(|| self.brought.action(name))
    }

    fn action_names(&self) -> Vec<&str> {
        let mut names = self.registry.action_names();
        names.extend(self.brought.action_names());
        names
    }

    fn absent(&self, name: &str) -> Option<String> {
        (self.absent)(name)
    }

    // What is asked of the library for the project is noted, so that a
    // start compiles a text once for every project that would be told
    // the same (`crate::compiled`).
    fn has_blueprint(&self, name: &str) -> Option<bool> {
        let there = self.blueprint_there(name);
        compiled::asked(|| Asked::Blueprint(name.to_string(), there));
        there
    }

    fn events(&self) -> Vec<&EventSpec> {
        self.registry.events().into_iter().chain(self.brought_events).collect()
    }

    fn knows_events(&self) -> bool {
        self.registry.knows_events()
    }

    fn said(&self, pattern: &str) -> Option<Said> {
        let said = self.said_on(pattern);
        compiled::asked(|| Asked::Said(pattern.to_string(), said.clone()));
        said
    }

    fn said_topics(&self) -> Vec<String> {
        let topics = self.topics();
        compiled::asked(|| Asked::Topics(topics.clone()));
        topics
    }
}
