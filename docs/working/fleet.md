# The fleet: stages, rounds, reviews and segments

This file covers how the fleet that ships works cards: its tables
(`agents`, `stages`, `segments`, and the `actions` and `tells` tables
beside the board), `std/arrangement`, the three kinds of stage
(`std/working`, `std/reviewing`, `std/planning`), a card's round
(`std/lane-round` with `std/work`, `std/keep`, `std/send`), reviews and
councils, segments, the blocks a stage calls to decide, and what happens
when the daemon stops in the middle. Read it before changing anything
under `blueprints/std/` that a fleet uses, or a block in `deciding`. What a
card is, buttons, sending back, cost, pools and features are in
`board.md`. A *play* below is a scripted run in the conductor's suite or
the simulation, stopped at each point and restarted.

## One fleet, driven by tables

- There is one fleet, `std/fleet`; a project's `fleet.bp` says where it
  is and calls it (`crates/projects/src/fleet.bp`). Who is in it and what
  they do with a card are two project tables, `agents` and `stages`,
  which it declares and fills once; after that the tables decide and no
  param is read for either. A single worker with a person reviewing, and
  a council with a testing stage, are the same blueprints with other rows.
- Something new a fleet can be told is a field of one of the tables and a
  param of `std/role`, or a cell a stage reads of its row: never a second
  kind of fleet and never a param that repeats a row. A project's own
  fleet may write its rows itself and call `std/fleet`
  (`crates/cli/tests/suite/keyed.rs`); nothing that ships targets a fleet
  assembled by hand from parts. Nothing gives a card its own instance; a
  card's journey is tested against the shipped fleet (`lanes::shop` in the
  API suite).
- **`std/arrangement` brings the fleet to what its tables say**, in one
  place. `std/fleet` calls it when applied and again for each `record.*`
  of either table, one at a time. It reads each table whole each time with
  `rows.read` (`studio::rows`), which returns each row with unsaid cells
  filled from what the blueprint passes (`fill`) and facts about rows
  together, and is idempotent.
- `rows.read` knows no table. A row default is a line of `fill` in
  `std/arrangement`; what a stage's kind implies (agents per card, how
  often it goes round) is a table there too, of forms `rows.read` computes
  per row (`fill` for an empty cell; `giving` for what a row gets once
  counted: `like`, `by` another cell, `share`, `each`, `least`), with
  nothing computed in the text. A cell a row may override over the
  fleet-wide value (`review`, `proposal`) is a `fill` line from the
  fleet's param; one that may only lower the computed value (`at_once`,
  never more cards than the role's agents can take) is a `least` form.
- Nothing in Rust goes by what a stage `does`. Anything new that follows a
  row is a step of `std/arrangement`; anything that follows the fleet
  having taken a change listens for `fleet.arranged`.
- `rows.read` returns only what the table's blueprint declares and what
  `fill` names, never a column somebody added (`added` of a field). Such a
  column says no `takes`, and a row is passed whole to what runs as
  whoever applied the fleet, so an added column named like a `fill` cell
  (`key`, `push`, `remote`) or a segment param (`command`) reaches
  nothing: it is for people to read. A blueprint that is to read a column
  declares it (`crates/api/tests/suite/steering.rs`).

## Filling the tables

- A table's first rows are added by one idempotent action, `rows.start`
  (`studio::first`: a row is added unless the table has one of that name,
  as many per name as the list has), never by `record.add` per row. It
  adds and never changes or removes, and knows nothing of why. That an
  emptied table stays empty, and a removed row stays removed, is the
  `when` of the calling step (`first_agents`, `first_stages` of
  `std/fleet`: not once the fleet has been up, and only while empty). Which
  rows are still owed is a function with cases (`owed`, the studio suite's
  `first.rs`). Something new that gives a table its first rows calls the
  same action.
- A cell emptied in the client is kept as empty text; one never filled is
  nothing. Readers of a person-edited row treat both as unsaid:
  `rows.read` does so once for any table (`said` in `kernel::rows`). A
  blueprint reading such a row itself writes `(x ?? '') != ''`.
- Where a row says nothing of a cell and `fill` has no default, it is
  passed on as empty, and empty is what the agent gets: an emptied
  `model` means the provider's usual model, an emptied `told` means no
  instructions of the row's own (`std/arrangement` passes `told` with
  `also`, never as `""`). `agent.ensure` treats `""` for a model or
  instructions as none. The only standard words of a role are those the
  first rows are written with (`first_agents`). The Edit drawer and
  `byhand` behave the same. A value that should survive an emptied cell
  is a `fill` line (`crates/api/tests/suite/edited.rs`).
