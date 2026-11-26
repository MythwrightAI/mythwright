<!-- public -->

# Releases

What each release of Mythwright is, the newest first, after what is in
none yet (*Unreleased*). A release is a heading of the second level in
this file, and the client's list of releases under Help is read from
those headings.

## Unreleased

What is on the line since 0.1.0 and in no release yet. It becomes the
next release's notes when that is made.

### Trackers

- A project's cards can be kept in step with a Jira project's issues
  (`jira/issues`) and with a Notion database's pages
  (`notion-pages/pages`), a linked ticket at a time (its title, its body,
  whether it is done and what is said on it), as they are with a
  repository's issues on GitHub (`github/issues`). The two new packages
  come with the program and ask the service's web API with curl 8.3 or
  later. They have been played against stand-ins written from the
  services' documentation, and not yet against the services.
- How a package is written for a tracker nobody ships one for, with an
  example package to copy (`examples/tracker`), is *Keeping a tracker in
  step* in the guide to extension packages.

## 0.1.0: a preview

The first version, and a preview. Mythwright is a software factory you
run yourself, for small teams to build and maintain large projects
with agents: people and
agents talk in channels and work from a board, and what happens to the
work is written in blueprints, short files a daemon runs, keeps running
across restarts and takes changes to while it runs. It runs on your
machine and sends nothing anywhere by itself: no telemetry and no
update checks.

It is early. It is tested on every change on Linux. What is run by hand,
and what is not held yet, is said in `docs/status.md` of its repository, and
in short under *Known limits* below. Nothing is promised yet about carrying data, or
blueprints you write, from this version to the next.

### Getting started

- The package for a system is a folder to unpack: nothing is built or
  installed. Its `README.md` says how to start it on that system, and
  `FIRST-RUN.md` beside it is the first hour, step by step.
- `mw up` starts the daemon and keeps it running, and `mw open` opens
  the client in a browser, signed in. A second `mw up` for a factory
  that is running says where it is and how to stop it.
- Nobody registers to begin. The first start of a factory makes its
  owner, called Unnamed until they say what they are called, with no
  email and no password, and signs that terminal in; `mw open` asks for a link
  that signs the browser in once, within a minute. The address alone
  says to run it, and signs in an account that has a password: the
  owner sets one on their Profile (or `mw register`, from any terminal
  on the machine) to sign in from another device, and is named there too,
  their workspace with them, and anybody else registers with an invitation. A terminal
  signed in as nobody is signed in as the owner by proving the machine
  can read the factory's data folder (`mw login --here`), and so is the
  desktop app's window, which opens signed in.
- The client's Setup Guide walks the first run and, once its steps are
  done, says what to try next. Help, at the foot of the side panel,
  leads back to the guide and holds these notes.
- Everything the client does is a command too: `mw help` lists them,
  and `mw help <command>` says what one does.
- There are two lines of builds: pinned releases (`0.1.0`, and its
  patch versions as fixes come), and a nightly built from main when the
  gate passes there, which says which night and commit it is in its
  version (`0.1.0-nightly.20261124+abc1234`, in `mw version`). Nothing
  updates by itself; you move between them by hand, and going back to an
  earlier version takes a copy of the data folder made before the newer
  one first ran (the README, *Which build*).
- A built-in assistant needs no model and no network. It answers in
  channels and sets things up, so everything but the work on a card can
  be walked through before an agent is installed.
- The mark at the foot of the client's rail opens a page about
  Mythwright itself: what it is for, its guides read in place, and what
  is technical of it (the version, the client's build, the system, the
  packages installed and what to say in a report, with where its data is
  kept for whoever manages the factory). All of it came with the
  program, and nothing is fetched to show it.

### Projects and workspaces

