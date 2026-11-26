# Testing: rigs, processes, ports, stand-ins and the gate

This file covers how the test suites are built and run: test rigs, the
processes and ports tests start, the stand-in program, the simulation,
Windows, flaky tests and how to chase them, the gate, and the budgets
that hold build time, test time and the cost of a card. Read it before
you write a test that starts a daemon, a service or a program, before you
add a rig, and when a test fails only now and then.

Terms used here: a *rig* is a test's harness that starts a factory (a
daemon and its data); a *play* is a scripted run, in the conductor's suite
or the simulation, stopped at each point and restarted; the *stand-in* is
the test program in `crates/hosting/standin`.

## General rules

- No sleeping in tests. Time is a `kernel::Clock`; tests use
  `ManualClock` and wait on events, not on the wall clock.
- A test does not wait for what the system will not report
  (`Watch::problem`): the suites run many nodes at once and can use up
  the system's file watches.
- A rig that starts a node asks it what it emitted (`held`) when it puts
  it away, so every emitted event is checked against its description.
- A wait in the API suite that times out prints the last events
  (`Rig::event`); the conductor suite's waits do the same (`Rig::soon`,
  used by its `event` and `finished`; a wait that has a rig waits through
  it, not the bare `soon`). The gate keeps what a failed test printed.
  Read an intermittent failure from that, not from the code alone.
- A test's project is a folder under `target/`, inside this repository,
  so git run there with no repository of its own finds this one. A test
  of lanes runs `git init` in the project first, even to test one with no
  history.
- A repository a test makes says how its lines end and who commits
  (`lanes::git` in the API suite, after `init`), or git on Windows
  changes line endings on checkout.
- A path a test compares is a path, or is written as the product writes
  it: `kernel::whole::whole` where the product gives a whole path (never
  `canonicalize`, which gives the long form on Windows), `Path` against
  `Path` where a route returns one, and joined a folder at a time where
  it is matched as text (`dir.join("data").join(..)`).
- A check of the client finds a choice in a form by the start of its
  label (`getByLabel(/^Sprint/)`): the options of a `<select>` are part
  of its label's text.
- A script that edits a file and puts it back must `touch` it afterwards,
  or cargo keeps the edited text (`include_str!` of the standard
  blueprints).
- Build scripts must not watch paths that may not exist, or read the
  clock: either makes every build a rebuild (`docs/build-times.md`).
- A factory made for a test does not wait for the disk, and says so:
  `Db::open_unwaited` where a suite opens a journal's file (the runner's
  rig), `Config::disk` for a node in the test's process (the API and CLI
  suites' rigs), and `JOURNAL_DISK=unwaited` for a daemon the CLI suite
  starts (set where `Daemon` starts one). A daemon that is run is never
  told; what a power loss may take depends on the waits
  (`journal::checkpoint`). A test of the disk itself opens with `Db::open`
  (the journal suite), and only a waiting database counts a commit as on
  the disk (`Db::kept_to_the_disk`). A new rig that opens a journal's
  file, or makes a node, says the same.
