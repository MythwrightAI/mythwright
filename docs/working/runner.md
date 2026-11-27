# The runner

This file covers the runner (the `conductor` crate and the crates it stands
on: `runs`, `texts`, `comparisons`): how an instance is given a blueprint
text, how its handlers hear events, what the runner records and emits of its
own steps and runs, how texts are pinned and followed, and how comparisons
plug in. Read it before changing anything in `crates/conductor`,
`crates/runs` or `crates/texts`, or before writing a blueprint whose
handlers must behave in a particular order. The overall model (instances,
runs, generations, the three ways a text is taken) is in
`docs/architecture.md`, *Instances and runs*; it is not repeated here. What
happens when the daemon stops in the middle of any of this is in
[restarts.md](restarts.md).

## Applying and giving an instance a text

- `Engine::apply` is someone asking: it checks everything the text reaches
  and refuses what is wrong. A step of a running instance goes through
  `apply_from` with the calling step and its kind (`Asker::Spawn`,
  `Asker::Call`), and goes on with the last text that checked
  (`crates/conductor/src/reach.rs`, which calls `texts::reach`). Code that
  starts a blueprint for a person uses `apply`; nothing but a step uses
  `apply_from`.
- How an instance comes to a text is one rule, held by `Engine::reapply`
  (`update.rs`) for every kind of asker (`Asker`): its steps run again, it
  takes the text where it is (`current.rs`), or it is carried over
  (`carry.rs`). The look below a parent runs steps only of a child whose
  text ships and is behind what the tree above it is held to (a take
  moved it), once that child's steps have settled; a `call` always
  asks for them. Code that gives an instance a text goes through `reapply`;
  nothing else decides.
- What is applied over an instance is the text as read while the instance's
  turn is held. An asker reads first (`Engine::read` for a step or a person,
  `Engine::there` for the look below and for following) and takes the turn
  afterwards (`Live::apply_lock`, via `Engine::turn_of`, a `Turn`), and
  someone else may have had the turn in between. So the asker notes a mark
  before its first read (`update::mark`), and once it has the turn checks
  `Turn::given_since`; if the instance was given anything since (came into
  hand, a text or params put in force, a carry asked for) it reads again and
  works out the blueprint, params and reach again. What moves with a text
  (what is vouched for, what is pinned and was pinned before) moves inside
  the turn and is put back there if the apply is refused
  (`Engine::apply_over`). `reapply` is handed the turn and takes none.
  Checked by `crates/conductor/tests/suite/in_turn.rs`, which plays every
  number of turns between the read and the turn.
- Something new that reads anything for an instance and then applies it
  takes the mark before its first read and calls `given_since` once it has
  the turn. Something new that gives an instance something to run, other
  than through `Live::redefine`, calls `Live::given`.
- The look below a parent (`Asker::Below`) uses the params the child has
  when its turn comes. `Engine::apply_again` (an instance applied again as
  it is) reads the params before the turn too, noting the mark first and
  handing it on (`apply_read`). Not checked: `Engine::put_back_instance`
  reads params the same way but hands on no mark.
- A turn is let go of before a stop is waited for (`Engine::met`): a stop
  takes no turn.
- A tree is put back on what it reached before it moved by
  `Engine::put_back`, which is an apply and refuses what does not fit.
  Anything new kept for an instance by revision (`pinned.texts`) is named
  where unused texts are put away (`pin_resumed`), or it is gone at the next
  start.
- The definition an instance would go back to carries the state to go back
  to (`Definition.state`), whether its steps run again or it is carried
  over. `was` in `on_update` is read from it, so it is set before the run
  starts and survives a restart.
- Whether an instance is behind its text is worked out from two texts when
  it is shown (`Engine::shown`); a route that hands out an `InstanceInfo` as
  stored says nothing of it.
- A child's key is exactly what its step says with `as:`
  (`crates/conductor/src/child.rs`); `#<position>` is only for a `for_each`
  step without `as:`.
- An apply refuses what it can see is wrong. A test that wants a called
  blueprint to be wrong when called puts it in the library after the caller
  is applied, or names it through a `${ }`.
- `Engine::compile` is for a text that was kept and does not check it
  against what its handlers hear; `compile_new` is for a text an instance is
  about to be given. Making the first strict would fail every running
  instance whose event changed.

## Taking a text: handlers and writes

