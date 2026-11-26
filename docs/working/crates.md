# Crates: what each holds and where new code goes

This file lists the crates of the workspace in dependency order, says what
each one holds, and says where something new belongs. Read it before you
add a module, move code between crates or propose new structure. The
workspace-wide rules that shape crates (one-way dependencies, size
ceilings, one test binary per crate) are at the end.

## Dependency order

- Dependencies point one way. A crate may use the crates before it and
  never those after it:
  `brand`, `link`, `reader` → `blueprint`, `kernel` → `journal` → `runs`,
  `people`, `manual`, `windowed` → `texts`, `environments` → `conductor`,
  `projects` → `blocks`, `talk`, `pages` → `workspaces`, `search` →
  `comparisons`, `hosting`, `studio` → `deciding`, `trash`, `places`,
  `told`, `outline`, `topics`, `extensions` → `agents`, `sandbox` →
  `gateway` → `folders` → `setup`, `served` → `api` → `cli`.
- The `layers` gate checks this, dev-dependencies included: a crate may
  not take a crate above it even for its tests.
- `desktop` is a workspace member but not a default one; plain
  `cargo build` and the gates skip it (`cargo build -p desktop`).
- When code moves down into a new crate, the crate it left usually keeps
  offering it under its old path (`blueprint::expr` is `reader::expr`,
  `studio::Projects` is `projects::Projects`, `agents::adapter` is in
  `extensions`). Old paths still compile; new code may use either, but
  new modules go in the lower crate.

## The crates, in order

- **`brand`**: the product's name and the values derived from it, read
  from `brand.toml` (`brand::DISPLAY`, `brand::CLI`, `brand::env_var`,
  `brand::SUPPORT`, blueprint file names). Only `brand.toml` and Markdown
  may spell the product's name.
