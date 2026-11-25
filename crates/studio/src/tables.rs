//! The tables a project's records are kept in, as they were made and
//! each thing added to them since: a step is added last and changes none
//! before it. They are here and not beside what reads them
//! ([`crate::collections`]) only because that file is as long as a file
//! may be.

pub(crate) const COLLECTIONS: &[&str] = &[
    "
    CREATE TABLE collections (
        project TEXT NOT NULL,
        name TEXT NOT NULL,
        fields TEXT NOT NULL,
        views TEXT NOT NULL,
        store TEXT NOT NULL,
        owner TEXT,
        created INTEGER NOT NULL,
        updated INTEGER NOT NULL,
        PRIMARY KEY (project, name)
    ) WITHOUT ROWID;

    CREATE TABLE records (
        id TEXT PRIMARY KEY,
        project TEXT NOT NULL,
        collection TEXT NOT NULL,
        number INTEGER NOT NULL,
        data TEXT NOT NULL,
        position REAL NOT NULL,
        created_by TEXT NOT NULL,
        created INTEGER NOT NULL,
        updated INTEGER NOT NULL
    );
    CREATE INDEX records_collection ON records (project, collection);
",
    "
    -- What each collection counts as the history of its records.
    ALTER TABLE collections ADD COLUMN history TEXT NOT NULL DEFAULT '[]';

    -- The history of each record: a line an event, written once. Nothing
    -- here is ever changed or taken away, whatever becomes of the record.
    CREATE TABLE record_history (
        record TEXT NOT NULL,
        seq INTEGER NOT NULL,
        ts INTEGER NOT NULL,
        topic TEXT NOT NULL,
        actor TEXT,
        who TEXT NOT NULL,
        said TEXT NOT NULL,
        data TEXT NOT NULL,
        PRIMARY KEY (record, seq)
    ) WITHOUT ROWID;
",
    "
    -- Where the store service of a collection kept by one is reached.
    ALTER TABLE collections ADD COLUMN at TEXT;
",
    "
    -- Which model its blueprint said it is (`models`), where it said one.
    ALTER TABLE collections ADD COLUMN model TEXT;
",
    "
    -- The scope changing its records takes, where its blueprint said one.
    ALTER TABLE collections ADD COLUMN takes TEXT;
",
    "
    -- The scope reading its records takes, where its blueprint said one.
    ALTER TABLE collections ADD COLUMN reads TEXT;
",
    "
    -- The collection whose records its own steer, where its blueprint
    -- said one. A column added is a line of the table's own description:
    -- no row is written again, however many there are.
    ALTER TABLE collections ADD COLUMN steers TEXT;
",
    "
    -- A record is its project's from the write that makes it, and never
    -- another's: its rows are to be kept in its project's own journal, and
    -- found there by what the row says. A record is written whole each
    -- time (`INSERT OR REPLACE`), so both are held as one is inserted: a
    -- new one's id begins with its project, and one there already is
    -- written again only as the project's it is. A rule is a line of the
    -- table's own description: no row is read or written by adding one,
    -- and one from before is held to nothing but staying whose it is.
    CREATE TRIGGER records_begin_with_their_project BEFORE INSERT ON records
    WHEN NOT EXISTS (SELECT 1 FROM records WHERE id = NEW.id)
     AND substr(NEW.id, 1, length(NEW.project) + 1) IS NOT NEW.project || '-'
    BEGIN
        SELECT RAISE(ABORT, 'a record''s id begins with the project it is of');
    END;
    CREATE TRIGGER records_stay_their_projects BEFORE INSERT ON records
    WHEN (SELECT project FROM records WHERE id = NEW.id) IS NOT NULL
     AND (SELECT project FROM records WHERE id = NEW.id) IS NOT NEW.project
    BEGIN
        SELECT RAISE(ABORT, 'a record is of the project it was made in, and of no other afterwards');
    END;
    CREATE TRIGGER records_project_stays BEFORE UPDATE OF id, project ON records
    WHEN NEW.project IS NOT OLD.project OR NEW.id IS NOT OLD.id
    BEGIN
        SELECT RAISE(ABORT, 'a record is of the project it was made in, and of no other afterwards');
    END;
",
    "
    -- The table it says it sits beside and what of that a row is about,
    -- where its blueprint said so: how it is shown (`shown`). A column
    -- added: no row is written again. It is the last step, after the
    -- rules, because that is where a step is added: a database that has
    -- run the steps before it runs only what comes after them, and this
    -- one was first written in before the rules, where a factory already
    -- made ran the rules a second time and would not start.
    ALTER TABLE collections ADD COLUMN beside TEXT;
",
    "
    -- The records of one column, in their order, and those that say they
    -- wait on others, each read by what it says and not by reading its
    -- table whole (`crate::column`): a card's place in its column, what a
    -- stage finds in its column and what waits on a card that is finished
    -- read the board whole, and what one card cost grew with every card
    -- beside it. Each holds only the records that say such a thing, so a
    -- row of a table that says neither is written as it was. And a
    -- table's records by their number, which the index of a table's
    -- records was not: the next record's number was the highest, found
    -- by going through every record of the table, and a table is read in
    -- the order of its numbers. It is the index it takes the place of
    -- with the number after, so it is what that was for too. Made over
    -- 300,000 records of 322 MB from memory, the three took 1.0 to 1.1 s,
    -- and over 30,000 of 32 MB a tenth of one or so.
    DROP INDEX records_collection;
    CREATE INDEX records_numbered ON records (project, collection, number);
    CREATE INDEX records_column ON records (project, collection, json_extract(data, '$.status'), position)
        WHERE json_extract(data, '$.status') IS NOT NULL;
    CREATE INDEX records_waiting ON records (project, collection, json_extract(data, '$.waiting'))
        WHERE json_extract(data, '$.waiting') >= 'after ' AND json_extract(data, '$.waiting') < 'after!';
",
    "
    -- What was deleted and can be given back (`crate::trash`): a row of a
    -- table, a file of a project, or a project, each with what puts it
    -- back where it was, who deleted it and when. A row of a project's
    -- is kept where that project's records are, so that a record goes
    -- in and out of it in the one write that takes it out of or puts it
    -- back among the records; a project is the factory's, and says no
    -- project. A record's number is among what is kept of it, so that
    -- the next record of its table is numbered after it (`trash_numbers`).
    CREATE TABLE trash (
        id TEXT PRIMARY KEY,
        kind TEXT NOT NULL,
        thing TEXT NOT NULL,
        project TEXT NOT NULL,
        of_table TEXT,
        called TEXT NOT NULL,
        kept TEXT NOT NULL,
        reads TEXT,
        by TEXT NOT NULL,
        at INTEGER NOT NULL
    );
    CREATE INDEX trash_at ON trash (at);
    CREATE INDEX trash_numbers ON trash (project, of_table, json_extract(kept, '$.number'))
        WHERE kind = 'record';
",
    "
    -- A table's records by their place, so that a record added is placed
    -- after the last by reading one row (`Collections::last_position`):
    -- found by going through every record of the table, a table of runs
    -- that gains a row for each run took longer to add to with each.
    -- Made over 300,000 records of 400 MB, it took a quarter of a second.
    CREATE INDEX records_placed ON records (project, collection, position);
",
    "
    -- What somebody asked to be changed of a record that they may not
    -- change themselves, kept beside the record so that its approval and
    -- the change are one write (`told::asks`, `docs/approvals.md`).
    CREATE TABLE asked (
        id TEXT PRIMARY KEY,
        project TEXT NOT NULL,
        record TEXT NOT NULL,
        collection TEXT NOT NULL,
        kind TEXT NOT NULL,
        cells TEXT NOT NULL,
        updated INTEGER NOT NULL,
        asker TEXT NOT NULL,
        why TEXT NOT NULL,
        named TEXT,
        place TEXT NOT NULL,
        status TEXT NOT NULL,
        decided TEXT,
        owes TEXT,
        at INTEGER NOT NULL
    );
    CREATE INDEX asked_of ON asked (record, status);
    CREATE INDEX asked_owes ON asked (owes) WHERE owes IS NOT NULL;
",
    "
    -- What is attached to a record (`crate::attaching`): a file of its own
    -- beside the database, in the folder a channel's attachments are in,
    -- with what it is called, what kind of thing it is, how big and who
    -- added it. Kept by its record's id, as a line of its history is, and
    -- where that record's project's records are.
    CREATE TABLE record_attachments (
        id TEXT PRIMARY KEY,
        project TEXT NOT NULL,
        record TEXT NOT NULL,
        name TEXT NOT NULL,
        media TEXT NOT NULL,
        size INTEGER NOT NULL,
        actor TEXT NOT NULL,
        who TEXT NOT NULL,
        created INTEGER NOT NULL
    );
    CREATE INDEX record_attachments_of ON record_attachments (record, created);
",
];

/// What a scan last saw of a project's files, as it is made in the file
/// of a project's journal that keeps the project's tables. In the
/// factory's own it is made with the list of projects
/// ([`crate::projects`]), which no project's file holds: so it is made
/// here under a name of its own, and only ever in such a file.
pub(crate) const FILES: &[&str] = &["
    CREATE TABLE project_files (
        project TEXT NOT NULL,
        path TEXT NOT NULL,
        mtime INTEGER NOT NULL,
        size INTEGER NOT NULL,
        PRIMARY KEY (project, path)
    ) WITHOUT ROWID;
"];

/// Whether the database of `connection`, the factory's own, holds a
/// table of `project`'s: so of a project of a factory from before, and
/// of nothing a journal of the project's own keeps the records of. A
/// table is never taken away, so one that was ever declared says so.
pub(crate) fn held_here(connection: &journal::copy::Connection, project: &str) -> rusqlite::Result<bool> {
    let made: bool = connection.query_row(
        "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'collections')",
        [],
        |row| row.get(0),
    )?;
    if !made {
        return Ok(false);
    }
    connection.query_row("SELECT EXISTS (SELECT 1 FROM collections WHERE project = ?1)", [project], |row| {
        row.get(0)
    })
}
