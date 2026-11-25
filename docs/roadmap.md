# Roadmap

Where the application is going. [architecture.md](architecture.md) records
what is built and why, and the other documents here say how each part is
used. This one records what the product is for, what is intended and not
built, and what is still being designed, so that none of it is worked out
twice.

Each item is **decided**, a **leaning**, **open**, or a **design**: a
proposal written out, with the questions that have to be answered before it
is built. Where something is built it is named, with where it is described,
and what follows is what is left of it.

What belongs to the hosted side of the product is said here only as far
as the application meets it.

## The product, and what is paid

**Decided**, as the README's *Product Vision* says:

- **Team collaboration and web publishing are paid, hosted services.**
  Working as a team across machines (one installation reaching another
  that it cannot reach directly) and publishing to the web (a wiki made
  from topics and kept current by agents, for example) will be offered as
  services we host. They are the cloud parts, and they are conveniences:
  most of what makes it better for a team can be built and run on one's
  own, and the services are for whoever would rather not.
- **The application works fully without them.** Everything in this
  repository stays free to run yourself. A free installation never
  contacts our services (the `offline` gate holds that on every change),
  nothing in the application depends on them, and two machines of one's
  own can be linked directly.
- **Sharing blueprints** with other people is coming: a blueprint, or a
  set of them, handed to another factory and installed there.
- **The standard blueprints are to be versioned and released on their
  own**, always free, so that they can change faster than the program,
  and installed and updated the way an extension is, with no restart. A
  version somebody has keeps working. Today they are built into the
  program, and a new version of them needs a new program and a restart.
- **The interface in more languages.** The client and the command line
  speak English today; other languages are planned.

## What it is for

- **Small teams building large systems.** The tools a company with a
  platform team has for changing a running system safely (a rollout a share
  at a time, two versions compared, everything measured) in one program
  that a few people can run. Larger teams can use it as well.
- **Iteration and measurement come first.** What it is meant to give is
  direction from evidence: two components, two versions, two ways of
  handling a piece of work, run against each other and compared on what
  happened. A feature is judged by whether it makes such an experiment
  cheaper to run or its evidence better.
- **Agents do the work, and it does not compete with those who make them.**
  It works with their models and with their own orchestration, with some
  overlap. What it adds is the place where what agents build is hosted,
  compared, kept or thrown away.
- **It runs on one's own machines.** That must always be possible, and
  nothing leaves the machine unasked ([architecture.md](architecture.md),
  *What leaves the machine*). It is not a stand against anybody's cloud:
  what matters is that devices of one's own can be set up, joined on one
  network and managed from one place.

### Why blueprints

In a system that is run by agents and people together, the logic at its
core does not change often. What changes is configuration: who gets what,
which version takes how much, what happens when something fails. A factory
has to answer such a change at once, and without being built again.

So the line between a blueprint and compiled code is how often a thing
changes, not what kind of thing it is. What varies, or is worth trying two
ways, is text. What stays the same is an action or a component.

It follows that how a running factory takes a changed text is the product
itself: which instances are behind, a text taken in place, state carried
over, everything put back when a change fails. That has to be seamless.

**A priority: blueprints people and agents read together.** The founding
idea was that the hard parts are Rust, behind a few plain keywords and
common actions, so that a blueprint keeps Rust's performance and the care
of a designed language while being a short text a person and an agent can
both understand and change. The blueprints that ship have grown long and
intricate since, further from that than was meant. Bringing them back to
it is a high priority, as the README's *Product Vision* says. The `plain`
gate (a blueprint decides nothing itself) and the rule that shipped
blueprints only get shorter are steps toward it, not the end of it: what
a blueprint still spells out step by step is to move into actions and a
few words, until one can be read at a glance.

### Components (leaning)

The aim is an easy way to write the parts of a system (services, the things
a cloud would otherwise be rented for) in Rust, with what is hard about
Rust made lighter. A component is a program built by itself, with a build
of its own, that the factory provisions, supervises, rolls out and
measures. A blueprint is its kit: what it is, what it needs and what it
offers are text, and the conductor owns time, retries, state and doing
many things at once, so that the program can stay small. It need not be
Rust.

What is missing is written out under *Designs*, below: saying in text that
a running service's actions are requests to it, and comparing two running
components on what they answered.

## The rules it is built by

- **What happens per request is a primitive; what happens per decision is
  a blueprint.** Carrying connections, checking health and supervising a
  process are in the daemon, compiled once. Which version gets how much
  traffic, when to go on and when to go back, is a blueprint: text, changed
  and applied again in moments, with nothing to build.
- **Primitives are few and general.** A new need is first tried as a
  blueprint over the ones there are. A canary rollout is one already: the
  router is the primitive, `std/slot` and `std/canary-stage` are text.
- **Text is only quicker than Rust if nobody has to remember it.** What can
  be written at a place is offered there, with what it means: the fields,
  the actions this device has, an action's arguments, the steps and what
  each gives back, the events and what each carries. The editor is handed
  all of it by the daemon, so what is added is offered without the client
  being changed ([architecture.md](architecture.md), *The client*).
- **Adding a primitive must not mean building the daemon.** A package
  brings actions of its own, each a way its program is run
  ([extensions.md](extensions.md)). What still means building the daemon is
  anything that has to run for long or answer many times; that is what a
  component is for.
