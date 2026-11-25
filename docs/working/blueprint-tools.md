# Trying a blueprint without changing what runs

This file covers the tools that let somebody try a blueprint's text without
touching what is running in the factory: the Testing menu on a blueprint's
page, Preview, the band under a graph, the Reference page with its examples
and scenarios, sandbox tests (daemon side and client side), and side by
side. Read it before you add a way to try a blueprint, change how a
sandbox runs, or add a word or action that should be shown in the
Reference. The language itself and the plain-words summary are in
`blueprints.md`.

## The Testing menu

- Everything that tries a blueprint's text without changing what runs is
  one head button of its page, **Testing**, whose menu is `RowMenu` (as
  the gear's is). Four lines in one order, each with a short note under
  it: Preview, Sandbox test, Side by side, Compare on real work
  (`testingLines` in `ui/src/testing.ts`, with cases; drawn by
  `views/testing.tsx`).
- A line the reader may not use is not shown; one that cannot be pressed
  right now is closed and says why. The menu shows what was true when it
  opened. Stop, Fork and Apply stay separate head buttons.
- Something new that tries a text without changing what runs is a line
  there and a case of `testingLines`, never another head button.
- A check presses Testing and then the line (`pressTesting` in
  `ui/scripts/testing.mjs`), after waiting for what the line waits on (the
  text checking).
- Any `RowMenu` hears Escape before anything under it, so one Escape
  closes the menu and not a drawer beside the page as well.
- Testing is in the head from the first drawing. The tab's key is read
  when a line is pressed, not before, so nothing shifts under the
  pointer.

## A blueprint page beside an open drawer

- While a drawer is open, a blueprint's page lays itself out in the room
  left of it (`useBesideDrawer` in `views/besidedrawer.ts`, over
  `clearOfDrawer` in `ui/src/drawer.ts` with cases), so Testing and Apply
  stay pressable with Preview open. The page sets its own `padding-right`
  and its single column (`.blueprint-page`) is that wide, so its head
  wraps as on a narrow screen.
- The drawer's position is read from its laid-out box (`offsetLeft`), not
  as drawn, so the page is laid out once as the drawer opens and once as
  it closes, never while it slides. It follows the drawer with a
  `MutationObserver` and a `ResizeObserver`.
- Where less than `LEAST_BESIDE` would be left (a phone) the drawer lies
  over the page as before.
- `blueprint.mjs` checks it by the boxes (`ui/scripts/besidedrawer.mjs`).
  Another page with something to press at its right while a drawer is
  open calls `useBesideDrawer` the same way.

## Preview

- Preview is made from the blueprint's text and written for no particular
  blueprint (`ui/src/preview.ts`, with cases; drawn by `views/preview.tsx`
  in the frame's one drawer). It is opened from Testing by whoever may
  read blueprints.
- Each part of the Summary is a section in the daemon's words. What a
  section plays on the graph is worked out from the graph the same check
  returned, so a shipped blueprint, a fork, a project's own and an unsaved
  text preview alike, and the preview follows the text as it is edited.
  Nothing is asked of the daemon beyond the check. A text that does not
  check has no Summary and Preview is closed, saying why in the check's
  words.
- What each part plays:
  - what it does when applied: steps by `layer` (what waits on what),
    one layer together, a task while its insides run, a step's `when`
    asked first as written;
  - what it hears: each handler's event arriving at its tile, its `when`,
    then its steps in turn;
  - what it does as it ends: each ending by the word the daemon's summary
    gives its line (`ending` of `blueprint::summary::Line`: `success`,
    `fail`, `finish`, `update`, `stop`; `endedAs`), then what that line
    does. The language has no `on_timeout` for a whole blueprint, so no
    ending says `timeout`;
  - what it is: the whole; any other part: each line that is about a tile.
- A new ending of the language is a word of `ending`, a case of
  `endedAs`, and a line of the endings case in
  `crates/blueprint/tests/fixtures/summary`. Something new a part can play
  is a case of `framesOf` with its line in `preview.test.ts`.

## The player

- **The player knows no summary** (`ui/src/player.ts`). A `Frame` is
  which tiles are lit and how, which line it is of, what is said and how
  long it lasts. `frameAt` says which frame shows after a time; `stillOf`
  what frames become with nothing moving. A replay of a recorded run can
  hand it frames of its own events and be drawn the same way (`usePlaying`
  and `Controls` in `views/player.tsx`). Something new that plays a graph
  hands the player frames.
- **A part plays once**: from its first frame when its section first
  becomes the one being read, then held on its last frame. Each section
  keeps its playhead (`Head`), so scrolling away and back leaves it as it
  was; new frames (the text changed) play once again from the start.
- Under the graph are Play/Pause, previous and next frame, and a track of
  the part's frames (pressed or dragged). With the drawer holding the
  keyboard, Space plays and pauses and the arrows, `,` and `.` step. A
  line a frame belongs to is a button that moves the playhead to its first
  frame, paused, marking the line `aria-current`. Each is a function with
  cases (`begun`, `playedFrom`, `pressedPlay`, `stepTo`, `trackAt`,
  `lineAt`, `keyed`); while paused the playhead, not the clock, decides
  what shows.
- Timing uses `requestAnimationFrame`, never a timer. Playing stops at the
  end, on pause, when its section stops being the one read, when handed
  other frames and when the drawer closes. The section being read is found
  by an `IntersectionObserver` (the first crossing a band one pixel short
  of the top edge).
- The frames array is the same object across drawings (`NONE` for none);
  a new empty array each drawing would restart playback forever.
- Where reduced motion is asked for nothing plays by itself; each section
  shows what it lights, still (`useStill`), until someone steps.
- The graph follows what is happening (`focus` and `scene` of
  `GraphView`, `around` and `fitBox` in `graph/layout.ts`), gliding by
  CSS and never zooming larger than drawn, until the reader moves or zooms
  it; then it is theirs until another part is read or Fit is pressed.
- `ui/scripts/previewed.mjs` checks all of it, reading each frame as it is
  shown with a `MutationObserver`, never by waiting a fixed time.

## What is said under a graph

- Text under a graph and the buttons that move it are a band of their own
  under the map (`.canvas-band` under `.canvas-map` in `graph/view.tsx`),
  for every graph, so they never cover tiles or group names. The map is
  what is fitted and dragged, measured by its own box (`box`).
- Something new said of a graph, or a button for it, goes in the band.
  `legendCrossed` (`ui/scripts/legended.mjs`) checks it by the boxes in
  `previewed.mjs` and `blueprint.mjs`.

## The Reference page

- The Reference is generated, never written (`ui/src/reference.ts`,
  `views/reference.tsx`): one page, in a tab, of everything a blueprint
  can say, as the daemon describes it to an editor
  (`/api/blueprints/language`: words, actions, events). **No sentence of
  it is written in the client.**
- Its model is one function of that answer (`reference`): three parts
  (`words`, shown as Keywords; `actions`; `events`), each a list of
  entries holding sub-entries (a mapping's words, an action's arguments,
  keys inside an argument at every level), rows (allowed words, what
  follows a name, what it gives, what it carries) and marks (needed,
  needs, makes, runs, reaches, who hears it). The view draws only from the
  model and writes only the few words naming each mark.
- `reference.test.ts` walks everything the daemon returns, knowing no
  field by name, and checks every name and sentence is on the model.
  **Something new the daemon describes appears on the page with nothing
  done, or that test fails.** What the page lacks is added where the
  language is described, not in the client (e.g. `after` of each field,
  `blueprint::vocabulary::after`). Extra fields are read through
  `ui/src/blueprint/told.ts` (`makes`, `runs`, `hears`, `after`).
- **Each entry has a name, which its address ends in** (`wordId`,
  `nameId`, `functionId`, `actionId`, `eventId`): `word/<mapping>/<word>`,
  `name/<name>`, `function/<name>`, `action/<action>/<argument>/<key>…`,
  `event/<topic>`. The tab is one whatever it is opened at
  (`{ kind: 'reference' }`; `showEntry` in `ui/src/referenced.ts`, used by
  addresses, the editor's card and the page's own marks).
  `/reference/<entry>` opens the page at the entry, unfolded, marked and
  in view. An entry that does not exist is found as far as its nearest
  parent (`pathTo`); one that names nothing leaves the page as it is.
- Search (`found`) keeps entries that contain every typed word in a name
  or sentence, plus their parents, counts each part, and says what to
  unfold (`open`). Top-level things are single lines until unfolded, so
  the page draws at once. An asked-for entry is scrolled into view only
  after the page has unfolded for it (`unfoldedFor`, then `brought`).
- Ways in: the link on the editor's card for a pointed-at word (`entry`
  of what `explain` returns, for a word, action, argument, key, event
  topic or expression name; a test checks each is a real entry), a line
  above the side panel's blueprints, and a line in Help's menu.
- The page is a lazily loaded part of the client (`lazy` in
  `ui/src/workbench.tsx`), reads the language once (`blueprintLanguage`)
  and again on `extension.added` or `extension.removed`, and follows
  nothing else.
- Something new described that is not a word, action or event is a part
  made in `reference`; something new an entry says of itself is a mark or
  list in the view; a new way of naming an entry is a case of `partOf` and
  of the grammar. `ui/scripts/reference.mjs` checks the page against the
  daemon.

## Examples

- **Every word says what it is for and has an example**: a whole
  blueprint written with it (`.example(shown::FETCH)` in
  `blueprint::vocabulary`, texts in `vocabulary/shown.rs`, shared where
  words are best seen together; expression roots in `ROOTS`; functions in
  `vocabulary/called.rs`). Every action a person writes in their own
  blueprint has one too (`ActionSpec::example`; the language's own in
  `catalog/asked.rs`, the studio's as rows in `kernel::asked`).
- The blueprint suite's `shown.rs` fails for a word whose sentence is too
  short or just a name, with no example, or whose example does not read,
  is not formatter-laid-out, or does not use the word. Every example must
  check with no warning against every action; an action with none is
  listed with why in the API suite's `exampled.rs`. Give a new word or
  action its sentence and example in the same commit.
- An example is a project's own text and still plain, and in the
  language's crate names no card, board or fleet (the `general` gate
  reads it). It is not part of the contract.
- Where shipped blueprints use each word, name and action is computed,
  never written (`blueprint::used`: shortest texts first, five at most,
  the rest counted), returned by `GET /api/blueprints/language/used` and
  read with `readUsed`. A place pressed opens that blueprint at the line
  (`openTab` with `line`).
- An example is coloured by the editor's own reader with no editor
  (`ui/src/blueprint/coloured.ts`), at the top of an unfolded entry; the
  editor's card shows it under the sentence (`exampleAt` in
  `complete.ts`).

