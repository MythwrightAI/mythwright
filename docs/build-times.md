# Build times

Iteration speed was the main trouble with the old codebase, and it had
been measured there carefully. This repository starts from those results
instead of rediscovering them. Each rule below says what it is, what enforces
it, and what was measured. Numbers marked *old* are from the old codebase's
own experiments (`docs/build-time-*.md` and `docs/build-footprint-*.md`
there); numbers marked *here* were taken in this repository.

## Where this repository stands

Measured here on 2026-11-10: 32 cores, but shared with a busy fleet (load
average between 4 and 24 during the runs), builds limited to 8 jobs and niced.

| What | Time |
|---|---|
| Clean dev build of everything | 70 s |
| Build every test binary after that | 3.7 s |
| Run every test | 6.0 s |
| Rebuild after adding a function to `kernel` (9 crates recompile) | 1.9 s |
| Rebuild after adding a function to `conductor` or `api` | 2.0 s, 1.7 s |
| Edit a conductor source file, then compile and run its tests | 10 s |
| `cargo xtask check`, all gates, nothing to rebuild | about 13 s |
| `cargo xtask check --fast` (static gates) | under 0.4 s |
| Clean optimised build of the daemon, `--profile fast` | 49 s |
| Clean optimised build of the daemon, `--release` | 63 s |

The daemon is 13 MB as a release build, 22 MB with `--profile fast`, 67 MB as
a dev build. The whole target directory after dev, fast and release builds is
2.8 GB. About 23,000 lines of Rust and 9,000 of tests at the time.

The client builds in 0.2 s and its checks (types, tests, build) take about
one second. It is served from disk, so a change to it rebuilds nothing else.

`cargo xtask timings` shows how long each gate has taken over time.

Measured again on 2026-11-15, on the same machine, with 61,500 lines of
Rust, 47,000 of tests and 378 packages: nearly three times the code.

| What | Time |
|---|---|
| Clean dev build of everything | 55 s |
| Build every test binary after that | 2.3 s |
| Run every test | 33 s, and 38.5 s on a second run that day |
| Rebuild after adding a function to `kernel` | 3.9 s |
| Rebuild after a change to `cli` | 0.9 s |
| Edit a conductor source file, then compile and run its tests | 3.5 s |
| `cargo xtask check`, all gates, nothing to rebuild | about 16 s |

Compiling has stayed where it was. What grew is the running of the tests,
which is now most of the gate: 1,088 of them, and the API suite, which
starts a real node for each of its 251, is 21 s of the 33 as cargo runs
it. The gate was 95 s the day before, when two tests that each looked at
thousands of passwords, which are slow to look at on purpose, were half of
the whole run; nothing said so, and they were found by timing each suite
by hand. It was 61 s that morning, 45 s once it stopped running the two
suites that drive the whole product twice, and 16 s once it ran the test
programs side by side (both below, under *The rules*). Every test is now
timed and the whole run has a budget (*Kept watch on*).

Measured again on 2026-11-16, with 68,700 lines of Rust and 55,600 of
tests: every test takes 75 s as cargo runs it, one program at a time, and
the gate about half a minute of tests with the programs side by side. The
API suite is 56 s of the 75. What it gained is the simulation (*Stopped
anywhere, and started again* in [architecture.md](architecture.md)):
twelve scenarios that come to 52 plays, each a factory of its own set up
(1.3 s), played through, stopped at a place and started again (0.65 s a
start), about three seconds apiece and four at a time in a process. As
cargo runs the suite its other tests are over at 34 s, and the plays go on
alone to 55 s. The budget for a run of every test was raised from 60 s to
90 s for that. What was found first, by writing down where each play's
time went: about one play in fifty waited half a minute for something it
had already been told. It counted the places the journal had passed a
moment before the one it waited for was counted, and was not asked again
until the node next said anything, which at the end of a play is its
sample of the device, thirty seconds after it started. That was ten
seconds of every run, and thirty-odd of whichever test it fell in, a
different one each time; a play now asks again whenever nothing is being
said.

Measured again on 2026-11-19, with 105,000 lines of Rust and 103,000 of
tests, twice in a day. In the morning every test took 296 s of the 300
a run may take (138 s the API's suite, 129 s the runner's), and nothing
more could be added. In the evening, on the same machine at rest, 107 s:
71 s the API's suite, 19 s the runner's, 10 s the command line's, and
the agents' 1.1 s where it was 7.4 s. No test was taken out, made
shorter or made to hold less, and four were added. The ceiling was
brought down with it, from 300 s to 150 s (`build-budget.toml`), so that
it warns again long before the tests are as slow as they were. What it
was, in the order of what each was worth (*Where the tests' time was on
2026-11-19*, and the rules it became, below):

| What | A run of every test |
|---|---|
| As it was | 296 s |
| The database library keeps no count of the memory it has in use, and a factory made for a test does not wait for the disk | 151 s, on a machine with other gates running |
| The runner's rig made once, the plays of a test made several at a time, eight plays of the simulation at a time | 127 s |
| The simulation's tests begun before the others | 107 s |

A rebuild from the bottom was 4.0 s and from the top 1.0 s, as they
were: nothing was added to what is built, and the one thing said to
the build is a definition for a file that was already compiled. A gate
in a lane of eight processors went from some six and a half minutes to
under three: 47 s its `test` where it was about 200 s, and 107 s its
`offline` where it was about 180 s, before the simulation's tests were
begun first.

Measured again on 2026-11-20, with 115,000 lines of Rust and 119,000 of
tests, three times each way in a lane of eight processors with other
gates running beside it. Every test took 184 s, 188 s and 191 s, over
the 150 s a run may take: 59 s, 59 s and 64 s of it the runner's suite,
which had been 19 s the evening before. What it had gained in between
is what makes its plays the same on any machine: a play holds its
databases from when it is begun until it is over, so everything it
does goes forward a turn at a time, and a play came to some two
hundred turns. After, 139 s, 137 s and 139 s: the runner's 26.4 s,
26.7 s and 26.7 s, and the API's 81 s to 84 s where it was 91 s to
99 s. The plays are the ones they were, and each is answered what it
was (the count of them is held: `played.txt`). What it was, with the
runner's suite alone on those eight processors:

| What | The runner's suite |
|---|---|
| As it was | 60 s, 385 s of processor time |
| A turn in which nothing was asked of any database is answered where the test stands | 47 s, 300 s |
| Twice as many plays at a time as the program runs tests | 36 s to 39 s, 280 s |
| The runner and the four crates under it built at the first level of optimisation | 26 s to 30 s, 195 s to 220 s |
| The runtime, the database's binding and what reads JSON built fully optimised | 21 s, 150 s |

What was looked for first was a wait, and there was none to find: the
suite kept six of its eight processors busy, a short spin in place of
a thread's parking made it slower, and a runtime with no driver for
the system's events took the same. A turn cost some 250 millionths of
a second of processor time, a fifth of it the eight times a play gives
way and the rest the database's thread and the runner's own code,
unoptimised. A quarter of a play's turns ask nothing: a play waits a
dozen of them each time it is to be at rest, and each was a round trip
to the database's thread and two commits of nothing. Of what a play
takes, three fifths is its turns, a tenth its being set up, and three
tenths its conductor being stopped and started (a twentieth of the
whole opening its files again). The longest test of the runner's, which
plays one thing behind thirty-two amounts of what a start has to do,
takes three seconds by itself; it is the longest only because its
plays wait their turn among everybody's from the first to the last,
so making its amounts side by side would have gained nothing.

A rebuild from the bottom was 4.7 s to 7.2 s before and 4.7 s to 6.7 s
after, and from the top 1.1 s to 1.5 s and 1.2 s to 3.5 s, each with
the other gates' load in it: the crates that are now optimised are
built a little at a time as they were. Building the runner's test
program for the first time with them took some forty seconds more,
once.

## Kept watch on

The old codebase reached builds of fifty minutes a little at a time, with
no one change to blame. So what a build takes here is measured the same way
each time, written down with the source, and held to a budget:

```sh
cargo xtask build-times           # a few seconds: adds a line to docs/build-history.csv
cargo xtask build-times --clean   # also builds from nothing, in a folder of its own
```

It times a rebuild after a change to the crate everything is built on
(`kernel`), after a change to the one built on everything (`cli`), after
a change to the runner with it alone built (`conductor`, the crate that
comes nearest its ceiling of lines time and again: `rebuild_runner_ms`,
so that its growth shows as seconds), the test binaries, and a run of
every test once they are built; with `--clean`, a build from nothing. Each line also says how many lines of
Rust and how many packages there were, so the history shows what growth
has cost. A line from before something was measured has nothing there,
and is not held to it.

[build-budget.toml](../build-budget.toml) says what each may take: half a
minute for a rebuild from the bottom is the most that is acceptable. The
`budget` gate, which costs nothing to run, fails when

- the last line of the history is over budget, for a build or for the run
  of every test (`test_run_s`);
- the Rust has grown by more than a tenth since that line was written, so
  that it no longer says what a build takes: measure again, with nothing else building: a measure taken while
  other builds run on the machine says what they cost, not what this
  one does;
- `Cargo.lock` names more packages than the budget allows. A dependency is
  compiled by everyone on every clean build, so one is added on purpose and
  the budget raised with it.

First line, 2026-11-11, on the same machine while it was quiet: 27,353
lines of Rust, 357 packages, 1.4 s from the bottom, 0.6 s from the top,
45 s from nothing.

The budget is on the whole run because that is a figure that can be taken
the same way twice. One test's time cannot: the gate works it out for
every test, from the line a test program prints as each one finishes and
the order it starts them in (`xtask/src/clock.rs`; within a millisecond of
tests that wait a known time), but that is its time with the others
running beside it. A test that starts a whole fleet takes about five times
as long there as it does alone. So a test's time is shown and nothing is
decided by it: the gate names the slowest on its own line every run, and
`cargo xtask timings` lists the slowest fifteen and what each crate's
suite took.

A run of every test is one program after another, so what it takes is
the sum of what each program takes, and a program is never over before
its longest test is. Both are kept. A line of the history says what each
crate's tests took of the whole (`tests_by_crate_ms`, the longest first,
those under a second put together), read from what cargo and the
programs print as they go: it is still cargo that is timed. And `cargo
xtask timings` shows the two runs it knows of apart, since they are two
figures: every test as `build-times` last ran them, which is what the
budget holds, and the tests as a gate last ran each crate's, side by side
where there is room or held to a few processors. For each crate it says
how many tests, how long the program ran, and which test of it took
longest; a change that is to shorten a run of every test shortens the
first crate of that list or does nothing.

Two suites play one thing stopped at every place it passes (the runner's
plays, and the simulation of a factory), and what such a test takes is
how many plays it makes. A suite that plays writes that down where
whatever runs it to time it asks (`PLAYED_TO`, a file: `xtask/src/played.rs`;
`played` in each of the two suites), the gates and `build-times` keep it
beside the times, and `timings` shows it: for each crate, and beside each
of the slowest tests. A program run by anybody else is told nothing and
writes nothing.

## The rules

### Debug info is line tables only

`[profile.dev] debug = "line-tables-only"`.

*Old:* 80.5 % of 46.6 GB of test executables was debug sections. With line
tables only it was 46.5 %. Backtraces keep their `file:line`.

The saving is disk, not time. The old codebase tested the belief that builds
were waiting on the disk and found they were not: 1.5 % I/O wait during a
cold build. Cutting debug info further (`debug = 0` for dependencies) saved
13.5 % of bytes written and no measurable time, so it was not adopted there
and is not here.

### No `lto` or `codegen-units` in the dev profile

