# Stopped anywhere, and started again

This file covers what happens when the daemon stops at any point (killed,
asked to stop, or told to stop at a place for a test) and starts again: which
writes are atomic, what a conductor finishes as it starts, how stops,
carries and the look below a parent survive a restart, how the tests prove
all of it, and what "the daemon has stopped" means. Read it before changing
anything the runner writes, anything a conductor does in `Engine::resume`,
or any code that stops a node. The model of instances and runs is in
`docs/architecture.md` (*Instances and runs*, *Stopped anywhere, and
started again*); the runner itself is in [runner.md](runner.md).

## The guarantee and how it is tested

- A blueprint stopped anywhere ends as one that was never stopped. This is
  checked in the conductor's own suite, with no fleet in it
  (`crates/conductor/tests/suite/anywhere.rs`, scenarios in `plays.rs`).
- A *play* is a scripted run: a small blueprint and what is done to it,
  played once, then once more for every place it passed, with the conductor
  stopped at that place and started again. There are two ways to stop
  (`Way`): the journal and its database shut at the place; or nothing
  emitted after the place while what was being written is kept. The
  simulation in the API suite (`crates/api/tests/suite/simulation.rs`) plays
  whole factories the same way.
- No place is skipped and there is no list of places known not to hold: a
  play that ends differently is a fault. Of what is done for one step (its
  action and its hooks), the one thing that was under way may be asked once
  more, never more. Differences allowed by design are said by a play's judge
  (`kept_nothing`, `left_behind`), never by a looser comparison.
- A change to the runner is tried against the plays first; a new way a run
  can go is a new play.