## Scenarios

- A word that is hard to grasp from a sentence is **shown working**: a
  scenario is an example run in a sandbox with a few acts, plus a sentence
  or two saying what to notice (`blueprint::vocabulary::SCENARIOS` in
  `vocabulary/scenarios.rs`; returned by
  `GET /api/blueprints/language/scenarios`, read with `readScenarios`).
- Acts are written as a sandbox takes them, in order; the sandbox starts
  with the text not applied and applying it is one of the acts, so what
  must exist first (a channel it posts in) is made first.
- A scenario is offered wherever its example is shown (`scenario` of an
  entry). Run it opens the frame's one drawer (`views/scenario.tsx`),
  which plays the acts and draws the result with the Sandbox view's own
  parts (`Happened` and `Foot` of `views/sandboxed.tsx`, never a second
  drawing); the foot says in the daemon's sentence that it was a sandbox
  test.
- Its sandbox is kept beside the tabs' (`sandboxkept.ts`) under a fresh
  key each time it is opened (`scenarioKey` in `ui/src/scenario.ts`) and
  ended when the drawer is closed or given something else; one the daemon
  made after that is ended as it answers.
- Every scenario is run by the API suite's `scenarios.rs`, which checks
  what its sentence claims against the sandbox's answer; a new scenario
  adds its checks there. A word with no scenario has no button. `durable`
  has none (it shows only when a daemon stops mid-run, which a sandbox
  does not), nor has `scope` (a sandbox has one project).
  `ui/scripts/scenarios.mjs` checks the drawer.

