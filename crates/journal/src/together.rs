//! Writing something down and saying so, as one write.
//!
//! Most of what is kept beside the journal is written and then said: a
//! run is written as ended and `run.finished` is said, a record is put
//! where it goes and an event says it moved. Done as two pieces of work,
//! a daemon stopped between them leaves something written that nobody
//! was ever told of, and whatever waits to hear it waits for ever. No
//! look at a start puts that right for certain: it has to guess, from
//! what is written, what was said.
//!
//! So the two are one transaction ([`Journal::write`]). What is written
//! and the events that say it are kept together or not at all, wherever
//! the daemon is stopped; the events are numbered there, on the
//! database's thread, where nothing else is numbered in between, and a
//! run of numbers drawn for them is drawn there too, its mark kept with
//! them or not at all ([`crate::giver`]); and
//! whoever follows the journal hears them once the transaction is kept,
//! in their order, and never before.
//!
//! The rule: wherever something is written that an event then says, both
//! go through [`Journal::write`]. [`Journal::emit`] is for an event that
//! says something nobody wrote down beside it.
//!
//! **An event is kept in the journal that keeps the row it is said of.**
//! A factory has several journals, and what is said of a project whose
//! journal keeps what is run for it is kept in that journal
//! ([`Journal::emit`]). A row of such a project that is still kept in
//! the factory's own (its journal was made before a journal kept such
//! rows) cannot be one write with what is said of it: they are two
//! files. So [`Writing::say`] asks where the event it is handed is
//! kept, and one that is another journal's is said there once the write
//! is kept: written and then said, as everything was once, with whatever
//! looks for a row written and never said behind it. Where the row and
//! what is said of it are one journal's (what is nobody's, a project
//! from before, and a project whose journal keeps the row) nothing
//! changes: one write. What is kept in this journal whatever project it
//! names (a channel and what is said in it are the factory's) is said
//! with [`Writing::say_here`], and by itself with [`Journal::emit_here`].

use std::panic::Location;
use std::sync::Arc;

use kernel::{Event, Result, Source};
use serde_json::Value;

use crate::{Inner, Journal, Prepared};

/// A write under way: a transaction to write in, and a way to say events
/// that are kept if and when everything written in it is.
pub struct Writing<'a> {
    transaction: &'a rusqlite::Savepoint<'a>,
    inner: &'a Arc<Inner>,
    ts: i64,
    /// What the events said here are numbered from: the journal's run of
    /// numbers as this write found it, and put back as this write leaves
    /// it only once all of it is written.
    run: crate::giver::Run,
    /// The runs of numbers drawn for what is said here.
    drew: Vec<crate::giver::Run>,
    /// What was said here, each with whether it is kept or only passed
    /// on ([`Writing::pass_on`]).
    said: Vec<(Event, bool)>,
    /// What was said here that another journal keeps: said there once
    /// this write is kept, in the order it was said, each with whether
    /// it is kept there or only passed on.
    after: Vec<(Journal, String, Source, Value, bool)>,
}

impl<'a> Writing<'a> {
    /// The transaction, for what is written down: a whole of this
    /// write's own, inside whatever commit keeps it.
    pub fn transaction(&self) -> &'a rusqlite::Savepoint<'a> {
        self.transaction
    }

    /// Says an event of what is written here. It is kept with
    /// everything written here or not at all, where this journal is the
    /// one that keeps it; one that another journal keeps (it names a
    /// project whose own journal keeps what is said of it, and this is
    /// the factory's) is said there once this write is kept, and gives
    /// no number. Otherwise gives the number it has if it is kept: the
    /// next the journal has to give, counted here, so that a write which
    /// keeps nothing gives no number away.
    pub fn say(&mut self, topic: &str, source: Source, data: Value) -> rusqlite::Result<u64> {
        if let Some(own) = (Journal { inner: self.inner.clone() }).said_in(topic, &data) {
            self.after.push((own, topic.to_string(), source, data, true));
            return Ok(0);
        }
        self.kept_here(topic, source, data, true)
    }

