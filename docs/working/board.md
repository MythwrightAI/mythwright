# Cards on a board

This file covers what a card is and what happens to it outside the work a
fleet does: what a card can do beyond being moved (`std/cards`), why
nothing runs for an idle card, how a card's row and its events are kept in
step, the `moved` field, buttons pressed on a card, sending a card back,
what a card cost, assignees, features, pools, packages that bring in work
from a service, and the examples that ship. Read it before changing
`std/cards`, `std/board`, the board in `studio`, or anything that moves,
presses or counts a card. How the fleet works cards (stages, rounds,
reviews, segments and its tables) is in `fleet.md`.

## What a card is

- A card is a row of the board's table. **Nothing is started for a card
  nobody is working on**: not when it is added, not while it waits its
  turn, not while it waits in review for a person. What looks after cards
  is one instance per board (`std/cards`) and one per stage
  (`std/working` and the other stage texts), whose handlers take turns by
  the card (`turn:`, `crates/runs/src/turns.rs`).
- Anything either has to remember about a card between whiles is a `quiet`
  field of the card's row (`asker`, `taken`, `again`), never an instance
  per card and never per-card state in an instance. What exists for one
  card is the work in hand, only while it is in hand: a round, a review, a
  seat. `crates/api/tests/suite/rows.rs` checks that adding cards, and
  cards waiting their turn, start nothing.
- A record the daemon's own code reads by field is a model
  (`crates/studio/src/models.rs`). A new field of a card is one line there
  and one in `std/board`; a test says when they disagree.
- A test that waits for a card to reach `doing` to know work began is
  waiting for its round (see `fleet.md`).

## What a card can do beyond being moved

- That is not in `studio::board`. It is `std/cards`, one instance per
  board, over `record.commented` and the board's own field `question`
  (declared by `std/board`, not the model; `quiet`, so setting it is not a
  history line). The daemon only knows that something was said on a record
  (`Records::comment`).
- A new thing a card can do starts as a handler in `std/cards` that says
  `turn` by the card. What the handler decides is a block
  (`studio::cards`: `board.came`, `board.said`, `board.waits`,
  `board.after`): a function of the row and what was said, with a table of
  cases, handed the columns it distinguishes and naming none.
- `std/cards` remembers nothing in its instance, since what an instance
  remembers is gone when another takes its place and what is asked before
  it listens is not heard. A card asked whether it may begin, and told no,
  records what it waits for on its own row (`studio::cards`: `waiting_for`
  writes it, `waits_on_others` reads it, `board.held` lists such cards).
  `first` in `std/cards` looks again at each card that says it waits on
  others when it starts; nothing waits on having been asked. A card looked
  at again (`after.again`) is told nothing is left only if it says it
  waits; one asked (`after.asked`) is always answered, so two cards
  finished at once free a card that names both exactly once.
- A card counts how many times it was sent back from review (`rounds`,
  counted by `std/cards` on the move and kept on the card). Whatever caps
  a loop between agents reads that and ends by leaving the card to a
  person: a loop between two agents always has a number in it. A move
  that sends a card back for something other than a verdict on its work
  says so (`uncounted` of `board.move`, said on `board.item.moved`, read
  by `board.came`) and is not counted; the only one is a card going back
  to be brought up to date (`fleet.md`, *Reviews and councils*), which
  has a cap of its own. Something new that sends a card back and is no
  verdict says `uncounted`, and is bounded where it is kept.

## A card's row and its events

- **The row is what is so.** A row may exist whose event was never
  emitted: in a journal an earlier version wrote (it wrote, then emitted,
  and a stop could fall between), or for a board not kept in the database.
  The two are reconciled after the daemon starts by the board itself
  (`Board::settle` in `crates/studio/src/unsaid.rs`, called as
  `board.settle` by `std/owed`, which a working stage calls from
  `started`, and by `started` and `first` in `std/cards`).
- A card whose row is in another column than it was last said to be in
  has that move emitted then, late (`late` on `board.item.moved`), by the
  system. From there it is a move like any other, heard once by whatever
  would have heard it. It is done under the record's turn, so a move in
  progress is seen whole or not at all, and it is idempotent. A card added
  with nobody told is emitted the same way (`late` on
  `board.item.created`).
- **Nothing judges a card by what its row looks like.** What was said with
  a move is on the row, written with the move (`models::Moved`), and is
  emitted with a late move; who made it is not known beyond the kind of
  actor. A board with no history of its cards' moves, and nothing said of
  late, is left alone.