## Sandbox tests: the daemon side

- A sandbox test runs a blueprint in a factory of its own that is handed
  nothing live (`crates/sandbox`, `api::sandboxes`, `mw sandbox`; *Run in
  a sandbox* in `docs/architecture.md`).
- The crate stands beside `agents` and on nothing above `deciding`. It is
  handed the text, a project's own texts and the packages' as read, what
  each action and event says of itself, and the standard blueprints
  (`sandbox::Made`, built in `api::sandboxes`), and never an `App`, a
  journal, a studio, agents, hosting, a gateway, a secret or a key. A test
  of its suite reads its source and manifest for any of those (`held.rs`).
  Something new a sandbox needs from the daemon is a plain value in
  `Made`, never a handle.
- Each action in a sandbox is one of three kinds, decided where it is
  registered:
  - the real one, handed nothing (`blocks`, `deciding::register`,
    `hosting::register_blocks`);
  - the real one over the sandbox's own journal (`Studio::register_kept`:
    every studio action except those in `outside` of its `actions.rs`);
  - a stand-in under the same description, for everything else.
- **A new action that reaches outside the tables** (a program, a project
  file, an agent, a service, another device, the network, a secret) **is
  registered where its crate registers outside-reaching actions**, and is
  then a stand-in in a sandbox automatically. One that only computes or
  only keeps rows goes through one of the first two entrances and is
  added to `AS_THEY_ARE` in the API suite's `sandboxed.rs` with why, or
  that test fails.