- **`link`**: what a link between installations is made of (the
  installation's secret, its keys, the Noise handshake, sealed frames).
  Stands on nothing else here. `gateway::own` and `gateway::sealed`
  re-export it.
- **`reader`**: the language as it is read, needing nothing of a checked
  blueprint: the text as a tree with positions (`syntax`), diagnostics
  (`diag`), and what is written inside values: expressions (`expr`),
  templates with holes (`template`), durations (`duration`), times of
  day (`at`), and whether what a text works out is plain (`plain`).
  Offered as `blueprint::syntax`, `blueprint::expr` and so on. A new
  function of the text alone, or of one written value, goes here.
- **`blueprint`**: the language as checked: the model, vocabulary
  (`vocabulary.rs`, one entry per word), catalog, validation, summary,
  formatter (`format`, `layout`), schema, `differs`, `parts`, `names`,
  `unheard`. The summary and formatter stay here because they need the
  model, catalog and vocabulary.
- **`kernel`**: plain values and functions of plain values, standing on
  nothing in the workspace (not even the language), each with a table of
  cases in its suite: `Clock`, `principal`, `pool`, `why`, `git`,
  `gathered`, `sides`, `rows`, `tickets`, `first`, `pressed`, `cards`,
  `addressed`, `given`, `walked`, `window`, `outline`, `go`, `flags` and
  more. Because it cannot use the language, a block's answer words are
  written out there (`Fit`, as `blueprint::came!` writes them).
- **`journal`**: events and the databases: one thread per database,
  writes with the events they emit (`Journal::write`, `together`), the
  giver of event numbers, places across journals (`every`), migrations
  (`steps`), commit gathering, checkpoints, copies and moves of journals.
- **`runs`**: what the runner keeps on disk (its tables, the shapes read
  from them, each write with its events) and the runner machinery that
  needs no engine once handed a way to start a task (`runs::Hands`,
  which `Engine` implements): event delivery to handlers (`delivery`),
  turns taken by key (`turns`), work done whole once begun (`whole`),
  what a run holds back until its next write (`held`), stops under way
  (`stops`), what a run left under way results in (`left`), and what is
  kept beside the rows (`beside`). `conductor` re-exports it
  (`conductor::InstanceInfo`, `conductor::copy`). Something new of the
  runner's that needs nothing of the engine but a started task goes
  here first.
- **`people`**: accounts and sessions, invitations to register,
  credentials of keys, agents and devices, groups, given scopes, callers
  that sign what they send (`people::callers`), and what a person keeps
  with their account for the client (themes, side-panel arrangement,
  clock), stored as text and never acted on. Knows no project, table or
  channel. Its tables migrate under the name `identity` and are still
  listed with the studio's (`studio::copy`, `studio::apart`). Offered as
  `studio::Identity`. New account, session or key logic goes here.
- **`manual`**: what the program says of itself: command-line help, the
  public guides of `docs/` built in (`manual::guides::GUIDES`, files whose
  first line is `<!-- public -->`), and all of it written out as pages by
  `manual::pages::pages`, which `docs export` and the action `docs.export`
  both call. `help --json` is read from the help itself
  (`manual::commands`): a new command is a line of `COMMANDS` and a new
  topic a line of `TOPICS`. The pages are listed in `contract/docs.txt`
  (`WRITE_DOCS=1`, `BREAK_DOCS="why"`); the CLI suite's `documented.rs`
  runs every `mw …` in a public guide's code blocks.
- **`windowed`**: a table's rows read a window at a time as plain values:
  a row as a record (`Record`, `read_record`, `record_id`), records kept
  as files (`windowed::files`), what a window asks and returns (`Asked`,
  `Answer`, `Reading`), the reads `kernel::window` plans, counts kept by
  a table's write mark (`Counts`) and indexed columns (`Indexes`).
  Planning a window of a studio table, managing its indexes and asking
  who may read it stay in `studio::window` and `studio::indexes`.
  Something new a window reads over a connection goes here.
- **`texts`**: which blueprint text something running goes on with:
  where texts are found (`texts::library`: a project's own before shared
  ones before shipped ones), and what a conductor keeps of texts, as
  methods of one keeper (`texts::Texts`: `reach`, `pinned`, `vouched`,
  `taking`), plus `behind`, `known`, `compiled`, `listed`, `forked`,
  `remembered` (a check given again where its catalog answers what it
  asked as before) and `texts::ran` (functions of a text and plain
  values the runner asks).
  No engine and no registry; it calls back through `texts::Runner`,
  which `Engine` implements. Something new kept of texts is a method of
  `texts::Texts` (handed the runner only to ask who is above whom or to
  check a text), not a field of the conductor's `Inner`. Something the
  runner works out from a text and plain values goes into `texts::ran`
  first.
- **`environments`**: the factory profile's variables
  (`environments::factory::Held`, the turn `Held::changing`) and a
  project's environments as plain values (`environments::project`),
  with rules, flags and what is held for every conductor (`held`). The
  runner offers it as `conductor::profile`, `Engine::variables`,
  `Engine::set_variables`, and keeps each environment with its events
  in `conductor::environs`.
- **`conductor`**: the runner engine for one journal (`Engine`): apply,
  update, carry over, stop, steps, handlers, hooks. It re-exports
  `runs` and `texts` under the names it always gave and keeps the thin
  wrappers that first find which conductor keeps a thing. It calls the
  comparisons through what is installed (`conductor::Compared`,
  `Engine::set_compared`) and knows nothing else of them.
- **`projects`**: a project and its files: its row, its folder and the
  factory's place for its own blueprints (`Roots`), the last scan, what a
  table keeps private, what a new project starts with (`templates`,
  `fleet.bp`), what is written with its row (`Placed`), the Trash's rows
  and files (`projects::trashed`), and where a copy's projects are put
  (`projects::copied`). It is handed what it needs of tables as plain
  values (`Projects::keep`, `trashed::KnownAt`). Offered as
  `studio::projects`, `studio::Projects`, `studio::trash`. Something new
  of a project's row or files that needs no table goes here.
- **`blocks`**: actions every registry adds (`pool.turn`, `text.put`,
  `profile.set`, `environment.ensure`, `flag.set`), registered by
  `blocks::register`. A layer of its own so any crate above it can have
  its test blueprints call one.
- **`talk`**: channels, who is in each, messages and attachments, the
  factory's bot, inboxes, conversations, kept channels, the bot's
  commands (`talk::bidden`, `asking`, `commanded`). Knows no project
  table or file. Its actions are added from `Studio::register`; its
  events are listed among the studio's (`put_before` in `studio`'s
  `events.rs`). Something new of a conversation, a bot block, or what
  somebody is told goes here.
- **`pages`**: a page made from a document (`pages::made`), a library
  written out as pages (`pages::site`), and a document and a mockup as
  files (`pages::document`, `pages::mockup`). Offered as `studio::pages`,
  `studio::site`, `studio::documents`, `studio::mockups`. Something new
  of how a page is made, or of a document or mockup as a file, goes
  here; making a record of one is the studio's.
- **`workspaces`**: workspaces and which projects and channels belong to
  each (`Workspaces`, `place_new`, `forget_project`, `place_channel`).
  Offered as `studio::workspaces`. Something new of a workspace's goes
  here.
