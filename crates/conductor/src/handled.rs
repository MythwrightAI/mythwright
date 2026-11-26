//! Whether what was written has been handled by whatever hears it.
//!
//! What somebody writes is heard by handlers after the write is answered,
//! each in its own time. Something that writes what a running instance
//! takes its direction from may wait, before it answers, until each
//! handler that hears the write has done with it ([`Engine::handled`]),
//! so that whatever it asks next goes the way it said. Every conductor
//! has handlers of its own listening, to its own journal and to others,
//! so each is asked. Nothing is kept: a wait ends with the daemon.

use journal::Journal;
use kernel::Event;

use crate::Engine;

impl Engine {
    /// Waits until every handler listening now that `event`, said in
    /// `journal`, is for has done with it: its run for it has ended (been
    /// begun, for one that takes turns by a key), or it passed it over.
    /// Nothing is waited for while the daemon begins nothing new, since
    /// no run would be begun.
    pub async fn handled(&self, journal: &Journal, event: &Event) {
        for engine in self.every() {
            if self.holds_new_work() {
                return;
            }
            if let Some(delivery) = engine.inner.delivery.found(journal.name()) {
                delivery.done_with(event).await;
            }
        }
    }

    /// The handlers that have not yet done with `event`, said in
    /// `journal`: each an instance and the handler's name.
    pub fn handling(&self, journal: &Journal, event: &Event) -> Vec<(String, String)> {
        let found = self.every().into_iter().filter_map(|engine| engine.inner.delivery.found(journal.name()));
        found.flat_map(|delivery| delivery.owing(event)).collect()
    }
}
