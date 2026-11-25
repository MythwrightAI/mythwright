//! Whose each row kept here is.
//!
//! What a project has here (its tables, their records and what was said
//! of each, what a scan last saw of its files) is to be kept in the
//! project's own journal, and what is the factory's (people, the list of
//! projects, what groups them) is to stay where it is. Which is which is
//! said here once, a line for every table ([`TABLES`], in the words of
//! [`journal::apart`]), so that a row is moved, and the right file asked
//! for it, by what its table's line says and never by working it out. A
//! test holds every row to its line, and fails on a table that has none.
//!
//! Two things are held where a record is written, by the database: a
//! record's id begins with its project, and a record is never written
//! again as another project's (the rules made with the tables in
//! [`crate::collections`]).
//!
//! A channel, who is in it, what was said there and what is attached
//! there are the factory's, whatever project the channel says: a
//! conversation is not a project's thing. A channel says a project, or
//! none, and may come to say another
//! ([`crate::workspaces::Workspaces::place_channel`]); nothing is kept
//! in a project's file for that.

use journal::apart::Told;
use journal::{Journal, keeps};
use kernel::Result;

/// A line of a record's history is its record's: of the record's own
/// project while the record is there, and afterwards (or for a record
/// kept as a file or by a store, which has no row here) of the project
/// whose table its id is one of, read as [`crate::Collections::record`]
/// reads it: a board's cards are `<project>-<number>`, anything else
/// `<project>-<table>-<number>`, and a document or a mockup
/// `<project>-<table>:<where it is>`. Never by what the id begins with
/// alone: `shop-2-5` is a card of `shop-2`, or the fifth of `shop`'s
/// table `2`, and only the tables there are say which.
const OF_ITS_RECORD: &str = "coalesce(
    (SELECT record.project FROM records record WHERE record.id = kept.record),
    (SELECT each.project FROM collections each
      WHERE (each.name = 'board'
             AND substr(kept.record, 1, length(each.project) + 1) = each.project || '-'
             AND length(kept.record) > length(each.project) + 1
             AND substr(kept.record, length(each.project) + 2) NOT GLOB '*[^0-9]*')
         OR substr(kept.record, 1, length(each.project) + length(each.name) + 2)
            IN (each.project || '-' || each.name || '-', each.project || '-' || each.name || ':')
      ORDER BY each.name = 'board' DESC, length(each.project) DESC
      LIMIT 1))";

/// Every table kept here, each with how its rows are told apart by
/// project. A table added to what is kept here is a line of this list
/// beside its line of [`crate::copy::TABLES`]: the suite fails on one
/// that has none.
pub const TABLES: &[(&str, Told)] = &[
    // People, and how each is let in: nobody's.
    ("accounts", Told::Factory),
    ("sessions", Told::Factory),
    ("credentials", Told::Factory),
    ("invites", Told::Factory),
    // The themes somebody made, and the one they chose: kept with their
    // account, and nobody's project's.
    ("themes", Told::Factory),
    ("themes_chosen", Told::Factory),
    // How somebody arranged the side panel: kept with their account.
    ("arranged", Told::Factory),
    // How a time is written for somebody: kept with their account.
    ("clocked", Told::Factory),
    // How what somebody reads looks for them: kept with their account.
    ("appearance", Told::Factory),
    // Groups of people and agents, and who is in each: the factory's.
    ("groups", Told::Factory),
    ("group_members", Told::Factory),
    // The list a project is found in.
    ("projects", Told::Factory),
    // What a scan last saw of a project's files.
    ("project_files", Told::Says("project")),
    // A channel, who is in it, what was said there and what is attached
    // there: the factory's. A conversation is not a project's thing; the
    // project a channel says is where it is listed, and may change.
    ("channels", Told::Factory),
    ("channel_members", Told::Factory),
    // What a channel keeps to hand: the channel's, whatever project it names.
    ("channel_docs", Told::Factory),
    // What a channel offers of discussions, and each one: the channel's.
    ("discussing", Told::Factory),
    ("discussions", Told::Factory),
    ("messages", Told::Factory),
    ("attachments", Told::Factory),
    // What was addressed to somebody: a person's or an agent's, and no
    // project's whatever it is about.
    ("inbox", Told::Factory),
    ("follows", Told::Factory),
    ("follow_changes", Told::Factory),
    // A table and its records each say.
    ("collections", Told::Says("project")),
    ("records", Told::Says("project")),
    // A line of history is its record's.
    ("record_history", Told::Reads(OF_ITS_RECORD)),
    // Cards from before a board was a table: read once, never written.
    ("board_items", Told::Says("project")),
    // What groups projects and channels: the factory's own, whatever
    // each row names.
    ("workspaces", Told::Factory),
    ("workspace_projects", Told::Factory),
    ("workspace_channels", Told::Factory),
    // What was deleted and can be given back: a project's row or file is
    // its project's, and a project, which says none, the factory's.
    ("trash", Told::Says("project")),
    // What somebody asked to be changed of a record: its record's project's.
    ("asked", Told::Says("project")),
    // What is attached to a record: its record's project's.
    ("record_attachments", Told::Says("project")),
];

/// The tables of [`TABLES`] whose rows a project's own journal keeps,
/// each with how much that journal has to have been made to keep for it
/// ([`journal::keeps`], asked as "at least"): a project's tables, their
/// records and each record's history, and what a scan last saw of its
/// files. Such a row of a project whose journal keeps less is in the
/// factory's own, and so is every row of a table that is not here.
///
/// The order is the one they are to be taken out of a file in: a line
/// of history is told apart by its record and the tables there are, so
/// it goes before either.
pub const KEPT_APART: &[(&str, i64)] = &[
    ("record_history", journal::keeps::RECORDS),
    ("records", journal::keeps::RECORDS),
    ("collections", journal::keeps::RECORDS),
    ("project_files", journal::keeps::RECORDS),
    ("trash", journal::keeps::RECORDS),
    ("asked", journal::keeps::RECORDS),
    ("record_attachments", journal::keeps::RECORDS),
];

/// Says to the factory's own journal what a project's journal that keeps
/// these is made with: the tables of [`KEPT_APART`], by the steps the
/// factory's own are made by ([`Journal::makes`]). Asked as the studio
/// is opened, and by whatever makes such a journal without one (what
/// moves a project of a factory from before into a file of its own),
/// before it asks for the journal.
pub async fn makes(factory: &Journal) -> Result<()> {
    // A project whose tables the factory's own journal holds is given a
    // journal that keeps none of them (`crate::tables::held_here`).
    let held = Some(crate::tables::held_here as journal::Held);
    factory.makes(keeps::RECORDS, "collections", crate::tables::COLLECTIONS, held).await?;
    factory.makes(keeps::RECORDS, "files", crate::tables::FILES, None).await
}

/// Makes the tables of [`KEPT_APART`] in a database, or brings them up
/// to what this build knows, by the steps and under the names they are
/// made by in a project's file ([`makes`]): for whatever writes such a
/// file with no daemon, before any journal is said to keep it.
pub async fn make(db: &journal::Db) -> Result<()> {
    db.migrate("collections", crate::tables::COLLECTIONS).await?;
    db.migrate("files", crate::tables::FILES).await
}