- An instance's handlers are started in one place (`trigger::listen`),
  which reads the text in force while holding `Live::listening`. Anything
  that gives an instance another text in place calls `trigger::rearm` after
  `Live::redefine`, never `arm`; anything that ends a generation resets
  `listening`.
- `instance.updated` marks the point where a text is taken: every event
  emitted after it belongs to the new text, and no handler of the old text
  takes it. So whatever writes a text as taken first tells the handlers to
  take no more (`trigger::retire`; a generation ended; a carry, which waits
  for them to end). For a text taken in place, the starting place of each
  handler the new text brings is taken before the write and written with it
  (`take_in_place`: `trigger::begins`, `Store::take_text`, then `rearm`
  handed that place). Checked in `as_taken.rs`.
- A save of a followed file is answered when the file is written, not when
  the text is taken; whoever waits for the text waits for
  `instance.updated` with its `revision`.
- Taking a text in place is all or nothing (`crates/runs/src/whole.rs`,
  `Whole`): retiring handlers, the write, `Live::redefine` and `rearm` are
  one piece of work. If the asker goes away (a closed request, a step's call
  cut short) once it has begun, the work goes on as a task of the conductor
  (`Engine::spawn`), still holding the instance's turn (`Live::apply_lock`
  is held apart from the instance so that it can move with the work). While
  the asker is there it is the asker's work, not a task. Writing a run of an
  instance's steps and beginning its task is a `Whole` too
  (`Engine::start_main`, in `unbegun.rs`). Something new made of several
  steps that only make sense together, done for someone who may go away, is
  a `Whole`.
- A handler has a place in the journal only while its instance's text has
  it: a text that drops a handler takes its place away, and
  `trigger::prepare` gives a new one its place, either now or just before
  the event that brought the text (`during`); for a text taken in place, as
  of before the write that takes it.

## How handlers behave

- A handler listens only from when its instance is active. Work whose
  consequences the instance must hear is started in a handler, not in its
  steps: a stage takes up the cards already in its column from `first`, on
  its own `instance.active`.
- What is sent to an instance before it is listening is not heard, and what
  an instance remembers is gone when another takes its place. So state that
  says a question was asked lives on the row it is about (`std/cards` writes
  what a card waits for on the card: `studio::cards::waiting_for`,
  `waits_on_others`, `board.held`), and whatever answers such a question
  also looks for itself when it starts (`first` in `std/cards`).
- Two handlers of one instance run side by side. Ordering across two topics
  cannot rest on state one of them sets: read the row in the handler that
  needs it, and where two handlers write the same field of one record, give
  both `turn:` by that record. A blueprint that says `one_at_a_time: true`
  has its handlers take turns (`Handler::turn`): one that waits then holds
  up the rest, and one that waits for another of its own handlers waits for
  ever.
- A handler that says `turn:` is not held to `one_at_a_time`, is not waited
  for when its instance takes another text, and is ended only by its
  instance being stopped (`Handler::lasting`). What it reads and then writes
  must be the row's, not the instance's state, except for a `set` that adds
  or removes one thing. Its run is written when it is given its place and
  begun when its turn comes, so a `running` run with no step may not have
  begun.
- A list a text keeps of what it has in hand is changed one entry at a time
  with `keep` and `drop` (`conductor::intrinsic`), never by `set` of a whole
  list, because handlers run side by side and a whole list set from one
  handler's read removes what another just kept. Each is written with the
  record that its step has acted. Entries are matched `by` a named field; an
  entry that is not a mapping is compared whole.
- A handler's guard that reads what its instance keeps is evaluated again
  when the handler's turn comes (`one_at_a_time`). A handler that only
  removes something from a list says in its guard that the list is not
  empty: a run that finds nothing to do still costs a run.
- A handler does not hear what its own runs emit. A text whose handler
  emits its own topic gets an `own-event` warning, which a shipped text may
  not have.
- When a text is checked, what a handler reads of an event is checked
  against what the visible emits of that topic carry; a field no emit
  carries is refused by name. To hand a whole event to a block, pass
  `${event.data}`.
- A template that is a single hole keeps the kind of its value, quoted or
  not (`asking: "${event.seq}"` passes a number). An action argument of the
  wrong kind is refused, so an argument that takes such a value says `Any`.