- Every word of the blueprint language is checked four ways: a case that
  reads, is formatted and is refused when written wrongly
  (`crates/blueprint/tests/fixtures/words`, the blueprint suite's `held.rs`),
  and a blueprint that is played, stopped and started (the conductor suite's
  `held.rs` over `plays.rs`). Which words a text uses is read from the text
  (`vocabulary::written`). A new word fails both until its case and play
  exist.
- Every run in every play is checked by `amiss.rs`: a run that ended emitted
  that it finished, once, and none of its steps reads as under way; a run
  that went on emitted that it started, never twice.
- A play that never reaches its end records what it kept, as one that ends
  wrongly does. A play whose fault shows only in the order of two things
  says from what each play kept whether that order came about
  (`Scenario::there`, `Found::noted`), and fails if no play reached it.

## Making order deterministic in plays

- Without help, which of two things a conductor is doing happens first
  depends on the machine (the database's thread may answer before or after
  the asker yields). So a play holds its databases from its beginning to its
  end (`Stepped` in `stepped.rs`; `Stepped::holding`, handed on by
  `Rig::stopped_there_told`) and they answer one piece of work per turn;
  what comes first is then how many answers each side needs. A turn in which
  nothing was asked (`Db::done`) means nothing is about to change and is
  answered where the test stands (`Stepped::asked_nothing`). Anything the
  play's driver asks of a database mid-play goes through `Stepped::through`
  (asked directly it waits for ever); a play about an asker that does not
  yield says `Does::StopsAtOnce` (`Stepped::at_once`).
- A play that exists for one order gets it by what it puts between the two
  sides, never by timing, and checks that the order came about
  (`Found::came_by_the_step`, from the `instance.begun` measurement).
- The plays of one scenario run several at a time, each on its own thread,
  runtime and conductor (`crates/conductor/tests/suite/anywhere/abreast.rs`),
  and produce the same plays as running them one after another (checked by
  `the_plays_made_several_at_a_time_are_those_made_one_after_another`). Each
  runtime always polls what its own thread woke first (`runtime`, `first`),
  so the order a conductor keeps things in does not depend on load. How many
  run at once is twice the tests run at a time (`RUST_TEST_THREADS`, or the
  processors). What the harness reads of a finished play is read on its own
  thread and returned as plain values (`Made`).
- How many plays each scenario makes is recorded in
  `crates/conductor/tests/fixtures/played.txt` and checked by `counted.rs`
  (`Found::held`). If the plays or the runner changed,
  `WRITE_PLAYED=1 cargo test -p conductor` writes the new counts, committed
  with the change; if nothing changed, a count that moves means an order now
  depends on something unwritten, which is a fault to find first. Plays
  outside `anywhere` (`alone.rs`) are not counted.

## One write

- What is written and what is emitted about it are one write
  (`Journal::write`, `journal::together`): the writer is handed the
  transaction and emits its events inside it; followers see them once it
  commits. Stopped at any instant, both are there or neither, so nothing
  looks at a start for something written and never emitted.
- Every place the runner writes a run, step or instance as begun, ended,
  settled, stopped or carried over emits its event that way
  (`runs::together`: the event is prepared with `to_say` and handed to the
  write). How a run ended and what became of its instance are one write
  (`Store::end_run`). Something new that writes and then emits goes through
  that door; `emit` alone is for an event that records nothing beside it.
- What the conductor emits about something it writes is emitted in that
  write, including its own step and run events: `step.skipped` with the
  held record (`run::Held`, `RunCx::pass_over`); `hook.failed` in the write
  that marks the entry done (`Around::keep_saying`), with a failed list
  recording why (`Around::failed`) so it is not redone; a text refused as it
  was to be taken, in the write that ends the carrying run
  (`Engine::end_carry`). A handler a text brings has its place written with
  the text (`Taken::places`, `trigger::places`). These are checked by
  handlers that hear them (`apart.rs`); a new event the conductor emits of
  its own gets a handler there.
- A handler's run is written when it is given its place and emitted as
  started when it begins. A run found running with nothing emitted is
  begun. A run taken up that never started emits `run.started`, not
  `run.resumed` (`run::gone_on`).
- What becomes of an instance is decided in the write that records it,
  never before. `Engine::settling` works out the outcome, then waits (for a
  failed update's work to be handed back, for children to be counted); so
  the write checks again (`Becomes::still`, the generation, read inside the
  transaction) and writes nothing for an instance an apply or a stop has
  since taken. What is in hand changes in that same write (`Becomes::then`),
  and `Engine::settled` checks once more before starting handlers or letting
  go. Likewise an instance is written as stopped, or a text as taken, only
  where it is still running (`Store::stop_instance`,
  `update_instance_definition`). An apply that finds the instance ended is
  retried once where applies come in (`Engine::apply_from` over
  `apply_once`), and only one that ends again in that moment is told to ask
  again (`update::ended_meanwhile`). Checked by `settled.rs`. Something new
  that settles an instance states in its write what it assumes.
- A run of steps takes no turn to end, and its ending is one write. So
  whoever decides by an instance's row and its last run together reads both
  in one database answer (`Store::standing`), never one after the other:
  holding the instance's turn stops others beginning its steps, not ending
  them. `reapply` reads them the same way. Checked by `begun.rs`, whose last
  test plays every amount of work a start could do before reaching the
  child.

## What is owed across a restart

- An ending that cannot be one write records what is owed. An instance that
  fails emits `failed`, stops what it started, then emits that it is over.
  The failure is written with a note (`instances.owes`), removed in the
  write that says it is over, and a starting conductor finishes every
  instance that owes (`Engine::take_down_owed`, via an index of those rows
  only). Something new that does something slow between two things it emits
  records what it owes the same way; never look by a window of time.
- At a start nothing is in hand, and an instance stopped without being in
  hand is only written as stopped (its `on_stop` does nothing). So what is
  alive under an instance that owes its taking down is brought into hand
  first (`hand_below` in `settle.rs`), only to be stopped: not begun, not
  listening. Something new that stops instances as a conductor starts does
  the same.
- A journal written by an earlier version may hold an instance written as
  active but never emitted as such. The migration that added `instances.owes`
  marks every active instance, and the first conductor to start on that
  journal checks them once (`Engine::say_active_unsaid`), emitting what was
  missing in the write that clears the mark. Anything owed once by an older
  journal is marked by the migration that makes it owed.
- An instance that keeps nothing, found alive by a starting conductor, is
  stopped with its runs in one write (`Engine::over_at_start` in `left.rs`,
  `Store::stop_with_its_runs`: stopped, why, each run cancelled, all events
  including `run.finished`), then `notify_run_finished`. A test of this has
  several items in hand, not one.
- A run found under way whose instance is over is ended as cancelled
  (`conductor::left`, `Engine::end_left`): unsettled steps written as
  cancelled, `run.finished` emitted, waiters told, in one write. Never end a
  run by writing its status alone; nothing in the store writes a run's or an
  instance's status by itself. A handler run whose handler the text no
  longer has is ended the same way (`trigger::listen`). A passed-over step
  whose held record was not yet written reads as cancelled after a stop.
- A turn of an agent that is `running` at start was left by the previous
  daemon and is decided on once, in `turns::run`: taken up where its program
  is, begun again, or left. The asker said which (`again`, `most`, `until`
  of `TurnRequest`; `after_restart` and `restarts` of `agent.task`). Something
  new that asks an agent for work that can be mid-way at a restart passes
  those on.
- Something new that is answered before it is done, with nobody left to ask
  again, writes what it owes with the answer, as carries and the look below
  a parent do.

## A conductor's start

- A start never fails for one instance. One that cannot be begun (what it
  attaches cannot be brought up, its first run cannot be written) is settled
  as failed (`conductor::unbegun`). Anything new in `Engine::resume` that can
  fail for one instance settles that instance and goes on.
- An instance that was being given another text when it could not be begun
  has not failed; the text has. It goes back to its previous text, authority
  and state, leaving what it had running (`gone_back`). Nothing may leave an
  instance written as taking a text with no run: `reapply` settles it before
  answering.
- An instance whose kept text no longer checks against this device's actions
  and events has failed and is written owing its taking down
  (`unreadable_at_start` in `taken_up.rs`), so what it started is stopped by
  `take_down_owed` before anything is taken up (checked by `heard.rs`).
- A start compiles each text once (`texts::compiled`, used as
  `crate::compiled`). A compile is keyed by whether it is to be given, its
  name and revision, plus what it asked the library for its project
  (`compiled::Asked`); a result is reused for another project only where the
  library answers the same. Anything a compile comes to ask the library is a
  question of `Known` that records itself (`compiled::asked`). The cache
  lives only for the start's own task. Measured as `system.resumed`,
  `system.resumed_count`, `system.resumed_texts_count`; checked by
  `compiled.rs`. A wrapper over a whole start is boxed, as `for_a_start`
  boxes what it is handed, to keep stack frames small.
- An instance written but not yet begun is begun one way only
  (`Engine::begin_owed` in `unbegun.rs`), under the instance's turn
  (`Live::apply_lock`), reading the row and its last run together
  (`Store::standing`). Both the start and the step that asked for it may
  reach it; whichever comes second finds the run the first began, or the
  instance over, and does nothing. A step taken up looks first for the child
  it made whose steps never ran (`never_ran` in `child.rs`: a `call` by the
  instances before it asked, a `spawn` by its child having been emitted
  after the step began) and begins it or fails as it failed; it never asks
  by name for what it already made. Who began it is measured
  (`instance.begun`: `start`, `step` or `apply`). Checked by `begun.rs`.
  Something new that begins an instance's steps where none are under way
  holds the turn and calls `begin_owed`.
- An update is two writes (text taken, then the run of its steps); an
  instance stopped between them is owed its steps (`owed: again`), begun
  through `begin_owed` by an apply that asks for what is written. One being
  carried over is written as taking a text too, but is not owed steps
  (`Store::carrying`).
- A step taken up that finds the start began its child's steps, and they
  have since ended, takes that run (`never_ran`). A `spawn` likewise finds
  its child by its having been emitted after the step began (`made_here`),
  whether its steps never ran, are under way or ended.
- The runs of an instance's handlers that were under way when the conductor
  stopped belong to those handlers once started: at once for an instance at
  rest, after its steps or carry settle otherwise (`trigger::leave`, taken by
  `listen`). They are kept through the end of a generation
  (`Listening::ended`, asked by `Live::next_generation`), and whatever takes
  an instance down ends those no handler took (`trigger::unclaimed`, in
  `Engine::teardown`). Something new kept for handlers until they start is
  kept the same way and ended with the instance.
- A run taken up with one of its own steps written as cancelled stays
  cancelled (`run_steps`): a step is written so only when its run was being
  cancelled. Steps inside a task are not read this way; they may be cancelled
  by a sibling failing.

## Stops

- A stop that has begun is owed. It is recorded for the top of what is
  stopped before anything is done (`Store::save_stop`, `owes` is `stop` with
  why), and for what is under it from the first entry of its `on_stop`. A
  starting conductor finishes every owed stop before taking anything up
  (`Engine::finish_stops`); what was half stopped is never brought back.
  A stop of which only its beginning is recorded has done none of its
  `on_stop` (`Around::kept`) and does all of it. The plays read stops from
  what their conductor starts on (`Rig::stopping`, `STOPPING`); the entry a
  stop reached may be done once more.
- A stop is done whole once begun (`Engine::stop_as` is a `Whole`): an asker
  that goes away does not leave it half done.
- What is under something stopped is called off first, in one pass, before
  anything is taken down or any `on_stop` runs (`Engine::call_off`, called
  by `stop_as` and `teardown`): each generation ended, its handlers taking
  no more, its runs cut, parents before children. Then each is taken down in
  turn, newest first, with its `on_stop` done whole. Whoever first calls an
  instance off also calls off what is under it (via its `Cancel`). Something
  new that takes an instance down calls `call_off` first and never cancels
  one instance alone. Checked by `cut_first.rs` and
  `crates/api/tests/suite/stopped.rs`.
- An apply that reaches an instance being stopped takes nothing and changes
  nothing: it waits until the stop is over and then applies as where nothing
  runs under the key, making the instance anew (`crates/conductor/src/meets.rs`).
  Both happen, in the order asked. A stop announces itself from before it
  does anything until it is written (`Stops`, in `Engine::stop_as`), and
  `Engine::reapply` checks it twice: once it has the turn and has read the
  instance, and again right after ending the generation where steps will
  run. `apply_once` waits for the stop (`Engine::met`, which also walks up to
  a stop the instance is part of) and `apply_from` retries.
- A stop takes no turn and waits for no apply. Where waiting would be the
  stop waiting for itself, the apply is refused with a sentence saying what
  to do: an apply from the instance's own `on_stop` or anything it started
  (`WITHIN`, carried by `Engine::spawn` and a run's steps), or from a step of
  the instance or anything under it (`asks`). The look below a parent
  (`Asker::Below`) skips an instance being stopped. Something new that gives
  an instance a text, or decides by what is alive under a key, calls
  `Engine::meets` once it has read the instance; work a stop waits for is
  begun through `Engine::spawn`. Checked by `meets.rs` (`test.lingers`,
  `Does::Asks`). Not checked: an apply made by something that hears of the
  stop through an event waits; nothing ties the two together.
- A `stop` step done again after a restart looks first for what it already
  stopped since it began (`stopped_since`), then at what is alive under the
  name, so it does not stop a new instance started since.
- A `stop` step that stops its own instance cannot wait for that stop (which
  cuts its run), so the stop begins beside it, announced from the moment the
  step asks (`meets::Ahead`); the run's ending leaves alone an instance a
  stop is under way for (`Engine::settling`). Something new that ends an
  instance from beside its own run announces it the same way first.
- What is ended begins nothing more. A handler and a waiting run each choose
  between being ended and the next event, and `tokio::select!` picks at
  random. So each checks again once it has an event: a `wait` checks whether
  it was called off before giving the event out (`intrinsic.rs`), and a
  handler checks whether it was ended or retired in the one place it begins a
  run (`Handler::start`), leaving the event and its place for the next
  handler of the name. Checked by `called_off.rs`. Something new that waits
  for an event and can be ended checks once it has the event and before it
  acts. `biased` alone is not enough on a multi-threaded runtime.
- A step chooses between its action ending, its run being cut, a watching
  rule and a timeout (`exec_step`). A step whose run was cut before its
  action began does not begin it (it checks before first polling the
  action). After that the order is fixed (`biased`): an action that ended
  ended as it did, then a cut run is cut. Checked by `called_off.rs`.
- Not checked, listed in the README: an action made of several calls is
  looked at once more as its run is cut, and may make the next call first;
  and a run that was cut may still be ending when handlers of the next
  generation have handled the next event.

## Carries, the look below, and handlers starting

- A request to carry a running instance over to a text is answered once the
  run that will do it exists, and that run waits for the handlers to be
  between runs. So the target text is written with the run, in the write
  that makes it (`Store::insert_carry`, `runs.carries`), and cleared in the
  write that takes a text or ends the run (`runs::carried`,
  `Store::end_run`). A starting conductor has each instance owed a text wait
  for it again (`Engine::carries_owed`), and carries it over once the start
  has reached the instance and started its handlers, retired at that moment
  (`Engine::reached`).
- Asked again for the same thing by the same bearer the same way, the run
  already there answers (`waits_for` in `carry.rs`). The run records which
  step asked (`runs.asked_by`, `carry::asked_by`), so a `spawn` taken up
  finds its answer there before asking again (`Store::carried_for` in
  `child::start`). Checked by `below.rs`.
- A text asked for during a start, for something the start has not reached,
  waits for the start (`Carries::unreached`). Every carrying run is ended by
  what waits to carry it (`wait_to_carry`); the start ends only runs with
  nothing kept, written by an earlier version, saying to apply again.
  `reached` is called at the end of whatever the start does for each
  instance, and a start that gives up reaches the rest (`carry::Reaching`).
  Checked by `answered.rs`.
- Whatever gives an instance a text calls off any pending carry, in its
  write (`runs::carried` keeps the run as called off) and in hand in the same
  turn (`Live::next_generation`, `take_in_place`). Checked by `waited.rs`.
- That the children of an instance are owed a look, once the steps of an
  apply someone asked for settle, is recorded with the text (`owes`,
  `below`) and cleared when the look is done (`Engine::look_below`,
  `Store::looked_below`). In hand it is the generation plus one
  (`Live::owes_look`). A starting conductor looks where the note is, as the
  steps it takes up settle or at once (`Engine::resume`).
- A text taken before the look is over keeps the look owed, whoever gives
  it: the write keeps the note and reports it
  (`Store::update_instance_definition` returns `runs::Below`). The write and
  the clearing are under one lock (`Engine::take_owing`, `Live::looks`), and
  a look clears the note only if no text was taken owing one since it began.
  An update reverted owes no look unless the note was there before it
  (`Started::below`; then `Settled::Reverted`, `gone_back`). Something new
  that writes a text as taken, with steps or `on_update` to run after it,
  goes through `take_owing`; `take_text` leaves the note alone. Checked by
  `below.rs` (`Does::Put`, `Does::Again`).
- A child's text read by a step before it has the child's turn is read
  again under the turn if the child was given anything meanwhile (see
  [runner.md](runner.md), applying; `in_turn.rs`).
- An instance's handlers start a moment after it is written active. No
  handler starts while a carry has retired them (`Listening::retired`, set by
  `trigger::retire`, cleared by a new generation or by `rearm`), and what
  settles a generation's steps starts handlers only while that generation is
  current (`trigger::arm_of`; `Listening::ended`). How long a start was held
  back is measured (`instance.listening_late`). Checked by `carried.rs`.
  Something new that starts handlers because something settled says which
  generation it settled.
- A freshly started handler is not yet in the delivery index
  (`delivery::Index`) for one database question; events emitted before that
  are read from its place, not lost, but are not counted by
  `handler.delivery`. A test that counts deliveries does not count from an
  instance being active (`addressed.rs`, `rung`).

## Taking up a step

- An idempotent step that leaves something behind looks for it when done
  again: `emit` for its event (`said_before` in `intrinsic.rs`), `stop` for
  what it stopped. A `set` writes its value in the same transaction as the
  record that its step acted (`conductor::state`), so it needs no look.
  Something new that changes instance state goes through `Engine::set_state`
  naming its record; never a "who set last" mark.
- Hooks around a step (`on_start`, `on_success`, `on_fail`, `on_timeout`,
  `on_finish`) and around a run's ending record how far they got
  (`hooks::Around`) in the step's or run's record: which entries ran, and
  how the action ended, recorded before acting on it. Taken up, they go on
  from the next entry; the entry in progress looks for an event it emitted.
  This is written only where such entries exist, never for a run that keeps
  nothing, and read only when taking up.
- Rewriting a step's record (taken up, saving progress, ending) leaves what
  is kept beside it untouched (`runs::put`), except the write that begins a
  new try, which clears it. Something new kept in a step's row is preserved
  by that write, never written back afterwards.
- An entry is told it is being done again only where it may have begun: a
  run records that its ending began before the first entry
  (`Around::begin`). Values set by hooks or by a watching rule never mark the
  step as acted (`Call::around`).
- `on_update` (run in the carrying run) and `on_stop` (in no run, so kept in
  the instance's row, `instances.hooks`) are kept the same way
  (`hooks::Around::alone`): begun recorded before the first entry, progress
  after each, a `set` written with the count; an entry whose `when` fails
  records nothing. Taken up, `on_update` continues from its entry with what
  earlier entries set. Something new done as an instance changes hands or
  ends goes through `alone`.
- A step with `on_start` entries is under way from the first of them; its
  action begins when the last is done. Taken up with one still to do, the
  action never began (`Around::starting`), so it is run as new, not refused.
  Taken up with all done, the action may have begun and is redone or
  refused. What the action emitted is searched from after the last start
  entry (`hooks::started`, read by `said_before`). Checked by `started.rs`.
- A `wait` takes its place among the journals as its action begins and
  emits `step.waiting` after `step.started`. A test emits what a wait is to
  hear only after the wait says it waits (`Rig::started`; `several.rs`).

## The simulation

- The simulation takes in what a daemon recorded as it came up, before the
  play was listening (`stop_and_start` in `simulation.rs`), in emitted
  order and before what the daemon emits once the play listens again
  (`caught_up` in `simulation/passed.rs`). Something new the simulation
  reads from a restarted daemon goes through `hooked` and `caught_up`. A
  play given up on reports how it differed from what was expected.

## Shutting the journal at a place

- `Journal::shut_after` makes the journal take nothing after a given place;
  from then its database does nothing more for anyone (`Db::hold`) and every
  caller waits for ever. It is never reopened: `Node::stop` emits one last
  event (`Journal::emit_last`). Only the simulation and `down --at` set a
  place.
- A test that set a place asks that node nothing more until it is stopped
  and started; a request in flight may never be answered. It learns the
  journal reached its place from `Journal::shut()`, not from inside the
  place's own check. The rig's stop, start and every request it makes have
  time limits (`crates/api/tests/suite/main.rs`: `in_time`, `Rig::call`).
- Whatever goes on after a journal may have shut asks it through
  `Journal::unless_shut`, which gives up when it shuts. Checked by
  `crates/api/tests/suite/shutting.rs`.
- A step whose action comes back with the database-gone refusal
  (`ActionError::gone`), where its journal has shut or its conductor was
  halted, writes nothing more and waits (`exec_step`); the refusal alone is
  not enough, since a step on another device may fail with the same words.
  Checked by `refused.rs`. Something new in the runner that records a
  failure checks this first.

## When a daemon has stopped

- A daemon has stopped when its database files are free, not when it has
  asked them to close (`Db::closed`, awaited by `Node::stop`). Every database
  of the node (the factory's, each project's journal, search) is asked to
  close before any is waited for, under one bound (`PUT_AWAY`, a minute). One
  not free in time is reported with what its thread was doing
  (`Db::standing`); `Node::stop` returns that, the daemon prints it, and a
  test rig fails on it (`Rig::stop`). Checked by `given_back.rs`. Something
  new that waits for a database thread reports `Db::standing` on timeout
  and never carries on as if it had finished.
- A node dropped without being stopped asks its databases to close and waits
  for none: a test that starts a node on another's data stops that one
  first.
- A served store stops the same way (`StoreServer::stop`, `put_away` in
  `crates/served/src/store.rs`), and binds its address before opening its
  database, so one that cannot listen opened nothing.
- A daemon has stopped when its address is free too. A program the daemon
  was starting holds copies of its open files until it execs, and gives the
  folder lock back before the listener. So a stopping daemon connects to its
  own address until nothing answers, for two seconds at most, saying so if
  it waits (`let_go_of` in `crates/api/src/lib.rs`, `stop_saying`), with the
  folder still held. A killed daemon cannot, so a daemon starting on its
  folder waits for the address it was served at
  (`folder::served_when_it_ended`, in `listen_at`). Both are Unix only.
  Checked by `listened_with` in the CLI suite's `daemon.rs`. Something new a
  daemon holds that a following start would be refused for is waited for in
  the same two places.
- A node that is dropped gives back what it held, stopped or not
  (`Drop for Node`, reached last by `Node::stop`): what it started is ended,
  and what it follows, its databases and its open logs are closed. Something
  new a node opens that holds a thread, a file or a file watch is released
  there, with a line in `crates/api/tests/suite/given_back.rs`.
- Its memory is given back too: no part of a node may hold another part that
  holds it. What one part installs into another is held weakly
  (`Engine::downgrade`) or holds only what it reads, and a task that follows
  a journal is ended with the node (`Studio::halt`). `given_back.rs` checks
  that journals and the engine are released
  (`journal::Downgraded::is_given_back`); something new that can be held
  weakly is added to it.

## Restarting on purpose

- A daemon never restarts itself; what keeps it running does. The
  `factory.restart` step (`api::restarting`) has the node stop saying it
  should come back (`Node::again`); `serve` exits with 4
  (`restarting::AGAIN`) and `up` starts it again at once. `up` tells each
  daemon it is kept (`serve --kept`, `Config::kept`). A test standing in for
  `up` holds the keeper's lock (`folder::keep`) and restarts the node when
  asked (`Node::asked_again`).
- The step is cut off with everything else and done again by the daemon
  that comes back, which finds it done from that alone (`ActionCx::again`).
  Nothing in it may answer before the stop, and nothing is kept for it.
  Where nothing keeps the daemon, the step is refused before doing anything.
- Before restarting, it waits for what is under way to end, woken by
  `run.finished`, `instance.finished`, `agent.turn.*` and by another such
  step arriving, never by a timer. Something new that is under way and ends
  in none of those is added to `under_way`, or a restart cuts it. Checked by
  the API suite's `restarting.rs` and the CLI suite's `restarted.rs`.
