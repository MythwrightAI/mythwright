//! Where each event handler has got to among the events, journal by
//! journal.
//!
//! A handler's place is the number of the last event it has dealt with:
//! handled, or passed over as not its. Starting again from there after a
//! restart is what makes delivery durable. It moves together with the run
//! an event starts (`Store::insert_triggered_run`), and here for the events
//! that start none.
//!
//! A place is a place in one journal, and is kept with what that journal
//! is called (`journal::Journal::name`). There is one journal today, the
//! factory's own, and a project is to have one each: a handler that hears
//! its project's events and the factory's own will follow two, and keeps
//! a place in each. Numbers are drawn from one giver, so no two events
//! have one, but they say nothing of the order of two events in different
//! journals: a place in one is never held against a place in another, and
//! nothing here moves one by what is written of the other.
//!
//! A place says whose it is as a run does (`crate::apart`): the project of
//! its instance, copied from the instance's row when the place is first
//! kept, and not written again as it moves.

use journal::optional;
use kernel::Result;
use rusqlite::{Connection, params};

use crate::store::Store;

/// Where a handler has got to in one journal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    pub instance: String,
    pub handler: String,
    /// What the journal it is a place in is called.
    pub journal: String,
    /// The number of the last event of that journal it has dealt with.
    pub seq: u64,
}

/// Keeps `seq` as where a handler has got to in `journal`, in whatever is
/// being written, whether or not it had a place there. One it had is
/// moved where it is, which writes its row and nothing else; one it had
/// not is made, and says whose it is.
pub(crate) fn put(
    connection: &Connection,
    instance: &str,
    handler: &str,
    journal: &str,
    seq: u64,
) -> rusqlite::Result<()> {
    let moved = connection.execute(
        "UPDATE places SET seq = ?4 WHERE instance = ?1 AND handler = ?2 AND journal = ?3",
        params![instance, handler, journal, seq as i64],
    )?;
    if moved == 0 {
        make(connection, instance, handler, journal, seq)?;
    }
    Ok(())
}

/// Gives a handler a place in `journal` where it has none there, in
/// whatever is being written; one it has is left where it is.
pub(crate) fn make(
    connection: &Connection,
    instance: &str,
    handler: &str,
    journal: &str,
    seq: u64,
) -> rusqlite::Result<usize> {
    connection.execute(
        &format!(
            "INSERT INTO places (instance, handler, journal, seq, project)
             VALUES (?1, ?2, ?3, ?4, {})
             ON CONFLICT (instance, handler, journal) DO NOTHING",
            crate::apart::of_instance(1)
        ),
        params![instance, handler, journal, seq as i64],
    )
}

impl Store {
    /// Where a handler has got to in the journal called `journal`, if it
    /// has a place there.
    pub async fn place(&self, instance: &str, handler: &str, journal: &str) -> Result<Option<u64>> {
        let (instance, handler, journal) = (instance.to_string(), handler.to_string(), journal.to_string());
        self.db
            .call(move |connection| {
                optional(connection.query_row(
                    "SELECT seq FROM places WHERE instance = ?1 AND handler = ?2 AND journal = ?3",
                    [&instance, &handler, &journal],
                    |row| row.get::<_, i64>(0),
                ))
            })
            .await
            .map(|seq| seq.map(|seq| seq as u64))
    }

    /// The least place any handler of `instance` has in the journal
    /// called `journal`: what it has dealt with of that journal at the
    /// most. Nothing where none of its handlers has a place there.
    pub async fn least_place(&self, instance: &str, journal: &str) -> Result<Option<u64>> {
        let (instance, journal) = (instance.to_string(), journal.to_string());
        self.db
            .call(move |connection| {
                connection.query_row(
                    "SELECT min(seq) FROM places WHERE instance = ?1 AND journal = ?2",
                    [&instance, &journal],
                    |row| row.get::<_, Option<i64>>(0),
                )
            })
            .await
            .map(|seq| seq.map(|seq| seq as u64))
    }

    /// Says where a handler has got to in the journal called `journal`:
    /// its place in any other is as it was.
    pub async fn set_place(&self, instance: &str, handler: &str, journal: &str, seq: u64) -> Result<()> {
        let (instance, handler, journal) = (instance.to_string(), handler.to_string(), journal.to_string());
        self.db.call(move |connection| put(connection, &instance, &handler, &journal, seq)).await
    }

    /// Brings the places of many handlers in the journal called
    /// `journal`, each an instance and a handler's name, forward to `seq`
    /// in one write, and gives how many were moved: none is a piece of
    /// work that wrote nothing. A place is never moved back, and a
    /// handler that has no place there is not given one: its instance has
    /// gone.
    pub async fn advance_places(
        &self,
        handlers: Vec<(String, String)>,
        journal: &str,
        seq: u64,
    ) -> Result<usize> {
        let journal = journal.to_string();
        self.db
            .call(move |connection| {
                let transaction = journal::whole(connection)?;
                let mut moved = 0;
                {
                    let mut advance = transaction.prepare(
                        "UPDATE places SET seq = ?4
                         WHERE instance = ?1 AND handler = ?2 AND journal = ?3 AND seq < ?4",
                    )?;
                    for (instance, handler) in &handlers {
                        moved += advance.execute(params![instance, handler, journal, seq as i64])?;
                    }
                }
                transaction.commit()?;
                Ok(moved)
            })
            .await
    }

    /// Takes away the places of one handler, in every journal it had
    /// one in: it is not in the text its instance now runs.
    pub async fn drop_places(&self, instance: &str, handler: &str) -> Result<()> {
        let (instance, handler) = (instance.to_string(), handler.to_string());
        self.db
            .call(move |connection| {
                connection
                    .execute("DELETE FROM places WHERE instance = ?1 AND handler = ?2", [&instance, &handler])
            })
            .await
            .map(|_| ())
    }

    /// Takes away the places of an instance's handlers but for those
    /// named: the ones the text it runs has. A place left behind by a
    /// handler that is gone would be where a later handler of that name
    /// began, and it would be handed everything since.
    pub async fn keep_places(&self, instance: &str, handlers: Vec<String>) -> Result<()> {
        let instance = instance.to_string();
        self.db
            .call(move |connection| {
                let mut statement =
                    connection.prepare("SELECT DISTINCT handler FROM places WHERE instance = ?1")?;
                let have: Vec<String> =
                    statement.query_map([&instance], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
                for gone in have.iter().filter(|name| !handlers.contains(name)) {
                    connection.execute(
                        "DELETE FROM places WHERE instance = ?1 AND handler = ?2",
                        params![instance, gone],
                    )?;
                }
                Ok(())
            })
            .await
    }
}