- **`search`**: the search index, in a database of its own (`Search`).
  It reads projects' files and cards back as plain values through
  `search::Indexed`, which `studio`'s `indexed.rs` implements, and every
  table's rows, the topics with what was said on them and which folders
  are libraries through `search::Rows` (`rows.rs`), which `api`'s
  `searched.rs` implements over the tables alone (holding the studio
  there would hold it from being let go). A library's Markdown files are
  found as documents, at the heading nearest what matched
  (`kernel::sought::heading_at`). Projects, channels, people, agents and
  runs are not indexed: they are few, or (runs) said too often, and are
  found by what they are called as a search is answered
  (`search::named`, listed by `searched::named` as its asker may read
  them). Whoever answers a search holds each hit to how its kind is read
  (`readable` in `routes/fleet.rs`). Something new the index reads of the
  studio is a method of `Indexed`; of tables, of `Rows`; something found by
  its name, a `Named` there.
- **`comparisons`**: one blueprint started several ways
  (`comparisons::Comparisons`). Stands on `conductor` and answers the
  four questions the runner asks through `conductor::Compared`.
- **`hosting`**: services, processes and the wrapper that starts them
  (`proc`, `kept`, `wrapped`, `group`), ports, walls, secrets on disk,
  file watching (`hosting::Watch`), routers and rollouts.
- **`studio`**: a project's tables, records, histories, board, views,
  columns and documents (`Collections`, `Records`, `Board`). What is of
  a project's tables, records, board and files is the studio's.
- **`deciding`**: the blocks a fleet's blueprints call so that they
  work nothing out themselves (`work`, `reviews`, `decided`, `column`,
  `segments`, `turn`, `tally`, `tickets`, `gathered`, `sides`, `told`,
  `triggered`, `spent` and more): functions of plain values, each with a
  table of cases in the crate's suite and the action that calls it. Uses
  no agent. Actions are added by `deciding::register` and from
  `Agents::register`; the kept schema lists actions in the order they
  are added, so add new ones at the end. A new block that uses no agent
  goes here, in a file of its own.
- **`trash`**: reading the Trash, giving things back, deleting for good
  and emptying it (`trash::Trash`).
- **`places`**: a piece of work's place in a project's repository (the
  actions `place.make`, `place.keep`, `place.send`, `place.merge`,
  `place.clear`, `place.changed`, `place.take`, `place.follows`,
  `place.gather`, `place.forward`), the per-repository turn and git calls
  they share, and the repository as the Git page reads and follows it
  (`places::seen`). `agents` holds one (`Places`) and asks it to let git
  finish as the daemon stops (`Agents::settle`).
- **`told`**: what a line a blueprint told shows beside its words as the
  reader reads it now (`told::parts`) and presses on it (`told::press`);
  change requests a person may not make themselves (`told::asks`,
  `docs/approvals.md`); runs waiting for a person's go-ahead
  (`told::goes`, the action `approval.ask`). The routes
  (`routes/reviewed.rs`, `routes/asked.rs`) and `mw ask`, `approve`,
  `decline` each call into it. Something new a told line's part results
  in, or does when pressed, goes here.
- **`outline`**: the keeper of every project's side-panel outline
  (`Outlines`, event `project.arranged`). The plain values are
  `kernel::outline`.
- **`topics`**: the keeper of a forum's topics (revisions, replies, the
  context an agent is given).
- **`extensions`**: what a package says and how one turn of its program
  runs: the manifest (`Extension`), declared actions (`declared`,
  `program`, `performed::once`), blueprints and prompts it brings
  (`brought`), what it shows of a repository (`shown`), how its program
  is handed a sign-in (`handed`), wall needs (`walled::Needs`,
  `Behind`), and one turn (`Adapter`, `Turn`, `running`). Emits no event.
  Offered as `agents::extension`, `agents::adapter`, `agents::Turn`.
  Something new a package says, or a turn is run with, goes here.
- **`agents`**: agent slots, turns, dispatch, walls, keys and sign-ins,
  which packages are installed and the actions they bring
  (`brought::Brought`, `performed::Offered`), and `Agents::behind`.
- **`sandbox`**: sandbox tests: a blueprint run in a factory of its own
  that is handed nothing live (`sandbox::Made`, `sandbox::kept`).
- **`gateway`**: devices and their hub (`hub`, `member`, `door`,
  `place`).