- What is done as a text is taken (`on_update`) cannot call a block: each
  entry is one action and reads only what earlier entries set. Record the
  before and after there, and work out what it means once the handlers
  listen again (as `stage.taken` and `column.changed` do).
- Work done as an instance comes up is done in the run that heard it come
  up (a stage's `first`, `staffed`, `listening` call `std/due` directly),
  not by emitting an event for another handler of the same instance, which
  would queue behind everything else it is doing.
- A handler's place moves for what was addressed to it (and, at rest, every
  few hundred events on its topic: `delivery::SAVE_EVERY`). A handler that
  replaces one of the same name goes on from where that one got to, with
  `for:` worked out from what its instance is handed now, so it can be
  handed events from before its `for:` changed. Nothing a handler addressed
  by a param does may rest on the event being new: it checks what is true
  now before it acts (a stage asks `work.due`, `review.owed`, `work.owed`;
  what it has begun for each card is its own `had` list). Not built: giving a
  handler whose `for:` changed a fresh place, written with the text
  (`Taken::places`); `crates/api/tests/suite/comings.rs` says when it is.
- **Whoever wrote something may wait for its hearers** (`Engine::handled`,
  `delivery::Delivery::done_with`): each handler's waiter says, as it
  comes back for more (`Inbox::rest`), the last event it has done with
  (its run ended; begun, where it takes turns by a key; or passed over),
  starting from its place. A wait is for the handlers on the index that
  the event is for, in every conductor, looked at again when one stops
  listening, so one that takes its place is waited for too. Nothing is
  kept; a wait ends with the daemon, and none begins while it holds new
  work. Only the API's record routes wait, for tables that steer
  (`fleet.md`); a step never does, since its own handler may be the one
  that hears it.
- A handler's run that fails does not fail its instance and does not run
  the blueprint's `on_fail`, which belongs to its steps. It emits
  `run.failed`; dealing with it is another handler's job.
- A `spawn` carries on without waiting for the child's steps. A step that
  needs what the child's steps did `call`s it.
- A `call` runs the child's steps again, which cancels whatever the child
  was doing. Something long-lived that must finish what it has in hand when
  changed says `on_update` and is `spawn`ed, so it is carried over in place.
  A `call` cut short (run cancelled, time out) stops only the child it made
  (`child::start` keeps which, only where `Applied::created`; a call taken up
  after a restart uses `had`): a child that existed before is left running
  with nobody waiting. So what a handler calls into being should own nothing
  that must outlive the handler's run, and a timeout on a call of something
  long-lived does not bound its steps. Checked by `compose.rs` and
  `crates/api/tests/suite/reapplied.rs`.

## Who hears what

- A handler and a `wait` hear what the principal that applied them may read
  (`Engine::hears`), plus what their own instance's tree emits. A test with a
  narrow key that waits on another area's event gives that key that area's
  read scope.
- With no `scope`, a handler or `wait` hears its own project's events and
  the factory's own (no project's); `scope: fleet` hears every project
  (`crates/conductor/src/whose.rs`, `trigger::in_scope`). An instance
  applied for no project hears only events that are nobody's. `for:`
  narrows, never widens.
- Whose an event is, is one function, `Engine::whose`: the project its data
  names, else that of the emitting instance, else nobody's. Nothing else
  works it out, and nothing reads a project out of an id. A workspace's
  events and messages in a workspace's channel are nobody's;
  `project.placed` and `channel.placed` move a channel between projects. So
  an event of something that belongs to a project and names no instance
  carries `project` (an agent's turn via `Agents::of_slot`; a channel's
  events). A test build checks this for slots and channels
  (`crates/api/src/whose.rs`), and `crates/api/tests/suite/whose.rs` plays
  whole journeys with a short list of factory-owned families.
- An apply warns of a handler or `wait` that will hear nothing (`unheard`,
  `whose::unheard`) and goes ahead. Nothing shipped may warn; a package's
  case fails if one does. A shipped handler meant to hear nothing when
  applied for no project says `scope: fleet` and is narrowed by `for:`.
- A text applied by an earlier version goes on hearing every project's
  events until applied again: `instances.heard` says how a row hears, and is
  kept with the definition it would go back to. Anything that writes an
  instance row or a `Definition` passes the mark on unchanged; only a text
  taken by this version sets it. Never set it in a migration.
