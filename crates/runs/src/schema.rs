//! The tables the runner's rows are kept in, and what they have become:
//! each step takes them from one version to the next, and steps are only
//! ever added. A journal an earlier program made is brought up by the
//! steps it has not had, as it is opened; what each of them does to a
//! large one is said with it.

pub(crate) const SCHEMA: &[&str] = &[
    "
    CREATE TABLE instances (
        id TEXT PRIMARY KEY,
        blueprint TEXT NOT NULL,
        revision TEXT NOT NULL,
        source TEXT NOT NULL,
        params TEXT NOT NULL,
        state TEXT NOT NULL,
        status TEXT NOT NULL,
        parent TEXT,
        key TEXT NOT NULL,
        target TEXT,
        project TEXT,
        principal TEXT NOT NULL,
        holds TEXT NOT NULL DEFAULT '[]',
        error TEXT,
        created INTEGER NOT NULL,
        updated INTEGER NOT NULL
    );
    CREATE INDEX instances_parent ON instances (parent);
    CREATE INDEX instances_status ON instances (status);

    CREATE TABLE runs (
        id TEXT PRIMARY KEY,
        instance TEXT NOT NULL,
        flow TEXT NOT NULL,
        revision TEXT NOT NULL,
        status TEXT NOT NULL,
        event TEXT NOT NULL,
        outputs TEXT NOT NULL DEFAULT 'null',
        error TEXT,
        started INTEGER NOT NULL,
        finished INTEGER
    );
    CREATE INDEX runs_instance ON runs (instance, started);
    CREATE INDEX runs_status ON runs (status);

    CREATE TABLE steps (
        run TEXT NOT NULL,
        step TEXT NOT NULL,
        status TEXT NOT NULL,
        attempt INTEGER NOT NULL,
        tolerated INTEGER NOT NULL DEFAULT 0,
        output TEXT NOT NULL DEFAULT 'null',
        error TEXT,
        saved TEXT NOT NULL DEFAULT 'null',
        started INTEGER NOT NULL,
        finished INTEGER,
        PRIMARY KEY (run, step)
    ) WITHOUT ROWID;

    CREATE TABLE trigger_cursors (
        instance TEXT NOT NULL,
        trigger TEXT NOT NULL,
        seq INTEGER NOT NULL,
        PRIMARY KEY (instance, trigger)
    ) WITHOUT ROWID;
",
    "
    -- The definition an instance had before the update now in progress, as
    -- JSON, so a failed update can put it back (even across a restart).
    ALTER TABLE instances ADD COLUMN previous TEXT;
",
    "
    -- Every definition an instance has been given, in order: the text and
    -- the params it was applied with each time, who applied it, and whether
    -- it held. An instance already here starts its history with what it has.
    CREATE TABLE instance_versions (
        instance TEXT NOT NULL,
        version INTEGER NOT NULL,
        revision TEXT NOT NULL,
        source TEXT NOT NULL,
        params TEXT NOT NULL,
        actor TEXT NOT NULL,
        actor_name TEXT NOT NULL,
        applied INTEGER NOT NULL,
        outcome TEXT NOT NULL DEFAULT 'applied',
        PRIMARY KEY (instance, version)
    ) WITHOUT ROWID;
    INSERT INTO instance_versions (instance, version, revision, source, params, actor, actor_name, applied)
    SELECT id, 1, revision, source, params, '', '', updated FROM instances;
",
    "
    -- What everything measured under an instance also carries, as JSON: the
    -- labels its parent's step gave it, added to its parent's own.
    ALTER TABLE instances ADD COLUMN labels TEXT NOT NULL DEFAULT '{}';
",
    "
    -- On whose authority each version ran, as JSON: who applied it and the
    -- scopes they held then. No token is in it. Until now an instance kept
    -- the authority of whoever applied it first, whoever applied over it,
    -- so that is what every version already here ran with.
    ALTER TABLE instance_versions ADD COLUMN principal TEXT;
    UPDATE instance_versions
    SET principal = (SELECT principal FROM instances WHERE instances.id = instance_versions.instance);
",
    "
    -- What each element of a `for_each` step gave, while the step is under
    -- way: a loop that the conductor stopped in the middle of goes on from
    -- the element it was at. An element that has begun and not finished
    -- has no output yet. Gone when the step's run has ended.
    CREATE TABLE step_elements (
        run TEXT NOT NULL,
        step TEXT NOT NULL,
        position INTEGER NOT NULL,
        output TEXT,
        PRIMARY KEY (run, step, position)
    ) WITHOUT ROWID;
",
    "
    -- The last thing the journal had kept when an element began: what was
    -- said for that element is after it, which is where a step done again
    -- for it after a restart looks for what it had said already. A column
    -- added to a table that holds only steps under way.
    ALTER TABLE step_elements ADD COLUMN began INTEGER;
",
    "
    -- How far what is done around a step, and as a run ends, has come:
    -- which entries were done, and how the step's action ended, so that
    -- one taken up among them goes on from the next. Each is a column of
    -- a row that is written anyway, and is read only when something is
    -- taken up after a restart.
    ALTER TABLE steps ADD COLUMN hooks TEXT;
    ALTER TABLE runs ADD COLUMN hooks TEXT;
",
    "
    -- An instance that failed says so, what it started is stopped, and
    -- then it says that it is over. That it has failed is written with a
    -- note that the rest is owed, taken off in the write that says it is
    -- over (`together`). A column of a row that is written anyway, and an
    -- index of the rows that owe alone: none, but for the moment an
    -- instance is being taken down. Making it reads the table once.
    ALTER TABLE instances ADD COLUMN owes TEXT;
    CREATE INDEX instances_owing ON instances (id) WHERE owes IS NOT NULL;
    -- A program from before this wrote an instance as active and then
    -- said so, and may have been stopped between the two. Each instance
    -- that is active as this is made is owed one look for that, the
    -- first time a conductor starts on it, and none after: what this
    -- program writes it says in the same write.
    UPDATE instances SET owes = 'said' WHERE status = 'active';
",
    "
    -- How far what an instance does as it stops has come (`on_stop`),
    -- and why it was stopped: a stop is done in no run, so it is kept in
    -- the instance's own row, with a note that the stop is owed (`owes`)
    -- from before the first of it until the instance is written as
    -- stopped. A column of a row that is written anyway.
    ALTER TABLE instances ADD COLUMN hooks TEXT;
",
    "
    -- What an instance is to be carried over to, kept with the run that
    -- will do it from the write that makes the run until the text is
    -- taken or the run ends: whoever asked was answered, and a conductor
    -- stopped in between carries it over when one starts (`carry`). A
    -- column added to the table, which rewrites nothing.
    ALTER TABLE runs ADD COLUMN carries TEXT;
",
    "
    -- How the text an instance runs reads a handler or a wait that says
    -- no `scope`: `project` in one taken by a program that has the word,
    -- and nothing in one from before, which is read as it was meant then,
    -- the whole factory (`whose`). A column added to the table, which
    -- rewrites nothing; a row from before is as it was until its
    -- instance is applied again.
    ALTER TABLE instances ADD COLUMN heard TEXT;
",
    "
    -- Whose each row is, said where it is asked by. A project is to have a
    -- journal of its own, and what is kept for what runs is to be moved
    -- there and not worked out: so a row says its project, or is reached
    -- by its key from one that does (`apart`).
    --
    -- Nothing here reads a table from before, or an index of one, but
    -- for the rows of what is alive and under way, which a conductor
    -- reads as it starts in any case: no index is made over a table that
    -- has rows, and none is taken away (`shed`). Timed on a journal from
    -- before of several hundred megabytes, in the notes (*A journal for
    -- each project*).

    -- What is alive, each with its project (the empty text for what is
    -- nobody's): a row for an instance from when it is written until it
    -- is over, kept by the database as an index is (the rules below).
    -- It is what is asked for what is alive, by project or of every
    -- project, in place of an index of every instance's status that
    -- only what was alive was ever asked of.
    CREATE TABLE alive (
        instance TEXT PRIMARY KEY,
        project TEXT NOT NULL
    ) WITHOUT ROWID;
    CREATE INDEX alive_project ON alive (project);
    INSERT INTO alive (instance, project)
        SELECT id, coalesce(project, '') FROM instances INDEXED BY instances_status
        WHERE status IN ('starting', 'active');

    -- A run says its project as that table does, copied from it in the
    -- write that makes the run. One a program from before wrote says
    -- nothing, and is reached through its instance, as every run can be;
    -- those of them still under way are told here. And the runs under
    -- way are kept by project as what is alive is, in place of an index
    -- of every run's status.
    ALTER TABLE runs ADD COLUMN project TEXT;
    UPDATE runs INDEXED BY runs_status
       SET project = coalesce((SELECT project FROM alive WHERE instance = runs.instance),
                              (SELECT project FROM instances WHERE id = runs.instance), '')
     WHERE status = 'running';
    CREATE TABLE under_way (
        project TEXT NOT NULL,
        run TEXT NOT NULL,
        instance TEXT NOT NULL,
        PRIMARY KEY (project, run)
    ) WITHOUT ROWID;
    INSERT INTO under_way (project, run, instance)
        SELECT project, id, instance FROM runs INDEXED BY runs_status WHERE status = 'running';

    -- A handler's place is a place in one journal, and is kept with what
    -- that journal is called: one that follows two has a place in each,
    -- and neither moves the other. It says its project as a run does,
    -- when it is first kept. The places there were are all in the one
    -- journal there was, the factory's own, and those of instances still
    -- alive are brought over; the rest were nobody's to go on from.
    CREATE TABLE places (
        instance TEXT NOT NULL,
        handler TEXT NOT NULL,
        journal TEXT NOT NULL,
        seq INTEGER NOT NULL,
        project TEXT NOT NULL,
        PRIMARY KEY (instance, handler, journal)
    ) WITHOUT ROWID;
    CREATE INDEX places_project ON places (project);
    INSERT INTO places (instance, handler, journal, seq, project)
        SELECT held.instance, held.trigger, 'factory', held.seq, alive.project
        FROM alive CROSS JOIN trigger_cursors held ON held.instance = alive.instance;

    -- An instance is in `alive` while its row says it is alive, and a
    -- run in `under_way` while its row says it is under way, whatever
    -- writes the row.
    CREATE TRIGGER alive_begins AFTER INSERT ON instances
    WHEN NEW.status IN ('starting', 'active')
    BEGIN
        INSERT INTO alive (instance, project) VALUES (NEW.id, coalesce(NEW.project, ''));
    END;
    CREATE TRIGGER alive_ends AFTER UPDATE OF status ON instances
    WHEN OLD.status IN ('starting', 'active') AND NEW.status NOT IN ('starting', 'active')
    BEGIN
        DELETE FROM alive WHERE instance = OLD.id;
    END;
    CREATE TRIGGER alive_again AFTER UPDATE OF status ON instances
    WHEN OLD.status NOT IN ('starting', 'active') AND NEW.status IN ('starting', 'active')
    BEGIN
        INSERT INTO alive (instance, project) VALUES (NEW.id, coalesce(NEW.project, ''));
    END;
    CREATE TRIGGER alive_gone AFTER DELETE ON instances
    BEGIN
        DELETE FROM alive WHERE instance = OLD.id;
    END;
    CREATE TRIGGER under_way_begins AFTER INSERT ON runs
    WHEN NEW.status = 'running'
    BEGIN
        INSERT INTO under_way (project, run, instance) VALUES (coalesce(NEW.project, ''), NEW.id, NEW.instance);
    END;
    CREATE TRIGGER under_way_ends AFTER UPDATE OF status ON runs
    WHEN OLD.status = 'running' AND NEW.status != 'running'
    BEGIN
        DELETE FROM under_way WHERE project = coalesce(OLD.project, '') AND run = OLD.id;
    END;
    CREATE TRIGGER under_way_again AFTER UPDATE OF status ON runs
    WHEN OLD.status != 'running' AND NEW.status = 'running'
    BEGIN
        INSERT INTO under_way (project, run, instance) VALUES (coalesce(NEW.project, ''), NEW.id, NEW.instance);
    END;
    CREATE TRIGGER under_way_gone AFTER DELETE ON runs
    WHEN OLD.status = 'running'
    BEGIN
        DELETE FROM under_way WHERE project = coalesce(OLD.project, '') AND run = OLD.id;
    END;

    -- What says whose a row is, is said rightly and once, whoever
    -- writes it. An instance is of the project of what it is started
    -- under; a run and a place say the project of their instance as they
    -- are written, which is found in what is alive; and none of it is
    -- another afterwards: an instance's project, a run's instance, and
    -- what a run or a place said. The database refuses a write that
    -- would have it otherwise. (A run from before that has ended and
    -- says nothing may be told whose it is, once: one under way is kept
    -- by what it said as it began.) What is refused another value costs
    -- nothing where no such column is written.
    CREATE TRIGGER instances_of_one_project BEFORE INSERT ON instances
    WHEN NEW.parent IS NOT NULL
     AND EXISTS (SELECT 1 FROM instances WHERE id = NEW.parent)
     AND coalesce(NEW.project, '') IS NOT
         coalesce((SELECT project FROM alive WHERE instance = NEW.parent),
                  (SELECT project FROM instances WHERE id = NEW.parent), '')
    BEGIN
        SELECT RAISE(ABORT, 'an instance is of the project of what it is started under, and a tree of instances is never of two');
    END;
    CREATE TRIGGER runs_say_whose BEFORE INSERT ON runs
    WHEN NEW.project IS NOT
         coalesce((SELECT project FROM alive WHERE instance = NEW.instance),
                  (SELECT project FROM instances WHERE id = NEW.instance), '')
    BEGIN
        SELECT RAISE(ABORT, 'a run says whose it is as it is written: the project of its instance, and the empty text for nobody''s');
    END;
    CREATE TRIGGER places_say_whose BEFORE INSERT ON places
    WHEN NEW.project IS NOT
         coalesce((SELECT project FROM alive WHERE instance = NEW.instance),
                  (SELECT project FROM instances WHERE id = NEW.instance), '')
    BEGIN
        SELECT RAISE(ABORT, 'a place says whose it is as it is first kept: the project of its instance, and the empty text for nobody''s');
    END;
    CREATE TRIGGER instances_project_stays BEFORE UPDATE OF project ON instances
    WHEN NEW.project IS NOT OLD.project
    BEGIN
        SELECT RAISE(ABORT, 'an instance is of the project it was applied for, and of no other afterwards');
    END;
    CREATE TRIGGER runs_project_stays BEFORE UPDATE OF instance, project ON runs
    WHEN NEW.instance IS NOT OLD.instance
      OR (NEW.project IS NOT OLD.project AND (OLD.project IS NOT NULL OR OLD.status = 'running'))
    BEGIN
        SELECT RAISE(ABORT, 'a run is of the instance it was made for, and of its project, and of no other afterwards');
    END;
    CREATE TRIGGER alive_stays BEFORE UPDATE ON alive
    BEGIN
        SELECT RAISE(ABORT, 'what is alive is kept from its instance''s own row, and is not written to');
    END;
    CREATE TRIGGER under_way_stays BEFORE UPDATE ON under_way
    BEGIN
        SELECT RAISE(ABORT, 'what is under way is kept from its run''s own row, and is not written to');
    END;
    CREATE TRIGGER places_project_stays BEFORE UPDATE OF project ON places
    WHEN NEW.project IS NOT OLD.project
    BEGIN
        SELECT RAISE(ABORT, 'a place is of the project its instance is of, and of no other afterwards');
    END;
",
    "
    -- What is kept beside the rows (which texts a tree reached, what was
    -- vouched for an instance, what a handler last read) is kept from
    -- here on in the journal the rows are in (`crate::beside`). A
    -- journal that has something alive as this is added may have what
    -- is kept for it in the factory's journal still, where a program
    -- from before kept all of it: it notes that, for the conductor that
    -- first starts on it to bring its own over once. One with nothing
    -- alive has nothing to bring, and notes nothing; a journal made now
    -- is one of those. The note is all this step is: it reads one small
    -- row, whatever the journal has ever held. And a program from before
    -- must not run on what this one keeps: finding nothing vouched for
    -- where it looks, it would take a project's files as they stand for
    -- what was vouched for, which is somebody's authority given away.
    INSERT INTO kv (namespace, key, value)
        SELECT 'beside', 'adopt', 'true' WHERE EXISTS (SELECT 1 FROM alive);
",
    "
    -- Which of its run's steps a step was to begin, counted from one.
    -- When a step began is kept in milliseconds, and several steps of a
    -- run begin in one: read by when they began, those came in the
    -- order of their names. Said in the write that first keeps the
    -- step's row, as one above the highest its run then has, and never
    -- another afterwards (`runs::put`). A step a program from before
    -- wrote says nothing, and is read as it was, before those that say.
    -- A column added to the table, which rewrites nothing and reads no
    -- row of it.
    ALTER TABLE steps ADD COLUMN nth INTEGER;
",
    "
    -- Which of its versions the text an instance runs is kept in, where
    -- it is kept there and not in the instance's own row: the row is
    -- written again whenever the instance's state or status changes,
    -- and a text is thousands of bytes, so each of those writes was the
    -- text's pages written again. Every text an instance takes is one of
    -- its versions already. Nothing says it where the text is in the
    -- row, as in every row a program from before wrote, and in one that
    -- went back to what it ran before an update (`store::end_update`).
    -- A column added to the table, which rewrites nothing and reads no
    -- row of it.
    ALTER TABLE instances ADD COLUMN text INTEGER;
",
    "
    -- Where the journal a run says things in had got to as the run was
    -- written: everything the run says is after it. A run's events are
    -- found from there when it is taken up after a restart, along the
    -- index of their topics or in order, where a journal made now keeps
    -- no index of the events by run or by instance (`journal::events`).
    -- Nothing in a row a program from before wrote, which is looked for
    -- from the beginning. A column added to the table, which rewrites
    -- nothing and reads no row of it.
    ALTER TABLE runs ADD COLUMN place INTEGER;
",
    "
    -- Where a step began, and when, and whether a run began: what was
    -- read back from what the journal kept of their saying so
    -- (`step.started`, `run.started`), which it keeps now only where a
    -- text that is running could hear it, and otherwise passes on. A
    -- step's is the number its last try's beginning was said with, and
    -- the time of that write: a step done again after a restart looks
    -- for what it said after there. A run's is said where it begins and
    -- its beginning is not kept. Nothing in a row a program from before
    -- wrote, whose beginnings were kept and are read where they are.
    -- Columns added to the tables, which rewrite nothing and read no
    -- row of them.
    ALTER TABLE steps ADD COLUMN began INTEGER;
    ALTER TABLE steps ADD COLUMN began_at INTEGER;
    ALTER TABLE runs ADD COLUMN begun INTEGER;
",
    "
    -- The environment of its project's an instance was applied under,
    -- the empty text for none, which every row a program from before
    -- wrote is: written as the instance is made, and refused another
    -- value afterwards, as its project is. A column added to the table
    -- with a default that is the same for every row, which rewrites
    -- nothing and reads no row of it.
    ALTER TABLE instances ADD COLUMN environment TEXT NOT NULL DEFAULT '';
    CREATE TRIGGER instances_environment_stays BEFORE UPDATE OF environment ON instances
    WHEN NEW.environment IS NOT OLD.environment
    BEGIN
        SELECT RAISE(ABORT, 'an instance runs under the environment it was applied under, and under no other afterwards');
    END;
",
    "
    -- Which step asked for the text a run carries its instance over to
    -- (`on_update`): its run and its name, for a step that spawned
    -- what was there. Kept with the run however it ends, so that the
    -- step, done again after a restart, finds that its asking was
    -- answered and does not ask again. Nothing in a row a program from
    -- before wrote, and in one somebody asked for. A column added to
    -- the table, which rewrites nothing and reads no row of it.
    ALTER TABLE runs ADD COLUMN asked_by TEXT;
",
];

/// What a journal from before still has that nothing reads any more:
/// the index of every instance's status and of every run's, each
/// written again whenever a status changes, and the places of handlers
/// as they were kept before a place said which journal it is in.
const LEFT: &[(&str, &str)] =
    &[("INDEX", "instances_status"), ("INDEX", "runs_status"), ("TABLE", "trigger_cursors")];

/// How many instances, and how many runs, a journal may ever have kept
/// for what is left in it from before to be taken away as it is opened.
///
/// Taking an index away reads all of it, so it is done only where that
/// is little: a hundred pages or so at this many, which is a moment
/// from any disk. A journal that has kept more keeps them, and goes on
/// writing each index with its table (a page more for a run that begins
/// and for one that ends, and the same for an instance), until its
/// projects are moved to journals of their own, which are made as one
/// is made now. One made now is empty when it comes here.
pub const SMALL: i64 = 20_000;

/// Takes away what is left from before ([`LEFT`]) where the journal is
/// small enough for that to be nothing ([`SMALL`]). Asked each time the
/// tables are opened; it reads the names of what there is and, where
/// something is left, where two tables end.
pub(crate) fn shed(connection: &rusqlite::Connection) -> rusqlite::Result<()> {
    let mut left = Vec::new();
    for (kind, name) in LEFT {
        let there: bool = connection.query_row(
            "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE name = ?1)",
            [name],
            |row| row.get(0),
        )?;
        if there {
            left.push(format!("DROP {kind} {name}"));
        }
    }
    if left.is_empty() {
        return Ok(());
    }
    let most: i64 = connection.query_row(
        "SELECT max((SELECT coalesce(max(rowid), 0) FROM instances), (SELECT coalesce(max(rowid), 0) FROM runs))",
        [],
        |row| row.get(0),
    )?;
    if most <= SMALL {
        for drop in left {
            connection.execute(&drop, [])?;
        }
    }
    Ok(())
}
