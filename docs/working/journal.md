# The journal and its databases

This file covers the journal: the SQLite databases a factory keeps, events
and their numbers, places in the event stream, the separate journal each
project may have, what is kept where by project, moving a project to its own
journal, copies of a factory, the database thread (transactions, gathered
commits, checkpoints, the disk), measurements, how long run logs are kept,
and the lock files of the data folder. Read it before you add a table, emit
a new event, follow events for the whole factory, write a row with its
event, or change how the journal commits.

Terms: a *write* is one transaction on a journal's database. A *play* is a
scripted run in the conductor's suite or the simulation that is stopped at
each point and restarted, then compared with a run never stopped.

## Several journals

- A factory has its own journal and one for each project that has a journal
  of its own, each a database file with its own thread (`journal::set`).
- Which projects have one is a row of the `journals` table in the factory's
  database and nothing else; `Journal::own` writes it when a project is
  added. A project with no row (any project of a factory an earlier version
  wrote) is kept in the factory's journal. Never infer a project's journal
  from its id or from a file existing, and never give an existing project a
  journal because the daemon started.
- How much a project's journal keeps is a level on its row, compared as "at
  least" (`journal::keeps`, `Journal::rows_of`): `keeps::RUNS` (runs and
  events), `keeps::RECORDS` (tables and records), `keeps::TURNS` (agents'
  turns). A journal made now keeps all three.
- `Journal::of(project)` keeps the project's events,
  `Journal::records_of(project)` its records, `every()` is all journals and
  `dbs()` their databases in closing order. A journal with no folder for
  others (`Journal::memory`, unless told `keeps_projects_in_memory`) returns
  itself for every project.
- Shared once for all journals: the number giver, the event checks, the place
  they shut at, and the measurements store (the factory's; `measure` on a
  project's journal is stored there).
- Anything done to "the journal" for the whole factory (pruning, closing, a
  copy, `repack`) is done to each journal.
- The first start of a new version on an earlier version's journal is one
  way: the earlier version refuses tables newer than it knows. Anything that
  says an upgrade can be rolled back says it needs a copy of the data folder
  made beforehand. A test of an earlier-version factory removes the
  `journals` row and file while stopped (`crates/api/tests/suite/journals.rs`).

## Where an event is kept

- An event is kept in the journal that keeps the row it is about, decided in
  one place: `said_in` in `crates/journal/src/events.rs`, which every way of
  emitting calls (`Journal::emit`, `Writing::say` in `Journal::write`,
  `emit_ephemeral`).
- `Journal::emit` on the factory's journal stores an event naming a project
  (`project` in its data) in that project's journal. That keeps one order for
  everything about a project (a card's move, then its round's events, then
  its history line). So an event about a project includes `project`; one
  without it goes to the factory's journal with no order relative to the
  project's.
- An event about a project's thing that cannot carry `project` (what a
  service or router emits through its owning instance, `Hosting::said_of`)
  is emitted in `Journal::said_of(project)`. Nothing else picks a journal.
- Order is promised only within one journal (`docs/architecture.md`, *A
  journal for each project*).
- What belongs to the factory for good stays in its journal whatever project
  it names: channels and messages (`Writing::say_here` for `channel.message`
  with its row, `Journal::emit_here` for other `channel.*`) and which
  workspace a project is in (`project.placed`). Their topic prefixes are the
  list `journal::HERE`: `said_in` checks it, `say_here`/`emit_here` refuse a
  topic not on it in test builds, and moving a project apart leaves those
  events in place (`journal::moved::said`).
- A test that plants a rule to make a project's event fail plants it in
  `journal.of(Some(project))`; one that reads a project's runner rows reads
  `journal.dbs()`, not the factory's database.

## A row and its event are one write

- What is written and what is emitted about it are one transaction
  (`Journal::write`, `journal::together`): events are emitted inside the
  write, numbered there, heard by followers once it commits. After any stop
  both exist or neither. Never write and then call `emit`.