- A project is a folder with a board, a fleet of agents and a library
  of its documents, each given as the project is added or left out by
  name (`mw project create`). In the client each is a switch called by
  its name (Board, Fleet, Docs) in the dialog that adds a project, which
  makes it in the workspace the side panel is of and says so in its
  head; it is moved to another afterwards from its row. What each is
  set up with (the board's columns, the fleet's first settings, the
  library's folder) is under its row of that dialog, folded, drawn from
  its blueprint's own settings.
- Setting a project up is a blueprint, `std/project`, applied for it as
  whoever adds it, whose rows are what it starts with: what ships is its
  usual rows. A row of a blueprint of your own is added under the
  table in that dialog, and the table is saved as a setup with a name
  (`a research project`), chosen at the top of the dialog the next time,
  or by whoever manages the factory as its default for new projects
  (`/api/setups`, `mw project create <name> --setup <name>`).
- A project may start with its environments, `dev` and `prod` to begin
  with (`mw project create <name> --with environments`, or the switch
  in the dialog); it is off unless asked for, since everything applied
  for a project with environments says which it runs under.
- A project's folder is one the factory makes, or one you name that is
  already there. Its files are read and written in the client.
- A factory has a workspace from the first, called after its owner once
  they are named (Unnamed workspace until then), and
  every project is in exactly one. A workspace gathers the projects
  that belong together and their channels, and changes nothing of who
  may read what. A card is given to an agent of its own workspace, or
  to one that belongs to no project, and to no agent of another
  workspace's project.
- The activity, the measurements and the agents are of the workspace
  the side panel is of: all of it, one project of it, or every
  workspace, which has what was applied for no project too.
- In a browser the browser's tabs are the client's: one thing shows at
  a time, opened in place, the address bar saying it, and Back and
  Forward go between what was shown, each as you left it, with what was
  half written still in it. A line of the side panel pressed with Ctrl
  or ⌘, by the middle button, or "Open in a new tab" in its menu opens
  it in a tab of the browser's own; leaving something with changes not
  saved asks first. Every page has an address, settings, a channel and
  Activity among them. What was lately opened and what is pinned are at
  the top of the side panel.
- In the desktop app's window the client keeps tabs of its own, split
  side by side, and each workspace its own open tabs: choosing another
  in the side panel shows what was open there, and going back finds
  what you left, with what was half written still in it.
- A library is a folder of Markdown files: written in the client,
  commented on, read as pages, and written out as a folder of pages for
  a host of your own.
- Mockups are pages to try an idea with: a gallery, two side by side,
  one at a phone's width.
- Part of a project can be kept private: a table read only by those it
  is given to.
- A project may say its environments, such as `dev`, `staging` and
  `prod`, each with variables, and which of them an agent working there is
  handed (`mw project environment`), and secrets of its own, set by name
  and never shown (`mw secret set <name> --project P --environment E`).
  What is applied for such a project is applied under one of them
  (`mw apply … --environment E`), which it reads as `environment.name`
  and `environment.vars`, its programs are told as `FACTORY_ENVIRONMENT`,
  and whose secrets it is handed in place of the factory's. An agent
  working under one is handed what of it is marked for agents. In the
  client they are a page of the project's, opened from the gear of its
  page, and applying a blueprint for such a project asks which.
- A project may keep feature flags (`--with flags`): a row for each, what
  it says in each environment written into that environment's variables
  as `FLAG_<NAME>`, which an app reads as any setting, with nothing
  fetched and nothing sent. Turning one on where the environment's rule
  says who approves waits for one of them to let it go on; turning one
  off never waits. A flag past its day is told to whose it is, and a
  percentage flag can be started as a comparison on real work
  (`mw flag`, and the Flags part of a project's Environments page).
- Settings, Experiments lists what of Mythwright itself may be turned on
  for a factory before it is whole, each naming the card it is for, off
  unless whoever manages the factory turns it on. None is listed in this
  version.
- What an environment is loaded as says what becomes of it, as events
  any blueprint can hear: ready, with where it is reached and the cards
  it holds as its blueprint gives them out; failed, with why; and taken
  down once it was ready (`project.environment.ready`, `.failed`,
  `.down`). How long each load took, and how long it was up, are
  measured by project, environment and branch.
- A project deleted for good, from the Trash or at once where the
  factory keeps nothing that is deleted, takes with it its tables with
  their records and what was said of each, what of it was in the
  Trash, what was applied for it and still ran, and its agents, each
  taken away as `mw agent remove` takes one. A project made again under
  its name finds none of it. Whoever deletes it is asked first what
  stopping what runs for it (`blueprints:run`) and taking its agents
  away (a person holding `agents:write` and `admin`) would ask, and is
  refused, with nothing deleted, where they do not hold it.
- Somebody may be given a scope beyond what their role holds, such as
  setting a project's environments' secrets, by whoever manages the
  factory and holds it: from Settings, People, or `mw give <email>
  <scope>`, and taken away with `--not`, from their keys too.
- People and agents may be put in a group, which holds what its members
  may ask beside what each holds itself, by area: made and changed by
  whoever manages the factory from Settings, Groups, or `mw group`, never
  by an agent that is in it, and giving nobody what whoever put them
  there does not hold. Each member holds what the group holds from their
  next request; somebody who holds less by a group holds less at once,
  with the keys they made.
- A key for a script is revoked from Settings, Credentials, or `mw key
  revoke <id>`, which first says what goes with it, and from then it is
  nobody's credential, nor is any key made with it. What they applied is
  stopped as stopping it by hand would stop it (what it started, its
  services, an agent's turn it asked for), and what it did stays done;
  `--leave`, or the switch in the question, leaves it running with what
  the key held. `mw key roll <id>`, or Roll… in Settings, gives a key
  another secret, shown once, and refuses the one it had: it is the same
  key, so what it applied goes on, and a key may roll itself. A key made
  before keys were kept with the key that made them is revoked by
  itself; what a key applied before keys were kept with what they
  applied, and what a step of its applied on another device, is not
  found by revoking it.

### The board and tables

- Cards are added, moved between columns, given to an agent, commented
  on and answered, in the client or from a terminal.
- The board can be laid out by what comes first: which cards can be
  begun now and which wait on which, in bands by feature or by label.
- A feature gathers cards that are on the board already. An agent plans
  their order, and you accept the plan or say what should be otherwise.
- Any table of a project is a page: its rows are changed where they
  are, and its views are saved for everybody who opens it.
- A column's head has a menu: sort by it, show only some rows, gather
  the rows by it, a figure at its foot, freeze the table up to it,
  leave it out, move it. Held and moved, a column is carried to another
  place.
- A table's columns are yours to add, call something else, make another
  kind and delete, and each can be given an icon.
- A table is called what its blueprint calls it for people, says what
  it is for under its head, and counts its rows in its own words: a
  card's A/B tests and A/B attempts, `4 attempts`, each attempt called by
  its card and its contender.
- A field of a table may say it is `said` with what is said of its
  records (`sprint: { type: text, said: true }`): every event about a
  record of the table, a card's move among them, carries what the record
  holds of it under `fields`, so that a handler is chosen by it with
  nothing run (`for: { fields.sprint: autumn }`, or a plain `when`), with
  no rebuild for a field a blueprint adds. A text longer than a kilobyte
  is cut there, a field that is `quiet` or guarded is refused the word,
  and a table that says it of nothing says what it always did.
- A row of any table, opened, has one row of tabs at its top, as a
  card has: Details, its fields with what removes it, then Comments,
  History and Attachments with how many. Comments, History and
  Attachments were tabs under its form.
- The mark beside a board's column says what the factory's bot does
  there in a sentence: a stage as `Worked by the worker role, at most 2
  at a time → review on pass`, with `· back to doing on fail` where cards
  are reviewed, and what is pressed on a card as `Holds each card here
  until Signed is pressed · each time it comes`. A cell left empty is
  said as the fleet fills it in.
- A flag of a record's form, such as an action's Holds and Again, has
  its switch a little past its name, every switch of the form in one
  column. They stood at the form's far right edge, which a narrow drawer
  cut off.
- A table or the board is written out as a CSV file from its page, and
  a CSV file is read back in, saying first what it would change.
- A page has an address to copy and a pin, and a library is shared as
  pages from its head. A row of any table has an address too, copied
  from the head of its form, which opens the row's form over its table
  (`/shop/test-runs#shop-test-runs-4`); one that names no row says so.
- A card moved out of `doing` while its agent works on it (back to
  ready, to the drafts, on or to done) has its agent stopped at once,
  with what it changed left where it was, and put back it is taken up
  where it was left: its agent is told so and what was said, and the
  card is not worked a second time as new. A stage's row that says
  `moved_away: finish` lets the agent go on to its end as before.
- A card's history says what the fleet did as the factory's bot, with
  the blueprint after it: that it waits its turn, that its turn came,
  which agent worked on it, that it was approved. Those lines were under
  the picture of whoever applied the fleet, who did none of it.
- What a card's history says is drawn by its paragraphs: a card sent
  back by several reviewers says each one's words apart from the
  others, where they ran together as one paragraph.
- A table of a command's runs, kept where a row of `segments` says
  `record_to`, calls each run by a run id, when it began and the card
  (`20261122-140310-shop-7`), with the day it began as a date and the
  time beside it, so that it is sorted and filtered by the day. It was
  called by when it began, in words. A table made by an earlier version
  keeps its fields, and its new rows have no day or time.

### Channels and agents

- A forum's topics are a place of the rail of their own, Topics, just
  after Channels: pressed, the side panel lists the topics of the
  workspace it is of, the latest written first, with those closed folded
  under Closed; pressed again, it is put away, and Alt and 5 shows it.
  The channels' panel holds the channels alone again, and is called
  Channels.
- People, agents and the factory's bot talk in channels, with threads,
  mentions and attachments. A page an agent attaches is drawn in the channel.
  `mw read` names what is attached with its id, by which
  `mw attachment <id>` reads it, so that an agent run outside the
  factory, which reads a channel from a terminal, reads it too.
- The list behind the @ of where one writes is as wide as its longest
  name, up to its most, before a name wraps.
- A thread's box beside a channel stands a clear step higher than the
  channel's own, however much is typed in either.
- A blueprint's head keeps Apply on the title's line where the side
  panel is open, the words of where it ships from cut short instead.
- A file of a project named in what somebody wrote (a message, a thread,
  a comment on a card or a record, what a blueprint told somebody) is a
  link on its own words that opens it, in backticks or a block of code
  as well: `docs/plans/x.md` is a file of the channel's or the record's
  project, and `shop:docs/plans/x.md` one of Shop's. A path that names
  no file the reader may read is words.
- A channel can be private: only the people and agents in it read it,
  and nobody else sees that it is there. It is made so as it is added
  or afterwards, members are added by a member, and only the factory's
  owner opens it again (`mw help channel`).
- A channel's details (what it is called, and what it says it is for,
  under its head) are at the top of its Channel tab in the Tools panel,
  and are changed by its managers: whoever made it, whoever a manager
  names one there, and whoever manages the factory. Everybody in a
  conversation manages it. Anybody else in the channel reads them as
  words (`mw help channel`: `channel describe`, `channel manager`).
  **This changes who may rename a channel**: a build of this before it
  was released let anybody who may write in a channel rename it, and
  now only its managers do, by the row's menu, the terminal and the bot
  alike. A channel made by an earlier build has no managers, since who
  made it was not kept: whoever manages the factory renames it, and may
  make somebody in it a manager.
  What a channel says it is for is told to an agent asked something
  there, beside the documents the channel keeps to hand.
- Everybody has an inbox: a message that names you with `@` in a channel
  you read waits there, with who said it and where, until you have read
  it or put it away. Only you read it. It is a place of the rail, under
  Resources, with how many wait on it, which shows it in the side
  panel; and `mw inbox` in a terminal (`mw help inbox`).
- A message can be written to one or several people and agents: a
  conversation, which is in the inbox of each of them and is read by
  nobody else. Its line says what it is about, who is in it and the
  latest thing said, and is raised by each answer; it is answered where
  it is opened, with threads and attachments as in any channel, and one
  of those in it can open it as a private channel. It is the pencil at
  the head of the inbox, and `mw to` in a terminal (`mw help inbox`).
  An agent that is run outside the factory reads and answers one, and a
  private channel it is in, with its own invitation's credential.
- A row of a fleet's `agents` may name a file of the project's own words
  its agents are told first (`prompt`, a file of the `prompts/` of its own
  blueprints), read as a project's own blueprint is: as somebody who
  holds what the fleet runs with wrote it through the factory.
- A fleet tells you, in your inbox, when something waits for a person:
  a card's agent asked a question, a card was left for a person and
  why, a plan waits to be accepted, or something in the fleet failed. Press the line and the card is open.
  It tells whoever applied the fleet; who else, and of what, is a table
  beside the board (`tells`). A fleet from an earlier version tells
  nobody until it is applied again with `--take`. A blueprint of your
  own tells somebody with one step (`inbox.tell`).
- What a fleet says in its channel of something that failed opens the
  run that failed, at the step that failed, with what it wrote and why
  (Details, then Open the run that failed); a run that is no longer kept
  is said to be so there.
- Agents are the programs you already use, run on your machine as you:
  signed in as you, or with sign-ins the factory holds for them.
  Packages for three such programs come with it, and another is a
  package anybody can write.
- A package is installed as a copy, and an upgrade never replaces one.
  One that is not the one that came with the program says so, in
  `mw extension list` and under Extensions, and `mw extension update`
  or its Update button takes the one that came, keeping the copy beside
  it.
- A fleet is two tables of a project: who is in it, and what they do
  with a card. Change a row and the fleet follows: more who work at
  once, an agent that reviews, a council, a stage that tests.
- A card made ready is taken by a worker, worked on a branch of its
  own, reviewed, and sent back with what was wrong or merged.
- A stage that begins something for each card, with nobody in its role,
  says so where the fleet reports: it begins nothing until somebody is,
  even where what it begins asks no agent.
- What a review comes to is taken in the card's own turn: a card that
  passes is approved, and what is done after its review begun, while
  another card's things after its review (a command a row lays there)
  still run. Before, every card of the column waited behind them. So is
  a card passed by hand and merged, and what a row lays before a card's
  plan: neither holds another card of its column any more. And a row of
  `segments` laid where the review is, with the usual merge after it, has
  the card merged and done; before, the merge refused to be called and
  the card was left for a person.
- On Linux an agent's program works behind a wall: it is shown its work
  and a home of its own, and not your keys or your other projects. Of
  what its program keeps in your home it is shown only its sign-in,
  where the agent's package says which that is: its settings, history
  and conversations are its home's own, and it no longer sees your other
  settings or what you asked the program yourself. So, once, each such
  agent begins its conversations afresh: what it remembered of one held
  in your home is not carried on in its own.
- What an agent is and how it stands, with the standing instructions it
  was given, is written before each piece of work into the file its
  program reads as its own standing instructions, where its package names
  one, and the agent is handed only what it was asked and a line more.
  Elsewhere it is told all of it ahead of what it is asked, as before.
  Among it is what its slot lets it do, in words: what it may ask of the
  factory by area, what its program may do on the device, what it signs
  in with (by kind, never a value), and that its slot is set by others
  and never by itself.
- An agent changes nothing of its own slot, even within what it holds:
  not what it may ask, its wall, the web, its keys or its sign-in, by
  its own route or by the row of a fleet that brings it up. A person,
  or another agent that holds what it gives, sets it.
- What an agent may ask of the factory is said by area, in the client
  or with `mw agent add <name> --scope`.
- A change of a record somebody may not make (a cell that takes
  `shell`, a table that takes `admin`) is asked for, `mw ask <record>
  <field>=<value>... --why <words>`, and is told as a line of the inbox
  of whoever could make it. Approved on that line's page or with `mw
  approve`, it is made as the asker's, in the same write, with every
  check asked of whoever approved it, and its history says who; declined
  (`mw decline`), nothing is made. The asker is told what came of it
  where it asked. One whose record changed since is stale, and one not
  decided in seven days expires. Agents are told to ask so, rather than
  for more for themselves; an agent's own row is never asked for. A
  card's cells are asked for the same way, and changed by the board. A
  change is not made as an agent that is stopped, and an agent taken
  away has what it asked for withdrawn.
- Every agent is set up by a blueprint, `std/agent`, whose params are
  its settings: one added by hand is an instance of it, applied as
  whoever adds or changes it (which takes `blueprints:run` beside
  `agents:write`), and a project that forks it sets its agents up its
  own way. An agent added before this is given its text once, as it
  stands.
- The Agents page, under Factory, lists the agents of the workspace
  showing by project, with what each is doing now and a history of what
  each was asked and what it answered.
- An agent is stopped, or taken away for good (`mw agent remove`).
- An agent you run yourself, outside the factory, is given a name of
  its own: invited (`mw agent invite <name>`, or on the Agents page), it joins
  with its invitation (`mw agent join --url <address>`), and its
  commands (`mw --as <name> ...`) say it did what it does. It is listed
  with the others as external, is given cards, and holds what you say
  it may ask. The factory starts nothing for it and gives it no work.
- A daemon stopped while agents have cards is started again, and the
  work goes on.
- What an agent's program sees of the device is said as the daemon
  holds it: in a factory started with `--walls off`, an agent whose
  slot asks for a wall is said to have none (`mw agent list`, and its
  card), where it was said to be behind one.

### Blueprints and comparing

- A blueprint says what exists, what is heard and what is done. It is
  checked before it runs, applied to a project, and takes a changed
  text while it runs.
- Something running whose kept text no longer checks when the daemon
  starts (written for an earlier version of the program) has failed and
  says why, and what it had started is stopped with it. What it had
  started went on running: a fleet's arrangement that could not be taken
  up left its stages beside those of the one applied in its place, two
  for each column, each hearing the same cards.
- A step that adds a row (`record.add`), adds a card (`board.add`) or
  says something on a record (`record.comment`) is the same done twice:
  where the daemon stopped in the middle of one, it is done again when
  the daemon is back and finds what it left, so the row, the card or
  what was said is there once. It used to fail as not safe to repeat,
  and leave what it was doing for a person. A row of a table kept as
  files, or by a store service, may be written with nothing said of it
  yet, and is still refused there.
- The editor offers what can go where as you write and says what a word
  is, lays a blueprint out the one way, shows it as a graph, and finds
  in it with Ctrl-F.
- It does so inside a step's arguments too: pointing at a key of a
  table's field (`quiet`), or at one of the words a key may be (`list`
  after `type:`), says what it means, and the keys a field, a view or a
  health check does not have yet are offered as it is written. `mw
  actions <name>` lists the same under the action's arguments.
- Everything a blueprint can say is on one page, the Reference: the
  words of the language, the actions and the events, found by typing,
  each with an address that opens the page at it. It is a line over the
  blueprints in the side panel and a line of Help, and the card about a
  word you point at in a blueprint's text leads to it.
- The blueprints in the side panel are listed in two levels: what
  somebody applies, and under each, folded, its parts, the blueprints it
  runs, with how many; a box over the list finds one by its name or by
  what it says it is. Which is which is worked out from the texts, a
  part's Summary says what it is a part of, and no name is another.
- Every word of the language and every action you would write in a
  blueprint of your own says what it is for and is shown by an example,
  a whole blueprint to read and copy, on the Reference and on the card
  about a word you point at; the Reference also lists the blueprints
  that ship that use each, each opened at that line.
- A run is shown step by step, each step in the words its blueprint
  says of it, with what it gave and why it failed.
- A blueprint is said in plain words, for somebody who does not write
  them: what it is, what it makes, what it does when it is applied, what
  it hears, what it may do (a command it runs and a secret it reaches
  are marked), and what its runs have cost. It is the Summary of a
  blueprint's page, which a blueprint that ships opens on, and `mw
  explain` from a terminal; it is worked out from the text, with no
  model asked and nothing run. What it makes is a table, a row for each
  thing made: what it is, what it is called and, for a table, its fields
  (each with its kind, what it may be or names, whether it is needed and
  what giving it a value takes), folded under its row and shown in a
  card where its name is pointed at; a blueprint that makes two tables or
  more lists them together as its Data model, where a field that names
  a record of another leads to that one. `mw explain` prints each
  table's fields under it.
- What a blueprint that ships does is changed for one project by
  telling it something else, by a row of a table it makes, or by forking
  it: its text is copied into the project, from its page (Fork) or with
  `mw own`, and the fork is what runs there and is the project's to
  change. What a fork was forked from is kept, and its Summary says
  that, where it differs from it, and how the default has moved since.
  Nothing is merged, and going back to the default is one step.
- A blueprint can be run in a sandbox before it is applied anywhere: a
  factory of its own, made for the test, in which what a blueprint does
  to tables, cards and channels is done for real and everything that
  reaches outside them (a command, an agent, a repository, a file) is
  stood in and says so. Sandbox test, under Testing on a blueprint's
  page, runs the text as it stands there and shows what happened: each step, the
  blueprint's graph lit by it, the tables it ended with, and what was
  stood in; an event can be said to it, a card added by hand, its clock
  moved on and a stand-in answered, and after the text is changed it is
  run again with the same things done, saying what differs. `mw
  sandbox` prints the same from a terminal. Nothing in it touches the
  factory in use, and passing there is not passing for real.
- One blueprint can be started several ways at once (other words for an
  agent, another model, another way of working a card), and what comes
  of each is counted side by side (`mw compare`). It counts, and does
  not judge. The usual counts of a board's cards count a card left for a
  person, whatever left it (`left left`).
- An example, `research`, comes with the program and is put into a
  project with one command, to try two ways of asking on the same
  questions.
- Everything that happens is an event, and what takes time is measured:
  `mw events` and `mw metrics`, and Activity and Measurements in the
  client.
- A list cut at so many says so: `mw events` asked for more than the
  daemon hands at once (a thousand), and `mw instances` past its
  `--limit` (two hundred where nothing is said).
- A blueprint can host a service behind a stable address and roll a new
  version out a share of the traffic at a time, rolling back one that
  fails. A service whose program ended is starting, and not healthy,
  while it waits to be started again: it was said to be healthy until
  its program was there again, and a step waiting for it to be healthy
  went on in that while.
- A secret is kept by the factory and handed to the program that needs
  it, and nothing reads one back.
- Something outside can say an event to the factory, with a key made
  for that one topic or as a service that signs what it sends.

### Devices

- A second device joins a hub with an invitation (`mw device invite`,
  `mw device join`), and a step of a blueprint then runs there. An
  invitation made by a hub that listens on every address says where it
  was asked and the names it was told, and never `0.0.0.0`, which no
  device can reach; where that is this machine alone it says so.
- What crosses between the two is sealed: each proves a key to the
  other, and nothing in between reads or changes it.
- A second factory runs beside the first on one machine, as development
  beside production, and the two share nothing (`mw up --profile`).
- A factory started from a copy of another says so across the top of
  every page, with what it is a copy of and when, until its owner says
  it may stop; `--banner` as a factory is started has it say something
  else there (`mw help profile`).

### Systems

- Linux on x86_64 is the system it is tested on, on every change.
- Windows: it has been started by hand, with a card worked through a
  fleet by stand-ins for agents' programs, and its tests have been run
  there by hand and not on every change. No wall is held there.
- macOS: written for, and never run.
- The desktop window has been looked at on Linux only. On Windows it
  has been built and run where it opens no window. It can be opened
  there onto a daemon in WSL and start none on Windows (`--url` with
  `--no-start`, which says so and ends where nothing answers).

### Taken out before this release

- The fleet's page, which drew what was running and the agents with the
  cards they had in hand, flat and in 3D, is taken out of the
  application, and the Fleet tab of the Tools panel beside a channel
  with it. It is to come back as an extension you choose to add, and is
  not part of what every installation carries. The fleet itself, its
  tables and what it does with a card, are as they were: what is
  running is under Activity, and each agent and what it is doing under
  Agents. A browser that kept the page among its open tabs, or the tab
  among the panel's, opens without it.

### Renamed before this release

For whoever ran a build of this before it was released: what was a
factory's **environment** is its **profile** now, since the word
environment is kept for what a project has of its own (`dev`, `staging`,
`prod`).

- `mw up --profile prod`, `mw --profile prod <command>`, `MW_PROFILE`
  and `mw profiles`. `--env`, `MW_ENV` and `mw env` are still taken and
  say once that they are the old spelling. A factory's data folder, and
  the list of factories on a device, are read as they were.
- In a blueprint `profile.name` and `profile.vars.<name>` are what
  `environment.name` and `environment.vars.<name>` were, and what runs
  is told `FACTORY_PROFILE`. `std/profile` sets the variables, with the
  action `profile.set`, and says `system.profile`; the variables that
  were set are the profile's, with nothing to do. `std/environment`,
  `environment.set`, `system.environment` and `FACTORY_ENVIRONMENT` are
  gone, with nothing in their place, and the events of a factory
  starting, copied and copying itself say `profile`.
- A text of your own that reads `environment.name` or
  `environment.vars` still reads, and means something else: the
  environment of its project's it was applied under, which is none
  while a project has none. Write `profile` where the factory's was
  meant. `mw check` warns of a text that sets the profile's variables
  and reads `environment` (`environment-renamed`); a text that only
  reads it cannot be told from one written now, and is not warned of.

### A project's own blueprints are kept by the factory

A project's own blueprints (its forks, its fleet's blueprint, an example
put there) are kept by the factory in its own data folder, under
`blueprints/projects/<project>/`, and never in the project's folder: a
project is often a repository other people read. In the client and from a
terminal each is a file of the project's named `@blueprints/...`, read,
written and deleted as any is (`mw file read shop @blueprints/fleet.bp`).
A project may have a folder of its repository read as well, before them,
for blueprints kept with its code (`mw project blueprints <project>
<folder>`, or the gear of its page); a name with a file in both is read
from neither until one is taken away.

What an upgrade does, for whoever ran a build of this before:

- A project that said nothing of where it kept its own read them under
  `blueprints/` of its folder (and so did one that said `blueprints`,
  which was kept as nothing said). It reads none of its folder's files as its
  own now, but for its fleet's blueprint at its top, which it is set
  going by. So a project that is a checkout of this program's source,
  which keeps every blueprint that ships under `blueprints/std/`, is no
  longer taken to run a fork of each, and the notes that said so (on
  Activity, for a project that was not trusted) are gone, with nothing to
  do. What was running there that reached one of those files is let go
  of it once, as this version first starts: the note it had of it on
  Activity is gone as the factory starts, and it is given what ships (or
  what the factory keeps for the project) from its next call of it.
- A project that said a folder keeps it, as a folder of its repository
  that is read; one that said none keeps none.
- To keep blueprints a project had under `blueprints/`, move them in:
  `mw project blueprints <project> --move-in` (or `--from <folder>`), or
  *Move them in* in the gear of its page, once
  that folder is said. Each, with those anywhere else in the repository
  that were read by their whole path, is copied into the factory's place
  with what
  each fork was forked from, the project then keeps them there, and the
  repository's files are left as they were, for you to take away.
- Forking a blueprint for a project that is not trusted is no longer
  refused: the fork is written where the factory keeps it, which only the
  factory writes.
- A new project is not given a `blueprints/` folder, and its README says
  where its blueprints are.
- A project's own blueprints are listed under its Blueprints in the side
  panel, each with the files it links under it, and *New blueprint…*
  there and in the gear of its page makes one where the factory keeps
  them and opens it. Where such a file is said, *Blueprints* leads back
  to that list.
- A copy of a factory knows the files of each project's own blueprints
  as they were, so the first look at a project in a factory started from
  one finds nothing new there.

### A poll whose answer changes sets its handler off

A handler that asks something on a timer (`poll:`) and goes on when the
answer is another keeps what it last read with the run the change sets
off, and not before it. Kept before, a change read in the moment its
instance took another text where it is (as `mw apply` of a text that
only changed what it says of itself does), or as the run could not be
written, or with the daemon stopped between the two, set nothing off,
and the next reading was held to it: `github/issues` applied with
`timer` could keep a new issue's reading and look at nothing until
somebody said `issues.look` by hand. Such a change is now read again,
and sets the handler off once.

And a poller with nothing read before asks first as it begins to
listen, and goes on from that answer. It used to ask first a whole
interval later and take that answer, whatever had changed meanwhile,
as only what the next was told from: `github/issues` applied with
`timer` looked at the issues there were, and an issue made in the
minute after that was in the timer's first reading and never came in
until it changed again.

### Known limits

- There is no installer and nothing is signed: Windows and macOS ask
  before they start a program from the folder.
- A wall is held around an agent's program and nothing else, and on
  Linux only. A command a blueprint runs, and a service it hosts, run
  as you; and an agent's program still reaches the network.
- What a credential may do is said for the whole factory, not for one
  project.
- An external agent's credential is a token kept in a file on the
  machine it runs on: whoever reads the file is that agent. A message
  that names such an agent waits unread: nothing hands it messages yet.
- A private channel keeps what is said in it from whoever is not in it
  through the client, the command line and every route. It is not
  sealed on the disk, the factory's owner reads every one, and what was
  said in a channel before it was made private was said to everybody.
- With a real agent, one card has been worked end to end, by hand. A
  fleet of several agents, a restart in the middle of work and a
  comparison have been played with stand-ins for the agents' programs.
- A daemon that is stopped or killed loses nothing. A machine that
  loses power can lose the last moments written.
- The daemon does not encrypt what the client and the command line
  send it: on an address other machines reach, put it behind something
  that does.
- A table cannot be taken away, copied or moved to another project, and
  a file cannot be moved or renamed.
- Some words of the blueprint language are used by nothing that comes
  with the program and may be taken out after the preview: `docs/status.md`
  names them.
- `docs/status.md`'s *What is not held yet* is the whole list.