- What a stand-in answers by default is a word the action declares
  (`ActionSpec::stands`), given only where an empty answer would stall or
  mislead the shipped texts (a command, a device check, which fleet agents
  are present, an item's place answered as in a non-repository project).
  What answers for such an action is chosen in one place
  (`sandbox::outside`) and recorded in one record (`stood::Record`).
- The sandbox's studio is opened with `Studio::open_within`, which starts
  no program (a project is no repository there) and refuses a table kept
  anywhere but its journal (store service, files, library documents).
  Something new in `studio` that starts a program or sends a request
  calls `Projects::is_within` first; `talk` does neither.
- The sandbox thread's runtime has a timer and nothing else, on purpose.
  Files go under `sandboxes/<id>/` in the data folder only, count against
  a byte limit as they are written (`Logs::written`), are removed when it
  ends and all at once when a daemon starts, and are never copied.
- Besides tables and channels it returns the made-up person's inbox (the
  only one there): `inbox.tell` is real there, and a handle names the
  sandbox's own account.
- An event an action emits naming no run is attributed to a run
  (`offered::Doing`, `shown::SaidFor`) by order: the one run with an
  action at work, else the one whose instance it names, else the one
  whose action is of the event's family; otherwise it belongs to no run
  and is printed separately. What the factory emits as the system (a line
  of a record's history) is never attributed to a run.
- **Rest is computed, never waited for by a clock** (`sandbox::rest`): the
  runtime has nothing left to run (`on_thread_park`), no real action is
  mid-work (each counts itself: `offered::Counted`), and every database,
  asked for a piece of work queued after everything else, did nothing
  else, twice in a row (one quiet round is not enough, because a piece is
  counted done a moment before its caller is answered). Something new a
  sandbox runs that is woken by neither another of its tasks nor a
  database answer (off-thread work, a wall-clock timer) is counted there,
  or an act is answered mid-work; this is why no table is kept as files
  there.
- The API suite checks a factory in use is unchanged row for row while
  sandbox tests run every shipped blueprint and every action
  (`sandboxed.rs`). A table the daemon writes by itself meanwhile is a line
  of `ITS_OWN` there with why; nothing else is let through.

## Sandbox tests: the client side

- In the client a sandbox test is a view of a blueprint's page
  (`ui/src/views/sandboxed.tsx`, `sandboxacts.tsx`, with its own sheet),
  run by Sandbox test under Testing for whoever may read blueprints, on
  the text as it stands on the page, saved or not.
- It has three parts sized by the view's width: what is done to it, what
  happened, and a foot saying what this was. It computes nothing: what
  happened is the daemon's `timeline` grouped by run, the lit tiles are
  what the text's own instance did, and the difference between two runs is
  computed from the two answers (`ui/src/sandbox.ts`, with cases).
- **The foot is the daemon's own sentences** (`said`, `stood_said`, with
  each action stood in and how often). Nothing that shows a sandbox words
  it for itself; `mw sandbox` prints the same.