- The runner does this for every run, step and instance it writes as begun,
  ended, settled, stopped or carried over (`runs::together`, `to_say`).
- Records go through `studio::whole` (`Say`, `Collections::add_saying`,
  `put_saying`, `remove_saying`); a history line is written with its event
  and the history keeper's place (`Collections::note`).
- Turns go through `crates/agents/src/together.rs` (`queue`, `begin`, `end`,
  `call_off_unbegun`), checked at every point by the agents' suite
  (`together.rs`, with `Journal::shut_after_saying`).
- The exception: an agent's answer posted in a channel is kept first with
  the turn still in hand (`answered`, `Agents::answered`), then emitted
  (`say_answer`), then the turn ends (`conclude`). Whatever asks for turns in
  hand includes `answered`; a daemon that finds one finishes it
  (`end_answered`) without asking the agent again. The answer is found again
  by its words, author and what it answers, so any other way of posting an
  answer keeps that true.
- Where the row and event cannot share a file (a project journal at
  `keeps::RUNS`), the row goes in the factory's file and the event in the
  project's after the commit, with a check at start behind it
  (`Board::settle`; a turn found in hand). Writers never decide this: they
  emit through the write they are handed.
- Tables kept as files, by a store service, or as documents and mockups are
  written first and emitted after.