*Old:* `lto = "thin"` in dev took 7401 s against 381 s (19.4 times slower).
`lto = "off"` is already the default. `codegen-units = 1` was 16.4 % slower;
`256` made no real difference; and setting the key at all cost about 8 s,
because it stopped host and target builds sharing 64 dependency crates, which
were then compiled twice.

The lesson is the last part: a profile key that looks harmless can double
work. Leave the dev profile alone unless a measurement says otherwise.

### One test binary per crate, and libraries are not built twice

Enforced by the `manifests` gate: tests live in `tests/suite/main.rs` plus
modules, and every library sets `test = false` and `doctest = false`.

*Old:* folding one crate's 289 test targets into 29 took the target directory
after one run from 46.0 GB to 8.2 GB, cold links from 306 to 46, the cold
compile from about 207 s to about 143 s (31 % less) and the test run from
763 s to 537 s (30 % less). Separately, compiling the largest library a
second time in test mode was a single compiler process needing about
6.4 GiB, which could not be built at all under a 5 GiB limit.

Two cautions from the same work. Tests that change process-wide state
(environment variables, the working directory, global registries) cannot
share a binary; there, 28 files had to stay separate. Here no test does
that: each is handed what it needs, and programs under test are run as
child processes with an environment of their own. And an earlier attempt at one binary failed 36 tests that passed alone:
folding is only safe for tests that do not interfere.

### Tests do not sleep

Enforced by the `sleeps` gate. Time is `kernel::Clock`; tests use
`ManualClock` and wait for events.

*Old:* of about 771 s of test run time, roughly 650 s was waiting and 120 s
was work (an estimate from a sample; the waits were mostly deadline-and-poll
loops, and assertions that something does *not* happen waited out their whole
deadline). Here, on 2026-11-15, eight of the twelve suites take under a
second and a half; the two that run whole daemons take 21 s (`api`) and
5 s (`cli`), and in neither is any test waiting on the clock: the slowest
of the API suite's takes 2.9 s alone.

### The database library keeps no count of the memory it has in use

Said in `.cargo/config.toml` (`LIBSQLITE3_FLAGS`, which the library's own
build reads from the environment), and held by a test of the journal's
suite, which asks the library how it was built and fails where it was
built otherwise.

The library counts the memory it has in use, and takes a lock to do it
at every allocation it makes: one lock for the whole program. Each
database here has a thread of its own, so every thread that was doing
anything stood in line for it, and a thread that has to wait is put to
sleep by the system and woken again. A factory has a database for each
project's journal; a test program has a node, or a conductor, for each
test, thirty-two at a time.

*Here,* 2026-11-19, the runner's suite on thirty-two threads, nothing
waiting for the disk (below): 123 s with the count, 425 s of processor
time in the program and 2,265 s in the system, and 63 million times a
thread gave way to wait for another; 32 s without, 287 s and 96 s, and
3 million. Held to eight processors the same suite had taken 63 s, and
to four 82 s, with the count: the work was always about 240
processor-seconds, and thirty-two threads at once made ten times that
of it. The agents' suite went from 7.4 s to 1.1 s by this alone. It
was measured once before, on 2026-11-15 (*Tried here on the tests*,
below), when the suites were a tenth of what they are, came to three
seconds of twenty-two, and was left: what it costs grows with how many
databases a program has at work at once, and faster than they do.

Nothing here reads the count, or sets a limit that goes by it. What
ships is built the same way on every system, and gains the same where
a factory has many projects at work; what is given up, and what holds
a build made some other way, is said where the journal is described
([architecture.md](architecture.md), *The journal*). Checked for
Windows and for macOS from this system the same day (*Checked for
other systems, from this one*, below): both checks end with no error
and no warning with the flag said, and the library as it was compiled
for each has `DEFAULT_MEMSTATUS=0` in its own list of how it was
built, which is what the journal's test asks it. The C compiler in
those checks is the one at hand and not either system's own;
Microsoft's takes a definition written the same way, and has not been
run on this. The test is compiled for every system and asks nothing a
system has to have.

### A factory made for a test does not wait for the disk