- A handler hears from several journals (`Engine::hears_from`: its
  project's and the factory's, or every one for `scope: fleet` and for a
  text from an earlier version), through one inbox per journal
  (`delivery::Inboxes`), each fed by that journal's own task
  (`Deliveries::of`). Whatever a handler does with the event in hand uses
  the journal it came from (`Inboxes::last`): its place moves there
  (`Handler::start` is handed the journal), its chain is that journal's
  (`chain_of`). Never take `engine.journal()` for it. A handler given its
  first places gets one per journal (`trigger::places`); a journal with no
  place is read from its beginning, and that is the only time a place is
  zero. Checked by `several.rs`.
- A handler checks for journals made since it was last told each time it
  comes back to wait (`Inboxes::known`), not only while waiting, since it is
  at `Inboxes::next` only between runs. Something new that reads from every
  journal does the same.
- A handler's place is a place in one journal, stored with that journal's
  name (`runs::places`, `Journal::name`: `journal::FACTORY`, or
  `journal::of_project`). A number of one journal is never compared with a
  number of another, and moving a place in one touches no other
  (`crates/runs/tests/suite/places.rs`). A journal is named for whose events
  it keeps, not where its file is. Something new that keeps where it got to
  among events keeps the journal's name with the number (a `wait` and a
  comparison's count use `journal::Places`). Still a bare number in the
  run's journal: where a step, an element or a hook began.
- No order between two journals' events may be relied on, by a handler, a
  test or a blueprint.

## Events and numbers

- An event's number is unique in the factory and grows within a project;
  nothing more (`docs/architecture.md`, *A journal for each project*). There
  are gaps, and two projects' numbers say nothing of their order. Nothing
  adds to a number, waits for the next one, or orders two projects' events
  by it.
- Where someone outside has read to is an opaque place, handed out and taken
  back unread (`crates/api/src/cursor.rs`: `cursor`, a frame's `id`,
  `since`, `after`). A new answer that says where the journal has got to
  hands out a place beside any number; the client and the CLI keep the place
  (`Stream.at` in `ui/src/events.ts`).
- An id is a name: nothing takes one apart except a record id
  (`Collection::owns`). The data folder layout is private to the program.
- What a program emits for itself is a printed line starting `::measure` or
  `::event` (`hosting::said`); a blueprint uses a `measure` or `emit` step.
  Neither may use a system name (`blueprint::labels::measurement`,
  `RESERVED_TOPICS`). A new family of system measurements is added to
  `MEASURED`.
- `finished` is emitted after any ending, with `status`; it is not the
  opposite of `failed`. A failing step, run or instance emits `failed` then
  `finished`, and `on_finish` runs after `on_success` or `on_fail`. Read
  `status`, never take `*.finished` to mean success. A step that never began
  emits only `step.skipped` or `step.cancelled`. A run is `cancelled` only if
  something of it was cut short (`run_flow`); one whose steps had all ended
  when its instance stopped went well. A cancelled run is not counted as
  failed (`metrics run` counts `run.failed`, checked by `endings.rs`; agent
  turns called off are not failed, `leaving.rs` in the agents' suite).
  Something measured that can be stopped from outside uses `ok` to mean "did
  not fail".
- A failure is shown to people once, by its reason. The whole error names
  every step it came through (`kernel::why`); `reason` on `run.failed`,
  `instance.failed` and a step is the innermost alone. `instance.failed`
  says which instance's `call` waits on it (`called_by`) and whether that
  step lets it fail (`let_be`). A handler that reports failures under it
  (`trouble`, `let_be`, `slipped` in `std/fleet`) skips what a step will fail
  with. Whether a step called for an instance is checked before its run
  reads as finished (`run.rs`).
- A registry checks handlers against events only after `emits_no_more`,
  which only the node calls. The conductor's tests emit made-up events under
  the program's own families on purpose. A test rig that starts a node asks
  it what it emitted (`held`) when it is put away.

## What the runner emits of its own work

- The runner's own topics (`conductor::owed::OWN`: `step.started`,
  `step.finished`, `step.failed`, `step.retrying`, `step.skipped`,
  `step.cancelled`, `step.waiting`, `run.started`, `run.resumed`,
  `run.failed`, `run.finished`) are stored only while some text in force
  could be owed them (a handler, a `wait`, or a topic with a hole), worked
  out from the texts and never from who is listening now; while a conductor
  takes things up at a start all of it is stored. Otherwise the event is
  passed on (`runs::Say::kept`, made by `Engine::own_say`): emitted and
  numbered in the same write, delivered to followers once it is committed,
  but not stored (`journal::Writing::pass_on`; `Journal::pass_on`).