- A test needing a row written and never emitted plants it as an earlier
  version's journal would hold it (`moved_and_not_said`,
  `added_and_not_said` in the API suite's `late.rs`). One-write tests first
  check the card and its events share a journal (`one_write` in the API
  suite's `main.rs`); the journal suite checks every level (`kept.rs`).

## Event numbers

- The journal numbers events itself (`journal::numbered`). An events table
  made now is not numbered by the database; an earlier version's keeps its
  table and page size until `mw repack` (`journal::rewrite`). Never rewrite
  that table or change the page size in a migration or on open.
- A number is drawn, not last-plus-one. One giver serves the factory
  (`journal::giver`); each journal draws a run at a time (`giver::RUN`).
  Promised: unique, and greater for a later event of the same journal. There
  is a gap at every start.
- Never add to or subtract from an event number to find another event, wait
  for "the next number", or count events by subtraction. `seq - 1` is a place
  just before an event and nothing more.
- To count the newest N events (pruning), the journal records each gap with
  the first event after it (`numbered::keep_gap`) and counts over them
  (`numbered::floor`). A journal with events removed from its middle records
  that it has holes (`numbered::HOLES`) and counts from its rows.
- The giver's mark (`giver::DRAWN` in the `kv` of the journal that keeps it)
  is stored before any number at or below it is used: raised in the
  transaction that writes a run's first event (`Giver::draw_here`). A journal
  drawing from a giver in another database (`Journal::open_drawing`) gets a
  run only after that database has the mark on disk
  (`Db::call_to_the_disk`), and requests the next when half is used without
  waiting (`Giver::draw_ahead`; it waits only for its first,
  `Giver::draw_apart`). Events after shutting draw even though the giver's
  database is held (`though_shut`).
- A run goes in hand only through `Inner::put_back`, after events are
  written. A failed write returns what it drew (`Inner::undraw`); a failed
  commit restores the previous run and returns the drawn one
  (`Inner::put_back`); never both. A journal joining with numbers of its own
  says so (`Giver::above`), and a run in hand that reaches no higher is
  dropped where it is read (`Inner::run`).
- The mark only rises (`raise`) and a giver starts above everything its
  journal held (`numbered::last`), so opening an earlier version's journal
  writes nothing. The mark is in `kv` under the journal's name, kept by a copy
  (`copy::keep_state`); anything new the giver keeps goes there.
- Anything that removes events records the highest number given with it
  (`numbered::keep_said`).
- Tests state "the next number" in terms of `RUN` (the journal suite's
  `given.rs`, `drawn.rs` for two files).
- An event's time is indexed for one event in 512 (`events_when`); pruning
  uses it. Anything querying by time finds a place there and continues by
  number. Do not index every event's time.
- A migration's effect on a large earlier-version journal is timed first
  (`journal::REINDEXED`) and emitted once by the daemon that ran it
  (`system.reindexed`); more than a second or two waits for `repack`.

## Places and following events

- Whatever follows or reads events for the whole factory uses every journal
  with a place in each (`journal::every`): `Journal::follow_every` from a
  `Places`, `read_every`, `read_within`, `read_after`, `said_in_any`,
  `forget_in_every`, and `now()`.
- `follow`, `read`, `last_seq`, `said` and `forget_events` are one journal's;
  on the factory's journal they miss projects' events. Outside the runner
  nothing calls them on `app.journal`; do not add such a call.
- A stored place is `Places::kept()`, read with `from_kept` (which accepts a
  bare number from an earlier version); a place handed outside is
  `cursor::at` (`crates/api/src/cursor.rs`).
- A `Places` becomes one journal's number only through `Journal::place_in`
  (over `journal::joined`; handlers use `Journal::joins`), never
  `Places::of`, or a journal that had events moved into it is re-read.
- Never order two journals' events by number or take `highest()` for a
  place; what people see is ordered by time. An event's place in a stream is
  `FollowEvery::after(event)`.

## The runner across journals

- The runner names which journal it means: its own (`Engine::own`,
  `Inner.own`: where it keeps and emits about what it runs) or the factory's
  (`Inner.factory`; `Engine::journal` for other crates: shared `kv`, what
  belongs to nobody, whole-factory calls like `now`, `keeping`,
  `follow_every`). Choose by purpose: clock, ids and measurements are its
  own; a `kv` key is its own for an instance or project and the factory's
  for one factory-wide thing; a run's events and step places are its own.
- Whose an event is, is decided with the journal it was heard in
  (`whose_heard`).
- A project whose journal keeps runs has its own conductor
  (`crates/conductor/src/set.rs`), with rows in its file and its own tasks,
  waiters and instance lock. The factory's conductor runs everything else and
  is the front other crates call.
- A call is answered by the conductor that keeps the thing: an apply by
  project (`Engine::of`), an instance or run by own rows then elsewhere
  (`Engine::elsewhere`, `elsewhere_run`), lists from all
  (`conductor::routed`). Inside the runner, `self.inner.store` and
  `self.live(..)` mean this conductor's own; code handed an outside id calls
  a routing public method or `elsewhere`. A new public `Engine` method that
  names an instance, run or project routes the same way, and one that lists
  goes over `every()`; test it in the API suite's `apart.rs`.
- Factory-wide shared state is behind an `Arc` handed to each conductor
  (`Engine::beside`): hooks, the remote, project locations, the environment,
  text taking, what is owed, the comparisons answerer. Something new that
  belongs to the factory is shared there. Only the factory's conductor tells
  comparisons a start is happening.
- Conductors hold one another through the set and are released by
  `Engine::halt` on the factory's conductor; dropping an engine releases
  nothing.
- A conductor is called once it has taken up what it keeps (`standing`).
  Reaching another conductor goes through a door that waits while it starts
  or was halted (`Engine::of`, `elsewhere`, `elsewhere_run`, `keeper`,
  `conductor::routed`); row-only reads by key (`whose`, `alive_everywhere`)
  never wait. `Engine::is_there` is awaited and answers from rows for a
  conductor that is not up.
- A start never waits on a conductor that starts after it. Conductors start
  in turn, the factory's first; a start's own work is marked
  (`Engine::within_a_start`) and answered at once. Nothing `resume_own`
  awaits may wait on a task that calls another conductor; stopping another
  conductor's instance is spawned as that conductor's task
  (`stop_for_another`). Plays fail by name where a start waits (`STALLED`,
  `alone.rs`).
- One conductor is stopped and started alone with `halt_one`, `halted_one`,
  `start_one`: its journal stays open, a new `Engine::beside` replaces it,
  waiters are handed on (`wait_run`). A halted conductor spawns nothing
  (`Engine::spawn`).
- An event an instance emits about another project goes to that project's
  journal (`Engine::kept_in`), where the run's places mean nothing, so the
  place that journal had reached is stored before emitting (`keep_place` in
  `intrinsic.rs`; `hooks::Around` for hook entries) and a step done again
  looks after it (`said_before`, `entry_said`). A rule watching a step stores
  its place wherever it emits, with which firing it was
  (`hooks::keep_watched`, never by writing the step's record), so it emits
  once (`rule_said`; `elsewhere.rs`).

## What a conductor keeps beside its rows

- A conductor's `kv` state is kept where its rows are (`runs::beside`,
  `crates/conductor/src/beside.rs`). Every key is a line of
  `runs::beside::KEPT` saying whose it is (comparisons' in
  `comparisons::copy::KEPT`): an instance's, a project's, the conductor's own
  (pinned texts by revision) or the factory's (text taking, profile
  variables, comparisons). The first three live in `self.inner.own`, the
  last in `self.inner.factory`. Tests fail for an unlisted key, a key in the
  wrong file, or instance data away from its rows (runner suite `beside.rs`,
  API suite `apart.rs`).
