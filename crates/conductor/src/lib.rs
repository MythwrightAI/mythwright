//! The conductor runs blueprints.
//!
//! It turns a checked [`blueprint::Blueprint`] into something that happens:
//! instances with durable state, runs of their step graphs, and handlers
//! that react to events for as long as an instance lives. Everything it does
//! is recorded in the journal first, which is what makes it restartable: stop
//! it at any point, start it again, and it carries on.
//!
//! The conductor has no opinions about what work *is*. Building, hosting,
//! talking to agents, moving cards on a board: those are [`Action`]s that the
//! crates above register. This crate only knows how to run a graph of them
//! reliably.

mod action;
mod applied;
mod asked;
mod back;
mod beside;
mod carry;
mod child;
mod comparing;
pub mod copy;
mod current;
pub mod deepest;
mod downgraded;
mod engine;
pub mod environs;
pub mod events;
pub mod forked;
mod hooks;
mod intrinsic;
mod labels;
mod left;
pub mod letgo;
mod meets;
mod owed;
mod pinned;
pub mod profile;
mod reach;
mod routed;
mod run;
mod set;
mod settle;
mod sink;
mod state;
mod step;
mod taken_up;
pub mod taking;
mod task;
mod trigger;
mod unbegun;
mod update;
mod vouched;
pub mod whose;
mod why;

/// Where blueprints come from is a crate below this one ([`texts::library`]):
/// here under the names the runner always gave it.
pub mod library {
    pub use texts::library::*;
    /// Which of the blueprints that are no project's runs which.
    pub use texts::listed;
}

/// What a text is checked against, and what a start of conductors
/// compiled, are of texts ([`texts::known`], [`texts::compiled`]): here
/// under the names the runner always gave them. What is checked against
/// is handed this device's registry, which is the runner's.
pub(crate) use texts::{compiled, known};

/// What a run holds back from its rows until its next write is of what
/// the runner keeps ([`runs::held`]): here under the name it always had.
pub(crate) use runs::held;

/// Turns taken by key, and work done whole once it is begun, are of what
/// the runner keeps ([`runs::turns`], [`runs::whole`]), handed the engine
/// as what begins a task ([`runs::Hands`]): here under the names they
/// always had.
pub(crate) use runs::{turns, whole};

/// Getting an event to the handlers it is for is of what the runner keeps
/// ([`runs::delivery`]), handed the engine: here under the names it always
/// had.
pub(crate) mod delivery {
    pub(crate) use runs::delivery::{Deliveries, PLACES_KEPT, Wanted};
    pub(crate) type Inboxes = runs::delivery::Inboxes<crate::Engine>;
}

/// Who a pool gives a place to is a function of plain values, and is the
/// kernel's: here under the names the runner always gave it. `pool.turn`,
/// which asks it for a blueprint, is `blocks`'.
pub mod pool {
    pub use kernel::pool::*;
}

/// What parts of a text come to is the language's ([`blueprint::parts`]):
/// here under the names it always had. `text.put`, which asks it for a
/// blueprint, is `blocks`'.
pub mod words {
    pub use blueprint::parts::*;
}

pub use action::{
    Action, ActionCx, ActionError, ActionResult, Brought, FailureKind, Registry, RemoteCall, check_args,
};
pub use blueprint::differs::differences;
pub use comparing::{Chosen, Compared, Start, Starts};
pub use current::{Behind, Shown, Takes};
pub use downgraded::Downgraded;
pub use engine::{Applied, Apply, Engine, Remote, StopHook, Workdirs};
pub use library::{DirLibrary, Layers, Library, MemoryLibrary, standard};
pub use owed::{Keeping, OWN as PASSED_ON};
pub use pinned::{Linked, Moved, Reached};
pub use runs::delivery::PLACES_KEPT;
pub use runs::held::{BESIDE as WRITTEN_BESIDE, Quiet, holds};
pub use runs::{
    Definition, InstanceFilter, InstanceInfo, InstanceStatus, RunInfo, RunStatus, Started, StepInfo,
    StepStatus, Version,
};
pub use runs::{LOOKED_UP, blueprint_kept};
pub use sink::Forward;
pub use update::Took;
pub use vouched::Vouched;
pub use why::reason;

/// What the runner's machinery that came out of it is handed in place of
/// the engine ([`runs::Hands`]): a task begun is one of the conductor's
/// own, and none is begun by one that was halted.
impl runs::Hands for Engine {
    fn spawn(&self, work: impl std::future::Future<Output = ()> + Send + 'static) {
        Engine::spawn(self, work)
    }

    fn store(&self) -> &runs::Store {
        &self.inner.store
    }
}