- What is wrong with a stage's row is looked for in a handler on the
  stage's own `instance.active` (`looked`), emitted each time it is
  carried over, not in its steps (which run once), since a row is added
  by name and filled a cell at a time. What is wrong, and each sentence,
  is a block (`column.wrong`, `agents::column`), a case per sentence.

## Agents of the fleet

- Every agent the factory starts is brought in by `std/agent`, whose
  params are the slot's settings (`docs/architecture.md`, *An agent is set
  up by a blueprint*): a role's agents one each under the role, an agent
  added by hand by one keyed by its name (`api::byhand`, which the add and
  change routes call, `POST /api/channels/{id}/agents` among them).
- Whose an agent is, is read from what instances hold and nothing else
  (`Agents::bringing`, `Brought`): one keyed by the agent's name is its
  own, and what is above it counts it. Something new that adds or changes
  an agent by hand goes through `byhand`; something new that asks whose
  an agent is calls `bringing`. A new slot setting is a param of
  `std/agent` (with `description`), a line of `byhand::params_of` and
  `said`, and an argument of `agent.ensure`.
- An agent's name is unique in the factory (`slots.name`), so the fleet
  names each after its project and row (`shop-worker-1`), and work asked
  of a role never goes to another project's agent
  (`Agents::available_apart`). Anything new that brings agents up for a
  project names them for it.
- Which of several equally free agents gets work is a word the asker
  gives (`pick` of `agent.task`, `picking::Order`); a stage's row says it
  for a fleet (`picks`), passed down to a round or a review's seat. A new
  way of choosing is a word there; a rank is a number of the row's and
  means nothing to the daemon. A card's assignee overrides the row
  (`std/work`).
- Who gets work, of those who could take it, is a rule (`std/dispatch`,
  `agents::dispatch`, via `deciding::dispatch`): the daemon orders them by
  the asker's `pick` and calls the rule, read as the step's instance may
  run it (`Engine::text_within`). It has no steps and never runs; what it
  names must be one of those shown. A new fact a rule could use is a field
  of `dispatch::Offered` or something the asker puts in `about`.
- Choosing and queueing are one thing under `Agents::choosing`. Anything
  new that gives work to whichever agent is free holds it from choosing to
  `enqueue_chosen`; `enqueue` takes it itself.
- What comes of work is counted, never scored: a turn's measurement says
  what the agent chose (`decision`), a stage what it came to
  (`review.verdict`).
- An agent no longer counted always leaves by finishing what it has
  (`std/role`, `leave: finish`), so it does not take a just-begun review
  with it.

## Stages

- A stage is a row of `stages` and an instance of the text for its kind.
  There are three texts, by what is done in the column: `std/working`,
  `std/reviewing`, and `std/planning` (a plan, or the blueprint a row
  names). `std/arrangement` starts the one a row's `does` maps to (`runs`
  and `called`, from `rows.read`) and hands it the whole row (`row`: every
  cell, how many agents its role has, and the fleet-wide stage settings),
  so something new a stage is told is a cell, not a param.
- A stage does with the cards of its column what its row says: plans them
  (`std/proposal`), works them (a round each when its turn comes,
  `std/lane-round`), reviews them (`std/review`), or, for any other word,
  starts the blueprint of that name for each (`other` in `std/planning`).
  A working stage tracks a card by its row; a planning stage has a
  long-lived instance per card while it is in the column. Either way the
  agent is whoever holds the conversation (`plan:<card>`, `lane:<card>`).
  Something new done with a card in a column is a word in a row and a
  blueprint of that name, never a second pipeline or Rust.
- A card arriving in a column is due what its stage does, emitted as the
  stage's own event (`stage.due`, `review.due`), and what is done with a
  due card is emitted once.
- The working handlers are one text because they take turns with each
  other by the card (`go`, `once_more`, `back`, `answered`, `tidied`).
  What a review emits is heard by the instance that began it
  (`scope: own`), so the reviewing handlers are one text too; two stages
  of one project would otherwise take each other's verdicts.