- So nothing reads these back from the journal. What a restart needs is on
  the row, written in the same write: where and when a step's try began
  (`steps.began`, `began_at`, `Say::begins_step`, read by
  `Store::step_began`) and that a run began (`runs.begun`,
  `Engine::begin_run`, read by `run::said_started`). Something new a restart
  must know of a step or run goes on its row; a new topic of the runner's
  own is a line of `OWN` emitted through `own_say`.
- A test reads emitted events, stored or passed on, through the order its
  rig records (`Order::read_every`, `follow_every`, `follow_from`,
  `read_after`; every rig's `events`, `event`, `follow`, `said`). Such a
  follower hands over passed-on events its journal streams missed without
  moving any of its places. A follower catching up from the log goes by what
  it read there (`Follow`). A journal that passed a number on records that
  it has holes (`numbered::HOLES`), so pruning counts from its rows.
- A follower that falls behind live events goes back to the log, where
  passed-on events are not. Whatever acts on one of these topics as it is
  emitted calls `next_or_behind`, which returns nothing when it fell behind,
  and then reads what it needs another way (`Studio::watch_runs_from`,
  checked by the API suite's `rescanned.rs`).
- The client's stream: `/api/events` resumed from a place after which
  something was passed on (`Last-Event-ID`, a `cursor`), or falling behind,
  first sends a `behind` frame with `conductor::PASSED_ON`, which is no
  event and moves no place (`Journal::passed_after` decides). Opened from
  now it sends a frame with an `id` alone. In the client a subscription
  says how it reads again (`again`, last argument of `useEvents` and
  `stream.on`; `followsAny` in `ui/src/events.ts`). Anything that follows
  `run.*` or `step.*` must pass `again`: `events.test.ts` reads the source
  for it, and `ui/scripts/reconnected.mjs` plays a cut stream.

## Step and run records

- A passed-over step (its `when` failed, or its run ended first) has no
  write and no journal place of its own (`RunCx::pass_over`). Its record is
  held and written with the run's next write, held only while no other step
  of the run is at work, and written before anything slow after the last
  step (`RunCx::write_held`) and always where the run ended early. Anything
  new that passes a step over uses `pass_over`; to learn afterwards which
  steps were passed over, read the run's steps, never the journal.
- A step whose action only decides has no write either (`ActionSpec::deciding`,
  `RunCx::hush`, `Held::quietly` in `crates/conductor/src/run.rs`): when
  nothing is done around it (no hook, no watching rule, no list, no other
  device) and no other step of its run is at work, its records are held and
  written with the run's next write. Stopped before that, it has no record
  and runs again from its start. So an action is `deciding` only if it emits
  nothing, writes nothing and reaches nothing outside the factory; an
  intrinsic is never marked (`set` writes into the step's row). Checked by
  `decides.rs`.
- A step whose action is idempotent (`.idempotent()`, including `emit`,
  `set`, `wait`, `stop`) and does more than decide has its ending held the
  same way and its beginning written as usual (`Quiet::Ending`,
  `conductor::holds`). Stopped while its ending is held, it is taken up as a
  step stopped mid-action, told so (`ActionCx::again`), from what it saved,
  without re-evaluating `when`. Its beginning is never held. The API suite's
  `quiet.rs` lists every action by what is held of it and checks that a
  `deciding` action makes and runs nothing and is idempotent. Something that
  reads a step's record between its action ending and the run's next write
  reads `Engine::run`, which includes what is held.
- A step's record is where its own action keeps its place
  (`ActionCx::checkpoint`, a `wait`'s `wake_at`). What is done around a step
  (hooks, a watching rule) keeps no action place there and is redone from
  its start; it is told apart by the kind of call (`Call::around`,
  `ActionCx::around`), never by the try number.
- An instance of a blueprint with `durable: false` keeps nothing as it goes:
  whatever records a step's progress (checkpoint, loop element, a stored
  `step.*` event) checks `RunCx::kept` first and emits via `RunCx::say`. Step
  records go through `RunCx::put_step`, which holds them until the run ends
  (`run::Held`) and writes them together.
- A run's steps are read in the order they began, given by `steps.nth`, not
  by `steps.started` (milliseconds; many steps share one). `nth` is assigned
  in the statement that first writes the row (`runs::put`: one above the
  highest of the run), never changed after, and is the sort key (`IN_ORDER`).
  Held steps are kept in begin order and read after written ones
  (`Engine::run_here`). Rows from an earlier version have no `nth` and sort
  first. The number is handed to nobody (`STEP_COLUMNS`, `StepInfo`), and
  nothing re-sorts steps after reading. New writes of `steps` go through
  `runs::put`. Checked by `crates/conductor/tests/suite/ordered.rs` and the
  runs suite's `ordered.rs`.
- A journal made now keeps no index of events by instance or by run
  (`journal::events::UNKEPT`). Every read by instance or run after a restart
  says where to begin (`EventQuery::after`: where the step began, a stored
  place, or `runs.place` via `Store::run_place`). Nothing new reads by them
  on the path of a factory at work.
- An instance's row keeps no text: the version row does, and the instance
  names it (`instances.text`, read through `INSTANCE_COLUMNS` in
  `crates/runs/src/read.rs`), because the row is rewritten on every state or
  status change. Rows written by an earlier version, and one reverted after
  an update (`store::end_update`), keep the text inline with no version.
  Something new that gives an instance a text does one or the other, never
  both.

## Texts, pins and versions

- A running tree goes on with the shipped blueprints as it reached them when
  applied (`crates/texts/src/pinned.rs`), whatever version runs it later,
  and moves only when someone applies with `take`. Anything that gives an
  instance a text by name goes through `text_for`, which calls `pinned_for`.
  Only what ships with the program or a package is pinned
  (`Library::ships`): a program run from the cargo build folder reads the
  checkout's blueprints live and pins none of the standard ones; an API test
  node pins the built-in ones. An upgrade test declares its library ships
  (`MemoryLibrary::ship`), runs an installed program, or uses a package's
  blueprint (`crates/cli/tests/suite/moved.rs`).
- What a blueprint links (`text.put`'s `from:`, a prompt) is pinned with the
  tree under its library name (`std/prompts/review-seat.md`) and read via
  `Engine::linked_for`, never `text_within` alone. A kept text is a
  blueprint or words, told apart by name (`library::is_linked`); words are
  never compiled, compared with `means_the_same` (`pinned::same` is exact
  equality) or diffed as a blueprint (`words::differences`). They are
  checked against their blanks where linked (`Walk::linked`, and `text.put`
  at run time). A package brings linked files under `prompts:` in its
  manifest (`crates/api/tests/suite/prompted.rs`).
- How a running tree comes by new texts is a factory setting
  (`crates/texts/src/taking.rs`: `held`, `follow`, or unset; and `in_place`
  or `again`). Whether a text is held is asked of `Engine::holds`, never of
  `Library::ships`. Anything that reads a pin goes through `pinned_for`,
  which returns none when texts are followed.
- `follow` reads no file past the vouching. A project's own file is taken by
  being vouched for as the principal who turned `follow` on, with what they
  held then (`Engine::follow`, over `vouch_saved`). Turning it on needs
  `shell` and `admin` (`set_taking`). Something new that takes a text
  because it changed goes through `Engine::follow`. The node says when a
  text changed (`crates/api/src/following.rs`): the journal for project
  files, `hosting::Watch` for a checkout's blueprints unless texts are held.
  A test of following writes the file through the daemon
  (`PUT /api/projects/{id}/file`); a file written with `std::fs` is noticed
  only at the next scan.
- Every shipped blueprint states its `version:`, and a version is one
  meaning (`crates/api/tests/suite/versions.rs`, `contract/versions.txt`,
  `extensions/<name>/contract/versions.txt`). Whoever changes what one means
  runs `WRITE_VERSIONS=1 cargo test -p api the_versions`, which bumps the
  number in the text and writes its line; commit both together. Layout and
  comments are not changes. A line is removed by hand only for a version
  that never left the repository. Linked files (`blueprints/std/prompts/`, a
  package's `prompts:`) have lines too, written by the same command; their
  version is their last line, every word counts, and a changed prompt is
  never a new version of the blueprint that links it.
- What moved between two texts is reported by version first, then in words
  (`conductor::Moved`; `moved` in `crates/cli/src/applied.rs` and
  `ui/src/versions.ts`). The words omit the version; a new place reporting it
  passes on both.
- "Version" belongs to the blueprint text. How many definitions an instance
  has had is shown in the client as which time it was applied (`called` in
  `ui/src/versions.ts`), never as a version. What a time was applied with is
  shown in words, folded, not as JSON (`AppliedWith` in
  `ui/src/views/history.tsx`, `ui/src/handed.ts`; folds kept per tab in
  `ui/src/handedopen.ts`). No secret is ever there.

## Comparisons

- One blueprint started several ways lives in its own crate
  (`crates/comparisons/src/compared.rs`) and is chosen in the one place a
  step starts a child (`child::start`). No blueprint is written for it, and
  it knows nothing of the work. It counts and never scores: `compare show`
  prints measurements labelled `compared`, `way` and `text`; a way that
  follows its file is counted per text, and nothing adds counts across
  texts. A way's own text may call what it stands in for: started from
  inside that text, the name is started as written and counted with the way
  (`within_its_way`). Something new a comparison could go by is a field of
  `Comparison`, not a language word. Pure shapes and functions are in
  `kernel::compared`.
- A way pinned to a version (`<blueprint>@<version>`) is found only among
  texts trees are pinned to or can be put back on (`Engine::text_at`); an
  unknown version is refused, listing what exists.
- A way may read another linked file (`--prompt <way>.<file>=<other file>`,
  `Way.prompts`, `crates/comparisons/src/instead.rs`). `text.put` calls
  `read_instead` before reading through the pin; what an instance reads is
  recorded when it is started under a way (`keep_reads`), never looked up
  from labels at run time. Something new that starts things under a way
  (`way_for`) records what they read where it gives them their labels.
- What came of a thing under a way is a counted event
  (`crates/comparisons/src/counted.rs`, `system.comparison.came`): whoever
  starts the comparison says which events, which field names the thing and
  which fields say the outcome. The usual list for cards is `CARDS` in
  `crates/api/src/routes/compared.rs` (`counts: "cards"`, the CLI's
  `--per item`); a request that says nothing counts nothing. Something new
  worth counting for a card is an event a blueprint emits plus a line of
  that list.
- The counting follower is told of changes by `Notify::notify_one` and
  checks for it before taking its next event (`biased`, in `count` of
  `counted.rs`); `notify_waiters` would miss a follower busy with an event.
  Its place is taken by whoever starts the comparison (`count_from_here`),
  and starting, telling and ending are under one lock (`Kept::counting`).
  Something new that tells a daemon task that something changed keeps the
  notification for it the same way.
- The runner knows only that something may answer for comparisons and asks
  it four things (`crates/conductor/src/comparing.rs`,
  `conductor::Compared`): what a step starts instead (`way_for`), what an
  instance reads instead (`read_instead`), which kept texts are still wanted
  at a start (`pin_resumed`), and that a conductor is starting
  (`Engine::resume`). `Comparisons::attach` installs the answers, and must
  be called before `resume`. With nothing installed the runner behaves as if
  no comparison ever ran; comparison tests live in the `comparisons` suite,
  since `conductor` may not depend on it even for tests (the `layers` gate).
  What is installed is handed the engine per call and holds none.
- What a comparison shows is laid out once by the daemon
  (`crates/comparisons/src/beside.rs`, `beside` in `/api/comparisons/<name>`):
  columns by way and text, rows by kind (`came`, `chose`, `spent`,
  `accepted`, `per_accepted`, `took`). The CLI and client only render it. A
  new figure is a cell field or row kind there. Accepted is a count's word
  (`kernel::compared::accepted`), counted once per thing. A thing another
  process continues with is recorded as the way's (`compare.went`). A count
  reads no board: a board's last column is read when the comparison starts
  and handed in (`last_column`). An ended comparison keeps where it ended
  (`Comparison::until`) and counts what was emitted before then. Rows are in
  the order things happened; nothing orders, marks or names a column as
  better. Sums are divided by how many things the column was given
  (`counted: given`).
- A measurement is a duration unless its name's ending says otherwise
  (`blueprint::labels::unit`, `UNITS`: `_usd`, `_tokens`, `_mb`, ...).
  Whatever hands series out says each `unit` (`with_units` in
  `crates/api/src/routes/fleet.rs`), and displays go by it, never by a list
  of measurement names.