- **What ships is the same on every system** (built). A script is one
  shell's, so a blueprint that ships, standard or a package's, has none of
  its own: what it asks of a program is the program and its arguments
  (`cmd:`, and `cli:` for this program's own command line), and what a
  script would decide is steps, each seen in a run. A command its user
  hands it as a param is theirs. The `portable` gate holds it.
  - *Not built:* `std/log-feed` still follows a file with a system's own
    program and is the gate's one named exception. It needs a command of
    the product's own that follows a file and hands its lines on, and then
    says `cli:` as `std/store` does.
  - *Not built:* the command the client offers for a new bot is a shell's,
    and so is a service's health check by `cmd:` (no shipped blueprint has
    one).
  - *Not run:* Windows and macOS. That `cmd:` is the same there rests on
    how a program is started, and on `git` being installed.
- **A factory in use is a copy** (built). Work on the product must not
  reach a factory someone is using. A program reads the checkout it was
  built from only when it is run from where it was built;
  `cargo xtask install` copies the programs, the client and the packages
  into a folder of their own; and what is installed changes only when it
  is installed again and its daemon is next started. Working on the
  product is a second factory, run from the checkout under a profile
  of its own ([architecture.md](architecture.md), *Factories and
  profiles*).
  A release is the same thing made to be downloaded: `cargo xtask package`
  builds what a release is built with and makes a folder to unpack and
  run, for the system it is run on, with the one file of it and that
  file's sum: a `.tar.gz` for Linux and for macOS, a `.zip` for Windows,
  each with a page of its own in that system's commands and paths. The
  one for Linux is made on the oldest system that has the web view the
  window needs, when a version is tagged, and the other two on their own
  systems after it. `--target` makes another system's package from this
  one, for trying there and never for a release.
  On Linux an install, and a package by a script of its own, put the
  product in the desktop's menu, with an icon that is a placeholder.
  - *Not built:* an installer for any system (an update; a menu entry
    anywhere but Linux; the mark the icon is a placeholder for); an
    application for macOS, which is a bundle with an icon of its own,
    where the package holds the plain programs; the icon in the programs
    for Windows, and the window's program there marked as one, so that
    it opens no console; signing what is released, and for macOS having
    it notarised; a zip that is compressed;
    taking the client back to the one before when an
    install is regretted; holding a new client back until the daemon is
    restarted, so for that long a newer client talks to an older daemon.
  - *Not run:* Windows and macOS, where the folder and the form of the
    commands differ.
- **The contract with blueprints is checked both ways** (built). A
  blueprint is held to what this program offers when it is applied: every
  action and argument, through everything it reaches, and every event a
  handler hears and what it reads of it. And what the program offers is
  held to what it offered before: `contract/offered.txt` is everything a
  blueprint can depend on, and a break is written down only by saying it is
  meant and why ([blueprints.md](blueprints.md), *Events*).
  - *Not built:* a package saying what its own program emits (see
    *Components*, below); what a step reads of another's result
    (`steps.<id>.out.<field>`), which is not held to what the action gives
    out, because many actions give out more than they list; the `where:`
    of a `wait`, and a handler on a timer or a poll.
- **Letting agents work in the factory is made as safe as it can be**
  (decided, and a continuous goal). The product is worth having only if
  a person can let agents operate in it without watching each step, so
  what an agent may do is narrow, plain and held by the daemon, never by
  the agent's good behaviour: it holds only the permissions it was given
  and never widens them or changes its own settings; its program works
  behind a wall; what it asks beyond its permissions waits for a person's
  go-ahead and is then done as the agent, credited to it and to who
  approved it; and everything it does is an event. Each new capability is
  asked who may use it before it is built. Stronger walls, permissions
  that are easier to read and to set, and fewer places an agent can reach
  without being given them are never finished
  ([security.md](working/security.md)).

- **Iteration stays fast** (leaning, and an ongoing goal). Rust makes
  building, compiling and testing slow unless that is worked at, and a slow
  loop slows every change made after it. So the times are measured on every
  change and held to `build-budget.toml` ([build-times.md](build-times.md)),
  and when one nears its limit the first answer is to make it faster, not to
  raise it. Splitting a crate into smaller ones is the quick way to shrink
  what a change rebuilds, and it has been used; but it is a bandage, and
  letting rebuild times decide the architecture would make the code worse to
  work in. So how the crates are divided and joined is expected to change
  as the product grows, by as little as it can, and each split has to make
  sense as a design as well as a build time.
  - *Not done:* the API suite, whose time grew from 40 s to 159 s between
    2026-11-20 and 2026-11-24 while the code grew by half, brought back
    under 120 s after 0.1.0.

## Blueprints and the conductor

What is built here is described in [blueprints.md](blueprints.md) and in
[architecture.md](architecture.md), *Instances and runs* and *How an event
reaches a handler*. What is left of each:

**Addressed delivery** (built). The conductor decides who an event is for:
a handler gives its address with `for:`, and an event wakes only the
handlers it is addressed to. The word for what is published to is *topic*,
since a channel is where people and agents talk. Sending to one instance
has no word of its own: the sender names who it is for in the event, and
the receiver's `for:` is its own key.

**A blueprint and the build that reads it** (built). Applying checks
everything a blueprint reaches, and a running instance goes on with the
last text that checked when one it calls no longer does; a project's own
file it runs only as somebody who holds what it runs with applied or saved
it.

- *Not built:* a line in a blueprint giving the version it needs. It would
  catch what the check cannot see, an action that keeps its shape and
  changes its meaning. It is worth adding once there are released versions
  to name.

**One way of laying a blueprint out** (built). One formatter, which the
editor, `fmt` in the terminal and the check on shipped blueprints all
call; a project's own settings in `blueprint-format.yaml`; and laying out
is never an update ([blueprints.md](blueprints.md), *Laying one out*).

- *Not built:* an instance's own stored text is not brought up to a new
  layout; blueprints inside a package's manifest or in Rust are not laid
  out; comments at the ends of neighbouring lines are not lined up; a long
  sentence or expression with no braces to open is not broken.

