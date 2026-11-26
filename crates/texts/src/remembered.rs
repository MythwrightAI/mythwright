//! A text checked once, and the check remembered for whoever asks again
//! where the answers would be the same.
//!
//! A check of a text is a function of the text and of what it asks the
//! catalog it is checked against (`blueprint::Catalog`): which actions
//! there are and what each takes, which events, whether a blueprint of a
//! name is there, what the others say. So the same text checked against
//! a catalog that answers each of those questions as before comes to the
//! same, and is not worked out again: a fleet's apply checks a score of
//! the texts that ship, the start of each instance of one checks it
//! again, a factory started again checks them all, and every factory of
//! a test program checks the same ones. Worked out each time, that was a
//! seventh of what the API's suite asked of the processors (2026-11-26,
//! docs/build-times.md).
//!
//! What is remembered is each question asked and its answer, and a text
//! is given what it came to only where every one of them is answered the
//! same now: asked again of the catalog in hand, which goes on answering
//! from what is there now (a blueprint written or taken away, a package
//! added). An answer that is a whole description (an action's, every
//! event's) is held as what its text comes to, which is the same for the
//! same description and, but by chance of one in many billions, for no
//! other.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use blueprint::{ActionSpec, Catalog, Checked, EventSpec, Said};

/// A question a check asked of its catalog, with the answer.
#[derive(Clone, Debug, PartialEq)]
enum Question {
    Action(String, Option<String>),
    Names(String),
    Complete(bool),
    Absent(String, Option<String>),
    Blueprint(String, Option<bool>),
    Events(String),
    KnowsEvents(bool),
    Said(String, Option<Said>),
    Topics(Vec<String>),
}

/// What a whole description comes to.
fn described(what: &impl std::fmt::Debug) -> String {
    blueprint::revision(&format!("{what:?}"))
}

impl Question {
    /// The same question asked of `catalog`, with its answer there.
    fn again(&self, catalog: &dyn Catalog) -> Question {
        match self {
            Question::Action(name, _) => Question::Action(name.clone(), catalog.action(name).map(described)),
            Question::Names(_) => Question::Names(described(&catalog.action_names())),
            Question::Complete(_) => Question::Complete(catalog.is_complete()),
            Question::Absent(name, _) => Question::Absent(name.clone(), catalog.absent(name)),
            Question::Blueprint(name, _) => Question::Blueprint(name.clone(), catalog.has_blueprint(name)),
            Question::Events(_) => Question::Events(described(&catalog.events())),
            Question::KnowsEvents(_) => Question::KnowsEvents(catalog.knows_events()),
            Question::Said(pattern, _) => Question::Said(pattern.clone(), catalog.said(pattern)),
            Question::Topics(_) => Question::Topics(catalog.said_topics()),
        }
    }
}

/// A catalog that notes each question it is asked, and its answer.
struct Asking<'a> {
    catalog: &'a dyn Catalog,
    asked: RefCell<Vec<Question>>,
}

impl Asking<'_> {
    fn noted<T>(&self, answer: T, question: impl FnOnce(&T) -> Question) -> T {
        self.asked.borrow_mut().push(question(&answer));
        answer
    }
}

impl Catalog for Asking<'_> {
    fn action(&self, name: &str) -> Option<&ActionSpec> {
        self.noted(self.catalog.action(name), |spec| Question::Action(name.to_string(), spec.map(described)))
    }

    fn action_names(&self) -> Vec<&str> {
        self.noted(self.catalog.action_names(), |names| Question::Names(described(names)))
    }

    fn is_complete(&self) -> bool {
        self.noted(self.catalog.is_complete(), |complete| Question::Complete(*complete))
    }

    fn absent(&self, name: &str) -> Option<String> {
        self.noted(self.catalog.absent(name), |said| Question::Absent(name.to_string(), said.clone()))
    }

    fn has_blueprint(&self, name: &str) -> Option<bool> {
        self.noted(self.catalog.has_blueprint(name), |there| Question::Blueprint(name.to_string(), *there))
    }

    fn events(&self) -> Vec<&EventSpec> {
        self.noted(self.catalog.events(), |events| Question::Events(described(events)))
    }

    fn knows_events(&self) -> bool {
        self.noted(self.catalog.knows_events(), |knows| Question::KnowsEvents(*knows))
    }

    fn said(&self, pattern: &str) -> Option<Said> {
        self.noted(self.catalog.said(pattern), |said| Question::Said(pattern.to_string(), said.clone()))
    }

    fn said_topics(&self) -> Vec<String> {
        self.noted(self.catalog.said_topics(), |topics| Question::Topics(topics.clone()))
    }
}

/// One check: the text, what it asked, and what it came to.
struct Remembered {
    source: Arc<str>,
    asked: Vec<Question>,
    checked: Checked,
}

/// How many checks are remembered at most. Past that they are let go
/// all at once and remembered again as they are asked for: a factory
/// checks some fifty texts that ship and those of its projects, each
/// against a few catalogs.
const KEPT: usize = 1024;

/// Every check remembered, by what its text comes to.
static REMEMBERED: Mutex<Option<HashMap<String, Vec<Arc<Remembered>>>>> = Mutex::new(None);

/// `source` checked against `catalog`: what it came to before, where
/// every question it asked then is answered the same by `catalog` now,
/// and otherwise checked, and remembered.
pub fn checked(source: &str, catalog: &dyn Catalog) -> Checked {
    let revision = blueprint::revision(source);
    let before: Vec<Arc<Remembered>> = {
        let kept = REMEMBERED.lock().expect("remembered lock");
        kept.as_ref().and_then(|kept| kept.get(&revision)).cloned().unwrap_or_default()
    };
    // Asked again without the lock: what a catalog answers may be read
    // from the disk.
    let same = before.iter().find(|one| {
        *one.source == *source && one.asked.iter().all(|question| question.again(catalog) == *question)
    });
    if let Some(same) = same {
        return same.checked.clone();
    }
    let asking = Asking { catalog, asked: RefCell::default() };
    let checked = blueprint::check(source, &asking);
    // Each question once: a check asks after the same action at each
    // step that takes it.
    let mut asked: Vec<Question> = Vec::new();
    for question in asking.asked.into_inner() {
        if !asked.contains(&question) {
            asked.push(question);
        }
    }
    let one = Remembered { source: source.into(), asked, checked: checked.clone() };
    let mut kept = REMEMBERED.lock().expect("remembered lock");
    let kept = kept.get_or_insert_with(HashMap::new);
    if kept.values().map(Vec::len).sum::<usize>() >= KEPT {
        kept.clear();
    }
    kept.entry(revision).or_default().push(Arc::new(one));
    checked
}
