# Projects: setup, pipelines, environments and flags

This file covers what a project is given when it is added (`std/project` and
setups), what can be given to it later, how a project's pipelines are run by
the rows of its `triggers` table, a project's environments and their secrets,
feature flags, and the application's own experiments. Read it before changing
how a project is set up, how a pipeline is triggered, or anything an
environment holds.

## A project's library and records

- A library's records are files as they are (`studio::documents`). A
  document's id is where its file is (`shop-docs:guides:install`), not a
  number, so nothing may split a record's id at its last `-`.
  `Collection::owns` says whether an id belongs to a collection.
- A mockup is known by where it is (`shop-mockups:pricing`), like a document;
  `Collections::record` tries mockups, then documents, for an id with a `:`.
- A new project is given `docs/architecture.md` and `docs/overview.md`,
  unless whoever added it said what it starts with and left its library out
  (`Projects::create_with`). A test of a library clears the folder first, or
  counts those two files.
- A project the factory makes a folder for (an empty one, or none said) is a
  repository from the start, and the files it was started with (`README.md`,
  and the library's two where it has one) are its first commit, made as it is
  made, on `main` (`projects::first`, card 477). Without it the first card had
  no branch and was worked in the project's folder, and where a person then
  made a first commit of their own the starting files were still untracked
  there: the first approved branch that changed `README.md` could not be
  merged over them. It is committed as the fleet commits a card's work
  (`place.keep`): the factory as author, at an address nobody has
  (`<product> <cli>@factory.invalid`), and whoever git says commits there
  as committer; only as somebody git was told about (`user.useConfigOnly`),
  never one it makes up from the machine's names. Where git knows nobody,
  nothing is committed, the files are left added to the index, and the
  answer says so in the fleet's words (`kernel::git::TELL_WHO`). What came
  of it is `first_commit` in the answer to `POST /api/projects` (`commit`,
  `files`, `named`, `says`) and `committed` on `project.created`; `mw project
  create` prints `says`.
- A folder that has anything in it, a repository or not, with history or
  none, is taken up as it is: nothing is written there and nothing of it is
  committed. A folder with files of a person's own and no commit is theirs
  to commit; until they do, its cards are worked in the folder itself, as in
  a folder that is no repository.
- Only a program that runs a factory makes the first commit
  (`Config::commits_first`, said in `serve.rs`). A test's own node does not:
  its projects are repositories with no commit, as the API suite's lanes
  tests expect to begin from, and who git commits as there would be whoever
  runs the suite. The first commit is played by the real program with a
  home of its own (`begun.rs` in the CLI suite).
- A file written through the factory is said once, by whoever wrote it. It
  is held (`projects::writing`) from before it is on the disk until it is
  written down as the project's, and a look over the folder
  (`Projects::scan`) says nothing of a held file: one between the two said
  a plan was the system's, and its cards named no agent (card 475,
  `looked.rs` in the studio suite). Something new that writes a project's
  file through the factory holds it the same way.

## What a project starts with: `std/project`

- What a new project may start with is not listed in Rust. Setting a project
  up is a blueprint, `std/project`, applied for it as whoever adds it
  (`usual::give`; the apply is waited for and its run's outputs are returned
  in the answer). Its rows are what the project starts with.
- Each row is a blueprint (one that ships, a package's, or somebody's own) and
  what it is handed. The shipped default is that blueprint's own `rows`
  (`usual::shipped`): a board, a fleet, a library, and features, card times,
  A/B tests, pipelines and environments, the last five given only where asked
  for. A row has: a name; what it is called where a person chooses it (Board,
  Fleet, Docs: `title`); how it is said in a sentence (`said`); a description;
  its blueprint; whether it is on unless said; what it cannot be had without;
  which of its blueprint's params are told the project; and the table it
  gives.
- The table a row gives lets `usual::lacking` tell somebody who asks for a
  record of that table, in a project that has none, the one command that
  gives it.
