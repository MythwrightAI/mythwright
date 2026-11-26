# Working on it

## Contributing

- **Open an issue first**, for a bug or for a feature, using the
  templates: [a bug](.github/ISSUE_TEMPLATE/bug.md) or [a
  feature](.github/ISSUE_TEMPLATE/feature.md). Questions and ideas that
  are not yet an issue go in the repository's Discussions.
- **Pull requests are taken for issues marked `accepted`.** One for
  anything else may be closed without review; say in the issue what you
  would like to do, and wait for it to be accepted.
- **Issues are read as time allows.** No response times are promised.
- **Every commit carries a Developer Certificate of Origin sign-off**
  (`git commit -s`, which adds `Signed-off-by: Your Name
  <you@example.com>`): it says you wrote the change, or have the right to
  hand it on, as the [DCO](https://developercertificate.org/) sets out. A
  pull request whose commits are not all signed off is not merged.
- **What you contribute comes in under the repository's licences**, MIT
  or Apache 2.0 ([Licence](#licence), below), with no other terms.
- **Be kind.** The [Code of Conduct](CODE_OF_CONDUCT.md) applies to
  issues, Discussions and pull requests alike.
- **A security problem is not an issue**: report it privately, as
  [SECURITY.md](SECURITY.md) says.

## The gate

```sh
cargo xtask check            # every gate; it must pass before a change is done
cargo xtask check --changed  # only what local changes affect (a change outside every crate affects everything)
cargo xtask check --fast     # the static gates only, under a second
cargo xtask timings          # how long each gate has been taking, and the slowest tests of the last run
cargo xtask crowd <crate> <test>  # one test many times, several at once on a few processors: for one that fails now and then
cargo xtask left             # what is running here that a test or a check started and nobody waits for; it ends nothing
cargo xtask ways [one] [own] [gathered] [ungathered]  # the suites played another way than the gates play them; all four where none is named
cargo xtask changes          # how many commits needed a rebuild, week by week
cargo xtask notices          # after a dependency changed: the notices a package carries, made again

cd ui && npm run check       # the client: types, tests, build
cd ui && npm run walkthrough # drive the first-run story in a headless browser, saving pictures
```

Each rule of the repository is a gate, and a gate that fails says why it
exists. The rules, and the things that are easy to get wrong, are in
[AGENTS.md](AGENTS.md) and the topic files under
[docs/working/](docs/working/): they are written for an agent working here
and are as true for a person. In short:

- The product's name is written in one place, `brand.toml`.
- The application knows no agent, and no other product, by name: each is a
  package under `extensions/`.
- Dependencies point one way, down the table below.
- One test binary a crate; a test's name is a sentence saying what is
  true; tests drive the public surface and never sleep.
- A test ends what it starts, however it ends: the daemon leaves its
  services running on purpose, so a rig ends what is of its folder as it
  is put away, and a stand-in that waits ends by itself as well. The
  gates that run the tests look at what is running before and after and
  fail for what was left, naming it; a check of the client says of its
  daemon that it ends with the check (`ui/scripts/ends.mjs`).
- Every event is described where it is emitted, and what a blueprint can
  depend on is written down in `contract/` and held to.
- Blueprints that ship run no shell script, work nothing out for
  themselves (what is decided is a building block in Rust, tested as a
  function with a table of cases, which they ask), and are laid out by
  the formatter. The gate `plain` lets nothing through: its list of
  what was still to be made plain is empty and stays so.
- A blueprint that ships only gets shorter: each is held to the lines it
  has (`contract/lines.txt`; `WRITE_LINES=1 cargo test -p api the_lines`
  when one got shorter or is new, `GROW_LINES="why" …` where one has to
  be longer, which keeps the reason).
- A blueprint that keeps a list of what it has in hand changes it an
  entry at a time, with `keep` and `drop`, never with `set` and a list it
  worked out.
- A blueprint that ships says its `version:`, and a version is one
  meaning. Change what one means and run `WRITE_VERSIONS=1 cargo test -p
  api the_versions`: it gives the text its next number and writes its
  line in `contract/versions.txt`, and the two are committed together.
  Laying a text out, or a comment, is no change.
- The steps a database's tables are made by only grow at the end: a
  step that is there stays where it is, word for word, and a new one is
  added after the last, since a database that has run a list's first
  steps runs only those after. Each list is held to what it was
  (`contract/steps.txt`; `WRITE_STEPS=1 cargo test -p api the_steps`
  when a step was added last or a list is new, `BREAK_STEPS="why" …`
  where a step has to be changed where it stands, which keeps the
  reason and is built into the program).
- Nothing is taken out of the blueprint language, but for the words the
  notes named before the preview as ones that may go
  ([docs/blueprints.md](docs/blueprints.md), *Words that may be taken out
  after the preview*).
- Files stay under 1000 lines and crates under 15000. A crate let past
  that for a while is said with its reason where the gate is (`ROOM` in
  `xtask/src/gates.rs`), and is owed a split. None is now. What the
  runner keeps on disk became a crate of its own (`runs`), comparisons
  one after it (`comparisons`), the actions it brought itself a third
  (`blocks`), and where its texts are found and what it keeps of them
  a fourth, below it (`texts`), and what it keeps of the factory's
  variables (its profile's) a fifth, beside that (`environments`). What a fleet's blueprints ask
  that uses no agent came out of `agents` (`deciding`), which is under
  its ceiling.
  Who is who came out of `studio` (`people`), and where people and
  agents talk (`talk`), and later what groups projects and channels
  (`workspaces`) and the search index (`search`).
- `docs/status.md` names the test that holds each claim, and says what
  is not held yet; the README says it in short. Change them with what
  they describe.
- Nothing leaves the machine unasked, and a gate runs the whole product
  with the network cut off to hold it to that.

## How it is put together

| Crate | What it is |
|---|---|
| `brand` | The product's name, from `brand.toml`. |
| `link` | What a link between two installations is made of: the installation's secret and the keys made from it, the handshake and sealed frames. On nothing else here. |
| `blueprint` | The blueprint language: reader, checker, formatter, graph. Pure; no I/O. |
| `kernel` | The few types everything shares: events, principals, clock, errors. |
| `journal` | The factory's SQLite file and one for each project that has a journal of its own: events, measurements, key-value state, logs. |
| `runs` | What the runner keeps on disk: instances, runs, steps and the places of handlers, each written with what is said of it. No engine. |
| `people` | Who is who: accounts and sessions, invitations to register, the credentials of keys, agents and devices. On the journal alone; `studio` opens it and offers it under the names it always gave. |
| `texts` | Which text something running goes on with: where texts are found (a project's own, the shared ones, the ones that ship), what a text reaches, what a tree is held to, what was vouched for and how a changed one is come by, kept in each conductor's own journal by one keeper. No engine and no registry; `conductor` offers it under the names it always had, and answers the two things it asks back. |
| `environments` | What a blueprint reads of where it runs that somebody said: the variables of the factory's profile (`profile.vars`), kept in the factory's journal and held for every conductor, with what a change of them comes to. `conductor` offers it under the names it always had (`conductor::profile`, `Engine::variables`, `Engine::set_variables`) and says what changed itself. A project's environments (named sets of variables and secrets, one of which an instance of the project is applied under) are to be kept here beside them. |
| `conductor` | Runs blueprints: durable instances, runs, steps, hooks, triggers, and how a running one takes a new text. |
| `blocks` | The three actions the runner used to bring itself: `pool.turn`, `text.put`, `profile.set`. Each asks the conductor what anybody may; what each works out is `kernel::pool`, `blueprint::parts` and the conductor's own variables. |
| `talk` | Where people and agents talk: channels, who is in each, what was said and what is attached to it, the factory's own bot and what it does when it is spoken to (by asking the daemon's routes, which it is handed, as whoever spoke), and what a blueprint can do in a channel. On the conductor; `studio` opens it, offers it under the names it always gave and adds its actions where they always were. |
| `workspaces` | What gathers the projects, and the channels, that belong together: a name and what is in it, and nothing else. On `talk`, knowing of the projects only their table; `studio` opens it, hands its projects what it writes with a project's row, and offers it under the names it always gave. |
| `search` | The index of everything a project knows (files, blueprints, what was said, cards), in a database of its own, kept current from events. On `talk`; what it reads of files and cards it asks back of `studio` as plain values, which offers it under the names it always gave. |
| `comparisons` | One blueprint started several ways, side by side: the ways and their shares, what a way reads instead, what came of each thing counted. The conductor asks it; it knows no more of the work than the conductor does. |
| `hosting` | Processes, long-lived services, health checks, the weighted router. |
| `studio` | Projects, collections of records, the board, libraries of documents; and who is who (`people`), where they talk (`talk`), what groups projects and channels (`workspaces`) and search (`search`), under the names each had here. |
| `deciding` | What a fleet's blueprints ask so as to work nothing out themselves: what a turn, a round, a review and a list of things done come to. Functions of plain values, each with its table of cases and the action that asks it. No agent; `agents` offers each under the name it always had. |
| `trash` | The Trash for whoever reads it: what was deleted that they could read, given back and deleted for good by whoever could have deleted it, emptied after the days the owner says, and what a stop left half done with a file finished as a daemon starts. The writes that move a thing in and out are `studio::trash`, each one write with what is said of it. |
| `agents` | Slots, turns, the one adapter that runs any agent from its package, the sign-ins the factory holds for their programs, and the blueprints packages bring. |
| `sandbox` | A sandbox test: a blueprint run in a factory of its own, made of nothing of the live one. Beside `agents` and on nothing above `deciding`: it is handed texts and what each action says of itself, and has a stand-in for every action that reaches outside the tables. |
| `gateway` | Other devices: a member links out to its hub; the hub runs steps on it. |
| `folders` | A factory's data folder worked on as files: who has it, what it is called, the factories beside it on the device, a copy of it and a factory started from one, a project moved to a journal of its own, the database written out again. Serves nothing; `api` asks it and offers each part under the name it always had. |
| `setup` | What a project and an agent are set up with as somebody asks: what a new project starts with, an agent added by hand (applying `std/agent`), an example put among a project's own blueprints. Each applied or written as whoever asked; serves nothing, and `api` offers each under the name it always had. |
| `served` | What the daemon serves beside the routes of its API, and what every request is held to first: the client's files, the names it answers to, what is sent past a route's limit, a library's pages, mockups, and the store program. Handed what it needs of the node; `api` offers each under the name it always had. |
| `api` | The daemon: HTTP API, event stream, everything assembled into one node. |
| `cli` | The command, which is also the daemon. |
| `desktop` | A window onto the client. Built only on request (`-p desktop`). |
| `xtask` | The repository's own tooling: the gates, installing, packaging. Standard library only. |
| `ui/` | The client: TypeScript, Preact, CodeMirror. |
| `blueprints/std/` | The blueprints that ship. |
| `extensions/` | Packages for what the application does not know by name. |

[docs/architecture.md](docs/architecture.md) has the decisions behind this,
and [docs/roadmap.md](docs/roadmap.md) what is intended and not built. Read
both before proposing new structure.

## Working beside a factory you use

A daemon run from this checkout (`cargo run -p cli -- up`, or the program in
`target/`) reads the client, the standard blueprints and the packages from
the checkout as they are, so changing them never rebuilds Rust and never
restarts that daemon. An installed copy, or a package, reads none of it.

So run a second factory from the checkout, with a profile of its own,
which gives it its own data and its own address:

```sh
cargo run -p cli -- up --profile dev1     # or target/debug/factory up --profile dev1
mw --profile dev1 status                  # the same commands, said to that one
```

`cargo xtask install` puts a copy where it is used, and `cargo xtask
package` makes the folder that is released.

## Adding or updating a dependency

A package carries the notices of what it bundles: every crate the
programs are built from and every package in what the client's build
writes, each with its licence and that licence's text
([THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md)). The file is made by a
command and committed, so after adding, updating or taking away a
dependency, of the programs or of the client:

```sh
cargo xtask notices          # makes THIRD-PARTY-NOTICES.md again; commit it with the change
cargo xtask notices --check  # says whether the one there is what would be made, and writes nothing
```

The programs' half is read by a tool, [cargo-about](https://crates.io/crates/cargo-about),
at the one version the file is made with. Its program is built only with
the feature named here:

```sh
cargo install cargo-about --locked --version 0.9.3 --features cli
```

It is needed for this and nothing else. A build, the tests, the gate and
`cargo xtask package` need no tool: a package copies the file as it is
committed. And a change to the client's packages alone needs none either:
the client's half is said by the client's own build, and the programs'
half is kept as it is while nothing it was made from has changed. The
tool fetches nothing and asks no service; it reads each crate's licence
where cargo keeps the crate's source, so on a machine that has never
fetched the crates of the other systems, `cargo fetch` once.

What holds the file is in the gate, and needs no tool either. A test of
the tooling fails, saying to run the command, where the crates the
programs can be built from are no longer the ones the file was made from
(a crate added, updated or taken away, or one that only tests used now
built into a program), where `about.toml` or `about.hbs` changed, or
where a library it names is not the version that is locked
(`xtask/tests/suite/notices.rs`). A crate only the tests use, and a tool
that only builds or tests the client, are in no package and move
nothing. And the client's check fails where a package is in what was
just built and not in the file (`cd ui && npm run check`).

Which licences a library may be under is one list, `accepted` in
[about.toml](about.toml), for crates and for the client's packages alike.
Each asks only that its notice and its text go with a copy. Where a new
dependency is under one that is not there, the tool refuses to make the
file and names the crate and the licence:

- one that asks no more than its notice is added to the list, with a
  word of why beside it;
- one that asks more (its source offered with the program, or the
  program handed on under the same terms) is the maintainer's to accept
  and nobody else's, one library at a time, as an exception written in
  `about.toml` under that library's name with what the licence asks and
  when it was accepted. The licence is never added to the list. There
  is one today, `option-ext`, and the file says under it where its
  source is; a test holds that a library under such a licence has that
  line.

A crate whose licence text the tool cannot tell (a file under a name it
does not look for, a text it does not recognise) is written into
`about.toml` with the file and why, never skipped. One that carries no
file at all is said by the standard text of the licence its manifest
names, and the command lists those each time it is run. A crate that
builds in somebody's code that is no crate (C built from source, a
library carried built) is read for that when it is added, and what it
holds is said in `BUILT_IN` in `xtask/src/notices.rs`.

## Proposing a change

- Say what it is for before how it is done. For anything that adds to the
  blueprint language, or to what a blueprint can depend on, open an issue
  first: those are kept few on purpose.
- A change comes with the test that says what is now true, and passes
  `cargo xtask check`.
- A change to where anything is kept (which journal, which file, which
  table), or to how the journal commits, passes `cargo xtask ways` too.
  The gate plays every suite as a daemon is: a project that is added has
  a journal of its own. That command plays them the other ways they
  are to hold, three of them a run of every test again: with every
  project in the factory's own journal, as a factory an earlier program
  made keeps them and every upgrade finds them; with the plays of the
  runner's suite each a project's with a journal of its own; and with
  whatever waits for a database kept by one commit. The fourth is the
  journal's own suite with every piece of work kept by a commit of its
  own. A test of a project's own journal asks first (`one_journal` in
  its suite's `main.rs`) and is not played the first way, saying so.
- New capability is usually a new action, and new behaviour is usually a
  blueprint under `blueprints/std/` that says what exists, what is heard,
  who may and when. What it has done step by step is an action.
- An action's `summary` is read by people who do not write blueprints:
  it is what a step that says nothing of itself is said by where a
  blueprint is said in plain words (`mw explain`, the Summary of its
  page). Write it as a sentence, in the imperative, whose first full
  stop ends what the action does. An action that makes something that
  is there after the run says so (`.making("a table", "name")`), and one
  that runs a command says which of its arguments is the command
  (`.runs("cmd")`): nothing lists actions by name to work either out.
  And an argument that is a mapping with words of its own (a table's
  `fields`, a health check) says what is written inside it
  (`.within("health", ROWS)`: rows of where, what kind and a sentence,
  as `blueprint::inside` says them), in place of a long sentence that
  lists its keys: the editor, `mw actions`, the schema and the table in
  `docs/blueprints.md` are made from the rows, and a test holds them to
  the code that reads the words.
- A security problem is not an issue: [SECURITY.md](SECURITY.md).

## Branches: `release` by day, `main` by night

- **`main`** is what has passed the hosted gate. It is protected: a
  change reaches it only through a pull request, and only once the check
  *the gate, on Linux* has passed on it. The nightly is built from it.
- **`release`** is where finished work lands during the day. Each piece
  has passed `cargo xtask check` where it was made (and `cargo xtask
  ways`, where it changes how anything is kept) before it lands.
- **Each night** that `release` has moved, one pull request takes it to
  `main`. The hosted gate runs once on it, and it is merged with a merge
  commit, so what landed that day reads as the commits it was.
- **Pull requests from outside** are opened against `release`. The gate
  runs on them as well, once a maintainer has let the workflows run.
- **A pinned line** has a branch of its own, `stable/<minor>`
  (`stable/0.1`), as the next section says.

## Releases: a pinned line and the nightly

There are two lines of what is released, and nothing in the program
moves anybody from one to the other: the README says how a person does,
under *Which build*.

- **Pinned.** A tag `v<version>` is a release, built by
  `.github/workflows/package.yml` for the three systems, whichever branch
  it was made on. The tag and the version in the root `Cargo.toml` are
  the same (the workflow refuses otherwise), and the newest heading of
  `docs/releases.md` is that version (a test of the API suite,
  `releases.rs`, refuses otherwise).
- **Nightly.** `.github/workflows/nightly.yml` builds main once a night,
  where main has moved since the last nightly and the gate passes on that
  very commit, into one release called `nightly` whose files are
  replaced each time. Its version is the one in `Cargo.toml` with the
  night and the commit (`0.1.0-nightly.20261124+abc1234`,
  `cargo xtask package --nightly 20261124`), which the program says where
  it says its version; there is no heading of `docs/releases.md` for a
  night, and the notes are written for the next pinned version.

A fix that a pinned line is to have is made on main first, as any change
is, and then carried to the line's branch, `stable/<minor>`, which is
made from the tag the first time:

```sh
git switch -c stable/0.1 v0.1.0        # the first fix only; after that `git switch stable/0.1`
git cherry-pick -x <commit>            # each commit of the fix, oldest first
# Cargo.toml: version = "0.1.1"; docs/releases.md: a heading `## 0.1.1: ...` above 0.1.0,
# saying what was fixed
cargo xtask check                      # the whole gate, on the branch, as anywhere
git commit -am "0.1.1"
git tag v0.1.1
git push origin stable/0.1 v0.1.1      # the tag makes the release
```

What cannot be carried as it is (main has moved under it) is written
again on the branch, small, with its test; a fix that changes how
anything is kept (a step added to a database's tables) is not carried to
a pinned line at all: the steps would stand there in another order than
on main, and the next version would refuse every journal a program of
the line had made (*The steps a database's tables are made by only grow
at the end*, in `AGENTS.md`). Main's own version
stays where it is until the next pinned version is made from it: the
next minor (`0.2.0`), with its notes, a commit that raises `Cargo.toml`,
and its tag on main.

## Licence

Offered under either of two licences, as whoever uses it chooses:
[MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE). Unless you say
otherwise, anything you intentionally submit for inclusion in this work, as
defined in the Apache 2.0 licence, is offered under both of those, with no
other terms or conditions.