**How a running instance takes a text** (built). An instance says which
text it runs and whether it is behind the one now there. How it comes to
that text is one rule, whoever asks: the same text and params, nothing;
asked with force, its steps run again; a text that has `on_update`, on an
active instance, carried over; a text that begins as its own does, taken
where it is; otherwise its steps run again
([architecture.md](architecture.md), *Instances and runs*). `on_update`
is done in place of the steps and is handed what the instance was
(`was.version`, `was.state`, `was.params`), so that state of one shape is
made into state of another; a failure puts back the text and the state.

Two other shapes for `on_update` were turned down. A guard on each step
(only on the first apply) would be needed on every step, and one would be
missed. Running again only the steps that changed breaks a rollout, where
a build step whose text is the same must still run because the source is
not.

- *Open:* a called blueprint is read where it is each time it is called.
  Whether what an instance reaches should be fixed when it is applied is
  the *Pinning* design, below.
- *Not built:* an instance whose text no longer checks when the daemon
  starts is failed, and is not tried on the text it had before.
  `on_update` cannot read what the steps gave out, and a step a new text
  adds has given nothing. A run that never ends holds a carry up for as
  long, and all the while the instance hears nothing new.
- *Not built:* telling the client that a blueprint's text moved when
  nothing else happened. A file edited by hand with nothing run, and a
  standard or a package's blueprint replaced on disk, show only at the
  next read. Following the folders blueprints are read from would tell it.
- *Not built:* bringing the roots themselves up to date without being
  asked; an apply that answers with what it left behind below it.

**Tasks** (decided and built). A task is a group of steps with one policy:
a step whose one thing is `task:`, which succeeds or fails as one. Steps
were not renamed. The steps of a task are run by what runs a blueprint's
own, so `needs` works inside one and a restart picks it up at the step it
was on; what its steps gave out is read through the task; in a run a step
inside is `build.compile`.

- *Open:* drawing a task as a box around its steps in the node view.
- *Not built:* how a step inside went (`.failed`, `.skipped`) is not
  readable from outside the task.

**An agent when a step fails** (built). `if_fail_ask_agent: <agent>` with
`else: <code>` on a step: the agent is asked to put it right, and the step
fails under that name when it did not. It is `on_fail` with
`agent.consult` written short. No blueprint that ships uses it yet.

**Labels, and measurements side by side** (built). `labels:` on a `call`,
a `spawn` or a task; everything measured under the step carries them, and
the Measurements view compares by a label
([blueprints.md](blueprints.md), *Labels*). It is what experiments stand
on: the same work given to two agents on different models, two versions of
a deployment, two ways of handing out work.

- *Not built:* labels on a service's health checks and a router's
  connections, on a turn started by a message a labelled blueprint posted,
  and on a step sent to another device. Steps in two tasks have two names
  and are compared only as a whole; two calls of one blueprint step by
  step.

**Who decides which was better** (decided, not built). The blueprint names
who is asked, in order, and how long each is waited for: a person first,
and an agent if there is no answer in a stated time. It is a step, which
resolves to the choice that was made and to who made it, so that what
follows can act on the one and record the other. See *Experiments*, below.

**Profiles** (decided and built; called environments until they were
renamed before the first release). A profile is a name a
factory has, stamped into its data folder for good; a factory is the unit
that shares nothing; the name is not the factory, so several may one day
share one; a blueprint reads it as `profile.name` and its variables as
`profile.vars.<name>` ([profiles.md](profiles.md)). The word environment
is a project's own (`dev`, `staging`, `prod`): named sets of variables and
secrets, one of which an instance of the project is applied under, read
as `environment.name` and `environment.vars.<name>` (being built).

- *Not built:* promoting a blueprint from one profile to another; an
  address by name for each profile; anything that copies data between
  factories; and the hard line itself against what a factory runs, which
  on one device under one user needs another user or another device.

**Work queues** (leaning, not built). Several workers, and each message to
exactly one of them, will not be a kind of delivery in the runtime. It is
a broker: something that owns the queue and hands each message to one
entity that is free, by addressed delivery, written as a standard
blueprint. Two ways of handing work out can then be run side by side and
measured. The runtime's part stays getting an event to whoever it is
addressed to.

**Packages in blueprints** (leaning). Tools that describe infrastructure
answer the same question the same way, and the answer fits what a
blueprint is:

- *A plugin knows one product; the engine knows none.* Here, a package,
  named in a `requires:` header of a blueprint, so that applying it
  without the package fails saying so.
- *The same kind of thing is held in two ways:* made and owned (`call:` or
  `spawn:`, and the instance holds it), or already there and only read.
  The second is missing: a blueprint cannot yet read what an instance it
  did not start gave out.
- *A module gives out what a consumer takes in.* A database blueprint's
  outputs say how to reach it; a board's blueprint takes a store and
  cannot tell whether it was made for it or was there before. Whoever
  applies decides.
- *What was made is unmade with what made it; what was only referred to
  never is.*

The package says how its product is run and nothing else. How it is rolled
out, logged and watched stays in blueprints.

## The client

**Decided: the client is a workbench**, not a page with a settings screen.
It holds many kinds of view, and people arrange them. Anything that would
crowd Settings gets a view of its own on the rail.

Built ([architecture.md](architecture.md), *The client*): a rail that
chooses what the side panel shows; tabs in groups that split, join, resize
and are remembered, a set of them for each workspace; the Factory (fleet, blueprints, activity,
measurements), which is the whole installation, and the Projects, each
with what it has and not what its folder has (its Board once a blueprint
has made one, its tables, the Blueprints it uses, and its Repo where its
folder is one somebody said); the factory's
views of the workspace the side panel is of, narrowed to one project of it or widened to every workspace by one choice; a page for each project about
how it stands; what a blueprint is applied with chosen and not typed; a
board changed from the board, through its blueprint; the keys the client
answers to in one list; and on a phone, one view at a time, by touch.

