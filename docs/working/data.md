# Tables, records and migrations

This file covers how a project's tables and records are kept: how the
tables of every database are made and migrated, how a record is found,
numbered and changed, what a field may require of whoever writes it, how a
large table is read a window at a time, and how deleting goes through the
Trash. Read it before you add a table, a migration step, a field option,
a new way of writing a record, or anything that deletes.

## Migration steps only grow at the end

- Each crate says how its tables are made as a list of steps
  (`Db::migrate`, and `Journal::makes` for a project's own journal file).
  A database that has run a list's first N steps runs only the steps after
  those, by count. So a step that exists stays where it is, word for word,
  and a new one is added after the last, in every list: the journal's own,
  the runner's (`crates/runs/src/schema.rs`), the studio's (`tables.rs` and
  each file's `SCHEMA`), the agents', the hosting's, the search index's and
  a store's.
- Why it matters: a step written in before the last is never run by a
  database that was already past its place; that database runs the last
  step a second time instead. Where the last step is re-runnable
  (`CREATE INDEX IF NOT EXISTS`) nothing fails and the new step is silently
  never run. Tests do not catch this on their own, because a test always
  makes its databases new, and a new database runs any order of steps.
- A step is the exact text handed to the database. A comment in it, or how
  it is laid out, is part of it, so neither is tidied in a step that exists.
- It is checked three ways:
  - **A kept file**, `contract/steps.txt`, records what each step of each
    list results in. A test of the API suite (`crates/api/tests/suite/steps.rs`)
    reads the lists from the databases a node makes (a project's file and a
    store's among them) and fails on a list that differs, saying which step
    and what to do. It also fails on tables made under a name none of those
    databases has, and on a place in the source that calls `.migrate(` or
    `.makes(` without writing the list's name there. Tables made only when
    asked are made by that test (`made` in `steps.rs`).
    `WRITE_STEPS=1 cargo test -p api the_steps` writes a step added last, or
    a new list; commit the file with the step.
  - **The database** (`journal::steps`) keeps what each step it ran results
    in beside the count (`schema_versions.steps`), and refuses a list whose
    first steps differ, before running anything, in a sentence naming the
    tables, the step and what to run instead. A list whose every step was
    run is no write.
  - **A play** of the fault itself, with a last step of each kind (the
    journal suite's `steps.rs`).
- A step changed in place and recorded as meant is written with
  `BREAK_STEPS="why" cargo test -p api the_steps`, which keeps the reason in
  `contract/steps-changed.txt`; that file is built into the program so a
  database that ran the old text is not refused. Never change a step and its
  line by hand. Only a step changed where it stood can be meant: a step
  removed leaves every database that ran it "newer than this build", and
  nothing can fix that.
- A database written by an earlier version keeps the count alone. The first
  time this version opens it, it records the step results it is taken to
  have run (a single cheap read of its own schema) and has nothing to check
  until then.
- A test that makes a database as an earlier version left it sets the count
  back by hand (`UPDATE schema_versions SET version`); the recorded step
  results after it are then not checked.
- `crates/runs/tests/fixtures/before.sql` is the runner's tables as the
  previous version made them, and is never changed: a journal from before is
  made from it and taken up (`before.rs`). Two tests of the conductor's suite
  play a journal from further back by undoing the steps since (`owed.rs`,
  `authority.rs`): a new step's undoing is added to what they undo
  (`BEFORE_ROWS_WERE_APART` in the suite's `main.rs` undoes the last).
- What a new step of the runner's does to a large journal from an earlier
  version is timed (from disk and in memory) and stated in the release notes.

## Finding, numbering and changing a record

- One record is found by its id without reading the rest
  (`Collections::one`). Which table it belongs to is asked of the tables of
  each project the id may begin with (`Collection::owns`, in
  `Collections::record`), never read from where a dash is: a project's name
  and a table's may each contain dashes. Checked by `records.rs` in the
  studio's suite and the API's.
- A new record is numbered under the collection's lock (`Collections::add`),
  never by reading the last one: two records added together would get one
  number.
- A new record is placed after the last by one row of an index
  (`Collections::last_position`, over `records_placed`; `placed_at` says
  whether a place is taken, which a row restored from the Trash calls). The
  studio suite's `records.rs` counts the database's own steps for an add
  beside three rows and three hundred, and holds them equal.
- Anything new that needs one record asks for one; it never reads a table or
  a board whole to find it.
- A record is changed by reading it, working out the new record and writing
  the whole of it back. Whoever does that holds the record's lock
  (`Collections::changing`) from before the read until after the write:
  without it, two changes side by side each write what they read and the
  later undoes the earlier. Anything new that changes a record takes the lock.
- A record is checked whole every time it is written, so
  `Collections::records` returns only what the collection now declares, and
  `Collections::refit` refuses a re-declaration its records would not fit.
  Anything that reads `records.data` another way must leave out what is no
  longer a field.
- `rows.read`'s `count` is how many rows the table has, whatever `saying`
  asked. Whether any row matched is `steps.<id>.out.those[0] == null`.

## Fields that name records

- A field names one record (`record`) or several of one collection
  (`records`). `Field::named` returns the ids either holds; anything that
  follows what a record names calls it, never `as_str` on the value. In the
  client it is `names` and `idsOf` (`ui/src/table.ts`).
- A `record` field is checked when it is written and never when it is read:
  what it names may be gone. Anything that shows or follows one must accept
  an id that names nothing.
- Naming a record requires being able to read it (`Collections::may_name`,
  called before `pointed` wherever a record is added or changed), because
  whatever reads the row follows what it names, and being told whether an id
  is a record of a table is reading that table. It is asked only of the cells
  a write gives or changes, never of what the row already named, and it
  answers in the same words whether or not the record exists. A new path that
  writes a record's fields calls it beside `may_set` (checked by the API
  suite's `scoped.rs` with a key with fewer scopes).
- What a row opens into is the rows of the tables that name it (`namedBy` in
  `ui/src/table.ts`, `views/within.tsx`); no table says what is inside
  another. Something new that nests records is a field that names, not a
  second mechanism.

## What a field requires of whoever writes it

- A field may say what giving it a value takes (`takes`, `fields::may_set`),
  or what giving it one of its values takes (`takes: { on: shell }`,
  `Field::takes_of`; choices and flags only). Either is one scope or a list,
  every one of which is required (`takes: [shell, secrets:use]`). It is kept
  as written (`studio::Takes`), and a refusal names every scope required and
  which are missing (`Takes::refused`). A value that can be two things (a
  param that may be a command or a secret's name) requires both.
- It is checked wherever a record is added or changed, for each field with
  its new value. A new path that writes a record's fields calls `may_set`.
- Re-declaring a table so a field requires less takes what was required,
  each scope separately (`fields::still_guarded`).
- `takes: each` (`Field::takes_each`; `holds` of a fleet's `agents`) marks
  a list of scopes that something with more authority then gives to someone
  else. Whoever adds the record, or changes any cell of it, must hold every
  scope the list then says (`kernel::principal::covers`), checked on the
  whole record as it is to be written (so `may_set` is handed the record),
  because other cells (instructions, count, program) change what an agent
  with those scopes does. This matches `Agents::ensure`: an agent never has
  more than whoever changes it. A row made to say no more than its writer
  holds, in the same write, is theirs. What an empty list stands for is the
  table's `unsaid` (`AGENT_SCOPES` for `std/fleet`, checked by a test).
  Removing the word on re-declaration, or narrowing `unsaid`, takes `admin`.
  Something new a row gives that is a list of permissions uses the same
  word, never a Rust check on one table's name (`scoped.rs`).
- `steers` (of a collection, `Collections::steered`; `stages` steers
  `agents`) marks a table whose rows say what is done with what another
  table's rows give. A row of it is added, or changed in any cell, only by
  someone who could write every row of the steered table as it stands
  (`fields::may_set` of each). It is checked at every way a record is written
  (`Collections::add`, `Records::update`, a card's change and move,
  `Board::moved` under the card's lock); removing a row is not. Re-declaring
  so it steers nothing takes `admin`. Checked by `scoped.rs`, the studio
  suite's `steered.rs`, and `snapshots.rs` for copies. A row of a table
  that steers, or is steered (`Collections::steering`), written through
  the record routes, is answered once what hears it has taken it
  (`api::taken`; `fleet.md`).
- A cell that lets an agent's program do more (`permission: full`) says
  what giving that value takes (`takes: { full: shell }`), or the row is a
  way around the agent's own route.
- `stands` (`Field::stands`, beside `takes`; `crates/studio/src/stood.rs`)
  says that what giving a cell its value took is asked again of whoever
  adds the record or changes any cell of it, while it says that value
  (for a `takes` of any value, while it says anything). It is checked in
  `fields::may_set` against the record as it is to be written, so a write
  that also sets the cell to a value that takes nothing is anybody's who
  may write the row. Removing a record is not checked. A field that takes
  nothing cannot stand; with `takes: each` it adds nothing. Declaring a
  field again so that it does not stand takes holding each scope it took
  (`stood::still_standing`). Something new whose cell lets more be done
  with the rest of its row says `stands`, never a Rust check on a table's
  name.
- A cell whose value becomes a param of a blueprint run as whoever applied
  the fleet is guarded at least as the param's own cell would be: `with` of
  `segments` takes `shell` and `secrets:use`, and never sets what a row's own
  cell or its stage sets (`fitted` in `deciding::segments`). An older row is
  checked the next time it is written and refused when its card comes
  (`crates/api/tests/suite/steering.rs`). A cell that only names the
  blueprint a stage runs (`does`, `review`, `proposal`) takes what the row
  takes: a project's own blueprint named there runs only as it was vouched
  for (`texts::vouched`).
- `writer_of: <field>` (`fields::stamp`) makes the factory record who last
  wrote another field, as an actor (a key as `key:<id>`, never its maker), in
  every write that sets or changes that field: at `Collections::add_numbered`,
  `Records::checked` and `Board::change`. Nobody writes it: a value given on
  add is overwritten, and a change of it alone is refused. It lets whatever
  acts on the other field with more authority check the writer. Such a field
  is `quiet` and `hidden`. Something new that writes a record's cells goes
  through one of those doors.

## Declaring tables again

- A table may hold a million rows and is declared again at every apply of
  its blueprint, so nothing done at declaration reads every record.
  `Collections::records` reads a table whole and is only for handing every
  record out. A declaration queries rows only for a field that is changing,
  one statement each (`studio::held`: `still_chosen`, `refit`, `numbered`
  for what `reworded` writes). While a rewording is under way it is marked on
  the field in the table's row (`Field::rewording`), so a declaration or a
  start finishes what a stop left.
- The row count is kept by the daemon (`Collections::count`,
  `studio::counted`), updated in the same transaction by every write that
  adds or removes a row of `records`. Something new that adds or removes such
  rows does the same. The studio suite's `counted.rs` counts database steps
  of a declaration beside three rows and a hundred thousand.
- A shipped table may be re-declared over rows written by an earlier version:
  what it now says of a field must accept them. A choice that becomes free
  text keeps what was chosen; an option removed from a field that stays a
  choice is refused while a row holds it. Something new said of a fleet's
  table is tried over the table as it was (`staged.rs`).
- A collection that is a model's has the model's fields whatever its
  blueprint wrote: one left out is added when declared and again when read.
- Views of a table are shown in name order, whatever order the blueprint
  wrote them in.

## Reading a table a window at a time

- A table is read a window at a time by the daemon (`GET …/rows`,
  `…/rows/counted`). `kernel::window` decides (order and stretches, what a
  view shows, where a reading got to, `plan`, the figures checked against
  `ui/src/figurecases.json`, and `window::statements`); `studio::window` runs
  those statements.
- Every statement names the project and table as literals and the index it
  reads by (`INDEXED BY`), so cost never depends on the planner's guess. A
  walk is a range on an index expression (`> -9e999` for "has a value",
  never `IS NOT NULL`, which walks through every empty row), never a row
  value.
- A column's index (`studio::indexes`, `records_by
  <project>/<table>/<field>`) has project and table as its first columns.
  It is made on demand (when a table of `SMALL` rows or more is first sorted
  or filtered by the column, or when a field declared `indexed` is
  declared), emits `collection.indexed` and is measured; never made by a
  migration step. The database's own list of indexes is the only record of
  them.
- Something new that writes a table's rows other than through `whole`, the
  Trash and `reshape` moves its write mark (`Counted::writing`), or stale
  view counts are kept.
- Something new that reads a table for a page reads a window, never
  `Collections::records`. `windowed.rs` (API suite) checks that each way of
  reading costs the same steps beside a hundred rows and a hundred thousand
  (`ROWS=1000000` by hand); the studio suite checks every statement against
  its plan and every window against the in-memory rules.
- Search (`find`) is a check per row beside the view's filter
  (`kernel::window::found`, one rule both in memory and as SQL, part of a
  reading's mark). Going to a row by id or number (`around`) counts its rank
  off the index where no row check applies (`Shape::before`,
  `sorted_before`) and reads backward and forward from it (`From::Of`); a row
  the view hides says why (`kept_out`). Something new a view can filter by is
  a case of both `found::shows_sql` and `found::shown`, and goes into the
  mark.

## Deleting goes through the Trash

- Something new that deletes goes through the Trash. What is deleted is
  kept, left out of everything that reads what it belonged to, can be
  restored where it was, and is gone for good after the configured number of
  days (`trash.kept`: thirty by default, zero for at once, or forever;
  `kernel::trash` for the plain values).
- The writes are `studio::trash`: a record leaves its table and enters the
  Trash in the one write that emits both (`Collections::trash_saying`, called
  by `Records::remove` and `Board::remove`); a file or folder moves from its
  project's folder into `trash/<id>/` of the data folder (`Projects::delete`,
  `trash_path`: where it came from, then the move, then the row, each found
  and finished by a start); a project's row in the write that forgets it
  (`put_project`).
- Reading the Trash, restoring, deleting for good and emptying is the
  `trash` crate (`trash::Trash`, called by `api::trash` and `mw trash`).
  Whoever could read a thing now sees it; whoever could have deleted it
  restores or purges it.
- A Trash row is kept where its project's records are (`KEPT_APART`), so a
  project moved apart takes its own, and a copy of the factory takes every
  row, the folder and the setting.
- Anything that deletes on someone's behalf (a route, an action, the client)
  goes through those doors, never `remove_saying` or `Projects::remove`,
  which delete at once and are for internal deletes (a file a blueprint keeps
  a record in, a work place, a copy's tidying).
- A new kind of deletable thing is a `Kind` with what restores it to its
  row, a case of who sees it and who could have deleted it, and its events
  emitted in the write that moves it. The client says "Moved to Trash" with
  Undo where it deletes, never "Delete forever".
- A channel is trashed through `trash::Trash::delete_channel`. A table cannot
  be deleted this way: only a blueprint declares one.