- **`folders`**: a factory's data folder as files, with no daemon needed:
  who holds the folder (`folder`), its name (`profile`), the factories on
  the device (`registry`), a copy and a factory started from one
  (`snapshot`, `arrival`, `tree`), a project moved to a journal of its
  own (`moved`), the database written out again (`repack`), a project's
  own blueprints as a library layer (`library`), where a text stands
  among those that ship (`forked`), where the running program is
  (`place`), a factory restarted by a step (`restarting`), and what the
  program and packages offer a blueprint (`contract`). Its events are in
  its own `events.rs`; its actions are `factory.snapshot`
  (`folders::register`) and `factory.restart`
  (`folders::restarting::register`). Offered as `api::snapshot`,
  `api::folder`, `api::library`. Something new done to a factory's files
  that needs no route goes here, and the API suite runs it through the
  daemon.
- **`setup`**: what a project and an agent are set up with when somebody
  asks, by applying shipped blueprints as that person: a new project
  (`usual`, which applies `std/project`; saved setups, `setups`), an
  agent added by hand through `std/agent` (`byhand`), and an example put
  among a project's blueprints (`examples`). Offered as `api::usual`,
  `api::byhand`, `api::examples`; its tests are the API suite's
  `usual.rs`, `byhand.rs`, `examples.rs`.
- **`served`**: what the daemon serves over HTTP beside the API routes,
  and what every request is checked against first: the client's files
  (`client`), whether a request was addressed to this daemon and from an
  allowed origin (`guard`, `Names`), reading through what was sent past a
  route's limit before a refusal (`sent`), library pages (`library`),
  mockups under a way in (`mockups`, `Ways`), and the store program
  (`store`). Registers `system.answer_as` (`served::register`). Offered
  as `api::ui`, `api::client_file`, `api::store`, `api::sent`. Something
  new served beside the API that needs nothing of a route goes here.
- **`api`**: the node and the HTTP routes (`routes/`), the `App`, the
  event stream, the bot's command routes (`api::commands`).
- **`cli`**: the `mw` command line, including the wrapper word and the
  credential helper.

## Where something new goes

- A function of the blueprint text alone, or of one value written in
  it: `reader`. Of a checked blueprint: `blueprint`.
- A function of plain values with no engine in it: `kernel`, even when
  the runner or the studio is what calls it (as `blueprint::differs`,
  `kernel::why`, `kernel::pool`, `kernel::rows`, `kernel::cards` are).
  The action that calls it goes in `blocks` or `deciding`.
- Something the runner works out from a text and plain values:
  `texts::ran`. Something kept of texts: a method of `texts::Texts`.
- Runner machinery that needs only a started task: `runs`.
- A block a fleet's blueprints call that uses no agent: `deciding`.
- A project's row or files without tables: `projects`. A page or a
  document as a file: `pages`. A window of rows over a connection:
  `windowed`.
- Accounts, sessions, keys: `people`. Conversations and bots: `talk`.
  Workspaces: `workspaces`. The search index: `search`.
- A factory's files with no route: `folders`. A project or agent set up
  for somebody by applying shipped blueprints: `setup`. Something served
  beside the API: `served`. A told line's parts: `told`. A package's
  manifest or a program turn: `extensions`.
- Something that serves no route and needs nothing of the `App` but
  what can be handed to it goes into a crate below `api`.
- New capability is usually a new action (`Registry::add_fn` in a
  crate's `actions.rs`), not a change to the conductor; new behaviour is
  usually a blueprint under `blueprints/std/`.

## Workspace rules that shape crates

- **Size ceilings.** Files stay under 1000 lines and crates under 15000.
  When one grows past that it is split into two things. A crate allowed
  past the ceiling for a while is listed with its limit and the reason
  in `ROOM` in `xtask/src/gates.rs`, and owes the split; the list is
  empty now. When a crate nears its ceiling, move out what is a function
  of plain values (to `kernel`, `reader`, `texts::ran` or `runs`) or a
  part that needs nothing of the rest, into a crate below it.
- **Every dependency is declared once**, in the root `Cargo.toml`, with
  its features. Member crates say `{ workspace = true }` and nothing
  more. After adding, updating or removing a dependency (a client
  package too), run `cargo xtask notices` and commit what it writes
  (`THIRD-PARTY-NOTICES.md`).
- **The licence is declared once**, in the root manifest; every member
  says `license.workspace = true`.
- **One test binary per crate**: `tests/suite/main.rs` plus modules.
  Libraries set `test = false`; no `#[cfg(test)]` modules in `src/`.
- **Nothing leaves the machine unasked.** No telemetry, no update
  checks, no addresses of ours in the code. The `offline` gate runs the
  whole product with the network cut off and fails if one packet heads
  outside, whichever library sent it. What a user points it at (an
  agent's model, a hub, a command in a blueprint) is theirs.
