# Blueprints: the language and what ships

This file covers the blueprint language as a developer of Mythwright meets
it: the rules every blueprint that ships is held to and the gate or test
that holds each, how a word or an action is described, the formatter and
file names, a project's own blueprints and forks, the plain-words summary,
what is written inside an action's arguments, how the shipped blueprints
are listed, and how steps are named on the graph. Read it before you add or
change anything under `blueprints/std/`, an action's description, or a word
of the language. How to try a blueprint without changing what runs is in
`blueprint-tools.md`; cards, stages and the fleet are in `board.md`.

## Rules for the blueprints that ship

- **No shell script of ours.** A shipped blueprint does the same on every
  system. What it asks of a program is `cmd:` (a program and its
  arguments) or `cli:` (this program's own command line); what a script
  would decide is steps (`ok_codes`, `when`, `allow_failure`). `run:` is
  only ever wholly a param, its user's own command. The `portable` gate
  fails otherwise; its one exception is listed with the reason in
  `xtask/src/portable.rs`.
- **Nothing is worked out in a shipped blueprint.** It says what exists,
  what is heard and what is done. Whatever is decided (whose turn it is,
  what a row evaluates to, whether a card goes round again, the sentence
  that says so) is a building block's: a Rust action tested as a function
  with a table of cases, which the blueprint calls and then obeys.
  - An expression in a shipped blueprint names a thing, asks one plain
    question of it (`==`, `!=`, `in`, `??`, `!`, and `<` or `>` against a
    number written out), or joins up to three such questions with one
    word, all `&&` or all `||`.
  - A function call, arithmetic, `? :`, a sentence built with `+`, picking
    the greater of two names, mixing `&&` and `||`, or joining four things
    belongs in a block.
  - The `plain` gate checks it: the language reads the texts itself
    (`reader::plain`, also offered as `blueprint::plain`) through a test of
    the runner's suite. The list of exceptions (`xtask/src/plain.rs`) is
    empty and stays so. A package that says a number of its own
    (`extensions/<name>/contract/plain.txt`) may only lower it.
  - A project's own blueprint may say what it likes.
- **Laid out by the formatter.** `blueprint::format` is the one opinion. A
  test in the CLI suite runs `fmt --check` over `blueprints/std` and
  `examples`. `cargo run -q -p cli -- fmt blueprints/std examples` fixes
  it.
- **Shipped blueprints only get shorter.** Each one's line count is in
  `contract/lines.txt`; `lines.rs` in the API suite fails on one that grew.
  What was to be added is a block's to decide, a text of its own that this
  one calls, or a row. `WRITE_LINES=1 cargo test -p api the_lines` records
  a shorter text as shorter and a new one at its first length, and never
  records growth. Where growth is needed, `GROW_LINES="why" …` writes it
  and keeps the reason in `contract/grown.txt`.
- **Every event is described where it is emitted**, in its crate's
  `events.rs`, as an action is. A test reads the source for every emit,
  and every node the API and CLI suites start answers for what it
  emitted. `docs/blueprints.md` has a row for each event; the test prints
  the row to add.
- **What a blueprint may depend on is a written contract**:
  `contract/offered.txt`, and `extensions/<name>/contract/` for a package.
  `WRITE_CONTRACT=1 cargo test -p api the_contract` records additions.
  `BREAK_CONTRACT="why" …` is the only way to record a break, and adds it
  to `contract/broken.txt`.
- **A step's `description` is part of what its text means.** Adding or
  changing one takes the text's next version (see the versions files in
  `contract/`). A description is not part of how a text begins
  (`blueprint::outset`), so a running instance takes such a text in place
  or is carried over to it with no step run again; until then it is shown
  as behind.

## Words of the language

- A word of the language is one entry in
  `crates/blueprint/src/vocabulary.rs`: its name, what goes after it, and
  a sentence. The reader accepts it and the editor offers it from there;
  the client is not touched. The client's tests read a copy
  (`ui/src/blueprint/language.json`); a test says when it is stale and how
  to write it again.