`journal::disk`. A database in a file waits for the disk as its file is
made, each time its log is brought back into it, as it is closed, and
for a commit that is to be on the disk before it is answered. A factory
that is run always does. A test cannot hold any of it (none takes the
power away, and what a program that is killed had written is the
system's to keep either way), so a factory made for a test says it does
not wait: `Db::open_unwaited` where a suite opens the file itself (the
runner's), `Config::disk` where a node is in the test's process (the
API's suite, and the nodes of the command line's), and one word in the
environment, read in one place, for the daemons the command line's
suite starts as programs of their own (`JOURNAL_DISK=unwaited`). A
project's journal is opened as the factory's own was. A daemon that is
run is never told.

*Here,* 2026-11-19: one small test of the runner's
(`plays::steps_in_order…`, 73 conductors stopped and started on files)
took 15.1 s and 12.1 s waiting for the disk and 1.73 s and 1.78 s not
(A, B, B, A), for the same 2.2 s of processor time. Of the samples of
which call into the system each of its threads was waiting in, the
test's own thread was waiting for the disk in a third and its
database's thread in two fifths. The disk was 99% full then; emptied to
80% and with other gates running, it was 11.6 s and 12.9 s against
2.9 s. So it is the waits and not how full the disk was. (The root
file system here is ext4 mounted with `discard`, on a virtual disk:
what a wait for the disk costs on this machine cannot be changed
here, and is dearer than on most.) With the count of memory still
kept, not waiting took the runner's suite from 129 s to 123 s and no
further, which is how the count was found; with the count gone, from
81 s to 32 s.

What still waits, because it says nothing: every test of the journal's
own suite that opens a file (the giver's mark held to the disk before a
run is used, `drawn.rs` and `given.rs`; a log brought back,
`brought.rs`; the rest), the two files the suite of `runs` opens, one
of the studio's, and the runner's count of what a write costs
(`costs.rs`). And whatever is held to the disk by a way of its own
waits whoever asks, in a test's factory too: a database written out
again (`journal::rewrite`), a project's file moved apart
(`journal::moved`, which opens the file as any is opened and holds it
to the disk itself), a copy of one moment (`journal::moment`), the
installation's secret (`link::own`), a store that is served. In the
command line's suite the daemons started some other way than its
`Daemon` (`profiles.rs`, `guarded.rs`, `keyed.rs`, `outliving.rs`,
`places.rs`) are not told, and neither is any command run on a factory
that is stopped (`apart`, `repack`).

It was tried on the API's suite on 2026-11-15 and showed nothing but on
one thread: what was waited for then was the count above, and a suite
that waits in line for a lock does not notice the disk.

### The plays of one test are made several at a time

`crates/conductor/tests/suite/anywhere/abreast.rs` for the runner's
plays, each on a thread and a conductor of its own, twice as many at a
time as the program runs tests (as many, until 2026-11-20: a play is
two threads that take turns); `at_once` in the API suite's `simulation.rs`
for the simulation's: sixteen at a time where the program runs from
five to sixteen tests at a time, eight where it runs more, and four
where it runs four or fewer.

A test program is never over before its longest test is, and the
longest tests are the ones that play one thing stopped at every place:
281 and 331 plays in the runner's two longest, made one after another
on one thread while the processors beside it stood idle. Which plays
are made is what it was, and a test holds the two ways of making them
to the same plays (*Stopped anywhere, and started again* in
[architecture.md](architecture.md)).

*Here,* 2026-11-19, the runner's suite on thirty-two threads: 29 s with
the plays of a test one after another, 18 s several at a time, for 273 s
of processor time where it was 256 s (a play made ahead that is then
not the one asked for is made again). Held to four processors: 62 s
and 59 s, 151 s and 150 s of processor time. It had been written the
day before and showed no gain: every play it added at a time stood in
the line for the count of memory. The API's suite: 107 s with four
plays of the simulation at a time, 84 s with eight, 85 s with twelve
and 84 s with sixteen, each on a machine other gates were running on;
what was left was the other tests, which the simulation's began
behind. Held to eight processors or fewer it was left at four at a
time, as it was, until 2026-11-20 (the last paragraph here).

And the simulation's tests are begun first. A test program begins its
tests in the order of their names, and the scenarios' modules
(`simulated`, `segmented`, `rounded`) come late in it: every other test
was over when they had most of their plays still to make, a few at a
time. They are under a name that comes before the rest (`abreast`, in
the API suite's `main.rs`; the files are where they were, and a test
is found by its module's name as before), so the plays are made beside
the other tests: 89 s to 72 s for the suite, with eight plays at a
time. Sixteen at a time is then slower (81 s and 85 s): each scenario's
test holds one of the program's thirty-two threads while it waits for
its plays, and the other tests have what is left.

On eight processors it is the other way about, and that is where a run
of every test is measured. A program of eight threads begins with
eight scenarios' tests, and the next eleven take each thread as it
comes free: until the last scenario has begun no other test has a
thread, and each scenario's test holds its thread until its last play
is made. With four plays at a time that was 43 s of a suite of 95 s,
on three of the eight processors (110 s of processor time), and the
other 588 tests then took the 52 to 54 s they take by themselves. *Here,*
2026-11-20, the API's suite alone in a lane of eight processors, each
figure taken in alternation with the ones beside it on a machine other
work was running on: 91 to 97 s with four plays at a time, 83 to 85 s
with eight, 78 s with twelve, 65 to 74 s with sixteen, and no less
with twenty-four (71 and 75 s) or thirty-two (67 and 70 s). So it is
sixteen where the program runs from five to sixteen tests at a time.
The most ways of being told that files changed the program held at
once was seventeen, where with eight plays at a time it was fifteen,
of the 128 a user is given; the most threads, 151 and 110. Four or
fewer threads is what the hosted machines have, and is left at four:
nothing was measured there.

### The tests that drive the whole product are run once

Held by how the gate runs them (`xtask/src/suites.rs`): it builds every
test program in one build, as `cargo test` does, and then runs them
itself. The `test` gate runs every crate's but `api` and `cli`; the
`offline` gate runs those two, where there is no network, as whoever ran
the gate, each from its own crate's folder. Where no such place can be
made (the hosted machines, and systems other than Linux) it runs them all
the same with the network there, and says that it could not count.

*Here:* with cargo running every test first, those two suites ran twice,
and the second run was 25 s of a 61 s gate. Run once, the gate was about
45 s. Nothing is tested less: the same tests run, counted both ways.

The programs are built in the one build because asking cargo for some
crates' tests and not others changes which features their dependencies
are built with: leaving the two out compiled a second copy of every crate
here. That is the *dependencies are declared once* rule again, met from
the other side.

*Old:* the same shape. Three quarters of a gate from nothing there was
running tests (771 s of 1,026 s), and most of what a change waited for
was gates it could not have affected. That codebase came to run only the
gates a change obliges, and to work out which from what each suite reads,
after a list written by hand had left out the one suite that would have
failed. `cargo xtask check --changed` is that here, and for the same
reason takes a change outside every crate to affect everything: tests in
half the crates hold the notes, the packages and the examples to the code.

### Test programs run side by side, and a large one as several processes

Held by the same code, and by `xtask/src/sharing.rs` for the arithmetic.
`cargo test` runs the test programs one after another, each on as many
threads as there are processors. The gate runs them side by side on eight
threads each, five at a time on thirty-two processors, and a program with
more than 64 tests is run as up to four processes that each take every
so-manyth test by name. On fewer than sixteen processors none of this
happens and the tests run as cargo runs them.

*Here,* on thirty-two processors: the API suite takes 21 s as one process
and 9 s as four, for two fifths less of the processors' time; with the
command line's suite beside it, both take 11 s where they took 28 one
after the other. The other ten suites take 2.6 s where they took 9.6. The
gate with nothing to rebuild went from 45 s to 16 s, three runs within
half a second of each other.

Why it works is the finding under *Tried here on the tests*: a program
whose tests each start a daemon gets no faster past a dozen threads,
because the tests wait on one another inside the one process. Eight of
four threads and four of eight finish within a second of each other, so
the width is eight and no finer.

Two things are held. Every test is in exactly one share (a test of the
arithmetic), and the gate counts: a program that says it has 251 tests
and ran 249 fails, since a share that named a test the program did not
know would run nothing and call it passed. And a test may not lean on
another being in its process, or on what ran before it; here none does.

*Old:* this is what `cargo nextest` gave there (39.5 % off a 397 s run),
by another route and with nothing to install.

### What the tests leave running is looked for

Held by the two gates that run the test programs (`xtask/src/left.rs` for
the working out, `leftover.rs` for where it is asked). A daemon leaves
its services running when it stops, on purpose, and a stand-in told to
wait goes on waiting: a test that does not end what it started leaves a
program that holds a port, a lock or a processor until somebody comes
across it. The rule was written down and nothing looked. After a gate
here there was a stand-in a rollout's play had started and no daemon
knew of; after another, a `sleep` that a shell of the command line's
suite had started a moment after its group was asked to stop; and after
a check of the client that threw, its daemon.

So `test` and `offline` each read what is running before the test
programs and after them, and fail for what is new, is this checkout's
and has nobody waiting for it, naming each thing and the crate whose
tests left it (a test program is told whose tests it is, `TESTS_OF`,
which whatever it starts is told with the rest). What another checkout's
gate is running is not this one's; what was there before is not new; and
what somebody who was there before still waits for (a build in another
terminal, an editor's) is theirs. Whether a process has lost its parent
is said by the system only as who has it now, so the look finds out who
takes in what this program leaves behind, by leaving something behind:
its own program, started to start itself again and end. The gate ends
nothing. `cargo xtask left` lists what is running here that nobody waits
for, whenever it is asked.

*Here:* a look reads `/proc`: some twenty milliseconds for the two looks
of a gate with a hundred and fifty processes running (18 ms of `test`'s
50 s and 15 ms of `offline`'s 102 s in one run), which is said on the
gate's line each time. Something found is given three seconds to be gone
before it is said to have been left, looked at every 25 ms: a guard that
ended what its test started has asked by then, and nothing says of a
process that is not one's own that it has ended but its no longer being
there. That wait is made only where something was found, so a gate that
passes never makes it.

The reading is Linux's. On a system with no `/proc` the gates say that
what was left running was not looked for, and the tests are still tests.

### The suites played another way are a command, and no gate

Held by `xtask/src/waying.rs` over `xtask::ways`. A test program is told
two things of the journal by a word in its environment: whether a
project that is added has a journal of its own (`PROJECT_JOURNALS`), and
how work that waits for a database is gathered into commits
(`JOURNAL_GATHERS`). The gates play every suite told nothing, which is
what a daemon does. The suites are to hold three other ways as well:
`one`, every project in the factory's own journal, which is a factory an
earlier program made; `own`, each play of the runner's suite a project's
with a journal of its own; and `gathered`, whatever waits kept by one
commit. Each is a run of every test again, so none is a gate: a run of
every test is one of the figures held to a budget, and three more of it
on every change is what the rules above took out. A fourth,
`ungathered`, is every piece of work kept by a commit of its own, and is
a run of the journal's suite alone: no other suite has a test that goes
by that word, and to the rest it is what a daemon does but where
sixteen pieces of work wait at once.

```sh
cargo xtask ways              # all four, one after another
cargo xtask ways one          # or those that are named
```

The test programs are the gate's, from the one build, and each way is
the tests' runner started again as a command of its own, as `offline`
starts it, told that way's word and no other: a word in the environment
of whoever asks is not played beside it. It is asked for before a change
to where anything is kept or to how the journal commits, and the hosted
run plays all four after its gate.

*Here,* on 2026-11-19, 2,060 tests each way with nothing to rebuild: on
eight processors `one` took 160 s, `own` 213 s and `gathered` 179 s; held
to four, as the hosted machines are, `one` took 354 s. What a gate last
kept of how long each test took is put back after each way, so `cargo
xtask timings` goes on saying what the gate's own run took.

Why it is there: nothing played the suites told `one` for some weeks,
and 32 tests came to fail there, each a test of a project's own journal
that did not ask first whether there was to be one. None was a fault of
what a factory from before does, and nobody could have said so. One
test of the journal's failed the same way told to keep each piece of
work by a commit of its own: it arranges a commit that several share.

### Small crates, layered one way

Enforced by the `layers` and `size` gates (1000 lines a file, 15,000 a
crate).

This is the structural fix for the two things above that no setting can fix:
a compiler process too big for the machine, and an edit that rebuilds
everything. Here, adding a function to the lowest shared crate recompiles
nine crates in under two seconds. *Old:* peak memory in a build was set by
the single biggest compiler process (6.2 to 6.5 GiB) and barely moved with
the number of jobs.

### Dependencies are declared once, with their features

Enforced by the `manifests` gate: only the root `Cargo.toml` names versions
and features.

When two crates ask for different features of one dependency, building one
crate and then the workspace compiles that dependency twice, and which copy
you get depends on what you built last. One declaration means one build.
(This is the same failure as the `codegen-units` result above, by another
route.)

A dependency is weighed the same way before it is added. The one added for
file notifications (`notify`) brings six small crates on Linux, costs 1.5 s
of processor in a clean build, and changes the features of nothing that was
already being built.

### Build scripts watch only what exists, and never the clock

One build script exists (`brand`), and it watches `brand.toml`.

*Old:* a build script that watched `.git/HEAD` was permanently out of date
in a git worktree, where `.git` is a file and that path does not exist. A
build with nothing to do took about 54 s and re-ran the compiler. (The fix
was not verified there; the rule is the cheap way never to find out.)

### An optimised profile for machines that rebuild all day

`[profile.fast]`: `release` without link-time optimisation, 16 codegen units,
incremental. `cargo xtask install` uses it.

*Old:* a one-file rebuild of the daemon took 35 s instead of 284 s, for a
binary 39 % larger and a target directory three times the size. Releases
stayed on `release`. *Here:* 49 s against 63 s from clean, 22 MB against
13 MB.

### Incremental compilation is left on for people

The dev profile does not set `incremental`, so it is on.

*Old:* it was turned off for the machines that built a different branch for
every ticket, where the cache was paid for and almost never hit (and had
grown to 33 GB), and turned back on for people editing one tree, where
touching one module went from 73.3 s to 16.1 s. On a clean build it costs:
6.6 % in total there. So: on for working, and `CARGO_INCREMENTAL=0` for a
one-off clean build if the last few percent matter.

### Configuration is not compiled

Blueprints and the client are read from disk while the daemon runs. The most
common change, to how work is done or how it looks, rebuilds no Rust at all
and does not restart the daemon: a blueprint is applied again, and a rebuilt
client (0.2 s) is announced to the pages that are open.

`cargo xtask changes` says how far the history bears that out. The claim is
that what changes often is configuration and not compiled code, so the share
of commits that need the daemon built again should fall as the product
settles; this is the number to watch, and before it nothing reported one.
Each commit of the branch it is run on is put in one group by the files it
touched, the first that fits: `rebuild` (anything under `crates/`, `desktop/`
or `xtask/`, or `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, or
`brand.toml`, which the `brand` build script compiles in), `client` (`ui/`),
`text` (`blueprints/`, `extensions/`, `examples/`) and `notes` (everything
else). It prints a row for each week that had a commit, named by the Monday
it began, with the commits in each group and the share that were `rebuild`,
and a last row for everything. A merge is not counted, since its commits
are. `--since` begins the count later, at a commit (a tag, a branch, a hash)
or a date, and it is git that reads it, not the tool:
`cargo xtask changes --since "2 months ago"`.

## Tried here on the tests, and not adopted

Measured on 2026-11-15, on the same machine while it was quiet, against
the API suite (251 tests; 127 s one at a time, 40 s on four threads, 25 s
on eight, and 21 to 22 s on anything from twelve to thirty-two).

That it gets no faster past twelve threads is the finding. No test is
slow, none waits, and a quarter of the processors are idle. The time is
in the suite's own process and not in the programs it starts, and most of
the kernel's share is in the threads that do each node's database work:
one card going from a worker to a reviewer and being merged is about
2,600 pieces of database work, each handed to that node's one database
thread and handed back. Run as two processes at once, twice the tests
took 26.5 s where one process takes 21; so part of the limit is inside
one process. That became the rule above: the suite is run as four.

- **Every dependency optimised** (`[profile.dev.package."*"] opt-level = 2`).
  No change: 127 s one at a time, 42 s on four threads. The time is in
  this repository's own code, not in what it is built on.
- **The dev profile at `opt-level = 1`.** A fifth off on four threads (41 s
  to 33 s), a third off one at a time, for a build of every test program
  from nothing that took 112 s where it takes 69. What the tests wait for
  is mostly not their own instructions, so it bought little.
- **SQLite without its count of memory in use**
  (`SQLITE_DEFAULT_MEMSTATUS=0`, a lock every allocation takes). 22 s to
  19 s on thirty-two threads, nothing on eight or four. Not worth building
  what ships differently, as it stood that day; adopted on 2026-11-19,
  when it had become three quarters of the runner's suite (*The
  database library keeps no count of the memory it has in use*, above).
- **Another allocator**, loaded under the suite. 22 s to 18 s on
  thirty-two threads; the same reasoning.
- **Scratch folders in memory** in place of under `target/`. A fifth off on
  four and eight threads (40 s to 32 s, 26 s to 21 s), nothing on
  thirty-two. Not adopted: a test program that is killed would leave its
  folders in memory until the machine restarts. It is the lever for a
  hosted machine with a slow disk, if one becomes the wait.
- **Not asking the disk to confirm what was written.** Nothing on any
  number of threads but one, that day; adopted for the tests' own
  factories on 2026-11-19 (*A factory made for a test does not wait for
  the disk*, above), once what hid it was gone.
- **More threads than processors, on four processors.** The hosted
  machines have four. There the API suite takes 40 s as one process of
  four threads, 33 s as one of eight, and 30 s as two of four or three of
  two: a quarter off. Not adopted yet: every test then takes longer beside
  the others, on machines that are slower to begin with, and what a test
  waits for gives up after thirty seconds. It is the first thing to try
  for the hosted gate, watched over several runs.
- **A second build folder for the lints**, so that they and the test build
  run at once after a change (2 to 4 s and 3 to 6 s, one after the other
  today). Not tried: it is every dependency checked a second time and
  kept twice.
- **A limit on one test's time.** A test's time beside the others is five
  times its time alone and moves with what else the machine is doing, so
  a limit either fails for no reason or catches nothing. The whole run is
  held to a budget in its place.

One thing found and left: a node a test started is not let go of when the
test ends, so by the end of the API suite its process holds about 600 idle
database threads and 4.2 GB. It costs no time (a module run alone takes
what it takes at the end of a whole run), and it would on a machine with
less memory.

## Where the tests' time was on 2026-11-19, and how it was found

A run of every test had come to 296 s of the 300 it may take, with
nothing more able to be added: 138 s the API's suite, 129 s the
runner's. Each was said to be waiting on hand-overs between a test's
thread and its database's thread. That was half right: they were
waiting on each other, and not for the reason given.

There is no profiler on this machine, and nothing may be installed.
What told, in the order it was done:

- **What the whole took of the processors.** `cargo xtask build-times`
  under `time`: 1,275 s in the programs and 2,995 s in the system, for
  work that is mostly a small database being written. Time in the
  system that large is waiting done badly, not work.
- **One test alone.** A small play of the runner's took 11 to 17 s for
  2.3 s of processor time: it was asleep for four fifths of it.
- **Which call into the system each thread is asleep in.**
  `/proc/<pid>/task/<tid>/syscall` says it for a thread that is not
  running, and may be read only by what started the program, so a
  shell script starts the test and reads the files itself, fifty times
  a second. Call 74 is the wait for the disk: a third of the test's
  own thread's samples and two fifths of its database's. (A thread
  that is running says only `running`: this finds what is waited for,
  and not what the system is busy with.)
- **A trial that changes one thing**, with a word in the environment
  read where the thing is done, so that the same program is timed both
  ways in alternation. Not waiting for the disk: that test 1.7 s. And
  the whole suite hardly moved (129 s to 123 s), with 2,265 s in the
  system.
- **Which threads the system's time is charged to**
  (`/proc/<pid>/task/*/stat`, summed by the thread's name): nine tenths
  to the threads called `journal-db`, each a third of its life on a
  processor and nine parts in ten of that in the system.
- **The same suite held to fewer processors** (`taskset`): 63 s on
  eight and 82 s on four, where thirty-two took 123 s. Less to run on
  and sooner done: whatever it is grows faster than the threads do.
- **How often threads gave way** (`/usr/bin/time -f %w`, the voluntary
  context switches) and how many interrupts one processor sent
  another (`CAL` in `/proc/interrupts`): 63 million and 33 million in
  the one run, where the same plays one after another come to three or
  four million. So threads were being put to sleep and woken for
  something the plays themselves do not ask: a lock.
- **Which lock**, by taking away one candidate at a time (below): the
  database library's count of the memory it has in use.

What was tried on the way and changed nothing, each on the runner's
suite with nothing waiting for the disk (123 s, 2,265 s in the system):

- **The tests' scratch folder in memory.** 119 s, 2,218 s. Not the
  file system.
- **The allocator told to give no memory back** (`GLIBC_TUNABLES`, the
  thresholds for trimming and for mapping raised to their most). 123 s,
  2,213 s. Not memory being mapped and unmapped.
- **Each test, and every thread it starts, kept to one processor**, the
  least loaded when it began. 109 s, 2,090 s. Not the hand-over between
  a test's thread and its database's crossing processors, though that
  costs something alone: a token handed between two threads takes 37
  millionths of a second a round here free, and 6 kept to one
  processor.
- **Fewer tests at a time on every processor** (`--test-threads`): 100 s
  with eight and 250 s in the system, 143 s with sixteen and 892 s.
  Fewer at a time is less waiting and a longer run all the same.
- **The database's files opened without the system's locks on them**
  (the library's own way of holding a file for one program). 151 s on a
  busy machine, 2,229 s. Not the locks a file is held by.

A scratch program that does one kind of thing on one thread and on
thirty-two (hands a token between two threads, starts a thread, makes
and removes a file, writes a file) says what this machine makes dear
when many do it at once: a thread started and joined is fifteen times
dearer a piece on thirty-two threads; a file made and removed in one
folder goes from 35 millionths of a second to 1.2 thousandths, with
359 s in the system for 640,000 of them. The machine is a guest with
thirty-two processors of a host that has as many, and whatever makes
threads wait for each other costs it more than it would cost a machine
of its own. None of that was what the suites were doing; it is here
for whoever next finds the system's time large.

**A crate built optimised where the tests are built**, each alone,
timed on three tests of the API's suite and a play of the runner's,
each alone (processor seconds, the middle of three; as it is: 1.33,
2.17, 4.5 and 9.1). `conductor`: 1.30, 2.46, 6.4, 9.7, and 75 s to
build where a rebuild from the bottom takes five. `runs`: 1.35, 2.19,
4.5, 9.2. `journal`: 1.31, 2.27, 4.5, 8.9. `kernel`: 1.45, 2.28, 4.7,
8.7. `serde_json`: 1.31, 2.12, 4.2, 8.6. `studio`: 1.29, 2.09, 4.3,
9.0. `agents`: 1.31, 2.12, 4.3, 9.0. Two runs of the same thing differ
by as much as any of these does from none. None is in the place the
language was in, where one crate was half of a suite; none was kept,
and each would have cost three quarters of a minute of a clean build.

**Everything optimised a little**, on the same tests, with the plays of
a test made several at a time by then (as it is, twice: 1.4, 2.4, 4.9
and 15.7 for the play, then 1.37, 2.2, 4.4 and 12.9; the play's figure
moves with what else the machine is doing). The runtime everything
runs on (`tokio`) at `opt-level = 3`: 1.32, 2.2, 4.45 and 11.7. Every
dependency at `opt-level = 2`: 1.5, 2.1, 4.2 and 11.8, for 104 s to
build the test programs from nothing. The whole dev profile at
`opt-level = 1`: 0.98, 2.0, 4.0 and 7.8, for 125 s.
So what is left of the tests' time is this repository's own code and
the runtime under it, unoptimised and spread over every crate: about a
quarter of the API suite's processor time and half the runner's. It
was not adopted: it buys time in the tests with time in every build,
and the tests are no longer short of room. What it does to a rebuild
after a change (`rebuild_bottom_s`) was not measured; it is weighed
against that, and not against a clean build, if the tests come to
need the room again.

## Where the API's and the command line's tests' time was on 2026-11-20

A run of every test was measured at 216 s that day, with the ceiling at
150: 105 s the API's suite and 23 s the command line's, where the day
before they were 71 s and 10 s. It was taken for something the day's
changes had added to every test's rig. It was not that. A rig is 29 ms
to start, 14 ms to register its owner and 3 ms to stop, on one thread.
And most of the difference was the machine, which other work was
running on: the commit measured at 71 s the day before took 104 s that
day (548 tests), alone in a lane of eight processors, in alternation
with the one measured at 105 s, which took 91 to 95 s there (607
tests). A measurement says what the machine is doing as much as what
the tests take: two commits are compared in alternation, on the one
day, or not at all.

What the time was, in the order it was found, each figure a suite
alone in a lane of eight processors, in alternation with the one it is
held against:

| What | The API's suite | The command line's |
|---|---|---|
| As it was | 91 to 97 s | 20.0 and 20.4 s |
| Sixteen plays of the simulation at a time, where it was four on eight processors (*The plays of one test are made several at a time*, above) | 65 to 74 s | |
| The tests that look in every file a factory keeps no longer read its program through the link in its data; a package's case asks where the journal is only where it waits; a test that waited for a line in a file is told when it is written | 70 and 73 s | 15.2 to 15.5 s |
| What a blueprint's text says with `emit` kept by the text (`said_by` in `texts::known`, then the runner's `known.rs`) | 52 and 54 s | 12.9 to 14.8 s |

A run of every test (`cargo xtask build-times`), three times each way
in alternation, with a good deal of other work on the machine (a load
of thirty and more): 190, 190 and 202 s as it was, of which the API's
suite 99, 102 and 103 s and the command line's 18.3, 19.8 and 21.5 s;
139, 152 and 157 s with all of the above, the API's suite 56, 60 and
61 s and the command line's 15.1, 17.5 and 17.6 s. The runner's suite
was 55 to 67 s in all six and is what is left: nothing here was for
it. No test was taken out, made shorter or made to hold less: 607 and
121 by name, before and after.

How each was found, there being no profiler here:

- **Each line a test program prints, with when it came.** The last of
  the scenarios' tests ended at 43 s of 95, and 85 of the other 588
  tests had ended by then: the scenarios are begun first and hold
  every thread. Under `/usr/bin/time` the program had used five and a
  half of its eight processors.
- **Each of the slower tests by itself, under `/usr/bin/time`.** Three
  tests of the command line's and two of the API's took two to four
  seconds of a processor for a few commands: each looks in every file
  under its factory's folder for a value, and the folder holds a link
  to the program, 140 megabytes there and 180 in the API's suite, where
  the program is the test. One of them looked again and again while it
  waited. And the same list says which tests only wait (below).
- **The processors' time by what each process is** (`/proc`, read as
  the suite runs, for everything it started): of the command line's
  140 s, 55 s was its daemons', 45 s the test program's own and the
  rest commands too short to be seen, some five thousand of them at
  six thousandths of a second of the system's time each, which is what
  starting a program of 144 megabytes takes here (1,250 pages).
- **A sampler loaded beside the test program** (a few lines of C: a
  timer on the processor time the process uses, and at each tick the
  thread that is running writes down where it is and the frames above
  it by their frame pointers; the addresses are named afterwards).
  The frames are lost wherever code was built optimised, so it says
  where the processor is more than how it came there. In the tests
  that bring a fleet up it was in the language's reader and in the
  allocator, under one initialisation: what the blueprints of a
  library say to each other, read afresh for each text checked. A
  check of one text that hears something read and checked all fifty
  that ship, 19 ms, and one blueprint applied is a score of such
  checks: a quarter to two fifths of what such a test took.

Found and left, each with what it would take:

- **Tests that wait two seconds for the product to be sure.** A node
  that finds its folder held, or its remembered address taken, waits
  two seconds before it says so (`LOCK_PATIENCE`), on the system's
  clock and not the node's, which is not made yet; a command that
  works on a stopped factory's files waits the same way; and an `up`
  beside another says what it waits for after two (`QUIET`). The tests
  of each wait with it: 12.6 s of the API suite's threads in five
  tests, and 12.8 s of the command line's in four. A test's thread
  that waits is a thread no other test has. The node would have to
  wait on its own clock, which a test can then move; the commands
  would have to be told how long, as a daemon of the tests' is told
  not to wait for the disk.
- **A statement is prepared each time it is asked.** Seven to eight
  parts in a hundred of the samples were the database reading SQL
  (`sqlite3Prepare`), under the library's `prepare`.
- **Looking a blueprint up by name asks the system about three files**
  for each name and each place it may be: four parts in a hundred of
  the samples, since what the blueprints say to each other still asks
  the library for every text there is, each time.
- **The dev profile at `opt-level = 1`**, as weighed above. Not tried
  again: the tests have room.

The line of the history written after all of it
(`docs/build-history.csv`, 2026-11-20) is a run of every test in
107.6 s, against the 150 s a run may take: 50.6 s the API's suite,
30.2 s the runner's and 13.5 s the command line's.

## Where the API's suite's time was on 2026-11-26

A run of every test (`cargo xtask build-times`, in a lane of eight
processors) took 201.6 s, 130.8 s of it the API's suite, with the
machine at a load of 1.4 before it and 10 to 23 while it ran, most of
that its own. The suite alone in the lane took 119.0 and 119.8 s, for
766 and 768 s of processor time: 640 parts in 800 of what the lane
had, and 85 to 90 in a hundred through most of the run, sampled each
second from `/proc/stat`. So the time was work, and what was owed for
it was less of it.

What it was not, each tried first:

- **The longest scenario of the simulation.** The simulation's 44
  tests took 62 s by themselves, the longest of them 34 s, so the
  suite was not waiting on its longest test. Sixteen tests at a time
  in place of eight (61.4 s), 24 and 32 plays at a time in place of 16
  (62.9 and 62.3 s), and a rest of 100 ms in place of 400 before and
  after each play (63.5 s) each changed nothing: the processors were
  busy. Sharing a scenario's places between tests would have bought
  as little, and was not done.
- **Waiting on the clock.** Every test outside the simulation run
  alone, six at a time, came to 480 s, 427 s of it on a processor: at
  most 72 s of waiting in all, nine seconds of the suite on eight
  threads, most of it in a dozen tests that wait for a factory to be
  at rest by its silence (`costed`, `timed`) or for a held folder
  (`folders`, `LOCK_PATIENCE`). Left as it is.

Where the processors' time was, by a sampler loaded beside the test
program (below): checking blueprint texts, 14 to 19 parts in a hundred
of the program's own, the same texts again and again; the database
library, a quarter, a third of that preparing statements and a quarter
making a new database's tables; reading a text into its tree, a
twentieth by itself, in `reader`, built unoptimised since it came out
of `blueprint`; hashing the password of the owner each test's factory
registers, a twentieth; and JSON, a tenth in the simulation, most of
it a step's scope copied as each step begins and ends. A play of the
simulation was 0.8 s of processor time, 0.3 s of it setting its
factory up.

What was done, each the API's suite alone in a lane of eight
processors, in alternation with what it was held against, on one
evening with the machine otherwise at a load of 1 to 14:

| What | Before | After | Processor time |
|---|---|---|---|
| `reader` built at `opt-level = 3`, as it was inside `blueprint` | 120.3, 122.6 s | 111.3, 109.8 s | 772, 790 s to 698, 698 s |
| A check given again where its catalog answers what it asked as before (`texts::remembered`) | 109.7, 111.1 s | 107.0, 108.6 s | 688, 698 s to 664, 671 s |
| A test's factory hashes passwords with the least the hash takes (`people::Hashing`) | 107.0, 108.6 s | 103.9, 103.8 s | 664, 671 s to 646, 642 s |

No test was taken out, made shorter or made to hold less, and no
scenario's places changed: as `cargo xtask build-times` counted them,
the simulation's tests made 594 plays before and 594 after, each test
as many as before; the places its plays with nothing in their way
passed came to 8,619 and 8,636, three scenarios passing one fewer or
nineteen more as the machine ordered what they said.

The line of the history written after it (`docs/build-history.csv`,
2026-11-26) is a run of every test in 161.7 s with the machine at a
load of 10 to 15, against 201.6 s before: 101.8 s the API's suite,
29.1 s the runner's and 16.7 s the command line's. The suite's five
slowest tests, beside the others, were the A/B scenarios and two of
going ahead: 25.4, 22.8, 22.8, 21.7 and 20.9 s, where they had been
33.8, 31.0, 30.8, 29.7 and 28.4 s.

Tried and not kept: a database new to a list of steps given what the
same steps left the first time, all at once, so that a step adding a
column did not have the database read every table's description again.
110.2 and 113.1 s before, 110.3 and 108.3 s with it (689, 692 and 685,
676 s of processor time): within what two runs of the same differ by,
for a module in the journal. And remembering every check by its text
alone, whatever its catalog said, which is wrong: 101.6 and 101.3 s
where the same program took 112.8 and 111.9 s, so what is left of
checking, some six parts in a hundred, is the library answering
whether a blueprint of a name is there, from the disk each time.

Left, each with what it is: statements prepared again each time they
are asked (`prepare_cached` where a statement is asked often, as
`runs::runs::put` is at every step: some eight in a hundred); a new
database's tables, a step that adds a column reading every table again
(six in a hundred, and not mended by the above); the library asked for
a blueprint by reading its file (four); a step's scope copied as it
begins and ends; and a play's factory set up from nothing for each
place, two fifths of what the simulation asks of the processors.

The sampler, there being no profiler here: a few lines of C loaded
beside the test program (`LD_PRELOAD`) that set a timer on the
process's processor time (`setitimer(ITIMER_PROF)`), and at each tick
write the running thread's stack as `backtrace()` gives it, which
unwinds by the program's own tables and so needs no frame pointers;
named afterwards by `addr2line` from each address less where the
program was loaded (the first mapping of its file in `/proc/<pid>/maps`,
not the mapping the address is in). It runs in the test's process, so
what it says is that process's alone: the programs a test starts (git,
some 21,000 times in the simulation, for 40 s of processor time) are
counted by a second library loaded the same way, which writes each
program's own time as it exits.

## What one card costs, 2026-11-20

Not a build or a test, but the same kind of question, asked of what the
daemon does: what one card worked by a stand-in and reviewed by another,
then merged, costs the disk, the journal's thread and the processor, and
where that goes. A comparison of 2026-11-15, by tooling kept outside
this repository, had twenty such cards at 225 MB written; on `main` as it
was on 2026-11-20 they were 114.75 MB in 1.76 s, with 4.65 s of the
daemon's processor time.

**How it was measured.** A factory set up as that comparison's story sets
one up (a worker and a reviewer, the optimised program), two cards through
it to warm it, and then five, one at a time, each in a factory at rest
before and after (nothing written for twelve seconds, which takes in the
flush of a minute's measurements). For each card, from the daemon's own
`/proc` (its pid is the one the story started): each thread's `wchar`
(bytes handed to the system to write: for a database's thread, its log's
frames exactly) and `write_bytes`, and its processor time; each file
under the data folder, by size; the log's frames read from the `-wal`
file itself (its header's salts, and each frame's page and whether it
ends a commit), and which table or index each page is of, read with
SQLite's `dbstat` from a copy taken at rest; the daemon's own count of
its database work (`metrics db.work --by db,caller,kind`, read before
and after); and every event said, by topic and by the blueprint whose
instance said it. Which call site asked for each commit was read with a
change to the database's thread that was never committed: each commit
written to a file with the `Location` of each piece of work that wrote,
and how many frames the log grew by. The probe (a script for a card, one for the
tables) and that change were kept apart, and are not in this repository. Counts and bytes are what
is trusted; times are a second figure, on a machine shared with other
builds, said with their spread, base and changed measured in
alternation (two runs of five cards of each, then a third of the
changed).

**Where it went on `main`**, a card, largest first (the mean of ten
cards; what each card ranged over in brackets):

| | |
|---|---:|
| written to the disk, all of it | 5.34 MB (4.98 to 5.67) |
| the project's log (`journals/shop.db-wal`) | 4.30 MB: 2,123 to 2,134 frames, 302 to 303 commits, 7.0 frames a commit, 257 pages |
| the log brought back into the project's file | 0.71 MB |
| the search index's log | 0.14 MB |
| the factory's own log (measurements, `kv`) | 0.12 MB |
| the logs of steps | 2.5 KB |
| the project's file grown | 262 KB |

The project's log by what its pages are of (of a first run of five,
read the same way): the index of events by
instance 322 frames, by topic 321, the steps 285, the events 278, the
index of events by run 253, instances 102, instances' versions 85, the
database's first page 71 (it is written again where the file grows or a
page is let go), runs and their two indexes 133, what is under way 54,
handlers' places 75, what is alive 28, records and their history 42.
By who asked for each commit: a step's record with what is said of it
119 (844 frames, 2 a step), a run's ending 27 (323 frames, 12 each), an
event said by itself 42 (223), an instance made 7 (154 frames, 22 each),
an instance's state set 9 (145, 16 each), a handler's run with its place
20 (121), records and their history 16 (118), the search index 10 (67),
the moving of handlers' places 24 (54), the rest under 40 each. The
journal's database did 325 pieces of work that wrote and 457 that only
read (the runner 236 and 141, events 55 and 122, the studio 27 and 119,
agents 7 and 37, who is who 31 reads). 236 events: `step.started` and
`step.finished` 59 each, `run.started` and `run.finished` 27 each, then
`instance.state` 9, a record's history 8, instances started and ended 7
each. By blueprint (instances, runs, steps run, events): `std/reviewing`
0, 4.8, 12.8, 41; `std/working` 0, 5, 13, 40; `std/pool` 0, 4, 12, 38;
`std/review` 1, 3, 11, 37; `std/review-seat` 1, 2, 5, 19; `std/cards`
0, 3, 5, 17; the daemon itself 17 events; `std/lane-round`, `std/work`,
`std/cleared`, `std/keep` and `std/merge` an instance and a run each
and 4 to 6 events, their steps held (`durable: false`). Time: the
daemon's processor 0.18 to 0.24 s a card (0.14 to 0.27), of which the
journal's thread about 0.09 s and the runtime's threads 0.15 to 0.17 s;
from ready to merged 0.21 to 0.25 s (0.15 to 0.32), most of it the
stand-ins' and git's.

So nine tenths of what a card writes is the project's log, and that is
pages written again whole: a card touches 257 pages and writes 2,123,
since each of its 302 commits writes the last page of every index it
adds to, and an event is in four. Three things were cut, each a commit
of its own, largest first:

1. **No index of events by instance or by run** in a journal made now
   (`journal::events::UNKEPT`): 1,920 frames to 1,355. Nothing reads
   by them but what is taken up after a restart, which now says where
   to begin.
2. **An instance's row keeps no text**: the version that has it does
   (`instances.text`). 2,113 frames to 1,920, and the file grows 194 KB
   a card where it grew 264 KB: a text is pages long, and the row is
   written again with every change of state or status.
3. **A step whose action only decides has no write of its own**
   (`ActionSpec::deciding`): 302 commits to 256, 1,355 frames to 1,241.

**After them** (fifteen cards in three runs): written to the
disk 3.30 to 3.38 MB a card (3.04 to 3.58), the project's log 2.50 MB,
1,235 to 1,239 frames in 253 to 255 commits, 4.9 frames a commit, 169
pages; brought back 0.35 MB; the file grown 178 KB. That is 38 % less
written, 42 % fewer frames, half as much brought back. The same 231 to
236 events; the journal's database did 279 pieces of work that wrote
and 447 that read. The daemon's processor 0.17 to 0.20 s a card, and
from ready to merged 0.20 to 0.24 s: no change that two runs of the
same thing do not show between them.

**What was tried before, met again.** Sharing commits among whatever
waits, however little (`JOURNAL_GATHERS=all`), was measured with one
card again: 302 commits to 256, 2,113 frames to 2,026, 4 %. A card's
work comes a piece after another, each waiting on the one before, and
little is ever waiting at once; it is not what a daemon does, for the
reasons *The journal* in [architecture.md](architecture.md) gives. The
size of a page was halved before (*What a write costs the disk is
whole pages*, there) and was left. What the disk is waited for was not
touched: with `synchronous = NORMAL` a commit is not held to the disk,
the log is, each time a thousand pages are brought back (about twice a
card before these, and once after), so a setting to wait less would buy no bytes and a wait or two on the disk a card, and was
not built.

**What is left**, largest first: a step's record with what is said of
it is still two writes for a step that does something (73 commits and
375 frames a card), of which an `emit` step is three (it says its event
by itself); a run's ending 224 frames; a handler's run written with its
place 106; an instance made 111, its version's text a copy for each
instance though seven texts are all a card's; an event said by itself
100. Each is a page or two of the events' table and of its index of
topics, and of the steps' and runs' tables, written again by the next
commit. What reads is time and not bytes: 447 pieces of reading a card,
of which the board read whole for a card's position and by a stage
 and what a `call` reads of a text are most.

**What the runner says of its own steps and runs passed on** (2026-11-21): `step.*` and `run.*` are passed on with their numbers, in
the writes they were said in, and kept only where a text that is running
could hear them (`conductor::owed`). Measured by the test that holds a
card to its budget (`costed.rs`), 192 runs of the program before and 384
after, six at a time held to two processors, a card (plain, with a
command, with a second review of two seats) came to:

| | before | after |
|---|---:|---:|
| events said | 220, 228, 316 to 321 | the same |
| of them kept in the journal | 220, 228, 316 | 61, 65, 84 |
| instances, runs | 8 and 25, 10 and 27, 10 and 32 to 33 | the same |
| pieces of work done | 633 to 643, 684 to 697, 920 to 940 | 632 to 647, 683 to 700, 919 to 944 |
| writes, and commits | 243 to 249, 261 to 263, 357 to 366 | 235 to 242, 250 to 253, 347 to 357 |
| frames to the project's log | 1140 to 1266, 1265 to 1410, 1702 to 1849 | 842 to 962, 964 to 1078, 1236 to 1399 |

A quarter fewer frames; what is kept is the events a text hears (a
fleet's `run.failed`) and everything else said, `instance.*`, a card's
and a record's. The writes are fewer by the run said to have started,
which was an event said by itself and is now a write of the run's row
only where its beginning is passed on and its run keeps what it does.
Once in the 384 (and so left out of the table), a card with a second
review had its usual review begun twice (417 events, 12 instances, 40
runs); nothing like it in the 192 before, nor in some 3,200 runs since
with what was said written down for just that, and so it was not found
why.

With a step's ending held as well (`conductor::held`), 240 runs
of each, six at a time held to two processors, of main as it was and of
both together:

| | with the ending held | and what is said passed on |
|---|---:|---:|
| events said | 220 to 225, 228, 316 to 321 | 220, 228, 316 to 321 |
| instances, runs | 8 and 25 to 26, 10 and 27, 10 and 32 to 33 | 8 and 25, 10 and 27, 10 and 32 to 33 |
| pieces of work done | 611 to 629, 662 to 674, 883 to 904 | 610 to 626, 662 to 676, 885 to 901 |
| writes, and commits | 221 to 229, 238 to 241, 321 to 334 | 212 to 219, 228 to 231, 311 to 321 |
| frames to the project's log | 1114 to 1224, 1210 to 1341, 1627 to 1786 | 834 to 950, 946 to 1051, 1207 to 1348 |

`[card]` of `build-budget.toml` holds the writes, the commits and the
frames to five in a hundred over the most seen with both.

## What a card is held to

What one card costs is held to a budget, as a build's time is: the
`[card]` section of [build-budget.toml](../build-budget.toml), and the
test that holds it, `what_a_card_costs_the_factory_is_held_to_its_budget`
in the API suite (`crates/api/tests/suite/costed.rs`), which runs with
every gate. It works cards through the fleet that ships, with stand-ins
for the agents (a worker, and two reviewers one of whom looks at a card
unless a row says both), on a clock that stands still so that nothing on
a timer comes in the middle of a card. A first card is worked and held to
nothing: it pays for each table's first pages and what is first kept of
each text. Then three, one at a time, each in a factory at rest before and
after, which is known from what was said and asks no database (no run
said to have begun and not ended, no agent's turn queued and not ended,
nothing said and nothing asked of any database for 400 ms, and then the
rows asked once, after the counts are read):

- `plain`: worked, reviewed by one seat, merged and done;
- `command`: the same with a row of `segments` that runs a command in its
  round before its work is kept (`std/run` at 15);
- `council`: the same with a row that lays a second review of two seats,
  both of whom have to approve, after the first (`std/review` at 15).

Each is held, as a ceiling, by seven figures, read from what the journal
counts and from nothing timed. The pieces of work, the writes and the
commits are what the card asked for itself: what the factory does in
whichever card's moment it falls is counted apart (below), and the
three are the same in every run:

| figure | what it is | plain | command | council |
|---|---|---:|---:|---:|
| `events` | everything said, in every journal | 220 | 228 | 316 |
| `instances` | `instance.started` | 8 | 10 | 10 |
| `runs` | `run.started` | 25 | 27 | 32 |
| `done` | pieces of work the journals' databases did (`Db::done`, what `db.work` counts), but for what is counted apart | 590 | 635 | 849 |
| `writes` | of those, the ones that wrote (`Db::writes`), but for what is counted apart | 204 | 214 | 296 |
| `commits` | commits that wrote to a log (`Db::commits`) | as `writes` | as `writes` | as `writes` |
| `frames` | frames written to the project's log (`Db::logged`) | 818 to 848 | 941 to 986 | 1245 to 1327 |

Events, instances and runs are of forty-eight runs on 2026-11-21, six at a
time held to two processors; the rest of 480 runs the same way, once a step
whose action is the same done twice had its ending written with the next
thing its run writes (`conductor::holds`). Before that they were 634 to
643, 684 to 696 and 921 to 933 pieces of work, 243 to 249, 258 to 263 and
357 to 361 writes, and 1158 to 1262, 1274 to 1379 and 1692 to 1864 frames:
some twenty writes a card, every one of them the runner's. Two of the 480
council cards had a run more and five events more (321 and 33), both in
the first round, with a simulation running beside them, and none of the
other 478. A frame is a page and its head, 2,072 bytes, so the plain card
writes some 2.4 MB to the project's log. What it comes to is printed (run
the test with `--nocapture`), with the pieces of work that wrote by
database and by whose work they were, from `db.work`: for the plain card
the runner's 146 to 153 in the project's database (169 to 173 before its
endings were held), the journal's own 39 there, the studio's 17, the
agents' 7, the factory's own database 13 (one of them the test writing
out what was measured, which it does after the counts are read), and the
search index's 10, which is no journal's and is printed and not held.

Three of the figures are the same every time, on any machine, but for the
two council cards above: what is said and made goes by what the texts say.
Which run those two had more is not known yet: crowded again in 432 runs
the same way, with a simulation beside some of them, it did not come.

**What is counted apart** (2026-11-22). Of what a card's
moment holds, some is the factory's whichever card it falls in, or comes
about or not by which of two things side by side came first, and each
such piece of work is measured as itself, so that the test takes it out:

- a handler's place kept for events it passed over, each 256 it was
  handed or each 256 of a topic it waits on (`conductor::PLACES_KEPT`):
  which card it falls in goes by everything said before, and by how a
  handler's reading fell (seven to ten a plain card, and some ten more
  on a board that 250 cards were added to);
- a step's record written by itself that would have been held for the
  next thing its run writes, but for another step of the run at work in
  that moment (`conductor::WRITTEN_BESIDE`);
- an instance's row read for want of it in hand, its parent or whose it
  is, as it ends (`runs::LOOKED_UP`, four to fifteen a card);
- a page a follower read back from the log, of what was said between
  its place being taken and its first look (`journal::READ_BACK`);
- a turn read by its id in every file once it had ended and was
  forgotten as in hand (`agents::TURN_LOOKED_UP`, by how many files more
  than one), and read by whoever waits for it as it is woken
  (`agents::TURN_WAITED`): whether a council's seat had ended as the
  council first looked goes by the machine.

Counted so, 30 runs held to two processors, three at a time, came to the
same pieces of work and writes in every run. Before, the writes came to
212 to 219, 228 to 231 and 311 to 321, and the pieces of work of one
kind of card as much as fifteen apart; that the runner's moved by a few was taken for the
order handlers side by side come to their steps in, and was the
keepings. Each budget is five in a hundred over what was seen, rounded
up. The frames still have the keepings in them, since no page is counted
for what wrote it; those are a page or so each and move them by a few.
How many frames a write takes is how many pages it touches. That went
by where the ids a card is given fall among an index's pages:
an id was random after its millisecond, and the test's clock stands
still, so every id of the factory was one millisecond's and fell
anywhere among the pages of what is kept by it (an instance, a run and
its steps, a handler's place, a turn), each on a page of its own and
splitting pages wherever it fell. Sixty runs, three at a time held to
two processors, came to 868 to 942, 998 to 1094 and 1283 to 1426 frames,
and the council card was over its 1416 twice (180 such runs came to 842
to 942, 979 to 1094 and 1283 to 1426, five over). Ids made in one
millisecond now follow one another (`kernel::id::new`, held by
`ids_made_in_one_millisecond_sort_in_the_order_they_were_made_whatever_is_made_between`
in the kernel's suite), so a row is put after the last, where the last
page is, which is what a factory whose clock goes on had too, but for
what it makes within a millisecond. 240 runs the same way came to
the figures in the table, some seven in a hundred fewer and half as far
apart or less, and the same in the API suite run whole, whose other
factories make ids on other clocks between this one's. What is left goes by the order runs side by side write in, which
is the machine's to say: where a page splits, and how many of a step's
records, rewritten longer as it ends, share one; the writes are the same
in every run. A card with a run more (seven events, a handler's run that
comes about one time in fifteen to sixty, crowded) writes some thirty
frames more and is inside the spread. It is held to five in a hundred
over the most seen, as the rest are. And a commit is a write by itself unless the journal's thread
fell behind and gathered several (`journal::gather`), which can only make
fewer: in the runs above it never did, and played told `JOURNAL_GATHERS=all`
(`cargo xtask ways gathered`) there are far fewer, and the frames with them.
The test is a project's own journal's, so it is not played told
`PROJECT_JOURNALS=one`. The frames are counted as each commit says how long
the log is now (`journal::checkpoint::Logged`): one that finds the log
begun again wrote all of it, and the log begins again only once a thousand
pages have been brought back, which a commit never writes.

**When it fails** it says which figure of which card grew, by how much,
and what wrote. Find what grew before anything else: run the test with
`--nocapture` before the change and after, and read the figures and whose
writes moved; *What one card costs*, above, says how a card was taken
apart by table, by index and by the place that asked for each commit,
which is where to look next. A figure that grew for something worth
having is raised in `build-budget.toml` with a sentence beside it that
says what and why, as the other budgets are. One that came down is
written down too, so that what was won is not given back unnoticed.

## What a card costs beside many cards, 2026-11-21

On an older `main` (2026-11-18) the same card, worked one at a time,
cost the studio 12.3 ms of the journal's thread with a dozen cards on
the board and 29.1 ms with 131. How many pieces of work it asked for
did not move: what grew was what each read, since a few of them read
the board whole for one card. A second test beside the budget's holds
that now, `what_a_card_costs_is_the_same_beside_three_cards_as_beside_two_hundred_and_fifty_three`
(`costed.rs`): the plain card of the budget, worked on a board of four
cards and then on one of 254 (two hundred more waiting in its first
column and fifty done), with every journal's database counting its own
steps (SQLite's progress handler, once a step, on the connection its
thread asks everything on). With what is counted apart taken out
(*What is counted apart*, above) the journal's counts are held to be
the same to the one, and the steps, which have what is counted apart in
them, to a fiftieth. It was a twentieth while a handler's place kept for
events it passed over was counted with the card: the cards added
between brought some ten of those into the second card, and the first
came to seven, nine or ten from one run to the next, which once put the
two 13 writes apart where 12 were allowed (214 and 227). Counted apart,
either card writes 204 times and does 590 pieces of work in every run.
Run with `--nocapture` it prints every piece of work each card asked
for, by database, by whether it wrote and by whose it was, with how long
it took.

What read a table whole for one card, each found by counting the reads
of a table whole while the card of 254 was worked (seven, one of them
the test's own reading of the board once the card is done):

| what | who asks | what it asks now |
|---|---|---|
| a card's place at the bottom of its column as it is added (`Board::add`) | whoever adds a card | the highest position in that column, by the index of records by column (`Collections::highest_in`) |
| a card's place as it is moved, at the bottom or above another (`Board::moved`) | every move: the person's, the round's, the review's | the card it goes above by its id (`record_of`), and the highest position above that in the column, by the same index |
| what waits on a card that is finished (`board.came`), and `board.held` | `std/cards`, as a card comes to done, and as it starts | the cards that say they wait on others, by an index of only those (`waiting_on_others`); the board whole only where it keeps nothing of what a card waits for |
| the next record's number (`last_number`) | every record added | the highest number by an index of a table's records by their number, which the one by table alone was not |
| a stage's look at its column (`board.list` with `status`, `std/due`) | a stage as it comes up | that column alone (`Board::column`) |
| the record a field named before it was changed (`pointed`) | a write that names a record that is gone | that record by its id |

The indexes are one step, the last of the records' tables
(`crates/studio/src/tables.rs`): the index of a table's records with
the number after it in place of the one without, one of records by
their column and position holding only the records that say a column,
and one holding only those that say they wait on cards. Made over
300,000 records of 322 MB, from memory, the three took 1.0 to 1.1 s;
over 30,000 of 32 MB, 0.11 to 0.14 s (`EXPLAIN QUERY PLAN` says each
question is asked of its index). A record that says neither a column
nor that it waits is in no more indexes than it was.

| | steps, a board of 4 | steps, of 254 | the studio's reads, of 4 | of 254 |
|---|---:|---:|---:|---:|
| before | 62,040 | 107,715 (74 % more) | 9.7 ms | 16.9 ms |
| after | 61,839 | 62,334 (0.8 % more) | 12.6 ms | 12.7 ms |

Pieces of work, writes, commits, events, instances and runs are the
same before and after, at both sizes (640 and 651, 247 and 257, 220,
8, 25); the frames of the project's log came to 1,171 to 1,223 either
way, within what one run differs from another by. What is left of the
growth is the 495 steps: an index a level deeper, and what the
journal's reads of events go over with 250 more said. Times are of one
run each on a shared machine and are said for the order of things.

### Beside another table's rows, 2026-11-23

A card's move was found to cost some 35 steps more for each row of
another table of its project: 3,416 beside 3 rows and 5,468 beside 60,
counted by a project whose table of how long each card
took is written a row at a time by a run of a blueprint. Profiled a
statement at a time (SQLite's own count of each statement's steps), the
move read nothing more beside 60 rows than beside 3: what grew was the
counting. Whatever waits for a factory to be at rest asked what runs
were under way of the newest five hundred runs there were
(`observed::at_work`, which the simulation's `rested` and `costed.rs`
both ask, inside the window they count), and each row of that table
was a run more to read. It asks the runs under way now (`under_way`, in
every file that keeps runs), which is what it wanted to know and costs
the same however many runs there were; it had also missed a run under
way older than the newest five hundred.

`what_a_card_costs_is_the_same_beside_a_table_of_three_rows_as_beside_one_of_sixty`
(`costed.rs`) holds it: the plain card of the budget, and a card nobody
works moved to done and back, each beside a table of the project's own
of three rows and then of sixty, each row naming a card and written by
a run of its own, held to the same steps to a fiftieth. Counted as it
was counted before, the move came to 9,143 steps beside three rows and
10,272 beside sixty, and fails; counted now, 7,923 and 7,928. The
card's whole journey came to 64,236 and 64,747.

The same test reads what each of the two cards cost, by round (`GET
/api/board/items/{id}/cost`), and holds that reading to the
same fiftieth: it asks for the card, its board's columns and its
history by their keys, and for its turns by the index of turns by
conversation (`turns_thread`), so nothing is kept for it and nothing
of the cards beside it is read. On the first day it came to 754 steps
beside four cards and 755 beside 254.

## What twenty cards cost the daemon, 2026-11-26

Asked of the daemon as a whole, and not of one card in a test: twenty
cards through a fleet of three workers and a reviewer, stand-in agents,
the optimised program (`compare/measure.py <clone> load`, the first and
the last measured once each, in a lane of eight processors). On release
`a9c85db3` that was 3.94 s of the daemon's processor time and 55.8 MB
written, where the copy with the card's path in Rust (the same
everything else) took 1.04 s and 16.7 MB: 3.8 and 3.3 times.

**How it was taken apart.** Times on this machine move by a fifth from
one run to the next, the same program, with the machine at a load of 10
to 20; so what was held was counted, and times were read beside it.

- **Instructions.** The processor's own counters can be read here
  (`perf_event_open`, of a program of one's own, counting what it does
  outside the system), though no profiler is installed. A few lines of C
  loaded beside the daemon (`LD_PRELOAD`) open a counter of instructions
  and one of cycles for each thread as it starts (`pthread_create`
  wrapped), and on a signal add them up into a file; the driver signals
  as the cards are made ready and as the last is done. Twenty cards came
  to 8.85 to 8.90 G instructions in every run of the same program, a
  half a hundredth apart, where its processor time went from 3.0 to
  4.7 s. A change that saves a twentieth shows in one run each.
- **Where the time was**, by the sampler of the 2026-11-26 section above
  (`setitimer(ITIMER_PROF)`, `backtrace()` at each tick), switched on by
  the same signal so that only the cards are sampled, its addresses named
  from the program's own symbols (`nm`): the system's library has none
  here, so a frame of it is named by the nearest name it exports, and
  what each such frame was is told by what called it (`pwrite`, `pread`
  and `open` under a name of the system's waiting, `malloc`'s insides
  under `__default_morecore`, `memcpy` under `_dl_mcount_wrapper`). Some
  400 ticks a run: four runs were added together.
- **What was written**, by thread (`/proc/<pid>/task/*/io`): each
  database's thread writes its log and nothing else, so what each
  database cost the disk is its thread's; and by table, from the
  project's log kept as the last card was done, its frames' pages named
  by `dbstat` of a copy, and its commits grouped by which tables they
  wrote together.
- **Alternating.** Each program a copy of its own beside a
  `.fingerprint` folder, so that it reads the checkout as the one cargo
  built does, run in turn with the others, three to six rounds.

The driver, the sampler and the counter are under the clone's
`target/sampler/` and are not kept here.

**What the processor did** on `a9c85db3`, of the daemon's ticks: the
database preparing statements, a tenth (three in ten of its own thread);
the runner's steps copying the run's scope, a step's view of itself and
dropping them, a tenth; checking texts as each child begins, reading
every blueprint the library has from the disk each time, a tenth;
reading every table of a project and working out what each says of
itself to find which owns a card, as a card is looked up at every move
and write, a thirtieth; and the system's side, two fifths of the time
in all, mostly writing the logs' frames, reading pages the database's
cache did not hold, and waking threads. **What was written**: the
project's log 44 MB, its pages brought back into its file 4.5 MB, the
search index 4.4 MB, the factory's own database 0.9 MB. The project's
log by table: the steps a quarter, events and their index of topics a
sixth, runs and their two indexes a sixth, the instances' versions
(each instance made wrote its text again, pages long, though 203
instances ran 25 texts) and the first page (written again whenever the
file grows) an eighth, handlers' places and what is under way a tenth.

**What was done**, each a commit, each measured against what came before
it by instructions (the median of three, alternating) and then all of
them against the release:

| change | instructions | processor | written |
|---|---:|---:|---:|
| `a9c85db3` | 8.90 G | 3.94 s | 55.0 MB |
| statements kept prepared (`journal::Prepared`, 512 a connection) | 8.90 to 8.56 G | | |
| a step's view of itself worked out only where a hook reads it | 8.56 to 7.87 G | | |
| and planned by the statement alone (`SQLITE_DBCONFIG_ENABLE_QPSG`) | 7.87 to 6.66 G | 3.63 to 3.33 s | |
| a card looked up reads whole only the table that may own it | 6.69 to 6.15 G | 3.62 to 3.26 s | |
| the search index written again only where what is found changed | | | 53.5 to 51.6 MB |
| an instance's text kept once in its project | | | 51.6 to 48.9 MB |

Kept prepared, a statement was planned again each time it was given
another value: the database is built to weigh the values it is given
against what it knows of a table (`STAT4`), and marks such a statement
to be prepared again when a value changes, which was nearly every
statement here. So keeping them saved a twentieth until each
connection was told to plan by the statement alone, and then a fifth
more. That changes no plan here: nothing has the database weigh a
table's values (no `ANALYZE`), and every partial index is read by what
its own condition says, written out in the statement, or by what
implies it whatever the value (`owes = ?1`); the windows of a table say
their project and table as text for the same reason. The card the
budget holds asks the database 64,493 steps where it asked 67,904, and
the same pieces of work, writes, commits and frames; the search index's
writes are not held, and an instance's text was written on the first
day of a project's journal as now. So no figure of `[card]` moved, and
none was lowered.

**Before and after**, the release, the last commit and the Rust copy
(`target/rustcopy`, phase 1b at `aa1aea8`, built here) in turn, five
rounds, the machine at a load of 10 to 20 (median, with the least):

| | instructions | processor | of it the system's | written | from ready to done | at most |
|---|---:|---:|---:|---:|---:|---:|
| `a9c85db3` | 8.85 G | 3.18 s (3.01) | 1.39 s | 55.0 MB | 1.17 s | 88.8 MB |
| the last commit | 6.09 G | 2.60 s (2.47) | 1.19 s | 47.9 MB | 1.00 s | 89.4 MB |
| the Rust copy | 2.22 G | 1.14 s (1.08) | 0.55 s | 16.7 MB | 0.89 s | 73.5 MB |
| the last against the Rust copy | 2.7 | 2.3 | 2.2 | 2.9 | 1.1 | 1.2 |

At scale (`measure.py <clone> scale:20,100`, twenty workers and five
reviewers), the last commit came to 3.98 s and 52.0 MB at 20 cards and
21.8 s and 290.9 MB at 100, where release `effb6a27` came to 4.87 s and
57.9 MB, and 28.4 s and 322.0 MB (`compare/SCALE-2026-11-27.md`): 1.8
and 2.6 times the Rust copy at 100. What a card writes grows from 20
cards to 100 by a tenth: the project's log 2.2 to 2.4 MB a card, as more
runs are under way at once and their steps' rows fall on more pages, and
pages brought back into the project's file 0.18 to 0.28 MB a card, as
the file is larger and fewer of the pages a checkpoint brings back were
brought back by the one before.

**Tried and not kept.** Every file of the library looked for and read
once in a check (a check asked the disk the same name some ten times
over): 6.29 G instructions where the program without it took 6.12, and
the system's time 1.38 s where it was 1.42, six rounds each; the system
on this machine answers those asks quicker than the sampler made them
look. And sharing commits among whatever waits on these twenty cards
(`JOURNAL_GATHERS=all`): 52.3 MB where 54.0, and 1.13 s where 1.07, so
`gather::DEEP` was left.

**Left**, largest first: the system's side, the frames of the logs
written two calls each, the pages a step's records rewrite, the
threads woken; checking a text as each child begins, which asks the
library for every blueprint there is; a run's scope built again for
each step, every step before it copied in; the steps' rows, written as
a step begins and again as it ends; reading an instance reads its text
whole, pages long, wherever one is asked for.

## Tried on the old codebase, and not adopted

So that nobody spends a week finding out again.

- **sccache.** Never showed a hit: 0 hits, 0 misses, 27 not cacheable across
  eight builds. Workspace crates compiled incrementally cannot be cached.
- **A faster linker (mold).** No verdict: the experiment was struck before it
  ran, and the reason for wanting it (the disk being the limit) was later
  shown false.
- **`split-debuginfo = "unpacked"`.** 17 % fewer bytes; no time saved.
- **No debug info at all in tests.** 45 % fewer bytes, and every `file:line`
  gone from test backtraces. Rejected for that.
- **Incremental on cold builds.** 6.6 % slower in total, as above.
- **`cargo-sweep` by age.** Deletes the wrong things: old third-party
  artifacts that are still needed, while keeping generations of the
  workspace's own. Pruning the workspace's own artifacts after a run took one
  target directory from 43.8 GiB to 1.8 GiB with no change in build time, at
  the price of rebuilding them when the same commit is checked twice.
- **`cargo nextest`.** This one helped (39.5 % off a 397 s test run) and was
  not adopted there only because it had to be installed everywhere. What it
  gave there was test programs run side by side, and a process for each
  test. The gate here now does the first and most of the second itself
  (*Test programs run side by side*); a process for every test was
  measured and is no faster than eight threads to a process.

## How to measure

The old experiments' own rule, which is worth keeping: a difference only
counts if it is bigger than the difference between two runs of the same
thing, measured in alternation (A, B, B, A) on a machine doing nothing else.
Several early conclusions there were reversed once this was done.

```sh
cargo xtask timings          # each gate over time, and the slowest tests of the last run
cargo xtask changes          # how many commits needed a rebuild, week by week
cargo build --timings        # which crates, and how long each
```

Where a suite takes long and no test in it does, and there is no
profiler: time the program under `/usr/bin/time -f '%e %U %S %w'`
before anything else. Seconds in the system far above the seconds in
the program, or voluntary switches in the tens of millions, is threads
waiting on each other, and no test made shorter will mend it. Then
which threads (`/proc/<pid>/task/*/stat`, by name), what each sleeps in
(`/proc/<pid>/task/*/syscall`, read by the program's parent), and one
thing changed at a time by a word in the environment (*Where the tests'
time was on 2026-11-19*, above).

## A test that fails now and then

The gate has to say the same on a busy machine as on a quiet one, and a
test that fails once in fifty runs fails for an order of two things that
a machine with processors to spare never comes to. Run again by itself
it passes. What brings the order about is more of the test at once than
there are processors for, over and over:

```sh
cargo xtask crowd conductor begun::a_test_named_whole   # forty runs, four at once, held to two processors
cargo xtask crowd api rows::another --times 200 --at-once 6 --cpus 2 --limit 120
```

The test program is the one the gate runs, built in the same one build
(`xtask/src/suites.rs`), and run as the gate runs it, from its crate's
folder. Everything a run that failed printed is kept, a file a run,
under `target/xtask/crowd/<crate>/<test>/`, and of the runs that passed
nothing; what was kept by the crowd before is taken away first. The last
line says how many ran, how, and how many failed, and the command fails
where any did. Three things it will not take for a pass: a name that is
no test of the program's (a test is named whole, with its module, and
the command says which tests have the words in their names); a run that
ended well having run no test; and a run that had not ended when its time
was up (`--limit`, ten minutes unless said), which is ended and counted
apart from a failure, with what it had printed kept. The runs are held
to the first processors of those the command itself may use, so that one
started where it is already held to some stays among them; that is
`taskset`, and where a system has none the command says so and the runs
are crowded only by one another. A run is told which it is
(`CROWD_RUN`), and a test's own waits say what there was when they ran
out (`Rig::event` in the API suite, `Rig::soon` in the conductor's), so
what is kept of one failure in fifty is usually enough to read it from.

## Checked for other systems, from this one

Nothing here has been run on macOS, and the workflow that would build
and test on either system (`.github/workflows/elsewhere.yml`) has not
been run. On Windows the suites have been built and run by hand (*Run
on Windows*, below). What can be found with neither system at hand is
what would not compile there, and that is asked of the compiler on
Linux:

```sh
rustup target add x86_64-pc-windows-msvc aarch64-apple-darwin
cargo check --workspace --tests --target x86_64-pc-windows-msvc --target-dir target/win
cargo check --workspace --tests --target aarch64-apple-darwin --target-dir target/mac
```

A check reads everything that would be compiled for the system it is
told, the tests and the desktop program with the rest, and makes no
program: nothing is linked, so none of that system's own libraries is
needed, and nothing is run. Two dependencies build C even to be checked
(the database, and for the window on macOS a helper of the bindings to
that system), and are told a C compiler and an archiver that make
objects for the system asked about, in the variables the `cc` crate
reads (`CC_<target>`, `AR_<target>`): `zig cc` stood in for both where
this was done. A target folder for each keeps the ordinary build as it
is.

On 2026-11-19 both end with no error and no warning. That says the code
is written for both systems and nothing of what it does on either: not
that a program links there, or starts, or that one test passes. It is
asked by hand and no gate asks it, so a change may undo it unseen: the
usual way is a test that uses a helper of a module that is marked for
Unix and is not marked so itself.

A package for another system is made from here the same way a program
for it is, and is for trying there, never for a release:

```sh
cargo xtask package --target x86_64-pc-windows-gnu --cargo-build zigbuild --no-window
```

`--target` is handed to cargo and names the package, which then holds
that system's programs under that system's names, with a page in its
commands and a line saying where it was built. `--cargo-build` is the
word cargo is asked in place of `build`: what builds and links for the
other system is not this toolchain alone (`cargo zigbuild`, with `zig`
as the linker, where this was done), and the target has to have been
added with `rustup` to the toolchain that builds: where the pinned one
has it not and another of the same version has, `RUSTUP_TOOLCHAIN` says
which is used. `--no-window` leaves the window's program out,
which is not known to build for another system from here. The zip of a
package for Windows and the sum beside it are written by the tooling
itself, so nothing else of the other system is needed. Nothing in such a
package is run where it is made.

A test that rests on what only Unix has is marked `cfg(unix)`: the
module where every test of it does, and the one test where it alone
does, each with a sentence beside the mark saying what of Unix it
needs. On 2026-11-19 Linux ran 2,032 tests and Windows compiled 1,838,
which is 194 fewer: 79 fewer of `agents`' 154, 52 of `cli`'s 108, 30 of
the tooling's 104 (`xtask`), 20 of `api`'s 551, 8 of `hosting`'s 104
(11 that are Unix's or Linux's, less three that are Windows' own) and 5
elsewhere. On 2026-11-20 Linux ran 2,265 (the gate's count: 1,537 in
`test` and 728 in `offline`) and Windows compiled 2,047, which is 218
fewer, for the same reasons. macOS compiles all but the few that are
for Linux alone.

What is still Unix's, by what it needs:

- **A wall** (32): held by a program of Linux's (`bubblewrap`), and
  nothing holds one on Windows. `walls.rs` of `hosting` and of
  `agents`, `keys.rs` and `repositories.rs` there, `walled.rs` of
  `api`, one test of `outlives.rs`.
- **A package's or an agent's program that is a file of a Unix shell**
  (63), with commands or state of its own and made a program by its
  mode: `actions.rs`, `extensions.rs` and `signins.rs` of `agents` and
  of `api`. These are owed a program in their place, as the rigs'
  stand-ins now are; the stand-in for a program does one thing after
  another and is not a made-up command line.
- **A daemon that is the real program as a process of its own, stopped
  by a signal** (52): the command line's rig (`daemon.rs`), and the
  fourteen modules of that suite that stand on it or start lines of a
  Unix shell, the wrapper's own tests among them (`wrapped.rs`).
- **The tooling's stand-ins for a build** (30): `crowd.rs`,
  `install.rs` and `package.rs` of `xtask`, where a file of a Unix
  shell stands in for cargo or for a test program, a program is told
  by its mode, and an install's commands are links.
- **What only a signal, a pipe with a name, a link or a file's mode
  says** (the rest): two tests of `hosting`'s `kept.rs`, three of
  `agents`' `outlives.rs`, a test each of `api`'s `copies.rs` and
  `stores.rs`, `given_back.rs` there and two of `hosting`'s `watch.rs`
  (Linux alone: `/proc`, and what its notifications say), the example
  whose steps are scripts of several lines (`examples.rs`), and
  `std/log-feed`, the one blueprint that ships which needs `tail`.

### Run on Windows

On 2026-11-19 every suite was built on Windows 11 by that system's own
toolchain and run there (`cargo test --no-fail-fast`, from a clone in a
scratch folder, by hand: no gate and no workflow runs anything there,
so this says what held that day). Every one of the 1,838 tests compiled
for Windows passed:

| Crate | Tests on Linux | Compiled for Windows | Passed there |
|---|---|---|---|
| `api` | 551 | 531 | 531 |
| `conductor` | 391 | 391 | 391 |
| `blueprint` | 162 | 162 | 162 |
| `studio` | 146 | 144 | 144 |
| `journal` | 110 | 110 | 110 |
| `hosting` | 104 | 96 | 96 |
| `agents` | 154 | 75 | 75 |
| `xtask` | 104 | 74 | 74 |
| `gateway` | 65 | 64 | 64 |
| `cli` | 108 | 56 | 56 |
| `blocks` | 36 | 36 | 36 |
| `deciding` | 35 | 35 | 35 |
| `runs` | 22 | 22 | 22 |
| `kernel` | 20 | 19 | 19 |
| `comparisons` | 18 | 18 | 18 |
| `kv-cache` | 6 | 5 | 5 |

The first run there, that morning, compiled 1,419 and passed 1,397, and
the tests of an agent's work were not compiled at all: their stand-ins
for programs were lines of a Unix shell, held at pipes with names. They
are a program now (`crates/hosting/standin/main.rs`): built with each
suite that starts it, as a program of that suite's own crate
(`CARGO_BIN_EXE_<crate>-standin`, the one source file named by `path`),
the same on every system, told what to do in its arguments, and held
where a test needs it held at a connection to a port the test listens
on, said to it in a file the test makes. A script is written only
where a test is about a shell reading one, once for each shell.

They were run there again on 2026-11-20, the same way and by hand:
2,047 tests were compiled for Windows and every one
passed, none failed, and nothing was left running. Linux ran 2,265 of
the same source, so 218 fewer are compiled there. What each crate came to
that day was not written down: the table above is the first run's.

One test passed and failed there by turns on 2026-11-19, and is not of this work:
`access::a_file_is_attached_to_a_message_fetched_back_and_an_agent_is_told_where_it_is`
failed once in six runs of the API suite, where it sends more than a
channel takes and the connection was ended under what it was still
sending (`ConnectionAborted`) before the refusal was read. On Linux
the refusal is read. More than a route takes is read to its end before
it is refused since (`crates/api/tests/suite/sent.rs`), which is what
was in that test's way.

What those runs found, beside what was wrong with the tests:

- What a program printed after its first line was read when the
  program ended: Windows says that a file kept open was written to
  once, and then when it is let go of. So there the daemon looks at
  the output of what is at work four times a second as well as being
  told (`docs/architecture.md`, *Being told, not looking*).
- A daemon told to listen on every address said it was running at
  `http://0.0.0.0:<port>`, which Windows will not connect to: it says
  this device's own address now, on every system.
- A connection to an address nothing listens at is refused after about
  two seconds there, where Linux refuses at once: the system goes on
  trying meanwhile. The stand-in's test of it takes 2.07 s, and the
  test of a backend that will not answer 6.06 s for three such
  connections, which is most of the twelve seconds the hosting suite
  takes there against one on Linux. It is felt wherever the product
  asks an address that is not answering: a command that finds no
  daemon says so after two seconds (the command line's test of it
  takes 2.07 s there); a connection a router first
  sends to a backend that is down is answered by another two seconds
  later; and one sent to a service that ended badly reaches it once it
  has been started again, a second later, since the system is still
  trying (the example's version that dies under traffic answered
  twice there: its sixteenth connection after a second and a half, by
  the version started again). Nothing in the product was changed for
  it: a backend whose program has ended is still routed to until it is
  seen to be unhealthy or its slot rolls it back, on every system, and
  what it costs on Linux is nothing.
- That system's shell puts what a `%NAME%` holds into its line before
  it reads the line: a user's line that writes `%MESSAGE%` of what was
  said to a bot runs what was said (`echo heard %MESSAGE%`, told
  `this& echo that`, said both). The line names a program that reads
  the variable instead; `hosting::script` and `docs/status.md` say so.
- A program whose group is let go of ends there with the code 0, as
  one that ended well does: where a test's own process stands in for
  the wrapper, a program that was ended reads as one that ended well,
  which on Unix a signal tells apart.
- A folder with a file in it that a program has open cannot be given
  another name there, and git checks files out with that system's line
  endings unless the repository says not: both were tests' to allow
  for, and are said in `docs/working/testing.md`.