    /// Says an event of what is written here that is passed on and not
    /// kept: for what nobody reads back from the journal, and something
    /// may be listening for as it is said. It is numbered here, as one
    /// that is kept is, passed on once this write is kept, in its order
    /// among what is said here, and is the place the journal was told to
    /// shut at as one that is kept would be: so everything about it is
    /// what it was when it was kept, but that it cannot be read back.
    /// Its number is no event's in the journal, which the journal
    /// writes down once it has any such hole (`numbered::HOLES`), so
    /// that the newest so many are counted from what is there. Gives its
    /// number; nought where another journal is to pass it on.
    pub fn pass_on(&mut self, topic: &str, source: Source, data: Value) -> rusqlite::Result<u64> {
        if let Some(own) = (Journal { inner: self.inner.clone() }).said_in(topic, &data) {
            self.after.push((own, topic.to_string(), source, data, false));
            return Ok(0);
        }
        if !self.inner.holed.load(std::sync::atomic::Ordering::SeqCst) {
            crate::numbered::keep_holes(self.transaction, "main")?;
            let inner = self.inner.clone();
            crate::gather::then(move |kept| {
                if kept {
                    inner.holed.store(true, std::sync::atomic::Ordering::SeqCst);
                }
            });
        }
        self.kept_here(topic, source, data, false)
    }

    /// When what is said here is said.
    pub fn ts(&self) -> i64 {
        self.ts
    }

    /// Says an event that this journal keeps whatever project it names,
    /// with everything written here or not at all: for what is said of
    /// a row that is this journal's for good.
    pub fn say_here(&mut self, topic: &str, source: Source, data: Value) -> rusqlite::Result<u64> {
        debug_assert!(
            crate::stays_here(topic),
            "`{topic}` is said as the factory's for good and is not in `journal::HERE`"
        );
        self.kept_here(topic, source, data, true)
    }

    /// Numbers an event and writes it in this write, where it is `kept`.
    fn kept_here(&mut self, topic: &str, source: Source, data: Value, kept: bool) -> rusqlite::Result<u64> {
        self.inner.held.look(topic, &data);
        let event = Event {
            seq: self.inner.number(self.transaction, &mut self.run, &mut self.drew, false)?,
            ts: self.ts,
            topic: topic.to_string(),
            source: Source { device: source.device.or_else(|| Some(self.inner.device.clone())), ..source },
            data,
        };
        if kept {
            crate::events::insert(self.transaction, &event)?;
        }
        let seq = event.seq;
        self.said.push((event, kept));
        Ok(seq)
    }

    /// How many events have been said here so far.
    pub fn said(&self) -> usize {
        self.said.len()
    }

    /// Keeps a small durable value ([`Journal::kv_set`]) with everything
    /// written here, or not at all: where something that follows the
    /// events has read to, say, with what it wrote for the last of them.
    pub fn keep(&self, namespace: &str, key: &str, value: &Value) -> rusqlite::Result<()> {
        self.transaction
            .execute(
                "INSERT INTO kv (namespace, key, value) VALUES (?1, ?2, ?3)
                 ON CONFLICT (namespace, key) DO UPDATE SET value = excluded.value",
                [namespace, key, &value.to_string()],
            )
            .map(|_| ())
    }

    /// A small durable value as this write finds it ([`Journal::kv_get`]):
    /// read inside the write, so that a change worked out from it is made
    /// from what is there, whatever was written a moment before.
    pub fn kept(&self, namespace: &str, key: &str) -> rusqlite::Result<Option<Value>> {
        let found: Option<String> = crate::optional(self.transaction.query_row_kept(
            "SELECT value FROM kv WHERE namespace = ?1 AND key = ?2",
            [namespace, key],
            |row| row.get(0),
        ))?;
        Ok(found.and_then(|value| serde_json::from_str(&value).ok()))
    }