- A public method handed an instance asks its keeper first
  (`Engine::kept_by_another`); one that searches all trees goes over
  `every()` (`text_at`). Texts are looked up in the own journal, then the
  factory's (`text_of`). Outside the runner use `Engine::kept_beside` and
  `Engine::own_journals()`.
- Each conductor tidies its own keys on start; the factory's, starting
  first, removes only what nothing alive anywhere uses (`alive_everywhere`).
  An earlier version's journal may note that some of its keys are still in
  the factory's (`runs::beside::OWED`); the conductor brings them over once
  (`Engine::bring_over`) before taking anything up. Never tidy over less than
  may still be kept, and never move keys by an unnoted look.
- A copy drops all of it in every file (`conductor::copy::leave_out`).

## What is kept where, by project

- Every runner row states its project or is reached by key from one that
  does (`runs::apart`). A new runner table is a line of `runs::apart::TABLES`
  (`Told`) beside `runs::copy::TABLES`; `crates/runs/tests/suite/apart.rs`
  checks every row.
- A row copies its project in the write that makes it, from its instance's
  row (`apart::of_instance`), never from the caller. Runs and places use the
  empty text for nobody; a run with no project (earlier version) is reached
  through its instance.
- The database enforces it: an instance under another project's instance is
  refused (`Store::insert_instance`), and `instances.project`,
  `runs.instance`, `runs.project`, `places.project` must match the instance
  and never change (rules in `crates/runs/src/schema.rs`). A test writing a
  run or place by hand states whose it is.