- Shared pieces are written once: the look at a column (`std/due`), what
  is wrong with a row (`std/looked`), pool state (`held`, `going` of
  `std/arrangement`, which has the pools under it), and every sentence and
  decision, which are blocks. What is repeated is what must be the
  instance's own: looking when first there (`first`, `staffed`) and what
  it does as its row is taken (`on_update`).
- A block is handed the whole row where it reads several words (`own`,
  `row`) and a whole event's data (`said`, `taken`) through one helper
  (`agents::begun::with`); a card's words are still passed one by one.
  Something new a stage does goes into a text it calls or a block it
  calls, never a fourth kind of handler and never a text that starts one
  of the three under itself.
- **One card's slow work holds up no other card.** A stage is
  `one_at_a_time`; a handler that may wait on one card's work (a command a
  row lays in, a merge, a review's ending) takes its turn by that card
  (`turn:`) and changes stage state an entry at a time (`keep`, `drop`):
  `due`, `came`, `decided`, `handed` of `std/reviewing`; `begun`, `after`,
  `went` of `std/planning` (`segments.rs`, `planned.rs`). Something new a
  stage does that waits takes its card's turn; what stays stage-wide waits
  on nothing.
- **What has a stage look at its column** (`first`, `staffed`,
  `listening`) calls `std/due` in its own run, never by emitting an event
  for another handler, which would queue behind whatever the stage is
  doing.
- **A handler's place moves for what was addressed to it**, and a handler
  replacing one of its name carries on from that place with its `for:`
  recomputed. So a stage whose row is given another column is handed every
  move to that column since its handler last got anything. Nothing a stage
  does for a card rests on the card having been said to be due: what it is
  owed is asked of its row (`work.due` then `work.turn` where cards are
  worked; `review.owed` where reviewed; `work.owed`,
  `agents::work::coming`, where planned or a named blueprint is started:
  the card is in the column now, and this arrival, told apart by the move
  count, is not one the stage keeps as begun). The stage keeps `had`, an
  entry per card, dropped when the card leaves the column (`went`, which
  checks the row first) or the board. Something new a handler addressed by
  a param does for an event asks what is so now before acting. `handed` of
  `std/reviewing` does not, and cannot from the row; what would end it is
  the runner giving a handler whose `for:` changes the place a new handler
  is given, written with the text (`Taken::places`), which is not built
  (`crates/api/tests/suite/comings.rs` says so).
- What a stage says of a restart goes only to shipped blueprints: a
  project's blueprint a stage starts is given what it always was, or it
  would be refused for an undeclared param. Where cards are planned that is
  a step each (`proposal`, `proposal_own`, `other` in `std/planning`);
  where reviewed it is one step, and the words go with who looks only where
  the shipped review is used (`restarting`, from `std/arrangement`, read by
  `segments::council`).
- **A stage stopped and applied again takes up what it had in hand**
  (`crates/api/tests/suite/stopped.rs`, both orders of agents and stages).
  A review in session is begun again by the new stage's look at its
  column. A round in hand was called off with its stage, and the new
  working stage takes each such card up again (`took`, `std/resumed`,
  `work.left`: in `doing`, taken from its column, row saying a round began
  and nothing more), giving back the card's named pool place. Its agent is
  told the card is taken up again, not sent back (`again: [resumed]`,
  `resumed` of the round and `std/work`,
  `prompts/lane-round/resumed.md`); it is not counted as a retry.
- An instance never becomes another blueprint by being given its text: its
  name is what it is listed, compared and pinned by
  (`update_instance_definition`). So a row told another kind keeps the old
  instance with what it had in hand, and a new instance of the new kind is
  made under another key (`working-<row>`, `reviewing-<row>`,
  `planning-<row>`; `other` and `another` in `std/arrangement`).
- What is under something stopped is taken down newest first
  (`Engine::teardown`), but everything under it is called off first
  (`Engine::call_off`), so a round is cut before its agent's turn is.
  Where an agent is stopped by itself under a running fleet, nothing above
  the round is stopped and the round fails (said in the README). A play of
  something stopped under work in hand is made in both orders (`later` of
  `held`).

## Rounds

- **A round is a list** (`std/lane-round`): three small blueprints, each
  one thing and each ending in a word: the work (`std/work`, at 10),
  keeping it (`std/keep`, at 20), and sending it where the stage says
  `push` (`std/send`, at 30), plus whatever rows of the project's
  `segments` add between them or in place of one (a command before the
  commit: a row at 15 naming `std/run`; cards with a label to one agent: a
  row at 10 naming `std/work`, `labelled`, `with: agent=<name>`).