- **It reads after each act.** A sandbox's events are on no stream, on
  purpose: the page asks one thing at a time and reads the result
  (`ui/src/sandboxed.ts`), showing that it is working meanwhile. Something
  new a sandbox reports is returned with its answer, never put on
  `/api/events`.
- **It is kept by the tab's key** (`ui/src/sandboxkept.ts`): which sandbox
  the daemon holds for the tab, the latest and previous results, and what
  it was run with (what starts first, what the text was told, each act
  since), so the view comes back intact and running again replays the
  same acts against the current text. It is released, and the daemon told
  to end the sandbox, where a tab's other kept things are (`closeTab`,
  `lose`, sign-out); it is not kept over a page reload, after which the
  daemon ends what nobody asks about. A button press is kept with it and
  spent when the view shows, so one press runs once.
- **Nothing of a sandbox is drawn outside its view.** Instances, runs and
  records named there are plain words, never `Running` or `WithRunning`
  (which would open a factory tab for an id not in the factory). The side
  panel, Activity, the Members page and the inbox read the real factory and
  show nothing of it.
- Forms are made from what the daemon describes (an event's topic and
  data, an action's arguments, what a start needs first); manual acts are
  the list the sandbox returns (`does`). A stand-in is answered as usual,
  to fail, with written output (checked as JSON before sending), or all
  stand-ins are told to answer as usual from then on (`answers`, which a
  test can start with, since the shipped fleet asks stand-ins five things
  before it is up).
- The project is never asked: the daemon gives the sandbox's one
  (`UNASKED`). What starts first is the start whose blueprint is the text,
  or a project's own text of the same name without its folder (a
  project's `fleet` calls `std/fleet`).
- A tile's mark (`badges` of `GraphView`) shows how a step ended without
  relying on colour; a new outcome is a word of `LIT` with its mark and a
  line under the graph.
- A check waits for an answer by the count the page keeps
  (`data-answered` on Testing), never by the view being idle.
- Not built: stepping one step at a time (an act runs to rest), and
  stand-in answers beyond the usual (no action declares any).
  `ui/scripts/sandboxed.mjs` checks all of it.

## Side by side

- **Side by side** plays the same acts against several texts of the
  blueprint, each in its own sandbox (`POST /api/sandboxes/beside`,
  `sandbox::played`; an answer given as usual is each sandbox's own),
  laid out as a comparison is (`comparisons::beside::sandboxes`: columns
  in the order asked, rows by kind, counts and never scores).
- What the runner measures of its own work is no row
  (`ITS_OWN`, e.g. `handler.event`, which can differ between two plays of
  the same text): only what ran (`run`, `step`) and what the text's
  actions measured are laid out.
- The texts are what the page knows of its blueprint (`offered` in
  `ui/src/sidebyside.ts`): as it stands, as saved, the default, and each
  project's fork from `/api/blueprints/stands`; a text identical to
  another gets no second column. What is played is the tab's sandbox test
  plan.
- Its sandboxes are kept by the tab beside the sandbox test's (`besides`
  in `sandboxkept.ts`), ended when played again and wherever the tab's
  sandbox is released.
- A column is drawn with the Sandbox view's parts (`Happened`, `Foot`) and
  its buttons are the page's own (Apply; `fork` of `useForking`). Compare
  on real work starts a comparison with each text as a way that sends its
  text (`Way.source`, kept as `sent` by revision, never followed), since a
  fork's stand-in name and an unsaved text have no name where the work
  runs.
- With only one text it says so, and says nothing about how many there
  are until it has read the saved file, the default and the forks. A press
  is then answered by that (counted as answered, never left pending), with
  a line that makes a second text where the page can (the text changed
  here, played as it stands beside as saved; Fork) and words where it
  cannot (`standing` in `sidebyside.ts`, with cases).
- `ui/scripts/sidebyside.mjs` checks it.
