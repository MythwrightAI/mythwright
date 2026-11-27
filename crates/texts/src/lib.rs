//! The texts something running goes on with.
//!
//! A blueprint is a text, found by its name ([`library`]): a project's own
//! before the shared ones, and those before the ones that ship. And what
//! a conductor keeps of texts beside what it runs is one keeper's
//! ([`Texts`]), in that conductor's own journal:
//!
//! - what a text reaches, each text checked, and the last of each name
//!   that checked ([`reach`]);
//! - what a running tree was held to when somebody applied it
//!   ([`pinned`]);
//! - which of a project's own files were vouched for, and by whom
//!   ([`vouched`]);
//! - how a changed text is come by, and how one that is taken lands
//!   ([`taking`]);
//! - what a project's fork of a blueprint that ships was forked from
//!   ([`forked`]).
//!
//! And what a text is checked against, handed this device's actions and
//! events by whoever runs ([`known`]), and what a start of conductors
//! compiled, once for all that run the same text ([`compiled`]).
//!
//! And what is said of an instance from two texts, with nothing kept:
//! whether it is behind the one now there, and how it would come to run
//! it ([`behind`]). And of the blueprints that are no project's, which
//! runs which, for whoever lists them ([`listed`]).
//!
//! There is no engine in it and no registry: nothing here holds an
//! instance in hand, knows an action, applies anything or says an event.
//! The runner stands on it, offers what is here under the names it always
//! gave (`conductor::library`, `conductor::Moved`, `conductor::taking`),
//! and is asked two things back ([`Runner`]). What is kept is kept under
//! the names the runner always kept it under (`runs::beside`).

pub mod behind;
mod broken;
pub mod compiled;
pub mod forked;
pub mod known;
pub mod library;
pub mod listed;
pub mod pinned;
pub mod ran;
pub mod reach;
pub mod remembered;
pub mod taking;
pub mod vouched;

use std::sync::Arc;

use blueprint::Checked;
use journal::Journal;
use kernel::BoxFut;
use runs::Store;

use library::Library;

/// What is asked back of whatever runs, by what is kept of texts.
///
/// It is two questions, and each is here because the answer is in the
/// runner's hand and nowhere this crate could read it as it is:
///
/// - **`check_own`**: a text is checked against the actions and events of
///   this device, which are the registry's, and the registry is what
///   actions are run through. That is the runner's.
/// - **`above`**: what an instance is called and what started it. It is
///   written in the instance's row, which this crate could read; but the
///   runner has every instance it runs in hand and answers from there,
///   asking the database nothing. What a tree is held to is looked for
///   upwards from a step's instance each time a step starts something,
///   so read from the rows it would be a question of the database for
///   each level of the tree at every such start: more work on the
///   journal's one thread, and other counts of answers than the plays of
///   the runner's suite, which make their orders by counting them, are
///   written for.
///
/// Everything else is handed in as plain values by whoever calls (what
/// of a request is read, which revisions something else still wants
/// kept), or is the runner's to do around the call (which conductor
/// keeps an instance, saying an event, noting something beside an
/// instance). A third question would have to be something that is so
/// only in the runner's hand at the moment it is asked, that cannot be
/// handed in when the call is made, and that this crate cannot read
/// where it is written without asking the database more often than the
/// runner did. What can be handed in is handed in.
pub trait Runner: Sync {
    /// The text alone, checked against this device's actions and events.
    fn check_own(&self, source: &str, project: Option<&str>) -> Checked;

    /// What an instance is called and what started it.
    fn above<'a>(&'a self, instance: &'a str) -> BoxFut<'a, Option<(String, Option<String>)>>;
}

/// What one conductor keeps of texts, in its own journal, and what every
/// conductor of a factory shares of it.
pub struct Texts {
    /// The journal of the conductor this is for: where it keeps what it
    /// runs, and what it keeps of texts beside that.
    own: Journal,
    /// The factory's own journal: the same, for the factory's conductor.
    factory: Journal,
    /// The project that conductor runs for; none for the factory's own.
    project: Option<String>,
    store: Store,
    library: Arc<dyn Library>,
    /// Held while what is kept for an instance is read, changed and
    /// written back: one at a time, for every conductor of the factory.
    vouching: Arc<tokio::sync::Mutex<()>>,
    /// The last text of each name that was remembered ([`reach`]).
    known: reach::Known,
    /// What has been read of what trees are held to ([`pinned`]).
    pinned: pinned::KeptTexts,
    /// How texts are come by here, and how a taken one lands
    /// ([`taking`]): one for every conductor of the factory.
    taking: Arc<taking::Kept>,
}

impl Texts {
    /// For the factory's own conductor.
    pub async fn open(journal: Journal, store: Store, library: Arc<dyn Library>) -> Texts {
        let taking = taking::load(&journal).await;
        Texts {
            own: journal.clone(),
            factory: journal,
            project: None,
            store,
            library,
            vouching: Arc::default(),
            known: Default::default(),
            pinned: Default::default(),
            taking: Arc::new(taking),
        }
    }

    /// For a conductor of `project`, beside the one this is for: what it
    /// keeps it keeps in `own`, and what every conductor shares it is
    /// handed.
    pub fn beside(&self, project: &str, own: Journal, store: Store) -> Texts {
        Texts {
            own,
            factory: self.factory.clone(),
            project: Some(project.to_string()),
            store,
            library: self.library.clone(),
            vouching: self.vouching.clone(),
            known: Default::default(),
            pinned: Default::default(),
            taking: self.taking.clone(),
        }
    }
}