- What a row is and what is decided about rows are plain functions with their
  cases (`deciding::setups`, tested by the deciding suite's `setups.rs`):
  `rows` reads them; `chosen` works out what a request asks for (it hands
  every row on, those not asked for switched off, so that what one row needs
  is said by the row that gives it); `came` says what a row evaluates to as it
  is reached: already there, left out with a reason, or given.
- The applying is `std/project`'s one action, `project.set_up`
  (`setup::given`). It calls `came` for each row in turn and applies what is
  to be given through the runner as somebody asking (`Engine::apply`), as
  whoever applied `std/project`, at the top level (not under `std/project`),
  under no environment, and waits for it. So what a project starts with is
  exactly what that person could have applied by hand, and `std/project`
  itself is over once its run is.
- A row of `fleet` is the project's own fleet blueprint, written first
  (`init_fleet`) unless the folder came with one of its own. A folder's own
  fleet blueprint is never applied by adding the folder.
- `std/project` applies each row as whoever asked and makes nothing itself.
  Somebody who may not apply a blueprint is given the project alone and told
  why. A row left out for lack of what it needs (a fleet with no board) is
  reported and is not an error.
- A list param's default is not taken as the name of a blueprint where the
  side panel's two levels are worked out (`blueprint::names`): what
  `std/project`'s rows name stays at the top level, where somebody applies it.

## Asking for a setup

- A project is given anything only where whoever adds it asks (`set_up`).
  `set_up` is `true` (every row that is on), a list of the default's rows by
  name, each optionally with what it is handed
  (`{ "name": "board", "with": { "columns": [...] } }`), rows of the
  request's own (a row that says its `blueprint`), or
  `{ "setup": "<name>", "rows": [...] }`. `mw project create --setup <name>`.
- A request that says nothing adds the project alone. That is not the same as
  an empty list: an empty list also gives no documents.
- The command line and the client's dialog ask for the usual rows unless told
  not to (`--bare`, a switch). A test, a check or a package's example that adds
  a project through the command line says `--bare` unless what the project
  starts with is the subject.
- The command line leaves a row out by name (`--without`); neither it nor the
  client names a blueprint.
- A row's `with` is refused where it names something the row is told of the
  project (`told`: the fleet is told its project and channel, which its
  blueprint says itself), and a param its blueprint does not take is left out
  and reported in its apply's message.
- What each row takes is `GET /api/starts` (`?setup=<name>` for a setup's
  rows; `usual::settings`): its blueprint's params as the language reads
  them, less what it is told; the fleet's are read from the text a project's
  own fleet is written from. The dialog that adds a project draws them under
  each row, folded, as the apply drawer draws a blueprint's params
  (`ParamField`), and hands on only what somebody wrote (`askedOf` in
  `ui/src/startset.ts`). Nothing in the client names a param.

## Setups a factory keeps

- A factory may keep setups of its own (`setup::setups`, in its small durable
  state under `project.setups`, which a copy takes): named ones, saved by
  whoever may add projects and apply (`projects:write`, `blueprints:run`), and
  one default for new projects (`default`, saved by whoever holds `admin`).
  The default is then what `set_up: true` means and what the client is handed
  (`usual::the_usual`).
- Each save or removal is one write with its event (`project.setup.saved`,
  `project.setup.removed`, kept by the factory's journal for good).
- `GET /api/setups` lists them with what ships and which is the default;
  `PUT` and `DELETE /api/setups/{name}` save and remove.

## Starts offered in the client, and given later

- The client is handed the rows (`project_starts` of `/api/meta`) and draws a
  switch for each. Each row says where it is offered (`offered`): `adding`, in
  the dialog that adds a project (`offeredAt` in `ui/src/starts.ts`), or the
  name of something it needs, on whose page it is offered (features: `board`,
  a gear menu line on the board's page, `startLines` in
  `ui/src/views/starts.tsx`).
- `offered` only decides where a switch is drawn. By name, any row can be
  given while adding (`--with features`, `set_up`) or to an existing project
  (`POST /api/projects/{id}/starts` with `names`, `usual::give_later`): the
  same asking of the same blueprints as the caller, refused whole without
  `blueprints:run` (checked before anything about the project is revealed),
  rows left out with their reason where they lack what they need, idempotent.
- What a project has is what is alive in it of a start's blueprint (`used`,
  the project's list of blueprints; `hasOf` in the client), never "a table
  exists", since a table outlives what looks after it. So the board page's
  line turns on and not off: stopping the instance (under Activity) keeps its
  table and records, and the line then gives it back over what was kept.
- While a start is being given it is asked for once (`giving` in
  `starts.tsx`); on, it is closed; lacking what it needs, closed with why; for
  somebody who may not apply, no line while off and a sentence while on.
  Checked by `ui/scripts/features.mjs`.
- Something new a project starts with is a row of `std/project` and nothing in
  the client; something new offered on a page is that page handing its head
  `startLines`.
- The environments row (`std/environments`) makes `dev` and `prod` with no
  variables unless its `names` says others (action `environment.ensure` of
  `blocks`, as whoever adds the project). It is off unless asked for, because
  a project with environments has every apply name one
  (`environments::held::must_choose`), which would refuse the first-run notes'
  commands (`written.rs`). What the project has of it is its environments
  (`has`), not a live instance.
- What a project starts with is applied under no environment, by the empty
  environment name, which no request may say.

## Applying what a project lacks, and forgetting a project

- A start applied by hand to a project that lacks what it works on (a fleet
  and no board) is not refused. The apply's `said` names the command that
  gives the missing piece (`usual::lacks`); the command line prints it and the
  client shows it. A fleet that is up before its board works the board once
  it is there. The built-in assistant offers the board before the fleet, and
  says the same where it is asked for a fleet first.
- Forgetting a project stops nothing and is not refused for what runs (a
  project can be let go of and taken up again with its fleet). The answer
  lists what is alive that was applied for it, its agents that are up and its
  channels (`still_running` in `crates/api/src/routes/work.rs`), each kind
  only to whoever may read it, and the command line prints the command that
  ends each.

## Pipelines and triggers

- A project's pipelines are its own blueprints (`pipelines/<name>`), run as
  the rows of its `triggers` table say (`std/pipeline` declares the table;
  `docs/blueprints.md`, *A project's pipelines*): by hand (`pipeline.asked`,
  emitted from outside as whoever asks), as a branch moves
  (`project.branch.moved`), as a card comes to a column, a set time after it
  last ran (`every`), and at a time of day with its zone (`at`).
- They are rows and not handlers each pipeline writes, because a pipeline run
  again is applied over, which ends its own handlers.
- Which rows fire, under which key and handed what, and which go by the clock,
  is a block's (`deciding::triggered`: `triggers.fired`, `triggers.timed`,
  with their cases). A run is handed only what its pipeline declares it takes;
  a name asked by hand that it does not take is refused. A row runs its
  pipeline as the pipeline reads to whoever applied `std/pipeline`
  (`text_within`, as a project's own blueprint is vouched for).
- A row says what more its pipeline is handed in `with` (`name=value`, each
  read as its param's kind, as a row of `segments` does). The cell takes
  `shell` and `secrets:use` beside `blueprints:run`, since a value may reach a
  command or name a secret of whoever applied `std/pipeline`. Writing any row
  takes `blueprints:run` (each cell says so).
- Every run is a `call` of the pipeline under its key by a handler that takes
  turns by that key (`go`). A run asked for while one under the same key is
  going waits for it, in the order asked, and is taken up after a stop. Inside
  its turn it asks the project's pipeline pool for a place by its key
  (`std/pool`, `<project>/pipelines`, `at_once`) and gives it back however the
  run ended. The turn, not the pool, keeps one key's runs apart: two requests
  for one name in a pool cannot be told apart.
- A row that names no key runs under the pipeline's name and the
  environment's (`pipelines-deploy-prod`), since an instance is under one
  environment for its life.
- Somebody who asks by hand is checked first for `blueprints:run` and for what
  the pipeline says it runs with (`actor.holds` with `scopes`; `admin` where
  the pipeline says nothing, since it then runs with everything whoever
  applied `std/pipeline` holds). A refusal is emitted (`pipeline.refused`,
  with what they lack) and told to them in their inbox (`inbox.tell`, in the
  sentence `triggers.refused` makes).
- What goes by the clock is a child of `std/pipeline-timed` for each row:
  `std/later` for `every`, begun again each time it ends, and
  `std/pipeline-at` for `at`.
- Branches are followed only for a project something asked for
  (`branches.follow`, `places::branches`, kept in memory and asked again on
  `system.started`), with `hosting::Watch` on `refs/heads` and `packed-refs`,
  by the one follower of the repository its Git page uses too
  (`api::routes::git::follow`). What each branch was last reported to be is
  kept in the project's journal in the write that emits the moves
  (`places::branches::said`), so a move made while the daemon was away is
  emitted once it is back.
- The simulation plays a run asked by hand, stopped at every point and
  restarted (`crates/api/tests/suite/triggering.rs`).

### Triggers and environments

- A row says the environment its runs are under (`environment`, passed
  through `triggers.fired` to `pipeline.fired`), and `go` calls the pipeline
  under it with the step word `environment:`, so the environment's rules hold
  as the call starts it: who approves, checked against whoever applied
  `std/pipeline`, and the branches it runs, against what the run is handed.
  The cell takes `environments:secrets` beside `blueprints:run`, as setting the
  rules does, since the run is started as whoever applied the triggers, whom
  the rule may name.
- A triggered run never proceeds on the standing of whoever applied the
  triggers alone. Under an environment whose rule says who approves, `go`
  asks one of them first (`approval.ask` with `environment:`, before the call;
  the call is skipped where they decline) unless everybody the run is begun
  for is one of them. That is a block's decision (`asks` of each run
  `triggers.fired` gives, over `kernel::go::first` with its cases), over:
  whoever applied the triggers, whoever last wrote the row's `environment`,
  and whoever asked by hand.
- Who last wrote `environment` is kept on the row by the factory
  (`environment_by`, a field with `writer_of: environment`), so a row written
  by somebody the rule does not name (a key with fewer scopes included) waits
  for a go-ahead even when whoever applied the triggers is named. A row from
  an earlier version that records nobody waits too.
- Something new that has a row run something under an environment as
  whoever applied what keeps the rows calls `kernel::go::first` the same way.

## Environments

- A project's environments (`dev`, `staging`, `prod`: named sets of variables
  a project defines; `docs/environments.md`) are kept each whole under the
  project and its name, in the journal of the conductor that runs what is
  applied for the project (`runs::beside::PROJECT_ENVIRONMENTS`,
  `Whose::Project`), so a copy and a move of the project to its own journal
  take them by that line.
- Each change is one write with its event, holding one turn for every
  conductor (`Engine::change_environment` in `conductor::environs`, the turn
  `Inner.environs`), emitted by name and never by value
  (`project.environment.*`). Something new kept of an environment is a field
  of `environments::project::Environment`, read back by `read`, and changed
  through `change_environment`, never by a write of its own.
- Changing one takes `projects:write`, `shell` and `secrets:use`, since what
  it says reaches the commands and secret names of what is applied as whoever
  applied it (`CHANGING` in `crates/api/src/routes/environments.rs`).
- An agent is shown, and handed, only what is marked for agents
  (`Variable::agents`, off unless set; `Environment::shown` and `values` with
  `to_an_agent`). A name the application itself sets in a program's
  environment is never marked (`may_hand`). Anything new an environment holds
  is off for agents unless marked.

### An instance's environment

- An instance runs under one environment of its project, or none, for its
  life (`instances.environment`, with a database rule refusing later changes;
  `InstanceInfo::environment`). Applying something that would make an instance
  of a project with environments must name one (`Apply::environment`,
  `environment` of `POST /api/apply`, `mw apply --environment`), else it is
  refused with the list (`environments::held::must_choose`). Children, self
  applies and attachments inherit the parent's (`Engine::environment_for`); an
  apply over an instance naming another is refused (`not_another`).
- What runs under an environment reads it from what is held in memory for
  every conductor (`environments::held::Held`, `Inner.environs`; loaded at
  start by `Engine::hold_environments`, updated after each write, dropped by
  `Engine::environments_gone`), never from the journal during evaluation:
  `environment.name`, `environment.vars` (only marked ones for what an agent
  applied), `FACTORY_ENVIRONMENT`, and which secret a name in `env` is
  (`Held::sought`: the environment's where it lists the name, else the
  factory's; `/` in a name is refused; an unmarked one is refused to what an
  agent applied, never replaced by the factory's).
- `hosting::secrets::Secrets::get` stays the factory's alone; what the runner
  hands on is opened by `handed`. A device keeps no project environment
  secrets and refuses a step handed one (`member.rs`).
- An environment is removed only where nothing runs under it, checked holding
  the turn for changing environments (`environments::kept::remove`); an
  instance made under one holds that turn from the existence check to the
  write of its row (`Held::holding`; both orders made in `under.rs`).
- New code that reads an environment for what runs reads `Held`; new code that
  changes one updates `Held` after its write.

### Environment rules: branches and approvers

- What comes to run under an environment is checked against its rules in that
  same turn (`Environment::rules`, `environments::rules`): an apply that names
  it (from outside, or a step of a run under none that says `environment:`,
  the one case where a child is not under its parent's:
  `environments::held::chosen`). Never what is started by what already runs
  under it, which came in through the check.
- `branches` refuses a branch it does not name, read from what the run is
  handed as `branch` (as Run hands its deploy).
- `approve` lets anyone it names start anything, and anybody else only from a
  run that holds a go-ahead naming the environment, approved by one of them,
  read and never consumed (`told::goes::GoAheads`, installed by
  `Held::answers` before a start takes anything up; the runner sits below
  `told` and calls it through `environments::held::Approvals`, which holds the
  factory's journal and nothing of the runner).
- An apply over an instance under the environment that names it from outside
  is checked the same way (`apply_once`), so Run pressed again over a deploy
  that stays up is checked as the first was.
- Something new that names an environment for what it starts goes through an
  apply, and so through the check. Something new an environment enforces is a
  field of `Rules`, a case of `Rules::comes` with its line in the
  environments suite's `rules.rs`, and is set by `PUT .../rules`.

### Agents under an environment

- An agent works under one environment of its project, or none
  (`slots.environment`; `Slot::environment`). `agent.ensure` gives it the
  environment of the instance that brings it up; one added by hand names one
  (`SlotSpec::environment`, `mw agent add --environment`) and is refused,
  with the list, where it names none in a project that has some
  (`Agents::none_said`).
- Its row is written holding the turn at changing environments, as an
  instance's is, and an environment is not removed while an agent works under
  it (`kept::remove` is handed the agents under it).
- What its program is handed is worked out with its keys
  (`Agents::keys_for`, `crates/agents/src/keys.rs`): each variable marked for
  agents under its own name, each marked secret as its variable, and a key its
  slot names looked up in the environment first, refused where the
  environment lists it unmarked (never replaced by the factory's). All of it
  is scrubbed from the program's output (`Keys::unsaid`) and named, never
  valued, where the agent is told what it holds (`Keys::variables`).
  Something new an agent's program is handed of an environment goes through
  `keys_for`.

### Environment secrets

- An environment's secrets are listed by it (`Environment::secrets`: the name,
  who set it, what an agent is handed it as). Their values are files of the
  factory's one secret store, in a folder of their own
  (`hosting::secrets`: `secrets/@projects/<project>/<env>/`), each sealed
  under the whole name it is looked up by (`project/env/name`), so one moved
  into another's place does not open.
- Nothing is found by a name a blueprint writes that reads like one of these:
  `Secrets::get` is the factory's alone and refuses one. An environment's
  secret is asked for by its three parts (`get_of`), by whoever has said which
  project and environment it is for.
- The file is written before the list names it, and the list stops naming it
  before the file goes. A stop between the two leaves a file nothing reads,
  never a wrong list. A list entry with no file reads as *not set* (`kept`),
  which is also what a copy of the factory shows, since a copy carries no
  value.
- Setting, removing and marking one takes `environments:secrets`, which the
  factory's owner holds without being given it (`people::identity`). Its names
  are shown to whoever holds `secrets:use`, and to an agent those marked for
  it; for anybody else the list is left out, not empty.
- A project deleted for good (from the Trash or at once) takes what is kept by
  its name beside the runner's rows (its environments, its forks' origins:
  every `Whose::Project` line) and its environments' values with it
  (`Trash::project_gone`). In the Trash they stay, so it can be given back
  whole. Something new kept by a project's name beside the runner's rows is a
  line of `runs::beside::KEPT` and goes with it by that.

## Feature flags

- A project's feature flags (`docs/environments.md`, *Feature flags*) are rows
  of its `flags` table (`std/flags`, a start). What a flag says in an
  environment is kept in that environment's variables and nowhere else
  (`FLAG_<NAME>`, `kernel::flags::variable`). There is no copy in the row,
  because the row may be in another file than the environment and the two
  could not be one write.
- A flag is written with its events as one write (`environments::flagged`,
  through `Engine::set_flag`: `project.environment.changed` and
  `project.flag.changed`, which names the row so its history keeps the
  change). Its variable is changed only as the flag, by the route
  (`routes/flags.rs`) or `flag.set`; the route that sets variables refuses one
  beginning `FLAG_`.
- What a change evaluates to is one rule, `kernel::flags::change`, with its
  cases. A turning on or widening (`widens`) where the environment's rule
  names approvers, by somebody it does not name (a key counts as itself), is
  emitted as `project.flag.asked` and waits for `approval.ask` in `std/flags`,
  which writes it once approved, provided whoever asked still holds
  `projects:write`. Anything else is written at once as whoever asks; turning
  off never waits.
- An approval never undoes a later change. What is asked is kept on the
  environment (`Environment::asked`, the id last asked by the flag's variable)
  with what the flag said then, in the write that emits it (`flagged::ask`).
  The approved change is written only where, in the write that would land it,
  it is still that id and the flag still says that (`flagged::Expected`).
  Every write of the flag clears the id (a turning off that changes nothing
  included) and a later ask sets its own, so two approvals cannot land out of
  order. Overtaken, nothing is written and `project.flag.unchanged` records it
  on the row's history.
- Something new that writes a flag goes through `flagged::set` (via
  `set_flag`) so it takes a waiting change's place. Something new a flag can
  say is a `Kind` and a case of `said`.

## The application's own experiments

- What of the application itself may be turned on before it is complete is
  `kernel::experiments` (`LIST`), off unless the factory's owner turns it on
  in Settings, Experiments; kept in the factory's journal and so in a copy. A
  part of a feature that is not usable alone lands behind a line of it, and
  the line is removed when the feature is complete.