    /// Takes a small durable value away with everything written here,
    /// or not at all: something that is forgotten in the write that
    /// says it is gone.
    pub fn forget(&self, namespace: &str, key: &str) -> rusqlite::Result<()> {
        self.transaction
            .execute("DELETE FROM kv WHERE namespace = ?1 AND key = ?2", [namespace, key])
            .map(|_| ())
    }
}

impl Journal {
    /// Passes an event on as [`Writing::pass_on`] does, by itself: in a
    /// write of its own, which writes nothing else, so that it is
    /// numbered and comes in its order on the database's thread, and is
    /// a place as one that is kept would be.
    pub async fn pass_on(&self, topic: &str, source: Source, data: Value) -> Result<Arc<Event>> {
        let topic = topic.to_string();
        let (_, said) = self.write(move |writing| writing.pass_on(&topic, source, data)).await?;
        said.into_iter().next().ok_or_else(|| {
            kernel::Error::unavailable("the event was passed on by another journal, and is not here")
        })
    }

    /// Does `work` in one transaction of the journal's database, and
    /// keeps the events it says ([`Writing::say`]) with what it writes:
    /// all of it or none. Gives what `work` gave, and the events as they
    /// were kept.
    ///
    /// Where `work` fails, nothing is kept and no number is given away.
    /// Followers hear the events after the transaction is kept, in the
    /// order they were said, and nothing else is said in between: this is
    /// the database's thread, where things are said one at a time.
    ///
    /// A journal that has shut ([`Journal::shut_after`]) keeps nothing of
    /// a write that says something, not what it wrote either: whoever
    /// asked waits and is never answered, as with [`Journal::emit`]. One
    /// that says nothing is a write like any other.
    #[track_caller]
    pub fn write<T, F>(
        &self,
        work: F,
    ) -> impl Future<Output = Result<(T, Vec<Arc<Event>>)>> + Send + use<T, F>
    where
        T: Send + 'static,
        F: FnOnce(&mut Writing<'_>) -> rusqlite::Result<T> + Send + 'static,
    {
        let inner = self.inner.clone();
        // Read here, where whoever says it is: when it is, and what
        // chain of events it is said in.
        let (ts, chain) = (self.now_ms(), kernel::chain::current());
        let kept = self.inner.db.call_from(Location::caller(), move |connection| {
            let transaction = crate::gather::whole(connection)?;
            let before = inner.run();
            let mut writing = Writing {
                transaction: &transaction,
                inner: &inner,
                ts,
                run: before,
                drew: Vec::new(),
                said: Vec::new(),
                after: Vec::new(),
            };
            let did = match work(&mut writing) {
                Ok(did) => did,
                Err(err) => {
                    inner.undraw(&writing.drew);
                    return Err(err);
                }
            };
            let (said, run, drew, after) = (writing.said, writing.run, writing.drew, writing.after);
            // Said after the place the journal shut at: it is no part of
            // what happened, and neither is what was written with it.
            if !(said.is_empty() && after.is_empty()) && inner.shut.is() {
                inner.undraw(&drew);
                return Ok(None);
            }
            transaction.commit()?;
            inner.put_back(before, run, drew);
            // Each asked whether it is the place, though the first may
            // be: they are kept together, and are all before it.
            let mut kept = Vec::with_capacity(said.len());
            for (event, stored) in said {
                kept.push(inner.written(event, chain, stored));
            }
            Ok(Some((did, kept, after)))
        });
        async move {
            match kept.await? {
                Some((did, mut kept, after)) => {
                    // What another journal keeps is said there now that
                    // what it is said of is written.
                    // One that cannot be said leaves what was written
                    // written, as writing and then saying always did:
                    // whoever looks for a row nobody was told of finds it.
                    for (own, topic, source, data, stored) in after {
                        let said = match stored {
                            true => own.emit_as(&topic, source, data, false).await,
                            false => Box::pin(own.pass_on(&topic, source, data)).await,
                        };
                        if let Ok(event) = said {
                            kept.push(event);
                        }
                    }
                    Ok((did, kept))
                }
                None => std::future::pending().await,
            }
        }
    }
}
