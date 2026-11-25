//! Whose an event is, and whether an instance hears it with nothing said.
//!
//! An event is a project's, or nobody's. It is the project its data
//! names (`project`), since what it is about says more than who said it:
//! a card of a project that is moved is that project's whoever moved it.
//! Where its data names none it is the project of the instance that said
//! it. And where there is neither it is nobody's, which is to say the
//! factory's own: that the daemon started, that a device joined, what a
//! caller outside said.
//!
//! A handler or a `wait` that says no `scope` hears what is its own
//! instance's project's and what is nobody's; an instance that is in no
//! project hears what is nobody's. Hearing another project's takes
//! saying so (`scope: fleet`). A project is what will have a journal and
//! a conductor of its own, so what something hears beyond its own has to
//! be read from its text and never assumed.
//!
//! A text taken by a program from before the word `project` said nothing
//! and heard the whole factory. It goes on doing so, read as it was
//! meant, until somebody applies it again: which it is, is kept with the
//! text (`Definition::heard`, `instances.heard`).

use kernel::Event;
use serde_json::Value;

use crate::engine::{Engine, Live};

// What will hear nothing where a text is applied is a function of the text
// and of what each event says of itself, and is the language's.
pub use blueprint::unheard::{Unheard, unheard};

/// What a text taken by this program says of how it reads no `scope`.
pub(crate) const PROJECT: &str = "project";

/// The mark of a text this program takes: it hears by project.
pub(crate) fn taken() -> Option<String> {
    Some(PROJECT.to_string())
}

/// The project an event's own data names, if it names one.
pub fn named(event: &Event) -> Option<&str> {
    event.data.get("project").and_then(Value::as_str).filter(|project| !project.is_empty())
}

impl Engine {
    /// The project `event` is of; none where it is nobody's.
    pub async fn whose(&self, event: &Event) -> Option<String> {
        // The factory's own place for tables (`@`) is no project's.
        if let Some(named) = named(event) {
            return (!kernel::place::is_factory(named)).then(|| named.to_string());
        }
        let said_by = event.source.instance.as_deref()?;
        if let Some(live) = self.live(said_by) {
            return live.meta.lock().expect("live lock").project.clone();
        }
        // In another conductor's hand: what a project's instance has
        // other parts of the factory say for it is kept in the
        // factory's journal, and is heard by every conductor.
        let others = self.others();
        if let Some(live) = others.iter().find_map(|other| other.live(said_by)) {
            return live.meta.lock().expect("live lock").project.clone();
        }
        // Said by one that is no longer in hand: as it was written down.
        // Kept for the next event it says, which is seldom far behind.
        if let Some(known) = self.inner.whose.lock().expect("engine lock").get(said_by) {
            return known.clone();
        }
        let mut found = self.inner.store.project_of(said_by).await.ok().flatten();
        for other in &others {
            if found.is_some() {
                break;
            }
            found = other.inner.store.project_of(said_by).await.ok().flatten();
        }
        let project = found.flatten();
        let mut kept = self.inner.whose.lock().expect("engine lock");
        if kept.len() >= KEPT {
            kept.clear();
        }
        kept.insert(said_by.to_string(), project.clone());
        project
    }

    /// The project `event` is of, heard in the journal called `heard_in`:
    /// what a project's journal keeps is that project's, and what the
    /// factory's own keeps is whose [`Engine::whose`] says. Asked by what
    /// hears, which knows where it heard: a project's conductor would
    /// otherwise have to ask another's whose an instance is.
    /// The journal that keeps an event something this conductor runs
    /// says, by what the event carries: the journal of the project it
    /// names, where that is another than this conductor's and keeps
    /// what is run for it, or the factory's where that project is kept
    /// there; and this conductor's own for one that names its own
    /// project or none. So an event is heard by whoever it was always
    /// heard by: what a project's journal keeps is that project's
    /// ([`Engine::whose_heard`]), and what the factory's keeps is the
    /// project's it names.
    pub(crate) fn kept_in(&self, data: &Value) -> journal::Journal {
        let named = data.get("project").and_then(Value::as_str).filter(|project| !project.is_empty());
        match named {
            Some(named) if self.inner.project.as_deref() != Some(named) => {
                self.inner.factory.rows_of(named).unwrap_or_else(|| self.inner.factory.clone())
            }
            _ => self.inner.own.clone(),
        }
    }

    pub(crate) async fn whose_heard(&self, heard_in: &str, event: &Event) -> Option<String> {
        match journal::project_of(heard_in) {
            Some(project) => Some(project.to_string()),
            None => self.whose(event).await,
        }
    }

    /// Whether `live` hears `event`, heard in the journal called
    /// `heard_in`, where what would hear it said no `scope`: it is its
    /// own project's or nobody's. One whose text is from before hears
    /// everything, as it did.
    pub(crate) async fn within_reach(&self, live: &Live, heard_in: &str, event: &Event) -> bool {
        let (heard, own) = {
            let meta = live.meta.lock().expect("live lock");
            (meta.heard.clone(), meta.project.clone())
        };
        if heard.as_deref() != Some(PROJECT) {
            return true;
        }
        match self.whose_heard(heard_in, event).await {
            None => true,
            Some(of) => own.as_deref() == Some(of.as_str()),
        }
    }
}

impl Engine {
    /// The journals `live` hears events from: its own project's and the
    /// factory's own, which keeps what is nobody's; and every one, where
    /// what hears says the whole factory (`everywhere`) or its text is
    /// from before and hears everything.
    pub(crate) fn hears_from(&self, live: &Live, everywhere: bool) -> Vec<journal::Journal> {
        let (heard, project) = {
            let meta = live.meta.lock().expect("live lock");
            (meta.heard.clone(), meta.project.clone())
        };
        let factory = &self.inner.factory;
        if everywhere || heard.as_deref() != Some(PROJECT) {
            return factory.every();
        }
        // What is said of its project is kept in that project's journal:
        // this conductor's own, where it is the project's conductor.
        let own = match project == self.inner.project {
            true => self.inner.own.clone(),
            false => factory.of(project.as_deref()),
        };
        match own.name() == factory.name() {
            true => vec![own],
            false => vec![factory.clone(), own],
        }
    }
}

/// How many instances that are gone are remembered by project at most.
const KEPT: usize = 4096;
