//! Whose each row the agents keep is.
//!
//! A project is to have a journal of its own, and what is kept of it is
//! to be moved there, project by project, without working out whose a
//! row is. So every table kept here says how its rows are told apart by
//! project ([`TABLES`], in the words every crate's list is written in:
//! `journal::apart`), and a test reads every row of each against its
//! line.
//!
//! An agent is the factory's: its name is its own in the whole factory,
//! it may be no project's, and whoever gives work looks at all of them.
//! Its turns are its project's. A turn says so in the write that queues
//! it, copied there from what the database keeps of its slot and never
//! from what whoever asked hands in, and the database refuses a write
//! that would make it another afterwards (the rule made with the
//! column): an agent that is given to another project later takes none
//! of the work it did with it. A turn that was kept before a turn said
//! so says nothing, and is reached through its slot as that now is.
//!
//! A turn is kept in its project's own journal where that journal was
//! made to keep turns ([`KEPT_APART`]; `crate::filed` says how each
//! place that reads one finds the file).
//!
//! A turn of an agent that is no project's says the empty text, so that
//! one that says nothing is told from it.

use journal::Db;
use journal::apart::Told;
use kernel::Result;

/// How a turn is told apart: by what it says, where the empty text is
/// nobody's; and one that says nothing, kept before a turn said so, by
/// the slot it names as that now is.
pub const TURNS: Told = Told::Reads(
    "CASE WHEN kept.project IS NULL
          THEN (SELECT project FROM slots WHERE name = kept.slot)
          ELSE nullif(kept.project, '') END",
);

/// Every table kept here, each with how its rows are told apart by
/// project (`journal::apart`). A table added to what the agents keep is
/// a line here: the suite fails on one that is made and has none, and on
/// one that is in a copy and not here.
pub const TABLES: &[(&str, Told)] = &[
    // The list agents are found in. A row names a project or none, and
    // may come to name another: nothing of it is moved.
    ("slots", Told::Factory),
    // What an agent was asked and what it answered: its project's, as
    // its slot said when the turn was queued.
    ("turns", TURNS),
    // The sign-ins the factory holds, which any agent may draw from.
    ("signins", Told::Factory),
];

/// The project of the slot bound as parameter `n`, as a turn says it:
/// for the write that queues one. Read in that write from what the
/// database keeps of the slot, so it is what the rows say and nothing
/// handed in. The empty text where the slot is no project's.
pub(crate) fn of_slot(n: usize) -> String {
    format!("coalesce((SELECT project FROM slots WHERE name = ?{n}), '')")
}

/// Makes the tables the agents keep in a database, or brings them up to
/// what this build knows: where agents are opened, and for whatever is
/// to hold such tables besides.
pub async fn make(db: &Db) -> Result<()> {
    db.migrate("agents", crate::schema::SCHEMA).await
}

/// The tables of [`TABLES`] whose rows a project's own journal keeps,
/// each with how much that journal has to have been made to keep for it
/// (`journal::keeps`, asked as "at least"): the turns its agents were
/// given. Such a turn of a project whose journal keeps less is in the
/// factory's own, and so is one that is nobody's.
pub const KEPT_APART: &[(&str, i64)] = &[("turns", journal::keeps::TURNS)];

/// Says to the factory's own journal what a project's journal that keeps
/// its agents' turns is made with: the tables the agents keep, by the
/// steps the factory's own are made by (`Journal::makes`). The list of
/// agents and the sign-ins are made there with them and hold nothing: an
/// agent is the factory's. Asked as agents are opened, and by whatever
/// makes such a journal without them, before it asks for the journal.
pub async fn makes(factory: &journal::Journal) -> Result<()> {
    factory.makes(journal::keeps::TURNS, "agents", crate::schema::SCHEMA, Some(held_here)).await
}

/// Whether the database of `connection`, the factory's own, holds a
/// turn of an agent of `project`: so of a project of a factory from
/// before whose agents were asked anything, and of nothing a journal of
/// the project's own keeps the turns of. Asked of the index of turns by
/// agent, for each agent of the project: no turn is read. A journal made
/// for such a project keeps what is run for it and no more, and its
/// turns stay where they are until it is moved apart.
///
/// So a turn that says the project, of an agent that was given to
/// another since or taken away, is not found by this: nothing says
/// whose a turn is but the turn, no index goes by it, and every turn
/// is not read as a project is added. A project with such a turn and
/// nothing else is given a journal that keeps turns, and the turn
/// stays in this file, where it is read by its agent, by its id and
/// for its conversation (`crate::filed`), until the project is moved
/// apart: the move is handed the turns whatever the project's journal
/// keeps (`crate::moved::tables`).
fn held_here(connection: &journal::copy::Connection, project: &str) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS (SELECT 1 FROM slots of_it WHERE of_it.project = ?1
                         AND EXISTS (SELECT 1 FROM turns WHERE slot = of_it.name))",
        [project],
        |row| row.get(0),
    )
}