- Every table of a crate above the runner has a line saying how rows are
  told apart (`journal::apart::Told`: factory-owned, a column, another
  table's row, or custom), in `studio::apart::TABLES`,
  `hosting::apart::TABLES`, beside `copy::TABLES`. A row states its project
  when made and never changes it.
- Channels, members, messages and attachments belong to the factory whatever
  project a channel names; none of a conversation goes in a project's file.
- A project's tables, records, record history and last file scan live in the
  project's file where its journal keeps `keeps::RECORDS`
  (`studio::apart::KEPT_APART`), else in the factory's. Nothing moves them on
  start.
- A table added to a project's data is a line of `KEPT_APART`, made in
  project files by registering it with the journal as projects open
  (`studio::apart::makes` over `Journal::makes`; never a `migrate` by
  whoever opens the file), and every access passes the project to
  `records_of`. `Collections` does this as `rows(project)`; cross-project
  queries use `every_records`; a history looked up by id alone asks each
  project the id may belong to (`Collections::may_keep`). The factory's
  handle is named `factory` so it is not used by habit.
- The API suite checks every row of every file after a card's journey
  (`rows_are_kept_where_their_lines_say` in `apart.rs`). Tests that plant
  rules or rows do so in the file that keeps the table (`records_of`;
  `Rig::kept_in` once stopped).
- The history keeper stores its place in each journal's own `kv`
  (`history/seq`) and continues from the further of the two. Anything new
  that follows every journal and writes in project files does the same.
- An earlier-version project that is removed and added again while the
  factory's file still holds its rows gets a journal that keeps runs only
  (`studio::tables::held_here`, `agents::apart::held_here`, consulted by
  `Journal::own`).
- What is alive and under way is kept by project in `alive` and
  `under_way`, maintained by database rules; nothing writes them directly.
  Ask them (`Store::alive_of`, `under_way_of`, `instances` with
  `live_only`, `unfinished_runs`). Anything new a project's conductor asks at
  start reads a table that groups a project's rows, with a line in
  `asking_for_one_projects_share_reads_no_row_of_anothers`.
- Never add an index over `instances`, `runs` or `steps`: creating or
  dropping one reads every row on every upgrade. Large earlier-version
  journals keep two old status indexes unused (`schema::shed`); never query
  them.

## Agents' turns across files

- Agent rows and sign-ins belong to the factory; a turn belongs to its
  project (`turns.project`, set when queued from the slot,
  `apart::of_slot`; empty for no project). A slot may change project, a turn
  never. New agent tables are lines of `agents::apart::TABLES` beside
  `agents::copy::TABLES` (`crates/agents/tests/suite/apart.rs`).
- A turn lives in its project's file where that journal keeps
  `keeps::TURNS`, else in the factory's, so turns span files and nothing
  calls `self.inner.journal.db()` for one. Use one of: `Agents::keeps(turn)`
  for a turn in hand, `may_keep(id)` by id, `may_hold(slot)` for an agent's
  turns, `every_turns()` for a conversation; all go through `of_each`
  (`crates/agents/src/filed.rs`).
- Project files have all agent tables, `slots` and `signins` empty
  (`agents::apart::makes`); a copy empties them
  (`agents::copy::leave_out_beside`).
- A project's turn may still sit in the factory's file; it is read there
  until the project is moved apart, which takes such rows whatever the
  journal keeps (`Kept::straying`). Anything that can leave a project's rows
  behind says `straying` on its line.
- Turns of different files are ordered by `turns.nth` (`filed::NTH`) alone,
  given inside the queueing write (`Filed::next` in `Agents::queue`); a start
  continues above each file's newest (`filed::read`). Earlier-version turns
  have none and order by `rowid`; moving one writes its number. It is not an
  event number.
- An agent moved to another project has turns in two files: the holder of a
  conversation is the highest-numbered newest turn per file; counts are
  summed over files, only for agents that could take the work
  (`could_take`); `has_turn_for` checks one turn per message across files.
- Choosing who gets work never reads every turn: given counts are in memory
  (`Inner.given`, `Agents::was_given`), in-hand counts use the slot/status
  index one agent at a time (`picking::IN_HAND`), as does finding turns in
  hand at start (`crates/agents/tests/suite/given.rs`). Anything writing a
  turn outside `Agents::queue` updates the counts.

## Moving a project apart

- An earlier-version project moves to its own journal only when asked
  (`mw apart`, `folders::moved` offered as `api::moved`; `--look` previews),
  on a stopped factory, never by a start or migration.
- The move knows no table (`journal::moved`). Crates hand it their tables
  with how a row says whose it is (`runs::moved::tables`,
  `studio::moved::tables`, `agents::moved::tables`) and say what blocks a
  move (`runs::moved::under_way`, `agents::moved::in_hand`; `answered`
  counts as in hand). Anything new that can be mid-way with no run or turn is
  added where blockers are gathered (`looked` in
  `crates/folders/src/moved.rs`).
- Conductor `kv` keys move in the same writes by their line
  (`runs::moved::beside`, `Kept::beside`); each line says which value wins if
  the target has the key (`Kept::yields`). A key whose owner is stated
  another way is a new `runs::beside::Whose`, never a second mover.
- Only tables the project's journal does not keep yet are moved
  (`tables(keeps)`), and `keeps::MADE_AT` is set in the deciding write. New
  tables are made by the daemon's own steps (`studio::apart::make`,
  `agents::apart::make`). Lines that find ownership through another moving
  table are deeper (`Kept::depth`); positional meaning is written out by the
  line (`Kept::writing`, `Kept::ordered`, `Kept::strays`). In-memory counts
  are re-read at start; the history keeper's place is translated
  (`place_in`).