- A board is settled once per daemon start however many stages ask
  (`Board::settle` keeps which in memory), looking closely only at rows
  written since a minute before the journal's last event before this
  start. `started` in `std/cards` also asks again each card that says it
  waits on others.
- Something new that is written and then emitted, which something waits to
  hear, is reconciled the same way: from what was emitted, never from what
  the row looks like.
- **A record's row and its event are one write** where its table is kept
  in the journal's database (`studio::whole`: `Say`,
  `Collections::add_saying`, `put_saying`, `remove_saying`, which the
  board and `Records` write through). A history line is written with its
  event and the keeper's place the same way (`Collections::note`). Tables
  kept as files, by a store service, or as documents are still written
  then emitted. A test that wants a row written and never emitted plants
  it as an earlier journal would (`moved_and_not_said`,
  `added_and_not_said` in the API suite's `late.rs`).
- `rows.read`'s `count` is how many rows the table has, whatever `saying`
  asked. Whether any row matches is `steps.<id>.out.those[0] == null`.

## How a card came to be where it is

- A card's row says how it came to be in its column, in the write that
  puts it there (`moved`, a model field only the board writes,
  `Board::moved`): the column it came from, the kind of actor, what was
  said as it was moved, and how many times it has been moved.
- What is done with a card that arrived is decided from that, never from
  which handler heard the move first. The count tells one move from the
  next for whatever must say which it has taken up. A board whose own
  blueprint keeps something else under that name is left with it.
- A round that begins writes which move it was told of
  (`again: [heard, n]`). A card in hand is owed a round for being sent
  back where its row says it was moved there by a move no round has been
  told of; `back` in `std/working` only asks for the card's turn. Rows an
  earlier version left (`again: [back, words]`, `[said, words]`) are read
  as meant.

## Buttons pressed on a card

- What a column offers to press on a card is the rows of a table that is
  an action's (`studio::models::ACTION`, read by `kernel::pressed::Action`)
  and sits beside the board: a fleet's `actions`, which starts empty. The
  board finds it by model and `beside`, never by name; nothing in Rust
  goes by a column's or action's name.
- **Pressing is one door, `Board::press`** (`crates/studio/src/pressed.rs`):
  the route (`POST /api/board/items/{id}/press`), `mw board press`, the
  bot's `board press` and a step's `board.press` all go through it, as
  whoever presses (a key as the key, a step as whoever applied it, never
  whoever applied the fleet). It checks what the row says (`who`,
  `scope`) beside `board:write`, and writes the press on the card's row
  (`pressed`, a field only the board writes) in the write that emits
  `board.item.pressed`.
- Whether a card is held is asked by the stage of its column before it
  begins anything, inside the block it already calls (`work.due`,
  `review.owed`, `work.owed` answer `held`, over `deciding::pressed::held`:
  a function of the rows the arrangement hands each stage as `actions`,
  the card's presses and its move count). A hold is emitted once per
  arrival and kept on the row (`board.hold`, `board.item.held`); the fleet
  tells of it (`held` of `std/told`). The press that frees a card emits
  `released`, which a stage hears as the card arriving (`pressed`).
- Something new a stage begins for a card checks the hold in its block;
  something new that presses goes through `Board::press`.

## Cards moved by people

- A card somebody puts straight into `doing` is theirs: the working stage
  takes up again only a card it had taken (`taken` on the row is its own
  column), so a person's card asks for no turn and no agent is asked
  (`crates/api/tests/suite/byhand.rs`). This is how work done outside the
  fleet shows in hand on its board. `ready` and `review` are the fleet's
  whoever puts a card there.
- A card somebody moves out of `doing` while its agent works on it has its
  round stopped where the stage's row says `moved_away: stop` (the
  default). See `fleet.md`, *Rounds*.

## Sending a card back

- Where a card that is sent back goes is said by a row, and nothing else
  names the column: `fail` of the reviewing stage (`doing` where the row
  says none).
- The client's Send back reads the rows about the board's columns (a
  table that sits beside the board and says `about`; the project's
  `stages` by name only for a fleet from an earlier version:
  `sendingBack`) and sends the card where the row about its column says
  (`backTo` in `ui/src/sentback.ts`, drawn by `views/sendback.tsx`). Only
  where no stage has the column, or it names none, is it `review` to
  `doing`.
- A card sent to the column it is worked from arrives like any card, and
  its row says it was worked from there before (`work.turn`: given to its
  agent again, told what was said). It is counted, by anyone, where it is
  put from `review` into an earlier column (`before` of
  `studio::cards::Columns`, read by `board.came`; `std/cards` hears a
  move out of `review` for that).
- Not checked: `std/cards` names `review` and `doing` itself, so a card
  sent back from another column to anywhere but `doing` is not counted on
  its row; the sending stage still counts its own verdicts (`back` in
  `std/reviewing`), which caps the loop.
- Something new in the client that moves a card for what a stage does
  reads the row the same way (`crates/api/tests/suite/sentback.rs`,
  `ui/scripts/review.mjs`).
- A card its round left for a person says so in its drawer with the
  reason and the column it is taken up again from (`leftFor` in
  `ui/src/left.ts`, with a case for every shape of `again`), only while
  nobody has moved it since (per its `moved`).

## What a card cost

- A card's cost by round is computed once by the daemon (`deciding::spent`,
  pure functions with cases; `api::spent`,
  `GET /api/board/items/{id}/cost`). `mw board cost` and the drawer
  (`views/cardcost.tsx`, over `ui/src/cardcost.ts`) only render it.
