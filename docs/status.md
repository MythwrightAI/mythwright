# Status

What this preview holds, and what each claim rests on. The README says it
in short; this is the whole of it.

Version 0.1.0. It is early, and this is what that means.

| | |
|---|---|
| **Works, and is tested on every change** | Linux on x86_64: the daemon, the command line and the client's own logic (the desktop window's own is tested by the hosted run, when that is asked for, and not by the gate). A card taken by an agent, worked on a branch of its own, sent back from review and merged. An agent's program behind a wall. A daemon stopped while agents have cards and started again, held place by place. A running tree kept to the blueprints it reached, taken forward and put back when asked. One blueprint started several ways and counted side by side, an agent's words among them. No blueprint that ships works anything out for itself, and each is said in plain words from its text. A blueprint run in a sandbox, a factory of its own made for the test, with the factory in use held to being as it was. What holds each is listed below the table, by the test that does. |
| **Played with stand-ins, and in daily use** | The whole loop (a card planned, worked, reviewed, sent back and merged; several agents; councils; a restart in the middle of work; comparisons) is played end to end on every change by the test suites, with stand-in programs in place of the agents' own, and the daemon stopped and started at every step of it. The project's own work is tracked and worked through a factory every day: an agent from outside working cards on its board, with a person reviewing. What is not yet shown: agents' real programs left to run many cards unattended over a long stretch, and many agents at once at volume. Nor has a real agent's program been left at work while the daemon was down, and a program kept going that way has not been run behind a wall the system really holds (the test of it tells a stand-in for the wall what to show). The packages for Codex and Kimi have answered in channels and have not taken a card end to end. The GitHub package is played with a stand-in for its program, written from that program's documentation. The Jira and Notion packages are played the same way, with a stand-in for curl answering as each service's web API is documented to, and neither has been run against its service. Issues kept in step with cards have also been tried by hand against GitHub itself, on 2026-11-24, on one private repository with a factory made for it: issues linked out and in by a label each, what is not linked kept on its own side, a title and a body changed both ways, an issue closed and opened again both ways, things said both ways with what was said on a card and not marked public kept on the card, a label given later bringing an issue in, and an issue made on GitHub coming in by the timer within a minute. Not run against GitHub: the signed call GitHub sends (its webhook), pull requests, the Git page's parts and the Git table, a repository with many issues, and what GitHub says of an author's standing in a repository, which so far only the stand-in has said. Claude Code, Codex and Kimi behind a wall: each has been started behind one and has answered which models it offers (`mw agent try`); none has yet done a piece of work behind one. The client is looked at by driving it in a headless browser (`ui/scripts`), by hand and not on every change, and the desktop window has only been looked at on Linux. On Windows, by hand, once, on one machine, with the program built for it on Linux and stand-ins for agents' programs: the daemon started, stopped and started again with what it kept; a project with its board, fleet and library; a card worked, committed, reviewed and merged; a step's program, a script of that system's shell, a service and an agent's program started and stopped, each with what it had started; and a service and an agent's kept program going on without the daemon and found again by the next. No real agent's program and no wall has been run there. The window's program has been built there by that system's own toolchain and run where it opens no window (on 2026-11-21): asked what it takes, and told to start nothing at an address where nothing answered, which it said in a sentence before it ended; no window of it has been seen there. The suites have been built and run there too, by hand, on that machine and by that system's own toolchain, on 2026-11-19 and again on 2026-11-20: that second day every one of the 2,047 tests compiled for Windows passed, among them the runner's whole suite and nearly all of the API suite's, with a card worked, reviewed and merged by a fleet and the daemon stopped and started in the middle of each, its agents' programs a stand-in that is a program on every system. 218 fewer tests are compiled there than the 2,265 that run on Linux: what needs a wall, the tests of packages and of sign-ins whose stand-ins are files of a Unix shell, the command line's tests that start the real program as a daemon of its own and stop it with a signal, and three modules of the tooling's ([docs/build-times.md](build-times.md), *Run on Windows*, has each crate and each reason). No gate runs anything there, so that says what held on those days. The suites played the other ways they are to hold (every project in the factory's own journal, as a factory an earlier program made keeps them; each play of the runner's suite a project's with a journal of its own; whatever waits for a database kept by one commit; and the journal's own suite with every piece of work kept by a commit of its own): by `cargo xtask ways`, which is asked for before a change to where anything is kept and by the hosted run after its gate. No gate plays them: the gates play every suite as a daemon is, a project that is added having a journal of its own. |
| **Written for, and never run** | macOS, and on Windows everything the row above does not name. The code avoids what is particular to one system; the workflow that would build and test it on either has never been run, and no test has been run on macOS. That the application and its tests would compile for both is checked from Linux, by hand, which says nothing of what any of it does there ([docs/build-times.md](build-times.md), *Checked for other systems, from this one*). A package for each: what one is called, what is in it and what its page says on each system are functions played on Linux (`xtask/tests/suite/systems.rs`, `package.rs`, `zip.rs`, `sha256.rs`; that it carries the notices of the libraries it is built from, made from what is locked now, `notices.rs` and `noticed.rs`), and one for Windows has been made on Linux and read back there; none has been made on Windows or on macOS, the workflow that would make and release them has never been run, and macOS's own `tar` has never been asked what the tooling asks it. What starts a program for the factory, waits for it and says how it ended is the program itself on every system, with no shell. Stopping a program and everything it started on Windows is a job object, held by what started the program, and a stop is an ending: nothing is asked first ([below](#what-is-not-held-yet), *Systems*). |
| **Not built** | An installer for any system, and anything signed: a package is a folder to unpack, and Windows and macOS each ask before they start a program from one. An application for macOS (a bundle, with its icon): the window there is a plain program started from a terminal. The icon in the Windows programs. Asking a program to stop on Windows, before it is ended. A wall on macOS or Windows, and one around anything but an agent's program: a command a blueprint runs, or a service, runs as you ([protected.md](protected.md)). Hosting a library of documents on the web (it is written out as pages for a host of your own). Comparing two running services request by request. [docs/roadmap.md](roadmap.md) has all of it. |

What the first row rests on. Each is a test of the gate
(`cargo xtask check`, which every change passes before it is landed),
named by its file under
`crates/<crate>/tests/suite/`:

- **Work landed by a pipeline of a project's own**: other items' branches
  gathered into one place, all of them or none, and the project's branch
  brought forward to it only by a fast-forward and sent, keeping what
  somebody changed in its folder (`api`: `gathered.rs`), the two stopped at
  every place of the pipeline and coming to the same commits (`api`:
  `landing.rs`); whether every item gathered was passed (`deciding`:
  `passed.rs`); and the factory started again by a step of its own, by what
  keeps it running, once nothing else is under way, its run going on in the
  daemon that comes back however a stop fell (`api`: `restarting.rs`;
  `cli`: `restarted.rs`, with the real `up`).
- **A card's journey** through the fleet that ships, with stand-ins for
  the agents: taken when its turn comes, worked on a branch of its own,
  reviewed, sent back with what was wanted, merged (`api`: `lanes.rs`,
  `rounds.rs`, `fleets.rs`, `planned.rs`); and a card that was in a
  stage's column before the stage's pool was listening is taken all the
  same (`api`: `listening.rs`); one sent back to the column its stage
  says, where that is not where cards are in hand, is counted and given
  to its worker again (`api`: `sentback.rs`); and a stage that starts a
  blueprint of the project's for each card starts it once for each time
  a card comes, and for none that is not there (`api`: `comings.rs`);
  a stage takes its cards in the order its row says, the highest
  priority first where it says so, as many at once as it says and
  never more than its agents can take, and reviews them with a
  blueprint of its own while another stage has the fleet's (`api`:
  `pooled.rs`, `councils.rs`; `blocks`: `ranked.rs`; `studio`:
  `rows.rs`);
  and what a project's `segments` lays into a round or about a review
  is done there, a command before the commit and a label's cards to one
  agent among it, a row's command handed its card (`CARD`,
  `CARD_TITLE`, `CARD_LABELS`) and run where the card was worked or in
  the project's folder as the row says, and one laid before the work,
  where the card has no place yet, run nowhere and the card left
  (`tidied.rs`), and where cards are reviewed something run once a
  card is approved, a second review with seats of its own, a review for
  the cards of a label and a person's sign-off, and where they are
  planned or handed to a blueprint of the row's naming something before
  that and after it; and a fleet that somebody stops and applies again,
  with a card in hand and another being reviewed, gives the one to its
  agent again and reviews the other again, each once, whichever of its
  agents and its stages was made first, with nothing said to have
  failed, while an agent stopped by itself under a card in hand leaves
  the card for a person, said once where the fleet reports; an agent's
  turn on a card, which is how a card is worked and planned, ends with
  words that are a setting of the turn, each with what it comes to, so
  that a third word is a row's to offer, and what the votes of a review
  come to counts the words it is told are for and against; and a
  review's council is one step that asks its agents side by side, is
  decided as soon as it can be with the rest called off, and asked again
  counts a seat called off for that as choosing neither, and is laid by a
  row of `segments` into the list of a stage that works, two looking at
  the work before it is kept, a card both approve kept and one that
  either sends back given to its worker again with what was said; and
  so many things at most are done in one going through a card's list as
  its stage's row says, a card with more left for a person with the
  command that raises the number, which a list under way keeps as it
  began (`api`:
  `segments.rs`, `tidied.rs`, `councils.rs`, `around.rs`, `stopped.rs`;
  `deciding`: `segments.rs`, `decided.rs`, `begun.rs`, `work.rs`,
  `turn.rs`, `tally.rs`; `agents`: `council.rs`).
  The built-in assistant, which
  needs no model, and the first run from a terminal (`cli`: `usage.rs`).
- **What a card cost**, by round: each time it was worked and looked
  over, the agents' turns of it with the tokens and the cost their
  programs said and how long each was at work, and what was decided at
  the round's end (passed, sent back with what was said, left for a
  person), laid out once by the daemon for whoever may read the board
  and for nobody else, a turn of a channel kept to its members with its
  figures alone, and read for one card by keys, as much beside 253
  cards as beside three (`api`: `spent.rs`, `costed.rs`; `deciding`:
  `spent.rs`; `agents`: `kept.rs`); `mw board cost` writes it down
  (`cli`: `fleets.rs`). In the card's drawer it is looked at by a check
  that is run by hand (`ui/scripts/cardcost.mjs`; the client's
  `cardcost.test.ts`).
- **A table of many rows read a window at a time** by the daemon: the
  first page of a view, sorted by a number, by words or by a choice, held
  to a choice, and a page read on from the middle, each costing the
  database the same steps beside a hundred rows as beside a hundred
  thousand (a million, by hand), and a view held to what a list holds
  looking at no more than 20,000 rows an answer (`api`: `windowed.rs`);
  every window the database reads what the same rules read in memory say,
  every kind both ways, and each statement reading by the index it names
  (`studio`: `windowed.rs`); a column indexed as a large table is first
  sorted by it, at most eight a table (`studio`: `windowed.rs`); the
  order, the plan and the figures, held case for case to the client's
  (`kernel`: `window.rs`); a view searched for words and a row gone to
  by its id or its number, the rows about it and where it stands, the
  same read beside a hundred rows and beside many (`api`: `windowed.rs`;
  `studio`: `windowed.rs`). The client reads a table laid out as a table
  this way, draws a row that changed where it is, counted, until the
  reader asks for it to be read again, searches it and goes to a row
  (`ui/scripts/bigtable.mjs`, run by hand; the client's `window.test.ts`,
  `rowslive.test.ts`).
- **A wall** the system really holds, wherever it will hold one, and what
  an agent's program is shown behind it (`hosting`: `walls.rs`; `agents`:
  `walls.rs`, `repositories.rs`; `api`: `walled.rs`).
- **A daemon stopped while agents have cards**: the work goes on, or is
  left for a person where the stage's row says so or after as many such
  stops as it says (`api`: `restarted.rs`). Work begun again as the
  daemon starts, whose card somebody moved away before its round asked
  for it again, is called off and its program ended, and the card put
  back is worked by a round of its own (`api`: `restarted.rs`,
  `takenout.rs`). An agent whose row says
  `outlives` goes on while no daemon is there and is taken up where it
  is, asked once, the daemon once killed outright; and since every
  agent's program is started by its wrapper, what a stop does is asked of
  the row as the daemon stops, so one changed while a turn is in hand
  holds for that turn, either way (`agents`: `outlives.rs`; `cli`:
  `outliving.rs`, `drained.rs`).