- Events move as one more table where the project had none
  (`runs::moved::said`): numbers are kept, both journals record holes, the
  row records the giver's position. Anything that keeps places per journal is
  a line of `runs::moved::placed`.
- The project's file is written whole and flushed, then one transaction of
  the factory's database decides (`journal::moved::decide`: the `journals`
  row, removing exactly as many rows as were written or none,
  `system.moved_apart`). Nothing else removes those rows.
- A file in `journals/` named by no row is removed by whoever next holds the
  folder (`moved::clear`, `system.move_cleared`), so nothing else may keep
  one there, and a new project journal has no events until its row exists.
- Tests: the API suite's `moved.rs` and `whole.rs` (cuts via `moved::Stop`,
  stopped at rest after `run.finished`; `kept_less` empties a file that keeps
  less, since the move refuses a row in both files), and `moved.rs` of the
  runner, studio and agents suites.

## Copies of a factory

- A copy (`std/snapshot`, then `up --from`) takes only what is listed: a new
  table, `kv` namespace or data folder is left out until its crate's
  `copy.rs` lists it (`TABLES`, `studio::copy::KEPT`, folders in
  `crates/folders/src/snapshot.rs`). `crates/api/tests/suite/snapshots.rs`
  fails on an unlisted table.
- A copy is the factory's database plus each project journal (`JOURNALS`),
  each cleaned by its lists (`leave_out`, `leave_out_beside`); anything new
  in a project file is listed in the second or emptied.
  `snapshot::leave_out_beside` empties unlisted tables and says which. Which
  files exist is what the factory's database lists at the moment, not the
  folder.
- A copy is every file as of one moment (`journal::moment`,
  `Journal::moment`): each journal's thread is parked between pieces of work,
  a read begins on each file on its own connection, threads are released,
  and each file is written from its read (`Read::write`, SQLite's page copy;
  `VACUUM INTO` cannot run inside a read). So no copied event is numbered
  above the copied mark, no run was set off by a missing event, and no row
  lacks its event (`crates/api/tests/suite/moment.rs`, via
  `Config::copying`, never set for a real factory).
- Project threads park first and the factory's last, because a project
  thread waits on the factory's (for a run of numbers), never the reverse.
  Keep that direction, or revisit `moment_within` (the journal suite's
  `moment.rs`).
- Threads are awaited five seconds together; a parked thread releases itself
  after fifteen and the copy is refused. A held read blocks its file's
  checkpoint, so every file is written before anything is removed from any
  copy, and each read is released when its file is done.
- In a copy, turns in hand are ended, and nothing is in hand in a factory
  started from it.

## The database thread

- Each journal database has one thread. Each piece of work runs in a
  transaction begun with `BEGIN IMMEDIATE` (`Db::start`), so it takes the
  write lock at once.
- A piece of work never begins its own transaction; what it writes as one
  thing goes in a whole (`journal::whole`, opened by `Journal::write`). The
  thread refuses work that tries (`gather::refused`, via `db_error`). The API
  suite's `wholes.rs` scans every crate but the journal for transaction
  statements; exceptions on a copy's own connection are lines of
  `LET_THROUGH`. A bare `COMMIT` statement is caught only by that scan.
- What must run outside a transaction (`PRAGMA`s that set how the file is
  held, `VACUUM`) runs on its own connection. No deferred constraints:
  failing work is refused where it runs and undoes only itself.
- Work that holds the thread until released is asked apart
  (`Db::call_apart`), or earlier callers wait behind it.

### Gathered commits