- Nothing is stored for it: it reads the card, its board's columns, its
  history and its turns, each by key or index (`Agents::turns_in`, by the
  turns-by-conversation index; checked beside 253 cards in `costed.rs`).
- A card's turns are those whose conversation is the card's
  (`spent::CONVERSATIONS`: `lane:<card>`, `plan:<card>`, `review:<card>`,
  each optionally with a seat; `contend:<card>:<way>`; `judge:<card>`),
  the same words the client reads (`heldIn` in `ui/src/heldin.ts`). A new
  conversation an agent holds about a card is a line of both, or its
  turns cost the card nothing.
- What was decided is what the card's history says (`card.approved`,
  `card.left`, and a move back or to the last column after `card.worked`).
  A turn belongs to the round decided by the first line after the event
  its asking names (`spent::begun_by`: a worker's word is the number of
  the event that gave the card its turn, a plan's `round-<number>`, a
  seat's the review instance `review-<card>-<number>`). Both numbers are
  of the one journal that keeps the card's project.
- Nothing places a turn by time, so the API suite plays it on a stopped
  clock (`spent.rs`). A turn from an earlier version that names no asking
  goes with the turn written beside it (`turns.nth`); where none names one
  they are grouped by order written, which is said (`by_order`).
- Which card an agent's turn is about is read from its conversation by
  one function (`heldIn` in `ui/src/heldin.ts`, with cases), so what an
  agent is said to be doing (`ui/src/doing.ts`), whether it is writing in
  a channel (`ui/src/typing.ts`), a card's cost and an agent's history
  agree. An agent the factory does not start has no turns: what it has in
  hand is the cards given to it that are in hand on its project's board
  (`inHand` in `ui/src/external.ts`). Something new an agent can have in
  hand is a case of `heldIn` and a conversation named for it.
- Something new that asks an agent for work on a card says a word ending
  in the number of the event that began the asking, or it is placed by
  order. A turn of a members-only channel is returned by `turns_for`'s
  rule: figures, not what it chose. It counts and never scores.

## Assignees

- A card names one agent (its assignee). Each stage honours the name only
  where it is one of that stage's role in that project
  (`agents::actions::pick`): whoever planned a card is not whoever works
  it. A name that is nobody's who is up leaves the card where it is.
- Who the drawer offers is one function (`givable` in `ui/src/givable.ts`,
  with cases): agents of the card's own project, then of other projects in
  its workspace, then agents of no project, and whoever already has it;
  never an agent of another workspace's project.
- The daemon refuses what the drawer does not offer, however the card is
  given (`agents::assigned`, called by the board on every change of
  assignee: `Board::gives_with`, installed by the agents' keeper as it
  opens). It is checked only for the change: other cells of a card already
  naming such an agent can still be changed, a name that is no agent is
  left to its readers, and a person can be given any card. It lets nobody
  in and keeps nobody out.
- This is the first thing a workspace bounds. Anything else that is to
  stop at a workspace's edge is checked the same way, in the write that
  would cross it and with a sentence, never only by leaving something out
  of a list. Something new that offers agents for a project's thing calls
  the same function; something new that gives a card to somebody goes
  through the board (`crates/api/tests/suite/assigned.rs`).

## Features

- A feature is a record of a project's `features` and an instance of
  `std/feature` (a card has no instance): bigger work with its own
  pipeline that ends as cards on the project's board (`part_of`, `after`).
  A second board is a table with a board view (`ui/src/views/laid.tsx`).
  Nothing in the daemon knows a feature, and nothing must.
- A feature may gather cards already on the board (`gathers`, or
  `labelled`, which the round writes into `gathers` until it has a plan),
  and a ticket of its plan may be one of them (`card`, its id). Three
  blocks decide this, knowing no feature or card (`deciding::gathered`:
  `tickets.named`, `tickets.fit`, `tickets.placed`): what a planner is
  shown, whether a plan fits, and what each card then waits on.
- A plan is checked against the board before anything is written and is
  taken whole or not at all. Something new a plan may say of a card is a
  case of `fit` first, never a step that writes before `fits` answers. An
  existing card gets `after` and `part_of` and nothing else; it is made
  ready only if not begun (`ready` of `tickets.placed`), never moved back.
- **A plan is accepted by being moved on, and that is checked of whoever
  moved it**, not whoever applied the features: what changing a card takes
  (`actor.holds`, `board:write`), and, unless the features say
  `accepts: anyone`, that they are a person (`who: person`).
- Whether a record was moved on is read from the event (`chose` of
  `record.updated`: what each changed choice now says), never from where
  the record is when the handler hears. A handler acting on a flag being
  ticked uses `flags` likewise. Both are built in one place (`now_says` in
  `studio`'s `records.rs`) for both ways a record change is emitted (a
  record changed, a document written through the daemon); something new
  that emits `record.updated` calls it. A document changed on disk by a
  program emits no `record.*`.
- A field its table marks `said` (`studio::fieldsaid`) is carried in every
  event about a record of that table, under `fields` of the event's data,
  as the record holds it then. It is added in one place every such event
  passes through (`Say::carrying`, called where a record is written,
  removed and restored: `Collections::save_also`, `remove_saying`,
  `trash_saying`, and by direct journal emits), and the event then needs
  what reading the table needs. `quiet` or guarded fields are refused the
  word, long text is cut at `fieldsaid::MOST`. `for:` names such a field by
  path (`fields.sprint`, `blueprint::address::at`). Something new that
  emits about a record goes through `carrying`.
- A plan is taken up by its own handler (`taken`, on `feature.moved_on`),
  asked for again when the daemon was away during somebody's move
  (`started`, while `mover` is kept and nothing is accepted). A second run
  finds what the first made by title among the feature's parts and makes
  nothing twice. Acceptance is recorded only once the cards match the plan
  (`settled`), and nothing after that step is unsafe to repeat. A step
  added to `taken` before `settled` must be idempotent or be found again as
  a card is (`crates/api/tests/suite/featured.rs` stops the daemon at four
  places).
- What a shipped fleet's architect is told (`told` of its row, set when
  the fleet's rows are first filled) says how to order cards already
  there: add a feature that gathers them and move nothing on. Rows filled
  earlier keep their words; nothing rewrites a row. The command line's
  `written.rs` checks a stand-in program following those words.

## Pools

- A place in a pool (`std/pool`) is held by an instance and freed when the
  instance ends (the pool hears `instance.finished`). It asks with
  `wait … say:`, which emits the question once the wait is listening; an
  `emit` then a `wait` loses an answer in between.
- Or it is held by a name (`as`), so something looking after many things
  can ask for one without anything waiting: the pool keeps the name,
  emits `pool.granted`, and the place is freed only by `pool.released`
  with the name. Whoever asks by name gives it back on every way out (in
  `std/working` that is `go`, whose steps before the round may all fail
  without failing the run, and whose last gives back what the round did
  not).
- A name asked again while holding a place waits for its next, as an
  instance does, on purpose: a card put back in its column while its round
  is still ending is taken up again. So a card said to be due twice gets a
  second turn once its round gives the first back, or none if the second
  saying is looked at after the round took the card. Which comes first is
  up to the machine; `rows.rs` checks both orders. What such a turn
  results in is decided by the card's row (`work.turn`): `go` gives it
  back where the card has moved on, or is in hand and owed no round.
  Never have the pool drop a second asking; it cannot tell a meant one
  from an unmeant one.
- Something holding its place is not said to wait (`conductor::pool::told`).
  One that gave its place back and must wait behind someone else is told
  then (`told` in the pool's `released`). One that waited holding nothing
  and is asked for again did wait, whichever asking its place is given for
  (`sat`, handed to `kernel::pool::told`). What something asked about is
  said once, when it asks, and kept by the pool (`abouts`,
  `pool::kept_about`).
- A place is given only to an instance that exists (`pool.turn` drops
  whoever is gone). A name asked `as` is written `key:<name>` and is never
  gone, a restart included, until given back.
- **A pool says when it is listening** (`pool.listening`, from a handler on
  its own `instance.active`, each time it is first there or carried over,
  with who holds and who waits), because a stage may ask before its pool
  listens. What asks by name asks again: `listening` in `std/working` says
  each card in its column is due once more; a waiting name is kept once
  (`pool::turn`); an unmeant turn is given back by the row (`work.turn`).
  An instance waiting for its own place must never ask a second time
  beside its wait; so a review whose session has not begun, and which the
  listening pool lacks, ends and has its stage begin another (`unheard` in
  `std/review`, via `review.due` with `again`). Something new that asks a
  pool from a handler does one of the two; something that asks in its
  steps is started once its pool exists. Checked in
  `crates/api/tests/suite/listening.rs`, the `blocks` suite's `pool.rs`,
  and the `BroughtUpOver` scenario.
- A step that hands on `...: ${params}` or `...: ${state}` hands on all of
  it, later additions included; the receiver must accept them (a pool's
  state is what `pool.turn` takes). A copy of `std/pool` that keeps more
  names again what `pool.turn` takes: who holds, who waits, why it is
  paused, what each asked about.
- Who a pool gives a place to is `conductor::pool::turn` (`pool.turn`); the
  order of waiters is the pool's `order` (`first`, `last`, `priority`:
  `kernel::pool::Order`, cases in the `blocks` suite's `ranked.rs`), set
  for a stage by its row (`next`). `priority` is a number each asker gives
  (`priority` of `pool.asked`), kept by the pool (`priorities`,
  `kernel::pool::ranked`, `kept_priorities`). A card's priority is where
  the board's `priority` field (a choice) stands among its words now
  (`kernel::ranked`, returned as `ranked` by `board.get`), read by
  `std/working` and `std/review` each time they ask and by nothing in
  Rust, so it is not a model field. A number written there by an earlier
  version is converted to a word when the board is declared again
  (`studio::reworded`, `record.reworded`). The pool knows no record.
  Something new that asks a pool for a card gives its `priority` the same
  way.

## Blocks know no card

- A block is called with an `item` and knows no card: `place.*` make, keep,
  send, merge and clear a place for a piece of work (`places`), `rows.read`
  reads any table (`studio::rows`), `pool.turn` says who is next,
  `review.tally` what votes come to. What the work is, what follows what,
  what failure leads to and who is told are the blueprint's. Something new
  a block would need a card, stage or fleet for is a blueprint step or an
  argument the blueprint passes.
- An agent is asked by writing or saying, and a blueprint does the rest:
  an architect writes a plan and `std/plan` adds its cards; a reviewer is
  shown what changed. An agent held to `read` or `edit` may be unable to
  run commands, so instructions that make it run one work only for their
  author.

## Packages that bring in work from a service

- Something kept in two places (a row here, its counterpart at a service)
  is reconciled by what the two last agreed on, recorded each time by
  whoever keeps them in step, never by how the two stand now
  (`deciding::sides`, called as `sides.cross`; what has not crossed yet is
  `sides.owed`). What was sent is what is agreed, so it never comes back as
  a change, and a call and a timer reach the same result. Whose changes on
  the other side are taken (`takes`: everyone's, a trusted writer's,
  nobody's) and what happens to a held one (`held`: `waiting`, `take`,
  `keep`) are arguments and cases of the block; whether the other side is
  trusted is the package's action's to say. Something new that crosses is
  a field handed to it; something new decided is a case there, never an
  expression in a package blueprint.
- A package blueprint that brings in what somebody outside wrote
  (`github/issues`) steers no agent by it unasked. What it says on a card
  goes through its own step, and `std/cards` takes nothing a blueprint says
  as an answer (`through`). Where relayed words may answer, the blueprint
  calls `board.said` for those it was told to trust and emits
  `card.answered` as `std/cards` does; its `answered` handler does what
  `said` of `std/cards` does and is changed with it (the package's case in
  `extensions/github/examples/` fails if they diverge).
- A card's title and body are what an agent works from: an outside change
  comes in only as the blueprint's setting says, and a held one says so
  once on the link's row and on the card. Of a service call, only which
  thing it was about is taken, and the thing is read again as it is. Its
  own events carrying who did something (`ticket.labelled`, `ticket.said`)
  are heard only from its own tree (`scope: own`).
- Something a package blueprint does at a service that cannot be done
  twice (an issue made, a comment) is refused after a stop in the middle,
  done or not. So what is made carries a mark it can be found by; the
  blueprint looks for the mark after the step however it ended (`there`),
  does it again only if absent (`again`), and reports trouble only if
  nothing carries the mark (`make` and `say` of `github/issues`). Cases
  play the three places (`stopped_at`, `stopped_inside`, the look). Adding
  a card is idempotent, so needs no mark; that a card is being made is
  still recorded first (`bringing`).
- A blueprint that reports in a channel ensures the channel as its steps
  run (`channel.ensure`, as `std/fleet` does), and nothing rests on the
  report: a posting step allows failure. A handler whose event comes from
  outside (a caller) says `outside: true`; a package's case fails if
  applying any of its blueprints warns (`crates/cli/tests/suite/packages.rs`).

## Examples that ship

- An example that ships is run by a test or a check, or it goes stale
  unnoticed. A test in the API suite fails for one nothing runs
  (`examples.rs`); a new example is applied in a test that says what it
  does.
- `examples/research` is put whole among a project's `blueprints/`, so its
  blueprints are `research/ask` and `research/answer` and link the
  `prompts/` beside them (`research/prompts/answer.md`); moved or renamed,
  they lose their files. It is the first run's experiment
  (`docs/first-run.md`, step 9, and the README).
- A new user has a package with no `examples/` and no checkout, and only
  the daemon knows where a project's folder is. So an example the notes
  ask someone to try is built in (`api::examples::OFFERED`: name,
  description and every file, checked against the folder by a test) and
  put into a project by the daemon (`mw example add <name> --project <p>`,
  `POST /api/projects/{id}/examples`), written through `Projects::write`
  as the requester, who vouches for its blueprints. It is idempotent, and
  writes nothing where a file of it exists with other content. Never tell
  anyone to copy files into a project folder by hand. Such an example is
  checked against the `plain` rule by a test.
- The commands the notes give for the first run's experiment are read out
  of `docs/first-run.md` and the README and run as written, by a copy of
  the program in its own folder (`crates/cli/tests/suite/written.rs`), with
  the stand-in program and the built-in assistant. `researched.rs` checks
  more of the experiment and is changed with them.

## The first run

- With all six steps done, the Setup Guide says what is next (`whatNext`
  in `ui/src/next.ts`, with cases; drawn by `views/home.tsx`): the board,
  the table whose rows say what the fleet's agents run on (found by what it
  declares, never by name), the example put among the project's
  blueprints, and Settings for inviting a device. Help's menu reopens the
  guide (`views/help.tsx`). A line is added only where pressing it does
  something.
- A package carries the first-run notes (`FIRST-RUN.md`,
  `package::notes`): `docs/first-run.md` as it stands under a head saying
  how a command is typed on that system, with links to pages the package
  lacks left as words. The text is not rewritten per system.
- Forgetting a project stops nothing and is not refused for what runs (a
  project can be let go and taken up again with its fleet): the answer
  lists what was applied for it and is alive, its agents that are up and
  its channels (`still_running` in the API's `routes/work.rs`), each only
  to whoever may read it, and the command line prints the command that
  ends each.
- A start applied by hand for a project lacking what it works on (a fleet
  without a board) is not refused: the apply's `said` says so with the
  command that fixes it (`usual::lacks`), since a fleet up before its board
  works the board once it exists. The built-in assistant names the board
  before the fleet.

## Tests around stand-ins and rest

- A test that stops a daemon while the stand-in program
  (`crates/hosting/standin`) has work waits for it to say it began, where
  the test listens (`restarted.rs`, `Held::until_asked`; the stand-in's
  `tell`, read by `Listening`); `agent.turn.started` is emitted a moment
  before the program exists.
- A test that lets a held stand-in go more than once waits between
  releases for what the first went on to do. A hold is a connection per
  waiter (`crates/hosting/standin/holds.rs`).
- A fleet's apply having answered is not the fleet at rest: stages may
  still be starting their first looks. A test that checks nothing ran for
  something tells by what set each run off (`RunInfo.event`), never by
  counting runs. The simulation waits for a factory it set up to be at
  rest before a play begins (`rested` in the API suite's
  `simulation.rs`). A play is a scripted run, stopped at each point and
  restarted.