- **A daemon stopped once what is in hand is done** (`mw down --drain`):
  nothing new begun from the moment it is asked, an apply refused and
  what is said to an agent left for the next start, the stop waiting for
  the turn in hand to end, and one past its bound saying what it cut off
  and what becomes of it, with the real program (`cli`: `drained.rs`);
  a turn on pipes said to end with the daemon (`agents`: `signins.rs`).
  Each open page says it drains, what it waits for and that it stopped,
  looked at by a check that is run by hand (`ui/scripts/drained.mjs`; the
  client's `drained.test.ts`).
- **Stopped at an exact place and started again**, and held to come to
  what it would have come to: scenarios of a card worked, sent back,
  reviewed and merged, reviewed by a council of three stopped as each
  seat is asked, begins, is at work and has answered, looked at by a
  council a row lays into the stage that works, stopped as each seat
  begins, of a fleet brought up, of a card planned and
  worked with each of its agents asked once whichever turn is in hand,
  of a round of seven things stopped wherever its fifth level is, and of
  a rollout through
  a slot, against the fleet (`api`: `simulated.rs`, `seated.rs`, `segmented.rs`, `rounded.rs`); and the runner by itself, a small blueprint
  for each way a run can go, stopped at every place it passes, two ways,
  and a second time (`conductor`: `anywhere.rs`, with the plays in
  `plays.rs`, `begun.rs`, `below.rs` and `answered.rs`; `waited.rs`).
  Among them: an apply that comes to something while it is being
  stopped (by a call or a spawn of what started it, a step of another
  tree, and somebody asking) waits for the stop and starts it anew; and
  something stopped with others under it has all of them called off
  before any is taken down (`conductor`: `meets.rs`, `cut_first.rs`);
  and what is ended begins nothing more, so that an event waiting
  behind a run an apply cuts is handled once the steps have run again,
  and a step whose action had ended as its run was cut is written as
  it ended, while one cut before its action began does not begin it
  (`conductor`: `called_off.rs`); and what is applied over something
  running is the text as it was read where that had its turn, so that
  a step asking for its child, the look below and somebody asking by
  name do not take it back from a text applied in that moment
  (`conductor`: `in_turn.rs`).
  Every word of the language is
  in a case that is read, laid out and refused written wrongly, and in a
  blueprint that is played and stopped (`blueprint` and `conductor`:
  `held.rs`). A step whose journal shut as it asked something of it
  writes no failure of its own in its log (`conductor`: `refused.rs`).
- **One bot** says what the factory's blueprints say, a failure is said
  there once, and its commands are done as whoever asked and refused a
  key that holds less (`api`: `commanded.rs`, `spoken.rs`).
- **Versions.** Each blueprint that ships says its version, and a version
  is one meaning (`api`: `versions.rs`), and is held to the lines it has, so
  that none grows back into what the fleet was cut out of (`api`: `lines.rs`). A running tree goes on with what
  it reached until somebody applies saying `--take`, and is put back with
  `--back` (`cli`: `moved.rs`). Whether what is
  running holds its blueprints or follows them is a setting of the
  factory, and a blueprint that ships is made a project's own with
  `mw own` (`api` and `cli`: `texts.rs`).
- **Forks.** A project's own copy of a blueprint that ships is a fork
  of it, and what it was forked from is kept as it is made: which
  blueprint, the version, that text to the letter, who and when; said
  as it is made and as it is taken away; still read after the program
  brings another version and the daemon is started again; never made
  up for a fork an earlier program made; gone with the fork, wherever
  a daemon is stopped as one is taken away; handed to nobody who may
  not read the project's files (`api`: `forked.rs`; `conductor`:
  `forked.rs`). It goes into a copy of the factory and with a project
  moved to a journal of its own (`api`: `forked.rs`, `moved.rs`;
  `runs`: `moved.rs`). A fork's summary says what it was forked from,
  where it differs by param, step and handler, and how the default has
  moved since, the same for a fork laid out another way, and one that
  ships says how what it does is changed (`blueprint`: `forked.rs`,
  `differs.rs`; `api`: `summarised.rs`); `mw explain` and `mw own
  --say` print the same parts (`cli`: `texts.rs`). In the client,
  forking from a blueprint's page and going back behind a question are
  looked at by a check that is run by hand (`ui/scripts/forked.mjs`;
  the client's `forked.test.ts`).
- **A project's own blueprints are kept by the factory**, in its own
  data folder and never in the project's: a fork, the fleet's blueprint
  and an example are written there, read, followed and vouched for as
  files of the project's named `@blueprints/...`, and a project that says
  nothing reads none of its repository's files as its own, a project
  from before among them; a folder of the repository is read too, before
  it, where the project says so, and a name in both is read from neither
  (`api`: `blueprinted.rs`, `named.rs`, `forked.rs`, `texts.rs`;
  `texts`: `folders.rs`). It goes into a copy of the factory, into the
  Trash with its project and back, and away with it for good; a
  project moved to a journal of its own reads it where it was; what a
  folder of a repository holds is moved in by one asking, with what each
  fork was forked from (`api`: `blueprinted.rs`, `environs.rs`,
  `moved.rs`); what was vouched for of a folder a project from before
  read is let go once, with the note an instance had of it gone in that
  start (`api`: `blueprinted.rs`). In the client, where a project keeps
  them is said and chosen from the gear of its page, and moved in from
  there; its own are listed under its Blueprints in the side panel with
  what each links, a new one is made from there and from the gear, and
  where one is said leads back to that list, looked at by checks run by
  hand (`ui/scripts/blueprinted.mjs`, `lists.mjs`, `forked.mjs`; the
  client's `blueprinted.test.ts`, `ownmade.test.ts`).
- **A project's Git page.** A project whose folder is a git repository
  of its own is said to be one, and its repository is handed to whoever
  may read the project: the branch its folder is on and how it stands
  against the one it follows, what is changed and what of that is staged,
  the commits of what the folder is on or of a branch asked for as it is
  listed, its branches and its remotes, with nothing written into an
  address before its `@` handed on; and a commit, or the folder made a
  repository, is heard as the system says its folders changed (`api`:
  `gitpage.rs`; `kernel`: `repository.rs`). A package that serves a
  remote of it shows the parts it says, each read by its own program as
  whoever asks holds what using the package takes, a row as its columns
  say and no more, and a manifest that says it wrongly refused saying
  where (`api`: `gitpage.rs`); GitHub's shows a repository's open pull
  requests, open issues and workflow runs, says where nobody signed its
  program in, and asks it nothing of a repository on no remote of
  GitHub's (`cli`: `packages.rs`, its cases in `extensions/github/`). A
  part whose program answered nothing says so, and is never drawn as no
  rows (`api`: `gitpage.rs`); the page is in tabs, each part read only
  as its tab is shown, looked at by a check run by hand
  (`ui/scripts/gitpage.mjs`; the client's `gittabs.test.ts`). GitHub's
  package keeps a Git table beside a project's board, a row for each card
  whose branch reached the remote with its pull request, checks, reviews
  and the issues it closes, read as it is and again as GitHub says it
  changed, written only by the package's blueprint, refused with how to
  sign in where its program is not (`cli`: `packages.rs`, its cases in
  `extensions/github/`); a card says its row in its Details, looked at by
  a check run by hand (`ui/scripts/gitwork.mjs`; the client's
  `cardtabs.test.ts`). And GitHub's package keeps a card and an issue
  the same once the two are linked (`github/issues`, by a row of the project's `links`,
  or by a label on either side): a title or a body changed on one is
  changed on the other and never comes back, an issue closed moves its
  card to the last column and a card moved there closes its issue, what
  is said on the issue is said on the card, and what is said on the card
  stays here unless it is marked public; a change there by somebody with
  no standing in the repository is held until somebody here says which
  stands; a call that is not signed with the secret changes nothing, and
  what changed is read the same whether a signed call or a timer says so;
  and a daemon stopped as an issue is made or something is said on one
  makes or says it once (`cli`: `packages.rs`, its cases in
  `extensions/github/`). A timer reads first as it begins to listen and
  keeps what it read with the run a change sets off, so a change soon
  after it is applied is not missed (`conductor`: `handlers.rs`).
- **A project's environments.** A project says its own (`dev`,
  `staging`, `prod`), each with plain variables, made, changed and taken
  away as one write with what is said of it, by name and never by value;
  an agent is shown only the variables marked for agents, and changing
  one takes writing the project, `shell` and `secrets:use` (`api`:
  `environs.rs`; `environments`: `project.rs`; `cli`: `environed.rs`).
  Each has secrets of its own, sealed apart from the factory's and never
  shown or said, set, marked and taken away by whoever holds
  `environments:secrets` (`api`: `environs.rs`; `hosting`: `secrets.rs`).
  It goes with a project moved to a journal of its own (`api`:
  `moved.rs`; `runs`: `moved.rs`) and into a copy of the factory, its
  secrets listed there with no value (`api`: `snapshots.rs`), and with a
  project deleted for good, its forks with it (`api`: `environs.rs`).
  What is applied for a project with environments is applied under one,
  or refused naming them; it and what it starts read that environment,
  its programs are told it and handed its secrets in place of the
  factory's of the same name, what an agent applies only what is marked
  for agents, and an environment is not taken away while anything runs
  under it, nor with something applied under it in between (`api`:
  `under.rs`; `environments`: `held.rs`); stopped
  anywhere, what runs under one and what it starts stay under it
  (`conductor`: `plays.rs`); a device refuses a step handed one of
  them (`gateway`: `held.rs`). An agent works under the environment of
  what brings it up, or the one said as it is added by hand, and is
  handed of it only what is marked for agents, its slot's own keys
  looked for there first and never the factory's in place of one it does
  not hand on (`agents`: `keys.rs`; `api`: `under.rs`; `cli`:
  `environed.rs`). The client's page of them, the apply drawer asking
  which, and what runs saying which, with no secret's value in anything
  the page is handed or draws and an agent's credential shown only what
  is marked for agents, are looked at by a check that is run by hand
  (`ui/scripts/projectenvs.mjs`; the client's `projectenvs.test.ts`).
  What Run loads one as says once that it is ready (with where it is
  reached and the cards it holds), failed or taken down, a load a stop
  cut short said by the daemon that comes back, each load measured by
  project, environment and branch (`api`: `loaded.rs`; `environments`:
  `project.rs`).
- **A project's feature flags.** A row of its `flags` table each, what
  it says in each environment kept as that environment's `FLAG_<NAME>`
  and written with what is said of it as one write, the change a line of
  the row's history; set in one environment and no other, turning one on
  where the environment's rule names who approves waiting for one of
  them and turning one off never waiting, an approval landing only while
  nothing changed the flag or asked again meanwhile and writing nothing
  otherwise, so that it never undoes a later change, a key that holds less refused
  and asking as itself, a flag past its day told to whose it is once, and
  a percentage flag started as a comparison of two ways (`api`:
  `flags.rs`; `kernel`: `flags.rs`). Settings, Experiments says what of
  the program may be turned on before it is whole, none today (`api`:
  `experiments.rs`; `kernel`: `experiments.rs`). The client's table of
  them beside the environments, and each in an environment's drawer, are
  looked at by a check that is run by hand (`ui/scripts/flags.mjs`; the
  client's `flags.test.ts`).
- **A project's pipelines.** A project's own blueprints under
  `pipelines/` run as the rows of its `triggers` table say: by hand, as a
  card comes to a column, as a branch moves, so long after the last run
  and at a time of day; one asked for under a key another runs under
  waits for that one; somebody who asks by hand runs one only where they
  hold what it runs with, and each asking finds the run it began and
  never another, a daemon stopped between the two included; a row runs
  its pipeline with what it says, under the environment it says, held
  there to that environment's rules, and waits for a go-ahead where
  whoever wrote the row is not one the rule names (`api`:
  `pipelines.rs`, `pipelined.rs`; `deciding`: `triggered.rs`); one asked
  for by hand and stopped anywhere runs once (`api`: `triggering.rs`).
  Every run has a page of its own with an address, its stages in the
  order of its blueprint's graph, and a project's pipelines a page with
  Run and their latest runs, looked at by a check that is run by hand
  (`ui/scripts/pipelines.mjs`; the client's `staged.test.ts`,
  `pipelines.test.ts`).
- **Approvals.** A change of a record that an agent may not make is
  asked for (`mw ask`), told in the inbox of whoever could make it, and
  approved, made as the asker in one write with every check asked of
  whoever approved it, or declined (`mw approve`, `mw decline`); the
  agent is told where it asked, an agent's own row is never asked for,
  and one whose record moved on, or that waited seven days, makes
  nothing (`told`: `asks.rs`; `kernel`: `approvals.rs`; `api`:
  `asked.rs`, and `approving.rs`, stopped anywhere in the asking and the
  approving; `cli`: `asked.rs`). A run may wait for a person to let it go
  on (`approval.ask`): decided by those its environment's rule names
  where it names any, by whoever started it only where it names them,
  never by an agent; going on once approved, failing with why once
  declined, and expired once it has waited its time (`kernel`: `go.rs`;
  `api`: `goahead.rs`, and `goplayed.rs`, stopped anywhere). In the
  client each is a part of the inbox line's page with Approve and
  Decline, and a run that waits has them on its own page too, looked at
  by checks that are run by hand (`ui/scripts/asked.mjs`, `goahead.mjs`;
  the client's `asked.test.ts`, `goasked.test.ts`).
- **Comparing.** One blueprint started several ways, each thing kept to
  its way, and what came of it counted by the way (`api` and `cli`:
  `compared.rs`); a way held to an earlier version (`comparisons`:
  `compared.rs`); a way that reads other words where a file is linked
  (`api`: `prompted.rs`); a gradual rollout, a new setup's share of the
  cards grown by a rule of its user's while its cards are sent back no
  more often than the usual one's and held where they are, said each time
  with why, and stopped anywhere (`kernel`: `rolled.rs`; `comparisons`:
  `rolled.rs`; `api`: `rolledout.rs`, `rolling.rs`). The experiment of the first ten minutes, below,
  is run as it is written here, on the built-in assistant and with a
  stand-in agent (`cli`: `written.rs`, `researched.rs`; `api`:
  `examples.rs`).
- **Plain blueprints.** No expression in a blueprint that ships calls a
  function, does arithmetic or chooses: what is decided is a building
  block in Rust with a table of cases (the gate `plain`; `conductor`:
  `plainly.rs`). A list a blueprint keeps is changed an entry at a time
  (`keep`, `drop`; `conductor`: `state.rs`).
- **By project.** A handler or a `wait` that says no `scope` hears its
  own project's events and the factory's own, and every project's only
  by saying `scope: fleet`, stopped at every place and started again
  (`conductor`: `whose.rs`); everything said of a card taken to done, a
  feature planned and accepted, an agent answering in a project's
  channel and a comparison counting is that project's, and an apply
  warns of a handler that will hear nothing and goes ahead (`api`:
  `whose.rs`; `cli`: `heard.rs`). Where somebody has read to among the
  events is text handed out and taken back as it was (`api`:
  `followed.rs`).
- **In an editor of your own.** A file called `<name>.bp.yml` and the
  schema `mw schema` prints, which every blueprint that ships fits
  (`api`: `schema.rs`).
- **Inside an argument.** An action says what is written inside one of
  its arguments, each key and each of a key's few words with a
  sentence, and what the editor explains and offers there, `mw actions`,
  the schema and the notes' table are made from that; nothing is
  checked by it. What a table's declaration, a reading of its rows and
  a service's health check say is held, word for word, to the code that
  reads those words (`blueprint`: `inside.rs`; `studio`: `inside.rs`;
  `hosting`: `services.rs`; `api`: `inside.rs`, `contract.rs`; `cli`:
  `usage.rs`; the client's `within.test.ts`). In the client it is looked
  at by a check that is run by hand (`ui/scripts/within.mjs`).
- **The Reference.** Everything a blueprint can say is one page of the
  client, made from what the daemon describes and writing none of it:
  every name and every sentence the daemon hands is on the page's
  model, wherever it is, and what is typed finds what says it (the
  client's `reference.test.ts`, `tab.test.ts` and `within.test.ts`;
  `blueprint`: `vocabulary.rs`, for what goes after each word). Every
  word says what it is for and every word, and every action a person
  writes, is shown by a whole blueprint that reads, is laid out as the
  formatter lays it out and checks with no warning (`blueprint`:
  `shown.rs`; `api`: `exampled.rs`), and where the blueprints that ship
  use each is worked out from their texts (`blueprint`: `used.rs`). The
  page itself, its address and the ways to it are looked at by a check
  that is run by hand (`ui/scripts/reference.mjs`).
- **In plain words.** A blueprint is said for somebody who does not
  write them, by one function of its text and of what each action and
  event says of itself: what it is, makes, does, hears and may do, with
  a command it runs and a secret it reaches marked, the same for a text
  laid out another way (`blueprint`: `summary.rs`, with a case for each
  part; the client's `summary.test.ts`). Every blueprint that ships
  comes to one with no blank, no action's name and no value as its text
  writes it; it is handed to whoever may read blueprints and nobody
  else, for a text that is sent and for a blueprint by name, and making
  one says and changes nothing (`api`: `summarised.rs`). `mw explain`
  prints it, and says a file from its text alone where there is no
  daemon (`cli`: `explained.rs`). In the client it is a blueprint's
  Summary, looked at by a check that is run by hand
  (`ui/scripts/summary.mjs`). Its Preview plays each part of it on the
  blueprint's graph as the part is read, worked out from the text alone
  and written for no blueprint: steps in the order of what waits on
  what, a step's condition and a handler's event arriving, its
  condition and its steps, and each way it ends as what it is (by the
  word its summary gives each ending, `blueprint`: `summary.rs`), still
  where less movement is asked for (the
  client's `preview.test.ts`, held to the daemon's graph by a check run
  by hand, `ui/scripts/previewed.mjs`).
- **In a sandbox.** A blueprint is run in a factory of its own, made
  for the test and of nothing of the one in use, and what came of it is
  handed back as a real run is: its runs with their steps in the order
  they began, everything said in the order it was, its tables and
  channels, what a blueprint told somebody, and what was stood in with
  what each was asked and answered; what its steps write to their logs
  counts toward what it may keep
  (`sandbox`: `shapes.rs`, `kinds.rs`, `limits.rs`). Beside sandbox
  tests of every blueprint that ships, of the fleet working a card and
  of a text that asks every action there is, a factory in use is as it
  was: every row of every table of each of its journals, everything it
  said, every file in its folder. No command is run there, however it
  is said; a secret's value is in nothing a sandbox hands back; nothing
  of the factory can be named in one; and every action of a sandbox is
  the real one for a reason that is written down, or a stand-in (`api`:
  `sandboxed.rs`). A sandbox is its maker's, there are eight at once,
  and one that is ended, is asked nothing for a quarter of an hour or
  whose daemon stops leaves no thread and no file (`api`:
  `sandboxes.rs`; `sandbox`: `ends.rs`); its code names nothing it
  could hold of the factory in use (`sandbox`: `held.rs`). `mw sandbox`
  prints what happened, with the real program (`cli`: `sandboxed.rs`).
  In the client it is a view of a blueprint's page, whose model is
  held by its cases (`ui/src/sandbox.test.ts`) and whose page is looked
  at by a check that is run by hand (`ui/scripts/sandboxed.mjs`).
  Several texts of one blueprint (the default, its forks, the text as
  it stands and as it was saved) are played against the same acts, each
  in a sandbox of its own, and what each came to is laid out side by
  side as a comparison's columns are, with nothing of any of them in
  the factory (`api`: `sidebyside.rs`); a comparison on real work may
  then start those very texts (`comparisons`: `compared.rs`). In the
  client that is a view of the page (`ui/src/sidebyside.test.ts`,
  `ui/scripts/sidebyside.mjs`).
- **A tab shown again.** A tab that another was shown over is drawn
  again as it was left, from what it last read and read again behind:
  scrolled where it was, the same view of it, what was unfolded, and
  the drawer that was open beside it given back with what was typed in
  it, and put away where what it showed was taken away meanwhile; a
  closed tab keeps nothing (`ui/src/returned.test.ts`; looked at by a
  check that is run by hand, `ui/scripts/returned.mjs`).
- **Every page has an address**, a path with how it is shown after the
  `#` (`/shop/test-runs#newest`), which the daemon answers with the
  client wherever it has no route of its own, none of its own taken for
  one, and no project may be called what its own addresses begin with
  (`api`: `addresses.rs`; the client's `tab.test.ts`). In a browser one
  thing shows at a time, in place, and Back and Forward go between what
  was shown (the client's `inplace.test.ts`; `ui/scripts/inplace.mjs`,
  run by hand).
- **The program's own page**, opened from its mark at the foot of the
  client's rail, in three tabs (What is Mythwright, Docs, Technical):
  what it is for and the guides that came with it, to anybody signed in,
  read in place with nothing fetched; and what is technical of it (the
  version, the client's build, the system), the packages installed to
  whoever may read them, and where the factory keeps its data to whoever
  manages it and nobody else (`api`: `about.rs`; the client's
  `about.test.ts`; `ui/scripts/about.mjs`, run by hand).
- **Which runs which.** The blueprints that ship are listed as what
  somebody applies and, under each, its parts: which is which is worked
  out from the texts and from what each action says it reaches, by one
  function with its cases (`blueprint`: `names.rs`; `texts`:
  `listed.rs`), and nothing is renamed or moved. Every blueprint that
  ships, with what the packages bring, is one somebody applies or a
  part of one, and those applied are the ones meant, each said with
  why; the list of blueprints hands what each runs, and a part's
  summary says what it is a part of (`api`: `listed.rs`; `cli`:
  `explained.rs`). The client's list, its counts and what its box finds
  are the same functions, held with the daemon's to one table of cases
  (the client's `blueprintlist.test.ts`), and are looked at by a check
  that is run by hand (`ui/scripts/lists.mjs`).
- **Who may do what.** A credential does only what it holds, by each way
  in (`api`: `held.rs`, `access.rs`, `authority.rs`); a fleet's row gives
  an agent nothing that whoever wrote the row does not hold, and a stage
  gives an agent no work on the say of somebody who could not give it
  that work themselves (`api`: `scoped.rs`); an agent changes nothing
  of its own slot, by its route or by the row that brings it up, even
  within what it holds, and is told so in its brief with what it may do
  (`api`: `slotted.rs`; `agents`: `briefed.rs`); what a row of `segments`
  hands its blueprint beside its cells takes `shell` as a command does,
  and `secrets:use` as naming a secret does, and says nothing its stage
  says of itself, such as the secret a remote is asked with (`api`: `steering.rs`; `deciding`: `segments.rs`);
  a stage's own review or plan, named in its row, is run only as it
  was vouched for, so somebody who holds less names their own text
  there and it is not run (`api`: `steering.rs`); a
  cell that takes several scopes is given a value only by whoever holds
  every one, and is told which they lack (`studio`: `takes.rs`); a
  table's columns are added to, retyped and taken away only by whoever
  holds `columns` beside what changing the table takes, never of a
  column a blueprint declares, and a column somebody adds gives, guards
  and steers nothing: it is never what a fleet takes a row to say
  (`api`: `added.rs`, `steering.rs`; `studio`: `added.rs`, `rows.rs`;
  `kernel`: `added.rs`);
  a person is given a scope beyond what their role holds only by
  somebody who holds it and `admin`, holds it from their next request,
  and loses it, with the keys they made, when it is taken away (`people`:
  `given.rs`; `api`: `beyond.rs`; `cli`: `beyond.rs`; `kernel`:
  `scopes.rs`; in the client, `ui/scripts/given.mjs`, run by hand);
  a group of people and agents is made and changed only by whoever
  manages the factory and never by an agent that is in it, puts nobody
  in it nor gives it a scope that whoever does so does not hold, is held
  by its members from their next request, and taken from them, with the
  keys they made, when they leave it or it holds less (`people`:
  `groups.rs`; `api`: `grouped.rs`; in the client,
  `ui/scripts/peoplegroups.mjs`, run by hand);
  a key revoked (`mw key revoke`) is nobody's from then on, with every
  key made with it, and stops what each applied unless told to leave it,
  and a key rolled (`mw key roll`) is the same key with another secret,
  what it applied going on (`api`: `withdrawn.rs`; `cli`: `keys.rs`; in
  the client, `ui/scripts/keyrevoked.mjs`, run by hand);
  a table kept private is read
  by nobody else, and what is said on one of its records is heard by
  nobody else either, whatever it was marked, nor is anybody else told
  how many things were said on which, nor shown what a handler's run
  kept of a change to one, nor told what is said of that run as it
  goes: each line a step prints and why a step or the run failed
  (`api`: `private.rs`, `marked.rs`, `commented.rs`, `anchored.rs`, `told.rs`;
  `conductor`: `handlers.rs`). A channel may be kept to its members
  (`mw channel create --kept`, `mw channel keep`; Private in the
  client), and is then read by them and by nobody else, by every way
  the daemon hands a channel's words to somebody, each answered to
  anybody else as for a channel that is not there (`api`: `kept.rs`;
  `talk`: `kept.rs`). Its door is its members': it is made with who is
  in it and what it belongs to, an open one is kept by somebody in it
  with what its agents were asked there, somebody is added by a member
  and leaves by asking, the last person in it not, whether its agents
  are asked nowhere else is said afterwards too, only the factory's
  owner opens it again, and a blueprint that asks for a kept channel
  finds the one it made (`api`: `kept_door.rs`; `talk`: `door.rs`;
  `agents`: `door.rs`; `cli`: `kept.rs`). An agent in it is no way out
  of it:
  its own key reads the channel only while a turn asked there is in
  hand, across a restart too; what it is asked there, what it answers
  and what is said of its turn are its members' to read, and anybody
  else is shown when the turn was, how long it took and what it cost;
  a conversation held there is gone on with only by a step of somebody
  who reads it; behind its wall its program has a home of its own for
  each such channel, with its sign-in in every one; an agent with no
  wall is in none, and neither is one whose program keeps its sign-in
  where it keeps what it is asked (it signs in as whoever runs the
  daemon and its package does not say which of what it keeps is its
  sign-in, or it may be handed a folder), refused where it would be
  brought in and taken out, which is said there, where it comes to be
  so; a fleet whose worker is in one goes on working its cards; and a
  channel may say that its agents are asked nowhere else (`agents`:
  `kept.rs`, `homes.rs`; `api`: `kept_agents.rs`; `talk`: `kept.rs`).
  A channel may be archived (`mw channel archive`, `mw channel
  unarchive`; Archive in its row's menu in the client), by whoever may
  write in it: it is read, listed and searched by whoever read it as
  before, and stays private where it was, and nobody says anything
  there (a person, an agent, the bot and a blueprint's step are each
  refused in a sentence that says how it is given back, and a step that
  says `allow_failure` lets it be), no agent is asked there, and nothing
  of it is counted as unread, in the side panel or the inbox. It is
  refused while something running posts there or an agent is being
  asked something there, and given back it is as
  it was, in the groups and pins its people keep of it. Archiving and its
  saying are one write (`talk`: `archived.rs`; `agents`: `archived.rs`;
  `api`: `archived.rs`; `cli`: `archived.rs`; the client: `archived.test.ts`,
  `scripts/archived.mjs`).
  A channel is called something else (`mw channel rename`; Rename… at
  the top of its row's menu in the client) by whoever manages it:
  an open one by its name, refused where another has it and free once
  it is another's, a private one by what it is called, its id as it was,
  which only its members are told; what was said there, its search,
  its people's groups and pins, a copy and the Trash go on with it as it
  is called now. It is refused while it is archived, while something
  running finds it by its name, and while an agent is asked something
  in an open one; the rename and its saying are one write (`talk`:
  `renamed.rs`; `api`: `renamed.rs`; `cli`: `renamed.rs`; the client:
  `renamed.test.ts`, `scripts/renamed.mjs`).
  A channel's details (what it is called and what it says it is for,
  under its head) are the first part of its Channel tab in the Tools
  panel, changed by its managers (whoever made it, whoever a manager
  names one, every person in a conversation) and whoever manages the
  factory, and read as words by anybody else in it; the last manager
  stays, and a channel made before managers has none until whoever
  manages the factory names one (`talk`: `managed.rs`; `api`:
  `managed.rs`; `cli`: `renamed.rs`; the client: `managed.test.ts`,
  `scripts/details.mjs`).
  Whoever says something in a channel is among its members: a person
  comes into an open one by what they say there, in the write that says
  it; an agent is refused saying anything in one it is not in, in a
  sentence that says how it is brought in. A fleet brings each agent it
  brings up into the channel it reports in, asked there by its name, and
  it leaves as it stops; an agent asked in an open channel it is not in
  (named there by a person, or asked to say there what it did) is
  brought in by being asked, so that no answer is lost (`talk`:
  `members.rs`; `api`: `posting.rs`; the client: `scripts/reported.mjs`).
- How somebody arranged the side panel (their groups of channels, the
  channels pinned to the top and what is pinned in Resources) is kept
  with their account, so that it is the same in every window and
  browser they sign in to, and is theirs alone: read and changed only
  as themselves, said to nobody else, kept in a copy of the factory;
  the daemon keeps it and acts on none of it, and what a browser kept
  before is taken up once (`people`: `arranged.rs`; `api`:
  `sidekept.rs`; the client: `arranged.test.ts`, `scripts/followed.mjs`).
- How a project's things are arranged in the side panel (folders of the
  arrangement's own, with its board, tables, libraries and the rest put
  in them) is kept by the factory for everybody who reads the project,
  changed by whoever may write its files one change at a time, and read
  as each reader may see it: a table kept from them is not in what they
  are handed, and nothing they may not read is placed by them. It only
  says where a thing is shown: nothing moves on the disk and nothing a
  thing is known by changes. A project nobody arranged is drawn as it
  always was; it goes into a copy of the factory, into the Trash with its
  project and away with it for good (`kernel`: `outline.rs`; `outline`:
  `kept.rs`; `api`: `outlined.rs`; `cli`: `outlined.rs`; the client:
  `outline.test.ts`, `scripts/outlined.mjs`). It is arranged from a
  terminal (`mw project outline`), from the side panel's menus (a folder
  made, called something else, removed with Undo, and things moved in and
  out of one by Move to…, and a thing renamed for everybody by Rename…,
  its key and what blueprints call it unchanged;
  `scripts/arranged-tree.mjs`), and by a
  blueprint, which shows a thing in a folder of its path as the same done
  twice (`outline.put`; `api`: `outlined.rs`); by dragging is not built
  yet.
- A project, and a table, may be called something else for people by
  whoever may write the project's files, from its row and its page's
  gear, from a terminal (`mw project rename`, `mw record title`) or a
  route, each said to every page in the write that keeps it; what
  everything else knows it by (a project's id, a table's name) never
  changes. A table's blueprint
  may say what it is called (`title`), and what somebody calls it stands
  over that when it is declared again (`studio`: `shown.rs`; `api`:
  `retitled.rs`; `cli`: `outlined.rs`; the client: `called.test.ts`,
  `scripts/arranged-tree.mjs`).
- How a time is written for somebody is kept with their account the
  same way: the zone (`America/Denver`) and a clock of twelve hours or
  twenty-four, twelve with AM or PM where nothing is said. Every time
  the client draws is in it, with the zone's short name as it is on
  that date (`10:07 PM MDT`, `10:07 PM MST`); the zone the browser says
  it is in is taken up the first time it is read with nothing kept, and
  is changed in Settings, where a page never read again follows it
  (`people`: `clocked.rs`; `api`: `clocked.rs`; the client:
  `clock.test.ts`, `scripts/clocked.mjs`).
- The themes somebody makes for the client are kept with their account
  the same way: eight colours, with a light version where they say one,
  held to being read on their ground by one rule in the daemon and the
  client, and a look beyond the colours (the letters, the size of text,
  how close things stand, how round corners are); theirs alone to read
  and change, chosen in any browser they sign in to, offered to
  everybody by whoever manages the factory, saved as a file and read
  back in, and kept in a copy of the factory (`people`: `themes.rs`;
  `api`: `themed.rs`; the client: `themes.test.ts`, `looks.test.ts`,
  `themefile.test.ts`, `picking.test.ts`, `scripts/themed.mjs`).
- **An inbox for everybody with an account**: a box of what was
  addressed to them, which only they read (`mw inbox`; a place of the
  client's rail, under Resources, with how many wait on it). A message that names a person with
  `@`, by the word their account is named by, puts a line in their box
  in the write that keeps the message, where they read its channel: in
  an open channel anybody who may read channels, in one kept to its
  members somebody who is of it, and never whoever said it. A line keeps
  no words: it is shown with the message as its owner may read it then,
  and one of a channel that has been kept from them since is not
  listed, not counted and not found, as the channel is not, until they
  read the channel again. It is read, read by its place (a channel that
  comes into view in the client has read the lines that open it), read
  all at once and put away by its owner; nobody else reads, hears or
  counts it by any route, stream or command, the factory's owner
  included, and a line that is somebody else's is refused as one that
  is not there. A key reads the box of whoever made it only where the
  key was given `inbox`. A line goes with the channel it opens and a
  box with an agent that is taken away; a copy of a factory keeps them,
  and a project moved to a journal of its own takes none. An agent the
  factory does not start is told the same way where it may read the
  channel, and reads its box as itself (`mw inbox --as <name>`).
  Whoever has a box follows it from a terminal (`mw inbox --follow`,
  `--json` for a program): what waits unread, then each line as it is
  told, once, and once more when it is told again, from what the
  daemon says of the box to its owner alone and never by looking on a
  timer, going on from where it was once a daemon that was lost is back
  (`talk`: `inbox.rs`; `people`: `handle.rs`; `api`: `inbox.rs`,
  `whole.rs`; `agents`: `external.rs`; `cli`: `inbox.rs`, `followed.rs`;
  the client: `inbox.test.ts`, `mentions.test.ts`, `scripts/inbox.mjs`).
- **Somebody named on a card, or any record**: what is said on a record
  that names a person with `@`, by the same word a channel goes by, or
  an agent the factory does not start by its name, puts a line in their
  box, by every door that says something on a record (the route,
  `mw record comment`, a blueprint's `record.comment`), where they may
  read the record as things stand and never whoever said it; an agent
  the factory starts is given nothing. It is told once however often the
  comment is heard, keeps no words and is shown with them from the
  record's history as its owner may read the record then, is passed
  over once they may not, and opens the record at that comment: a card
  on its board with its drawer, a row of another table in its form. A
  record's form, a card's among them, has one row of tabs at its top
  (Details, Comments, History, Attachments), and the box under Comments
  offers who can be named as an `@` is typed (`talk`: `commented.rs`; `api`: `tagged.rs`; the client:
  `inbox.test.ts`, `mentions.test.ts`, `scripts/tagged.mjs`).
- **A discussion between a channel's agents** (`std/discuss`, applied
  for the channel, or Discussions… under the Channel tab of the Tools panel beside it): an agent naming
  another that takes part, an agent saying its answer is a plan, or a
  person asking two or more of them to discuss something (each a
  setting) has them talk it over in a thread, a turn at a time and at
  most so many turns a stretch, until it is agreed or needs a person,
  who is told in their inbox with a line that opens where it ended; the
  person's answer in the thread lets it go on, and they may end one
  going or waiting for them without answering (End discussion on its
  thread, `mw channel end`), which calls off the turn in hand and is
  said to be theirs. With a lead, a person
  talks to the lead alone, which consults its advisors in a thread and
  answers the person with what was agreed. What a message is (a plan,
  something to discuss) is a mark it is posted with, never read from its
  words; whose turn it is and how it ends are `kernel::discussed`, with
  their cases. Played with stand-ins agreeing, needing a person, at the
  cap, with a lead, ended by a person and across a restart (`api`: `discussed.rs`; the
  simulation's `discussing.rs`, stopped at every place of a discussion;
  `kernel`: `discussed.rs`).
- **What runs in a channel, set from the channel**: under the Channel
  tab of the Tools panel, each thing applied whose `channel` names it (a
  discussion, a bot, a project's own status updates), its params a form
  made from its blueprint and applied again, and stopped behind a
  question, for whoever may apply, and told to whoever reads the channel
  and what is applied (`api`: `running.rs`; the client:
  `runsin.test.ts`, `scripts/runsin.mjs`).
- **A conversation**: a message written to one or several people and
  agents (`mw to`; the pencil at the head of the client's inbox), which
  is in the inbox of each of them and of nobody else. It is a channel
  kept to its members that says it is a conversation, so threads, what
  is attached, search and an agent's answer are a channel's; what is its
  own is that it is written as one write (the channel, who is in it, its
  first message and the line in the box of everybody it is to, or none
  of it, so that what the daemon refuses leaves nothing made), that it
  is one line of each member's box which each thing somebody else says
  there raises (never a second, and none for being named there; somebody
  added to it has its line from then), and that it is in no list of
  channels, from the daemon, a terminal or the
  side panel. Somebody who is not in it finds nothing of it by any
  route, list, search, stream or command, in the words a channel that
  is not there is refused in, but the factory's owner, who reads it as
  every channel kept to its members and has no line of it; one who
  leaves has their line passed over. An agent the factory starts is in
  it as in any kept channel, asked first and refused with nothing made
  where it may be in none, and is given a turn by what is said to it;
  one it does not start has it in its box, reads it and answers with
  its own credential. One of its members opens it as a channel, with
  everything said so far, which it then is for good; a copy of a
  factory keeps it and its lines, and one taken away takes its lines
  (`talk`: `conversed.rs`; `agents`: `external.rs`; `api`:
  `conversed.rs`; `cli`: `conversed.rs`; the client:
  `conversed.test.ts`, `inbox.test.ts`, `scripts/conversed.mjs`).
- **A forum's topics** (`mw topic`): a topic is a record of a table of
  the factory's own, a workspace's or a project's, set up as whoever
  manages the factory makes the first; its replies nest under what they
  answer, it is changed at the door any record is changed by with a
  revision kept each time what it says changes, it is marked answered by
  a reply or closed, and an agent named in a reply is given the topic and the
  chain it answers and answers under that reply, once wherever the
  daemon went (`topics`: `kept.rs`, `context.rs`; `kernel`: `topics.rs`;
  `api`: `topics.rs`; `cli`: `topics.rs`). In the client they are a
  place of the rail of their own, Topics, just after Channels, and a
  topic is a page of its own with an address, looked at by checks that
  are run by hand (`ui/scripts/topics.mjs`, `topicpage.mjs`; the
  client's `topicpage.test.ts`, `replies.test.ts`).
- **Everybody here, under Factory.** The Members line under Factory in
  the client's Resources panel lists every person with an account and
  the agents of the workspace the panel is of, each pressed for what can
  be done with them: read about, a direct message (a conversation of the
  two), and for whoever may, what they hold, or the lines an agent's menu
  has on the Agents page; looked at by a check that is run by hand
  (`ui/scripts/members.mjs`; the client's `direct.test.ts`).
- **A blueprint tells somebody, and a fleet tells whoever applied it of
  what waits for a person.** A step puts a line in the inbox of whoever
  it names (`inbox.tell`: whoever applied the text, an actor, a handle,
  or a list of those), in the blueprint's own words and from the
  factory's bot, taking `inbox:write` of whoever applied it. The same
  step done again where the daemon stopped tells nothing twice; told
  again of the same thing for the same reason, the line there is is
  raised and says the latest; a word that names nobody tells nobody.
  A fleet attaches a text that does this (`std/told`) for a question a
  card's agent or a feature's planner asked, a card left for a person
  (by its round, by those who looked at it, or by what else its stage
  does: each is said as `card.left` with why), a feature's or a card's
  plan that waits, and what fails in the fleet. Whom is the rows of a table of the project
  beside its board (`tells`), which starts with whoever applied the
  fleet; a row emptied or taken away tells nobody, and a card that
  waits for nobody has nothing of it run. A fleet from before tells
  nobody until it is applied again taking what ships, and one applied
  by somebody who may tell nobody says so once where it reports. In
  the client a told line says its words whole when pressed and shows
  the card, the row, the channel or what is running that it is about
  (`talk`: `telling.rs`, `door.rs`; `deciding`: `told.rs`; `api`:
  `tells.rs`, `telling.rs`, `featured.rs`; `cli`: `tells.rs`; the
  client: `inbox.test.ts`, `scripts/told.mjs`).
- **A told line's parts, and a card passed or failed from it.** What a
  step tells may show parts beside its words (`parts` of `inbox.tell`):
  addresses, and the cards of a board in one column or with one label,
  each with what was said on it since it came there; a part of a column
  keeps the cards it held as the line was told and lists each where it
  is now, or that it is gone. Whoever reads the
  line and may change the board passes or fails each, on the line's page
  or from a terminal (`mw inbox parts|pass|fail`): failed, with why unless the part says otherwise, it
  goes to `ready` or the column the part says, why being what was said
  as it was moved, which its agent is told when it is next given it;
  passed, to the column the part says, or it stays and that it passed is
  said on it. An agent does not pass or fail its own work. An agent
  given a card again is told what others said on it since it last had
  it (`record.said`, asked by `std/work`) (`kernel`: `parts.rs`; `talk`:
  `telling.rs`; `api`: `reviewed.rs`; `cli`: `inbox.rs`; the client:
  `reviewed.test.ts`, `scripts/reviewed.mjs`).
- **A button on a card, and a card held until it is pressed.** A row of
  a fleet's `actions` (a table beside the board, about a column, which
  starts empty) is something pressed on a card in its column, by anyone
  who may change cards, by a person only, or by whoever holds a scope
  its row names; one that holds keeps a card there until it is pressed,
  for good or for each time the card comes. Pressing is one door, as
  whoever presses: the route, `mw board press`, the factory's bot and a
  blueprint's step (`board.press`), the press and what is said of it
  one write on the card's row. A held card asks for no place and has
  nothing started, says once each time it comes that it waits and who
  may press, is told of where the fleet tells what waits for a person,
  and is taken up as soon as the last of it is pressed; held, it is
  held still when the daemon is back, and a press it went away just
  after is found. With no row, a card goes as it always did, and costs
  what it did (`kernel`: `pressed.rs`; `deciding`: `pressed.rs`,
  `work.rs`, `reviews.rs`, `told.rs`; `studio`: `models.rs`; `api`:
  `pressed.rs`, `pressing.rs`, `staged.rs`, `costed.rs`; `cli`:
  `pressed.rs`). In the client a card says on the board what it waits
  for, and opened, each thing its column offers as a button for whoever
  may press it, or as pressed, by whom and when, following each press
  with nothing read again (`cardactions.test.ts`, `scripts/pressed.mjs`).
- **A mistake is answered with where and what was meant**, for the ones
  made on a first day (`blueprint`: `check.rs`, `placed.rs`; `api`:
  `refused.rs`).
- **What is attached to a card, or any record** (a picture, a video, a
  log): added by whoever may change the record, by the route, `mw record
  attach` and a blueprint's `record.attach`, listed and read by whoever
  may read it and by nobody else where its table is kept private; taken
  away into the Trash and given back, kept with its record while that is
  in the Trash, and gone with its record, its file, its folder or its
  project for good. An agent working a card is told what is attached to
  it by name and size, with the command that reads it, and never what is
  in it (`studio`: `attaching.rs`; `api`: `recordfiles.rs`; `cli`:
  `records.rs`; `agents`: `docsed.rs`). In the client it is a card's
  Attachments tab, and a row's, where a file is dropped or a picture
  pasted, looked at by a check that is run by hand
  (`ui/scripts/attached.mjs`; the client's `recordfiles.test.ts`).
- **What is deleted goes to the Trash**: a row of a table or a card, a
  file or a folder of a project, a project forgotten, and a channel with
  everything said and attached in it, each left out
  of everything that lists or reads what it was of, kept with what was
  said on it, given back where it was (a card in its column and its
  place, or at the bottom where that is taken; a file at its path, or
  beside what is there now when asked, what was said on it going with
  it; a project in its workspace; a channel whole, a private one kept
  to the same members and seen and told of in the Trash by them alone),
  its history saying it was given back, by
  whoever could have deleted it, deleted for good by them, and emptied
  on the factory's clock after the days its owner says, thirty unless
  said; seen only by whoever could read it; a stop in the middle of
  moving a file finished as the daemon starts; and kept by a copy of the
  factory (`kernel`: `trash.rs`; `studio`: `trashed.rs`, `apart.rs`,
  `moved.rs`; `trash`: `trashed.rs`, `channels.rs`; `api`: `trashed.rs`,
  `removed.rs`, `whose.rs`;
  `cli`: `first.rs`, `trashed.rs`; the client: `trash.test.ts`,
  `scripts/trashed.mjs`). A project deleted for good takes its tables,
  their records and what was said of each, what of it was in the Trash,
  what was applied for it and still ran, and its agents with it, so that
  one made again under its name finds none of it, and only for somebody
  who could stop each and take each away by hand (`api`: `forgotten.rs`).
- **An agent taken away for good**, by a person who manages the factory
  and by nobody else, its own credential included: refused for one a
  fleet counts and for one with work in hand, each saying what to do;
  gone with its slot, its credential, its place in every channel and its
  folder, while what it said and did stays; and a later agent of its
  name no heir of it, in a conversation, in what it was given or in what
  is listed as its work (`agents`: `removed.rs`; `api`: `away.rs`; `cli`:
  `away.rs`).
- **A role's agents told a file of the project's own words first**, as a
  fleet's `agents` row names it (`prompt`), read as it was vouched for: a
  change written through the factory taken up, one written outside it
  not, and one the project has none of refused saying where it is
  written (`api`: `ownprompt.rs`; `conductor`: `linked.rs`).
- **An agent moved to another project**, or to none, from its line on
  the Agents page or with `mw agent move`, by whoever may change agents
  and holds what it holds: nothing else of it changes (one stopped stays
  stopped, it holds what it held and keeps its credential), its next
  turn is shown the new project's folder behind its wall and nothing of
  the one it left, and it is refused, saying what to do, while it has
  work in hand and where a fleet counts it, whose agents are changed in
  their row; one the factory does not start moves the same way
  (`agents`: `walls.rs`; `api`: `relocated.rs`).
- **A project's files from a terminal**: `mw file write` from standard
  input or a file, byte for byte (a picture arrives as it was), `mw file
  read` to standard output as it is, and `mw file remove` into the
  Trash, each as whoever is signed in or an agent by `--as`, refused to
  a key that holds less and, for a file a table keeps private, in the
  route's own words; and a file larger than is written as it is refused
  as too large with nothing written (`cli`: `filed.rs`; `api`:
  `sent.rs`).
- **An agent the factory does not start** (a program somebody runs for
  themselves, with this command) has a slot and a name of its own: it
  is invited by whoever may add agents, who give it nothing they do not
  hold; it joins with its invitation, once and within the time said, at
  a door that takes no session and answers a wrong, a used and an
  expired invitation alike; its token acts as `agent:<name>`, so a card
  it takes, moves or speaks on says so; what it holds is changed where
  any agent's is and held to at its next request; stopping it, taking
  it away or its own leaving withdraws the token, and a copy of a
  factory keeps the agent and neither its token nor its invitation.
  Nothing is started for it and nothing gives it work: not a role, a
  step that names it, a fleet in its project or a message, which is put
  in its inbox for it to find. In a channel kept to its members it is
  as a person is: brought in with nothing asked of a wall, it reads the
  channel it is in with its own token, and no other. Invited with
  nothing said, it holds what an agent usually holds and its own inbox. Its token is kept by the
  command line in a file only its user reads, apart from any person's
  session, and chosen by name (`mw --as <name>`) (`agents`:
  `external.rs`; `api`: `external.rs`; `cli`: `external.rs`;
  `deciding`: `work.rs`).
- **An agent's program** is started as its package says for each way it
  can be held, is told that the factory's own command is not one to ask
  leave for, and uses the web unasked only where its row says so
  (`agents`: `extensions.rs`).
- **What a new project starts with**: a board, a fleet and a library,
  each left out by name, or none, and features where they are asked for,
  as it is added or afterwards; its environments only where they are
  asked for, each empty, with nothing else it starts with run under one;
  and a project says whose its folder is, one that was said for it or
  one the factory made (`api`: `usual.rs`; `cli`: `usage.rs`). Setting
  a project up is a blueprint, `std/project`, applied as whoever adds
  it, whose rows are what it starts with: a row of somebody's own
  blueprint is added beside them, the rows are saved as a setup with a
  name, and a setup is made the factory's default for new projects, each
  by whoever may (`api`: `setups.rs`; `deciding`: `setups.rs`). In the client the
  dialog that adds a project is a table of those rows, each with a
  switch and what it is set up with folded under it, made in the
  workspace the side panel is of, looked at by a check that is run by
  hand (`ui/scripts/settingup.mjs`; the client's `startset.test.ts`,
  `starts.test.ts`).
- **Workspaces**: projects and channels gathered under a name, which
  changes nothing of them, lets nobody in and keeps nobody out, and is
  taken into a copy. A factory always has one, made with its owner as
  its first start makes them, called Unnamed workspace until they say what
  they are called and after them from then on, and every project is in exactly
  one: the usual one unless whoever adds it says another, put in
  another and never taken out of all, moved with a workspace that is
  taken away, the last of which stays. A factory an earlier version
  made, where a project might be in none, is brought to that the first
  time this one starts on it, with nothing else of any project changed.
  A project's place and what is said of it are one write, whichever
  journal keeps the project, and a workspace's own channels are moved
  with it only by somebody who could have moved each by itself. Every
  channel is a project's or a workspace's own, and none belongs to
  nothing: one made for no project is the own of the workspace that is
  said, or of the usual one, in the write that makes it; one is said to
  be another's and never nobody's; a project that is forgotten leaves
  its channels to its workspace; and a factory an earlier version made
  has each channel that belonged to nothing made the usual workspace's
  own the first time this one starts on it, said once
  (`studio`: `workspaces.rs`; `api`: `spaces.rs`, `settled.rs`,
  `snapshots.rs`; `cli`: `spaces.rs`). A card is given to an agent of
  its own workspace or to one that is no project's: an agent of another
  workspace's project is refused by whichever way the card is given,
  and the card's drawer offers none (`api`: `assigned.rs`).
- **Cards put in order together.** A feature gathers cards that are on
  the board already, by their ids or by a label; an agent's plan says
  which wait on which and what else is needed; a person accepts it, and
  the cards are worked in that order, those that wait on nothing of each
  other side by side. A plan that does not fit the board is refused
  whole, an agent's moving it on accepts nothing, and one being taken up
  when the daemon stops comes to the same cards once (`api`:
  `featured.rs`; `studio`: `gathered.rs`). The step of the first run
  that does it is run as it is written, with a stand-in that plans
  (`cli`: `written.rs`).
- **The notes of each release** come with the program, as one file
  ([docs/releases.md](releases.md)), and are read from the daemon
  by anybody signed in with nothing fetched from anywhere; the newest
  release they tell of is the program's own version (`api`:
  `releases.rs`).
- **One daemon for a factory.** A second `mw up` beside one whose
  daemon is up says where the factory is and the command that stops
  it, and ends; beside one whose daemon is not up it waits; and an
  address something else has is said to be something else's, with
  nothing made for the factory (`cli`: `beside.rs`; `api`:
  `folders.rs`). A port a service is given is nobody else's from before
  its program listens until the service is looked after no longer, a
  daemon that comes back speaking for it again (`hosting`: `ports.rs`;
  `api`: `given_back.rs`; `cli`: `daemon.rs`).
- **The first things anybody types.** `mw help up`, `mw help open` and
  `mw help register` each say what the command does; `mw open` says
  where the client is whether or not a browser comes up; a second
  device joins with an invitation read from the terminal, and one
  with no account yet is told what to do first; forgetting a project
  says what still runs for it and what stops each; what an agent
  holds is said whole from the terminal (`mw agent add <name>
  --scope`); and an agent on the built-in assistant, given work, says
  the one command that gives it a program (`cli`: `first.rs`,
  `usage.rs`, `allowed.rs`; `agents`: `real.rs`).
- **Nothing to register to begin.** A factory's first start makes its
  owner, called Unnamed until they say what they are called (nothing of
  the machine or of its user is taken for a name), with no email and no password,
  before it answers anybody, and signs the terminal that started it in;
  `mw open` asks for a link that signs a browser in once and within a
  minute, as whoever the terminal is signed in as (their session, or a
  key that holds all they hold; a key that holds less is refused), or as
  the owner where the terminal proves it can read the factory's data
  folder (`mw login --here`). The address alone says to run it, with a
  sign-in for an account that has a password, which the owner sets on
  their Profile, or with `mw register` from any terminal on the machine
  (which proves it can read the data folder, and names them, and their
  workspace with them); a request with nothing to show is given nothing, a link
  spent or past its minute signs nobody in, and registering from the
  machine makes nobody the owner (`api`: `opened.rs`; `cli`:
  `opened.rs`, `profiles.rs`; `desktop`: `asked.rs`; the client's
  `firstrun.mjs`, `walkthrough.mjs`).
- **The program writes out what it says of itself.** `mw docs export
  <folder>`, with no daemon, and a blueprint's `docs.export`, into a
  folder of its project, write the same pages of Markdown, each saying
  `public: true` and the version: every command and topic of the help,
  every keyword, action and event, each blueprint that ships, the release
  notes and the guides that say they are public, with an index of all of
  it, and replace a folder only where an export wrote what is in it.
  Which pages there are is kept (`contract/docs.txt`). `mw help --json`
  says every command, its flags and every topic, held to the words the
  program answers and the flags each command reads, and every command a
  public guide writes out is one the command line reads (`cli`:
  `documented.rs`; `api`: `documented.rs`).
- **More sent than a route takes** is read to its end before it is
  refused, so that whoever sent it reads the refusal (`413`) and the
  connection goes on, as far as four times what the route takes
  (`api`: `sent.rs`).
- **The client's own logic**: its types, its tests and its build (the
  gate `ui`), among them who is in a channel as one list, what somebody
  holds by area, what a new project's dialog offers, what the Setup
  Guide says is next once its steps are done (`ui/src/next.test.ts`),
  what a table written out as a CSV file and read back comes to
  (`ui/src/csv.test.ts`), and what Help at
  the foot of the side panel comes to: where support is asked for
  (nowhere, until whoever builds the product writes an address in
  `brand.toml`), what is copied to say in a report, and whether a
  release is new to a browser (`ui/src/help.test.ts`). What it looks
  like is not held by the gate.
- **The desktop window's own logic** (where the daemon is and whether
  it needs starting, and what the window and the client say to each
  other) is not in `cargo xtask check`, which skips that crate: the
  hosted run, which is asked for after a batch of changes and runs for
  a pull request, builds it and runs its suite (`desktop/tests/suite`:
  `main.rs`, `window.rs`, and `asked.rs` for what it is told on its
  command line).

What an event's number says is narrow, and is all that will stay true as
a factory comes to keep each project's events apart: no other event in
the factory has it, and of two events of one project the later has the
greater. There are gaps (one at every start today: numbers are drawn a
run at a time from one giver for the factory, and what is left of a run
is never given; `journal`: `given.rs`), and the numbers of two projects'
events say nothing of their order. What is kept for what runs already says
whose it is, project by project, in the one file it is all kept in, and a
handler's place says which journal it is a place in (`runs`: `apart.rs`,
`places.rs`, `before.rs`; `conductor`: `costs.rs`). A project that is
added has a journal of its own beside the factory's, a file with a
thread of its own, which a copy of the factory takes with it, every
file as it was at one moment whatever was being written
(`journal`: `beside.rs`, `drawn.rs`, `moment.rs`; `api`: `journals.rs`,
`moment.rs`, `given_back.rs`). What is run for such a project, and what
is said of it, are kept in that file by a conductor of the project's own, which
the factory's answers for, so that a project with a great deal running
does not hold another up (`api`:
`apart.rs`; the runner's plays are played for a project with a journal
of its own where that is asked for, `conductor`: `anywhere.rs` by
`cargo xtask ways own`, which no gate runs).
What that conductor keeps beside its rows (which texts its trees are
held to, what was vouched for them, what a handler last read on a timer)
is in the project's file too, and what a program from before kept in the
factory's for it is brought over once (`conductor`: `beside.rs`; `api`:
`apart.rs`; `comparisons`: `compared.rs`). A
project of a factory from before stays in the factory's file until its
owner has it moved apart (`mw apart`, asked of a factory that is
stopped): what is kept of what was run for it goes to a file of its own,
at rest, with what is kept beside it (its trees go on held to the texts
they had reached and to its files as they were vouched for), what was
said of it, its tables with their records and each record's history,
and what its agents were asked, in the order they were asked; it is
taken up there when the factory is started, every card where it was;
cut off, it is as it was or it is moved; and a project whose journal an
earlier program made keeping less is taken the rest of the way by the
same asking (`runs`: `moved.rs`; `studio`: `moved.rs`; `agents`:
`moved.rs`; `api`: `moved.rs`, `whole.rs`; `journal`: `every.rs`). What one conductor asks of
another waits until that one has taken up what it keeps, and one can be
stopped and started while the others run, though nothing asks for that
yet (`conductor`: `alone.rs`; `comparisons`: `compared.rs`). An event
an instance of one project says naming another is kept in that other's
journal and said once wherever the daemon is stopped (`conductor`:
`elsewhere.rs`). What a project's service and router say of themselves
is kept with what else is said of the project, in one order, and what is
only passed on goes out with what is kept of the same thing (`hosting`:
`services.rs`; `journal`: `every.rs`). A project
made now keeps its tables, their records (a board's cards among them),
each record's history and what a scan last saw of its files in that file
too, a card's row and what is said of it as one write there; those of
a project from before stay in the factory's file until it is moved,
read and written where they are, and a copy of a factory takes both
(`api`: `journals.rs`, `whose.rs`, `late.rs`, `snapshots.rs`; `studio`:
`apart.rs`; `journal`: `kept.rs`). What its agents were asked is kept
there as well, a turn and what is said of it as one write, in the
order asked whichever file keeps each, so also for an agent that is
nobody's or was given to another project (`agents`: `filed.rs`,
`apart.rs`, `together.rs`; `api`: `whose.rs`, `restarted.rs`). What reads or follows the events for
the whole factory (the stream, what was said of late, search, a
record's history, an agent asked in a channel) reads every journal and
keeps a place in each, and a place handed outside says so and is still
text to hand back (`journal`: `every.rs`; `api`: `journals.rs`). A
handler and a `wait` hear from their own project's journal and the
factory's, or from every one, with a place in each, a journal made
while a handler listens or has something in hand among them
(`conductor`: `several.rs`). An id is a name and nothing is read out of one,
a record's excepted (`shop-12`). How the data folder is laid out is the
program's own. [docs/blueprints.md](blueprints.md) (*Events*) and
[docs/architecture.md](architecture.md) (*A journal for each
project*) say the whole of it.

- **Logs kept for so many days, and written where the owner says.**
  Unsaid, everything is kept for ever, in the data folder. Said as a
  number of days (`mw logs --keep 30`, and in Settings), what runs'
  steps printed and what the runner said of each step as it went are
  let go of once they are older, at once, as the daemon starts and once
  an hour; what each run came to stays, with records, their history,
  channels and measurements, and a run whose detail went says so. Nothing
  goes that something alive has not been handed yet, nothing that a
  comparison counts, and nothing of a run under way. Said as a folder
  (`mw logs --folder DIR`), runs' steps write there from then on, and
  what was written before is read where it is; a copy of the factory
  takes how long and not where (`journal`: `letgo.rs`; `api`:
  `letgo.rs`; `cli`: `kept_logs.rs`; the client: `logskept.test.ts`,
  `gave.test.ts`, `scripts/logs.mjs`).

Nothing is promised yet about carrying data, or blueprints you write, from
one version to the next. What a blueprint can depend on is written down in
[contract/offered.txt](../contract/offered.txt), and every time that is broken
it is recorded, with the reason, in [contract/broken.txt](../contract/broken.txt).
The steps each database's tables are made by are held to what they were
the same way ([contract/steps.txt](../contract/steps.txt)): one is only ever
added after the last, and a factory handed a program whose steps are not
the ones it ran is refused by its own database before anything is changed
(`api`: `steps.rs`; `journal`: `steps.rs`).

## What is not held yet

Known, and said so that nobody finds out by relying on it. Each is a
sentence here; [docs/architecture.md](architecture.md) has the longer
form (*Stopped anywhere, and started again*, and at the end of *Identity*).

**Restarts**

- **A discussion between a channel's agents** takes no agent from
  outside the factory, and cannot: such an agent has no turns, which is
  what a discussion is made of (it is told what names it in its inbox
  and answers when its own program chooses), so it is no party to one.
  What a person says in a discussion's thread while its agents are
  still going is read by the next turn and lets nothing else happen. A
  lead's answer to the person is opened, from the person's inbox, at
  what the person asked, beside which it is said.
- What keeps a daemon running (`mw up`) is known to its factory's folder
  from its start where a daemon has had the folder before, and
  otherwise from the first time its daemon ends: while the first daemon
  of a new factory runs, a second `mw up` is refused by the daemon it
  starts, two seconds later, and not at once. A second `mw up` beside
  one whose daemon is not up waits for as long as that goes on, saying
  so: it ends when that daemon is up and goes on when the other has
  ended, and a factory stopped on purpose in the instant the other's
  daemon came up is started by it. And a `mw up` of a program from
  before this knows none of it: it goes on starting a daemon that is
  refused, as it did.
- A daemon draining to stop (`mw down --drain`) waits for every run under
  way, and one that waits for something nothing will say while it drains
  (a `wait` for an event a handler would have said, or for a long time)
  holds it to its bound: it then stops, saying so, and the next start
  takes that run up as any. A daemon stopped by a signal, or by `mw up`
  being stopped, stops at once and does not drain.
- A factory keeps what the steps of its tables were from the first time
  this program starts it. Until then it keeps only how many it ran, as
  every factory an earlier program made does, and that first time it
  has nothing to hold a program's steps against: what holds them then is
  the gate alone.
- The factory's file is not made smaller by a project being moved to
  a journal of its own (`mw apart`): `mw repack` does that. A line of
  a record's history whose record nothing says the project of stays in
  the factory's file, and is read there.
- A service rolled out through a slot came back with two services where
  one was meant, once, on a hosted machine: the daemon stopped as the
  step that announces the new version began, and started again. It has
  not happened again in over a thousand plays of that place here, and
  what its daemon said was not kept, so it is not explained; a hosted
  run that fails keeps what its plays wrote down since.
- A service's program is started a moment before the daemon has kept
  that it started it. A daemon killed in that moment leaves the program
  running with nothing to find it by, and starts another when the step
  is done again. The simulation plays the place (a rollout stopped as
  its `service.start` begins), and holds what the daemon then knows; the
  program left behind is ended by the test's rig, and by nothing where
  there is no rig.
- A router that was given its address by the system (`listen: 0`), and
  a factory with a name that was (`--profile` and no `--listen`), keep the
  port while they run and ask for the same one when the daemon is
  started again: while the daemon is stopped it is the system's to give
  to anything, and a router that finds its port taken says so
  (`router.failed`) and does not come back. A service's own
  port is no longer the system's to give
  ([docs/architecture.md](architecture.md), *A port a service is
  given is nobody else's*): it was, and a service was started again
  with nothing wrong with it, once in a gate and once in ninety runs of
  the example's suite.
- A service's health check that connects (`tcp`, `http`) is answered by
  whatever listens on the port. A service told a port that something
  else has (`port: 8080`) cannot listen, ends and is started again as
  often as it may be, and is said to be healthy meanwhile, and sent what
  a router has for it, because the other program answers there.
- A package's blueprint that keeps cards in step with a service's
  tickets (GitHub's issues, Jira's, a Notion database's pages) adds a
  card for a ticket by a step that is the same done twice, so a daemon
  stopped as that step begins has one card for the ticket when it is
  back, and something said on a ticket is said on its card the same way.
  A link found saying a card was being made for it and naming none is
  left for a person and given no second card, since one may have been
  added. What goes out is found again by a mark, and is made or said
  once wherever the daemon was stopped; but Jira's and Notion's look for
  a ticket made for a card again only after a making that failed, since
  their search may not list one made a moment before, so a daemon
  stopped in the instant after the service made one, and back within
  that moment, may make a second. Jira's and Notion's have been played
  against stand-ins written from the services' documentation, and not
  yet against the services (`cli`: `packages.rs`, their cases in
  `extensions/jira/` and `extensions/notion-pages/`).
- A round of work that the daemon was stopped in the middle of, after its
  agent had finished, is begun again and says a second time what it says
  for itself: a line of the card's history, a line of the channel, one
  more counted where a comparison counts it. The agent is asked once, and
  nothing acts on the second saying. The restart scenarios count each line
  of a channel once however often it was said, so they do not see this.
- Two stops of one instance asked in the same moment can both do what
  its text says is done as it stops.
- A rule that watches a step finds what it said before a stop by topic,
  step and run after the place it kept: where the step's own action says
  the same topic while the rule is between keeping its place and saying,
  and the daemon stops there, the rule takes the action's event for its
  own and its event is lost. Telling the two apart exactly needs a mark
  on the event that blueprints would see.
- A step that was passed over, in a run the daemon was stopped in and that
  is ended at the start, can read as cancelled where it was skipped. And
  after such a start, an instance that fails may not say which call was
  waiting on it.
- The database is written so that a daemon that crashes or is killed
  loses nothing. A machine that loses power can lose the last moments
  written; the database comes back whole, a little in the past, and what
  was done outside it meanwhile is not undone.
- No real agent's program has been through the restart scenarios: they
  are played with stand-ins.

**Who may do what**

- A scope is for the whole factory, not for one project: an agent whose
  slot is in one project can ask the daemon about another.
- An agent the factory does not start (`mw agent invite`) keeps its
  credential on the machine it runs on, as a bearer token in a file
  only its user can read on Linux and macOS, and on Windows in that
  user's profile with nothing more set on it: whoever reads the file is
  that agent, for as long as the agent is not stopped or invited again.
  The factory knows nothing of where such an agent runs or what it runs
  as, and holds no wall round it.
- A message that names such an agent starts nothing and is answered by
  nobody: it is put in the agent's inbox (`mw inbox --as <name>`), where
  it waits until the agent looks, or is printed at once where the
  agent follows its box (`mw inbox --follow --as <name>`). The line is written when the daemon hears the message
  for agents, a moment after the message is kept and not in the same
  write; stopped between the two, it is written when the daemon is back.
- **The inbox** holds four kinds of line: somebody named in a
  channel, somebody named on a record, a conversation, and something a
  blueprint told. An agent
  the factory starts has no
  inbox: it holds no `inbox` scope and is given a turn by what names
  it, and no line; told by a blueprint, it is passed over. Giving it one is a word added to what an agent
  usually holds (`kernel::principal::AGENT_SCOPES`), which is also what
  a row of a fleet's `agents` that says nothing gives: so the same word
  in `std/fleet`'s list (`unsaid` of `holds`), a new version of that
  text, and the contract files with it; and from then a key or an agent
  made before, which holds no `inbox`, is refused changing such a row
  and adding an agent with the usual scopes until it is given the
  scope. A key and an agent's slot made before there was an inbox hold
  no `inbox` until they are given it; people hold it at once.
- **A conversation** is as closely held as a channel kept to its
  members, and no more: the factory's owner reads every one, for now,
  and whoever has the device reads its files. An agent the factory does
  not start that is in one, or in any channel kept to its members,
  reads it with its own credential wherever its program runs, behind no
  wall this factory holds: what is said there is as closely held as
  that program is. An agent the factory starts is given a turn by a
  conversation as by any channel: at once where it is the conversation's
  one started agent or is named with `@`, and not at all where there
  are several and none is named; whether a message to an agent is a
  turn at once or waits is not a setting yet, and such an agent has no
  line. Nothing caps how many conversations an agent writes: what
  agents say to each other where no discussion holds them starts thirty
  turns an hour at most in a channel, and what bots say as many apart,
  after which they wait for a person, who is told in their inbox, and an agent from outside
  that writes a new conversation each time is stopped by nothing. A conversation opened as a
  channel is not made a conversation again, and the lines it raised
  stay in their boxes. It belongs to the usual workspace until it is
  opened as a channel, which nothing shows; so a workspace that holds
  one is taken away, with where its channels go, only by somebody who
  reads it. And nothing of mail beyond this is built: no copy to
  somebody who is not in it, nothing attached as it is written (it is
  attached once the conversation is open), and no telling outside the
  factory that something came.
- A word after `@` names every account called so, and every agent or
  bot called so: an account that is called what an agent of the channel
  or the factory's bot is called has a line put in its inbox when that
  one is spoken to. Two accounts called the same are both told.
- A round that fails is told twice: as something of the fleet that
  failed, about what failed, and as its card left for a person, about
  the card. And that a card was put to whoever plans it as often as it
  may be is said as `card.left` like any leaving, and held by no test:
  a stage that plans stops what plans a card as the card leaves its
  column, so it comes about only where a card is back before its stage
  has heard it go. A card that could not be taken at all (its board
  keeps no `taken`) is said where the fleet reports and by no event.
- What a blueprint tells is its own words, said as whoever applied it:
  whoever a row of `tells` names is told a card's question whether or
  not they may read that card's board, as they would be told it in a
  channel they read. The line opens only what they may read. A line
  about a card or a record that has been taken away since stays in the
  box, says its words and opens nothing.
- A step that told somebody, done again where the daemon stopped after
  another step had raised the same line (the same thing, for the same
  reason), raises it once more, with the earlier words. The fleet's own
  tellings take turns and cannot come so.
- A reading of an inbox hands its latest five hundred lines; how many
  wait unread is counted over all of them. What is said of a box as it
  changes (`inbox.*`) stays in the journal after the channel it was of
  is taken away: it names the line, the channel and the message by id
  and carries no words.
- A card given to such an agent while it is in a column a fleet works
  is taken into work in hand by the fleet's round and left there for
  it, said once where the fleet reports; a fleet that was started by a
  program from before this, and goes on with the blueprints it had,
  says of it what it says of an agent that is not up. Where cards are
  planned, a card given to one stays where it is and the fleet says
  that nobody of the role is free, which is so and is not why.
- What such an agent has in hand is read by the client from the board
  of the one project it is listed with: a card given to it on another
  project's board is not said in Settings.
- A row of a fleet's agents whose program was let at more of the device
  (no wall, commands unasked, more shown, a key handed) is still changed
  in its other cells by somebody who may change agents and could not
  have let it: told something else, or made more of. The agent's own
  route refuses them; the row asks it only of the cell that lets.
- A stage moves cards on and merges their branches as whoever applied
  the fleet. Its row is written only by somebody who holds what the
  fleet's agents hold; where those were all given less than moving a
  card takes, it can be written by somebody who may not move one.
- A blueprint that hears of something kept private does so as whoever
  applied it, and what it then makes of that is read as anything of a
  blueprint's is: what its instance keeps and why its instance failed,
  what it starts and with what, the events it says itself (an `emit`
  step, and what an action it asks says, an agent's turn among them)
  and what it posts. What the run itself kept of the event and of what
  its steps gave, and what is said of the run as it goes (each line a
  step prints, why a step or the run failed), is for whoever may be
  told the event. That such a run was set off, when, by an event of
  which topic and how it ended is still read by whoever may read runs,
  with where that event came from: who said it, and what hearing it
  took, which for a channel held to its members is the channel's id.
- A channel kept to its members (`mw help channel`; Private in the
  client) is held at every way the daemon hands a channel's words to
  somebody: the routes, a search, the stream of events, a handler and
  a step that waits, a blueprint's own actions, and an agent that is
  in it. What is not, of one that was open before it was kept:
  - What was said in it while it was open was said to everybody, and
    stays so wherever it was kept as it was said: the events of that
    time are in the journal for whoever reads it from an earlier place
    (`mw events`, a stream read again, a handler that takes its place
    from before), and an agent that was asked there kept what its
    program keeps of that in the home it has for everything else. The
    routes hand what was said there to its members alone from the
    moment it is kept, and a search leaves it out for anybody else.
  - That it was kept is told to whoever may read channels, with what
    it was called until then: whoever is not in it has to stop showing
    it. From there what it is called is its members'.
  - A blueprint that names it by the name it had (`channel: shop`)
    finds no channel once it is kept, and what it posts there fails: a
    kept channel is named by its id. The fleet that ships posts by id,
    and its channel is found again when it is applied again, as the
    kept channel of its project that is called so and has whoever
    applied it in it. `std/fleet` takes no word that makes its channel
    kept: it is kept from its page, or with `mw channel keep`.
  - It is not kept, and a kept one is not opened again, while one of
    its agents is being asked something there: nothing carries a turn
    in hand across. And keeping one reads every turn the factory's
    agents ever had, once, to find those asked there.
  - Nobody takes a person out of a kept channel but that person. Its
    last person does not leave it; the factory's owner, who reads every
    one for now, can add themselves, open it again or remove it.
  - A blueprint whose applier has left its kept channel is refused
    there as anybody outside it is, and `channel.ensure` then makes it
    another of the same name.

  What is not, of an agent:
  - The three packages that ship say which of what their programs keep
    is the sign-in (`agent.wall.signin`) from the names of what each
    keeps on Linux, and none has been run in a kept channel's home:
    whether each starts there signed in is to be tried
    ([docs/extensions.md](extensions.md), *`wall`*, has what to
    try and in what order). A copy of a package installed before it
    said so does not say it until its owner takes the one that came
    (`mw extension update <name>`, or Update on the package's page):
    until then its agents are refused a kept channel, and told that.
  - A sign-in that is one file is shown, in a kept channel's home, as
    that file: its program writes it where it is, and cannot put
    another file in its place from behind the wall. That is how two of
    the three packages' programs keep theirs (each package's own notes
    say which), and how either program renews it is not known:
    one that renews it by putting a new file in its place fails to from
    a kept channel's home, and its turns there fail as those of a
    program that is not signed in until it has renewed it in a turn
    asked anywhere else. A folder is renewed either way, and an agent
    handed a key or a token the factory holds keeps no sign-in in a
    file. A lock a program keeps beside its sign-in is each home's own.
  - What an agent's program writes where it works, in its project, is
    the project's, whatever channel asked for it; and its program
    reaches the network from behind its wall with what it read. What
    it is shown as its sign-in is one place, written from a kept
    channel's home as from any other, so that it can be renewed: a
    program that set out to could leave words there for a turn asked
    somewhere else. What is held is what a program keeps of its
    conversations by itself, not a program that means to carry them.
  - Nobody who does not read the channel is told, as it is said, that
    its agent has a turn in hand there: the Agents page shows such an
    agent as it shows one at rest. Its turns are listed for them with
    the words left out.
  - A turn that is in hand as its channel is taken away keeps its
    words on its row when it ends, shown to nobody.
  - Somebody who may change agents and is in a kept channel is told,
    where an agent is not brought in for it, that the agent is held to
    another channel, and not which.

  And of the rest: what a member's own blueprint says itself of what
  it heard (an `emit` step, what an action it asks says: each line its
  steps print and why one failed is for the members, as what is said
  of any run is for whoever may be told what set it off); the
  database, the search index and the attached files, to
  whoever has the machine or a copy of the factory, or runs an agent
  with no wall on it (nothing of a kept channel is sealed on the disk);
  a page that was
  attached, kept an hour by a browser that showed it; how many
  documents a search's index holds; what was said of a channel while
  it was open and was kept afterwards, which whoever reads the journal
  from an earlier place is told as it was said; and whoever holds what the owner
  holds reads every one, but for an agent. A member can repeat what
  they read.
- An installed package that is not the one that came with the program
  is said to differ, and where; it is not said which of the two is the
  earlier. A package says no version and nothing is kept of what was
  installed but the copy, so a copy you changed and a copy of what an
  earlier program shipped read the same, and `mw extension update`
  replaces either. The copy it replaces is kept beside the new one
  once: a second update writes over what the first kept.
- A library's pages, and a mockup's, are answered for the browser's cookie
  alone: something else served on the same machine and opened in that
  browser can read them, for as long as the session lasts, and change
  nothing.
- Withdrawing a key (`mw key revoke`) stops what it applied that this
  device runs and was applied since keys were kept with what they
  applied; what a step of its applied on another device is that device's
  to stop, and a key made with a key before that was kept is not
  withdrawn with it. An apply the key had begun as it was withdrawn,
  whose instance is written after what it applied was looked for, goes
  on.
- An agent's program reaches the network from behind its wall, and a
  command a blueprint runs, or a service it hosts, has no wall.
- An agent that is taken away (`mw agent remove`) is named as text by
  what it left, and whatever reads such a name as "whoever is called
  that now" takes a later agent of the name for it: a card given to the
  first is the later one's to work, what the first one did is answered
  for by what the later one holds where a blueprint asks, and what is
  measured of both is under the one name. What the first one was asked
  is kept, and once the name is somebody's again no command lists it.
  What its program kept of its own outside the folder the factory gave
  it (in the home of whoever runs the daemon, where its package shows
  it that) is not the factory's, and is left. In a factory from before,
  a piece of work asked of it before work said whose project it was is
  told apart, once it is gone, as nobody's, or as the project's of a
  later agent of its name.

**Blueprints and their refusals**

- A run that is cut because its instance is applied over may still be
  ending while the handlers started once the steps have run again
  handle what was said next: a `call` it was in the middle of stops
  the child it made, with whatever that child does as it stops. Its
  steps are over by then and it does nothing more, but for that while
  two runs of one handler read as under way.
- An action that asks several things one after another, and is looked
  at with its run cut, asks the next of them before it is cut, as it
  would have a moment before the cut.
- A fork of a blueprint that ships takes nothing of what a later
  version of the program brings for its default, and nothing merges the
  two: its Summary says what moved in the default since, and the text
  is the project's to change by hand. What it is said to differ from is
  the text it was first forked from, for as long as it is a fork. A
  fork an earlier version of the program made, and a file somebody put
  under a shipped name by hand, has nothing kept of where it came from
  and says only what its texts show.
- A number param written alone in quotes where an action takes text
  (`message: "${params.count}"`) passes `mw check` and fails when it runs.
- A `call` of a blueprint that is not there is a warning, with nothing
  offered in its place; applied, the instance starts and its run fails.
- A stage whose row says `does` with a word that is no blueprint says
  nothing when the row is written. When a card comes, the channel says
  that no blueprint is called that, and does not name the row.
- A blank handed to a linked prompt that the prompt has not got is taken
  silently.
- A handler that listens by something its instance is handed (a `for:`
  over a param) and is then handed something else goes on from where it
  had last been handed anything, and so is handed what there was for
  the new value from before it listened for it. What a fleet's stage
  does with a card that comes to its column asks the card's row first
  and is not misled. A stage that merges what a person passes by hand
  (`by_hand: merge`, as any stage that merges does unless its row says
  `leave`), given another column, may still merge the branch
  of a card somebody passed from that column before the row said so.
- These words of the language are used by nothing that ships and may be
  taken out after the preview; do not build on them. The functions
  `ends_with`, `upper`, `int`, `float`, `keys`, `min`, `max`, `where`,
  `sort`, `without`, `range`, `pluck`, `at`, `from_json` and `sum`; the
  operators `-`, `*`, `/` and `%`; `max_parallel` of a blueprint,
  `if_fail_ask_agent` of a step, `debounce` of a handler, and
  `max_backoff` of `retry`. Everything else stays: from the preview on,
  nothing is taken out of the language.
- An apply that comes to something while it is being stopped waits
  for the stop and is then an apply where nothing is running, and is
  refused, saying why, where it is asked by what the stop itself does
  or by a step of what is being stopped. Two things about it are not
  held. What something does as it stops may wait for something outside
  it that hears of that by an event and then applies the thing being
  stopped: nothing ties the two together, and each waits for the other
  until the step's own time runs out, if it says one. And where an
  instance takes a text by its steps running again while one of its
  handlers has a run under way, and the daemon stops between the text
  being written as taken and that run being cut, the run is taken up
  when the daemon is back and ends well, where with no stop it is
  cancelled (found by the runner's play of a stop whose asker is cut
  short, `meets.rs`, which is not stopped at that one place).
- A tree put back on the texts it had by its instance (`mw apply
  --instance <id> --back`) is asked with the params the instance had
  when the asking began: given other params by something else in that
  moment, it is put back with the earlier ones.
- An agent that somebody stops by itself under a card in hand fails the
  card's round: a step is told of a turn somebody called off what it is
  told of one that failed. The fleet, which is running, says so where
  it reports, as something that failed, and the card waits in `doing`
  to be put in `ready`. (Where it is the fleet that is stopped,
  everything under it is called off before any of it is taken down, and
  no round hears its agent's turn called off.)
- A card says how often it was sent back where it is put from `review`
  into a column the board has before that one. Sent back from a column
  of another name where cards are reviewed, to anywhere but `doing`, it
  is not counted on the card; the stage that sent it still counts what
  it decided, which is what ends a loop between agents.
- How many runs were cancelled is counted nowhere: where runs are
  measured they are counted with the rest, and nothing there says how
  a run ended (`--by` a label nothing has is passed over and says
  nothing). A step that was cut short is in no count at all.
- A blueprint's summary is of one text. What the blueprints it starts
  do, make and may do is said in their own: `std/fleet`'s says that it
  runs `std/arrangement`, and no more of it. And what a blueprint's
  runs have cost is said under its Summary from what was measured under
  its name, which is its runs and what it measures itself: an agent's
  turn is measured under the labels its step carries and not under the
  name of the blueprint that asked for it, so the tokens and money an
  agent spent for a blueprint are not said there. They are under
  Measurements, by agent.
- A rollout's rule reads a thing once it is counted, so a thing given
  the new setup is read when its count says something of it (a review),
  whatever came of things still under way; and an event a stopped daemon
  reads again that was said in the same millisecond as the shares were
  last set may be read as said after it.
- A comparison's name is used once: what was measured under it is kept.
  What was spent shows in a comparison only where an agent's package
  reports it.
- A handler or a `wait` that will hear nothing because of where its
  text was applied is said by the apply, as a warning; a check that is
  told no project says nothing of it, and neither says anything of a
  topic or a project that is filled in when the blueprint runs.
- A project's `segments` are run where cards are worked, among the
  things a round does; where they are reviewed, about the review and
  the merge; and where they are planned, or handed to a blueprint of
  the row's naming, before that one thing and after it. What is laid
  after a blueprint of the project's own is done only where that
  blueprint says what it came to and moves nothing itself; one that
  hands its card on has done with it. Nothing is sent back from a
  column where cards are planned: a thing that says so leaves the card
  for a person. With nobody in a reviewing
  stage's role nothing of its list is run either. A stage does so many
  things at most in one going through a card's list (`most` of its row,
  six where cards are worked and five otherwise unless it says), and a
  card with more is left for a person with the command that raises it;
  each going through is held to the rows and that number as they are
  when it begins (`segments.rs`, and stopped five deep in a list, the
  simulation's `rounded.rs`); what is done once a
  review has passed is what was laid after it when that review began.
  A review whose way of reviewing is changed while it is in session
  begins again at the first of its card's list. A
  setting typed wrongly in a row is found when a card reaches it, which
  is then left for a person with the sentence, and not as the row is
  written.
- A project that keeps a copy of the round from when a round was one
  text (`blueprints/std/lane-round.bp` of its own) is handed what the
  round is handed now, which that copy does not take: the fleet's apply
  says so. The copy is kept under another name and named in a row of
  `segments` at the place of the work, or its change is made a row.

**A sandbox test**

- A sandbox test (`mw sandbox`) shows what a blueprint does with what
  it is given, and nothing of what the things stood in would have done:
  no command is run, no agent is asked, and no repository, file of a
  project, service, router or other device is touched. A step that asks
  one of those is answered what whoever runs the test says, else what
  its action says a stand-in answers (a command ends with 0), else
  nothing: so the fleet in a sandbox is told that nobody was there to
  give a card to, unless it is told otherwise. Passing there is not
  passing for real.
- What such an action would have said as it worked (an agent's turn
  beginning and ending, a service started, a file changed) is not said
  in a sandbox: a handler that waits for one of those hears it only
  where whoever runs the test says it by hand.
- A sandbox starts empty: one made-up project, with no rows. Nothing of
  a real project is in it but the texts of its own blueprints, as they
  were read when the sandbox was made. A copy of real tables to begin
  from is not built.
- The factory's bot takes no command there, an agent named in a
  channel is asked nothing, and nothing said there is searched: those
  are the daemon's doing, and a sandbox has none. Nor has it the
  factory's variables or its profile's name: a text that reads
  `profile.vars` reads nothing there.
- A table kept anywhere but in the journal is refused in a sandbox: by
  a store service, which is an address requests are sent to, as files,
  and as a library's documents or a project's mockups, which are files
  of a project, and a sandbox has none. So what `std/docs`,
  `std/mockups` and `std/site` do is not shown there.
- A package's action cannot say what a stand-in for it answers: it is
  answered nothing, unless whoever runs the test says.
- Somebody a blueprint tells (`inbox.tell`) is the made-up person where
  it says `applier`, and nobody where it says a handle or an actor: a
  sandbox has no account but theirs, so a step that tells `@grace` is
  refused there as a handle that names nobody is, where in the factory
  in use she would be told.
- On its page a sandbox test runs to rest after each thing done to it:
  there is no stepping through a run a step at a time. A stand-in is
  answered what it usually answers, that it fails, or what is written
  out; an action says no other choices. And the page's sandbox is not
  kept over a reload of the page: the daemon ends it when nobody has
  asked it anything for a quarter of an hour.

**Logs**

- What a service prints (`logs/services/`) is not let go by days: it is
  kept as it always was, the newest eight megabytes of each and one file
  before them. Of a run, its row and each step's, with what each step
  gave, are kept for ever whatever the days said.
- A file of a step is old by when the system says it was last written;
  what was said of a step, by the factory's clock. The two are one clock
  on a machine whose clock is not put back.
- What was said of steps that a handler at rest is owed goes only once
  its place has been written past it, which for a handler that hears
  something seldom said may be long after the days said. And while
  anything alive waits (`wait`) for what is said of a step, or is not in
  the daemon's hand, nothing said of steps goes anywhere.
- The databases stay in the data folder whatever folder is said for
  logs: moving them is another piece of work.

**Systems**

- More sent than a route takes (a file of more than 32 MB to a channel)
  is read to its end before it is refused, so that the refusal is read
  (`api`: `sent.rs`), only as far as four times what the route takes. A
  client that sends
  more than that has the connection closed while it is still sending,
  and on a system that then gives up what had come back on it (Windows)
  is told the connection was aborted, and not why.
- Never built or run on macOS. On Windows run by hand once, and its
  suites built and run there by hand on two days (the status table says
  what was seen, and how many passed): no gate runs anything there, and
  both systems are checked from Linux, which finds what would not
  compile and nothing else. A test that rests on what only Unix has is
  not compiled for Windows (one in ten of them, each saying beside its
  mark what of Unix it needs). Of what follows of
  Windows, the suites hold there what a process does for a program it
  started itself (a step, a service, an agent's program and one kept
  going without its daemon: started, stopped with what it started,
  found again) and what that system's shell makes of a user's line;
  what a wrapper does as a program of its own, and a daemon as a
  process of its own, is in tests that are still marked for Unix: it
  is what the code is written to do, and was run there by hand once
  (`crates/hosting/src/group.rs` has the whole of it; what is a function
  of plain values is played on Linux, `crates/hosting/tests/suite/group.rs`,
  `named.rs`, `cleared.rs`, `script.rs`).
- The window's program on Windows has been built there and run only
  where it opens no window (`--help`; `--no-start` at an address where
  nothing answered, which it said and ended). Whether it opens a window,
  shows the client in it, moves and resizes when asked, and whether a
  picture copied on Windows is pasted into a channel there (the reason
  to run it on Windows against a daemon in WSL: the Linux window shown
  through WSLg is handed nothing of one), needs a person at a screen.
- On Windows nothing is asked to stop: a step that is cancelled, a
  service that is stopped (its `stop_grace` is not waited out) and an
  agent's program that is called off are ended where they stand, with
  everything they started, and a program so ended says nothing of how
  it ended. On Linux and macOS a program is asked first and may put its
  things away.
- On Windows nothing a program started outlives what holds its group.
  What a step, a service or an agent's kept program left running ends
  when that program's wrapper does (on Linux it goes on until its group
  is stopped), and an agent's program that speaks to the daemon over
  pipes, or a package's, ends with the daemon and with it whatever it
  had started, but for what it started in the instant before the daemon
  had put it in its group. A daemon that is itself started inside a job
  object that ends what is in it (some terminals, a hosted runner)
  takes everything with it when that job ends, kept programs included.
- On Windows a program that is to be stopped is left word in a file,
  which its wrapper looks for ten times a second; a wrapper that a
  later daemon found again and that no longer looks cannot be ended by
  the factory. And what keeps a daemon running (`up`) cannot ask it to
  stop by itself there: the daemon hears the interrupt typed at the
  terminal they share, and `down` asks it from anywhere.
- On Windows a program is found by its name as that system's shell
  finds it (`tool.cmd`, `tool.bat` as well as `tool.exe`, by the list of
  endings the system keeps), and a script of that shell is started
  through it by the standard library, which refuses an argument it
  cannot write for that shell: said in a sentence, with nothing started.
  A user's own `run:`, and a health check's `cmd:`, is put on that
  shell's line as it was written (`cmd /D /S /C "<the line>"`, which
  that shell reads as a line typed at its prompt: its quotation marks,
  `&`, `|`, `>`, `^` and `%NAME%` are the shell's to read, and not as a
  line of a `.cmd` file in every way), and one of several lines is
  refused before anything is started, saying to put them in a file of
  that shell's and name it: the shell is started with one line. This is
  run there by the hosting suite, where that is run (that shell handed
  lines with quotation marks, `&`, `|`, `^` and `%NAME%`, and a step and
  a health check that each have it read one); what was run there by
  hand before handed the script over as an argument, and a quotation
  mark in it was read with a backslash before it. By reading, what that shell's
  own commands print outside ASCII is in the system's code page and is
  not read here as it was printed.
- On Windows the daemon looks at what each program at work has printed
  four times a second, as well as being told: that system says a file
  kept open was written to once and then when it is let go of, and
  what a step said after its first line was read when its program
  ended. On Linux and macOS it is told, and what is quiet costs
  nothing.
- On Windows a connection to an address nothing listens at is refused
  after about two seconds, where Linux refuses at once, and the system
  goes on trying meanwhile. So a connection the router first sends to
  a backend that is down is still answered by another, two seconds
  later (the hosting suite's test of it takes six seconds there for
  the three connections that meet the dead one); one sent to a service
  that ended badly may reach it started again, a second later; and a
  command that finds no daemon says so after two seconds.
- On Windows a command of your own that a blueprint hands a variable
  (`run:`, a bot's command, a row's command for a card) is read by that
  system's shell, which puts what a `%NAME%` holds into its line before
  it reads the line: a bot whose command is `echo %MESSAGE%` runs what
  is said to it, and a row's `echo %CARD_TITLE%` what a card is called. Name a
  program there that reads the variable itself. On Linux and macOS
  the shell reads the line first, and nothing a `$NAME` holds is run.
- A package built on one Linux needs that system's C library or newer;
  the released one is built on an older system for that reason.
- The packages for Windows and macOS are folders and nothing more:
  neither is signed, nothing puts the command on a path or the window in
  a menu, and the one for macOS is no application. The one for Windows
  is a zip whose files are stored as they are, since the tooling writes
  it itself and compresses nothing: more than twice what the same
  folder is compressed (31 MB where that came to 13). A program built on Windows by that system's own
  compiler asks for Microsoft's Visual C++ runtime, which most machines
  have and no package brings. Started with a double click, the window's
  program on Windows opens a console beside its window.
- A package carries the notices of the libraries it is built from
  (`THIRD-PARTY-NOTICES.md`), and what holds that file to the code is
  that it names the crates and packages that are locked. That each text
  in it is the right one is the tool's reading of each crate's files and
  nobody's check: a crate that carries no file of its licence (eighteen,
  nearly all of macOS's and Windows' own interfaces) is said by the
  standard text of the licence its manifest names. And what a crate
  carries of somebody else's that is no crate is found by reading it
  when it is added: the database library is named there, and Microsoft's
  loader for its web view in the window's program for Windows is named
  without the text of its terms, which the crate that carries it does
  not hold.
- A factory made by an earlier program that has kept more than twenty
  thousand instances or runs goes on writing two indexes nothing reads
  any more, about two pages of some forty for each run of a handler:
  taking an index away reads all of it, and is not done by opening a
  factory that large. A project that is moved to a journal of its own
  (`mw apart`) no longer writes them, since they are not made in a
  project's file; what the factory's own file holds of them stays
  until it is repacked.

**The client**

- A project's Git page is told the repository changed by its own
  folders (`.git`, its branches and remotes), so a commit, a branch, a
  file staged and a fetch are heard however they were made; a file of
  the working tree written and not staged by another program is not, and
  is seen when something else is or when Refresh is pressed. What a
  package shows of it is read as whoever runs the daemon is signed in to
  the package's program, for anybody who may read the project and use the
  package: nothing is shown as whoever asks is at the service. Which of its
  tabs shows is a word an address can carry, and no address carries it yet.
- GitHub's Git table is applied by hand (`github/work`): a project whose
  program comes to be signed in later is not given it by itself. The
  issues a row says are those its pull request closes, with no time each
  last changed; where a card has no pull request yet, its branch's commit
  is read only while the place it was worked in is there.
- What the client looks like is checked by driving it in a headless
  browser and reading the pictures, by hand. Whether the desktop window
  moves and resizes when asked needs a person at a screen.
- The daemon writes no time in anybody's zone: it keeps a zone as text,
  held to the form a zone's name has, and looks up no zone but the one
  a handler's time of day says it fires in (`at: 02:00 Europe/Zagreb`). Every time is
  written by the client, by what its browser knows. So nothing a
  blueprint says (a channel's post, a line of somebody's inbox) can
  write a time in its reader's zone yet; that is later work, and brings
  a list of zones into the program. And a zone's short name is the one
  the browser has for it in English: a zone of the United States has
  letters (`MDT`), and most others are said by their offset (`GMT+9`).
- An example is put into a project from the terminal (`mw example add`)
  or from the Setup Guide once its steps are done (*What next*); no
  other page of the client offers it.
- Which running things were applied before handlers heard by project,
  and so still hear every project's events, is said by `mw instances`
  and `mw instance`; no page of the client says it. That the editor
  lists what an apply said under the text is held by the list's own test
  and by no check that drives a browser.
- When a table, a board or a library was last changed, as its page's
  head says it, is worked out from what the page read: a record that
  was taken away is not counted, since nothing still there says so, and
  who made the change is named only where a line of history was written
  in the same few seconds. So what another program writes to a file
  within those seconds of a change made through the factory is named as
  that one's. No route lists what last happened to a table's records,
  which is what would close both.
- A column somebody added is made another kind only where every row
  already fits it: nothing is converted, so words that are numbers are
  not a number until each is written as one. An option of a choice is
  renamed in the rows that chose it only where the table is kept in the
  factory's database; kept as files or by a store service, a row that
  chose it has to be changed first. A row written in the instant a
  column is retyped or one of its options taken away may hold what the
  column no longer takes, and is then refused until that cell is
  changed: the same is so where a blueprint declares a table again.
  What columns a board has is still its blueprint's and its page's
  (Arrange the columns, in the gear of its head): a column added there
  goes when `std/board` is
  applied again without it. Search goes on finding a renamed option by
  its old word until the row is next written. And no column is added
  to a library or to mockups.
- A channel given back from the Trash has none of what its agents'
  programs remembered of their conversations there, which went as it
  went.
- A table cannot be taken away, copied, moved to another project or
  locked, by the client or by any route, and a file cannot be moved or
  renamed: a document is known by where it is, and what was said on it
  would be left behind. A CSV file read into a table is written a
  record at a time, and what was written stays written where a later
  line is refused.
- A file deleted from its own page is taken out of what its person
  pinned and of what that browser lately opened; one deleted any other
  way is listed there until its project goes, and what another browser
  lately opened lists it there until its project goes.
- That a file's copy and its deleting are offered only to whoever may
  write a project's files is held by a check that has the daemon's
  answer say less: everybody who signs in holds that today.
