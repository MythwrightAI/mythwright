//! One test binary for the crate; add modules here.

mod appearance;
mod arranged;
mod clocked;
mod given;
mod groups;
mod handle;
mod identity;
mod outside;
mod themes;

use std::sync::Arc;

use journal::Journal;
use kernel::ManualClock;
use people::{Appearance, Arranged, Clocked, Identity, Themes};

pub struct Rig {
    pub identity: Identity,
    pub themes: Themes,
    pub arranged: Arranged,
    pub clocked: Clocked,
    pub appearance: Appearance,
    pub journal: Journal,
    pub clock: ManualClock,
}

impl Rig {
    pub async fn new() -> Rig {
        let clock = ManualClock::new();
        let journal = Journal::memory(Arc::new(clock.clone())).await.unwrap();
        let identity = Identity::open(journal.clone()).await.unwrap();
        let themes = Themes::open(journal.clone()).await.unwrap();
        let arranged = Arranged::open(journal.clone()).await.unwrap();
        let clocked = Clocked::open(journal.clone()).await.unwrap();
        let appearance = Appearance::open(journal.clone()).await.unwrap();
        Rig { identity, themes, arranged, clocked, appearance, journal, clock }
    }
}