- When the thread is behind (sixteen or more pieces waiting as a commit
  begins), it runs what is waiting in the open transaction, up to sixty-four
  pieces and two milliseconds, and commits once (`journal::gather`). A thread
  that keeps up commits each piece alone; nothing waits for company. Change
  the threshold only with a load that shows a gain (`db.shared_count`,
  `db.held`; `docs/architecture.md`, *The journal*).
- Rules for anything new in the journal: nobody is answered before the
  commit, and all are told if it fails; consequences of being kept (an event
  becoming last, followers hearing it, shutting at a place) are registered
  with `gather::then` and run after the commit; numbers come from the run in
  hand (`Inner::number`) and are put back only once events are written
  (`Inner::put_back`), so failures give no number away; whether an event is
  the shut place is checked once as it is written, so the commit ends with
  that piece (`gather::cut`).
- `JOURNAL_GATHERS=all` shares a commit among whatever waits; `=1` commits
  each piece alone (read once, `gather::said`; never set for a daemon). A
  journal-suite test needing a shared commit checks `gathers` first. Forced
  gathering changes commit counts, so `costed.rs` checks writes and steps
  (`journal::gathering_forced`); tight step bounds subtract eight steps per
  transaction (`Db::begun`, as in `timed.rs`).
- No test may assume one commit took everything it asked for: the limit is
  wall-clock. Each piece records its commit (`Commits` in `gathered.rs`),
  tests check what each actual commit means (`as_gathered`), and what needs a
  shared commit is arranged again until it happens (`drawn.rs`).

### Checkpoints and the disk

- The journal thread does not checkpoint the WAL itself
  (`journal::checkpoint`): commits report log length, another thread with
  its own connection is told every thousand pages, and the writer
  checkpoints only at a bound between pieces of work. Never set
  `wal_autocheckpoint` on that connection or request a checkpoint that waits
  for readers. The checkpoint thread starts when first told, releases the
  file before `Db::closed`, and does nothing for a held database (`Db::hold`).
- The checkpoint interval bounds what a power loss can lose: a durability
  decision, not a speed knob.
- Rewriting identical data is no write and makes no log; a test filling the
  log writes something different each time.
- A test factory never waits for the disk and says so itself
  (`journal::disk`): `Db::open_unwaited` in suites that open journal files,
  `Config::disk` for in-process nodes, `JOURNAL_DISK=unwaited` for daemons
  the CLI suite starts. A real daemon is never told. A test of the disk
  itself says nothing; the journal suite uses `Db::open`, and only a waiting
  database counts a commit as on disk (`Db::kept_to_the_disk`). Anything that
  flushes a file itself (repack, moving apart, a copy) always waits.
- SQLite is built without memory statistics (`LIBSQLITE3_FLAGS` in
  `.cargo/config.toml`), which are a global lock on every allocation. This
  applies to every cargo build here, packages included; SQLite's memory
  figures and heap limits do nothing, and the statistics are never turned
  back on (`docs/architecture.md`, *The journal*). A journal-suite test
  checks the build.

## Measurements

- What is measured regardless of activity (on a timer, or of the journal's
  own work) is measured quietly (`Journal::measure_quietly`), and timers call
  `flush_metrics_when_due`, never `flush_metrics`; otherwise a factory at
  rest writes on every tick. A journal-suite test checks a journal at rest
  is not written to.
- A measurement is in memory or in the database at every instant, never
  both or neither to a reader: a flush removes it from memory in the piece
  that writes it, a reading checks memory in the piece that reads stored
  rows (`rollups` in `crates/journal/src/metrics.rs`), and a write not kept
  puts values back first (`gather::then`). Read through `rollups`; remove
  from memory only on the database thread (`between.rs`).

## How long run logs are kept

- Two settings in the factory's journal (`journal::letgo`): `logs.kept`
  (days; copied) and `logs.folder` (where step output goes; not copied).
- After the days set, step files and the runner's own step and run events
  (`conductor::letgo::TOPICS`) are removed, never what a run resulted in.
  Those events are kept at all only where a text in force could be owed them
  (`conductor::owed`); tests keep them as a listener would
  (`Engine::keeps_all_it_says`).