- *Not built:* one document in two groups at once; a group in a window of
  its own; arranging from the keyboard; views that reflow for a narrow
  group; unsaved edits surviving a reload of the page.
- *Not built:* what a project's blueprints reach without an instance of it
  being alive just then, in the project's list; removing a blueprint from
  a project's list other than by stopping what runs it.
- *Not for a phone:* tabs side by side, writing blueprints and code (they
  open and can be read), and carrying anything from one place to another.
- *Tried and withdrawn:* a Fleet, an Activity and a Measurements entry
  under every project. It put the factory's things at the level of a
  project's files and repeated three times what the factory already shows.

**Decided and built: what a second factory is.** A second daemon on a
second data folder, with its own database, accounts, credentials and
address. What ties a set of devices to one factory is the hub and its
members.

### No editor is adopted (leaning)

The question was whether to build on a code editor. The leaning is no:

- The two things wanted most, writing in blocks and tables with views, do
  not exist in a code editor. They would be built anyway, inside a much
  heavier host.
- What an editor would give (code editing, diffs) can be embedded as
  parts.
- Its extension ecosystem is not wanted; this product needs one of its
  own.
- Being lightweight matters.

It would be the right choice only if the product had to be a code editor
itself: language servers, debugging, a full interface to source control.

### Writing (leaning, with a first version built)

A Markdown file in a project opens three ways: **Write** (blocks),
**Source** (the file as it is) and **Page** (what publishing would
produce).

- **Documents are files** (Markdown) in the project, not rows in a block
  database. Agents, source control and diffs then work on them as they
  are. The cost is that a table inside a page gets harder. This choice
  matters more than the choice of editor.
- Markdown alone is not enough for pages (callouts, columns, captions,
  embeds), so: Markdown plus a small, closed set of extra blocks, checked
  the way blueprints are.
- The standard for how writing should feel is the best block editors. A
  good four fifths of that comes quickly; the last of the polish (handles
  to drag a block by, nested blocks, flawless paste) is long work.
- *Not built:* handles to drag a block by, a toolbar on selected text,
  links and images from the menu, nested blocks beyond lists, and tables.
  A document with a table, raw HTML or a footnote is not opened as blocks,
  because writing it back would change it; it opens as its source.
- The libraries this would be built on are to be checked for size and
  licence before one is committed to.

## Tables

**Decided as the aim:** a board is not a part of the application that a
blueprint configures. It is what comes of giving a blueprint to the
conductor: the blueprint declares what the board's records are and where
they are kept, and where a database has to be set up first, sets it up.
The same holds for any component of the kind. A project has no board
until a blueprint makes one.

Built ([blueprints.md](blueprints.md), *Built-in actions*;
[stores.md](stores.md)):

- **A collection**: typed records a blueprint declares, with its fields,
  its views and where it is kept (this installation's database, files in
  the project, or a store service a blueprint hosts). The board is the
  collection called `board`, seen by its `status`.
- **Views** of the same records under names: a table, a board, a list, a
  gallery, a calendar. They are kept with the table, for everyone.
- **A record's history**, declared by the blueprint as the events that
  count and the sentence for each; a line once written is never changed.
- **What the cards of a board can do** (`std/cards`), one instance for a
  board: a card is a row, and nothing runs for one nobody is working.
- **What comes after review:** a card sent back goes round again with
  what was written as it was moved; an agent that is unsure asks on the
  card instead of guessing, and the card waits for an answer.
- **What a card can do is its blueprint's to supply.** What is general is
  a primitive on any record (`record.comment`: something written on it,
  kept in its history). What a question is, who may answer it and what
  follows is text, in `std/cards`, over a field `std/board` declares.
- **A field that names a record** of another collection, checked when it
  is declared and when it is written; taking the other record away is not
  refused, and the field then shows that what it named is gone.
- **Records of any collection** made and changed by blueprints, over HTTP
  and from the terminal.
- **A collection declared again does not freeze its records**: a field
  taken away is left out when read; a change its records would not fit is
  refused, with how many records and what to do first.
- **Core models.** Where the daemon's own code reads a record's fields by
  name, the fields and the struct it reads them through are written once
  (`studio::models`; today, the card). A blueprint that names the model
  writes its fields out and is held to them when it is applied.

Two ways of having that were turned down. *Generating the Rust from the
blueprints:* a step in the build can only read the blueprints that exist
when the daemon is built, and a model has to hold against one written
afterwards, so the check at apply is needed whatever is generated; and it
would rebuild the daemon for a field its code never reads. *Keeping a Rust
file and a blueprint in step both ways:* the two do not hold the same
things (several kinds of field are all a string in Rust, and a blueprint's
expressions have no Rust form), a tool would be rewriting a file that is
mostly comments and expressions, and two copies that may both be edited
need a winner.

Not built:

