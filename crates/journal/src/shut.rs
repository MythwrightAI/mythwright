//! A journal that takes nothing more after a place it was told.
//!
//! Stopping a daemon when asked stops it wherever it happens to be. To
//! stop it at one place exactly (just after it has said a certain thing,
//! and before it says another) the journal is told the place: the event
//! that is there is the last it keeps, and whoever says anything after it
//! is kept waiting and never answered, as if the daemon had gone the
//! instant the event was written. What stops the daemon then finds the
//! journal as a daemon that went at that place would have left it. And
//! not the journal alone: its database does nothing more of what it is
//! asked (`Db::hold`), so that nothing is written down after the place by
//! anybody, said or not. What is on the disk is what was there when the
//! event was kept.
//!
//! This is what the simulation of a restart at every step stands on
//! (`crates/api/tests/suite/simulation.rs`), and what stops a daemon at a
//! step somebody names. A journal nobody told a place pays for one look at
//! an empty cell for each event.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use kernel::{Event, Result, Source};
use serde_json::Value;
use tokio::sync::Notify;

use crate::{Db, Journal};

/// Whether an event that has just been kept is the place to shut at.
type At = Arc<dyn Fn(&Event) -> bool + Send + Sync>;

#[derive(Default)]
pub(crate) struct Shut {
    at: std::sync::Mutex<Option<At>>,
    is: AtomicBool,
    told: Notify,
    /// The database goes on taking what is written after the place.
    open: AtomicBool,
    /// The databases of the journals that shut together: a factory's
    /// own, and those of its projects' journals. At the place every one
    /// of them does nothing more.
    dbs: std::sync::Mutex<Vec<Db>>,
}

impl Shut {
    pub(crate) fn is(&self) -> bool {
        self.is.load(Ordering::SeqCst)
    }

    /// Asked of each event as it is kept, by the one thread that keeps
    /// them: so an event that is kept and shuts the journal is the last
    /// there is. Asked too of what is only passed on, by whoever says it.
    pub(crate) fn look(&self, event: &Event) {
        if self.asks(event) {
            self.reach();
        }
    }

    /// Has `db` be one of the databases that do nothing more at the place.
    pub(crate) fn holds(&self, db: &Db) {
        self.dbs.lock().expect("journal lock").push(db.clone());
    }

    /// Has every database of the journals that shut together do nothing
    /// more of what it is asked.
    fn hold(&self) {
        for db in self.dbs.lock().expect("journal lock").iter() {
            db.hold();
        }
    }

    /// Whether `event` is the place: asked once of each event, in the
    /// order they are written, by the thread that writes them. Asked as
    /// an event is written and before the commit that keeps it is made,
    /// so that the commit ends with whatever reaches the place
    /// (`gather::cut`); what follows from its being the place
    /// ([`Shut::reach`]) follows the commit. With several journals that
    /// shut together it is asked on the thread of each, of its own
    /// events in their order.
    pub(crate) fn asks(&self, event: &Event) -> bool {
        self.at.lock().expect("journal lock").as_ref().is_some_and(|at| at(event))
    }

    /// The place has been come to: the event that is there is kept.
    pub(crate) fn reach(&self) {
        // Nothing more is written at all: not what would be said, and
        // not what anybody would have written down without saying, in
        // the journal the place is in or in any that shuts with it.
        // Unless the journal was told to leave the database open.
        if !self.open.load(Ordering::SeqCst) {
            self.hold();
        }
        self.is.store(true, Ordering::SeqCst);
        self.told.notify_waiters();
    }
}

impl Journal {
    /// Has the journal take nothing more once `at` says so of an event it
    /// has just kept. That event is the last: whoever says anything after
    /// it waits, and is never answered. `at` is asked on the thread that
    /// keeps events, of every one, in the order they are kept; and of
    /// each event that is passed on and not kept (its `seq` is 0), which
    /// may be the place as well: the journal then keeps nothing more
    /// from when that was said.
    pub fn shut_after(&self, at: impl Fn(&Event) -> bool + Send + Sync + 'static) {
        *self.inner.shut.at.lock().expect("journal lock") = Some(Arc::new(at));
    }

    /// Like [`Journal::shut_after`], with the database left open: nothing
    /// more is said after the place, and whatever is written down without
    /// being said still is. This is the other half of how a daemon really
    /// stops. Its work is ended wherever it is waiting, and something it
    /// had already asked the database to write is written all the same,
    /// with whatever would have been said of it afterwards never said.
    /// Each piece of work goes on until the next thing it would say, and
    /// waits there for ever.
    pub fn shut_after_saying(&self, at: impl Fn(&Event) -> bool + Send + Sync + 'static) {
        self.inner.shut.open.store(true, Ordering::SeqCst);
        self.shut_after(at);
    }

    /// Shuts the journal now, at whatever it last kept: for a place that
    /// is not something the daemon said (the program of an agent's turn
    /// saying it is there). What is being kept at this instant may or may
    /// not get in; nothing after it does.
    pub fn shut_now(&self) {
        self.inner.shut.hold();
        self.inner.shut.is.store(true, Ordering::SeqCst);
        self.inner.shut.told.notify_waiters();
    }

    /// Whether the journal has come to the place it was told and shut.
    pub fn is_shut(&self) -> bool {
        self.inner.shut.is()
    }

    /// Completes when the journal has shut.
    pub async fn shut(&self) {
        loop {
            // Listening before looking, so that a shutting in between is heard.
            let told = self.inner.shut.told.notified();
            if self.is_shut() {
                return;
            }
            told.await;
        }
    }

    /// Does what is asked of a journal that may shut while it is being
    /// done, and gives it up when the journal shuts: none, where it was
    /// given up or the journal had shut already.
    ///
    /// A journal that has shut answers nothing, so looking whether it
    /// has and then asking is not enough: one that shuts between the two
    /// leaves whoever asked waiting for ever. For whoever goes on after
    /// the journal has shut (a daemon that is stopping); everything else
    /// of the daemon's is meant to wait there.
    pub async fn unless_shut<T>(&self, asked: impl Future<Output = T>) -> Option<T> {
        tokio::select! {
            biased;
            () = self.shut() => None,
            done = asked => Some(done),
        }
    }

    /// Says one thing more in a journal that may have shut: that its
    /// daemon stopped, said by whoever stops it. A journal that shut stays
    /// shut to everybody else, and its database held. What is still about
    /// of a daemon that is stopping, in the moment before it is gone, gets
    /// nothing in after the place.
    pub async fn emit_last(&self, topic: &str, source: Source, data: Value) -> Result<Arc<Event>> {
        self.emit_as(topic, source, data, true).await
    }
}