- The round takes the card and hands it on, so a card's history says the
  round moved it. What it does between is asked of the list
  (`segments.next`, the same block a reviewing stage calls) and called by
  the name the block returns. The round calls its first thing; the rest is
  `std/through`, which calls the next thing, asks what follows and calls
  itself with that, one tree level per thing (the same text continues a
  reviewing or planning stage's list after `std/segments` calls its
  first).
- A text may call itself only where the step says `when` (`recursion` in
  `blueprint::validate`), and the runner caps any tree at 128 deep
  (`conductor::deepest`), failing the step with a sentence naming the
  texts that looped. What bounds a round is a row cell, the most things a
  stage does in one pass over a card's list (`most`: 6 where cards are
  worked, 5 otherwise, filled by `std/arrangement`; 0 for the list's own
  length), passed to the first asking (`most_done`, `most_of`, the row id)
  and carried on. Each asking says which thing it calls (`nth`); one that
  would exceed it leaves the card for a person with a sentence naming the
  command that raises it (`segments::beyond`). What a stage begins and
  hears itself (a review) is not counted. `most` is read as a pass begins;
  it is passed to no segment (`segments::NOT_HANDED`), since `std/keep`
  uses that name for something else.
- An asking after the first is handed what the previous one answered
  (`was`) and what the called thing gave (`gave`), and nothing else; the
  first is answered back whole (`through`, `asked`), and a final answer is
  passed through to the end, so the last answer is what the round came to.
- **A round keeps nothing as it goes** (`durable: false`), nor does
  anything it calls: it costs a card no stage step or run, only an
  instance per thing done. A daemon restarted while an agent has a card has
  the stage's run for that card (`go`, which is kept and waits on the round
  with a `call`) ask for the round again; that step is the one place a
  stage acts at a restart. Begun again, the round starts at the first of
  its list and finds done what was done: the agent gets the turn it had,
  what was committed is not committed again, a command is run again.
- **A turn is found again by a word**: the round says which round it is
  (`asking`, from `go`: the event that gave the card its turn) and is
  given the turn it asked for then, whether waiting, in hand or answered
  (`Agents::claim`, `crates/agents/src/asked.rs`), so the agent is asked
  once wherever the daemon stopped. A word finds only a turn asked by the
  same asker (same principal, key and starting instance; where the starter
  keeps nothing, it is named by its key under what started it,
  `keeps_nothing` in `crates/agents/src/actions.rs`). With no word only a
  turn still in hand is found. Something new that asks an agent from a
  blueprint that keeps nothing says a word unique to that asking. An
  agent's turn on a card is one text, `std/turn` (called by `std/work`,
  by a plan's round, and named by `segments` rows); something new that has
  an agent take a turn on a card calls it.
- A blueprint that keeps what it does also says a word (`std/review` its
  own instance, each seat that plus its number; a proposal round the event
  that brought the card to its agent): a step redone from its start finds
  its turn by the word, where without one it would fail and leave the
  card. A review's council keeps waiting for its turns (`Task::resume`,
  `crates/agents/src/council.rs`). Neither changes without the restart
  tests (`crates/api/tests/suite/restarted.rs`).
- **A round begun again goes by the card's moves, never by where the card
  is** (`work.held`, asked as the round begins and again before it hands
  on; `heard`, from `go`, is the move it was told of). It may already have
  handed the card on, and someone may have moved it since. Something new a
  round does to a card after its agent finishes asks the same first.