- **Of mockups** ([mockups.md](mockups.md), built: `std/mockups` makes
  the folders of a project's folder that each hold a page its mockups;
  they are listed under the project beside its Board and its Docs as a
  gallery of small live pages, variants beside what they vary; one is
  opened at the width of a phone, a tablet or the room there is, alone or
  beside another, spoken on and moved through a few stages; an agent
  asked for one leaves one; each has an address of its own for whoever is
  signed in; and a mockup's scripts run in a place of their own, where
  they reach its own files and nothing of the daemon's): measuring two
  variants against each other with visitors, which is the experiments
  design below; publishing one to the web; carrying one from a
  profile to another by itself; a visual editor; following a folder
  that is renamed; a way in that is told to nobody, even the browser's
  history.
- **Of a library** ([library.md](library.md), built: `std/docs` makes a
  project's folder of Markdown files a collection, taking the files as
  they are; it is listed under the project beside its Board, each document
  opened in the editor or as a page, spoken on, and the whole served as
  pages to whoever may read the project, and written out as plain files
  by `std/site`): hosting what is written out (see *Publishing*); a page
  template of a project's own; following a file that is moved, which is
  another document where it then is; a library kept anywhere but in a
  project's files; and listing one without reading every document in it.
- The other layouts: timeline, chart, feed and map.
- Of a table's columns (built: a column added, called something else,
  made another kind where every row fits and deleted from the table's
  page, by whoever holds `columns`, kept beside what the blueprint
  declares; a view held to some of its rows, gathered by a column,
  with a figure at a column's foot, frozen up to one, and its columns
  left out and put in order, each saved with the view): converting
  what rows hold as a column is made another kind; changing a column a
  blueprint declares anywhere but in the blueprint; a column added to
  a library or to mockups; and sorting kept with a view, which is the
  reader's own and kept in their browser.
- Taking a collection away: stopping the instance that holds one leaves
  it and its records where they are.
- Choosing a record by typing where there are too many to read down;
  anything happening to the records that name one that is taken away.
  (A field holds several records of one table, `records`, and a row is
  opened into the rows of the tables that name it.)
- A store for another database (a blueprint that hosts the server and an
  adapter that speaks the protocol); moving a table between places; a
  blueprint reading the address of a store it did not start; a credential
  on the protocol.
- Anything about a pull request after it is opened; a lane hearing of a card that was sent back
  while no lane for it was alive.
- A person asking on a card (they can only answer). (A record that is
  not a card is spoken on in the client too: a document on its page, a
  mockup in its drawer, a row of any other table that keeps a history
  where the row is opened.)
- One handler hearing two topics as one stream. `std/cards` hears a
  question and what is written on a card by two handlers, which take
  turns by the card but in no settled order, so something written in the
  instant a question is asked may not be taken for the answer.
- History for anything but a record; keeping a history elsewhere than
  this installation's database; a blueprint naming which field of an
  event is the record.
- Checking what a blueprint's expressions read of a model when it is
  applied; and writing out the types of a declared collection for
  someone's own program that reads it (on demand, one way, and nothing
  the daemon depends on).

**A card that moves while an agent is working on it** (decided as the
aim, not built). The agent is told, and what it then does is the
blueprint's to choose: finish, stop and wait, or clear everything away.
See *Interrupting*, below.

## Files and source control

**Decided:**

- **People should be able to make their own source control.** It is one of
  the more ambitious aims. So nothing in the application may assume one: a
  project is a folder of files, and what tracks their history is something
  added to it.
- **A layer for git in the application is acceptable.** It is one way of
  keeping history, the common one, and would be the first such layer.
- **A hosting service for source code is an extension.** It is somebody's
  product; the application knows none by name, exactly as it knows no
  agent by name. The `providers` gate holds it.
- **Working with git is a high priority** (as the README's *Product
  Vision* says). Most of what a hosted platform for source is used for
  (reading a change and its history, diffs, review and merging, the
  checks and pipelines run on it, the board it belongs to) is to be done
  well here, locally, beside the agents doing the work; the hosting
  service stays where code is shared. With it goes weight on local
  infrastructure for testing a change and for research, before anything
  reaches a cloud. This does not undo the first point: git is the first
  layer of history, not an assumption.

Built: browsing a project's files with no source control at all.

- *Not built:* anything about history. Its shape is an extension point: a
  package says how history is read for a project (what changed, who
  changed it, what a file was at some point), and the browser shows what
  it is told.
- *Not built:* a link to a line of a file, searching within a project from
  its page, creating or deleting files from the client.
- *Leaning:* diffs with the merge view the client's code editor already
  has, and a view of changes on the rail.
- *Design:* a lane that names no program, under *Source control*, below.

## Publishing

**Decided that it is wanted; the design is a leaning.** Some documents and
tables are published as web pages, and what is seen in the document is
what the page shows.

- Pages are built on one's own machine into static files. Whoever hosts
  them only stores and serves files.
- One document, two renderings: one stylesheet serves both, and every
  document's Page view is exactly what would be published. The Page view
  is built, with the code that writes a page out.
- A collection with a page template is a blog or a documentation site: an
  index page and a page for each record. This is where tables and
  publishing meet. A library of documents is served so by the daemon, to
  those who may read the project, and is written out as plain files
  ([library.md](library.md)).
- Publishing is explicit, for each document, and fails closed: nothing is
  written out that was not marked so (`public: true` in a document's own
  settings), and a link to what is not written is its words. **Built**
  for a library. Not built: a list, before anything is written, of what
  is new, changed and removed.
- It is a blueprint: build again when a published document changes, and
  measure it. **Built** (`std/site`, over the action `library.export`).
  Not built: deploying what was built anywhere.
- The application exports the same static files for hosting anywhere.
  **Built**: a folder of files that works at any address, under a path of
  one, and from a disk; nothing in it runs and nothing is fetched from
  elsewhere.
- Not part of the application: hosting. Where the files are served from
  is a host of one's own choosing; publishing to the web for you is one of
  the paid hosted services (*The product, and what is paid*, above).
- Not built, of writing a library out: a page template or a look of a
  project's own; searching within what is written; a check of what is
  written against names that must not appear in it; a map of it or a
  feed; a table of anything but documents.
- *Open:* how bespoke pages must be. Documentation and blog pages fit
  blocks; a landing page with a layout of its own needs templates beyond
  them.

## Extensions

**Decided:** the application knows no agent by name; each comes as a
package ([extensions.md](extensions.md)).

