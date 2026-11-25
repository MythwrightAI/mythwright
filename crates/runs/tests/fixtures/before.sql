-- The runner's tables as the program before this one made them: its
-- twelve steps, in order, as they stood when the tables were the
-- conductor's own (`crates/conductor/src/store.rs`). Nothing here is
-- changed when the tables are: it is what a journal from before holds.

-- Step 1.
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

-- Step 2.
-- The definition an instance had before the update now in progress, as
-- JSON, so a failed update can put it back (even across a restart).
ALTER TABLE instances ADD COLUMN previous TEXT;

-- Step 3.
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

-- Step 4.
-- What everything measured under an instance also carries, as JSON: the
-- labels its parent's step gave it, added to its parent's own.
ALTER TABLE instances ADD COLUMN labels TEXT NOT NULL DEFAULT '{}';

-- Step 5.
-- On whose authority each version ran, as JSON: who applied it and the
-- scopes they held then. No token is in it. Until now an instance kept
-- the authority of whoever applied it first, whoever applied over it,
-- so that is what every version already here ran with.
ALTER TABLE instance_versions ADD COLUMN principal TEXT;
UPDATE instance_versions
SET principal = (SELECT principal FROM instances WHERE instances.id = instance_versions.instance);

-- Step 6.
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

-- Step 7.
-- The last thing the journal had kept when an element began: what was
-- said for that element is after it, which is where a step done again
-- for it after a restart looks for what it had said already. A column
-- added to a table that holds only steps under way.
ALTER TABLE step_elements ADD COLUMN began INTEGER;

-- Step 8.
-- How far what is done around a step, and as a run ends, has come:
-- which entries were done, and how the step's action ended, so that
-- one taken up among them goes on from the next. Each is a column of
-- a row that is written anyway, and is read only when something is
-- taken up after a restart.
ALTER TABLE steps ADD COLUMN hooks TEXT;
ALTER TABLE runs ADD COLUMN hooks TEXT;

-- Step 9.
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

-- Step 10.
-- How far what an instance does as it stops has come (`on_stop`),
-- and why it was stopped: a stop is done in no run, so it is kept in
-- the instance's own row, with a note that the stop is owed (`owes`)
-- from before the first of it until the instance is written as
-- stopped. A column of a row that is written anyway.
ALTER TABLE instances ADD COLUMN hooks TEXT;

-- Step 11.
-- What an instance is to be carried over to, kept with the run that
-- will do it from the write that makes the run until the text is
-- taken or the run ends: whoever asked was answered, and a conductor
-- stopped in between carries it over when one starts (`carry`). A
-- column added to the table, which rewrites nothing.
ALTER TABLE runs ADD COLUMN carries TEXT;

-- Step 12.
-- How the text an instance runs reads a handler or a wait that says
-- no `scope`: `project` in one taken by a program that has the word,
-- and nothing in one from before, which is read as it was meant then,
-- the whole factory (`whose`). A column added to the table, which
-- rewrites nothing; a row from before is as it was until its
-- instance is applied again.
ALTER TABLE instances ADD COLUMN heard TEXT;
