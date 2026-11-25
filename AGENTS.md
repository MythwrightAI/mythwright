# Mythwright: a guide for agents

This file is for any coding agent working with this repository. It has
two parts:

- **Part 1, installing and using it**: you were asked to install
  Mythwright, or set it up for somebody. Build it, start it, and get them
  to a first project with an agent working its board.
- **Part 2, working on the code**: you were asked to change Mythwright
  itself. Read Part 2 whole before you change anything, and the topic file
  under [docs/working/](docs/working/) for the area you touch.

# Part 1: installing and using it

Before anything else, read *Before you start* and *Known limitations* in
[README.md](README.md). Linux is the one system it is tested on: if the
person is on Windows or macOS, tell them what those say of their system
(no wall around an agent's program on either, and macOS never run)
before you install anything.

## What it is

Mythwright is a software factory a person runs on their own machine: a
daemon that keeps their projects, boards, channels and agents, a web
client (and a desktop window) to use it, and one command, `mw`, that does
everything the client does. It sends nothing anywhere by itself. The
agents it runs are command-line agents the person already has, run as
them and signed in as them. [README.md](README.md) says the rest;
[docs/first-run.md](docs/first-run.md) is the first hour in the client.

Ask the person before you install anything on their system (Rust, Node,
an agent's program), before you start an agent's program, and before you
remove or forget anything in a factory.

## Build and install it

Only Linux has been built and run from source. The same steps are written
for macOS and Windows, which have not been run from source (the README's
*Status* says what has been tried where). You need Rust, which `rustup`
fetches at the version `rust-toolchain.toml` pins, and Node 22.18 or newer
as nodejs.org builds it.

On Linux and macOS, from the top of the repository:

```sh
(cd ui && npm install)
cargo xtask install
```

That builds everything and puts a copy of the program in
`~/.local/lib/mythwright` and the command `mw` in `~/.local/bin`; it says
so if that folder is not on the `PATH`. `cargo xtask install --desktop`
also builds the window (on Linux it needs WebKitGTK 4.1 from the system).

On Windows, in PowerShell, from the top of the repository:

```powershell
cd ui; npm install; cd ..
cargo xtask install
```

The copy goes to `%LOCALAPPDATA%\Programs\mythwright` and `mw.exe` to
`%USERPROFILE%\.local\bin`, which is to be put on the `PATH`. A fleet
there needs Git for Windows.

What is installed is a copy: a change made in the checkout reaches it
only once it is installed again and the daemon is started again.

## Start it

```sh
mw up
```

`mw up` starts the daemon and keeps it running: it does not return. Run
it in the background or in a terminal of its own, and wait for the line
it prints, `Mythwright is running at http://127.0.0.1:7700`, with where
it keeps its data under it. Stop it with `mw down`, never by killing it.

The first start makes the factory's owner (called Unnamed until they say
their name) and signs this terminal in as them: nobody registers. Then:

```sh
mw status
mw whoami
mw open
mw open --link
```

`mw open` opens the client in a browser, signed in. On a machine with no
browser, `mw open --link` prints a link that signs a browser in once,
within a minute: give it to the person, and treat it as a password.

**If 7700 is taken**, or `mw status` shows a factory already running
there that the person uses, do not start another on their data. Start it
at another address:

```sh
mw up --listen 127.0.0.1:7701
mw --url http://127.0.0.1:7701 status
```

and give every command after it `--url http://127.0.0.1:7701` (or set
`MW_URL` to that address once). For a factory of its own to try things in,
with its data apart from theirs, give it a profile:

```sh
mw up --profile try
mw --profile try status
```

## Give it an agent that can work

Out of the box an agent runs on a small built-in assistant that needs no
model and cannot do the work on a card. A real agent is a command-line
agent with a package here. The person installs that program and signs it
in themselves; you do not sign in for them.

```sh
mw extension list
mw extension add claude
mw agent list
```

`claude`, `codex` and `kimi` come with the program. Install the package
before the first project is made: a new project's agents run on the first
package installed whose program is on the machine. A program that has no
package here is brought in as [docs/own-agent.md](docs/own-agent.md) says.

An agent's program is started behind a wall, which on Linux is held by
the system's bubblewrap (`/usr/bin/bwrap`): where it is not installed, an
agent is not started until it is, or until that agent is told to work
with no wall (`mw agent add <name> --no-wall`). Nothing holds a wall on
Windows or macOS: there the daemon is started with `mw up --walls off`,
or each agent is told `--no-wall`. An agent with no wall sees the machine
as the person does; say so to them before you choose it (`mw help wall`).

## A first project and a first card

```sh
mw project create shop
mw board add shop "Write a README for the shop"
mw board move shop-1 ready
mw board shop
mw read shop
```

A project is made with a board, a fleet (a channel named after the
project, an architect to talk to there, and a worker) and a library of
its documents. A card moved to `ready` is taken by the fleet's worker
when its turn comes, worked on a branch of its own, and put in `review`
for the person. `mw read shop` shows what the worker said in the
project's channel. On the built-in assistant the worker says it cannot
do the work, and what would.

## When something fails

- **Read the error.** Each one is a sentence that says what to do next
  when there is something to do.
- `mw status`: whether a daemon answers where the command looks, which
  program it is and what it has.
- `mw help`, and `mw help <command>`: every command, and one in full.
- The terminal running `mw up`: what the daemon itself says.
- `mw runs`, then `mw run <id>` and `mw logs <id>`: what a blueprint did,
  step by step, and what each step printed.
- `mw events --limit 20`: the last things that happened.
- `mw agent list` and `mw agent try <name>`: whether an agent's program
  is found, starts and is signed in, without asking it anything.
- `mw down` and then `mw up` start the daemon again; it takes up what it
  was doing.

## What not to do

- **Do not edit the data folder by hand** (the folder `mw up` said it
  keeps its data in; `~/.local/share/mythwright` on Linux). Everything in
  it is changed through `mw` or the client. Do not copy it while the
  daemon runs either: stop it first, or have the factory copy itself
  (`mw help profile` says how).
- **Do not commit secrets, or put them in a command.** A value a program
  needs is kept by the factory and read from standard input:
  `mw secret set <name> < file`. An invitation, a link from `mw open
  --link`, a key from `mw key` and the files under `~/.config/mythwright`
  (on Linux; the user's settings elsewhere) are passwords: never commit
  them, paste them into a channel or print them in a log. Nothing of the
  data folder belongs in a repository either.
- Do not run an agent's program yourself to see whether it works:
  `mw agent try` is the way to check, which starts it and asks it which
  models it offers and nothing more. It still runs the person's program
  as them, so ask first.
- Do not take anything away (`mw agent remove`, `mw project forget`,
  `mw channel remove`) unless the person asked for it.

# Part 2: working on the code

Read [README.md](README.md) for what Mythwright is, and
[docs/architecture.md](docs/architecture.md) before you propose new
structure: most "why is it like this" questions are answered there.
[CONTRIBUTING.md](CONTRIBUTING.md) says how a change is proposed.

## What it is made of

A Rust workspace (`crates/`), a web client in TypeScript and Preact
(`ui/`), the blueprints that ship (`blueprints/std/`), packages for agent
programs (`extensions/`), examples (`examples/`), the desktop window
(`desktop/`) and the build tooling (`xtask/`, standard library only).

The daemon keeps everything in a journal of events and tables (SQLite,
one thread per database). The runner (the `conductor` crate) runs
*blueprints*: YAML-like texts that say what exists, what is heard and
what is done. What a blueprint does is *actions*, written in Rust. The
client and the command line `mw` are two doors to the same HTTP API.

Dependencies point one way. A crate may use the crates before it in this
list, never those after it (the `layers` gate):

`brand`, `link`, `reader` → `blueprint`, `kernel` → `journal` → `runs`,
`people`, `manual`, `windowed` → `texts`, `environments` → `conductor`,
`projects` → `blocks`, `talk`, `pages` → `workspaces`, `search` →
`comparisons`, `hosting`, `studio` → `deciding`, `trash`, `places`,
`told`, `outline`, `topics`, `extensions` → `agents`, `sandbox` →
`gateway` → `folders` → `setup`, `served` → `api` → `cli`.

One line each (more in [docs/working/crates.md](docs/working/crates.md)):

- `brand`: the product's name, read from `brand.toml`.
- `link`: the installation's secret, its keys, the Noise handshake, sealed frames.
- `reader`: the blueprint language as text: syntax tree, diagnostics, expressions, templates, durations, times of day.
- `blueprint`: the language checked: vocabulary, catalog, validation, summary, formatter, schema.
- `kernel`: plain values and pure functions with tables of cases; uses nothing else here.
- `journal`: events, databases, migrations, event numbers, one journal per project.
- `runs`: the runner's tables and the machinery that needs no engine (delivery, turns, held writes).
- `people`: accounts, sessions, keys, invitations, groups, themes and arrangements kept per account.
- `manual`: the command line's help, the public guides, pages written from them.
- `windowed`: a table's rows read a window at a time.
- `texts`: where blueprint texts are found and what a conductor keeps of them (pins, vouching, reach).
- `environments`: the factory's profile variables and a project's environments.
- `conductor`: the runner: instances, runs, steps, handlers, applying, restarts.
- `projects`: a project's row, folder, own blueprints and Trash files.
- `blocks`: built-in actions (`pool.turn`, `text.put`, `profile.set`, `environment.ensure`, `flag.set`).
- `talk`: channels, members, messages, attachments, the factory's bot, the inbox.
- `pages`: a document or library made into pages; documents and mockups as files.
- `workspaces`: what groups projects and channels.
- `search`: the search index, in its own database.
- `comparisons`: one blueprint started several ways and counted.
- `hosting`: programs, services, ports, the wrapper, walls, file watching, secrets on disk.
- `studio`: a project's tables, records, board and files.
- `deciding`: the blocks a fleet's blueprints call to decide things, each with a table of cases.
- `trash`: reading, restoring and emptying the Trash.
- `places`: a piece of work's git worktree and branch; a repository as the Git page reads it.
- `told`: what a told inbox line shows and does; approvals.
- `outline`: how a project's things are arranged in the side panel.
- `topics`: the forum's topics.
- `extensions`: a package's manifest and one turn of its program.
- `agents`: agents (slots), turns, walls, dispatch, councils.
- `sandbox`: a blueprint run in a throwaway factory of its own.
- `gateway`: devices and hubs.
- `folders`: a factory's data folder worked on as files: copies, moves, repack, the lock.
- `setup`: what a new project or agent is set up with.
- `served`: what the daemon serves beside the API, and what every request is checked against first.
- `api`: the HTTP routes and the node.
- `cli`: the one program (`factory`, installed as `mw`): the command line and the daemon.

## Commands

From the top of the repository:

- `cargo xtask check`: every gate. It must pass before work is called done.
- `cargo xtask check --changed`: only what local changes affect.
- `cargo xtask check --fast`: the static gates, in under a second.
- `cargo xtask crowd <crate> <module::test>`: run one test many times at
  once on few processors, to find a test that fails now and then. Name the
  test whole; failing runs' output is kept under `target/xtask/crowd/`.
- `cargo xtask ways [one|own|gathered|ungathered]`: run every test program
  under each way the journal can be set up (one journal or one per
  project; commits gathered or not). Run it before changing where
  anything is stored or how the journal commits.
- `cargo xtask timings`: the slowest tests. `cargo xtask build-times`:
  measure the build and add a line to `docs/build-history.csv`.
- `cargo xtask left`: list processes of this checkout that nobody waits for.
- `cargo xtask notices`: rewrite `THIRD-PARTY-NOTICES.md` after any
  dependency (Rust or client) is added, updated or removed; commit it.
- `cargo xtask install`, `cargo xtask package`: install a copy, make a release package.
- `cd ui && npm run check`: type-check, test and build the client.
- `cd ui && npm run walkthrough`: the first-run story in a headless
  browser. Every other client check is
  `DAEMON=../target/debug/factory node scripts/<name>.mjs ../target/shots`;
  each is listed in [docs/working/client-checks.md](docs/working/client-checks.md).
- `cargo run -q -p cli -- fmt blueprints/std examples`: lay the shipped blueprints out.

Before pushing, run the gate on four processors, which is what the hosted
machines have: `CARGO_BUILD_JOBS=4 taskset -c 0-3 cargo xtask check`.

A program run from where cargo built it (`target/debug/factory`) reads
this checkout's blueprints, packages and client. An installed copy uses
what is built into it. To try a change, run a second factory from the
checkout with its own profile and data (`target/debug/factory up
--profile dev1`), and never against somebody's factory in use.

## The rules, and the gate for each

When a gate fails it says why it exists.

- **The name is written once** (`brand`). Only `brand.toml` and Markdown
  spell the product's name. Code uses `brand::DISPLAY`, `brand::CLI`,
  `brand::env_var(..)`; the client reads it from `/api/meta`.
- **No agent is known by name** (`providers`). Everything about one
  agent program is in its package under `extensions/`
  ([docs/extensions.md](docs/extensions.md)). A package's name, or its
  program's, appears nowhere else but Markdown; hosting services' names
  are in `extensions/reserved`. The two exceptions are written in
  `xtask/src/gates.rs`.
- **The runner and the blocks know no process** (`general`). `conductor`,
  `blueprint`, `runs`, `comparisons` and `blocks` name no card, board,
  stage, lane or fleet. What a piece of work is lives in blueprints and
  table rows. Exceptions are listed with reasons in `xtask/src/general.rs`.
- **Dependencies point one way** (`layers`), as listed above, dev-dependencies included.
- **Every dependency is declared once**, with its features, in the root
  `Cargo.toml`; members say `{ workspace = true }`. The licence is set
  once; members say `license.workspace = true`. **One test binary per
  crate**: `tests/suite/main.rs` plus modules; libraries set `test = false`;
  no `#[cfg(test)]` in `src/` (`manifests`).
- **Files stay under 1,000 lines, crates under 15,000** (`size`). Split
  before you add. Something new that is a pure function goes down to
  `kernel`, `reader` or `texts` first; see crates.md.
- **Nothing leaves the machine unasked** (`offline`). No telemetry, no
  update checks, no addresses of ours in the code. The gate runs the
  API and CLI suites with the network cut off and fails on any packet
  sent outside.
- **No sleeping in tests** (`sleeps`). Time is a `kernel::Clock`; tests
  use `ManualClock` and wait on events.
- **Formatting and lints** (`fmt`, `clippy`): `cargo fmt` and
  `clippy -D warnings`, no exceptions and no `#[allow]`.
- **Build time, test time and what one card costs are measured and held
  to `build-budget.toml`** (`budget`; the card cost in
  `crates/api/tests/suite/costed.rs`). A figure is raised only for
  something worth having, with the reason. See
  [docs/build-times.md](docs/build-times.md).
- **A shipped blueprint runs no shell script of ours** (`portable`). It
  runs `cmd:` (a program and its arguments) or `cli:` (`mw`'s own
  commands); `run:` is only ever wholly a user's own param.
- **A shipped blueprint decides nothing itself** (`plain`). Expressions
  in one name a thing, ask one plain question (`==`, `!=`, `in`, `??`,
  `!`, `<`/`>` against a literal number) or join up to three with all
  `&&` or all `||`. Anything more is a block in Rust, tested as a
  function with a table of cases, which the blueprint calls.
- **Shipped blueprints are laid out by the formatter** and **only get
  shorter** (`contract/lines.txt`, checked by the API suite's
  `lines.rs`; `WRITE_LINES=1`, or `GROW_LINES="why"` where one must grow).
- **The contract is written down**: what a blueprint may depend on
  (`contract/offered.txt`; `WRITE_CONTRACT=1 cargo test -p api
  the_contract`, `BREAK_CONTRACT="why"` for a break), each shipped
  blueprint's version (`WRITE_VERSIONS=1 cargo test -p api the_versions`),
  and the schema (`WRITE_SCHEMA=1 cargo test -p api the_schema_kept`).
- **Every event is described where it is emitted**, in its crate's
  `events.rs`, with who may hear it. A test reads the source for every emit.
- **Migration steps only grow at the end.** Each list of table steps
  (`Db::migrate`, `Journal::makes`) gets new steps after the last, never
  in between, and an existing step never changes. `contract/steps.txt`
  holds them (`WRITE_STEPS=1 cargo test -p api the_steps`).
- **Nothing is taken out of the blueprint language.** Running factories
  keep the texts they were applied with.
- **A device has no credential but the key it proves**: a Noise
  handshake and sealed frames (`link`, the `snow` crate); no cipher of our own.

The `ui` gate type-checks, tests and builds the client.

## How the code is written

- Tests are the specification. A test name is a sentence that says what
  is true (`a_failed_update_keeps_the_previous_version_running`). Tests
  drive the public surface: the API suite talks HTTP to a real node, the
  CLI suite runs the real binary.
- Comments say why, not what. Module docs say what the module is for.
- Errors are sentences for the person reading them, and say what to do
  next where there is something to do.
- New capability is usually a new **action** (`Registry::add_fn` in a
  crate's `actions.rs`), not a change to the runner. New behaviour is
  usually a **blueprint** under `blueprints/std/`.
- A word of the blueprint language is one entry in
  `crates/blueprint/src/vocabulary.rs`, with a sentence and an example.
  The reader, the editor and the Reference page take it from there.
- A record field the daemon reads is a model field in
  `crates/studio/src/models.rs`; nothing else reads a record's field by name.
- The client never polls: it reads once and follows `/api/events`. The
  daemon does not poll for what the system can tell it; every remaining
  timer has a row in `docs/architecture.md`, *Being told, not looking*.
- Everything that happens is an event; anything slow or fallible is also
  measured.
- A row and the event that says it was written are one write
  (`Journal::write`). Never write and then emit.
- Something new that changes a record holds its turn
  (`Collections::changing`); something new that deletes goes through the
  Trash; something new that hands out data asks who may read it.
- A process a test starts is ended by the test however it ends; a
  port a test hands to a program comes from `free()` of the stand-in's
  `ports.rs`, never from the system and let go. See testing.md.
- Client checks go by what the page shows and by element boxes, never
  by a longer wait. A check that fails now and then is a fault: crowd it.

## Topic files

Each file under [docs/working/](docs/working/) is readable on its own.
Read the one for the area you change.

- [crates.md](docs/working/crates.md): each crate, and where new code goes.
- [runner.md](docs/working/runner.md): instances, runs, handlers, texts, pins, comparisons.
- [restarts.md](docs/working/restarts.md): what happens when the daemon stops anywhere and starts again.
- [journal.md](docs/working/journal.md): events, journals per project, copies, moves, the database thread.
- [data.md](docs/working/data.md): tables, records, fields, migrations, windows, the Trash.
- [security.md](docs/working/security.md): scopes, keys, sessions, secrets, private tables, devices.
- [testing.md](docs/working/testing.md): rigs, ports, stand-ins, processes, Windows, flakes, the gate.
- [blueprints.md](docs/working/blueprints.md): the language, shipped-blueprint rules, versions, forks, summaries.
- [blueprint-tools.md](docs/working/blueprint-tools.md): Preview, the Reference, sandbox tests, side by side.
- [board.md](docs/working/board.md): cards, presses, sending back, cost, features, pools.
- [fleet.md](docs/working/fleet.md): the fleet's tables, stages, rounds, reviews, segments.
- [projects.md](docs/working/projects.md): project setup, pipelines, environments, feature flags.
- [agents.md](docs/working/agents.md): agents, turns, walls, programs, git, external agents.
- [channels.md](docs/working/channels.md): workspaces, channels, private channels, the bot, attachments.
- [inbox.md](docs/working/inbox.md): topics, the inbox, conversations, mentions, told lines.
- [client.md](docs/working/client.md): general client rules: tabs, drawer, events, themes, looks.
- [client-panels.md](docs/working/client-panels.md): the side panel and page heads.
- [client-tables.md](docs/working/client-tables.md): tables, views, columns and comments in the client.
- [client-checks.md](docs/working/client-checks.md): every headless-browser check, one line each.