Built: a package brings an agent, blueprints, actions, or several. Its
blueprints are in the library as `<package>/<name>`; its actions are each
a way its program is run, with no shell, the answer read as JSON. Two
packages that are not agents ship: one opens a pull request for each
branch a project's lanes send to its remote, and one runs a program in a
container of its own as a service of the factory.

**Leaning:** grow the same format, declaratively, to bring more: table
field types and views, panels on the rail. Anything that draws its own
interface is sandboxed.

Still open:

- *An action that is not one run of a program:* talking to a service over
  the network, or keeping something running. See *Components*, below.
- *A package's blueprints reading what another instance gave out.*
- *`requires:`* in a blueprint that calls one from a package.
- *Whether removing a package should offer to stop what was applied from
  it.* It leaves them running, as it must.

## Where work runs, and what an agent signs in with

**Leaning: a place to run in, declared by a package.** Containers, small
virtual machines and the like are wanted, and are kept out of the
application as agents are. The application knows a place by name only; a
package says how its program enters the place, runs a command in it and
leaves. A place reaches the daemon and the network; what matters is that
the credentials it needs are the slot's own. The design is
[places.md](places.md).

**Built of it: the system's own wall around an agent's program.** On
Linux an agent's program is started behind a wall that bubblewrap holds,
unless its slot says otherwise: it is shown its work, a home of its own,
what it signs in with and what its slot adds, and nothing else of the
device ([architecture.md](architecture.md), *Walls*; `mw help wall`). The
application asks for that one itself, since every Linux system has the
means and nothing has to be installed from a package. Still only designed:
a place a package declares, a `shell` step or a service behind one, a way
to the daemon that needs no network, and any system but Linux.

**Decided and built: an agent's sign-in to its provider is its slot's,
and is shared out by a rule that can be changed.** With nothing set, an
agent's program signs in as whoever runs the daemon. A slot may instead
draw, in order, from the sign-ins the factory holds: a subscription, or a
key. Each is held by one turn at a time unless it is given more; at a
limit it is set aside until it is good again, and the work that met the
limit begins again on the next. Which one is handed out is a blueprint
(`std/signin`), asked and not run. A key is never in an event, a
measurement, a log, an argument, the database or a copy of a factory
([extensions.md](extensions.md); `mw help signin`).

What an earlier system taught, and what was taken from it:

- It chose a sign-in for a device and not for a piece of work. *Choose
  when a piece of work starts.*
- One sign-in used by several programs at once was where work was lost:
  each program renews the sign-in for itself and puts the others out. *A
  sign-in is held by one running program at a time, and is never copied
  to another device.*
- How much of each was used was never known for certain. *Count what each
  sign-in was used for from the turns the journal already keeps.* Not
  built: each turn records which sign-in it was handed, and nothing adds
  them up yet.
- At a limit the agent slept until the limit lifted. *At a limit the
  sign-in is set aside, and the next piece of work starts on another.*
- A subscription is, on a reading of the providers' terms that should be
  made again before it is relied on, for one person's ordinary use of the
  provider's own program. One at a time on a subscription is therefore
  the default as well as the cure for the fault above.

Still to decide or do:

- What the packages kept here state about their programs has not been run
  against them, and none says how a limit shows. Until one does, a limit
  is a failed turn like any other.
- A conversation is carried on only under the sign-in it was held under,
  so with sign-ins used in turn an agent's next piece of work in a thread
  usually begins the conversation again. The rule can keep an agent with
  its conversation's sign-in instead; which should be the default is open.
- Nothing in the client. Asking a program for its models is still done as
  whoever runs the daemon.

**Not built: holding an agent's program in.** The daemon holds an agent
to its slot's scopes in what it asks of the daemon. What the program does
on the machine is done as whoever runs the daemon, and a permission mode
is a request to that program and not a wall. A place to run in is what
would hold it.

**Not built, and to be kept in mind before any of it is leaned on:**

- **Sharing anything but devices between installations.** A device and
  its hub prove a key to each other and what they say is sealed
  ([architecture.md](architecture.md), *More than one device*), and a
  device is let in by an invitation that works once. That is all that
  crosses between two installations today: no channel, project or person
  is shared.
- **A signature on an invitation.** One is redeemed at the hub that made
  it, which knows its own; a signature is for something else to check
  one, and nothing else does.
- **Reaching an installation that cannot be reached directly** is not
  part of this application and will not be: where two installations have
  no address either can call, getting them to each other is a hosted
  service outside it, as is finding an installation by who runs it, or
  inviting someone by email. What the application holds is links whose
  contents nothing in between can read, so that whatever carries them
  need not be trusted.
- **A scope for one project.** A scope is for the whole factory: an agent
  whose slot is in one project can ask about another.
- **A credential for a store service.** One answers anyone on the machine
  it runs on ([stores.md](stores.md)).
- **A first account that only its owner can make on a machine others
  use.** It is made from the machine itself; who on that machine is not
  told apart.

## Other people's services

**Decided as rules to build against.** The application is open, so anyone
may run a service of their own for it to use (a directory, a publishing
host, something that carries a link). What must hold:

- **A service is trusted only for what it issued itself, and only at its
  own address.**
- Credentials never travel: what one service issued is sent only to that
  service. The application keeps them apart for each service.
- A service configures only itself. It cannot change what the application
  holds for another.
- Nothing that carries a link is trusted: installations know each other by
  their own keys and what they say is sealed between them (built, for a
  device and its hub), so what is in between can delay or drop and nothing
  else.
- The application shows which provider it is using, so that none can pass
  for another.
- To begin with a provider is all or nothing for an installation. Mixing
  one party's service with another's is where the subtle cases are.