- A node a test makes hashes the passwords registered with it with the
  least the hash takes (`Config::hashing`, `people::Hashing::Least`, in
  the API suite's rig): its owner is registered as each test begins,
  and hashed as hard as a run factory hashes, that was a twentieth of
  the suite. A test of how a password is kept says `Hard` itself
  (`access.rs`). A daemon that is run is never told.
- Something that decides by what was measured says what it does when
  nothing was measured: absent is never quietly a pass (a canary stage
  that saw no connection emits `slot.unwatched` in `std/slot`).

## Processes a test starts

- The daemon leaves services running when it stops, on purpose. A test
  or script that starts a service stops the instance that owns it, and
  does so however the test ends: a test that starts one has a guard that
  ends it (`Ended` in the API suite's `secrets.rs`, `Stops` in the CLI
  suite's `unsaid.rs`), and starts a program that ends by itself.
- A package's example that starts a service stops the owning instance
  before it ends (`{ "stop": ... }`).
- The gates that run test programs (`test`, `offline`) compare what is
  running before and after (`xtask/src/left.rs`, read from `/proc`) and
  fail for anything new that belongs to this checkout (its program is
  under `target/`, it is in a folder of the checkout, or a test program of
  this gate started it: `TESTS_OF`) and that nobody waits for, naming the
  crate whose tests left it. Something on its way out gets three seconds.
  The gate ends nothing. `cargo xtask left` lists what is running here
  that nobody waits for (`--none` fails if there is anything).
- The rigs end processes, each as it is put away, however its test ended:
  - A factory in the test's own process (the API suite's `Rig`, the CLI
    suite's, the agents', the gateway's, the hosting suite's, an
    example's) calls `hosting::end_under(<its folder>)`, which ends
    everything that factory started and nothing else.
  - A daemon that is a process of its own (the CLI suite's `Daemon` and
    the other things there that start one) is ended with everything of
    its folder (`ended_under` in that suite's `main.rs`: the daemon, its
    keeper, each wrapper with its group, whatever is in the folder),
    looked for until none is left. A test that starts a wrapper itself
    says `Ended(&dir)`.
  - Something is of a folder by being in it or by being that suite's
    program told of it; a user's own command that only names the folder,
    and anything with a terminal that is not the tests', is left alone.
    Something new that ends processes for a test finds them as narrowly,
    never by a name pattern alone.
- A new rig that starts a factory or a daemon does one of the two in its
  `Drop`. A stand-in that waits also ends by itself (`idle` is ten
  minutes).
- A line of shell that waits on something it started ends that itself
  with a signal that cannot be caught (`kill -9 $!` in its trap): a group
  asked to stop does not reach a child started a moment later.
- The client's checks do the same through `endsWithThis` in
  `ui/scripts/ends.mjs`: the daemon is told to stop as the script exits,
  whatever made it exit; a check that starts a service says what stops
  its instances first (`stopsAll`); and on exit a check asks the tooling
  what it left and fails for it. A new check calls `endsWithThis` for
  each daemon it starts.
- A process is over when the wrapper that started it has ended. The
  wrapper outlives its program (`stays` in `hosting::wrapped`, a handler
  that does nothing and is not inherited as ignoring would be), so
  anything that waits on a process waits on the wrapper, never on having
  asked it to stop. How the program ended is written beside its files and
  then renamed into place (`wrapped::say`) after all its output is in
  them, so a reader sees the whole exit code or nothing.

## The wrapper inside a test process

- In suites whose node runs in the test's own process, the program is
  the test binary and cannot be started as the wrapper. Those rigs say
  `Hosting::stands_between` (`Config::stands_between`), and the wrapper
  is then a thread of the test process (`Wraps::Here`,
  `crates/hosting/src/here.rs`) that waits for the program, removes
  handed secrets from its output, reports how it ended, and keeps going
  while a node is stopped and another started.
- Every rig that makes a `Hosting` or a node in its own process says so,
  the example's included (`examples/kv-cache/tests/suite/main.rs`). A
  factory in a process that has not declared itself the factory's own
  program (`wrapped::answers`, in the CLI's `main`) and was not told it
  stands between starts nothing and says at once what it must be told
  (`wrapped::own`; `crates/hosting/tests/suite/wrapped.rs`).
- Three differences from the real wrapper are listed at the head of
  `here.rs`: the program leads its own group, a program ended without
  being asked is reported as nothing, and nothing waits at a pipe. The
  real wrapper is tested with the real binary
  (`crates/cli/tests/suite/wrapped.rs`, and `unsaid.rs`, `outliving.rs`,
  `daemon.rs`).

## Ports

- A port a service is given is reserved from before its program starts
  until the service is no longer managed (`hosting::ports`). It comes
  from numbers no system hands out by itself (`ports::GIVEN`) and is
  reserved by a socket of the other kind at a number unique to the port
  (`ports::Spoken`), held by the daemon and gone with its process. It is
  released when the service is stopped, has ended for good or is given
  up on, and by `Hosting::halt`; a conductor that finds a service again
  reserves its port again. Something new that tells a program a port to
  take later calls `ports::free()` and keeps the result as long as the
  program may take the port. Never ask the system for a free port and
  let it go: the system hands it to the next caller.
- Whether something listens on a port is asked by connecting
  (`listened_on`), never by listening briefly: a program another thread
  starts in that moment inherits the listener.
- No test asserts that a released port can be reserved again, or that a
  conductor restarted in the test's process reserves what it found: a
  copy of the released socket may be in a program another test is
  starting. Tests check `Hosting::spoken_for` and that `ports::speak_for`
  refuses what is held; a returning daemon reserving a service's port is
  checked with the real program (`crates/cli/tests/suite/daemon.rs`), the
  rest in the hosting suite's `ports.rs` and the API suite's
  `given_back.rs`.
- A service restarted for no visible reason is read from its own log
  (`cannot listen on port …: Address already in use`) and its exit code
  (`service.exited`) before the code that manages it.
- A port a test or check gives a program it starts is never one the
  system gave and took back, in this order:
  - A program that can take its own port is told port 0 and reports
    where it is, and whoever started it reads that: the daemon prints
    `is running at`, which a check reads with `saysWhere`
    (`ui/scripts/where.ts`, reading the whole line; a daemon that exits
    first fails the check there). A test or check that listens itself
    listens on port 0 and keeps the listener.
  - Where the number must be known before the program runs (it is
    started again at the same address; `up` restarts it as started; its
    first output is not read), it is a port of the test's own
    (`crates/hosting/standin/ports.rs`, `free()`, shared by `#[path]`
    with the CLI suite, the API suite and the hosting suite's `rigs.rs`):
    26000 to 28999, clear of `ports::GIVEN`, reserved the same way as a
    service's port for the life of the rig (`Daemon` keeps it) or the test
    (`own_address` in the CLI suite's `profiles.rs`).
- Not held by the product, and so not by tests: a named factory or a
  router that took its own port and is restarted at the same address.
  The tests that are about exactly that do it with the wait the product
  gives a taken address; any other test gives its factory an address of
  its own.

## The stand-in program

- A test that needs a program starts the stand-in
  (`crates/hosting/standin/main.rs`, whose head lists every word it
  takes), never a line of shell. It is a `[[bin]]` of the crate whose
  suite starts it (`<crate>-standin`; other crates name the same file by
  `path`), so cargo builds it for that suite's tests
  (`CARGO_BIN_EXE_<name>`). It uses the standard library only; nothing in
  `src/` starts it and nothing that packages or installs copies it.
- A blueprint starts it with `cmd:` and each word in single quotes
  (`standin::cmd` in the hosting suite), since Windows paths are full of
  backslashes. Where a shell starts it (`run:`, a health check's `cmd:`)
  its file is in double quotes (`standin::line`).
- A stand-in that waits is told when to go on: `held <at> <who>` connects
  to where the test listens, says who it is, and continues when told a
  line or released; where nothing listens it continues, so one that
  outlives its test ends by itself. A rig's stand-ins are given a file
  holding the address; a test that holds one creates it (`hold`).
- Releasing: `let_go` waits until somebody waits there and releases
  everyone who is; `pass` releases a number of them, those waiting and
  then the next to come. Only a stand-in whose program is still running
  counts as waiting. A test that releases stand-ins one at a time gives
  each its own hold, because releasing a hold releases everyone at it.
- A test that releases a held stand-in more than once waits, between
  releases, for what the first went on to do (`featured.rs` in the API
  suite waits for each card to be done), so it releases the one it means.
- A stand-in reports that it began with `tell` to where the test listens
  (`Listening`, read by its own task so no line is lost). A test that
  stops a daemon while a stand-in agent has work waits for that report
  (`restarted.rs`, `Held::until_asked`): `agent.turn.started` is emitted a
  moment before the program exists.
- The shared helpers sit beside the program, one file each (`argv.rs`,
  `cmd.rs`, `line.rs`, `hold.rs`, `holds.rs`, `let_go.rs`,
  `listening.rs`), included by a suite's own `standin.rs` with `#[path]`,
  so no suite carries dead code.
- Output of two programs side by side has no order; read each program's
  lines on their own.
- A script stays a script only where the test is about a shell reading
  one, and is written for each shell where they differ (`script` in the
  hosting suite's `shell.rs`). A word the stand-in lacks is added to it,
  with a line in its table and a case in the suite's `standin.rs`, rather
  than writing shell.

## Processes and scripts in the product

- The wrapper that starts a program, waits for it and records how it
  ended is this program started with a word of its own
  (`hosting::wrapped`, `WRAPS`, handled by the CLI before anything else).
  It is found by where this program is (`proc::program`, `current_exe`
  read once at start, never canonicalised), never along `PATH`; where
  the program cannot say where it is, nothing is started.
- Stopping a program is a system call (`hosting::group`, a signal to its
  group), never a program that is run. Nothing the factory does for
  itself is shell or `kill`; the API suite's `shells.rs` reads the source
  for shell names, with exceptions listed with reasons.
- A user's shell text (`run:`, a health check's `cmd:`) is run by the
  system's shell through one function, `hosting::script::command`. On
  Unix the script is the shell's single argument. On Windows it is put on
  the shell's command line as written between the quotes that shell
  strips (`script::line`, passed with `raw_arg`), and a script of several
  lines is refused (`script::refused`). The wrapper is told it is handed
  a script (`Asked::script`, `--script`). Something new that has a shell
  read a user's text calls `script::command`.
- Something new that starts a program to be found again goes through
  `proc::Launch` or `hosting::kept`; something new that stops one goes
  through `group::stop`.
- A step sent to another device may run twice if that device's daemon
  restarts mid-step; the gateway removes every other way it could (module
  docs in `crates/gateway`).

## Windows

- No gate runs anything on Windows; the suites are run there by hand.
  Some modules and tests are marked for Unix, and each says beside the
  mark what of Unix it needs (a wall, a Unix-shell program, the real
  daemon stopped by a signal in `crates/cli/tests/suite/daemon.rs` and
  modules that use it, a signal, a named pipe, a link, a file mode).
  Nothing new is marked for Unix for want of a stand-in word, and a
  newly marked test says why. `docs/build-times.md`, *Run on Windows*,
  records the last run.
- A group on Windows is a job object that ends its members when the last
  handle is released (`hosting::group`, through `win32job`). The wrapper
  creates its job and joins it before starting its program
  (`group::heads`); a program started with no wrapper is put in a job the
  daemon holds (`group::led`), released by `group::stop`, by its leader
  ending (`group::ended`) and by the daemon ending. `group::stop` acts on
  what was last recorded of a number (`group::Known`, cases in the
  hosting suite's `group.rs`): something new that starts or finds a
  wrapper calls `group::wrapped`; something that starts a program with no
  wrapper calls `led` and later `ended`.
- Never end a process by its number (`sysinfo`'s `Process::kill` runs
  `taskkill`, and a number may be reused). A Windows wrapper is stopped by
  a word file beside its exit file (`group::word`), which it checks ten
  times a second and removes as it reads it (`group::hears`);
  `Proc::clean` removes what is left.
- Windows differences, listed in the README and `group.rs`: nothing is
  asked to stop gracefully (a stop always ends it; a stopped wrapper exits
  137); nothing a program started outlives its group; one process cannot
  be asked to stop (`group::ask` refuses, saying to use `down`). A
  program whose job is released exits with 0, so a test that needs a
  failure tells the program to fail (`rollouts.rs` in the hosting suite).
- What is Windows-only is kept thin over functions of plain values that
  run on Linux: `group::Known`, `hosting::named::file` (program names with
  `PATHEXT` endings; start programs by name through `named::program`),
  `hosting::cleared_env`, `hosting::script::line`.
- Windows reports writes to a file kept open only once, then on close.
  Anything that follows a running program's output calls
  `Changes::or_looking()`, which on Windows also looks four times a
  second (`TOLD_OF_A_FILE_KEPT_OPEN` in `hosting::watch`). Checked by the
  hosting suite's `shell.rs`.
- The Windows shell expands `%NAME%` before parsing the line, so a
  variable's value is run as part of the command. A user's line that is
  handed what somebody else wrote (`MESSAGE` of `std/bot`) names a program
  that reads the variable itself. Checked by the hosting suite's
  `script.rs` on Windows.
- Whole paths go through `kernel::whole::whole`, never `canonicalize`
  (Windows returns the long `\\?\` form). `whole` returns the plain form
  where it means the same (`plainly`, with cases run on Linux) and keeps
  the long form only where nothing else can say it. Whether one whole
  path is under another is asked of `kernel::whole::within` and `under`
  (`same` when walking up), never `starts_with` or `strip_prefix`. The
  kernel suite's `whole.rs` reads every crate for `canonicalize`; the
  tooling has its own (`whole` in `xtask/src/install.rs`).
- The user's home folder is `HOME`, then `USERPROFILE`
  (`brand::config_dir`). A whole path an earlier Windows version stored
  in long form is read in today's form wherever compared or handed on
  (`whole::said_plainly` in `crates/folders/src/registry.rs`,
  `agents::signins::folder_kept`); something new that stores a whole
  path reads it back the same way.

## The simulation

- A change to what happens at a restart is checked by the simulation
  (`crates/api/tests/suite/simulation.rs`, scenarios in `simulated.rs`;
  *Stopped anywhere, and started again* in `docs/architecture.md`). The
  gate plays the places that hold. Before and after such a change run the
  wider sets by hand, under the gate's lock:
  `SIMULATE=kinds cargo test -p api --test suite simulated:: -- --nocapture`,
  and `SIMULATE=everything` for every place. `SIMULATE_SHARE=2/4` plays
  the second of four shares of each scenario's places (`found::Share`), so
  shares can run as separate processes.
- A place known not to hold is a line of the scenario's `not_yet` with a
  sentence; take it off when it holds.
- A play is judged only at rest (`observed::at_work`: no run under way
  beyond what was meant, no agent turn with a program at work); one that
  never comes to rest fails as "did not come to rest". Something new that
  keeps working with no run and no turn is counted there.
- What a play came to (`Scenario::came_to`) is evaluated at every quiet
  moment, so it waits for nothing that cannot have happened yet; a rule a
  scenario waits for is waited for only once what it reads is there
  (`rolling.rs`).
- A look is no fault while the daemon has emitted something the play has
  not heard: that is heard first and the play looked at again (`listen`
  in `simulation.rs`, ordered by `looked.rs`). That check asks whether
  anything arrives in a moment's listening and whether the rig's order
  moved since the read (`Rig::mark`, `simulation/overtaken.rs`), never a
  journal's last number. Something new the judge asks after reading what
  the play came to is asked before that check.
- A play starts in a factory at rest. What sets a scenario's factory up is
  not a place of the play, so something to be stopped in is an act of the
  scenario, never its `stage` (`BroughtUp` in `simulated.rs`). A new way
  of setting up that keeps working with no run, turn or database work is
  waited for in `rested`.
- A place "never reached" means two things came in the other order that
  time; such a play writes down what the daemon emitted. Places are
  played again up to `found::PLAYS` times, and one no play reached fails
  (`Found::wrong`). A scenario that goes two ways by design says where and
  why (`Scenario::either`, matched in `either.rs`); an order that is only
  chance is designed out of the scenario instead (`Play::said`). Find
  which places differ between plays by listing them
  (`SIMULATE=places`) many times, crowded, one process per test.
- A scenario's acts are written as a person acts, looking first: an act
  the daemon went away in the middle of is done again and must not do
  twice what it had done.
- An agent turn is emitted as started a moment before its program runs.
  The place where the program is at work is `program.began`, which a
  stand-in reports down a pipe; what becomes of the program while the
  daemon is away is `Scenario::away`. A new stand-in reports that it is
  there before it waits, and that it ended after its work
  (`observed::node_with_stand_ins`).
- The simulation's tests sit under the module `abreast` in the API
  suite's `main.rs`, which comes first by name and so starts first; they
  run a few plays at a time (`at_once` in `simulation.rs`). Name one as
  `abreast::simulated::…` for `cargo xtask crowd` and `--exact`; a new
  module of scenarios goes there too. Nothing in a scenario may rely on
  the other tests being over.
- A play, and the factory its scenario sets up, are boxed where awaited
  (`play` and `simulate` in `simulation.rs`) to keep the test thread's
  stack small; something new awaited there that sets a factory up is
  boxed too.

## Flaky tests

- A test that fails now and then is a fault, found by crowding it:
  `cargo xtask crowd <crate> <test> [--times N] [--at-once M] [--cpus K]`
  builds the test program as the gate does and runs one test many times,
  several at once, on a few processors (40 runs, 4 at once on 2 CPUs by
  default; `--limit` seconds per run). It keeps each failing run's output
  under `target/xtask/crowd/`, refuses a test name the program does not
  list, counts a run that ran no test as failed, and counts runs cut off
  by the limit apart from failures. Name the test whole, with its module.
- A gate that passes on many processors can fail on four. Before pushing,
  run it on four: `CARGO_BUILD_JOBS=4 taskset -c 0-3 cargo xtask check`.
- Something that starts a daemon for each of many things takes turns, as
  many at a time as there are processors (the CLI suite's `packages.rs`),
  and is tried on one processor.
- A package case's step that sets something off (`sets_off` in
  `packages.rs`) waits for the run that step set off, found by what set
  it off (`Stage::set_off_after`), never the first run of that handler to
  finish. A case's daemon is given the suite's `JOURNAL_GATHERS`.

## The gate

- `cargo xtask check` must pass before work is done; `--changed` checks
  what local changes affect and `--fast` runs the static gates.
- The gate builds every test program in one build and runs them itself
  (`xtask/src/suites.rs`): `offline` runs `api` and `cli` once with no
  network; `test` runs the rest. Picking some crates' tests with `-p` or
  `--exclude` changes dependency features and compiles a second copy. A
  program runs from its crate's folder with `CARGO_MANIFEST_DIR` set; a
  test that reads anything else cargo sets passes under `cargo test` and
  fails in the gate.
- On sixteen processors or more the gate runs programs side by side and
  splits a suite of more than 64 tests into processes by test name
  (`xtask/src/sharing.rs`). A test that needs another to have run first,
  or to share its process, fails there. A share that ran nothing fails.
- `check --changed` treats a change outside every crate as affecting
  everything (`Scope::changed`), except shipped blueprints (compiled into
  `conductor`) and the client's files (only crates that name the changed
  file). Do not add a list of which tests read what.
- A package built here needs this machine's C library or newer
  (`objdump -T <program> | grep GLIBC_`). Releases are built by the
  `package` workflow on an older system; a local package is for trying.
- To try the tooling with a scratch `HOME`, run the built
  `target/debug/xtask` directly; `cargo` with a scratch `HOME` makes
  rustup fetch a toolchain into it.
- A menu entry names its program by its whole path (`xtask::menu`). An
  install writes it; a Linux package carries `menu.sh`, which writes it
  from wherever the folder is. An install told `--to` or `--bin` keeps
  the entry under `<to>/share` and never writes in a home.
- The status table in `docs/status.md` and the README's first-ten-minutes section are claims.
  When what is tested or a command changes, change them, and
  `CONTRIBUTING.md`. Each claim names the test files that check it; a
  claim with none is not made, or is put in the row for what is run by
  hand. *What is not held yet*, after the table, is the short list of
  what is known not to hold (longer in `docs/architecture.md`); add what
  you find and leave, remove it when fixed.

## Budgets

- Build times: `cargo xtask build-times` adds a line to
  `docs/build-history.csv`; the `budget` gate fails when that line is
  over `build-budget.toml`, when the Rust has grown a tenth since it was
  written (measure again), or when a dependency took the package count
  over budget (`docs/build-times.md`).
- Test time: a run of every test is a figure in that line
  (`test_run_s`), with each crate's share (`tests_by_crate_ms`). The gate
  times each test and names the slowest; `cargo xtask timings` lists the
  slowest fifteen, each program's longest test, and how many places and
  plays the playing tests made (`xtask/src/played.rs`). A test that takes
  seconds alone usually does by the thousand what a dozen would show.
- The cost of a card: the API suite works cards through the shipped
  fleet (worked, reviewed and merged; with a command a row lays in; with
  a council of two) in a factory at rest before and after, and checks
  the journal's own counts (events, instances, runs, pieces of work and
  writes, commits, log frames) against `[card]` of `build-budget.toml`
  (`crates/api/tests/suite/costed.rs`; *What a card is held to* in
  `docs/build-times.md`). It checks no time. Raise a figure only for
  something worth having, with the reason.
- A card costs the same beside 253 cards as beside three, by every
  database's own steps: something new done for one card reads that card,
  or one column's cards, by an index (`studio::column`), never the whole
  board. A card and a move cost the same beside a large table of runs as
  beside a small one: what looks for a factory at rest asks what is under
  way, never the newest runs.