- Nothing owed is removed. Pruning asks how far every listener got
  (`Engine::held_back`; everything is held while a handler is awaited, an
  instance is not in hand, or a conductor is starting), where comparisons
  count from (`Comparisons::counted_from`, via `api::letgo`), and skips runs
  under way. Tests expecting nothing held back wait for rest
  (`simulation::rested`); the runner suite's `letgo.rs` covers the moment.
- A new runner step/run topic is a line of `TOPICS` if it is bookkeeping,
  and left off if anything reads it after the run. Anything owed events from
  a place `held_back` does not see passes it in like comparisons.
- A step's file is read from the first folder that has it (current, earlier,
  data folder: `Logs::folders`) with `Logs::read` or `tail`, never
  `Logs::path`. Service output stays in the data folder.

## The data folder and its lock files

- One daemon and one keeper (`up`) per factory, recorded by three non-data
  files (`folders::folder`, offered as `api::folder`): `lock` (the daemon, or
  whatever works on a stopped factory's files), `serving` (where the lock
  holder listens) and `keeper` (held by `up`).
- `serving` is meaningful only while `lock` is held: written when a node
  takes the folder, removed first as it stops (`Had::serves_no_more`) and by
  anything else that takes the folder (`repack::hold`).
- A refused start states its kind, which keepers act on: `Conflict` is
  permanent (served at the folder's address, the address belongs to
  something else, or the database is too new), `serve` exits 3 and `up` does
  not retry; `Unavailable` is passing (folder held, not served) and is
  retried. An address in use that may be this factory's daemon leaving is
  waited for, and the conclusion is read after the wait.
- A second `up` exits saying where the factory is if its daemon is up, and
  waits if not (`keeps` in `crates/cli/src/serve.rs`), so `down` then `up`
  restarts. Stop commands name the address (`--url`). A keeper writes
  nothing into a folder no daemon has had (`Keep::Nowhere`).
- Offline tools (`folders::repack`) hold the folder like a daemon and wait
  for it like a starting node; what they leave when cut off is removed by the
  daemon at start (`repack::clear`, `system.repack_cleared`).
- Tests: the CLI suite's `beside.rs`, the API suite's `folders.rs`.

## Testing with journals

- Suites run with one journal or one per project, chosen by
  `PROJECT_JOURNALS` (read once, `journal::apart`): `one` keeps added
  projects in the factory's journal; `own` runs the runner suite's plays each
  for a project with its own journal (`Rig::applies_for`, `PROJECT`). Unset,
  a node behaves like a daemon.
- Gates run suites unset only. `cargo xtask ways` runs `one`, `own`,
  `gathered` (`JOURNAL_GATHERS=all`) and `ungathered` (`=1`, journal suite
  only) (`xtask::ways`). Run it before calling done any change to where data
  is kept or how the journal commits.
- A test about a project's own journal first asks whether one exists
  (`one_journal` in the API suite, the same question in the journal, runner,
  comparisons, studio and agents suites) and prints that it was skipped
  otherwise.
- Tests tell order by recorded write order, never event numbers. Runner and
  API rigs record it (`journal::Order`, `Journal::orders_by`;
  `Config::order`, never for a real factory); rig methods (`Rig::event`,
  `said`, `events`, `follow`, `follow_after`) carry it as `seq`, with
  `Rig::mark` for "from here on" and `Rig::number` to convert a number the
  node returns. Never wait on `app.journal`'s `last_seq`, `read` or `follow`.
  The agents' rig and the CLI suite (`fleets.rs`) likewise follow every
  journal; the CLI suite passes `PROJECT_JOURNALS` to its daemons.
- The plays' harness holds every database between turns (`Stepped`): the
  factory's answers, then each project's, while the test waits (`still`); a
  rig stops all of them together (`Rig::stopped_there_told`). Anything that
  blocks one journal's thread on another's is checked there and against
  `journal::moment`.