Limits (storage, traffic, how often) are enforced by a service, never by
the application: anyone can take a limit out of an open application. The
application shows the meter.

## Channels

Built ([architecture.md](architecture.md), *Agents* and *The client*;
[views.md](views.md)): a channel takes people, agents and bots; a bot is a
blueprint; threads; attachments; a page attached to a message drawn where
the message is, in a box with no origin of its own; marks in a message
drawn as what they mean; a tools panel beside a channel; three themes.

**Keeping a conversation going without naming the agent every time** was
the reason for threads. The other ways, considered and not built:

- *The agent decides whether a message is for it.* Every message then
  takes a model call for every agent in the channel, answers arrive
  unbidden, and silence cannot be told from a mistake. If wanted,
  it is an option on one agent, with a small model doing the deciding;
  never the default.
- *Whoever was spoken to last, for a few minutes.* Cheap, but invisible:
  nothing on the screen shows whom the next message will go to. A thread
  does.
- *A channel of one's own with one agent.* Already how it works: with one
  agent in a channel, everything a person writes there is for it.

Not built: taking a member out of a channel from the client; a bot that is
a program of its own with its own credential, outside any blueprint;
anything happening when a person is mentioned; a bot following a thread
unnamed; a view that asks the daemon for what is happening now; links
opening from the desktop window; media from before what a channel has
loaded; a tools panel's tabs in another order, or a tab of the reader's
own making.

## Teams

**Decided:** working as a team across machines is a paid hosted service,
and the application works fully without it. **Open:** its shape. What is
built is several accounts on one installation. The
other shape: everyone runs their own installation and keeps their own
data; a channel lives on one host installation that orders its messages;
connecting two installations is their own business, and a service may
help them find and reach each other. The second fits the rule that
nothing leaves the machine far better, and would change how channels and
messages are identified. It is to be settled before more is built on
teams.

## Signing in

**Built.** No registration form on one's own machine: the daemon's first
start makes the factory's owner and signs in the terminal that started
it, and `mw open` signs a browser in with a link that works once. A
password is set later, for signing in from elsewhere. Other accounts
appear when someone is invited ([first-run.md](first-run.md), step 1).

## Licence

**Decided:** MIT or Apache 2.0, as whoever uses it chooses
(`LICENSE-MIT`, `LICENSE-APACHE`).

## Designs

Six proposals, none built. Each is what it is for, what is proposed, and
what is still open.

### Experiments