- New capability is usually a new **action** (`Registry::add_fn` in a
  crate's `actions.rs`), not a change to the conductor. New behaviour is
  usually a blueprint under `blueprints/std/` that says what exists, what
  is heard, who may and when; what it does is actions.
- **Every word is checked four ways**: a case that reads, is laid out, and
  is refused when written wrongly (`crates/blueprint/tests/fixtures/words`,
  checked by the blueprint suite's `held.rs`), and a blueprint that is
  played, stopped and started (the conductor suite's `held.rs` over
  `plays.rs`). A *play* is a scripted run in the conductor's suite or the
  simulation, stopped at each point and restarted. Which words a text uses
  is read from the text (`vocabulary::written`), never listed. A new word
  fails both tests until its case and play exist.
- **Every word says what it is for and has an example**: a whole
  blueprint written with it (`.example(shown::FETCH)` in
  `blueprint::vocabulary`, texts in `vocabulary/shown.rs`; names an
  expression starts with in `ROOTS`; functions in `vocabulary/called.rs`).
  Every action a person may write in their own blueprint has one too
  (`ActionSpec::example`; the language's own in `catalog/asked.rs`, the
  studio's as rows in `kernel::asked`). The blueprint suite's `shown.rs`
  and the API suite's `exampled.rs` check this. Give a new word or action
  its sentence and example in the same commit. See `blueprint-tools.md`
  for how examples are shown.
- **Nothing is taken out of the language.** Running factories keep the
  texts they were applied with; removing a word, a kind of step or a
  function breaks them on upgrade. What is no longer wanted stays readable
  and stops being used by what ships (`sum()` is still there).
  - One list stays open past the preview: the words nothing that ships
    uses, named in `docs/blueprints.md` (*Words that may be taken out after
    the preview*) and `docs/status.md` (*What is not held yet*). A word is on it
    only if no standard blueprint, example or package writes it; nothing is
    added to it after the preview.
  - Before the preview, the profile words were renamed with no alias
    (`environment.name` to `profile.name`, `environment.set` to
    `profile.set`, `std/environment` to `std/profile`, `system.environment`
    to `system.profile`), recorded as a break in `contract/broken.txt`.
    `environment` now reads what a project has of its own; a text that
    sets the profile's variables and reads `environment` gets a warning
    (`blueprint::renamed`). Nothing after the preview is renamed this way.

## The schema and descriptions for editors

- The language is also given out as a schema for ordinary editors
  (`blueprint::schema`, `mw schema`, `/api/blueprints/schema`), derived
  from the same word list and never written by hand.
  `contract/blueprint.schema.json` is the one written with this program's
  actions; `WRITE_SCHEMA=1 cargo test -p api the_schema_kept` refreshes it.
- The schema may say less than the reader, never more. A test checks every
  shipped blueprint against it, so a kind of value the reader accepts and
  the schema would flag is found there.
- What a project can say about layout is one entry in
  `blueprint::format::SETTINGS`; a test checks the table in
  `docs/blueprints.md` against it.

## File names and YAML

- A blueprint's file may be `<name>.bp`, `<name>.bp.yml` or
  `<name>.bp.yaml` (`brand::blueprint_files`, `blueprint_stem`), so an
  editor treats it as YAML. Never `.yml` alone: a project is full of YAML
  that is not ours. What ships, and a new project's `fleet`, use `.bp`.
- Anything that finds a blueprint by name calls `texts::library::named`
  (also `conductor::library::named`). Where a folder holds two files for
  one name, neither is read, nothing behind stands in (`Layers::load`), and
  the caller is told of both (`Library::refused`).
- Anything that tells a blueprint's file by its name calls
  `brand::is_blueprint_path` (`isBlueprint` in the client), never an
  ending written out.
- The reader accepts a few plain forms YAML does not: a value with `: `
  inside it (`description: A board: its cards`) and a bare `${ }` inside
  `{ }`. The reader keeps accepting them; the formatter writes each in
  quotes, where it is the same value and valid YAML (`parts` in
  `crates/blueprint/src/layout.rs`; layout cases `parted`, `braced`), and
  never removes quotes.
- A test of the API suite reads each shipped text with an independent YAML
  reader (`yaml-rust2`, a dev-dependency of `api` only) and checks it
  matches this program's reading value for value, and the kept schema.
  Something new the reader accepts that YAML does not is a line of `parts`.
- `mw check` warns of those forms in a `.bp.yml` file (`not-yaml`); a
  `.bp` file is told nothing.

## The formatter

- The formatter reads comments from the source by node position
  (`crates/blueprint/src/layout.rs`). A new way of writing a value, or a
  new place a comment can sit, needs a case in
  `crates/blueprint/tests/fixtures/layout/`.
- `format` never returns text that means something else. Never loosen
  that check to make a case pass.
- A value inside braces may begin with what no line value can
  (`{ to: %half }`, `&x`), and a key there with what no first key of a
  list item can (`[{ [x: 1 }]`). When braces are opened into a block, the
  formatter asks the reader whether each still reads the same and quotes
  it where not (`stands` and `key` in `layout.rs`; layout case `opened`).
- `robust.rs` lays out texts generated from what ships; its random choices
  depend on every blueprint there is, so a new or longer text generates
  different inputs. A new failure there is a fault that was already in the
  formatter: fix the formatter, never the text that moved the dice.
- Laying a blueprint out is never an update: rule 1 and `behind` call
  `blueprint::means_the_same`. Nothing may make layout or comments carry
  meaning.
- The client's editor is handed a text from outside as the edits that
  differ (`ui/src/editor/changes.ts`), never the whole text, which would
  throw the cursor to one end.

## Validation messages

- What a step, handler or hook is handed is kept as values that do not
  remember where they were written. A refusal of a name is pointed at
  where it is written by searching the text between the start and end of
  what reads it (`Validator::written` in `crates/blueprint/src/placed.rs`,
  called once in `Validator::expr`); where not found, at the start of what
  reads it.
- Something new that refuses a name in an expression reports it with the
  span `expr` passes on, never the step's own, and offers what was
  probably meant (`close_step`, `close_state`, `did_you_mean`).
- `validate.rs` is at its line limit: new checks go in a file of their own
  as an `impl` of the validator.

## Where a program reads blueprints from

- A program reads this checkout's blueprints, packages and client only
  when run from where cargo built it (`crates/folders/src/place.rs`: the
  folder holds `.fingerprint`). Any other copy, an installed one included,
  uses what is built into it and what is beside it.
- `cargo xtask install` copies, so the factory in use changes only when it
  is run again and the daemon restarted. To try a change against a live
  factory, run a second one from the checkout
  (`target/debug/factory up --profile dev1`).
  `--profile dev --no-build --to DIR --bin DIR` installs into scratch
  folders without touching `target/fast` or a home.
- A script that edits a shipped blueprint and puts it back must `touch` it
  afterwards: the restored file is older than the build, and cargo keeps
  the edited text (`include_str!` of the standard blueprints).

## A project's own blueprints

- A project's own blueprints are kept by the factory in its data folder,
  never in the project's folder (a project is often a repository others
  read): `<data>/blueprints/projects/<project>/` (`folders::kept_at`),
  named everywhere as a project path beginning `@blueprints/`
  (`folders::PLACE`, `folders::in_place`): file routes,
  `project.file.changed`, scans, the Trash, search, fork records, tabs.
  Beside `projects/` are `shared/` (for every project, not built) and
  `extensions/` (what packages bring), kept apart so later additions
  reshape nothing.
- **One seam for project file paths**: `studio::Roots` (`Projects::roots`,
  `file_of`, `place_of` in `projects/roots.rs`). Every file operation of a
  project calls it first; never `dir.join`. A folder in the project
  literally called `@blueprints` is not reachable through the daemon.
- **What a project says** is only whether a folder of its repository is
  read too (`conductor::library::folders`): nothing (`usual`, the place
  alone); a folder (`In`: that folder and the project's top, read before
  the place, a name in both refused); or `none` (no own blueprints, no
  forks). Its top-level `fleet` blueprint is read whatever it says; where
  the place has one too and no folder is said, the place's wins. A new
  project's fleet blueprint is written in the place
  (`Projects::init_fleet`).
- **Data from an earlier version**: a project that said nothing or said
  `blueprints` now reads only the place. The first start releases what
  was vouched for of such folders once (`api::library::settle`, recorded
  under `folders::SETTLED`), vouching each name again as the system and
  taking back what instances recorded about it (`Engine::mend_all`).
- **Moving files in** (`move_in` of `/api/projects/{id}/blueprints`,
  `mw project blueprints <p> --move-in [--from <folder>]`): each blueprint
  and each file it links is copied into the place (`in_repo` walks the
  repository, skipping what a scan skips and dot-folders), refused whole
  where the place has a different file of the same name. The project then
  says nothing, fork records note the new place (`Engine::fork_moved`),
  each is vouched for as the mover, and the repository's files are left.
- Only a repository file is passed over for an untrusted project
  (`Library::passed_over`); the place holds only what was written through
  the factory, so forks are made whether or not the project is trusted.
- The setting is in the factory's small durable state
  (`folders::NAMESPACE`, listed in `studio::copy::KEPT`), held in memory
  (`Folders`, `Projects::folders`), and changed in one write with
  `project.blueprints` (on `journal::HERE`, carrying `was_said` and `was`)
  by `/api/projects/{id}/blueprints` and `mw project blueprints`
  (`projects:write` and `admin`). A folder outside the project is refused.
- **It goes with the project**: a new project starts with nothing said
  (`Placed::anew`, from `Projects::create_with`); deletion for good
  removes it in the same write (`Placed::forgotten`,
  `studio::trash::take_out`). The place on disk moves into a Trash
  entry's folder with the project (`carry_place`) and back on restore,
  and goes with the entry or via `Trash::project_gone`. A factory copy
  takes `blueprints/` whole (`folders::snapshot::BLUEPRINTS`).
- **Nothing writes a folder's name.** Whatever finds, lists, forks,
  follows or vouches for a project's own blueprint calls `Own` in
  `api::library` (`own_files`, `repo_files`, `place_files`) or, below the
  routes, `Library::folder` with `library::folder_of`; messages use
  `folders::write_as`.
- Changing the setting is not an apply: a running instance keeps what was
  vouched for and shows as behind until applied again. Where texts are
  followed, a change acts like a save (`Engine::follow_folder`).
- A test writes a project's own blueprint at `@blueprints/...` through the
  daemon, or into the place on disk (`place_of`). A test that writes
  `blueprints/...` into the project folder sets that folder first and is
  testing the opt-in.
- In the client they are the project's Blueprints in the side panel
  (`views/ownblueprints.tsx`, over `/api/blueprints/used`, which returns
  each with the files it links, `api::library::own_links`). A line makes
  a new one for whoever may write the project's files
  (`newBlueprintLine`; names checked by `blueprintNamed`): it writes
  `@blueprints/<name>.bp` with one no-op step. Breadcrumbs of such a file
  lead back to that list (`showOwnBlueprints`).

## Forks of shipped blueprints

- A shipped blueprint becomes a project's own only by being copied in
  (`own` in `crates/api/src/routes/texts.rs`, `mw own`). What is copied is
  what the project's running tree is held to (`Engine::reached_in`),
  written in `@blueprints/` regardless of folder setting or trust.
- The copy is called a **fork** to people. To the runner it is just a
  project's own file: pins, trust, `behind` and `apply --take` know
  nothing of forks and must not.
- **Its origin is recorded as it is made** (`texts::forked::Origin`,
  `conductor::forked`): blueprint, version, exact text, where written, who
  and when. The text is kept whole under its own name
  (`runs::beside::FORKED`, `<project>\n<name>`), never as a pinned
  revision (those are pruned). It is in the project's conductor's
  journal, copied with the factory (`studio::copy::KEPT`, `KEPT_BESIDE`)
  and moved with the project (`runs::beside::KEPT`).
- Recorded in one write with `project.forked` (`Engine::fork_made`);
  forgotten with `project.unforked` (`Engine::fork_gone`). The events stay
  in the project family, since a new family would be reserved and refuse
  user texts.
- **A record is never wrong**: the file is written before the record.
  Removal marks the record leaving (`Engine::fork_leaving`), removes the
  file, then forgets the record; a marked record reads as none.
- A fork an earlier version made has no record and none is invented; a
  default whose version matches is taken as its origin and said to be
  assumed (`known` in `crates/folders/src/forked.rs`).
- **Where a fork stands is said in its Summary and nowhere else.**
  `blueprint::summary::summary_as` (`summary/forked.rs`) is handed a
  `Standing`:
  - a shipped blueprint says last how it can be changed (a row of
    `segments` via `forked::usual`, what it may be told, rows of a table it
    makes, its linked files, a fork), and first how the project's own file
    of that name stands (`Own`: in use, not yet via
    `Engine::goes_on_otherwise`, or passed over). `mw status`
    (`GET /api/blueprints/stands`) and a save's answer use the same
    sentence (`own_sentence`);
  - a fork says what it came from, how it differs, whether the default has
    moved since, and how to go back and what that loses.
- What moved is described by `blueprint::differs` (`between`,
  `differences`, `lead`); a part is the same if it means the same. A line
  that leads somewhere says where (`Line::opens`); a terminal prints the
  command after it (`hints` in `crates/cli/src/explain.rs`).
- The daemon cannot tell a fork from the default by text, so the caller
  says what the text is (`path` or `ships`, `crates/folders/src/forked.rs`).
  Standing is told only to whoever may read that file. `mw own --say` and
  `mw explain --project <p>` print it.
- Only a table whose making action names a setting argument
  (`Makes::setting`, `.setting("beside")` of `collection.ensure`) has its
  rows offered for change; `summarised.rs` checks exactly the fleet's
  three tables and `tells` qualify. Something new whose rows set what a
  blueprint does sits beside what it is of.
- In the client: Fork is a head button of a shipped blueprint while the
  project has none; the rest is lines of the Summary (`ui/src/forked.ts`,
  `views/forking.tsx`). Going back asks the daemon first what goes and
  what is lost. `ui/scripts/forked.mjs` checks it.
- **Nothing merges.** Not built: taking a later default into a fork and
  moving the record forward; trying a fork in a sandbox and comparing it
  on real work before keeping it. Something new kept of a fork is a field
  of `Origin`; something new said of standing is a line of
  `summary/forked.rs` with a case.

## The summary in plain words

- `blueprint::summary::summary` describes a blueprint from its text and
  what actions and events say of themselves (`Known`: specs, events, what
  each scope allows from `kernel::principal::SCOPES`). Nothing is run; the
  blueprint need not be applied or installed.
- Six parts, each omitted when empty: what it is; what it makes; what it
  does when applied; what it hears and does; what it may do; what it does
  as it ends or is changed.
- Cases: `crates/blueprint/tests/fixtures/summary` (terminal output, and
  `<case>.parts.txt` for the taken-apart form).
  `WRITE_SUMMARY=1 cargo test -p blueprint each_case` writes one; read it
  before keeping it.
- **Nothing lists action names.** A step is described by its
  `description` up to the first full stop, else by its action's `summary`.
  So an action's `summary` is an imperative sentence a newcomer can read.
- What is made is what an action declares (`ActionSpec::making("a table",
  "name")`; a package says `makes:`), a command an argument marked
  `.runs("cmd")`, a secret where the text names one; these are contract
  and schema additions. (`making`, not `makes`, because `.makes(` is how a
  journal is told its tables.) What a made thing is made of comes from
  `ActionSpec::modelled` (`summary/modelled.rs`), in no contract.
- Only the language's own actions are named (`summary/own.rs`); a test
  fails for one with no case. A hole is described by what it names
  (`summary/words.rs`); a plain `when` as a question, others as written.
  No line holds a `${`; `summarised.rs` checks every shipped blueprint for
  holes, action names, raw values and empty headings.
- **One rule for what a step says of itself**: `says`, in Rust and in
  `ui/src/graph/tile.ts`, checked against `ui/src/graph/says.json`. Add
  cases there only.
- A line names things as the graph does (`main/build`, `on.changed/@`)
  and holds no text position, so layout and comments never change it.
- Returned by the check's answer (`summary`, beside the graph) and by
  `POST /api/blueprints/summary`. `mw explain` and the client's Summary
  (`ui/src/views/summary.tsx`, `ui/src/summary.ts`) only render it.
- A line comes whole (`text`, what a terminal prints) and taken apart
  (`Line`: `title`, `label`, `items`, `from`, `only`, `tags`, `named`,
  `by`; `Opens`). Every step, hook and handler has a title. The page lays
  parts out (`ui/src/summarylaid.ts`: `sharedFrom`, `rows`,
  `startedFrom`, `foldsRows`); folds are kept by the tab
  (`ui/src/summaryfolds.ts`). The client writes no sentence but a fold
  count and the words `as` and `from`.
- Something new a summary says is a line of one of the six parts with a
  case; something new an action can say for it is a word of `ActionSpec`
  written into the contract. Standing is four extra parts the caller
  passes (`summary_as`: `change`; `differs`, `moved`, `back`).
- The Summary is where a blueprint page opens for a shipped blueprint and
  for whoever may not change its file (`startsAt`); a check that wants the
  text presses Text first.
- Cost of runs is not part of the function: the page reads measurements
  labelled with the blueprint's name (`costOf`, by each series' `unit`).
  Agent turns carry no such label, so agent spend is not shown there.

## What is written inside an action's arguments

- An argument that is a mapping with words of its own describes them
  (`ActionSpec::within`, `crates/blueprint/src/inside.rs`) as rows: a path
  from the argument down (`*` for a writer-chosen name), a kind (an
  argument type, `each`, or `word` for one of a key's few allowed words)
  and a sentence.
- **Sentences are written once.** An argument's `help` never lists keys.
  `mw actions <name>`, the schema (`inside` in `blueprint::schema`), the
  table in `docs/blueprints.md` (rows printed by the API suite's
  `inside.rs`) and the editor are made from the rows. In the contract they
  are lines no blueprint breaks on (`within <argument>.<path> <kind>`).
- **It describes and never decides; the editor never calls a key
  wrong.** Nothing reads it when a blueprint is read, checked or run.
- A test beside each table reads the source for the words the action
  reads and fails for any mismatch (studio suite `inside.rs`, matching
  `about.get("…")`, `flag("…")`, `said("…")`; `services.rs` in the hosting
  suite). A new word an action reads is a row, in `Within::rows` order.
  The studio's rows are in `kernel::inside`; a crate with room keeps them
  beside the action (`hosting::HEALTH`). Package manifests cannot say
  this yet.
- In the client: `inside` and `insideAt` in `ui/src/blueprint/within.ts`;
  `explain` and `suggest` in `ui/src/blueprint/complete.ts` use them. The
  client's tests read `ui/src/blueprint/inside.json`, refreshed by
  `WRITE_INSIDE=1 cargo test -p api the_copy_of_what_actions_say`.
- Not described: arguments whose keys are the action's other arguments
  (`own`, `said`, `row`), and a form inside a form (`else`, `otherwise`
  of `rows.read`).

## How the shipped blueprints are listed

- A blueprint's name is its identity, so listing renames and moves
  nothing. Under each family (`std`, each package) the side panel shows
  what someone applies and, folded under each, its **parts**: every
  blueprint it runs, recursively.
- **The daemon works this out from the texts**; the client holds no list
  of names. A text names another (`blueprint::names::of`) where it
  attaches, calls or spawns it; where a param defaults to that name; where
  an action it calls reaches it by a declared name
  (`ActionSpec::reaching`, `reaches`: `agent.task` and `agent.consult`
  reach `std/dispatch` and `std/signin`); and where the name appears whole
  as a value in the parsed tree, except a list or mapping param that is a
  setting chosen among (`settings` in `blueprint::names`). A name in a
  sentence, comment, expression, key or `${ }` names nothing.
  `parts`, `tops` and `part_of` are functions of what each names.
- **What no text says**: that a part is also applied by people. Those are
  the lines of `texts::listed::APPLIED`, each with why (`std/plan`,
  `std/pool`, `std/slot`), shown in a folded `misc` group of their family
  (`MISC` in `ui/src/blueprintlist.ts`). Never put a part at the top by
  adding it there, and never name a blueprint in the client to place it.
- `texts::listed::listed` gathers it each time it is asked (also
  `conductor::library::listed`); `GET /api/blueprints` returns `runs` and
  `applied`. A project's own blueprint returns neither and makes nothing a
  part by calling it. A shipped blueprint's summary says what it is a
  part of (`Standing::Ships`, `part_of`).
- **A new shipped text is a part by being named**, or is declared applied
  with why in `crates/api/tests/suite/listed.rs` (`SOMEBODY_APPLIES`; a
  package's `contract/applied.txt`). That test fails for an undeclared
  top, a declared one now named, or an unreached ring.
- In the client `ui/src/blueprintlist.ts` mirrors the daemon's `tops`,
  `parts`, `partOf`, `unreached`, both checked by
  `ui/src/blueprintparts.json`; add cases there only. Drawn by
  `views/blueprintlist.tsx` with `NavItem` and `Fold`
  (`views/navtree.tsx`). A line shows the full name (`std/fleet`); folds
  are kept per reader and stale ones dropped (`foldsGone`). The panel's
  search box narrows without changing folds (`sought`). A check unfolds a
  part's parent before pressing it. `ui/scripts/lists.mjs` checks it.

## Steps on the graph and in runs

- A step's tile shows its `description` up to the first full stop (two
  lines at most) with its key under it, smaller (`tile` in
  `ui/src/graph/tile.ts`); a step without one is drawn as before. The key
  stays what text, runs and logs call it. Tiles of a graph share a height
  (`tileHeight`); line fitting is measured by the browser with the tile's
  font (`measure` in `graph/view.tsx`). A new tile line is a line of
  `Tile`.
- A run's steps use the same rule (`stepSays` in `ui/src/runsaid.ts`),
  with the words of the text the page has drawn and only for a run of
  that exact `revision`; any other run is shown by keys. Never look a key
  up in whatever text is at hand. In the list of runs the text's own steps
  show as `main`; a handler's run is named by its handler (`runOf`).
- A shipped step whose key would confuse a reader gets a `description`
  (which takes the text's next version). `blueprint.mjs` checks it.

## What a blueprint says in a channel

- A blueprint speaks in a channel only through `channel.post`
  (`crates/studio/src/actions.rs`): as the bot its step names with `as:`,
  else as the factory's one bot (`studio::speaker`,
  `Channels::post_speaker`), with the step recorded on the message
  (`said_by`). No blueprint names the speaker; one that needs the bot's
  name gets it from `channel.ensure` (`speaker`). The client shows no
  blueprint, step or run there, only `detail`. Rust's own posts use
  `post_speaker`; nothing new posts as `system`. The bot's name is a
  factory setting in the journal's `kv` (`speaker`).
- **`channel.post` is idempotent.** A redone step finds its message by
  run, step and which doing of the step it is (`Channels::said_already`,
  `SaidBy.doing`, `ActionCx::doing`). Message, attachments and event are
  one write (`Channels::insert`); a message an earlier version wrote but
  never emitted is announced when found.
- Something new an action leaves behind records which step left it and
  looks for that when `ActionCx::again` says the step is being redone;
  only then may it declare `.idempotent()`, else its step fails at a
  restart.
- `record.add`, `board.add` and `record.comment` work the same way
  (`crates/studio/src/left.rs`): their events carry `doing`
  (`Through::doing`), and a redone step searches its run's events in the
  project's journal (`Journal::said_of`) from just before its try
  (`ActionCx::began_at`, `Journal::place_before`). Tables kept as files,
  by a store service or as documents cannot be found this way, so the
  step is refused (`Studio::left`). `twice.rs` in the API suite checks
  it. Something new that adds or comments on a record for a step emits
  `doing` (`left::mark`) and looks with `Studio::left`.

## Packaging

- `cargo xtask package` makes the release for the current system
  (`.tar.gz` for Linux and macOS, `.zip` for Windows). Names, contents and
  the package's page are functions in `xtask/src/package.rs`, with a case
  per system (`xtask/tests/suite/systems.rs`); making is `packing.rs`;
  zip and checksum are the tooling's own (`zip.rs`, `sha256.rs`); tool
  names per system are in `tools.rs`. A page claims only what was run on
  its system.
- `--target` makes another system's package for trying, never a release.
  `--nightly YYYYMMDD` makes a nightly named `nightly`, whose version
  (`package::nightly_version`) is first in the line beside its programs
  (`package::note`) and is reported by the daemon
  (`folders::place::version`, checked in the API suite's `releases.rs`).
  A fix to a released line is a tag on its branch (`release/0.1`); see
  `CONTRIBUTING.md`.
- `THIRD-PARTY-NOTICES.md` is made by `cargo xtask notices` when a
  dependency changes, and committed; packaging only copies it. Rust half:
  `cargo about` 0.9.3 (`--features cli`, `--frozen`, `about.toml`,
  `about.hbs`); client half: `node build.mjs --notices`
  (`ui/scripts/notices.ts`); merged by `xtask/src/notices.rs`. Its head
  records a mark of the shippable crates (`xtask/src/shipped.rs`) and the
  config; `xtask/tests/suite/notices.rs` and `noticed.rs` fail when it is
  stale. The client build's `--held` fails for an unlisted bundled
  package. A licence that asks more than a notice is accepted only per
  library in `about.toml` (`asks_for_source`). Anything built in without
  the bundler or cargo knowing is named in `bundled` (`ui/build.mjs`) or
  `BUILT_IN` (`notices.rs`).
- The icon is one drawing, `desktop/icon/icon.svg`; `node
  desktop/icon/make.mjs` makes every image of it.