- What a thing gives beside its word goes to what follows it if that
  declares a param of the name (`by`, who worked the card; `available`,
  how the stage knows the card's pool place was given back).
- `back` from a thing in a round is the card given to its agent again in
  another round, told what was said, and counted on the row
  (`again: [again, words, tries]`, or `[unkept, …]` from keeping), as
  often as the stage allows; then it is left. A question is the round's:
  the card keeps where in its list it asked (`at`, a `quiet`, `hidden`
  board field), and the round its answer begins starts there. A failure in
  a thing fails the round, which leaves the card saying why.
- A whole round as an earlier version wrote it (a copy under its own name,
  named in a row at 10) gives no word and is read as what it did
  (`segments::gave`): it handed its card on itself. Comparisons of ways of
  working are of `std/work`; of whole rounds, `std/lane-round`. Something
  new done in a round is a segment and a row, or a change to one of the
  three; never a step of the round, whose text is only the pass.
- **The card is taken (moved to `doing`) by its round**, begun when the
  card has its pool place (`go` in `std/working`); a waiting card stays a
  row in `ready`. Its worktree is made by the round too, then and not
  before, so it is cut after the cards it waits on were merged.
- **When the next card is taken** is a row cell, `next_after`
  (card 479). `handed`, the default and what was always so: as soon as
  the last card's round hands it on, so the next card's worktree is cut
  from the project's branch while the last waits in review, and has
  nothing of its work (`ordered.rs` in the API suite plays it). The
  default is kept because `merged` idles the stage's agents while a
  review waits for a person, and cards that build on each other can say
  so (`after`). `merged`: a card waiting in the column is taken only once
  no card the stage took (`taken` is its column) is in `doing` or in the
  column it hands on to (`hands_to`, given by `std/arrangement`: `pass`,
  or `review` where that says none). It is one card at a time
  (`std/arrangement` makes `at_once` at most 1). What is decided is a
  block, `work.ahead` (`deciding::ahead`, with cases), handed the board's
  cards whole (`board.list` gives each card's `more`): asked in `begun`
  before the card asks for a place, and again in `go` as the place is
  given, since two cards can both be due before either is taken; a card
  behind gives its place back (`gone`). It says why on itself through
  `std/cards` (`pool.waiting` with `why`, said once while it says the
  same), and `caught_up` in `go` takes that off once it is taken. Only a
  card waiting in the column is held back: a card taken already (sent
  back, answered, taken up again) goes on, or a stage carried over from
  `handed` with two cards in review would leave one in `doing` behind the
  other. What lets a card behind go is looked for, never remembered:
  the stage looks at its column again (`std/due`) when a card goes on
  from `hands_to` (`unmerged`), is moved out of `doing` by somebody
  (`away`), is taken off the board (`unboarded`), or the row's
  `next_after` changes (`on_update`). Nothing is kept of a card behind
  but its `waiting`, so a restart finds it as it was
  (`ordering.rs`, a play stopped about the wait).
- **A card that comes back to be worked keeps its worktree as it was**
  (`place.make` answers `there`): it is not brought up to date with the
  project's branch, which may have moved on since it was cut. Only the
  remote's own additions to the card's branch are taken in, where the row
  says `fetch` (`--ff-only`). Under `next_after: merged` nothing moves the
  project's branch while a card is out, beside a person's own commits.
  What happens when such a branch no longer merges is the review stage's
  (card 480).
- A card somebody moves out of `doing` while its agent works has its round
  stopped where the row says `moved_away: stop` (the default): `away` in
  `std/working`, which no card turn holds up. What a move means is a block
  (`work.away`, `deciding::work::away`, with cases): only a person's move
  of a card the stage took that a round has (`again: [heard, n]`); a
  blueprint's move is a round handing on. The card keeps
  `again: [away, <column>]`, written before the stop, and `work.turn`
  takes it up where it was left (`resumed`), or as sent back where a stage
  put it back in hand.
- What `go` makes of the round's ending is asked with the card as it is
  after (`status`, `moved`, `heard` of `work.ended`): one moved since its
  round was told (`moved_since`) leaves nothing for a person, since someone
  has it. `go` then lets go of what its round asked (`unasked`, action
  `agent.let_go`, `crates/agents/src/letgo.rs`): each turn an earlier
  daemon left in hand or queued, that nobody asked for since and that the
  round or anything it started asked for, is called off and its program
  ended (`Agents::let_go_under`, by `asked_by`); one not yet begun by this
  daemon is left as it begins (`turns::run`, `Left::LetGo`). Something new
  that starts something that keeps nothing, asks agents, and can end
  without asking again lets go the same way (the simulation's
  `takenout.rs`; `restarted.rs`).
- **What a round came to that someone acts on is emitted by the run that
  asked for the round**, which is kept (`go` in `std/working`), from what
  the round returns, never by the round, which would repeat it after a
  restart. The agent's question (`card.asked`) is emitted that way
  (`work.ended` gives `asks`, `asker`, `question`). What a round still
  emits itself (`card.worked`, `card.pushed`, `card.unkept`, posts) may be
  repeated after a restart, and nothing acts on it.
- **Work that could not be committed does not go on to review**
  (`std/keep`, which calls `place.keep` to answer rather than fail,
  `unkept_ok`, and ends in `back` or `left`). Where the agent can fix it,
  the card is given back to it with git's message by whatever asked for
  the round (`given_again` in `std/working`), bounded by the stage's
  `rounds`, passed as `most`. Where git has no identity to commit as
  (`named`), nobody is asked again.
- A card's place that cannot be made fails its round (`unplaced` in
  `std/work`). The project's own folder is where a card is worked only
  where there is no repository or no history yet; anything new that lets a
  round go on without its place puts an agent's work among the project's
  files with nothing saying whose it is.

## Reviews and councils

- A review is an instance of its own each time, named after the event that
  asked for it (`review-<card>-<event.seq>`), so a card back in review while
  its last review is ending is not handed that one.
- Its seats are not instances but one step, `agent.task` with `seats`
  (`crates/agents/src/council.rs`): asked side by side under one choosing
  turn, each answer counted with the tally, whoever is still looking
  called off once that decides, and in the step which turn is whose and
  what was decided, so the kept review waits for the same seats after a
  stop. A council anywhere else is that step (the agents' suite's
  `council.rs`, the simulation's `seated.rs`).
- **A card has one review under way at a time**, tracked by its stage
  (`reviewing` in `std/reviewing`) from when a review begins until it says
  how it came out, or the card leaves the column or board. A card said to
  be due while it has one gets no other; only a review that says of its
  own card that it is to begin again does (`again` on `review.due`).
- A card leaving and arriving are heard by one handler in the card's turn
  (`came`), so in emitted order.
- With each card the stage keeps which review it has, by name
  (`review-<card>-<seq>`, which it says itself and so keeps before the
  review exists; never the instance a spawn returns), and removes it when
  that review is over however it ended (`ended`, on its child's
  `instance.finished`, which carries the `key`). A review that stopped,
  failed, or kept nothing and went with the daemon says nothing of how it
  came out: its card then waits for a person, and is reviewed again when
  put back or when a role gains somebody.
- **Which kind of ending it was is never worked out where the ending is
  heard.** `ended` only emits that the review is over (`review.ended`),
  and that is heard by `decided`, which also heard the verdict: one
  handler takes events in emitted order, so the verdict comes first, and
  what an ending means is a word of `review.decided` (`waits`). Something
  new a stage decides from two things a review emits, or from a verdict and
  an ending, is heard by one handler the same way (`concluded.rs` in the
  API suite makes the order).
- A review that asks for another in its place and then stops emits
  `review.due` with `again` and says which review it is (`review`);
  `decided` hears that and removes that review by name before its ending,
  so the card is not said to wait while its replacement is coming.
- The shipped review does not end itself after its verdict: it says which
  review it is with it (`review` of `review.passed`, `review.failed`,
  `review.undecided`), and `decided` ends it once it is removed (`closed`),
  so its ending sets nothing off. A project's own review that ends itself
  is heard as before.
- A card left for a person after being sent back as often as allowed is
  tracked the same way with no review named, so it is said to be left once
  per arrival.
- Not built: a stage carried over from a text that kept none of this has
  nothing in `reviewing` for reviews already in session, so such a card
  could be given a second review. Filling it would need an action that
  gives an instance its live children with their keys and params.
- `in_flight` (a row of `stages`, read by `std/reviewing`) decides what
  becomes of a review in session, and nothing else.
- **An approved branch that no longer merges is brought up to date for
  its worker**, which behind its wall reads the repository's own files
  and cannot write them (so it can neither begin a merge nor end one).
  `std/merge` asks `place.merge` with `bring` (`crates/places/src/
  merged.rs`): the project's merge is called off, and the project's
  branch is merged into the card's in its place. With nothing conflicting
  that way it is made and the card's branch merged again, and the card
  goes on with nobody asked. Otherwise it is left under way (`MERGE_HEAD`
  in the place) and the card goes back, told the files and to change
  files only (`review.verdict`, `begun`, `files`); `place.keep` ends the
  merge with the worker's commit, even where nothing else changed, and
  refuses one with git's markers left in (`unkept`, so the card is given
  again as unkept work is). Asked again after a stop, a merge of the
  project's branch as it is now, under way with files to put right, is
  found as it was (`conflicted.rs` in the API suite, both places).
- Going back for that is no verdict (`behind` of `review.verdict`, passed
  on by `segments.next` and `std/segments`): the stage keeps it in `back`
  marked `behind` and moves the card with `uncounted`, so neither the
  stage's count nor the card's `rounds` has it. `review.owed` counts those
  apart and leaves the card after `reviews::BEHIND` (3) of them, so a
  project's branch that keeps moving in the way cannot loop it.
- A card left for a person having been sent back as often as allowed is
  kept with the limit it was left under (`most`); a row whose `rounds`
  changes drops those entries (`on_update` of `std/reviewing`), and its
  look at the column has each looked at, or left, by the new limit.

## Blocks a stage calls to decide

- What a stage decides is a block: what a turn for a card means
  (`agents::work::turn`, `work.turn`) and what an ended round means
  (`ended`, `work.ended`). A block gets a row's words as plain arguments,
  never a record; answers a word (`came`) and what goes with it; refuses
  nonsense with a sentence; and has a case table reaching every word it can
  answer (`crates/deciding/tests/suite/work.rs` fails for an unreached
  word).
- What an agent's answer means is `deciding::turn::came` (`turn.came`,
  called by `work.worked`), by the words a turn may end with and what each
  maps to (`ends`: `done=on ask=asks` for work, `propose=on ask=asks
  leave=left` for a plan), so a new word is a setting, never an
  expression. Votes are counted by the words for and against
  (`review.tally`'s `yes` and `no`, `tally.rs`).
- A council is a setting of the one turn (`seats`, `needed`, `yes`, `no`
  of `std/turn`, passed to `agent.task`; one agent when `seats` is 0), so a
  `segments` row can lay one into any stage. What the seats come to
  (`passed`, `failed`, `undecided`) is mapped by the row's `ends`
  (`Ends::council`: `passed=on failed=back`, and `undecided=left` unless
  said). A card a council fails goes back to whoever worked it with what
  those against said. The words a `segments` row hands a segment (`role`,
  `picks`, `seats`, `needed`, `ends`, `yes`, `no`) need no more than
  writing the row; only what reaches a command or secret (`command`,
  `with`) needs `shell` (`tidied.rs`, `seated.rs`).
- What a card due a review is owed is `agents::reviews::owed`
  (`review.owed`). What a verdict means (merge first, card on or back and
  with what words, sentences for a blocked merge or an undecided card,
  what is told where the stage reports) is `agents::reviews::verdict`
  (`review.verdict`), called by `std/merge` with its merge result. What
  becomes of a card for each way a stage hears of it is one answer,
  `agents::decided` (`review.decided`): do more first (`after`: the list,
  or the merge alone), or the word to act on. No block is handed what a
  stage `does`. Something new a stage decides is a case in a block's
  table before it is a line of the text.
- **A card left for a person is emitted as `card.left`**, by the stage,
  once per time and with `why` (`rounds` where it was sent back as often
  as allowed): by a reviewing stage (`said`, `gone`), a planning one
  (`gone`), a proposal (`leaves`), and for a round by the run that asked
  for it (`left` of `go`, from `work.ended`'s `left` and `leaves`), never
  by the round. Something new that leaves a card for a person emits it
  where it says so in the channel, or `std/told` and whatever counts
  outcomes never hear of it.

## Segments

- What more is done with a card in a column, beside what its stage usually
  does, is a row of the project's third table, `segments`, naming a small
  blueprint (a *segment*: `std/run`, or the project's own). It is handed
  the card and what it declares of its row and stage, and ends in a word
  (`came`: `on`, `back`, `again`, `asks`, `left`). The contract is in
  `docs/blueprints.md` (*Writing a segment*).
- It runs as whoever applied the fleet, so what its row hands it is
  checked against the row's writer at the write: `command` and `with` need
  `shell`, `with` also `secrets:use`, and `with` says only what is more
  than the row's own cells.
- Rows are changes to the usual list, not the list: a usual thing is at
  10, a row at 5 comes before it. A stage with no row, and a card no row is
  for, do the usual. What a card is owed says, in one call, which thing is
  begun and whether anything comes first (`lists` of `review.owed` and
  `work.owed`, i.e. `agents::begun::first`, with cases); the list is gone
  through before a review or plan only then.
- What comes next, what a segment is handed, what a command's exit means,
  and what is wrong with a list are blocks that know no card or stage
  (`agents::segments`: `segments.next`, `fitted`, `segments.ran`,
  `segments.wrong`), each with cases. What a segment is told of the card's
  arrival is what the pass said, filled in with the card's labels and
  column where missing (`segments::coming`).
- **Where cards are reviewed**, the list is gone through by something that
  keeps nothing (`std/segments`, called by `due` in the card's turn): each
  thing is called by the name the block returned, until there is a review
  to begin, which it hands back for the stage to begin and hear from
  (`begins`, with reviewers and quorum where a row names its own review:
  `segments::council`), or until the list ends, which the stage emits to
  itself (`review.came`) so the card goes on, back or is left in the one
  place that does that (`decided`). What came before a review is still
  emitted of the card (`stage.came`, once per pass).
- After a review passes, what was laid after it is gone through from where
  it was, which the stage kept with the review (`reviewing`: `row`,
  `order`, and `usual`, whether only the usual came after); where only the
  usual did, the merge is called directly (`std/merge`). The runner's
  suite checks what this rests on (`chained.rs`).
- **Nothing of a pass is read from `params` after the call**: a handler
  that takes turns by a key keeps going while its instance gets new
  params, and each later step reads the new ones. So the pass is handed
  the rows as they are, and each thing's name, input and retry count are
  the block's answer inside it. A row added mid-pass applies to the next
  arrival; what runs after a passed review is what was laid after it when
  that review began. A card moved mid-pass has left it: the pass looks at
  the card again as it ends (`last`, with its column and move count) and
  answers `nothing`.
- The name a row gives is read as the stage's instance may run it
  (`Engine::text_within`): a project's own only as vouched for; one nobody
  vouched for leaves the card for a person with a sentence. `text_within`
  does not go through the pin, so on a tree pinned to an earlier shipped
  segment the call refuses what the pinned text does not take, and the
  card is left, saying so.
- **A segment that runs something keeps nothing** (`std/run`,
  `durable: false`), so after a stop its command is simply run again. It
  gets its card in its environment (`CARD`, `CARD_TITLE`, `CARD_LABELS`;
  `segments.next` adds labels and `fitted` joins a list with commas for a
  text param) and runs where its row's `in` says (`segments.where`,
  `run_in`): the card's worktree or the project's own folder. A card
  meant to be worked in its own place that has none (`place.changed` says
  `there`) is left; its command never runs in the project folder instead.
- Where its row names a table (`record_to`, a cell needing `shell` like
  `command`), each ended run is kept there as a row (`record.add` with
  `declaring`, which creates a project table the first time, fills only
  fields the table has, and never writes a table that says `takes`,
  `reads` or `steers`, or is a model's); the row is a block
  (`segments.recorded`) and the commit `place.head`. A run cut by a stop is
  not kept; one redone after a stop is another run. Keeping it never
  changes what the card comes to (`allow_failure`).
- Where its row gives a time (`warn_after`, refused by `fitted` where
  invalid: `segments::TIMES`), the command step is watched (`after:`
  computed as the step begins; an empty time is never timed,
  `conductor::hooks::timed`) and past it the card is emitted as
  `card.slow`, which `std/told` tells to whoever `tells` names for `slow`.
  Nothing is stopped. It is told once per arrival whichever run says it
  (`once` of `inbox.tell`, from `tells.whom`, kept as the inbox line's
  `said_by`) (`slowed.rs`, the simulation's `slowing.rs`). Something new a
  fleet tells of a segment's progress is an event the segment emits and a
  word of `tells.whom`, told once by `once`.
- **Where cards are planned**, or handed to a blueprint the row names, the
  usual is that one thing (at 10), begun by the stage: rows before it are
  gone through first (`listed` in `begun`), rows after it once what was
  begun says what it came to and leaves handing on to its stage
  (`stage.came` with `hands`, heard `scope: own` by `after`, which goes
  through the rest and moves the card). `std/proposal` does that where
  something is laid after it (`hands: false`); a project's own blueprint
  does it by emitting the event and moving nothing. Nothing is sent back
  from such a column (`work.came` maps `back` to left).
- The card is then handed on by something that keeps nothing and looks
  first (`std/onward`, called by `after`, which keeps how the row said the
  card arrived): a kept step's move can be repeated after a stop, which is
  harmless only where nothing takes the card up at once. Something new
  that hands a card to a column another stage has looks first, as a round
  does (`work.held`).
- Something new a stage does with a card is a segment and a row before it
  is a handler of a stage's text; something new a segment may be told is
  a param it declares.