*Running two things against each other.* One mechanism, applied to two
components that answer requests and to two ways of handling a card (a
fleet's configuration, or the model an agent runs on). A canary is the
same mechanism with a small share and a verdict nobody has to give.

What is there: the router splits connections by weight and counts them;
`labels:` mark what a blueprint starts; steps, runs and agent turns are
measured and compared by a label. What is missing is the same on both
sides: nothing assigns a piece of work or a caller to a side and keeps it
there; a blueprint cannot record a measure of its own or read measures
back; nothing asks who decides; and the router sees a connection, not a
request.

Proposed, in six parts:

1. **Assign.** Given a key and the shares, which side the key is on, the
   same every time; as shares grow, keys only move from the stable side
   to the candidate. An action, `experiment.assign`, that keeps what it
   assigned and emits it, so that it can be seen and survives a restart.
2. **Label.** As now; and a router's backend carries its labels too.
3. **Measure.** `measure`, an action by which a blueprint records a number
   of its own, so that `std/cards` can record, as a card is done, the time
   from ready to review and to done, how often it was sent back, how many
   questions were asked and whether it was approved the first time. And
   the router times requests.
4. **Judge.** `experiment.compare` reads measures back, split by a label
   over a window, and holds them to a bar the blueprint states. With too
   few of either side it reports that and gives no winner.
5. **Decide.** The step under *Who decides which was better*, above.
6. **Promote, or go back.** Text over what exists.

```yaml
side:
  do: experiment.assign
  with:
    experiment: "lanes-${instance.project}"
    key: "${event.data.id}"
    shares: { stable: 90, candidate: 10 }
verdict:
  do: experiment.compare
  with: { measure: card.cycle, by: variant, since: 7d, at_least: 20, better_when: lower }
```

For two components the router gains a mode that reads requests
(`mode: http`): the same caller kept on the same side, by a header, a
cookie or where the connection came from; and **mirroring**, where the
stable side answers and the candidate is sent a copy whose answer is
measured and thrown away. Mirroring is the fairest comparison and the one
a small team can afford, since it needs no share of real traffic. Later:
the requests of an hour kept and run against a new version.

What the evidence is worth: pieces of work are not alike, so two sides
given different ones are compared across their difficulty too. The
stronger form gives the same piece to both, which is twice the work and needs
someone to decide which result is better.

Open: assigning as an action that keeps what it assigned, or a function
in an expression that only computes; one router with a mode, or a second
primitive; the names; the order (proposed: `measure` and the measures of
a card first, then assign, compare, decide, the router's mode, mirroring).

### Components

*A program of one's own, with a blueprint as its kit.* See *Components*
under *What it is for*.

What is there: a package's actions each start its program; a service is
hosted, watched and rolled out; one component exists, wired by hand (the
store service). What is missing: an action that is a request to a running
service; a component reporting that something happened; the shape of an
event that is not the daemon's; a request from outside as a trigger; a
project's own packages.

Proposed:

1. **A package states that it runs for long, and that its actions are
   requests to it.** `service:` in its manifest, and `request:` on an
   action in place of `args:`. Everything else an action has stays.
2. **A component reports what happened by printing it.** The manifest
   declares its `events:` and what each carries; the program prints one
   line of JSON; the daemon, which already follows what a service prints,
   puts a declared event in the journal as the service's own. No client
   library, address or credential, and the same in every language.
3. **Declared events are checked like the daemon's**, in the same catalog.
4. **A request from outside as an event.** A blueprint holds a hook: a
   path the daemon answers at, a secret, and the topic a request there
   becomes.
5. **A project's own packages**, kept in its repository and there for
   that project only.
6. **A helper for Rust, later and optional**, whose main work is writing
   the manifest's actions and events from Rust types, so that the
   contract is not written twice.

```yaml
program: inventory
service: { args: [serve], health: { http: /healthz } }
actions:
  reserve:
    with: { sku: { type: string, required: true }, count: { type: int, default: 1 } }
    request: POST /reserve
    answer: json
events:
  inventory.low:
    data: { sku: { type: string }, left: { type: int } }
```

A component's actions are for decisions: each is a step, written in the
journal. What it does a thousand times a second goes to it through the
router.

Open: whether a component is declared in a package's manifest or in the
blueprint that hosts it; whether it reports by a line it prints or by a
request with a credential of its own; the order (proposed: requests to a
running service first, then events, then a project's own packages, then
hooks).

### Fleets

*Who does what to a card.* A fleet configuration is a blueprint; a
worker, a reviewer and a planner is one configuration, and two
configurations, or two models, are compared as *Experiments* describes.

What an earlier system taught: workers and reviewers that looked for work
on a timer left work reserved for agents that had stopped; several
reviewers on one change turned each vote into a round of rework; a role
with nobody in it collected messages nobody read; and roles written into
the code in several places could not be changed. Here a card is handed to
a lane and nothing asks for work, and the application should know no role
by name: a role is a word in a fleet's text.

Built: the fleet that ships is `std/fleet`, and what it is made of is two
tables of its project, changed by changing a row: who is in it (`agents`,
a row for so many agents of one role) and what they do with a card
(`stages`, a row for a column and the role that has the cards there).
Those who review are a council, of one where there is one. A card counts
how often it was sent back, and at its stage's limit it is left for a
person. An agent that plans writes a document whose tickets become
cards, each saying what it comes after, and a card is held until that is
done. Which of a role's agents a card or a seat goes to is the stage's
row to say (the first, each in turn, the least used, the highest ranked),
and a card may name its agent; a stage may plan the cards of its column
before anybody works them (an agent of its role asks what it needs to
know, or says how the card would be done, and has the card while it is
there); what each agent chose and what each stage
came to are measured, and what is made of the figures is left to whoever
asks. [blueprints.md](blueprints.md), *The fleet that ships*, has all of
it.

Work that takes several cards is a feature (`std/features`): a board of
its own, each feature planned by an agent, accepted by a person, come to
cards that wait on each other, and done when they are. Which of a role's
agents work goes to is a rule that is asked (`std/dispatch`), which a
project replaces with its own.

Open: a rule that has work wait (for an agent that is busy, a time of
day, a budget) and not only choose; a feature whose plan is looked over
by agents before a person accepts it; a second stage that works a card (one agent changing what another
made, on the same branch); which card is taken next, by a rule of the
stage's own, where today it is the one whose turn came first; a way of
choosing an agent that is none of the four, said in a row and not in a
blueprint; a card's journey measured for each arrangement, so that two
can be compared; and none of it has been run with real agents in every
role.

### Source control

*A lane that names no program.* `std/lane-round` runs no
script, and in its place has sixteen steps that each run one program
with arguments and conditions on what it answered. That is a ladder of
conditions about something that never changes, and making one's own
source control means copying the lane.

Proposed: four actions in the application (`repo.place`, somewhere of its
own for a card to be worked on; `repo.keep`, what was changed as a commit
in the agent's name; `repo.send`, the branch to the remote; `repo.tidy`,
the place and the branch taken away when nothing would be lost), and the
lane taking the names of the blueprints that do the four, with the
application's own as the defaults. Someone's own source control is four
blueprints, or a package that brings them. A package for git itself
cannot be done as the rules stand: a package's name may appear nowhere
else, and the application rightly knows what a `.git` folder is.

Open: the four as places a project fills in, or the actions alone; the
names.

### Pinning

*What a running instance calls, and when that changes.* An instance runs
the text it was applied with, but what it calls is read when the step
runs. For the standard blueprints of an installed factory nothing can
change under it. For a project's own blueprints it can: they are files in
its repository, and a merged branch changes them under everything that is
running. A change nobody applied was not taken; it happened. And an
experiment's evidence is only worth something if neither side changed
while it ran.

Proposed: what an instance reaches by a fixed name is kept by revision
when it is applied, and a `call` or `spawn` runs that revision. An
instance is behind when something it reaches has moved, and applying it
again takes the new one in place. A name with an expression in it goes on
being read when it is called. What is lost is that editing a called
blueprint takes effect by itself; a factory for development may apply a
saved blueprint at once, which is the same convenience, turned on
deliberately.

Open: pin, or leave calls reading the text that is there; whether a
development factory applies on save.

### Interrupting

*A card that moves while an agent is working on it.* Today a move during
a round waits for the round to end, and the agent is told nothing while
it works.

Proposed: the choice goes in the card's own blueprint, which is never
busy for long and hears every move of its card at once, as a param:

| | The turn under way | Then |
|---|---|---|
| `finish` | goes on to its end | the lane goes round again with what was written (as now) |
| `tell` | goes on | what was written is waiting for the agent at its next turn |
| `stop` | is stopped, with why | the lane goes round again at once |
| `clear` | is stopped | the lane is stopped and its place tidied; the card stays where it was put |

It needs three things that are not there: stopping one turn; a note kept
for an agent's next turn without asking it for work; and a round that was
stopped being told from one that failed.

Open: the four choices and which is the default; whether the choice may
differ by where the card was moved to.
