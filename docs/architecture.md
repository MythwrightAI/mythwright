# Architecture: the decisions, and what is not built

This is a record of what was decided and why, written so that a decision can
be argued with. Where the old codebase taught a lesson the hard way, the
lesson is named. The last section lists what does not exist yet.

## The shape

One daemon per device, one binary (more than one on a device is more than
one factory; see *Factories and profiles*). It holds the journal, runs blueprints,
supervises what they start, serves the HTTP API and serves the client. The
command line is a client of that API, and so are agents, and so is the web
client; there is no second way in. The desktop app is a window onto the web
client.

Twenty small crates rather than a few large ones, with dependencies pointing
one way (the table is in [CONTRIBUTING.md](../CONTRIBUTING.md); a gate
enforces it). The old codebase had
one crate whose test build needed 6.4 GB of memory in a single compiler
process; small crates are the structural fix for that, and for "a one-line
change rebuilds everything".

The runner is two of them. `runs` is what it keeps on disk: the tables of
instances, runs, steps and the places of handlers, the shapes they are
read as, and every write of them with what is said of it as one
(`runs::together`). It stands on the journal and has no engine in it:
nothing there holds an instance in hand, starts a handler or knows an
action. `conductor` is the running, and stands on `runs`: where a write
has to ask something of what is in hand (whether a run still speaks for
its instance, `Becomes::still`) the conductor hands it the question, and
what `runs` holds is still offered under the names the conductor always
gave (`conductor::InstanceInfo`, `conductor::copy`), so nothing above
knows there are two. The tables are migrated under the conductor's name,
as they were before they were a crate's: a journal from before knows them
by that. It came out when the conductor reached the most lines a crate
may have, and was the first thing to come out of it.

Comparisons were the second, and came out the other way: `comparisons`
stands on the conductor, beside `hosting` and `studio`. It holds what a
comparison is kept as and everything done with one: starting it, the
ways and their shares, what a way reads in place of a linked file, the
counting of what came of each thing, ending it. The conductor offers
nothing of it. It knows only that something may answer for comparisons,
and asks four things of whatever was installed (`conductor::Compared`,
a struct of four functions, each handed the engine that asks):
what a step starts in place of the blueprint it names, what an instance
reads in place of a file it links, which kept texts are still wanted as
a conductor starts, and that one is starting. With nothing installed it
runs as where no comparison was ever started. What `comparisons` needs
of the conductor to answer is what the conductor offers anybody (its
journal and library, a text checked, a text kept by its revision or
found by its version, what an instance is pinned to and what started
it, a task that ends when the conductor is halted), which is what
became public with the move. It keeps what it kept in the same places
(`compared`, `compared.went`, `compared.reads` and `compared.counted`
in the journal's key-value state), so a program from before reads what
this one wrote and the other way about.

The actions the conductor brought itself were the third: `blocks` is
`pool.turn`, `text.put` and `profile.set`, each an action as any
other crate's is, handed a step's context and asking the engine what
anybody may ask it (whether an instance is in some conductor's hand,
the words an instance is given for a file it links and what a way of a
comparison has it read instead, the variables set). What each works out
was never the conductor's: who a pool gives a place to is
`kernel::pool` and what parts come to is `blueprint::parts`, still
offered as `conductor::pool` and `conductor::words`. It is a layer of
its own, under `comparisons`, `hosting` and `studio`, so that the suite
of anything that stands on the conductor can register it; the
conductor's own suite cannot, and the tests of a pool, of linked words
read through a pin and of the variables set by a blueprint are in the
suite of `blocks`, on a small rig of its own.

Which text something running goes on with was the fourth, and came out
downwards, as `runs` did: `texts` stands between `runs` and the
conductor. It is the library (`texts::library`: the layers a name is
looked for in, a project's own before the shared ones and those before
the ones that ship; the ones that ship, compiled in; the names a
blueprint's file may have), and everything a conductor keeps of texts
beside what it runs: what a text reaches, checked, and the last of each
name that checked (`reach`); what a tree was held to when somebody
applied it (`pinned`); which of a project's own files were vouched for
(`vouched`); how a changed text is come by (`taking`); and, with
nothing kept, whether an instance is behind the text now there
(`behind`). Each
conductor is made with one keeper for it (`texts::Texts`), handed its
own journal, the factory's, its rows and the library, and what every
conductor of the factory shares of it (the setting, and the one turn at
changing what is kept for an instance). The keeper keeps in the same
places under the same names (`runs::beside`), so a program from before
reads what this one wrote and the other way about.

There is no engine in it and no registry. It asks two things back of
whatever runs (`texts::Runner`, which the conductor answers): a text
checked against this device's actions and events, which is the
registry's to say (the conductor builds `texts::known::Known` with its
registry for each check); and what an
instance is called and what started it, which the conductor has in hand
and the rows would cost a question of the database for each level of a
tree to say, at every step that starts something. Whatever else such
code needs it is handed as plain values: what of a request it reads,
which kept texts comparisons still want. The conductor keeps the thin
wrappers, under the names it always gave, that say first which
conductor keeps a thing; and everything that is the running stays its
own: the text a step is given and what is said of it, a followed text
taken, a tree put back, the one rule by which an instance comes to a
text, carrying over and taking in place. The keeper says no event, so
what the conductor says is described where it was. The crate's own
suite holds what needs no conductor (the names a blueprint's file may
have); what a conductor keeps of texts is played where one runs, in the
conductor's suite, which may use the crate below it.

What is kept of the factory's variables came out beside it, the same
way: `environments` stands between `runs` and the conductor, and holds the
variables of the factory's profile as they are kept in the
factory's journal and held in memory (`environments::factory::Held`, made
as the first conductor is and handed to every one of the factory), what
a change of them comes to, and the turn at changing them
(`Held::changing`). What an expression is handed of them, and what is
said when they change (`system.profile`, said while the turn is
held, so that what is kept, what is held and what is said come in one
order), is still the conductor's, under the names it always gave
(`conductor::profile`, `Engine::variables`,
`Engine::set_variables`). It is where a project's environments, named sets
of variables and secrets one of which an instance of the project runs
under, are to be kept beside them.

`agents` reached the same ceiling, and what came out of it is
`deciding`: the blocks a fleet's blueprints ask so as to work nothing
out themselves. What a turn that came for a piece of work comes to and
what a round that ended comes to (`work`), what it is owed where it is
looked over, what the votes on it come to and what becomes of it then
(`reviews`, `tally`, `decided`), what comes next in a list of things
done with it and what is begun for it (`segments`, `begun`), what is
wrong with a row and what a row that was taken changed (`column`), what
becomes of work that could not be kept (`unkept`), and what so many of
a name are called and what is kept of something put to an agent a
number of times (`counted`, `rounds`). Each is a function of plain values with a table of cases,
and the action that hands it what a step was handed; none takes the
handle agents are asked through, and the one that reads a text
(`segments.next`, for the params of what it is about to have called)
asks the engine as the step's instance, which any action may. So the
crate stands under `agents` and over `studio`, of which it takes one
shape, how a row says it was moved (`studio::models::Moved`), and over
the conductor, for the registry and a step's context. `agents` offers
every module under the name it always had (`agents::work`,
`agents::segments`), so nothing above knows there are two, and adds
their actions with its own, each where it always was
(`Agents::register`): the schema kept for editors lists the actions in
the order they are added, and did not move. The tables of cases are in
the suite of `deciding`, which starts no conductor and no agent; what a
blueprint does with an answer is played where the fleet is, in the
API's suite.

`studio` came near the same ceiling (14,741 lines), and what came out
of it first is `people`: who is who. Accounts and their sessions,
invitations to register, and the credentials given to keys, agents and
devices were the part of `studio` that asked nothing of the rest, so
it went below, whole: it stands on the journal alone, beside `runs`,
and knows no project, table or channel. `studio` opens it as it always
did (`Studio::open`) and offers every name it had (`studio::Identity`,
`studio::identity`, `Studio::identity`), so nothing above knows there
are two; what is said of accounts is described where it is said
(`people`'s `events.rs`) and handed out with the studio's events where
it always was. Its tables are made by the same steps under the same
name (`identity`), so a factory from before starts on them with
nothing moved, and they are still listed with the studio's
(`studio::copy`, `studio::apart`), where a copy of a factory and the
move of a project read them. Its tests are in its own suite, on a
journal in memory.

The second is `talk`: where people and agents talk. Channels, who is in
each, what was said there and what is attached to it, the one bot that
says what the factory's blueprints say, and what a bot with a command
answers used nothing else of `studio` either. It stands a little
higher than `people`, on the conductor beside `blocks`, because four
actions went with it (`channel.ensure`, `channel.post`, `channel.join`,
`channel.leave`) and the block `channel.answer`; the studio adds them
to a registry among its own, each where it always was
(`talk::answered::register`, `talk::register`), so the schema kept for
editors did not move, and what `talk` says is handed out among the
studio's events where it was. `studio` opens it and offers every name
(`studio::Channels`, `studio::channels`, `studio::attachments`,
`studio::speaker`, `Studio::channels`). Its tables are made by the
same steps under the same names (`channels`, `attachments`) and are
listed with the studio's. One thing crosses the other way and is
written down at both ends: which project a channel says is a column of
`talk`'s table that `workspaces` writes (`Workspaces::place_channel`),
because it is written in one transaction with what workspace the
channel is of, which is that crate's table. That left `studio` some
2,800 lines under its ceiling.

The third came when `studio` was at its ceiling again, and was two
parts at once, each a crate beside the other on `talk`. `workspaces`
is what gathers the projects and the channels that belong together; it
asked nothing of the studio but the projects' table, which is in the
factory's own database beside its own, and two things said in the
writes of a project's row, which `Studio::open` now hands its projects
(`workspaces::place_new`, `workspaces::forget_project`) where the
workspaces used to as they were opened. `search` is the index of
everything a project knows, in a database of its own; what it reads of
the projects' files and of the board, which stand above it, it asks
back as plain values (`search::Indexed`, which the studio answers), as
the runner asks `texts::Runner`. Both are offered under every name they
had (`studio::workspaces`, `studio::search`, `Studio::workspaces`,
`Studio::search`), what they say is described with the studio's events
where it always was, and their tables are made by the same steps under
the same names. That left `studio` some 1,500 lines under its ceiling.

## Factories and profiles

A **factory** is one daemon and its data folder: its database,
accounts, credentials and address. A second factory on a device is a second
daemon on a second data folder, and the two share nothing. Nothing copies
one into another unless a person asks, and then one way, into a factory
that does not exist yet.

A factory may have a **profile**, a name such as `prod` or `dev1`
(`mw up --profile dev1`). [profiles.md](profiles.md) is the whole of it;
the decisions are these. Until it was renamed before the first
release a profile was called an environment, which is now
the word for what a project has of its own (`dev`, `stage`, `prod`); what
was written down under the old name stays so where a program from before
reads it (the stamp's `environment`, `environments.json`, the namespace
`environment` of the journal's `kv`), and the command line takes `--env`
and `MW_ENV` as the old spelling.

- **A profile is a name a factory has, not a thing of its own.** There
  is no object above factories that holds them. What must never be shared
  between production and development is exactly what a factory already
  keeps to itself, so the factory is the unit, and the name is how a person,
  a page and a blueprint say which. The name is not the factory: two may be
  called the same, and whatever has to tell them apart uses the data folder.
- **The name is the data folder's, for good.** It is stamped into the folder
  the first time (`factory.json` beside the database: it is read before the
  database is opened, to know where to listen) and a folder is refused any
  other. The stamp also keeps where a named factory listens, so that its
  address is the same every time, and an id of the factory's own, which a
  browser's session cookie is called after: cookies are kept by host and
  not by port, and two factories on one host would sign each other out.
- **Factories know of each other by name and address only.** Each named one
  writes its name, data folder and address into a list in the user's
  configuration folder as it starts (`crates/folders/src/registry.rs`). The
  command line finds `--profile dev1` there; a page is given the list to offer
  the way to another (`GET /api/profiles`), with an address only where
  one can honestly be given from where the page is. No credential is in it.
  The daemon reads it when asked and follows nothing.
- **A blueprint reads it**, as `profile.name` in an expression and
  `FACTORY_PROFILE` in what it runs. `profile` is a map built in one
  place (`crates/conductor/src/profile.rs`) so that what else a
  profile comes to hold is added there.
- **Its owner says what is true of it**, as variables that every expression
  reads as `profile.vars.<name>`: plain values, not secrets. They are
  part of the scope the conductor works every expression out in: kept in
  the journal's `kv` (`environment`) and held in memory from the moment
  the engine is made, by `environments::factory`, which every conductor of the
  factory is handed, and replaced whole by the one action that sets them,
  `profile.set` (`blocks`), which
  `std/profile` runs and which takes `admin`. They belong to the
  factory and to no instance: `std/profile` finishes and leaves
  nothing alive, so that stopping an instance, which takes less than
  `admin`, cannot clear them. A copy of a factory does not carry them.
- **One guardrail reaches outside a factory.** A project can be an existing
  directory, and that is the one way two factories could come to share
  files. The list also holds which directories each has adopted, and a
  directory held in one profile is refused to another
  (`studio::Holdings`, which whoever knows the device's other factories
  implements). Nothing is written into the directory.
- **A copy is a snapshot, and arrives where there is no factory.** To try
  something against real content, a factory writes a copy of itself to a
  folder (`factory.snapshot`, which `std/snapshot` runs, for the owner
  only) and a new factory is started from the folder (`mw up --profile dev2
  --from <folder>`). Each of the factory's files (its own database, and
  the journal of each project that has one) is copied whole on a
  connection of its own that only reads, every one as it was at the
  same moment (*A journal for each project*), and what must not travel
  is then taken out of the copy, never out of the factory: each crate that keeps tables says what of its own
  does not go (`copy.rs` in each, run in order by `crates/folders/src/
  snapshot.rs`), and the same is run again on arrival. Nothing arrives
  signed in and nothing arrives running: a copy that kept the rows of
  services would adopt, and later stop, the other factory's processes.
  What goes is an allow-list, of folders, of tables and of the small
  durable state alike, so that something added later is left out until
  somebody says it travels. On arrival the database is put in place last, since the
  database is what makes a folder a factory
  (`crates/folders/src/arrival.rs`).
- **What this is not** is a wall. Two factories under one user are one
  operating-system user's processes; an agent let loose in one can read the
  other's files. The hard line is another user or another device, and
  profiles.md says so in those words.

Data is one half of keeping two factories apart; the program is the other.
A program built in a checkout reads that checkout's standard blueprints,
packages and client as they are, which is what makes working on them quick,
and for a time every program built here did, wherever it was run from: each
build and each edit landed under the factory its author was using.

- **The checkout is read only by a program run from where it was built**
  (`crates/folders/src/place.rs`). That is told by cargo's own mark, the
  `.fingerprint` folder it makes wherever it builds, and by nothing about
  the path: a build folder kept outside the checkout still counts, and a
  copy of the program in a folder of its own does not, wherever that is.
- **A copy uses what is built into it and what is beside it:** the standard
  blueprints compiled in, and the `ui` and `extensions` folders beside the
  program. `--blueprints`, `--ui` and `--packages` point anywhere, for
  whoever says so.
- **Installing makes a copy** (`cargo xtask install`), in a folder of its
  own, and links the command names to it. Installing again over a copy that
  is running does not break it: a program is written beside the one it
  replaces and renamed over it; the client's files go in first and its
  entry page last, with the files of the client before it left for pages
  still on that one. The daemon that is running is the old program until
  it is stopped and started; the client it serves is the new one at once,
  as after any build of the client.
- **Which one it is, is said**: by the daemon as it starts, in
  `system.started` and `/api/meta` (`runs_from`), by `mw status`, and in
  the factory's log. An installed copy also says the revision it was
  installed from.
- **Working on the product beside a factory in use** is a second factory
  run from the checkout under a profile of its own
  (`target/debug/factory up --profile dev1`): its own data, its own address,
  the checkout's blueprints and client. The installed one changes only
  when it is installed again.

## Projects and workspaces

A project is one folder: its files, and what blueprints keep for it (a
board, a fleet's two tables, a library), under an id that is a slug of its
name and is in every card's id and every address. A channel may be a
project's, and in a project's channel the factory's bot is that project's.
A channel may also be kept to its members, whatever it belongs to
(*Identity*): a workspace still lets nobody in and keeps nobody out, and
what is left out of a workspace's list for somebody is left out by the
channel, which is asked.

Some work is made of several folders: an application, what is served for
it, its scripts. A **workspace** is the name for those together
(`studio::workspaces`): a name and the projects in it. A factory always
has a workspace and a project is in exactly one. The first is made with the
factory's owner, called after them (`Unnamed workspace`, while the owner
a first start made has not said what they are called, and after them
once they have, by the id it was made with); a project goes where
whoever adds it says, and otherwise into the usual one, which is the
workspace made first that is still there, and its place is written with
its row. It is put in another and never taken out of all; the last
workspace is never taken away, and one that holds something is taken away
only with where that goes, each thing moved in that write and said. One
place brings the rule about (`Workspaces::settle`), asked as the owner
registers and as a daemon starts: the start is what takes up a factory an
earlier program made, where a project might be in none, and it writes and
says nothing where there is nothing to bring about. Each of these is one
write of the factory's file, so a daemon stopped anywhere leaves all of
one or none of it, and what each says is said in that write: a workspace
made, renamed or taken away, what a channel belongs to, and that a
project was placed. Where a project is listed is a row of the factory's
own tables and nothing of the project's, so `project.placed` is among
what the factory's journal keeps for good (`journal::HERE`), whichever
journal keeps everything else said of the project: its place and the
saying of it are one write for every project, the event stays where it is
when a project is moved to a journal of its own, and it is heard by
project as any event is. Taking a workspace away moves its own channels,
so whoever does is asked, for each of them, what saying what a channel
belongs to is asked: nobody moves by removing what they could not move by
placing. A channel is a project's, and so of its project's workspace, or
a workspace's own, for talk that is of no one project of it: never both,
and never neither. The one write that says either takes the other away,
and saying that one belongs to nothing is refused. One made for no
project is the own of the workspace whoever makes it says, and otherwise
of the usual one, in the write that makes its row (`talk`, which knows no
workspace, is told how a channel is placed as it is made); a project that
is forgotten leaves its channels to the workspace it was in, in the write
that forgets it; and settling makes each channel that belonged to
nothing, or said a project that is gone, the usual workspace's own. So no
channel is the factory's: what one that belonged to nothing was, a
channel of the usual workspace is.

It is grouping and nothing else. Being in a workspace changes nothing of
a project, since nothing of a project is renamed or moved by it, and it
lets nobody in and keeps nobody out: what somebody may read is still said
by area for the whole factory (*Identity*), and whoever may read channels
reads those of every workspace. A project and a channel answer as they
did before there were workspaces: only where each is listed is said by one.

What belongs where is kept beside the projects and the channels and not
on them (three tables of `studio::workspaces`; a channel's project stays
where it was, on the channel), so a project and a channel are what they
were to everything that reads them. A row may name something that is
gone; it is passed over when read. A workspace's id is a slug of the name
it was made with and does not change when it is called something else. A
project's own channel, where its fleet reports, stays the project's while
that fleet is running: saying otherwise is refused, naming what posts
there, because the bot there is asked about that project. What is found
by searching within a project follows a channel made another's: each
thing said there is indexed again under the project the channel is of
now. The client's side panel is of one workspace
at a time, said in a strip at the panel's foot and chosen there: its
projects are listed as projects always were, with no row for the
workspace over them, and the channels a person put in no group of their
own are the workspace's own, then its projects' under each project, and
no others; a person's own groups stay theirs, kept with their account
so that they are the same in every browser they sign in to, and are
listed as they were, with what is pinned first of all, each while the
panel is of its channels' workspace. Nothing offers
what is in no workspace, since nothing is; a project whose place the page
has not read (from a daemon of an earlier program, or made a moment ago)
is listed under the usual workspace, which is where the daemon puts it. Which
workspace somebody is looking at is theirs, like their groups: it is
kept in the browser under who is signed in, the daemon is not asked and
is not told. The tabs that are open follow it: each workspace has its
own, kept the same way (which are open, their groups, the one showing in
each), so choosing another shows what was open there and going back
finds what was left, with nothing closed by it and nothing that was half
written lost; a workspace in which nothing was opened shows the start
page alone, and something of another workspace opened where the reader
is, is a tab there. Nothing else changes by it. A
count of what waits unread in a channel the panel leaves out is said at
the foot, and where the workspace chosen is taken
away the panel is of the workspace of what is showing beside it, else
the usual one; where nothing was chosen as a page is read it is the
usual one, or, in a browser that kept one set of tabs from before each
workspace had its own, the workspace of the tab that was showing in
that set, whose set it then is. What a workspace's own row offered (called something
else, taken away, a new one made) is in the foot's menu; the question
that takes one away says where what is in it goes. Until the workspaces
are read the strip says nothing, and every project and channel is listed.
At the top of the panel of what there is to work on are
what whoever is signed in pinned, and under it the eight things they
last opened there, each bounded as the panel is with what is the
factory's own in every workspace; both are theirs: what was pinned is
kept with their account, with their groups of channels, and what was
lately opened in the browser, of which the daemon is told nothing.

The head of a page of such a thing ends in the same few things on every
kind of page: when it was last changed, as far as its page read (the
later of when a table and its newest record were written; who, where a
line of that record's history was written as it was), what it is shared
by where anything shares it (a library as pages, a document by its own
mark), an address that opens it, that same pin, and a gear with what
else the daemon does with it as it is, and with what a page once had
buttons of its own for: the tables that sit beside a table, and what is
done to a board (a card or a column added, the columns arranged, its
view laid out). The address is a path that names
the thing by the names people know it by, with how it is shown after the
`#` (`/shop/test-runs#newest`): it opens the thing as a tab beside what
the reader had open, the address bar says the address of the tab showing
so that it can be copied as it is, it waits for somebody to sign in, and
it lets nobody in. The daemon answers such a path with the client's page,
as it answers any path that is not its own, and no project may be called
what the daemon's own paths begin with (`kernel::paths`), so the two
never meet. The daemon keeps none of the rest and was changed for none
of it: a table is written out as a CSV file and read back in by the
browser, a record at a time through the routes there were.

### A table read a window at a time

A table may hold a million rows, so what shows one never reads it whole:
the daemon is asked for one window of a view (`GET
/api/collections/{project}/{table}/rows`), the rows of the view as the
reader sorts it (`sort`, `down`), of one band (`band`), from where they
got to (`after`, `before`: a place handed out and handed back unread;
`at`, a rank), and answers them with where to read on and back from
and how far it looked. What is counted of the view (how many rows it
shows and the table has, its bands, the figures at its columns' feet)
is a second question (`…/rows/counted`). Everything that decides is
`kernel::window`, functions of plain values with their cases: the order
(a stretch of rows for each part of it, walked one way by one index:
what says something, then what says nothing; a choice an option at a
time), which rows a view shows, the place a reading got to, how a window
is read (`plan`), the figures, which are held case for case to the
client's (`ui/src/figurecases.json`), and the statements, which say the
project and the table as literals and name the index they read by, so
that a window begins at its place in an index and reads on from there.
It costs the database the same steps beside a hundred rows as beside a
million (`windowed.rs` of the API suite counts them), and a statement
cannot quietly become a reading of every row (the studio suite holds
each to its plan, and every window the database reads to what the same
rules read in memory say). Where a view checks what rows hold (what a
list holds), an answer looks at 20,000 entries at most and says it fell
short and where it got to.

The indexes are made by asking (`studio::indexes`): a column of a table
of 10,000 rows or more the first time a window is sorted or held by it,
or as the table is declared where a field says `indexed: true`; each is
an index of the one `records` table for one column of one table, named
for what it is (`records_by <project>/<table>/<field>`), so the file's
own list of what it holds is the list of what is indexed and nothing
beside it is kept, copied or moved. The project and the table are its
first columns as well as its condition, since a count read off it
otherwise read every row it went through. At most eight of a table's,
the one asked for least lately let go for another. Below 10,000 rows a
table with no index is sorted in the statement. A table kept anywhere
but the database is read whole where it is kept and windowed by the same
rules in memory, and says it was.

A view is searched for words (`find`) as it is held to what it is held
to: a row is shown where every word is somewhere in the columns the
view shows, as words with capitals folded, the middle of one included
(`kernel::window::found`); it is checked row by row as a window is
walked, so a search of a large table goes in answers that each look at
so many rows, and what is counted of it says how far it has looked
(`looked`) until it has gone through. A row is gone to by its id or its
number (`around`): the daemon says whether the view shows it, and where
it does, where it stands among the rows it shows (counted off the index
where nothing is checked of a row) and the rows before it and from it
on; where it does not, what keeps it out. And a page that reads a table
so is told only which record changed: it asks how the view shows each of
those (`ids`), a moment's worth at once, draws each it holds where it is
(marked where it would now stand elsewhere, struck where the view no
longer shows it) and counts what is new, and reads the view again only
when the reader asks, about the row at the top of the screen, which
stays where it was. Nothing on the screen jumps.

### The Trash

What somebody deletes goes to the Trash, and is given back from there;
it goes for good once it has been there for the days the factory's owner
says (`trash.kept`, thirty unless said, nought for deleting at once as
before there was a Trash, or kept until somebody deletes it for good), or
when somebody deletes it for good. It is how document tools commonly do it.

Four kinds of thing go there, and one table holds them all (`trash`, the
last step of the records' tables), each row with what puts its thing back
where it was, who deleted it and when:

- **A row of a table**, a card included. The row is taken out of the
  records and kept in the Trash whole, in the one write that takes it out
  and says so (`Collections::trash_saying`): nothing that reads a table
  (a list, a view, a count, `rows.read`, the board) has to be told to
  leave it out, and it is found by its id by the Trash alone. Its history
  and what was said on it are kept by its id, as they always were, and
  the next record of its table is numbered after it, so that it comes
  back under the id it had. Given back, it is in its column at its place,
  or at the bottom of the column where its place is somebody else's by
  then, and what hears a card or a record being added hears it again
  (`restored`).
- **A file or a folder of a project** (a document, a mockup). It leaves
  the project's folder, since agents work there, and is kept in the data
  folder (`trash/<id>/kept`, with `whence.json` beside it saying where it
  came from). Moving it is three things (what says where it came from,
  the move, the row), and a daemon stopped between any two finds what
  was begun as it starts and finishes it (`Trash::settle`); deleting one
  for good marks it as going before its row goes. A document's record and
  what was said on it are known by its path, and are its again where it
  is given back there; where something else is at its path by then it is
  refused, saying so, unless it is given back beside (`notes (restored).md`).
- **A project**, forgotten. Forgetting stops nothing, as it never did, and
  leaves its files where they are; the project's row and the workspace it
  was in are kept in the factory's own table, in the write that forgets it,
  and given back it is in that workspace again, or the usual one where
  that is gone. What of it was in the Trash is listed again with it.
- **A channel**, with everything said and attached in it. Its rows, who
  is in it, what was said and attached there and the lines of anybody's
  inbox that open it are taken out in one write that keeps them in the
  channel's row of the Trash and says so (`Channels::trash`, in `talk`,
  which the `trash` crate hands the row's write: `talk` stands below the
  studio), read and written again by their columns' names so that a
  column added later is kept with them; what is attached is moved into
  the channel's folder of the Trash after that write, which is how a copy
  of the factory has it; the journal's events of what was said go, as
  they did when a channel was taken away; and whatever follows a channel
  going forgets what it holds of it by what is said
  (`channel.removed`, saying `trash`), as it always did. The row is the
  factory's, as a channel is whatever project it says, and it is what
  hearing of the channel takes, so that a kept one is seen in the Trash,
  and told of, only by whoever read it as it went. Given back it is
  written again whole in one write (`channel.created`, saying `restored`,
  which the search index takes in), kept to the same members, the own of
  the workspace it was where its project is gone, as `<name>-2` where its
  name was taken, with each agent in it asked again whether it may be.
  What an agent remembered of its conversations there went with the
  channel and does not come back.

A row of a project's is kept where that project's records are
(`Journal::records_of`), so a project moved apart takes its own
(`KEPT_APART`); a project's says no project and is the factory's. A copy
of the factory takes the rows, the folder and the setting. The writes are
`studio::trash`; what reads the Trash, gives back, deletes for good and
empties it is the `trash` crate, above the studio, which the routes and
`mw trash` ask (`api::trash`). Who sees a thing is whoever could read it as
things stand (a private table's rows only whoever may read the table);
who gives it back and who deletes it for good is whoever could have
deleted it. Its own events (`trash.put`, `trash.restored`, `trash.gone`)
carry no words of what was deleted, and take what reading it takes.

What is not built: a table goes to no Trash, since only a blueprint
declares one, nor does a column somebody added. A file given
back beside what took its place is the document of its new path, and
what was said on it goes there with it, in the write that gives it back.

## Blueprints

A blueprint is a file. It is the unit of iteration: changing how work gets
done should mean editing a file and applying it, not rebuilding the product.

- **A strict subset of YAML, parsed by hand**, keeping the position of
  everything. That gives precise diagnostics, lets the graph view point back
  at lines, and avoids YAML's surprises (`no` is a string here). `${ ... }`
  is the only non-YAML thing. So that everything else that reads YAML
  takes a blueprint for what it is, its file may be called so:
  `fleet.bp.yml` is the blueprint `fleet`, as `fleet.bp` is
  (`brand::blueprint_files`), read by the same reader. What that reader
  takes written plain and YAML does not (a value with `: ` in it, a
  `${ }` bare inside `{ }`) is the same to it in quotes, and the
  formatter writes it so (`parts` in `layout.rs`): what is laid out is
  YAML, and a test reads every blueprint that ships with a reader of
  YAML that is not ours. Never `.yml` alone,
  which is anybody's file; and a folder with two files for one name has
  said which is meant with neither, so neither is read and whoever asks is
  told of both (`texts::library::named`, `Library::refused`).
- **Closed vocabulary, said once.** Every field a step, hook, watcher or
  trigger can have is in one table (`crates/blueprint/src/vocabulary.rs`),
  with what goes after it and a sentence saying what it is for. The reader
  refuses a field that is not there, with a suggestion: a language that
  accepts anything cannot tell you what you misspelled. The same table is
  what an editor is handed (`/api/blueprints/language`), so what is accepted
  and what is offered cannot come apart. For the editors people already
  have it is given out as a JSON Schema too (`blueprint::schema`,
  `mw schema`, `/api/blueprints/schema`), worked out from the table and
  from the actions there are. A schema cannot say everything the reader
  does (what may be worked out is text to it, and what YAML allows that
  the reader refuses it cannot refuse), so it is held to saying less and
  never more: every blueprint that ships is checked against it in the
  tests.
- **What is inside an argument is the action's to say.** The language
  knows a step's arguments by name and no further: the keys of a table's
  field, or of a health check, are read by the action. So an argument
  may say what is written inside it (`ActionSpec::within`,
  `blueprint::inside`): a table of rows, each where something is
  written, what kind of thing it is and a sentence, for the keys a
  mapping takes, the few words a key is one of and what a key holds in
  turn, of things by a name of their writer's choosing or of the mapping
  itself. It is written once and everything that shows it is made from
  it: what the editor explains and offers there (one function of a text
  and these descriptions, `ui/src/blueprint/within.ts`), `mw actions`,
  the schema, the table in `docs/blueprints.md`. It describes and never
  decides: nothing is checked by it when a blueprint is read, checked
  or run, the schema leaves every mapping open, and no editor marks a
  key for not being named, since a text may be older than what an
  action now says of itself. What keeps it true is a test beside each
  table that reads, in the source, the words the action asks for.
- **Said in plain words, by one function.** What a blueprint is and
  does is said for a person by `blueprint::summary`, a function of the
  text as it was read and of what each action and each event says of
  itself, handed to it as plain values: no model, nothing run, and no
  list of actions anywhere. A step is said by its own `description` and
  otherwise by its action's sentence; what is made is what an action
  says it makes (`ActionSpec::making`, written into the contract with
  the rest), and a command is an argument an action says is one
  (`runs`). The daemon hands it out with the check's answer and by a
  route of its own (`/api/blueprints/summary`), `mw explain` prints it
  and the client draws it as a blueprint's Summary; neither works
  anything out. A text laid out another way comes to the same summary,
  since a line says what it is about as the graph names it and holds no
  place in the text.
- **A fork has a record of where it came from, and so has the
  default.** A blueprint that ships is the default, and is in the
  program. A project changes what one does by telling it something
  else, by a row of a table it makes, or by forking it: its text copied
  into the project, where the copy is what runs under the name. To the
  runner a fork is a project's own file and nothing more. What the
  daemon adds is the record: as a fork is made it keeps what it was
  made from, the text itself to the letter, with who and when
  (`texts::forked`), in the journal that keeps what is run for the
  project and with what is said of it, as one write; it is not kept
  among the texts running trees are held to, which are put away when no
  tree is held to them, so it is there whatever the program ships by
  then. The summary is where it is said: the same function, handed
  where the text stands as plain values (the text the fork was made
  from, the fork, what ships now), says what the fork was forked from,
  where it differs, and how the default has moved since, in the words
  `blueprint::differs` already says what moved between two texts in.
  Nothing is merged, and no record is made up for a fork from before
  there was one.
- **Run in a sandbox, which is a factory of its own.** A sandbox test
  (`mw sandbox`, `/api/sandboxes`) runs a blueprint to see what it does,
  and the one thing it must never be is a way to touch the factory in
  use. So a sandbox is not a mode of the daemon's conductor: it is made
  new, of nothing of the live factory, by a crate of its own
  (`sandbox`) that stands beside `agents` and on nothing above
  `deciding`, and so cannot name the daemon, the agents or the gateway.
  It is a journal in memory with its projects' beside it
  (`Journal::memory`), a clock nobody else moves (`ManualClock`), a
  studio over that journal that starts no program, asks no service and
  keeps no table as files (`Studio::open_within`), a made-up owner and one made-up project,
  and a conductor with a registry built for it, on a thread of its own
  whose runtime has a timer and nothing else: nothing on it can open a
  socket or wait on a program, whatever code came to ask, and what a
  sandbox starts is over when its thread is. The daemon hands it plain
  values and one thing more (`sandbox::Made`): the text, a project's
  own texts as they are read at that moment, what each action and each
  event on the device says of itself, and the blueprints that ship,
  to be read: the standard ones, which are compiled in, a checkout's
  folder of them where the program is run from one, and what the
  installed packages bring as texts read at that moment too, since the
  agents keep those. It hands it no journal, no studio, no agents, no
  secret and no key, and takes nothing from it but what it answers.
  An action in a sandbox is one of three kinds, said where it is
  registered and by no list of names (`sandbox::offered`). What a
  crate adds through an entrance that is handed nothing (`blocks`,
  `deciding`, the two blocks of a rollout) is a function of what a step
  hands it. What the studio adds through the entrance for its tables
  (`Studio::register_kept`) is its own code over the sandbox's journal:
  a table, a card, a row, a channel. Everything else the device
  describes is given a stand-in under that description, so that a text
  which checks on the device checks there: it writes down what it was
  asked and answers what whoever runs the test said, else what the
  action says a stand-in answers (`ActionSpec::stands`), else nothing.
  What answers for such an action is chosen in one place as a sandbox
  is made (`sandbox::outside`), and what was asked and answered is one
  record whoever answers (`stood::Record`). So an action somebody adds
  to the daemon is a stand-in in a sandbox from the moment it is
  described, and is the real one only where its crate adds it through
  one of those entrances; a test holds the real ones to a list with
  why each reaches nothing outside.
  A sandbox is acted on (the text applied, an event said, a card moved
  by hand, the clock moved on, a stand-in answered), and each act is
  answered when what came of it is at rest, which is worked out and
  never waited for by a clock (`sandbox::rest`). Everything a sandbox
  runs is a task of its own thread's runtime, woken only by another of
  its tasks or by one of its databases answering. So rest is three
  things known exactly: the runtime says it has nothing left to run;
  no action that is the real one is in the middle of its work (each
  counts itself in and out); and each database, asked a piece of work
  that it does after everything asked before, did nothing else
  meanwhile, twice running, since a piece is counted as done a moment
  before whoever asked for it is answered. It is held to so many steps,
  events and bytes, and stopped past them. What it hands back is read
  from its own journal as a factory's is: its instances and runs in the
  shapes the daemon hands real ones in, everything said in the order it
  was written (`journal::Order`, since its clock stands still), its
  tables and channels, the lines of the one inbox it has, and what was
  stood in. What an action of its own says for a step often names no
  run (the studio says it as whoever applied the text): each such
  action writes down where the order of what was said stood as it began
  and ended, and what was said between is put with its run where that
  is known: one run alone had an action at work, or of several one
  alone was of an instance the event names, or one alone had an action
  of the event's own family at work (`channel.ensure` says
  `channel.created`). Where it is not known it is said to be no run's,
  never put with the nearest. Its one folder,
  `sandboxes/<id>/` in the data folder, is where its project is said to
  be and a step's lines go (counted, as they are written, toward the
  bytes it may keep), and nothing else: every table of a sandbox
  is in its journal, and one kept as files is refused as one kept by a
  store service is. It is removed as the sandbox ends, and the whole of
  `sandboxes/` as a daemon starts.
  It is no part of a copy and nothing follows it. The daemon keeps
  eight at most, each its maker's, each ended when nobody has asked it
  anything for a quarter of an hour (`api::sandboxes`).
- **Expressions always terminate.** No loops, no assignment, no user
  functions. A path that leads nowhere is `null`. This is what makes it safe
  to evaluate guards and templates inside the conductor.
- **Checked before anything runs**: actions exist, arguments have the right
  types, permissions cover what is used, a step only reads steps that ran
  before it, nothing is circular. The same check feeds the editor's squiggles
  and the graph, so the text and the picture cannot disagree.
- **One model for everything the spec asked for.** Steps with guards,
  timeouts, retries and `for_each`; tasks (`task:`), which are steps that
  hold steps, so that a group of them has one guard, one timeout and one
  policy for failing, and is run by the scheduler that runs any graph;
  hooks (`on_start`, `on_success`, `on_fail`, `on_timeout`, and `on_finish`
  after any ending), which may consult an agent and decide `retry`,
  `continue` or `fail`; watchers (`after:`, `every:`, `log:`) that emit events while a step
  runs; handlers (`on:`) fired by events, timers or polls; `call` and `spawn`
  for other blueprints; `uses:` to attach a blueprint to another's lifetime
  (a log feed, a canary stage). Canary rollout is not a feature of the
  conductor. It is `blueprints/std/slot.bp`, written in the language.

Full reference: [blueprints.md](blueprints.md).

## Instances and runs

An **instance** is a blueprint that has been applied: a durable object with
parameters, state, things it holds (services, routers, agents) and children.
This is what the spec calls a sub-conductor: a stage of a fleet is an
instance that looks after every card in one column of a board, and a round
is one that has a worktree and an agent for as long as the agent is at work. A **run** is one
execution of a flow: the main steps, or one firing of a handler.

- **Applying is idempotent by key.** Apply again and the same instance is
  brought to what is asked (how is the one rule, below); its children are
  matched by key. Nothing changed means nothing happens. A child's key is
  what its step says with `as:`, or the step's name; in a `for_each`, what
  `as:` says with nothing added (so it must tell the elements apart, and
  one that does not is refused), or the step's name and the element's
  position (`crates/conductor/src/child.rs`).
- **A failed update puts back the text, the authority and the state** the
  instance had, and the instance stays up, saying why. A key the failed
  text no longer declared is there again, and what its steps set is as it
  was: the state is kept with the definition to go back to, so this is so
  after a restart too. A failed first apply tears down. (Rolling back a
  canary by stopping the stable version would be the wrong lesson.)
- **A failed update hands back what it started.** Kept with the
  definition to go back to is where the instance stood as the update
  began: what it held, and the newest instance there was. Going back,
  what it has come to hold since is stopped as it is when an instance
  stops, and each child made since is stopped; then the going back is
  written down, so that a conductor stopped in between finds an update
  still under way, which fails again and comes to the same place
  (`Engine::give_back`, `crates/conductor/src/settle.rs`). What it held
  before is not touched, whatever the failed steps did to it under the
  same name, and a child that was there and was applied over by those
  steps keeps what it was given: each of those was an apply in its own
  right, and undoing it is the blueprint's to say (`on_fail`).
- **An instance with no handlers is active only for what it started.**
  Steps that went well leave an instance active where it has handlers,
  holds something or has children alive, and done otherwise; one that is
  active for its children alone is done when the last of them has ended,
  in a write that asks again that it is still active, owes nothing, has
  no child alive and no run under way, and nobody has applied or stopped
  it since (`Engine::rests`, `Store::rest_instance`). So it ends the same
  whichever came first, its steps or what they started, and so does what
  it is a child of in turn; it is asked as each child ends, as the
  instance is written active, and of each active one as a conductor
  starts. A child stopped for its parent's stopping asks nothing of it.
  It is not let go of as soon as its steps end: what a parent with none of
  its own handlers attaches or calls and keeps alive (a slot it hosts, a
  feed it attaches: `examples/blueprints/logged-service.bp`) is matched
  by key when it is applied again, and is stopped with it, only while it
  is active. What may differ at a restart is whether it said it was
  active on its way, which the plays' judge says (`rested` in the
  runner's suite, and its `rested.rs`, which makes each order).
- **Applying checks everything the blueprint reaches.** A blueprint names
  others (`call`, `spawn`, `uses`), each of them a text read where it is
  when the step runs, and any of them may have been written for another
  build than the one reading it. So when someone applies a blueprint, every
  blueprint it names is read and checked against this device the same way,
  and the ones they name, each text once; what is wrong in one refuses the
  apply, said against the step that reaches it with the way there (`this
  step calls `a`, whose step `go` spawns `b`, which has 1 problem(s): ...`),
  and what a step gives a blueprint is held to that blueprint's `params`.
  Where what is wrong is an action or an argument this build has not got,
  it says the blueprint may have been written for a newer version, and
  which this one is. A name only known when the step runs is left to that
  moment; one that is not found is a warning, as it was. The check the
  editor asks for says the same (`crates/texts/src/reach.rs`).
- **A project's own text is run for an instance as it was vouched for**
  (`crates/texts/src/vouched.rs`). A blueprint runs with the authority
  of whoever applied it, which is said of a text; the texts it calls,
  spawns and attaches are read by name when the step runs, and a project's
  own are files that whoever may write the project's files can change, an
  agent among them. Read as it is at that moment, such a file would run
  with an authority nobody gave it. So whoever first asks for an instance
  to be applied vouches for every blueprint the project has, as it stands
  then, for that instance and what is started below it. Applying it again
  takes up the text applied and leaves the others as they were vouched
  for, saying which have changed, unless the asker says to take the
  project's files as they now are (`take`, `--take`): a file an agent
  wrote is not run as whoever happens to apply next. Whoever saves one
  through the daemon has vouched for it wherever they hold what was
  vouched with;
  and a step is given the project's file only as the nearest instance above
  it has it vouched for by somebody who holds all its instance runs with.
  A file changed since is run as it was, one taken away likewise, one that
  was not there is not taken up (what ships under its name is run, if
  anything does), and each is said: `instance.call_kept`, and beside the
  instance's status, with what to do, which is to read it and apply again
  saying to take it. What is
  vouched for is kept by instance with what its voucher held, so a weaker
  asker's applying over an instance adds to it and takes nothing away. An
  action that applies (`blueprint.apply`) and a text attached (`uses`) ask
  as the instance they are of, and vouch for nothing.
- **What ships is run as a running tree reached it**
  (`crates/texts/src/pinned.rs`). A new program has new texts under
  the old names, and an instance written for one release that started
  another's by name would be running a pair nobody had put together. So
  when somebody applies a blueprint, every text it reaches that came with
  the program or a package (`Library::ships`) is pinned for that instance,
  by name and revision, and the texts are kept by revision in the
  journal's key-value state. A step, and an instance acting for itself, is
  given the text pinned by the nearest instance above it; the look below
  an instance brings a child to what its parent has pinned, not to what
  the program brought. The pins move when somebody says so and not
  otherwise: an apply that says to take the texts as they now are (`take`,
  the same word that takes a project's own), or an apply by name that
  brings the instance itself to another text that ships (a text and what
  it names are released together). Any other apply leaves them, is checked
  against them and not against the texts now there, and answers with which
  have changed since (`reached.left`; `reached.taken` when it took them).
  A tree from before there were pins is pinned, when the conductor
  starts, to the last text of each name that checked for its project. A
  name only known as a step runs is pinned at the top of its tree the
  first time it is reached. A pinned text this program can no longer run
  is not gone on with: the one now there is tried. A file somebody writes
  where the factory runs (a project's own, held as it was vouched for; a
  development checkout's, read where it is) is not pinned. Wherever an
  instance is shown as behind, what moved between its text and the one now
  there is said in words (`behind.changed`, `blueprint::differs`, still given out as `conductor::differences`):
  each param, step and handler that is new, changed or gone, after the
  version each text says (`conductor::Moved`: `was`, `now`, `changed`, the
  same for a text an apply left or took). The texts that ship each say
  their version, and a version is one meaning: what each comes to under
  `blueprint::meaning` is written down by a test
  (`contract/versions.txt`, and beside a package's contract), which fails
  on a text that means something else under a number already written.
  What a tree reached before it was last moved is kept with what it
  reaches now (`Pins.before`), and `Engine::put_back`
  (`crates/conductor/src/back.rs`) puts it back: an apply by whoever asks,
  checked against those texts first and refused with what does not fit,
  of the text handed, or of the one the instance ran where its own ships.
- **How a running tree comes by new texts is a setting of the factory**
  (`crates/texts/src/taking.rs`), kept in the journal's key-value
  state and taken into a copy, with two parts. *How texts are come by*:
  `held` is everything above (pins, vouching); `follow` has a text that
  changes taken as it changes by every instance that runs it, with nothing
  pinned and no pin read; with nothing said it is as the texts are (what
  came with the program is held, and the blueprints of a checkout the
  program is run from are followed, a project's own being followed only
  where `follow` was said).
  `Engine::holds` is the one place that says whether a text is held.
  *How a taken text lands*: `in_place` is the rule as written; `again`
  has another text taken by the instance's steps being run again, whoever
  asks, the look below a parent included. Following gives up what
  vouching is for, so turning it on takes `shell` beside `admin`, and a
  project's own file is then taken by being vouched for
  (`Engine::vouch_saved`) as whoever turned it on, holding what they held
  then: nothing reads a file past the vouching, and an instance that runs
  with more than that goes on as it was vouched for. What says a text
  changed is not the conductor's: the node follows the journal for a
  project's files (`project.file.changed`, which a watch already says) and
  watches a checkout's blueprints with the one `hosting::Watch`, unless
  texts were said to be held (`crates/api/src/following.rs`); nothing is
  looked for on a timer. So a save of a followed file is answered when
  the file is written, and the text is taken soon after and not by then:
  each instance that takes it says `instance.updated` with the text's
  `revision`, and what is said from then on is that text's to handle.
  A text that does not check is not taken, and the
  instance says so once: beside itself, as `instance.text_left`, and by
  the factory's bot in the channel its tree names.
- **The words a blueprint links are held with the tree, as the blueprints
  it starts are.** A file a step links (`text.put`'s `from:`) is a text of
  the library, and one that ships is among what a tree reached when it was
  applied (`reach::places` lists it), so it is pinned by revision with the
  rest (`crates/texts/src/pinned.rs`). The step reads through the pin
  (`Engine::linked_for`), so a program that ships other words, with a
  blank the pinned blueprint never hands, changes nothing that is running;
  `--take` moves the words with the blueprints, and `--back` puts both
  back. The three places that took a kept text for a blueprint know which
  it is: whether a kept one can still be gone on with is, for words,
  whether what links them still fills their blanks, asked where they are
  linked (`Walk::linked`, and again as the step runs); whether two are
  one is, for words, whether they are the same words (`pinned::same`);
  and what moved is that the words changed, with the blanks that came or
  went (`words::differences`). Words reached only as a step ran, or by a
  tree from before words were held, are held from when they are first
  read (`pin_later`).
- **One blueprint is started several ways, side by side**, by a crate
  the conductor asks (`crates/comparisons/src/compared.rs`; what is
  asked is `crates/conductor/src/comparing.rs`). A comparison is said
  from outside (`/api/comparisons`, `compare` on the command line) of a
  blueprint by its name, and no blueprint is written for it: where a step
  calls or spawns that name, the child is started one of the comparison's
  ways. A way may name another blueprint to start in its place (its text
  kept by revision as it was when the comparison began), what to hand over
  what the step hands, both, or neither. What a start is for is said by a
  param (`per`) or is its place in the tree; a new thing is given the way
  that has had least of its share, written down before it is given, and
  the same thing goes the same way from then on. The child is put under
  the labels `compared` and `way`, as `labels:` would put it, and other
  blueprints named `also` carry a thing's way when they are started for
  it. What comes of a thing is counted where whoever starts the comparison
  says which events are about its things, by which of their fields
  (`Counted`, `crates/comparisons/src/counted.rs`): one follower of the
  journal, for as long as a comparison counts, measures each such event
  under the way its thing went (`system.comparison.came`, by the labels
  `counted` and `came`), with how long it was since the thing was given
  its way. The runner knows none of the events: the usual ones for a
  board's cards are a list beside the route, asked for by a word. Its place in the journal is kept, so
  what was said while the daemon was away is counted when it is back, and
  only what whoever started the comparison may be told of is counted. The
  place is taken as a comparison is started, before whoever started it is
  answered, and that what is counted has changed is kept for the follower
  until it next looks: every event said once a comparison has been started
  is counted, none said once it has been ended, and none said of a thing
  before it was given its way.
  Each thing is also counted once as given to each text of its way it is
  started under (`counted: given`), which is what anything added up under
  that column is divided by. Nothing is scored: what was measured is read
  by those labels, and laid out once for the command line and the client
  (`crates/comparisons/src/beside.rs`): a column for each text of each way, and
  rows of what came of the things, what was chosen, what was added up and
  what else was measured. What is added up is whatever is not a length of
  time, which a measurement says by how its name ends
  (`blueprint::labels::unit`: `agent.cost_usd`, `agent.input_tokens`); a
  series handed out says its `unit`, and nothing that shows measurements
  keeps a list of names. Ending
  one may say the way everything goes from then on, which stands until it
  is ended again. A way goes by the factory's setting for texts: held, its
  text is the one kept when the comparison began; followed, it is its
  file as it now is (the last that checked, where that one does not), and
  each text a way runs is noted on the comparison (`Way.texts`) and
  carried as the label `text`, so that what one text did is counted apart
  from what another did. A way may read other words where a step links
  a file (`Way.prompts`, `crates/comparisons/src/instead.rs`): two prompts
  for one step with nothing else changed. Which words is settled as a
  thing is started under the way (kept where texts are held, the file as
  it then is where they are followed) and written down by what started
  it and the key it was started under, so that the step that links the
  file, however far below, is given them by asking upward
  (`read_instead`), a restart included; they are part of what `text`
  says was run, so other words are another column. A way may instead be held to a version of what
  it starts (`Way.version`): the text that calls itself that, found among
  what is there now and what running trees are pinned to or were moved
  from (`Engine::text_at`), and started whatever the setting is; a version
  that is not kept is refused when the comparison is started.
  It is in force for a step only where whoever started it
  held what the step's instance runs with (`instance.call_kept` says so
  otherwise). A name with no comparison costs a call nothing but a look at
  a list in memory; a thing's first start under one is two small writes,
  and every later one a read.
- **A text that ships goes on as it last checked.** A running instance did
  not ask for the texts it calls to change. Each time a text is read for a
  `call` or `spawn` and checks, it is remembered for its name as that
  project sees it (in the journal's key-value state, written only when it
  differs; what an apply reached is remembered then). When the text now
  there does not check, or is gone, the step goes on with the one
  remembered and says so: the event `instance.call_kept` of the calling
  instance (`blueprint`, `key`, `error`, `kept`: the revision gone on
  with), and the instance says it of itself beside its status until a call
  of that name finds a text that checks. Nothing remembered, the step
  fails with what is wrong. (A project's own is stood in for only by what
  was vouched for, above.) A person who applies that text is refused all
  the while; a text that checks and then fails is a failure like any
  other. `blueprint.fit` measures, by the blueprint called, how long the
  reading and checking took and whether the text there checked. What is
  remembered does not go into a copy of a factory.
- **An instance says which text it runs, and whether it is behind.** It
  keeps the text it was applied with; the text under its blueprint's name
  may since have become another, and nothing tells it. Wherever an
  instance is shown (`/api/instances`, `/api/instances/{id}` and its
  children, `/api/fleet`, `mw instances`, the client) it says the revision
  of the text it runs, the `version` that text gives itself, and, when the
  text now there for its name as its project sees it is not the one it
  runs, `behind`: that text's revision and version, how the instance would
  come to run it (`takes`) and the same as a sentence (`why`). It is worked
  out when it is asked for, from the two texts, each name read once for a
  listing: nothing is kept and nothing looks on a timer. One applied from
  a text that is in no library is behind nothing
  (`crates/texts/src/behind.rs`).
- **One rule says how an instance that is there takes what is applied over
  it, whoever asks.** It is written here and nowhere else, and
  `Engine::reapply` (`crates/conductor/src/update.rs`) is what holds to it:

  1. The same text and the same params: nothing happens. A text that
     differs from the one it runs only in how it is laid out and in its
     comments is the same text (`blueprint::means_the_same`): laying a
     blueprint out is never an update, and an instance is not behind one
     that was only laid out again. The instance keeps the text it was
     applied with, as it was written then. Where its steps are still
     going, they go on: asking twice for what an instance is in the middle
     of does not do twice what its steps have done so far, and whoever
     asked is given the run that is going (`run` in the answer, with
     `changed: false`) to wait on. Steps that failed or were cancelled are
     not what was asked for: then it is 5.
  2. Asked with force: its steps run again.
  3. The text says `on_update` and the instance is active: it is carried
     over.
  4. The text begins as the one it runs does and the params are the same:
     it takes the text where it is; nothing is run again.
  5. Otherwise: its steps run again.

  That is what becomes of an instance when a person applies its blueprint
  under the name and key it has, when one instance is applied again as it
  is (`POST /api/instances/{id}/apply`, `mw apply --instance <id>`, the
  button on an instance that is behind: nobody but what started a child
  knows what it was started with, so this goes by the params, the key and
  the place it has), when a step `spawn`s a child that is there, and for
  everything below an instance somebody applied. What an instance that is
  behind says of itself (`takes`: `carried`, `in_place`, `again`) is which
  of 3, 4 and 5 it would be, and whoever applies is told which it was
  (`took` in the answer; the terminal and the client say it in words).

  Who asks changes two things and no more. *The look below a parent asks
  for nothing to be run*: where the rule comes to 5 it leaves the instance
  as it is, and behind. *A `call` asks for exactly the steps*: it is not an
  apply of something that should be there, it asks for work and waits for
  what the work gives. So a call is answered by 1 (the child is running
  that text with those params: nothing is run, and the caller is given
  what the steps gave when they last ran, or what they give when they
  finish if they are still going) or by the steps, and never by 3 or 4;
  and it runs them on the text now there, or the last that checked.

  Two things make "an instance that is there" one thing, however many ask
  and whenever. Looking for an instance by its key and making it when
  there is none is done by one asker at a time (`Engine::apply_from`), so
  two handlers that hear one event and both spawn the child for it are
  given one child; and whoever makes it holds it until its first run is
  written down, so the second asker finds steps under way and waits on
  them. And when the conductor starts, every instance is known to it
  before any run goes on (`Engine::resume`): a `call` that was under way
  asks for its child again, finds it there with its steps going on from
  where they were, and waits for them, as it was doing.

  And one that is being stopped is not there to be applied over. A stop
  takes a while (what the text says is done as it stops is done, what
  is under it is stopped, what it held is given back) and until its
  last write the instance reads as alive. An apply that comes to it in
  that while, by any of the ways above, takes nothing and changes
  nothing: it waits until the stop is over, and is then an apply where
  nothing is running under the key, which makes the instance anew. Both
  happen, in the order they were asked; the stop is not cut into or
  undone, and nobody is told `cancelled` for having asked then. A stop
  says that it is under way from before it does anything until it is
  written, and the rule asks that once it has the instance's turn, and
  again as it ends the generation where steps are to run again
  (`crates/conductor/src/meets.rs`). The stop takes no turn and waits
  for no apply: one that was already at work when the stop began was
  first, and is stopped as anything is. Where waiting would be the stop
  waiting for itself, the apply is refused in a sentence that says so
  and what to do: asked by what the instance or something under it
  does as it is stopped, or by anything that began and may be waiting
  for (a task carries which stops it is part of to the work it
  starts); or asked by a step of the instance or of something under it.
  What is not tied to the stop that way (something that hears of the
  stop by an event, and applies) waits. The look below a parent asks
  nothing of an instance that is being stopped, and says nothing.
- **Stopping an instance stops what is under it, and all of it stops
  reacting first.** What is under an instance is taken down a thing at a
  time, the newest first, each with what its text says is done as it
  stops done whole (`Engine::teardown`). Before any of that, and before
  anything the instance itself does as it stops, everything in hand
  under it is called off in one pass, a parent before what is under it
  and with nothing waited for between (`Engine::call_off`): each
  generation ended, its handlers taking no more, its runs cut, which
  end as cancelled runs do. Called off only as its own turn came, what
  was stopped later went on reacting to what the stopping of an earlier
  one did: a fleet's round, under a stage not yet come to, heard the
  turn of an agent that was stopped first called off, and wrote that
  its card had failed. A stop is done whole once it has begun, whoever
  was waiting for it (`conductor::whole`): a step that asked for one
  and was cut short left what it was stopping half stopped. And that a
  stop has begun is kept for the top of what is stopped, whatever it
  does as it stops, so that a conductor which starts finishes it, with
  everything under it brought into hand to be stopped and for nothing
  else, before anything is taken up (`Engine::finish_stops`).
- **What it is to begin as another text does, and to take one in place.**
  `blueprint::outset` is the text read as a blueprint (comments, spacing,
  quoting and the order of the top do not count) without `on`, `on_stop`,
  `on_update`, `permissions`, `version` and descriptions: two texts with
  the same outset do and settle the same when they are applied, and differ
  only in what they do later. An instance that takes such a text where it
  is has nothing run again; its params, state and status are kept; the
  text is its next version; it says `instance.updated` with `in_place:
  true`. A run under way finishes on the text it began with, also across a
  restart: each handler is told to take no more events and ends when it is
  next between runs, and the new text's handler of that name waits for it
  and then listens from where it had got to
  (`crates/conductor/src/trigger.rs`). The handlers are told so before
  the text is written as taken, and where a handler the text brings
  begins is taken before that write and written with it
  (`take_in_place` in `crates/conductor/src/current.rs`): so
  `instance.updated` is what says the text is taken, and what is said
  once it is said is the taken text's, handled by no handler of the text
  from before and heard by a handler the text brings. Told after the
  write, a handler went on to handle in the old text what whoever heard
  `instance.updated` said next. And it is taken whole or not at all,
  whoever was waiting: let go of by whoever applied once it has begun,
  it goes on as a task of the conductor's own with the instance's turn
  and comes to its end (`crates/runs/src/whole.rs`). The instance then acts as whoever
  asked; one who may not use what the text lists is refused, and nothing
  is changed. An update of it that may yet be undone is not added to: then
  its steps run again. A handler the text no longer has ends and gives up
  its place in the journal (as one does when an update's steps settle), so
  that a handler a later text brings back under that name listens from
  when it came back, and is not handed everything since.
  A handler that is ended, because its instance's steps run again or
  its instance is stopped, begins nothing more whatever it was handed in
  that moment: an event waiting behind the run that was cut is left
  where it is, for the handlers started when the steps have run again
  (`called_off.rs` in the runner's suite; it was taken two times in
  three by the handler that had just been ended, and lost). The same
  holds of what is called off under a stop: whoever stops an instance
  may go on to what the first thing under it says as it stops without
  giving way, and what waited for that is then looked at with both at
  hand (`cut_first.rs`). A step asks the same before it begins its
  action; once the action is under way, one that has ended as its run
  is cut is written as it ended, since nothing is begun by saying so.
- **What is applied over an instance is the text as it was read in the
  instance's turn.** Whoever asks for an instance to be brought to a
  text reads the text first and waits for the instance's turn at being
  applied over afterwards; somebody else may have the turn in between.
  So whoever asks notes when it began to read, and once it has the turn
  reads again there where the instance was given anything to run since:
  the text, what it reaches, the params as it takes them. The one rule
  is asked with that, and whoever asked is told what they would have
  been told a moment later, which is mostly that nothing changed. Read
  before the turn and applied in it, a text took an instance back from
  one applied in between: a step that asked for the child it has left
  it on the text from before, with a handler the later text had dropped
  (`update.rs`, `in_turn.rs` in the runner's suite). The look below a
  parent asks with the params a child has when its turn has come, and
  not those it was listed with.
- **A text taken because of an event hears that event.** A card is moved
  back to where it is planned; the handler of the fleet's stage
  spawns the proposal that card has; the proposal takes the text now there where it is, and
  nothing of it runs. If the handler that takes the card up is one that
  text adds, it must hear the move that brought it, or the card sits there
  with nobody acting. So when a step of a handler's run (a `spawn`) has an
  instance take a text in place or be carried over, the handlers the text
  adds listen from just before the event that run is for, and are handed
  it if it is one of theirs (`trigger::prepare`); the handlers it had
  listen on from where they had got to, as always. For an instance that is
  carried over that place is written with the text, so a conductor stopped
  before `on_update` has ended still knows it; if `on_update` fails, the
  places of handlers only the failed text has are taken away. Nobody
  asking because of an event (a person, the look below), what a text adds
  listens from then.
- **Applying an instance again reaches what its handlers started.** Its
  steps, where they run again, apply the children they name. A child a
  *handler* started (a review, spawned when a card came to its column) is named by
  no step that runs again. So when somebody applies an instance, once it
  is as asked (at once if nothing of its own was run, otherwise when its
  steps have settled), every live instance below it is looked at, and one
  that is behind is brought to the text now there by the rule, as whoever
  asked, with the one difference above: it is carried over (3), or takes
  the text where it is (4), or is left as it is and stays behind, with
  `takes` saying `again`, or `unfit` for a text that does not check here.
  One difference to that difference: a text that ships, which the look
  brings a child to as the tree above it is held to it
  (`crate::pinned`), is the text the tree goes by, and nothing else
  would bring the child there. So a child whose steps have settled has
  them run again for it (5), and what is under that child is looked at
  once they settle, and not before: what its steps call has its new
  text before they run, and nothing is brought twice. That is what
  taking what ships at the top of a tree comes to: the top and every
  part under it move to it, whatever began otherwise. One whose steps
  are under way finishes them on the text it began with, and stays
  behind until something above it is applied again. A take moves what
  is under it with it (`Engine::unpin_below`: what a part taken by
  itself was held to before is let go), and a part somebody took by
  itself is held to its own text from then (its own name among what it
  is pinned to), and to what ships now of every name what is above it is
  held to (`Engine::held_above`: a name only known as a step runs is
  held to at the top of the tree, and what is under the part started by
  one moves with it), so that what is above it, applied again without
  taking, does not move it back (`Engine::apply_over`).
  A refusal (whoever asked may not use what the text lists) is said as
  `instance.update_failed` and leaves the child as it was
  (`crates/conductor/src/current.rs`).
- **A text may say how a running instance takes it, in place of its steps
  running again: `on_update`.** Applying over an instance runs its steps
  again, which is what a rollout is and is wrong for something in the
  middle of long work. A text with `on_update` (a list of things to do,
  like `on_stop`) is taken another way by an instance that is active and
  whose steps were not asked for (with `force`, or by a `call`): it is
  *carried over* (`crates/conductor/src/carry.rs`), whether or not it
  begins as the text before did, since it may be there to put right what
  the instance holds. In order, and with nothing of the instance ever
  running beside it: its handlers are told to take no more events and each
  ends when it is next between runs, so a run under way finishes on the
  text it began with, and until they all have the instance is as it was
  (one that had only just been written as active has no handler yet to
  be told, and none is started for it until it has been carried over:
  started in between, they heard that the text was taken and were ended
  in the middle of that; how long their starting was held back is
  measured, `instance.listening_late`);
  then the text, the params and the authority are replaced together as by
  any update and the state is carried as any update carries it (a key
  still declared keeps its value, a new one starts at its default, one no
  longer declared is gone); what the text attaches with `uses` is brought
  up; and `on_update` runs, each entry in turn, each seeing what the one
  before set. `was` is what the instance was, read from what it would go
  back to: `was.version`, `was.revision`, `was.state` (with the keys the
  new text dropped) and `was.params`. The first entry that fails ends it,
  and the instance goes back to the text, the authority and the state it
  had, as after any failed update. Otherwise it is active, the new text's
  handlers listen from where the old ones had got to, and, when somebody
  asked, what is below is looked at. It says `instance.updated` with
  `carried: true`; the run is one of the instance's, with the flow
  `on_update`, measured as a run is. A later apply or a stop calls off one
  that is still waiting, and so does a text taken where the instance is.
  After a restart `on_update` goes on from the entry it had come to, with
  what the entries before it set (begun from the state the instance was
  carried over with only where it had not begun); one that was still
  waiting is carried over then, once what was under way in it has ended:
  whoever asked was answered, so what it was to take is kept with the run
  that will carry it over, from the write that makes the run until the
  text is taken or the run ends. Asked again for the very thing it waits
  for, the run there is answers. A first apply, and an apply
  over an instance that has not settled, run the steps and never this.
- **An instance acts as whoever applied it last.** A definition is a text,
  its params and who applied it with what they held then (their scopes;
  never a token). The three are written to the instance together, kept
  together as each version, and put back together when an update fails; a
  run takes the text and the authority together when it starts and keeps
  them to its end. So no text ever runs with the authority of whoever
  applied another, and applying over a running instance gives the asker
  nothing they did not have (`crates/conductor/src/update.rs`).
- **Everything is recorded before it is done**, so the daemon can be killed
  at any point. On start it resumes runs where they were: finished steps are
  not repeated, a step that saved a checkpoint is continued, one that did
  neither is repeated only if its action says that is safe. A `for_each`
  is the same thing element by element: the list it was worked out to is
  saved as it begins, each element is written down as it begins and again
  with what it gave (`step_elements`, emptied of a run when the run ends),
  and it goes on from the element it was at, over the list it began with.
  That element is picked up as a step is; one that had not begun begins.
  A `call` that was under way finds its child there, with its steps going
  on from where they were, and waits for them (*Instances and runs*).
  Before a `call` asks for its child it writes down how many instances
  there had ever been, so that the child it then makes can be told from
  any before it, whatever else was or was not written down when the
  conductor stopped. Picked up again, the call does not ask anew: it
  finds the child it made by that, running or not, waits for its run and
  is given what it gives, or fails as it fails
  (`crates/conductor/src/child.rs`). Asking again by name finds a child
  only while it runs, and one that ended between the looking and the
  asking was made a second time.
- **A run's steps are read in the order they began.** When a step
  began is kept in milliseconds, and several steps of a run begin in
  one, so it is not what they are read by: read by that and then by
  name, a run that did `greet` and then `child` was listed `child`,
  `greet`, on a run's page and by `logs`. A step's row says which of
  its run's steps it was to begin (`nth`), given by the write that
  first keeps the row, inside that write's one statement, as one above
  the highest its run then has, and never another afterwards
  (`runs::put`); the rows are read by that alone (`IN_ORDER` beside
  it). So a step whose record is written again (as it ends, tried
  again, taken up after a stop) is where it first began, a clock put
  back moves nothing, and the steps of a task that is done again, whose
  rows are taken away and written anew, come after everything that
  began before they were, each with a place no step of the run has.
  Steps that are passed over are written with their run's next write
  and before it, and the steps of a run that keeps nothing with its
  ending, each lot in the order it was held in (`run::Held`), which is
  the order they came and what they are shown in meanwhile. A row a
  program from before wrote says nothing and is read as it was, by
  when it began and then by name, before those of its run that say.
  The number is an order and is handed to nobody. Giving it looks
  through the run's rows once for each step, as the step is first
  written and in that statement, and reads each to its end: 5 µs for a
  run of thirty steps that gave little, and some 19 µs for each step
  of the run that gave a hundred kilobytes, where the pages are in
  memory; a step written again looks through nothing. The column is added
  by a step that reads no row of the table: on a journal from before of
  525 MB (200,000 runs, 600,000 steps) it took 5.8 ms read from the
  disk with nothing of the file in memory, 16 KB of it read, and 4.5 ms
  with all of it in memory. A project moved to a journal of its own
  takes it with the rows, and a copy has it
  (`crates/runs/tests/suite/ordered.rs`,
  `crates/conductor/tests/suite/ordered.rs`).
- **An instance may keep nothing.** A blueprint that says `durable:
  false` is run by the same conductor doing less: `RunCx::kept` is false,
  so nothing is written for a step as it is done, no checkpoint or element
  of a loop is saved, what is said of a step is passed on and not kept,
  and its state is in memory alone (`Engine::set_state`). What each step
  did is held on the instance while the run goes (`run::Held`), shown from
  there, and written in one piece of work with the run's ending: one
  write for a run, where a kept run makes several for each step. Its runs
  are still rows, with how they went, and it is still an instance in the
  table while it lives. When
  the conductor starts, such an instance is stopped before anything else
  is picked up, with what it started and held (`Engine::resume`).
- **Stopping the daemon does not stop the fleet.** Services run in their own
  process groups and are found again by pid and start time. What is
  started in a program's place is a wrapper, which is this program under
  a word of its own (`hosting::wrapped`): it starts the program from a
  list of arguments, waits for it, writes what it prints without what it
  was handed, says how it ended in a file, and stays when its group is
  asked to stop until the program has ended. Nothing the daemon does for
  itself is a shell or `kill` run as a program: the wrapper is found by
  where this program is, and a group is stopped by a call to the system
  (`hosting::group`). This is run on every change on Linux only. On Windows a
  group is a job object that ends what is in it when the last hold on it
  is let go: compiled for Windows from Linux, run there by hand once,
  and held by no test. The wrapper makes its job and puts itself in it
  before it starts its program, so a group ends with its wrapper, and a
  wrapper is stopped by being left word in a file it looks for
  (`group::word`, `group::hears`); a program the daemon speaks to over
  pipes, which has no wrapper, is put in a job the daemon holds
  (`group::led`). No process is ended by its number: the one safe door
  to that (`sysinfo`'s) runs `taskkill` as a program. A stop there is an
  ending, with nothing asked first; `crates/hosting/src/group.rs` says
  what else differs.

- **A port a service is given is nobody else's.** A service that names
  no port is told one before its program is there to take it, and the
  program takes it again each time it is started again. It was given by
  asking the system for a free one and letting go of it, and the system
  gives a number it was handed back to whoever asks next: something else
  was given the service's port before its program had listened there,
  the program ended at once and was started again, and what answered at
  its address meanwhile, to its health check and to what a router sent,
  was the other program. So a port is given from numbers no system gives
  by itself (`hosting::ports`: 21000 to 25535; Linux gives from 32768 to
  60999 unless told otherwise, the others from 49152), and is spoken for
  from before it is given until its service is looked after no longer,
  so that no other factory on the device gives it either, nor this one
  twice: by a socket the system holds for the daemon and takes back when
  the daemon ends, at a number that is the port's alone and not on the
  port, where it would be in the program's way. A daemon that finds a
  service again speaks for its port; one an earlier program gave a port
  keeps the one it has for as long as it lives, spoken for by nobody. Whether something listens on a port
  already is asked by connecting to it: a listener of the daemon's own
  there, for a moment, is held for a moment longer by any program the
  daemon is just then starting. Not held: a port its owner named
  (`port: 8080`), which is the owner's to keep free; a service's port
  while its daemon is stopped, which nothing speaks for then, so that
  one whose program has not come to listen yet is another factory's to
  be given; a program that
  takes a given port because it was told the number; a system told to
  give out ports from a range that reaches into these; and the two
  addresses that are still asked of the system and then remembered, a
  router's own and the daemon's, which are held while the daemon runs
  and are the system's to give while it does not.

- **A chain of events that goes round is stopped.** What is said while an
  event is being handled is one deeper than that event, in the same chain
  (`kernel::chain`): the handler's run is done within the chain, each of
  its steps and each child it starts is given it, and the journal reads it
  as it writes an event down, whoever says the event, so an action's own
  events and a child's `instance.finished` are in it without being told.
  The journal remembers the place of each of the last 65,536 such events,
  in memory. A handler does not take an event that is 100 deep in a chain
  that began less than 10 s ago (`CHAIN_DEPTH`, `CHAIN_WITHIN_MS` in
  `crates/conductor/src/trigger.rs`): it moves past it and says
  `handler.runaway`. A chain older than that begins again at one. What an
  agent says in its own turn, and what another device says, begin chains
  of their own: both are far too slow to go round.

## The journal

One SQLite file per factory (WAL), written by one thread. It holds events,
measurements, key-value state and the conductor's tables (kept by the
crate `runs`, below the conductor). Step output goes to
plain log files beside it. A project that is added has a journal of its
own beside it, a file with a thread of its own, in which nothing is
kept yet (*A journal for each project*, below, says what there is and
what is to come); everything in this section is so of each.

- **The rest of the work is done on a few threads**: one for each
  processor up to eight, unless whoever starts the daemon says how many
  (`--workers`, `mw help workers`). Nearly all a daemon does is wait, and
  a thread that waits for work is a stack and a share of memory all the
  same.

- **The one thread is measured.** Everything that reads or writes the file
  takes its turn on that thread, the conductor's work behind a channel's or
  a page's. `db.wait` is how long each piece of work waited for the work
  ahead of it and `db.work` how long it then took, by whose it was
  (`caller`, the crate that asked: `runs` for the runner's own rows,
  `conductor` for what else it keeps), by whether it wrote or only read
  (`kind`), and by which file (`db`: the journal, or the search index, which
  is a file of its own). `mw metrics db.wait --by caller` says who is
  waiting; `mw metrics db.work --by caller,kind` says behind what. Of
  work that shared its commit with other work (below), `db.shared_count`
  is how many pieces the commit kept, whose worst is the most there
  were, and `db.held` how long a piece was done before the commit was
  made and it was answered. A journal whose numbers come from a giver
  kept in another database (a project's, in a file of its own) asks for
  its next run of numbers ahead of need, and stands still for one only
  where it is not there yet: `db.draw` is how long, measured once for
  each run it takes into hand, and is the first run after it is opened
  and nothing after that unless half a run was said sooner than the
  factory's database could keep a row.

- **The log is brought back into the database off that thread's turn.**
  What is written goes first into a log beside the file (the WAL) and is
  brought back into the file itself now and then, with both held to the
  disk. Left to the database that is done by whoever commits the write
  that makes the log a thousand pages long, on the one thread: measured
  with a board of 120 cards worked by eight at once, five sixths of what a
  write took was that, and the longest waits anybody had (a quarter of a
  second and more) were the thread stopped in it. So the thread is told
  how long the log is by each commit and tells another, which has a
  connection of its own, each time the log has grown by a thousand pages
  (`journal::checkpoint`); that one brings back what it can without
  holding anybody. It is being told and not a timer: a journal at rest
  tells nobody. Where writes never pause the log cannot begin again, so at
  32,000 pages (64 MB) the writer brings back what is left itself,
  between two pieces of work, and the log's file is never left longer
  than that. Measured the same way, the 120 cards were done in 9.4 to
  9.9 s where they had taken 17.5 to 18.3 s, and a card of another
  project made ready meanwhile took 104 to 137 ms where it had taken 270
  to 350 ms (and 96 to 113 ms with nothing else going on, where it had
  taken 127 to 186 ms). What a loss of power can take is what it was: the
  log is held to the disk when it is brought back and not when a write is
  answered, so it is what was answered since the log was last brought
  back, about a thousand pages of it, before and now. A daemon that is
  killed loses nothing.

- **Where that thread has fallen behind, what is waiting shares a
  commit.** With the log brought back elsewhere, what is left of a
  write's cost is half its own work and half the commit that keeps it:
  the pages it changed written to the log whole, though the next write
  changes the same ones. So with sixteen pieces of work or more waiting
  as a commit is begun, the thread does what is waiting in the one
  transaction and commits once (`journal::gather`: sixty-four pieces at
  most, for two milliseconds at most; nothing ever waits for company).
  What whoever asks is promised is what it was. Each piece of work is
  whole or not at all, in a whole of its own inside the commit; nobody
  is answered, no event is the journal's last and no follower hears it
  before the commit is made, and where it cannot be made every piece in
  it is told so; events are numbered in the order they are written; and
  a place the journal was told (`down --at`, a simulation) ends the
  commit it falls in, so that what was asked after it is not kept with
  it. What a loss of power can take is not changed by it, and a daemon
  that is killed still loses nothing that was answered.
  Measured with the journal alone (20,000 writes of a kind, by one,
  eight and sixty-four asking at once): a step with its event is kept
  7,000 times a second for one asker and 15,000 for eight, as it was,
  and for sixty-four 26,000 where it was 11,000; a record with its
  event 47,000 where it was 14,000, an event alone 66,000 where it was
  23,000. Measured with the board of 120 cards it comes to nothing that
  can be told from two runs differing: the cards were done in 8.0 to
  8.7 s where a commit each took 9.0 to 10.6 s, a card of another
  project meanwhile in 95 to 133 ms where it took 95 to 126 ms, and one
  piece of work in nine shared its commit, thirty-two at a time. The
  journal's thread is then at two fifths of a processor and the rest of
  the daemon at one: what a card costs there is no longer the journal's
  to give. Sharing among whatever waits, however little, was measured
  too (it is how the suites are played with this forced on,
  `JOURNAL_GATHERS=all`): with eight asking at once the journal alone
  keeps a third to a half more, and the board came to the same again
  (8.2 to 9.3 s, 106 to 117 ms) with four pieces of work in five
  sharing a commit. It is not what a daemon does because whoever
  shares a commit is answered when all of it is done, and nothing
  measured in a factory at work paid for that; how deep is behind
  (`gather::DEEP`) is the figure to lower if a load ever does.

- **What a write costs the disk is whole pages**, so they are small and
  few. A row or two is changed and every page they and their places in
  the indexes are on is written again, whole; nearly everything written
  here is that small. A database is made with pages of 2048 bytes; an
  event has a place in the index of topics and nowhere else (one event in
  512 is in the index of when, which is all that pruning needs; a journal
  made now keeps no index of its events by instance or by run, each of
  which was a page of nearly every write that says anything, since
  nothing reads by them but what is taken up after a restart, which says
  where to begin: `journal::events::UNKEPT`, and *What one card costs* in
  [build-times.md](build-times.md)); the journal gives
  an event its number itself, where the database kept a page of its own
  to remember the last one it gave (the numbers it gives are drawn a run
  at a time from the factory's one giver, whose mark is one row written
  again in the write of one event in 1,024: *A journal for each
  project*); a step that was passed over, and one whose action only
  decides, is written with its run's next write; and an instance's row,
  which is written again as its state and its status change, keeps none
  of the text it runs, which the version that has it keeps once. How big a page is, and whether the
  database numbers the events, are decided when a database is made: a
  factory made by an earlier program goes on exactly as it was, giving no
  number twice, until its owner asks for it to be written out again as
  one is made now (`mw repack`, `mw help repack`), which is done while it
  is stopped, beside the one there is, and leaves that as it was if it is
  cut off. Measured on a journal of 296 MB and 840,000 events, that took
  2.7 s. The one thing an upgrade does to such a journal by itself is the
  index of when, made anew as the daemon first starts on it: 1.2 s for
  the same journal read from the disk and 0.1 s with it in memory, with
  14 MB written once. The daemon says each once (`system.repacked`,
  `system.reindexed`) and measures it (`db.repack`, `db.reindex`).
  Neither can be gone back on: the first start of a program on a journal
  an earlier one made is one way, since the earlier program refuses a
  journal whose tables are newer than it knows. Going back takes a copy
  of the factory made beforehand (`std/snapshot`, or the data folder
  copied while the factory is stopped).

- **A database's tables are made by lists of steps that only grow at
  the end.** Each crate that keeps tables says how they are made as a
  list of steps, under a name of its own (`Db::migrate`: the journal's
  own, the runner's, which are still called the conductor's, nine of
  the studio's, the agents', the hosting's; the search index and a
  store are databases of their own, each with a list). A database keeps
  how many steps of each list it has run and runs those after, which is
  all an upgrade is, and why going back is refused: it has run more
  steps than the earlier program has. So a step that is there is never
  changed, moved or taken out, and a new one is added after the last. A
  step written in before the last is never run by a database that was
  past its place, which runs the last a second time instead: a factory
  that had run every step of its records' tables was handed such a list,
  its database refused the last step as done already, and it did not
  start; had that step been one that can be run twice, nothing would
  have been said and the new one left undone for good. Every test had
  passed, since a test makes its databases new. It is held twice. What
  each step of each list comes to is kept with the code
  (`contract/steps.txt`), and a test fails on a list that is not what it
  was, with which step and why; it reads the lists from the databases a
  node makes, so no list is left out by being forgotten. And a database
  keeps the same thing of the steps it ran (`journal::steps`,
  `schema_versions.steps`): handed a list whose first steps are not
  those, it is refused before anything is run, in a sentence that says
  which tables and which step, and is not changed. A step changed where
  it stood and meant to be is written down with why
  (`contract/steps-changed.txt`), which is built into the program so
  that a database that ran the step as it was is not refused. A
  database from before this keeps only the count: the first time this
  program opens it, it is given where the rest is kept (a line of what
  it says of its own tables and no row: about 15 ms, once, for a file
  of 300 MB as for an empty one) and is taken to have run the steps
  this program has, so that first time it has nothing to go by. A list
  whose every step was run is not a write, where it was a row written
  again at every start; what its steps come to is worked out each time
  it is handed over, 0.02 ms for 26 KB of steps.

- **One event schema** for everything, with a number that no other event
  in the factory has. Some events are stored; some (log lines, progress)
  are only broadcast. What the number promises, and what it does not, is
  *A journal for each project*, below.
- **The client never polls.** It reads what it needs once and follows
  `/api/events`, which goes on from the last place it was handed: text the
  daemon wrote and takes back as it was (`crates/api/src/cursor.rs`),
  never an event's number with something added. On the
  old system the hub's load turned out to be mostly browser tabs asking
  "anything new?" on timers; here the only polling that exists is what a
  blueprint asks for explicitly: a `poll:` trigger, with a floor of five
  seconds, jitter and backoff, and a service's health check. The daemon does
  not look for anything on a timer that the system can tell it; see *Being
  told, not looking*.
- **Measurements** are per-minute rollups with a histogram, written every ten
  seconds: every step, run, HTTP request (by route and by kind of caller),
  agent turn, health check, routed connection, poll, request to a store
  service, event handed to its handlers, snapshot taken of the factory (and
  how long every journal stood still to be read for it, `snapshot.stood`), and
  this device's load and free memory. A step can also name itself (`measure: build`) so "how
  long do builds take" is one query. `mw metrics`, the Measurements view and
  `/api/metrics` all read the same thing. Whatever is measured for a project
  carries a `project` label (a service and a router remember the project
  they were started for), so one project's numbers are the same query with
  `labels=project:<id>`; requests and the device's own numbers carry none.
  A label said more than once is any of the values it is said with
  (`labels=project:shop,project:blog`: what was measured for either, added
  up as one), since nothing carries a label twice; that is how the
  Measurements view asks for the projects of a workspace together, the
  daemon being what can say what is typical of several.
- **A blueprint can say more about what it starts**, with `labels:` on a
  `call`, a `spawn` or a task: names and values of its own choosing
  (`experiment`, `variant`), kept with the instance that was started and
  carried by everything measured under it, an agent's turns among it
  (`crates/conductor/src/labels.rs`). They are kept in a measurement beside
  what it says of itself and asked by in the same way, `by=variant` and
  `labels=experiment:cart-12`, which is why a label may not have one of
  those names (`blueprint::labels::TAKEN`). `/api/metrics/labels` lists
  the labels there are; the Measurements view sets what was measured side
  by side by one of them.

- **The database library keeps no count of the memory it has in use.**
  SQLite adds up what it has allocated, for its own figures of memory
  used and for a limit somebody may set on its heap, and takes a lock
  to do it at every allocation it makes: one lock for the whole
  program. Each journal has a thread, so a factory with a journal for
  each project had every one of them that was doing anything wait in
  line for it, and the more of them the worse (a test program with
  thirty-two at work spent twenty times as long in the system as in its
  own code: [build-times.md](build-times.md)). So the library is built
  with the count off (`SQLITE_DEFAULT_MEMSTATUS=0`, said as
  `LIBSQLITE3_FLAGS` in `.cargo/config.toml`, which every build made
  through cargo in this repository reads, the packages and the builds
  for other systems with the rest). Nothing here reads the count. What
  is given up is what goes by it: the library's own figures of memory
  in use (`sqlite3_memory_used`, `sqlite3_status`) say nothing, and a
  soft or hard limit on its heap (`PRAGMA soft_heap_limit`) holds
  nothing. Nobody is to turn it back on to read a figure: what a
  database's thread takes is measured where its work is (`db.work`),
  and what the daemon holds is the system's to say. A build made where
  cargo does not read that file (the crates built from another folder)
  has the count on and is only slower for it; a test of the journal's
  suite asks the library how it was built and fails there, on every
  system the suite is run on.
- **A factory that is run always waits for the disk** where the journal
  says it does (`journal::checkpoint`, and a commit asked to be on the
  disk before it is answered). One made for a test says it does not
  (`journal::disk`), since no test can take the power away and the
  waits were most of what such a test took. Nothing that is run is
  told so.

### A journal for each project

One journal, written by one thread, is what everything is kept in
today, and it is not what there will be. Everything a factory writes
takes its turn on that thread: a runner in one project with a great deal to write holds up a
card being moved in another, and a channel, and a page. That is not
acceptable between projects, and a single writer is what gives way first
as a factory is given more to do. So the decision is made, and what is
built from here is built towards it:

- **A project is the seam.** A project is to have a journal of its own
  (a file, and a thread that writes it) and a conductor of its own that
  runs what was applied for it, under the factory's, which keeps what is
  nobody's: who is in the factory, its settings, what is said of the
  daemon itself, and what was applied for no project. Nothing above a
  project is the unit. A workspace groups projects and channels under a
  name and keeps nothing apart: what is said of one is nobody's, and a
  channel that belongs to a workspace is no project's. It is the project
  that work is kept apart by, because that is where the work already
  divides: a tree of instances never spans two projects (a child
  is its parent's project's, and an instance's project is the one it was
  applied for and nothing else), a pool is a project's and a column's,
  and a card, a record, a file and an agent are each in one.
- **What is written in batches is a second thing, and both are wanted.**
  Gathering what one thread writes into fewer writes makes that thread
  go further; it does not keep one project's work from another's. The
  two do not stand in for each other.

What makes the division possible is that nothing outside the program
may depend on there being one journal. That is the first of the work,
and is done:

- **What is heard is by project.** A handler or a `wait` that says
  nothing hears its own project's events and the factory's own, and
  hears every project's only by saying `scope: fleet`
  (`crates/conductor/src/whose.rs`; *Whose an event is* in
  [blueprints.md](blueprints.md)). So what a project's conductor has to
  be handed is what is said in its own journal and in the factory's, and
  the few things that say `fleet` are the only ones that follow every
  journal. Whose an event is, is one function: the project its data
  names, else that of the instance that said it, else nobody's. An event
  of something that is a project's and names no instance says its
  project (an agent's turn, something said in a project's channel); a
  build made to be tested holds every node to that
  (`crates/api/src/whose.rs`), and the API suite holds, of a card taken
  to done, a feature planned and accepted, an agent answering in a
  project's channel and a comparison counting, that all that was said
  was the project's but what is the factory's own
  (`crates/api/tests/suite/whose.rs`).
- **A text from before goes on as it was meant.** An instance applied by
  a program that knew no such word heard every project's events where it
  said nothing, and still does until it is applied again: its row says
  how it hears (`instances.heard`, with the definition it would go back
  to), and one that says nothing is from before. Nothing changes under a
  running factory because the program was replaced.
- **A place among the events is text.** What follows events from outside
  is handed a place and hands it back unread, so that a place can come
  to be one in each of several journals with nothing outside changing.
  It is that now (`journal::Places`, written out by
  `crates/api/src/cursor.rs`): for each journal the number of the last
  of its events that was dealt with, and one number for every journal it
  does not name, which is nought for a place taken now, so that a
  journal made afterwards is read from its beginning. A number alone is
  the place above it in every journal, which is what a place kept or
  handed out while there was one journal means and what an event's own
  number means as a place: the giver only goes up, so whatever a journal
  made since holds is numbered above it.
- **What reads the events for the whole factory is handed what every
  journal says** (`journal::every`), and keeps a place in each. A
  follower (`Journal::follow_every`) is handed each journal's events in
  that journal's order with no gap and no duplicate, the journals' as
  they come, and takes up a journal made meanwhile from its beginning.
  What is read to be shown (`read_every`, `read_within`, `read_after`)
  is put in the order of when each thing was said, to the millisecond,
  and two said in one millisecond in the order of their numbers, which
  between two journals is no order and only the same each time. Nothing
  is ordered by number across journals. Each of these was one reader of
  the one journal, and is this now:

  - the stream a client follows (`/api/events`): every journal, each
    frame's `id` the place just after its event;
  - what was said of late (`/api/events/recent`) and the fleet's one
    read: the place that is now is taken first, and what is read is
    what each journal holds up to it, newest first by when it was said;
  - search: what it has read to is a place in each (`search/cursor`),
    and it catches up from each;
  - a record's history: followed from a place in each
    (`history/seq`), and a table that has only now said what its
    history is, is given what each journal held up to where the
    follower then is in it;
  - an agent asked in a channel (`agents/cursor`), the files looked at
    when a run ends, a table declared or a project removed, a text that
    changed, and what a member tells its hub of services and instances:
    every journal, from now;
  - a board held against what was last said of its cards
    (`Board::settle`): the newest of what any journal holds, and "when
    the daemon stopped" the newest thing any of them held as it was
    opened;
  - whether a message was ever said, and what was said in a channel
    that is taken away: asked of each journal, and taken out of each.

  The places these keep in the small durable state were numbers, and a
  number kept by a program from before is read as what it meant.
  While nothing is kept in a project's journal all of this comes to
  what it was.
- **The runner hears from several journals.** The events of each
  journal are handed out by a task of its own (`runs::delivery`),
  and a handler waits in each journal it hears, with a place in each
  (`delivery::Inboxes`, `Engine::hears_from`): its own project's and
  the factory's own, which keeps what is nobody's, and every one where
  it says `scope: fleet` or its text is from before and hears
  everything. So a handler of one project is handed nothing another
  project's journal says, and reads none of it when it catches up.
  Each journal's events reach it in that journal's order; of two
  journals' nothing is promised, and neither waits behind the other: a
  journal it has still to read is read before any is waited on, each
  in its turn. Its place in a journal moves in the write that records
  the run an event of that journal starts, as it always did, and its
  place in another is not touched by that. A handler a text brings
  begins now in each journal it hears, and just before the event that
  brought the text in the journal that keeps that event
  (`trigger::places`, `Journal::keeping`). A journal made while a
  handler listens is one it reads from its beginning if it hears it:
  everything in it came after. Whenever it was made: a handler looks
  for one each time it comes to wait, and not only while it waits, or
  one made while it was in the middle of a run was never read by it.
  Where a handler that waits for several
  things to go quiet (`debounce`) lets go of an event of one journal
  for a later one of another, its place in the first is written before
  the run is, so that what it let go is not handed over again after a
  restart; stopped between the two, the later event is handed over
  again and runs, which is what was owed.

  A `wait` keeps a place in each journal too, in its step's record
  where it kept a number, and follows every journal from there, passing
  over what is not within its reach as it did; whether it had said
  what it says as it begins to listen is asked of whichever journal
  keeps that. A comparison's count is kept the same way. A number a
  program from before kept in either is read as what it meant. One
  thing is still a number in the journal a run says things in: where a
  step, an element of a list or something done around a step began,
  which moves with the runner's rows.

  It costs what it did where there is one journal: an event that starts
  a handler's run is seven pieces of work and one that is passed over
  one write in 256, which the conductor's `costs.rs` still holds.
- **Numbers come from one giver** (`journal::giver`), apart from any
  journal's events. A journal draws a run of 1,024 numbers at a time
  and gives them out itself, in order, to what it writes; how far the
  giver has given (its mark, one row of the small durable state of the
  journal it is kept in, which is the factory's) is kept before any
  number of a run is an event's. The journal that keeps the mark, which
  is the one journal there is today, raises it in the write of the
  run's first event, so that one commit keeps both and drawing is no
  write of its own. A journal whose giver is kept in another database
  (a project's, in a file of its own) asks that database to raise it,
  and the run is its own once the mark is raised and on the disk
  (`Db::call_to_the_disk`: that one commit has the log held to the disk
  as it is made, where every other is held to the disk when the log is
  brought back). In one file a loss of power keeps the mark and the
  events it covers or neither; in two it could keep a project's events
  and lose the mark above them, and the next start would give their
  numbers again. Such a journal asks ahead of need: with half the run
  in hand given it asks for the next and waits for nothing, so its
  thread does not stand behind the factory's in the middle of a write,
  but for the first run after it is opened. What it says though it has
  shut (that the daemon stopped) is drawn for though the factory's
  database is doing nothing more by then.
  So no number is given twice wherever the daemon is stopped, and what
  was left of a run is a gap: there is one at every start. A write that
  failed and a commit that could not be made give their numbers back,
  and a run they drew with them, once.

  A journal that begins to draw here holding numbers of its own (one
  that numbered its own events) says how far they go as it is opened,
  and nothing at or below that is given from then on by anybody: not
  from a run drawn later, and not from one another journal had in hand,
  which lets go of it as it next writes and writes down the gap
  (`Giver::above`, `Giver::is_given_away`).

  Measured with the journal's own count of its work (`Db::writes`, and
  the length of its log with nothing brought back), 20,480 events said
  one at a time: one piece of work that wrote for each, before and
  after; 4.9436 pages written for an event of a run before and 4.9446
  after, 2.3503 and 2.3513 for one that names no run, which is the
  twenty runs drawn, a page each.

  A journal an earlier program made is taken up as it is, with nothing
  written and nothing read that opening one did not already read: it
  has no mark, the giver begins above the highest number it ever gave
  (the events', what was written down as events were taken out, and
  what the database remembers where it numbered them), and the mark is
  first kept with the first thing said. Opening one of 267 MB and
  840,000 events took 0.5 to 0.8 ms before this and after, whichever
  way its events' table was made. A copy of a factory takes the mark
  with the events, and gives nothing the copy came with.

  What goes by an event's number goes by its order and never by its
  being one above another: a handler's place and what it is owed since,
  a follower's, the stream's, a comparison's count, the search index,
  a record's history. What is counted in events is counted: the
  newest 50,000 are kept however old they are, and which those are is
  worked out over the gaps, which a journal writes down as it passes
  them (with the first event after each, in that event's write: the
  first number after the gap and how many were passed over, the newest
  256 of them; `numbered::floor`). Two things count numbers where they
  mean events, and are the looser for a gap, which today is a run at
  most at each start: a handler that passes events over has its place
  written when it is 256 numbers on, so at most once more after a
  start; and one that hears a pattern reads the journal a stretch of
  256 numbers at a time when it catches up, so at most four stretches
  more.

- **The detail of runs' steps is kept for as long as the owner says.**
  Unsaid, for ever, as it always was (beside which every event older
  than fourteen days goes but for the newest 50,000, above). Said as a
  number of days (`journal::letgo`, a setting kept in the factory's
  journal), what is older goes once an hour and as the daemon starts:
  the files runs' steps printed into, judged by when the system says
  each was last written, and of the runner's own events those it says
  of steps and runs as they go (`conductor::letgo::TOPICS`), in every
  journal, a piece of the database's work at a time. Where the old ones
  end is found from the index of when, as pruning finds it; events
  taken from the middle of a journal are holes the gaps do not say, so
  the journal keeps that it has them (`numbered::HOLES`) and the newest
  so many are then counted from its rows. What a run came to stays: its
  row and its steps' rows, and `run.finished` and `run.failed` where
  they were kept. All of these are kept in the first place only where a
  text in force could be owed them (`conductor::owed`, above), and are
  passed on otherwise, so what is let go is what something heard and
  has been handed; a journal that passes them on keeps that it has holes
  already. Nothing goes
  that is owed: nothing after the least place a handler whose text
  hears one of those topics keeps in a journal, which for one with no
  place there is its beginning; nothing anywhere while something alive
  waits for one, is not in hand, or a conductor is taking everything
  up; nothing after where a comparison that counts one counts from; and
  nothing of a run under way, events or files. Where runs' steps write
  is the other half of the setting: a folder of the owner's, under
  `runs/` as in the data folder, with the folders said before kept so
  that a step's file is read from the first that holds it. It is no
  part of a copy, since two factories never write in one folder; how
  long is. The databases stay in the data folder: moving them is
  another piece of work.

- **What is kept for what runs is told apart by project**, inside the
  one file there is (`runs::apart`). A project's rows are to be moved to
  its own journal and taken up by its own conductor, and neither is to
  be a matter of working out whose a row is: so every row says its
  project, or is reached by its own key from one that does. Table by
  table (`apart::TABLES`, which a test holds every table the runner
  keeps to, and every row of each):

  - an instance says (`project`, as it always did: the project it was
    applied for, and nothing for one that is nobody's);
  - a definition an instance was given (`instance_versions`) is its
    instance's, which is the first of its key;
  - a run says (`project`, the empty text for one that is nobody's),
    copied in the write that makes it from what the database keeps of
    its instance and never handed in. A run that had ended in a journal
    from before says nothing, and is reached through its instance,
    which every run names: so one that says nothing is told from one
    that is nobody's;
  - a step, and how far a loop of one has come (`step_elements`), is
    its run's, the first of its key. A step is found by its run and
    never by project, and goes where its run goes;
  - a handler's place says, as a run does, and says which journal it
    is a place in (below);
  - what is kept in a column of one of those rows (how far what is
    done around a step has come, what an instance owes, what a run is
    to carry its instance over to, the labels an instance carries) goes
    with its row.

  It rests on two things, each held where the rows are written and by
  the database itself (rules made with the tables), whoever writes. A
  tree of instances is of one project: an instance asked for under one
  of another project is refused in the write that would make it
  (`Store::insert_instance`, in a sentence). And what says whose a row
  is, is said rightly and once: a run or a place that does not say its
  instance's project is refused as it is written, and an instance's
  project, a run's instance and what a run or a place said are refused
  another value afterwards, which costs nothing where no such column is
  written.

- **What the other crates keep is told apart by project too**, each by
  a list of its own in one way of saying it (`journal::apart`:
  `studio::apart::TABLES`, `hosting::apart::TABLES`), so that what is a
  project's can be kept in its own journal's file and found there by
  what its table's line says, and whatever moves a project's rows knows
  no table. A table is the factory's own, whatever it names (people and
  how each is let in, the list of projects, what groups projects and
  channels, and a channel with who is in it, what was said there and
  what is attached there: a conversation is not a project's thing, and
  the project a channel says is where it is listed); or its rows say,
  in a column (a table and its records, what a scan last saw of a
  project's files, a service and a router: nothing there is nobody's);
  or they are a row's of another table kept beside it; or they are read
  a way of their own: a
  line of a record's history is its record's, and of a record that is
  gone, or is kept as a file or by a store, of the project whose table
  its id is one of, read against the tables there are and never by what
  the id begins with (`shop-2-5` is a card of `shop-2`, or the fifth of
  a table of `shop`'s called `2`). A test of each crate fails on a table
  with no line and reads every row against its line.

  Which journal keeps a project's such rows is asked, at every place
  one is read or written, of the factory's journal with the project in
  hand (`Journal::records_of`; `every_records` for what is asked across
  projects). It asks how much the project's journal keeps, a level
  (`journal::keeps`: nothing, what is run and said, its records as
  well, its turns as well; each asked as "at least"), and answers with
  the factory's own for a project whose journal was made keeping less;
  `studio` asks it for a
  project's tables, their records and each record's history, the last
  by the projects a record's id may be of, since a line of history is
  asked for by the record's id alone and outlives the record.

  Two things are held where a record is written, by the database
  (rules made with the tables, the last step of `studio`'s
  `collections`): the id of a record that is new begins with its
  project, and a record is never written again, or changed, as another
  project's. A record is written whole each time, so before this a
  second table whose records came to an id another project's card had
  took that card's place and said nothing; it is refused now. Adding
  the rules reads no row: on a
  file of 362 MB holding a million records and two million lines of
  history, read from the disk, the step took 141 ms, and a record
  written costs what it did (28 millionths of a second, with the rules
  and without). It is one way as each change of the tables is: a
  program from before refuses tables at a version it does not know.
- **What the agents keep is told apart the same way**
  (`agents::apart`, with its own `TABLES` and a test that holds every
  table the agents keep to it, and every row of each). An agent is the
  factory's (`slots`: its name is its own in the whole factory, it may
  be no project's, and whoever gives work looks at all of them), and so
  are the sign-ins the factory holds. A turn is its project's: it says
  (`turns.project`, the empty text where its agent is no project's),
  copied in the write that queues it from what the database keeps of
  its slot and never handed in. An agent may be given to another
  project afterwards, and what it did before is not that project's: the
  database refuses a turn another project, or another agent, once it
  is written. A turn kept before a turn said so says nothing and is
  reached through its slot as that now is; the column is added and no
  row written, so a journal from before is not read to take it (28 to
  40 ms on one of 200,000 turns and 837 MB read from the disk, where
  reading each of those rows once took six seconds).

- **Asking for one project's share reads nothing of another's.** What a
  project's conductor will ask as it starts (everything of the project
  that is alive, its runs under way, its handlers' places:
  `Store::alive_of`, `under_way_of`, `places_of`) is asked of what holds
  the project's rows together. What is alive and what is under way are
  kept by project in two small tables (`alive`, `under_way`), a row for
  an instance from when it is written until it is over and for a run
  until it ends, by rules of the database, as an index is kept: whatever
  writes the row. They are tables and not indexes because an index over
  tables that hold everything that ever ran cannot be made without
  reading all of them (measured below). The suite holds it by the
  database's own count of its work: each question is answered in as
  many of the database's steps whether another project has six
  instances or twelve hundred. The conductor's start asks for
  everything, as it did, of the same two tables; and they take the place
  of the two indexes of status, each of which was written again at every
  change of a status and only ever asked for what was alive or under
  way.

- **A handler's place is a place in a named journal**
  (`runs::places`). A journal is called for whose events it keeps, by
  whoever opens it: the factory's own is `factory`
  (`journal::FACTORY`), and a project's is to be `project:<id>`
  (`journal::of_project`), so no project's is called as the factory's
  is, whatever the project is called. It is never called for where its
  file is, which is the program's own to change: the name is the same
  after a restart and in a copy. A place is kept by the instance, the
  handler and the journal's name, and everything that reads or moves
  one says which journal (`Journal::name` of the journal the handler
  follows): a handler that follows two has a place in each, neither is
  moved by what is written of the other, and no number of one is held
  against a number of the other. A handler follows its own project's
  journal and the factory's, or every one (below).

  Measured with the journal's own count of its work (`Db::writes`,
  `Db::done`, and the length of its log with nothing brought back),
  2,048 events said one at a time to a handler with one step: an event
  that starts a run is seven pieces of work, each of which wrote, and
  nothing read, before and after; 39.9 to 40.1 pages before and 39.8
  to 40.0 after. One that is passed over is one write and one more in
  256, and 2.3125 pages, before and after. A test holds both
  (`costs.rs` in the conductor's suite). With no index of the events by
  instance or by run, the event that starts a run is the same seven,
  and 25.8 to 25.9 pages.
- **What the runner says of its own steps and runs is passed on, and
  kept only where a text could hear it** (`conductor::owed`). That a
  step started, finished, failed, is tried again, waits, was passed over
  or cancelled, and that a run started, was taken up, failed or
  finished, is said for every step there is: half of all a factory
  says, and every one of them a row of the events and of the index of
  their topics. Nothing reads it back: what a step and a run came to is
  their rows, which every page and command reads, and the client follows
  what is said as it is said. So each is said where it always was, in
  the write that keeps the row it is of, numbered there and in that
  order, and handed to whoever follows once the write is kept, but it is
  not written (`Writing::pass_on`; `Journal::pass_on` for one said by
  itself): to a follower it is what it was, and it is the place the
  journal is told to shut at as it was, so the plays and the simulation
  stop where they stopped. It is kept where some instance there is runs
  a text that could be owed it (a handler that hears its topic, a
  `wait` for it, or a topic written with a hole), read from the texts in
  force, and everything is kept while a conductor takes things up at a
  start; `run.failed` is kept wherever a fleet is up, since it hears it.
  What a restart read back of it is written on the rows instead: where a
  step's try began, and when (`steps.began`, `began_at`, written in the
  write that says it began, kept or not), which a step done again looks
  for what it had said from; and that a run began where that was only
  passed on (`runs.begun`, the one write of its beginning, as the
  event was), which tells a run taken up with nothing else written from
  one never begun. A row from before says neither and is read from the
  journal, which kept them. A journal that passes on a number has holes
  the gaps do not say, and writes so once (`numbered::HOLES`), so that
  the newest so many it keeps are counted from its rows. A follower that
  catches up from the log is handed what comes live with a number the
  log has not (`Follow`, by what it read there), where it used to take
  any number at or below where it had read for one it had. A test reads
  what was said, kept or passed on, through the order its factory
  writes down (`Order::read_every`, `Order::follow_every`), and so does
  a sandbox. Measured the same way as above, an event that starts a
  handler's run is the same seven pieces of work and 16.3 pages, where
  it was 25.8; and a card's work (*What one card costs* in
  [build-times.md](build-times.md)) is in that document.

  A journal an earlier program made is taken up as it is. The two
  tables are made and told what is alive and under way, the runs under
  way are told whose they are, and the places of handlers that are
  alive are brought over, all of them places in the one journal there
  was; what had ended is not touched, and no index is made over a table
  that has rows. Timed on a journal of 556 MB from before (4,000
  instances of which 308 were alive, 200,000 runs of which 154 were
  under way, 600,000 steps, 400,000 events): 18 to 31 ms with it in
  memory. Read from the disk, on the machine this was written on, whose
  disk is slow to find a page and slower when it is busy, it was 0.23
  to 0.31 s at a quiet time and 0.82 to 0.89 s at a busy one. Nearly
  all of that is the first reading of the rows of what is alive, which
  a start reads next whichever program it is: as it was, a start's own
  reading of them took 0.20 to 0.24 s and 0.83 to 0.88 s at the same
  times, and after this has read them 0.05 and 0.21 s. It grows with
  what is running and not with what the journal has kept. Made as
  indexes over the tables there were, the same took 16 s for the runs
  and 4 s for the instances read from that disk at the busy time, which
  is why they are not.

  One thing is left in a journal from before that has kept much: the
  two indexes of status, and the table places were kept in, which
  nothing reads any more. Taking an index away reads all of it (0.6 s
  from that disk for the 200,000 runs), so it is done as the journal is
  opened only where it has kept no more than 20,000 instances and
  20,000 runs (`runs::SMALL`), where it is a hundred pages or so. A
  journal that has kept more goes on writing them with their tables,
  about two pages more for a handler's event (42.2), until its projects
  are moved to journals of their own, which are made as one is made
  now. This too is one way: the tables are at a version an earlier
  program does not know, and it refuses them.

  A copy of a factory keeps whose each row was, and no place and
  nothing alive or under way, since nothing is running in one; written
  out again (`repack`), a journal is told apart as it was, rules and
  all.

- **A project that is added has a journal of its own**, beside the
  factory's: a file, with a thread that writes it, its own log brought
  back off that thread's turn and its own gathering of what waits
  (`journal::set`). It is opened when the daemon starts and closed with
  the factory's when it stops, each project's first and the factory's
  last, since a project's draws its numbers from the giver the
  factory's keeps. What is run for the project, and what is said of
  it, are kept there (below).

  **Which projects have one is written down, and is never worked
  out**: a row of `journals`, in the factory's own database, names a
  project, what its journal is called and the file it is in. The row is
  written when a project is added (`Journal::own`, asked by
  `Projects::create_with` before anything is said or kept of the
  project), and nothing takes one away. A project with no row has no
  journal of its own, and what is said and kept of it is in the
  factory's: that is every project of a factory an earlier program
  made, which goes on as one file until its owner asks for it to be
  moved apart, while a project added to such a factory now has its
  own. So everything that is built on this holds both ways, and is
  tested both ways. A project that is taken away leaves its journal
  where it is, and one of its name added later goes on in it.

  A journal's file says what it is called (`journal::NAME`, in its own
  small durable state, written as it is made), and one that says
  another name than its row is refused: the opener is not believed. A
  journal that is listed and whose file is gone is not made anew, and
  the factory is not started: what ran for the project would be gone
  with nobody told. The files are in a folder of the data folder
  (`journals`), each called for its project where the project's id can
  be a file's name; that is the program's own to change.

  What the journals of a factory share: one giver of numbers; where
  measurements are kept, which is the factory's (what is measured
  through a project's journal is read with everything else, and each
  database's work is told apart by `db`, the journal's name); what
  events are held to in a build made to be tested; and the place they
  shut at (`Journal::shut_after`, `down --at`, a simulation), which is
  one place in one of them, at which every one of their databases does
  nothing more. A journal whose thread is in the middle of a piece of
  work as another reaches the place finishes that piece, as a daemon
  killed an instant later would have.

  A copy of a factory takes each file as it was at one moment, the
  same for all of them (`std/snapshot`, `journal::moment`), so that the
  copy is a factory that could have been: no event in it is numbered
  above the mark that went with it, no run in it was set off by an
  event it lacks, no place it keeps is past the last event of the
  journal it is a place in, and it lists a journal exactly where it
  holds the file. The moment is made by standing every journal still.
  Each thread is asked for a piece of work, done apart, that only waits
  to be let go: what was asked of the thread before is done, committed
  and answered first, so it stands between two pieces of work. With
  all of them standing, a read is begun on each file the factory's
  lists just then, on a connection of its own; a read sees its file as
  it was when it began. Then every thread is let go, and each file is
  written out from its read, page by page (the database's own copying,
  which goes through the read that is open), the factory's first.

  Each project's thread is parked first, all of them asked at once,
  and the factory's own last, when they all stand. A
  project's thread waits on the factory's in the middle of its own
  work, for a run of numbers, and the factory's waits on none of
  theirs: so the factory's is free until every project's stands
  between two pieces of work, where it waits for nothing, and the two
  are never parked against each other. Nothing waits for ever. The
  threads are waited for five seconds, all of them together; one that
  has not come by then (it is in the middle of something long, or its
  journal has shut and is to do nothing more) is not waited for, those
  parked so far are let go, and the copy fails saying which journal it
  was. A thread that is parked goes on by itself after fifteen seconds
  where nobody lets it go, and a copy that then finds one gone on is
  refused, since it was not of one moment. Whoever parked them going
  away lets all of them go.

  What it costs is how long the journals stand still, and how long a
  log is held. A read that is open keeps its file's log from being
  brought back past where the read began, so each file's read is let
  go as that file is written, and nothing is taken out of any copy
  until all are written: a file's log is held for as long as copying
  it and the files before it takes, and grows by what is written to it
  meanwhile. Measured on a scratch factory of 1.5 GB (its own file 900
  MB and two projects' 300 MB each), each journal told 200 events a
  second throughout, on the machine this was written on while it was
  busy: every thread stood still for 0.9 ms (`snapshot.stood`); the
  factory's file was written, and its read let go, in 6.5 s (written
  out anew, as one file by itself is copied, it took 7.3 s); the first
  project's read was let go after 11.4 s and the second's after 14.6
  s, by when its log was 7,700 pages (15 MB) longer than as the copy
  began, which is what had been written to it. That is a quarter of
  the bound at which a writer brings its log back itself (32,000
  pages); a log that grows past the bound under a longer copy is cut
  back to it when it next begins again. And from the moment until its
  file is written a journal has three more files open (the database,
  its log, and what the two are read through): every journal's at once
  as the copy begins, which for a factory of some hundreds of projects
  is as many again as the daemon holds for them.

  `up --from` puts each where it goes before the folder is a factory,
  and refuses a snapshot that lists a journal it does not hold. Old
  events are taken out of each journal by itself, each keeping its
  newest however old. `repack` looks at each file, and finds a
  project's made as one is made now.

  What a node holds for it, as the system counts: three files open for
  each project's journal (the database, its log, and what the two are
  read through) and a thread, with a fourth file and a second thread
  from when its log has first grown by a thousand pages. A test holds
  the files and that they are given back, stopped or let go of
  (`given_back.rs`).

  This is one way, as each change of the tables is: the journal's
  tables are at a version a program from before this does not know,
  and it refuses them, from the first start of this program on a
  factory and whether or not a project has been added since.

- **A project whose journal keeps what is run for it has a conductor
  of its own** (`conductor::set`). Its instances, runs, steps and the
  places of its handlers are rows in the project's file
  (`runs::Store`, opened on the project's journal), each written with
  what is said of it in the one write, by that journal's thread; and
  the conductor that runs them has its own hand of what is alive, its
  own tasks, its own turn at making an instance (`Inner::making`, which
  was one lock for every project), its own waiters and its own handing
  out of events. So a project with a great deal running holds another
  up in the runner by nothing but the processors they share. The
  factory's own conductor runs what is nobody's, and every project
  whose journal does not keep what is run for it.

  **Which projects those are is written down**, as which have a
  journal is: `keeps` of a project's row in `journals` is a level
  (`journal::keeps`): 0 for a journal listed before anything was kept
  in one (nothing), 1 for what is run for the project and what is said
  of it, and above that what the other crates keep of the project
  (below), which is what this program makes a journal to keep. It is
  asked as "at least" (`Journal::rows_of`, `Journal::keepers`), so that
  a journal made at a level below is never read as keeping what it was
  not made to. The row has room, too, for where a project's rows were
  moved from and where the giver of numbers stood then (`moved_from`,
  `moved_at`): a project this program adds
  was made apart and says neither, and what moves a project of a
  factory from before apart says both in the write that decides the
  move (below). What is run for a project
  of a factory from before, and for a project whose journal was listed
  by a program that kept nothing in it, stays in the factory's file
  with the factory's conductor, whole, until its owner asks for it to
  be moved apart: nothing is moved by a database being opened, and a
  first start on a factory from before adds three columns to a table of
  a few rows. This is the one-way step for what is run: a program from
  before would find no rows of such a project in the factory's file
  and take it for one with nothing running, so the journal's tables
  are at a version it refuses.

- **A project of a factory from before is moved apart when its owner
  asks** (`mw apart`, `api::moved`, `journal::moved`), as `repack` is
  asked: of a factory that is stopped, by a program that holds its
  folder as a daemon would. Not by the daemon, which reads which
  projects have a conductor as it starts and cannot yet stop one and
  start another while the rest run. What is moved is what the runner
  keeps of the project, every table by its line (`runs::moved`, over
  `runs::apart::TABLES`: the move itself knows no table), finished runs
  among it, so that the factory's own rows hold nothing of the project
  afterwards; a run that a program from before left saying nothing is
  told whose it is as it goes, since the file it goes to is held to
  every row saying. What the database keeps by its own rules (what is
  alive, what is under way) is neither written nor taken out, and
  follows.

  What the conductor keeps beside those rows goes with them, in the
  same two writes, as more lines of the same list (`runs::moved::beside`,
  over `runs::beside::KEPT` and the comparisons' list:
  `journal::moved::Kept::beside`, a name of the small durable state with
  how a key under it says whose it is). The project's own conductor
  looks in its own journal for all of it, and a project moved without
  it was run as one nobody had held to anything: at its first start its
  trees were pinned afresh, to nothing, and read what ships now; its
  files were vouched for as they stood at that moment, whoever had
  written them since, in the name of whoever had applied the tree; and
  a `poll:` took its next reading for its first. So what is an
  instance's (what a tree
  reached, what was vouched for it, what a handler last read, what a
  way of a comparison has a thing read) goes where the instance's rows
  go, and what is a project's (the last text that checked) with the
  project: written into the project's file with the rows, and taken
  out of the factory's in the write that takes the rows out, as many
  of each as were written or none at all. A text a tree is held to is
  kept once in each journal, by its revision: it is written into the
  project's file for each of the project's trees that is held to it or
  can be put back on it (what is pinned for the instance names the
  revision, wherever in it), and is left in the factory's for whatever
  else is held to it there, where the factory's conductor puts it away
  as it starts once nothing is. What is the factory's (how texts are
  taken, the variables of the profile, the comparisons) and a
  journal's note of itself stay. Where a key is in the project's file
  already, the line says which stands (`Kept::yields`): what is kept
  for an instance is written over it, since the conductor that had the
  instance until now kept it, and the others are left as that file has
  them, a text being the same text and what the project's own
  conductor remembered under the project's name being of what is
  applied for it now. Nothing is left for a conductor to
  bring over, and no note is written for one: the note of a journal
  from before (`runs::beside::OWED`) is for what a program kept in the
  factory's journal for rows that were already in the project's file,
  which no move made.

  What the crates above the runner keep of the project goes too, in
  the same two writes, as more lines of the same list: its tables,
  their records, each record's history and what a scan last saw of its
  files (`studio::moved::tables`, over `studio::apart::KEPT_APART`),
  and what its agents were asked (`agents::moved::tables`). Each line
  says the level a journal keeps its table from, and the move is handed
  those the project's journal does not keep yet (`tables(keeps)` in
  `folders/src/moved.rs`): all of them for a project with no journal of
  its own; and for one whose journal keeps less than one made now (an
  earlier program made it, or moved the project when less was moved),
  the rest, into a file beside the one it has. The tables are made in
  the file by the steps a daemon makes them by there
  (`studio::apart::make`, `agents::apart::make`), and the row says the
  level a journal made now is made at in the write that takes the rows
  out, and not before: until then every reader is answered with the
  factory's file, and afterwards with the project's.

  A row goes as it is, every column: a record under its id, a table
  with where its store is reached and what declared it, a line of
  history under the number of the event it is of, which the event
  keeps wherever it is kept. A line is told apart through its record
  and the tables there are, so it is written after them and taken out
  before them (`Kept::depth`); one whose record nothing says the
  project of stays in the factory's file, and is read there. A turn is
  the project's by what it says, whoever its agent is of by now, and
  one kept before a turn said so by its agent, as the list of agents
  stands at the move. Two things such a turn says only by where it is
  are written out as it goes (`Kept::writes`): whose it is, since the
  list of agents it was read through is not in a project's file; and
  which turn of all it is, which for a turn from before is the row it
  was in the factory's file, the one file such turns were in. The
  turns are written in the order of those numbers (`Kept::order`),
  which is the order a file's rows are relied on to be in. Nothing
  else is kept for these that a move has to write. A record is
  numbered from the rows of its table. The count turns are numbered
  from, and how much each agent was given, are read from every file as
  a daemon starts. And where the keeper of histories has read to in
  the project's journal is what its place in the journal the project
  was moved from comes to there, by the rule for a place below: it may
  read a few events a second time, and a line is written once however
  often its event is read.

  A project is moved at rest. A run under way keeps numbers that are
  places in the journal it began in (where a step began, what it said
  before, a hook's place), and a turn of the project's that is
  waiting, running, or answered and not yet said to have ended is work
  in the middle of itself, perhaps with a program that goes on without
  the daemon, perhaps asked for in a channel with no run at all: with
  either, the project is left where it is and whoever asked is told
  which (`runs::moved::under_way`, `agents::moved::in_hand`, which
  tells a turn apart as the move does: one that says the project is in
  its way though its agent was given to another since). Instances that
  are alive and waiting to hear something are at rest, and are what is
  moved; each is taken up by the project's own conductor when the
  factory is started.

  Two files are not one write, so one write decides. The project's
  file is made as one is made now (its journal's tables, then the
  runner's, by what opens them on any journal, then those of the crates
  above) and every row of the
  project is written into it, counted, read back and held to the disk.
  Then one transaction of the factory's database writes the project's
  row of `journals` (the file, the level a journal made now is made
  at, that it was moved from the factory's journal, and where the
  giver of numbers stood: everything said of the project before is at
  or below that, and whatever its journal says is drawn above) and
  takes the rows out, as many of each table as were written or none at
  all; what was done is noted in the same write, for the daemon that
  next starts to say (`system.moved_apart`). Cut off before that write
  the factory is as it was, what is kept beside the project's rows
  with them in the factory's file, and a file lies in the folder that
  no row names; after it the project is moved, its rows and what is
  kept beside them in the file its row names and in no other, and the
  file it had before
  (where a program had listed one that kept nothing) may still lie
  there. A file of that folder that no row names is nobody's: the move
  asked again takes it away before it begins, and so does the daemon as
  it starts, holding the folder, before it opens anything
  (`moved::clear`, `system.move_cleared`). A project that had such a
  file is moved into another beside it (`shop.2.db`), which is that
  file whole with the rows added, and its row names the new one.

  What was said of the project is moved as its rows are, in the same
  two steps, where its journal had kept nothing said until then (the
  journal's own events are one more table the move is handed,
  `runs::moved::said`: an event is the project's that its data names,
  else that of the instance that said it). An event keeps its number
  where it goes and the giver is not touched. The write that takes the
  events out of the factory's journal keeps the highest number that
  journal ever gave (`numbered::keep_said`) and that its events have
  holes all through them, which the project's file is told as they are
  written into it (`numbered::HOLES`): the newest so many of such a
  journal are counted from its rows when old events are taken out,
  since a list of gaps cannot say them. The project's row says where
  the giver stood at the move, and from which journal. That is what a
  place in the factory's journal comes to in the project's, for
  whatever has none of its own there (`journal::joined`,
  `Journal::place_in`: the lesser of its place in the journal the
  project was moved from and where the giver stood, since everything
  the project said before is at or below that and was read there as
  far as the place; and a number alone that is above it stays what it
  is). It is the one place a place among all the journals becomes a
  place in one (a page's cursor, a `wait`, a comparison's count, the
  keeper of history, what is searched), and a handler, whose places
  are rows, asks the same as it first hears the journal
  (`Journal::joins`, in `Handler::join`). A place worked out so is kept
  by name from then on, since the place it was worked out from moves
  on. And a place that anything kept in the project's journal while
  nothing was said there (a handler hears its project's own journal
  from when it has one) is taken away by the move, in every file, so
  that it is worked out and not read from the beginning, which would
  hear everything moved a second time. A project whose journal kept
  what was said of it already has only its rows moved, with what is
  kept beside them: what the factory's conductor said for it meanwhile
  stays where it was said, and its row is not told it was moved. That
  is right for what is kept beside the rows too, which goes whichever
  way a project is moved: where the giver stood is what a place among
  events goes by, and nothing kept beside the rows is a place (what a
  tree reached, what was vouched for, what a handler last read and the
  last text that checked are each read by their key). The factory's
  file is not made smaller: `repack` does that.

  Measured on a journal of 850 MB (300 MB of events; 350,000 runs and a
  million steps of two projects and of nobody, half the runs saying
  nothing of whose they are), each reading from the disk and with other
  work going on: saying what would be moved took 4.4 s, and moving a
  project of 200,000 runs and 600,000 steps 15 s (9 s to write its file
  of 270 MB, 6 s to take the rows out), half that for one half as big;
  the factory then started in 0.2 s. A fleet of fourteen instances with
  a few dozen runs is moved in a third of a second.

  **The factory's own conductor is the front.** A node is handed it,
  and whatever is asked of it is asked of the conductor that keeps the
  thing: an apply by the project it is for (`Engine::of`, which makes
  and starts a conductor for a journal made since the factory's was
  started: a project added while it ran); an instance or a run by
  whichever has it, its own rows first, which costs a conductor
  nothing for what is its own, and then each other's hand and rows
  (`Engine::elsewhere`, remembered); a list from all of them, put
  together as one conductor's rows would give it (`conductor::routed`).
  Any conductor answers so, so a step's own (`ActionCx::engine`) can be
  asked of anything, and nothing outside the runner knows there is
  more than one. A start takes up the factory's own and then each
  project's in the order of their names; a halt lets go of all of them,
  which hold one another until then.

  **A conductor is asked once it has taken up what it keeps.** Each
  says how far it has come (`standing` in `conductor::set`): made,
  taking up what it keeps, up, or stopped by itself. What another
  conductor, or anybody outside, asks of one that is taking up what it
  keeps waits until it has: an apply for its project, an instance or a
  run found in its rows, a list that reads them, a stop. So a step
  that is taken up as the factory starts, and stops an instance of a
  project whose conductor has not started yet, has it stopped in hand,
  with what its text says is done as it stops; asked of the rows, as
  it was before, the instance was written as stopped and said and did
  nothing. What needs no hand does not wait: whose an event is, and
  what is alive anywhere, are read from rows by their keys. Whether
  something is still there (`Engine::is_there`, which a pool asks) is
  answered from every hand, and from the rows of a conductor that is
  not up.

  The conductors start one after another, and a start that waited on
  one that starts after it would wait on what cannot begin until it
  is over. So what a start does itself never waits: it is answered
  from what is written, which is all a start asks of another. The one
  thing a start may ask of a later conductor that needs that one's
  hand (a stop, asked by something that was being stopped when the
  factory went down and is finished before anything is taken up) is
  asked and not waited for, and is done by the conductor that keeps
  the instance once it is up. A stop one conductor asks of another is
  always a task of the other's, so that it is cut short where the
  keeper is stopped and not where whoever asked is.

  **One conductor can be stopped and started while the others run**
  (`Engine::halt_one`, `halted_one`, `start_one`). No route and no
  command asks for it: it is what moving a project apart, and
  starting a project again by itself, will stand on. It is halted as
  the conductors of a factory are halted together: its tasks end
  where they wait, what it had asked of its database is written, and
  its journal stays open, since what the rest of the factory says of
  the project is still kept there and whoever follows it goes on
  following. What is asked of it meanwhile waits, and whoever was
  waiting for a run of its to end goes on waiting. Started, a new
  conductor on the same journal takes up what the rows say, as any
  start does, and is the one that answers from then: an apply, a list,
  a stop another conductor's step had asked for, the run somebody
  waited for. What follows every journal (the stream, a `wait` or a
  handler that hears the whole factory, a comparison's count) is no
  part of the conductor that stopped and goes on throughout.

  Nothing but a test stops one conductor alone yet, and whoever first
  does has two things to know. An apply for the project that somebody
  outside is in the middle of as its conductor is halted goes on
  against the halted one: what it wrote is there, its run is begun by
  nobody, and it is taken up when a conductor is next started on that
  journal; so one that is to be stopped alone is stopped when nothing
  is being asked of it, or started again after. And the factory's own
  conductor is not stopped alone: halting it halts them all.

  Held by plays of three conductors (`alone.rs` in the runner's
  suite): one of them stopped alone just after each thing its journal
  kept, and at each place of a stop another conductor asked it for,
  and started again, with only what it had under way let be asked
  once more and the others held to exactly what they come to with
  nothing in their way; the factory stopped where a step had begun to
  stop another conductor's instance; and the factory stopped where
  something that was being stopped had still to stop an instance of a
  conductor that starts after its own, where a start that waits is a
  failure the rig names and not a test that never ends. A comparison
  counting across one conductor's stop is in the suite of
  `comparisons`.

  **What is said of such a project is kept in its journal too**, so
  that everything said of one project is in one journal, in the order
  it was said: what its conductor says of what it runs, what an
  instance says (`emit`), and what any other part of the factory says
  that names the project (`Journal::emit`, asked of the factory's own,
  keeps an event that names such a project in that project's journal).
  A card's history, a stage that hears a move and then a verdict, and
  a page that follows what happens to the work are then told in the
  order things were said, as with one journal.

  One kind of thing is said of a project with no `project` in what it
  carries, by something that is not its conductor: what a service says
  of itself (`service.*`, and whatever its program prints for the
  factory to say), what a router does (`router.*`), and a service's
  lines of output, which are only passed on. Each is said by the
  instance that owns it, and whose that is hosting keeps with the
  service. So hosting says them in the journal that keeps what is said
  of that project (`Journal::said_of`, which asks the one question
  `emit` asks and decides nothing itself): a service that stopped and
  the instance that owned it being over are then in one journal, in
  the order they were said, for a handler that hears both and for
  whoever reads what happened (the hosting suite's `services.rs`).
  An event a service prints that itself names a project is kept where
  that project's are, as any event that names one is.
  The rows of a service and of a router stay in the factory's file:
  they are a few, and a start finds them by reading all there are.
  That the row is in one file and what is said of it in another is no
  gap that was not there. A service's row and what is said of it were
  always two writes, the row first; and nothing looks, at a start, for
  a service's row that was written and never said: a start adopts what
  is alive, and says of a row written as up whose process is gone that
  it exited while the daemon was down. A daemon stopped between the
  row and the saying left the row with nobody told when there was one
  file, and still does.

  What is only passed on and not kept (`Journal::emit_ephemeral`: what
  an agent is writing, a line a step printed) goes out in the stream
  of the journal that would keep it, by the same question. Whoever
  follows every journal is handed each one's in its order and no order
  between two, so what an agent wrote, passed on in the factory's
  stream, could come after the turn it was written in had been said to
  be over in the project's (the journal suite's `every.rs`).

  **What is promised of the order of events, and it is all that is.**
  What one journal keeps is heard, followed and read in the order it
  was said. So everything said of one project that its journal keeps
  is in one order, and so is everything that is the factory's own.
  Between an event of a project's journal and one of the factory's,
  and between two projects', nothing is promised: a handler or a
  `wait` that hears both is handed each journal's in its order and
  either first, and what is read to be shown is put in the order of
  when each was said, to the millisecond. What is still kept in the
  factory's journal though it is about something a project has: an
  event that does not name the project and is said by no instance of
  it. Of what the program says today that is what is said of a
  sign-in an agent's turn holds (`agent.signin.*`: a sign-in is the
  factory's, and the event names the agent and the turn), and an event
  said from outside that names no project. Nothing that ships decides
  anything by the order of one of those and an event of a project.
  An instance of one project that says an event naming another has
  the event kept where that other's is kept, and a step done again
  after a stop has to find it there, where none of the places its own
  journal keeps (where the step began, where an element of a list did,
  where what is done as it starts was all done) mean anything. So
  where that journal had got to is kept before anything is said in it
  (`keep_place` in `conductor::intrinsic`): in the step's record for
  the step's own action, with the element where it is done for each of
  a list; and, for an entry of what is done around a step, as a run
  ends or as an instance stops, with how far those entries have come
  (`hooks::Around`), until the entry is done. What that step said of
  the topic there after the place is what it said, since one step says
  one thing after another. It costs one write more for each such
  event, and nothing where an event is kept in the run's own journal.
  A rule that watches a step keeps the place too, with which time in
  the step's try it fired, and in whichever journal it says something,
  its run's own included: done again after a stop, a step takes its
  whole time again and its rules fire again from its new beginning, so
  a rule that says something said it twice wherever it was kept, and
  now finds what it said the same time before and says nothing. What it
  does besides saying (a mark, a post) is done again, as it always was.
  Of what ships only an example has rules that say anything
  (`examples/blueprints/kv-cache.bp`), and those name no project.
  Played stopped just after each such event is kept (`elsewhere.rs` in
  the runner's suite, with a rule's; the plays of `watched` hold a
  rule's event in its own journal to being said once).

  **What all the conductors share** is held once: the registry, the
  library, the logs, what is owed a step's being passed over, the
  hooks, the remote, where projects are, the profile, how texts
  are taken, what answers for comparisons, and the turn at changing
  what is vouched for.

  **What a conductor keeps beside its rows is kept where its rows
  are.** Not everything it has to find again is a row: which text of
  each name that ships a tree reached and the texts themselves, what
  was vouched for an instance, what a `poll:` last read, the last text
  of a blueprint that checked, and what a way of a comparison has a
  thing read are small values in a journal's small durable state. Each
  name says whose what is kept under it is, in one list with the
  reason (`runs::beside::KEPT`, and `comparisons::copy::KEPT` for the
  comparisons' own): an instance's, a project's, the conductor's own
  with what is there, or the factory's. A conductor keeps the first
  three in its own journal (`Inner.own`), so a project's are in the
  project's file: copied, taken away and moved with the project (each
  name is a line of the move's list by what it says here:
  `runs::moved::beside`), and no work for the thread every project
  shares.
  What is one thing for the factory stays in the factory's journal,
  whichever conductor asks: how texts are taken, the variables of the
  profile, the comparisons, which way each thing went and where
  they count from. A text of a way of a comparison is kept with the
  comparison, so a text is looked for in a conductor's own journal and
  then the factory's (`Engine::text_of`), and a version of a blueprint
  among what every conductor's trees are held to (`Engine::text_at`).
  What is asked of any conductor about an instance and answered from
  what is kept with it is asked of the one that keeps it (`pinned_for`,
  `text_within`, `vouch_saved` for a project; `Engine::kept_beside`
  for what keeps something of an instance's from outside the runner).
  Each conductor tidies its own as it starts, over what it has alive.
  Tests read every name every file keeps after a tree has run and hold
  each to its line: it is listed, it is in a file its line allows, and
  what is an instance's is where that instance's rows are (the
  runner's suite's `beside.rs`, the API suite's `apart.rs`).

  A program before this kept all of it in the factory's journal, for
  every conductor. A journal that has something alive when this
  program first opens it notes that (a step of the runner's tables,
  which reads one small row and writes one: `runs::beside::OWED`), and
  the conductor that starts on it brings over what is its own, once,
  before anything is taken up or tidied (`conductor::beside`): each
  thing is written in its own journal and then taken out of the
  factory's, so a conductor stopped between the two finds it in both
  and brings the same over again. The factory's own conductor starts
  first, and puts away of what is in its journal only what nothing
  alive anywhere is kept for, so that what a project's conductor has
  still to bring is there when it comes to it. A journal made now has
  nothing alive and notes nothing. Timed on a journal from before of
  555 MB (200,000 runs, 600,000 steps, 308 instances alive) read from
  the disk with nothing of it in memory, the step took 43 to 270 ms,
  nearly all of it the commit: it reads one row of what is alive and
  writes one. That step is one way, as each is,
  and has to be: a program from before, finding nothing vouched for
  where it looks, would take a project's files as they stand for what
  was vouched for.
  Whose an event is, when it is heard in the factory's journal and
  names no project, is its instance's project, looked for in every
  conductor's hand and rows (`Engine::whose`); and whether something
  that holds a place in a pool is still there is asked of every hand
  (`Engine::is_there`).

  A copy of a factory ends what was running in each project's file as
  it does in the factory's (`leave_out_beside`), and counts it with
  the rest.

- **A project's tables, their records and what was said of each are
  kept in its journal's file too**, where its journal was made to keep
  them (`journal::keeps::RECORDS`, which a journal made now is): the
  tables a project's blueprints declare (`collections`), their records
  (a board's cards among them), each record's history, and what a scan
  last saw of the project's files (`studio::apart::KEPT_APART`, which
  says for each such table how much a journal has to keep for it). So
  a card's row, what is said of it and the line of its history are one
  file's, written by that project's thread, and a card moved and the
  saying of it are one write there (`studio::whole`): for a card of a
  project made now that is moved, with the line of its history, the
  factory's thread writes nothing, which the studio's suite counts
  (`apart.rs`). Counted in the API suite's rig, five cards one after
  another, each worked by one agent and reviewed by another: a card
  asked the factory's thread for 275 pieces of database work (37 of
  them writes) and its project's for 425 while its records were the
  factory's, and asks the factory's for 157 (20 writes) and its
  project's for 526 with them in its own file.

  The tables are made in a journal's file as the journal is made, and
  never by a file being opened: the crate that keeps such tables says
  so of the factory's journal as it is opened (`Journal::makes`: the
  level, and the steps the tables are made by, which are the ones the
  factory's own are made by), a journal made from then on is made with
  them before its row says what it keeps, and one there already whose
  row says it keeps as much is brought up to what the program knows of
  them, as the factory's own tables are. A journal whose row says less
  is given nothing, whatever program opens it.

  Whatever reads or writes such a row asks which journal keeps it with
  the project in hand (`Journal::records_of`), and that is the whole of
  the change at each place: one record by its id asks the file of the
  project its id says, and a second where the id may be read two ways
  (`shop-2-5`); a record's history, asked for by an id alone, the file
  of each project the id may be of. What is asked across projects is
  asked of each file that keeps such rows (`every_records`): the tables
  there are, every board for the index that is searched, and what a
  copy takes. A board is held against what was last said of its cards
  (`Board::settle`) in the file that keeps the board.

  The keeper of histories follows every journal and writes each line
  where its record is kept, and where it has read to in a journal is
  kept in that journal's own small durable state, in the write that
  keeps a line of an event of that journal: so a line of a project's
  card asks nothing of the factory's thread. The factory's own keeps
  the place in every journal as it did, written with each line it
  keeps itself and now and then besides, and a start goes on in each
  journal from the further of the two; what a program from before kept
  there is read as what it meant.

  A copy of a factory takes these with each project's file: what the
  studio leaves out of the factory's own it leaves out of each (a
  store service's address, what a scan saw), a table kept as files or
  by a store is said of whichever file declares it, the keeper's place
  goes with the lines it wrote, any other table found in a project's
  file is emptied and said, and what a scan would find of a project's
  files where the copy is put is written into the file the new factory
  will read it from.

  **A project from before stays where it is until its owner has it
  moved apart.** Its tables, records and
  history are in the factory's file and are read and written there: a
  row with what is said of it as one write, as they were, and written
  and then said only for a project whose journal keeps what is said of
  it and not its records (one a program between made so), with the
  look at a start behind it. A first start of this program on a factory from before does
  nothing to it: no step is added to the factory's tables, no table is
  made, no row is read to find out whose it is. On a file of 535 MB
  holding a million records, two million lines of history and what a
  scan saw of 20,000 files, of two projects from before, the studio
  was opened in 1.9 to 2.9 ms read from the disk and 2.6 to 2.9 ms
  with it in memory: fourteen pieces of database work, seven of which
  write (the version each set of tables keeps as it is opened), as
  before.
  A project of such a factory that is taken away and added again is
  given a journal that keeps what is run for it and no more, while the
  factory's file holds a table of it (a table is never taken away) or
  a turn of an agent of it: its cards are where they were, and go on
  being its. Each crate that keeps such rows says how the factory's
  file is asked for them as it says what a journal is made with
  (`Journal::makes`), and a journal is made at the level of what is run
  where any of them answers yes (`Journal::own`). Its owner has it
  moved apart (`mw apart`, above), which takes its tables and its
  agents' turns into its file with the rest; and a project whose
  journal keeps what is run and no more is taken the rest of the way
  by the same asking.

  This is the one-way step for a project's records: a program from
  before would look for a project's board in the factory's file, find
  none and make another. It does not get so far: the factory's own
  tables are at versions it refuses (the step that holds a record to
  its project, and the one that has a turn say whose it is).

- **What a project's agents were asked is kept in its journal's file
  too** (`journal::keeps::TURNS`, which a journal made now is). A turn
  is its project's, as its row says, and is kept in the file of that
  project's journal with what is said of it: queued, begun and ended
  each as one write, by the project's thread
  (`crates/agents/src/filed.rs`, `together.rs`). Who the agents are
  (`slots`) and the sign-ins the factory holds stay the factory's, and
  so does the turn of an agent that is nobody's, of a project from
  before, and of one whose journal was made keeping less. The tables
  are made in a project's file as its journal is made, by the agents'
  own steps (`agents::apart::makes`); the list of agents and the
  sign-ins are made there with them and hold nothing.

  So the turns of a factory are in several files, and each place that
  reads or writes one asks which. A turn in hand is in the file of the
  project its row says. A turn asked for by its id is looked for where
  this daemon queued it or found it in hand, and otherwise in each
  file: an id is a name. An agent's turns are in the files known to
  hold one of its, read from each file's index as agents are opened
  and noted as a turn is queued: the one its project's journal keeps,
  and whichever kept them while it was another project's or nobody's.
  A conversation's turns may be anybody's, and are asked of every file
  that keeps turns.

  **Which turn of all it is** puts turns of different files in order
  (`turns.nth`): where a row comes in its file says nothing of a row
  in another, and neither does when it was written. Every turn is
  given a number as it is queued, from one count for the factory, in
  the write that queues it and on the thread that writes its file, so
  that within a file the numbers go as the rows do and a file's newest
  row has its highest number. The count only goes up, and is kept
  nowhere but in the turns: a start reads the number of each agent's
  newest turn in each file, which the index finds, and goes on above
  them all, and that is what a factory started from a copy does too,
  so no number a turn has is given again. A number given to a turn
  that was never kept (its write failed, or the daemon went) may be:
  no turn has it. A turn from before says none and is ordered by where
  it came in the factory's file, the one file such turns are in, below
  every number given since. Nothing counts turns by it. It is not the
  journal's own giver that is asked: an event's number is drawn a run
  at a time by each journal, so between two journals the later event
  may have the lesser, and order across files is all this number is
  for.

  An agent may be given to another project, and then has turns in two
  files, each still the project's it was asked for. Who a conversation
  is held with (`Agents::holding`) and who the conversations beside it
  are held with (`beside`) are each file's newest turn of the
  conversation, the highest of them by number. Whoever chooses between
  agents is told what each has in hand from every file that holds a
  turn of its, added up, how many it was given likewise, and its last
  as the highest number among them; only those who could take the work
  are asked about. An agent has one turn for a message however often
  the message is heard: the database holds that among the turns of one
  file, and before one is queued every file that holds a turn of the
  agent is asked for it; messages are heard by one listener, one at a
  time, so nothing queues a second in between.

  A copy of a factory ends what was waiting or in hand in each file
  and counts it with the rest. In the API suite's rig a card asks the
  factory's thread for 140 pieces of database work (13 of them writes)
  with its turns in its own file, where it asked for 157 (20) with
  only its records there and 275 (37) before either. What is left on
  the factory's thread is who the agents are, what the runner keeps
  beside its rows by instance, who somebody is, and what is said in
  the project's channel, which is the factory's.

  A project from before keeps its turns in the factory's file, read
  and written there, until it is moved apart, which takes them into
  its file in the order they were asked (above). A turn of a
  project's may be in the factory's file all the same while the
  project's journal keeps turns: one kept before a turn said whose it
  is, whose agent was given to the project since; and one that says a
  project of a factory from before which was taken away and added
  again when no agent was its any more, since what asks whether the
  factory's file holds a turn of a project asks the agents that are
  the project's now (`agents::apart::held_here`: nothing says whose a
  turn is but the turn, no index goes by it, and every turn is not
  read as a project is added). Such a turn is read where it is, by its
  agent, by its id and for its conversation, as the turns of an agent
  given to another project are, until its owner has the project moved
  apart: the move is handed the turns whatever the project's journal
  keeps (`Kept::strays`), and writes those it finds among the ones the
  project's file holds, every one of them again in the order of their
  numbers, since a file's newest row is taken for its highest number.
  A first start of this program on a factory from
  before adds one column to `turns` and writes no row: 19 to 59 ms on
  a file of 1.6 GB holding 200,000 turns, read from the disk. What is
  read as agents are opened is the index of turns by agent, once, and
  the turns in hand and the newest of each agent through it: 2.2 to
  2.9 s on that file read from the disk and 17 ms with it in memory.
  It was some twenty seconds from the disk before, since the turns in
  hand were asked for by their status alone, which no index begins
  with, and found by reading every turn. This is the one-way step for
  turns, as each change of the tables is.

  A project of a factory from before that is taken away and added
  again is given a journal that keeps what is run, and may still have
  something running that the factory's conductor keeps. Run by two,
  what is applied for it again would be started a second time beside
  what is there: so it stays with the factory's until a start finds
  nothing of it alive there (`Engine::gives_over`), and what ran for
  it before is still read from the factory's rows.

  Measured on 2026-11-18, on a machine doing a great deal else, each
  way twice: a board of 120 cards worked by eight at once in one
  project, and in another, with one worker, a card made ready and
  waited for until it is done, over and over while the first is busy.
  With one file the board was done in 19.8 and 20.7 s and the other
  project's card took 499 and 525 ms at the middle (1.2 s at most);
  with a file and a conductor for each, 15.5 and 16.2 s, and 451 and
  459 ms (0.9 to 1.0 s at most). With nothing else going on the card
  took 317 ms with one file and 356 to 361 ms with its own: two
  threads are asked where one was. What one card costs is what it
  cost: 242 events, all of them in its project's journal and none in
  the factory's; 710 pieces of database work with one file, and 753
  with its own, 414 on the project's thread and 339 still on the
  factory's (its card and records, its agents' turns, what is kept
  beside the rows); 4.9 MB written either way. So about half of what a
  card asked of a database was off the factory's thread with that, and
  its records and its agents' turns have followed since (below). What it does to a journal
  from before as it is first opened: three columns added to
  `journals`, 22 to 30 ms on a journal of 300 MB read from the disk.

- **The tests go by the order things were written in.** A factory
  that is tested writes down, as each event is written by the thread
  of the journal that keeps it, where it came among everything all its
  journals wrote (`journal::Order`, in memory, for a program that is
  run to be tested; one that is run to be used pays for a look at an
  empty cell). The suites' rigs hand every event over with that as its
  `seq`, so that a test which does something and waits for what comes
  of it is not told "after" by two numbers of two journals. And the
  suites are played both ways by one word (`PROJECT_JOURNALS`: `one`
  keeps every project in the factory's own journal, `own` has each
  play of the runner's suite applied for a project with a journal of
  its own). The gates play them told nothing; told a word they are
  played by `cargo xtask ways`, which is asked for and is no gate,
  each way being a run of every test again. The
  plays' harness holds every database between turns, and a rig that
  stops a conductor closes and opens every one.

**What is promised of an event's number**, and it is all that is: no
other event in the factory has it, and of two events of one project the
later has the greater. Numbers have gaps. The numbers of two events of
different projects say nothing of which came first. One giver of
numbers for the whole factory is what keeps the first true when there
are many journals, each drawing a run of numbers at a time, which is
where the gaps come from; within a journal they are given in the order
things are written. Whatever keeps a place (a handler, a comparison's
count, a page) keeps one for each journal it follows, and none keeps a
number and waits for the next.

**What is not promised, so that it can change.** An id is a name:
nothing is read out of how a run's, an instance's, a channel's or a
turn's is written, a record's excepted, which says where the record is.
And how the data folder is laid out is the program's own: which files
there are, what they are called and what is in which. A factory is
copied by the factory (`std/snapshot`, `up --from`), which knows what
there is; a copy of the folder made while the daemon is stopped is
whole, and nothing else reads the folder.

The rest of the work, in the order it is to be done, each part landed
with the factory running on it:

1. What copies, prunes and repacks a factory taking each file. What
   a large journal from before still carries that nothing reads (two
   indexes of status) is not made in a project's file, and stays in the
   factory's until it is repacked.

## Stopped anywhere, and started again

That the daemon can be stopped anywhere and goes on from there is held by
playing it (`crates/api/tests/suite/simulation.rs`). A scenario is what a
person does and waits for, in order: a card worked and passed, one sent
back and worked again, one reviewed by an agent and merged, a fleet
brought up and a card worked by it, a service rolled out through a slot.
It is played once with nothing in its way, and
what it came to is written down: where each card is, what is on each
branch and which worktrees are left, what the channel was told, which
instances are alive, how many runs are under way. Every event the daemon
kept in that play is a place it could have been stopped after. The
scenario is then played again for each place asked for, in a factory of
its own: stopped there, started on the same data, and gone on with to the
end. What that came to must be the same, but that an agent may have been
asked up to two more times; whoever works a card is held to being asked
exactly as often, since a round that is begun again says which round it
is and is given the turn it asked for before (`asking` of `agent.task`). Where the application means something else (a
card whose review was interrupted is left in review, and the channel says
nobody decided) the scenario says so itself (`Scenario::meant`), so that
the choice is written down and a change to it is seen.

- **A stop is at a place exactly.** The journal is told the place
  (`Journal::shut_after`): the event there is the last it keeps, and from
  then its database does nothing more of what it is asked (`Db::hold`),
  for the journal or for anybody else who writes there. Whoever asks
  waits and is never answered. So what the daemon comes back to is the
  database as it was when that event was kept, which is what a daemon
  killed at that instant leaves; a place is the same each time a scenario
  is played, and a fault found there is found again. What this cannot
  find is a fault between two writes that have no event between them (a
  record changed, and the daemon gone before the event that says so):
  there is no place there to stop at.
- **A place is what was said and by what**: the topic, the blueprint of
  the instance, which of its runs, the step, the agent, and how many
  times that has been said so far. Not the ids, which are new each play.
  One kind of place is not something the daemon said: `program.began`, a
  stand-in's program saying it is there. An agent's turn is said to begin
  a moment before its program is started, and a daemon stopped exactly
  at that event never started it; stopped at `program.began` it goes
  while the program waits. What becomes of that program while there is
  no daemon is the scenario's to say (`Scenario::away`): a program that
  outlives the daemon is played both still at work when the daemon is
  back, and ended while it was away.
- **A play begins in a factory at rest, and is over in one.** A
  scenario's factory is set up by asking and being answered, and what
  was asked for goes on a little after its answer: a fleet whose apply
  has answered, or whose changed row it has taken, has stages still
  looking at their rows and their columns. Counted from the answer, what
  was still going on was a place in one play and had gone by before the
  next was listening, and a play stopped there came to its end without
  ever being stopped: of some 1,700 places, six to eleven were "never
  reached" in each of six whole runs kept on 2026-11-17, 52 over 24
  places, all but six of them in what a fleet does as it comes up and
  none in a round, a review or a merge. So a factory is waited for
  before its play begins (`rested` in `simulation.rs`): nothing said for
  four tenths of a second, nothing asked of the journal's database in
  that while (`Db::done`: whatever is on its way to saying something
  reads or writes first, and a handler that has heard something and not
  yet come to it is as silent as one with nothing to do), and nothing
  under way (`observed::at_work`). A play with nothing in its way is
  over at the same, and no longer at silence alone, which a slow commit
  also is; and so is one that has not come to where it was to be
  stopped, before it is said never to have reached it. What a fleet
  does as it comes up is tried on purpose instead: the scenario of a
  fleet brought up begins before there is one.
  What is not said in every play is a place where the scenario goes two
  ways: two things in what is played may come in either order, and one
  of the orders has no such place. A play that was to be stopped there
  comes to its end unstopped, having tried nothing. It writes down what
  the daemon said, as a play that went wrong does, and the place is
  played again, three times at most (`found::PLAYS`): the usual order
  mostly comes round the next time, the play is stopped where it was to
  be, and the place is held to what that play came to, said with which
  play it was. A place no play of the three came to is never reached,
  and that is a failure of its own, said in its own words with every
  such place (`Found::wrong`), however few they are. Until 2026-11-18 it
  was said beside the places that held and failed a scenario only where
  a quarter of what it tried was missed, so that one place in 1,926 at
  which nothing had been tried passed for held. The one thing not
  failed is a place its scenario says goes two ways by design, with the
  sentence why (`Scenario::either`, as a scenario says what is meant at
  a place and what is not held yet): it is said with the sentence and
  how often it was played. What scenarios say so is in one file
  (`either.rs` in the API suite), each as narrowly as what was seen
  allows:
  a stage of a fleet that is coming up looks at its column when it
  hears that it is active itself (`first`) or that the fleet has been
  arranged (`staffed`), whichever it hears first, and hears that its
  pool is listening or was not there yet, with one look (`std/due`) for
  each of those it heard;
  a fleet brought up over a card made ready finds the card, asks for
  its turn and hands it on while its stages and their pools are still
  coming up side by side, and each of those has a run for what it heard
  and none for what was said before it listened;
  of two cards where one is reviewed at a time the second is handed
  to review while the first is being reviewed, and waits, or after, and
  does not;
  and a proposal that ends itself as its card leaves the column says
  that the steps it never came to are cancelled for as many of them as
  its run had come to before the run was cancelled itself.
  An order that was only by chance is put right in the scenario
  instead. Two cards where one is worked at a time were made ready and
  whoever worked the first let go at once: with the machine busy the
  first was worked, and its place given back, before the second had
  asked for one, so that nothing waited in the scenario of something
  waiting, and half its places were one play's and not the next's. Its
  act now ends when the second card has asked for its place
  (`Play::said`). Counted on 2026-11-18 over every scenario's first
  play, each a process of its own, twelve at once on four processors
  and on two: ten plays of each, and nothing was said more often in one
  than in another but the last two of those three (one play of ten for
  the second review, three of ten for the fleet over a card). The first
  is from the plays kept before, where it was what every place never
  reached came to.
- **The gate plays the places that hold**: where an agent has work in
  hand (as its turn begins, and once its program is there) for a card
  worked and passed, one sent back, one reviewed and merged, one whose
  review its stage says to leave, and one whose worker's program outlives
  the daemon each of the two ways; the first time each program is there
  with two cards where one is worked at a time; as each kind of thing a
  fleet is made of is first there while one is brought up (the fleet,
  what brings it to what its tables say, a role, a pool, a stage:
  written down and not yet begun) and as each row its two tables start
  with is added, with a card made ready and worked once it is up, and
  again over a card that was made ready before the fleet; and after
  each kind of step of a rollout. And two tests of the harness itself: it finds a
  round that leaves its work at a restart whatever its stage says, and a
  review that keeps nothing leaving its card in review with nothing said.
  A third holds its rule for calling a fault: what a play has come to
  is read before it is asked whether anything is under way, the daemon
  may go on between the two, and a play is not called a fault on such a
  look while something the daemon has said is still unheard (`looked.rs`,
  a run that is nudged as it is looked at). A fourth holds the other
  side of it: a play at rest where it was not meant to be is a fault at
  once, saying what differed, though a journal's last number stays above
  what the play has read (`differed.rs`, an event passed on and not kept
  as the play is looked at), since what is waited for is what is there
  to be heard and whether anything was written, by the order the rig
  writes down, and never a number that may not come.
  `SIMULATE=kinds` plays one stop inside every kind of run and wait,
  `SIMULATE=everything` every place there is, `SIMULATE=places` lists
  them, and `SIMULATE_AT='<words>'` plays the places whose description
  has those words; `SIMULATE_SHARE=2/4` beside either of the first two
  plays the second of four shares of each scenario's places, so that
  every place can be played as several processes side by side and a
  share that was cut off played again by itself; each says how every
  place went, and writes everything
  the daemon said in a play that went wrong, or that never reached the
  place it was to be stopped at, to `target/tmp/simulation/`.
- **A step that says something is safe to be cut.** A message is written
  down and then announced, and a step that was under way when the daemon
  stopped is done again from its start. What a step says in a channel
  keeps which run, step and element said it (`said_by`), so the step done
  again looks for its own message before it says another
  (`Channels::said_already`): one that is there is not said twice, and
  one that was written and never announced is announced now. That is why
  `channel.post` says it is the same done twice, where a step that is not
  fails as one that cannot be repeated. An action that leaves something
  behind each time it is done asks the same of itself: `ActionCx::again`
  says the step is being done again, and what it left is found by what
  the action kept on it. A row added, a card added and something said
  on a record are found so (`record.add`, `board.add`,
  `record.comment`, `crates/studio/src/left.rs`): what is said of each
  in the write that keeps it names its run and its step, and says which
  of the things done under the step's name it is (`doing`), and the step
  done again reads what was said on that topic in the journal that keeps
  what is said of the project, from a little before its try began (the
  time its row says, `ActionCx::began_at`, made a place by
  `Journal::place_before`), by its run. A table kept outside the
  journal's database is written and then said, so where nothing is
  found there the step is refused as before: a row may be there that
  nothing said.
- **An event is kept in the journal that keeps the row it is said
  of.** A write says its events through `Writing::say`
  (`crates/journal/src/together.rs`), which asks where each is kept:
  in the journal being written, where that journal keeps what is said
  of the project the event names, and then the row and the event are
  one write; or in another, and then the event is said there once the
  write is kept. The second is a project whose journal keeps what is
  run for it and what is said of it and not yet the row (its records
  are kept from `keeps::RECORDS`, its turns from `keeps::TURNS`): two
  files cannot be one write, so that row is written and then said, as
  every row was before, and what looks for a row written and never
  said is still behind it. What is nobody's, a project from before,
  and a project whose journal keeps the row are one write. A channel
  and what is said in it are the factory's for good, so `channel.*` is
  kept in the factory's journal whatever project it names
  (`say_here`, `Journal::emit_here`) and heard by project as it was;
  which topics those are is one list (`journal::HERE`), and a project
  that is moved to a journal of its own leaves what was said on them
  in the factory's.
- **A record's row and what is said of it are one write**
  (`studio::whole`, over `Journal::write`), where its table is kept in
  the journal's database, which is where a board's cards usually are: a
  card added, changed, moved or taken away, and a record of any table,
  is written with its event or not at all, wherever the daemon is
  stopped; and a line of a record's history is written with the news of
  it and with where the keeper of histories has read to, as one; and a
  message is written, what is attached to it made part of it, and
  `channel.message` said, as one (`Channels::insert`). Whoever
  writes a record hands over what is to be said of it (`Say`,
  `Collections::add_saying`, `put_saying`, `remove_saying`). A table
  kept as files of the project, by a store service, or as the documents
  or mockups of a folder is written where the journal's write does not
  reach: it is written and then said, as it always was.
- **What was written and never said is said when the daemon is back.**
  That is what a journal a program from before left may hold (it wrote
  a row and then said it, and a stop could come between the two: a
  person's move that no blueprint heard), and what a table not kept in
  the database may still be left as. What was said is in the journal
  and what is so is the row, so when the daemon has started the board
  holds the two against each other (`board.settle`, asked by `std/owed`
  and by `std/cards`, which asks as it comes up too: one coming up as the
  daemon started was not listening when that was said): a card whose row is in another column than it was
  last said to be in has the move said then, late, and from there it is
  any move. It is done under the record's turn, as a move is made, so a
  move being made at that moment is seen whole or not at all, and looked
  for twice it is found once. Nothing is judged by what a row looks like.
  What was said as the card was moved, and who moved it, went with the
  event that was never kept: it is said by the system, which holds
  nothing. A card added with nobody told is said to have been added, the
  same way. A board is looked at once for a start of the daemon,
  whoever asks and however often (kept in memory; asked again, the
  answer is at once that there is nothing). And since a row is said in
  the moment after it is written, only rows written since a minute
  before the last thing the journal held when it was opened can be such,
  and the database is asked for those alone: on a board of 5,000 cards
  all said long ago the look takes about 8 ms, where reading the
  board whole takes about 55 ms; with all 5,000 written in the minute
  before the stop, each held against what was said of it, about 0.4 s
  (an unoptimised build, on a machine at work). A board kept by a store
  service does not say when a row was written, and is read whole.
  A minute is far more than can pass between a row's writing and its
  saying. Whoever moves a card holds the record's turn, writes the row,
  and says the move as the next thing it does: nothing is asked of an
  agent, of git, of the network or of anybody in between, and no other
  turn is waited for. The one thing waited on is the journal's
  database, whose one thread does each piece of work in the order it
  was asked, and which waits five seconds at the very most for a file
  somebody else has before it gives up. So a row written a minute
  before the last thing the journal kept was either said or its saying
  failed while the daemon went on, which is another thing and is not
  looked for here.
  What is said on a record (`record.commented`) has no such moment: it
  is kept as its saying and no other way, and the line of the record's
  history is written from the event. A daemon stopped as an answer was
  said has either kept it, and what looks after the cards takes it up
  from the journal when it is back, or has kept nothing, and whoever
  said it was not told it was taken.
- **A place asked for before its pool was listening is asked for
  again.** A handler hears only from when its instance is active, and a
  stage and its pool are started side by side: a stage that found a card
  in its column as it came up could ask the pool before the pool heard
  anything, and the card stayed there for good, with no stop needed (20
  plays of 160 with nothing in their way, eight at once on eight
  processors). So a pool says when it is listening (`pool.listening`,
  from a handler on its own `instance.active`: when it is first there,
  and each time it is carried over), with who holds a place and who
  waits. A stage that works says each card of its column is due again
  when it hears that, and each asks again: a name that waits is kept
  once, and a turn that comes for a card which has gone on, or is in
  hand and owed no round, is given back by what its row says. A review
  waits for its own place and cannot ask twice (answered before its
  wait was there to hear, it would hold a place and wait for its next
  for good): one whose session has not begun, and which the pool has
  not got, ends, and its stage begins another in its place. It holds
  in either order (a stage that comes up after its pool asks a pool
  that hears, and hears nothing it needs), where one of the two runs a
  text from before (which says or hears nothing of this, and is as it
  was), and at a restart: the pool's handlers have their places before
  it is said to be active, so a daemon stopped between that and the
  saying says it when it is back, and a stage hears it from its own
  place. What asks in its steps, which hear nothing while they run, is
  not helped: such a thing is started once its pool is there.
- **The rows a table starts with are added so that it is the same done
  twice.** A fleet gives each of its two tables its first rows once, and
  adding a record is not something done twice: stopped as one was added,
  with the step not yet having kept that it was, the daemon that came
  back refused the step, the fleet's steps failed and it was over with
  the rows it had got to left in its tables. And since its text fills a
  table only while the table is empty, applied again it added none of
  those still to come. So each table's rows are handed to one action
  (`rows.start`, `studio::first`), which adds those the table has not
  got, each found again by what it is called, and is safe to do again:
  a row added before the stop is found, the rest are added after it in
  their order, and none twice. It adds and never changes or takes away,
  so a row somebody has changed is the row of that name with what they
  gave it. Whether a table is still to be given its first rows is the
  text's to say, as it was (not once the fleet has been up, and only
  while the table is empty): a table somebody emptied stays empty, and
  a row taken away stays away. The gate plays a stop as each row of
  each table is added.
- **What does not hold yet is written where it is known**: a place in
  the scenario's `not_yet`, with the sentence, and the rest, found by
  stopping at every place, in a note before the scenarios in
  `simulated.rs`, each to be played by name. No scenario says a place
  today.
  Known besides, and not held by any play:
  - A round begun again says a second time what it says for itself
    (`card.worked`, `card.went_on`, `card.pushed`, `card.unkept`, and
    what it posts where the stage reports), where the daemon stopped
    after its agent had finished and before the round was over. Its
    agent is asked once and its question is said once; nothing acts on
    the rest. A scenario does not see it: what was said in a channel is
    compared with each thing counted once (`observed::said`), and a
    card's history is not compared.
  - Nothing takes a turn for a stop: two stops of one instance in the
    same moment can both do what its text says is done as it stops.
  - A handler's run that is under way when its instance takes a text by
    its steps running again is cancelled with the generation; where the
    conductor stops between the text being written as taken and that
    run being cut, it is taken up at the start and ends well. The
    runner's play of a stop whose asker is cut short (`meets.rs`) found
    it, and is not stopped at that one place.
  - A step that was passed over, whose record was held and not yet
    written when the conductor stopped, reads as cancelled where its run
    is ended at the start (`conductor::left`). And `instance.failed`,
    said at a start for an instance a `call` is waiting on, may say no
    `called_by`: who waits on a call is kept in hand and not written
    down.
  - No real agent's program has been through a scenario: the agents are
    stand-ins.
- **A node that is let go of gives back what it held**, stopped or not:
  what it followed (`hosting::Watch::close`), its databases' threads and
  files (`journal::Db::close`), the logs it had open (`Logs::close`), and
  what it started, ended (`Drop for Node`). A process plays hundreds of
  nodes, a few at a time (sixteen at most), and holds what those hold.
- **A play is over when it has come to what was meant**, and one that
  is somewhere else is a fault once it has said nothing for two seconds,
  or come no nearer for six, and is at rest there: nothing is under way.
  That is asked and not told from the silence. Under way is a run of the
  runner that has not ended, beyond those the scenario says are there at
  rest, and an agent's turn whose program is at work, which may have no
  run waiting on it (a turn asked for in a channel; one a daemon found in
  hand and took up before what asked for it was begun again). A service
  is not: it is meant to stay. While something is under way the play is
  waited for, forty seconds from when it first was; a slow commit says
  nothing for as long as it takes, and a play judged in the middle of
  one was a fault on a busy machine and never again. One still not at
  rest then **did not come to rest**: its own kind of result, said with
  what was under way and what was said last, counted apart from a fault
  and failing the run all the same. So something that hangs is found
  later than it was, and nothing slow is taken for something wrong. Two
  tests hold the harness to it (`simulated.rs`): a program that says
  nothing for longer than a play may be silent, and one that never ends.

The same stop is there for whoever writes a blueprint: `mw down --at
<step>` (*Playing one to see what it does* in
[blueprints.md](blueprints.md)).

The runner is held the same way by itself, apart from any fleet
(`crates/conductor/tests/suite/anywhere.rs`). A play there is a small
blueprint and what is done to it from outside: it is played once and
what it came to is written down (how each run ended, each step and what
it gave, what each instance keeps, what was said and how often, what was
asked of the stand-in actions and how often), and then once for every
place it passed, with the conductor stopped just after that place and
started again. It is stopped two ways at each: with the journal and its
database shut there, as if the conductor had gone in that instant; and
with nothing said after the place while what was being written is
written, which is what stopping a daemon does to work that had asked for
something to be written and was to say so afterwards. The second way
holds because what the runner writes down and what it says of it are
one write (`Journal::write`, `runs::together`): stopped at any
instant, both are there or neither. There is a play for each way a run goes
(`plays.rs`, and `begun.rs` for an instance written down and not yet
begun), and every word of the language is in one: a test reads the
plays and fails for a word that is in none.

The plays of a scenario are made several at a time, each on a thread and
a conductor of its own, and are the plays that were made one after
another: the ones no other play tells anything are made first, what the
rest will be told is worked out from those, and each play the old order
asks for is taken from those already made where one was told that very
thing, and made then where none was (`anywhere/abreast.rs`, with the
test that holds the two ways to the same plays).

A play holds its databases between turns from when it is begun until it
is over, through each start of its conductor and whatever it asks
itself, and is made on a runtime that looks at what its own thread woke
before what a database's thread woke, always. So the order in which its
conductor keeps two things is one order on any machine, and with it
which plays are made of a scenario: the first play's order of places is
the order the rest are made in, and a play finds its second stop among
what is kept after a start. How many plays each scenario comes to is
written down (`crates/conductor/tests/fixtures/played.txt`) and a
scenario that comes to another figure fails its test: a count that
moves says an order goes by something nobody wrote. Held only while a
play waited to be at rest, the databases answered in between as the
machine had it, an answer could be there before whoever asked had
looked for it, and that one went on without giving way: the suite made
between 7,156 and 7,172 plays of the same places from run to run, and
once in some thousands of plays came to an order the runner did not
hold. An order that only a busy machine comes to is made where a play
is there for it: something asked by whoever does not give way
(`Stepped::at_once` in the suite's `stepped.rs`).

Something asked for and answered is owed, whoever asked having gone on.
A step that asks for something running to take a text that says how
(`on_update`) is answered when the run that will carry it over is
there, and that run waits until what it carries is between the runs of
its handlers. What it is to be carried over to is written with the
run, in the one write that makes it, and a conductor that starts
carries it over: each instance owed a text waits for it again before
anything goes on, its handlers are started so that what was under way
ends on the text it began with, and then it is carried as it would
have been (`answered.rs`, where the text is taken only when time is
let pass, so that every place between the answer and the taking is
played). The step that asked, taken up before it had kept its answer,
asks again, and is answered with the run there is whether it or the
start comes first; a text asked for of something the start has not
come to yet waits for the start in any case. Who came first to a text
kept from before is measured as who began an instance is
(`instance.begun`, `owed: text`), and a second play there has the step
come first. A run that a program from before this made has nothing
kept, and ends as it did then, saying to apply the text again.

While a conductor is starting, each instance says whether the start
has come to it, and what waits to carry one over waits for that. So
every way a start leaves an instance comes to it, at the end of what
the start does for that instance: taken up in the middle of its steps
or of taking a text, found at rest and its handlers started, begun
where it was written down and never begun, settled as failed where it
cannot be begun, or put back on the text it had. What is over before
the start goes over what is alive was never said to be waited for: an
instance that keeps nothing, one whose stop is finished there, and
what was under one that failed and is taken down. One that is taken
down or stopped later, by what the start does for something above it,
is let go of by being stopped and is come to all the same; the run
that was to carry it over ends there, once. And a start that cannot
go on (it cannot read what it kept) comes to all that is left as it
gives up (`carry::Reaching`). What is kept of a text to be carried
over to is called off, in the write, by whatever takes a text or asks
for another: a text taken where the instance is as much as one whose
steps run, and what waits in hand is let go of in the same turn, so
that an earlier text never lands after a later one, stopped or not
(`waited.rs`). The run keeps which step asked, however it ends, and a
step taken up after a stop finds there that its asking was answered
once the text was taken: asked again, a text that failed as it was
taken was carried over and failed a second time (`below.rs`).

One ending is several writes, because something slow comes between its
halves: an instance that failed says so, what it started is stopped, and
then it says it is over. A conductor that starts finishes each such
ending that was cut short (`Engine::take_down_owed`, by the note the
failing left on the row), and stops what was started as it would have
been stopped: each thing still alive under the failed instance is
brought into hand first, so that what its text says is done as it stops
(`on_stop`) is done, and is then stopped like anything else. A run that
was under way and whose instance is over, by then or because the
instance was written as over a moment before the run had ended itself,
is ended as a run that was cancelled (`conductor::left`): the step it
was in is written as cancelled, the steps it never came to likewise,
and it says that it finished, in one write. Nothing is left reading as
under way for good, and what is read of such a run is what is read of
one cancelled with the conductor never stopped. A run of a handler that
the text its instance now runs no longer has is ended the same way,
saying why; and an instance that keeps nothing, found alive by a
conductor that starts, is written as stopped with why, its runs as
cancelled and all of it said, in one write. No run and no instance is
written as ended by its status alone.

What the conductor says of its own steps and runs is held to the same.
A step that is passed over is said to be in the write that keeps its
record, and so is a step whose action only decides, which is held as it
begins and as it ends and has no write of its own; that something done around a step could not be done, in the
write that keeps it as done; that a text was refused, in the write that
ends the run that was to carry it over; and a handler a text brings has
its place in the journal written with the text. A run that is taken up
and had never said that it started says so then; where that it began
was only passed on, its row says it began. The plays hold of
every run, in every play, that one that ended said so once with none of
its steps left under way, and that one that went on said it started
(`amiss.rs`); and what is said of steps is held by handlers that hear
it (`apart.rs`).

An instance is written down, and then its steps are begun: two writes,
and a conductor stopped between them starts with an instance that is
owed its beginning. Two things then come to it, in no order: the start
itself, which goes over everything alive, and the step that asked for
the instance, taken up as its own run goes on. There is one way of
beginning it (`Engine::begin_owed`), asked by both in the instance's own
turn, where what is so is read, the instance and the last run of its
steps as of one moment (steps end in no turn, and read one after the
other a run that ended in between was over beside an instance still read
as starting, which is one owed its steps again): the second to come
finds the run the first began, and waits on it if it is a `call`; or
finds that the instance could not be begun, which it says once, and the
step fails with what it was answered then. A step that is taken up finds
the child it made by more than its name, since one that has failed is no
longer found by name and another would be made: a `call` by where the instances
stood before it asked, which it had written down; a `spawn` by its child
having been said to start after the step began. Who came to it first
is measured: `instance.begun` is how long an instance had been owed its
beginning, by who began it (`by`: `start`, `step`, or `apply` for
somebody who asked for it again), and which steps it was owed (`owed`:
`first`, or `again` for those of a text it had taken: an update is two
writes too, and one stopped between them is begun the same way; or
`text`, for one owed no steps but a text it was waiting to be carried
over to).

What an instance becomes as its steps end is decided in the write that
says so. Working it out takes a while (what an update that failed had
started is handed back first), and an apply or a stop may come in that
while: so the write itself asks whether the run still speaks for the
instance, and where it does not, the run is written as ended and
nothing is written or said of the instance. What is in hand of the
instance is brought to what was written inside that write. The other
writes that say what an instance is are held the same way: it is
written as stopped, or as taking a text, only where it is still
running, and whoever applied over one that had just ended is told so
and asks again.

Somebody who applies over an instance is answered when its steps have
begun, and what is under it (what its handlers started, which no step
brings to the text now there) is looked at when they have settled.
That the look is owed is kept with the text, and taken off when the
look is done, so a conductor stopped in between still looks: as the
steps it takes up settle, or at once where they had. A text taken
before the look is over, whoever gives it (a step that carries the
instance over, or has its steps run again), keeps the note, and the
look is done again once that text's steps have settled; a look that
was under way meanwhile does not take off the note the later text kept
(`Engine::take_owing`, in one turn with the note being taken off). And
what the instance would go back to says that a look was owed from
before (`Started::below`), so that one gone back to is looked below
still, where one gone back from what somebody asked for owes nothing
(`below.rs`).

What a text says is done as a running instance takes it (`on_update`),
and as an instance stops (`on_stop`), are lists done by themselves, and
are kept as what is done around a step is: that the list has begun,
before the first of it, and how far it has come as each entry is done,
with a `set` among them written in the same write as the count. For
`on_update` that is in the run that carries the instance over. A stop is
done in no run, so it is in the instance's own row, with why it was
asked and a note that the stop is owed; the note is kept for the top of
what is stopped as its stop begins, whatever it does as it stops, and
for what is under it from the first of what that does. Taken up after a
restart, an `on_update` goes on from the entry it had come to. A stop
that had begun is finished by the conductor that starts
(`Engine::finish_stops`), before anything is taken up, from the entry
it had come to: an instance whose work was called off, and what is
under it, is not brought back to go on as if nobody had asked. Either way the entry that was under way is done
again, told so, and looks first for an event it had said; no other is
done twice.

A step with things done as it starts is under way from the first of
them, and its action begins when the last is done. Taken up with one of
them still to do, its action never began: it is done then for the first
time, and one that cannot be done twice is not refused for a stop that
came before it. Taken up with all of them done, its action may have
begun and is done again as any step's is, or refused where it cannot
be. What the action itself had said is looked for after the last of
them was done, since an entry may say the very event its step says, and
both are said by the one step.

### Stopped once what is in hand is done

A stop anywhere is held, and costs what was cut off: an agent's turn
begun again is its program's work done twice. So a daemon may be asked
to stop once what it has in hand is done (`mw down --drain`, `stop` of
`/api/system/stop` with `drain`, `api::drain`). From the moment it is
asked it begins nothing new, and that is held where work is begun and
nowhere else: nothing somebody applies (`Engine::apply` refuses it in a
sentence that says the daemon drains), no handler's run (a handler that
comes to an event waits where it is with its place unmoved, as one told
to take no more does, so that the next daemon hands it the event again:
`Engine::hold_new_work`), and no agent's turn that no run asked for
(what is said to one in a channel waits, queued, for the next daemon:
`agents::drained`). A turn a run under way asks for is that run's work,
and is begun. It then waits for the turns in hand (at work, or answered
and not yet said to have ended) and the runs under way, in every file
that keeps either, to end, looked at again each time a run or a turn is
said to have ended and never on a timer, and stops as `down` stops.
What waits for a person (a review, a question, a card held) has no run
and no turn while it waits, and holds nothing up. The wait has a bound,
always, said as it begins (ten minutes unless said, a day at most): past
it the daemon stops anyway and says, in what `system.stopping` carries
and to whoever waits on `/api/system/drain?wait=true`, what was still in
hand and what becomes of each: a turn whose program goes on without the
daemon is taken up where it is, and any other is begun again, or left
for a person, as whoever asked for it said; one whose program is on
pipes (handed something in no variable) ends with the daemon whatever its
slot says, and is said so. `mw status` and every page of the client say
that it drains and what it waits for, read from `/api/system/drain` as
it begins and again as each turn or run ends, and each page says that it
stopped and how (`draining` of `/api/meta`, followed as
`system.draining` and `system.stopping`). Putting a tree back on an
earlier text (`Engine::put_back`) is refused as an apply is. A plain
`down` asked meanwhile stops it at once.

Not held: a run under way that waits for something nothing will say
while the daemon drains (a `wait` for an event a handler would have
said, or for a long time) holds the drain to its bound, and is then
taken up by the next start as any run is.

## How an event reaches a handler

Handlers do not follow the journal. One task in the conductor does
(`crates/runs/src/delivery.rs`, begun as the conductor's own through
`runs::Hands`), and it keeps every handler that is
waiting by what it waits for: its topic, and under that the field and value
of its `for:`. An event is looked up there and handed to the handlers found.

- **An event is heard by project.** A handler that says no `scope` is
  handed the events of its own project and those that are nobody's, and
  one in no project only the latter; `scope: fleet` is handed every
  project's. Whose an event is, is asked in one place (`Engine::whose`
  in `crates/conductor/src/whose.rs`): the project its data names, else
  the project of the instance that said it, read from what is running
  or looked up once and kept in memory. A `wait` asks the same.
  An instance whose row says nothing of how it hears was applied before
  this, and is handed everything, as it was.
- **The conductor decides who an event is for**, not each receiver. Before,
  every handler was woken by every event and tested it for itself: with an
  instance for each card, one card moving cost a wake-up and an expression
  for every card on the board. Now it costs a lookup and wakes the card
  that moved. A handler with no `for:` is handed everything on its topic,
  as it was, and nothing on any other. Patterns over topics (`board.*`)
  cannot be looked up and are tried one by one; they are few.
- **Delivery is durable, in order and exactly once, as it was.** Each
  handler has a place in the journal, moved in the transaction that records
  the run an event starts. A handler that starts to listen is put on the
  index first and reads the journal from its place second, so what was
  published before it was on the index is in the journal by the time it
  reads, and what was published after is handed to it. What reaches it both
  ways is told apart by its sequence number. An event that happened while
  the daemon was down, or while the handler was being started, is handled
  once when it can be. The place is one in the journal the handler
  follows, kept with what that journal is called (*A journal for each
  project*).
- **A handler that is never woken cannot move its own place**, so it is
  moved for it: every 256 events on a topic, the places of the handlers
  waiting there with nothing in hand are brought forward in one write.
  After a restart a handler reads no more than that much of its own topic
  again, through the journal's index on topics.
- **Nothing piles up for a handler that cannot keep up.** At most 1024
  events wait in memory for one handler; past that they are let go and it
  reads them from the journal, where they are anyway.
- **It is measured.** `handler.delivery` is how many handlers each event was
  handed to, by the topic they wait on; `handler.event` is how long an event
  waited before its handler looked at it, and its count is how often
  handlers were woken; `handler.catch_up` is how many events a handler read
  from the journal when it started to listen.

None of this looks at anything on a timer: the task is told of each event
by the journal, and a handler is told by the task.

**What an event carries is said once, and held both ways.** Each crate
lists the events it emits in its `events.rs` (`blueprint::EventSpec`: the
topic, a sentence, each field with its type and whether it is always
there) and hands them to the registry beside its actions
(`Registry::emits`); the node says when every crate has
(`emits_no_more`). From then:

- A handler is checked against them when its blueprint is
  (`crates/blueprint/src/heard.rs`): its topic, its `for:`, and every
  `event.data.<field>` it reads. A topic outside the families this program
  keeps for itself is something blueprints say to each other, and is held
  to what their `emit` steps write, read from the library when a handler
  first asks.
- The program is checked against them as it emits, in a build made to be
  tested and in no other: the node hands the journal a check
  (`Journal::hold`), what it finds is kept (`Journal::faults`), and every
  node a test starts is asked as it is put away (`Node::undeclared`). A
  build made to be used looks at one empty cell for each event.
- A text that was kept is not failed for what its handlers hear
  (`Engine::compile`): it was checked when it was applied, and a handler
  whose event is gone does not fire. A text an instance is to be given is
  checked in full.

**What is offered is kept with the code** (`contract/offered.txt`, and for
what a package brings `extensions/<name>/contract/offered.txt`; written
and held by `crates/folders/src/contract.rs` and a test), with the list of
every time some of it was broken and why (`contract/broken.txt`), which
the conductor reads to say why a kept text no longer checks
(`crates/texts/src/broken.rs`).

## Agents

An agent works from a **slot**: a name, a project, standing instructions, a
permission mode, and a credential of its own whose scopes are what that slot
may do. The credential is a file readable only by the owner; it is never put
in an environment variable or on a command line.

- **The application knows no agent by name.** It knows how to run *an*
  agent that is a command-line program printing one JSON value a line. Which
  program, how a turn is put to it, how what it prints is read, and how it is
  asked which models it offers are all said by an **extension package**: a
  folder with a manifest, written in the language blueprints are written in.
  Packages for Claude Code and Codex are kept in `extensions/`; neither is
  installed until someone adds it, and a gate (`providers`) fails if anything
  outside a package names what a package is for. The reference is
  [extensions.md](extensions.md).
- **Models are the program's own list.** A package says how to ask its
  program, so the choice offered is what that tool's own picker shows for the
  account it is signed in with, and does not go stale. Asking sends no
  prompt. Naming a model by hand stays possible.
- **How much an agent may do** is one of three words of the application's
  own (`read`, `edit`, `full`); a package says what each means to its
  program.
- **Two things need no package**: `builtin`, a small offline assistant, so
  the product can be tried and tested with no model; and `--command
  name=program`, for anything that just reads a prompt and prints a reply.
- **The command line, not MCP.** An agent acts on the factory by running the
  same `mw` command a person does, signed in as its slot. That needs no tool
  schemas in the prompt (on the old system the tool descriptions alone were a
  noticeable share of every turn), has no second API to keep in step, and is
  something a person can run to see exactly what the agent saw. `mw help` is
  kept short on purpose. An MCP server can be added as a thin wrapper over the
  same API if a harness needs one; nothing here prevents it.
- **An agent is briefed, and asked.** What it is and how it stands (its
  name, its project and working folder, the factory's command, what its
  wall shows it, the variables its keys are under, what its slot lets
  it do in words (what it may ask of the factory by area, what its
  program may do on the device, what it signs in with by kind, and that
  a person or an agent holding what it gives sets its slot and never the
  agent itself), the standing instructions it was given) is written before each turn into the file its
  package names as its program's own standing instructions
  (`agent.wall.instructions`), where that file is its home's own, and what
  it is handed with the turn is what it was asked and one line more
  (`crates/agents/src/told.rs`). Told it ahead of every message, an agent
  took it for part of each, and one asked about its instructions went
  looking through the factory's source for where they came from. So every
  turn behind a wall is in a home of the agent's own, shown only its
  sign-in of the home of whoever runs the daemon, and the file is that
  home's. It is never written in the home of
  whoever runs the daemon: where what is there is theirs (an agent whose
  package cannot tell its sign-in from the rest), where the program is
  pointed at a folder in place of its home, or where there is no wall, the
  brief goes ahead of the prompt as it always did. A brief the factory
  wrote that is not a turn's is taken away, by the comment it begins with.
- **What an agent's program signs in with is its slot's to say.** Said
  nothing, it is whoever runs the daemon, the same for every slot. A slot
  may instead say what it draws from, in order (`subscription`, `key`, or
  one by name), of the sign-ins the factory holds: keys, a subscription's
  token, or the folder a provider's program keeps its own sign-in in, which
  is used where it is and never copied. One is chosen as each turn starts,
  held for that turn by no more turns than it says (one, unless said), and
  given back however the turn ends; a turn with none free waits to be told
  one was given back. A limit is the sign-in's, not the agent's: it is set
  aside until it is good again and the turn begins again on another. The
  mechanism is `crates/agents/src/signins.rs`; **which** one is handed out
  is a blueprint, `std/signin`, that is asked and never run (it has
  `params` and `outputs` and no steps), and that a project replaces with a
  file of its own. How a program is handed one, and how a limit shows, is
  its package's to say. A value is in a file only its owner reads, and in
  the environment of the one program it is handed to: never in the
  database, an event, a measurement, a log, an argument, or a copy.
- **An agent's program is started as everything else is**: with the
  daemon's environment less the daemon's own credentials (`hosting`'s
  `dress`), told where the daemon and its program are and what the
  profile is called, and then given its slot's four variables.
- **Turns are queued per slot and survive restarts.** In a channel, an agent
  answers when mentioned, or when it is the only agent there. Agents talking
  to agents are capped per hour.
- **A turn found in hand when the daemon starts was cut off, or went on.**
  A turn is `running` in the database only while a daemon has it, so one
  found so by `Agents::open` was left by an earlier daemon
  (`left_in_hand`). Its slot's worker takes it first, counts the stop on it
  (`restarts`) and decides once (`turns::run`): left, as whoever asked for
  it said or because it was stopped too often or out of time; begun again;
  or, where the slot `outlives` the daemon, taken up where its program is
  (`agents::running`, over `hosting::kept`: a wrapper of its own that
  writes its number and how it ended to files, found again by the turn's
  id and never by a path written down). Every turn's program is started
  by such a wrapper and read from its files, so whether it goes on is
  asked of its slot as the daemon stops, and a slot changed while its turn
  is in hand holds for that turn: one that does not outlive the daemon is
  ended as the daemon lets go of it, with its group, and said to have been
  (`Process::cut`), so that the next daemon begins the turn again and does
  not take what it left for an answer; one the daemon could not end (it
  was killed) is ended by the next as it finds it, where the slot says so
  then. Only a turn handed something in no variable is on pipes still, and
  ends with the daemon whatever its slot says. Whoever asks for the same thing
  again is given that turn, once (`Agents::claim`); a step that was kept
  waiting for it goes on waiting (`Task::resume`). What such a program
  prints is written without what it was handed by its wrapper, which
  stands between it and its files (`hosting::wrapped`), since no daemon
  is there to take it out later.
- **What is written of a turn and what is said of it are one write**
  (`agents::together`, through `Journal::write`): queued, begun and
  ended, each the row and its event (`agent.turn.queued`, `.started`,
  `.finished`) kept together or not at all, so a daemon stopped at any
  instant leaves no turn written as queued, in hand or ended with
  nobody told, and a turn called off before it began says that it
  finished. One thing is two writes: where an agent's answer is said
  in a channel, that is the channel's own write, and it comes before
  the turn is said to have ended, since whatever hears that finds the
  answer there. So the answer is kept first, saying nothing, with the
  turn still in hand (`answered`); it is said where it was asked; and
  then the turn is written as ended and said to be. A daemon that
  starts and finds a turn answered has its slot's worker go on from
  there: its agent is not asked again, its answer is said unless it is
  found there already (its words, by it, in answer to the same thing,
  since the turn was answered), and it is ended and said to be, once.
  A turn with nothing to say in a channel is never `answered`.
- **A thread is a conversation that needs no names.** A thread is one
  message and what is said under it, one deep, and kept out of the channel's
  own flow. Under a message, a person is talking to the agent they were
  already talking to there: the one that spoke last in the thread, else one
  the message was for. The agent is shown the thread, not the channel (it
  can read the channel itself with the command line), answers in the thread,
  and remembers each thread as a conversation of its own. Nothing here asks
  a model whether a message is for it: who a message is for is decided by
  where it was said and who was named, which costs nothing and surprises
  nobody.
- **Who work goes to is decided in one place** (`crates/agents/src/picking.rs`).
  Work given by a blueprint goes to the agent it names; failing that, to
  whoever the conversation it names was last held with; failing that, to
  whichever agent of the role asked for has least in hand, and with
  `apart` not to one that a conversation beside this one is held with
  (three seats on one card are three agents). Which it is of several
  with as little in hand is a word whoever gives the work says (`pick`,
  `picking::Order`): the one brought up first, each in turn, the one
  given the least so far, or the one ranked highest by a number its
  fleet gave it. What a rank means is nothing the daemon knows; a
  fleet's stage says the word in its row. Choosing and putting the
  work in the agent's queue are one thing, done by one asker at a time:
  two choices made together would both find the same agent free. A slot
  has a `role`, which is a word and nothing more; nothing here branches
  on what a role is called. A choosing asks nothing of the turns that
  grows with how many have been kept: how many each agent was given,
  and its last, is counted as turns are queued (`Agents::was_given`)
  and read from the turns only as agents are opened and where turns are
  taken away with a channel; what each has in hand is asked of the
  index of turns by slot and status, an agent at a time. Asked of all
  the turns on each choosing, as both were, the two took 0.05 ms of the
  database's one thread at 260 turns, 1.1 ms at ten thousand and 37 ms
  at two hundred thousand; they take 0.01 ms at each.
- **An agent stops one of two ways.** At once (`Agents::deactivate`): what
  it was doing is stopped where it was, what waited behind that is not
  begun, and its credential is revoked. Or when it has nothing in hand
  (`Agents::leave`): from then it is given no more work by any of the
  three ways above, it keeps its credential for what it is finishing, and
  its worker stops it when its queue is empty (`gone_if_done`). Which of
  the two a fleet means when it has fewer of a role is the fleet's to say
  (`std/role`, `leave`).
- **An agent is set up by a blueprint** (`std/agent`, whose params are
  its slot's settings and whose one step is `agent.ensure`): those a
  role brings up are each one under the role, and one added by hand is
  one keyed by its name, applied for its project and under its
  environment as whoever asked (`api::byhand`), so that a project's fork
  sets its agents up its own way. What is held to whoever asks is still
  held in the action; the routes ask it first only to refuse in its own
  words. An instance that holds an agent and is keyed by its name is the
  agent's own: under something else, that is what brings it up and
  counts it (`Brought::By`, a role by its row); with nothing above it,
  the agent was added by hand (`Brought::Own`), and is changed by its
  text being applied again, moved by its text being applied in the
  other project before the one it had is stopped, and taken away with
  its text stopped first. An agent goes with the last thing that holds
  it, so the stop of the text it had stops nothing of one that moved.
  One stopped or leaving is changed in its row, since applying would
  bring it in. An agent from before has no text: the step that added
  `slots.owes` noted every one, and a start gives each that is in, not
  leaving and brought up by nothing its text over its slot as it
  stands, as the system, and takes the note off (`byhand::take_up`).
- **Stopped, an agent is kept; taken away, it is gone for good**
  (`Agents::remove`, `agents::removed`), which is a person's to do, who
  manages the factory, and never an agent's. It is refused while
  something that is running holds the agent (an instance's `holds`
  says `agent:<name>` from `agent.ensure` until `agent.stop`: a fleet's
  agent is changed in its row) and while it has work in hand. Its slot,
  its place in every channel and what its program remembered of its
  conversations go in one write with what says so
  (`agent.slot.removed`); its credential is withdrawn before that, and
  the folder that was its own taken away after. What it said and did
  stays, under its name, and the name is free: a slot made for a name
  that has turns says which turn of all was the last before it
  (`slots.since`), and who holds a conversation, what an agent was
  given and what is listed as its work go by that, so a later agent of
  the name inherits none of it. Nothing is added to what a copy takes
  or to what goes with a project moved apart: no table is new, the mark
  is a column of the agent's row, and a turn keeps its number wherever
  it is moved.
- **What a fleet is, is rows.** Who is in a project's fleet and what they
  do with a card are two collections of the project, `agents` and
  `stages`, declared by `std/fleet` like any other. The daemon knows
  neither: a row changed is `record.updated`, the fleet's handler hears
  it and calls `std/arrangement`, which reads both tables and brings up a
  `std/role` for each row of the first and, for each row of the second,
  the text for what is done in its column (`std/working`,
  `std/reviewing`, `std/planning`), handed the row. So a fleet is changed where a table is changed, by a
  person or by anything else that can write a record, and whatever else
  is to follow hears the same events. A stage is carried over when its
  row changes (`on_update`) and never begun again, so that what it was
  in the middle of is finished first.
- **What more is done with a card is rows too, and small blueprints.** A
  third collection, `segments`, says what is done with a card where it
  is beside what its stage usually does: each row a small blueprint
  called for the card (a segment), the stage whose list it is in, and
  where in it. The rows are changes to the usual list, so a stage with
  none does what is usual and asks nothing. What comes next, and what a
  segment is handed, are blocks that know no card (`agents::segments`);
  the runner was not changed for any of it. Where cards are reviewed
  what is usual is two things, those who look (begun by the stage, which
  hears what they decide) and what was approved put with the rest
  (`std/merge`), and the list is gone through for a card by something
  that keeps nothing (`std/segments`): from the first of it until there
  is a review to begin, which the stage begins, with who looks and how
  many have to approve as a row that names a review of its own says;
  and again from where that review was once it has passed, the merge
  among what is called, until the list has ended or there is another
  review. The stage keeps with each review where in the list it is.
  One handler hears what a review came to, whichever verdict
  (`decided`), asks once what that comes to (`review.decided`,
  `agents::decided`) and does each thing that follows in one place:
  the card on, back, or left for a person. A stage with no row calls
  nothing but the merge, and one with rows goes through the list before
  a review only for a card something is laid before: what a card is
  owed says, in the one asking, which of the things done there its
  review is and whether anything is called first (`agents::begun`), so
  a row costs the cards it is for and nothing else. Where cards are
  planned, or handed to a blueprint of the row's naming, what is usual
  is that one thing, begun by the stage as it always was, and a row is
  laid before it or after it the same way: what is after is gone
  through when what was begun says what it came to and leaves the
  handing on to its stage (`stage.came` with `hands`, heard by the
  stage that began it), which then has the card moved by something
  that keeps nothing and looks at the card first (`std/onward`), so
  that a daemon stopped as the card was handed on moves nothing twice. A stage from before,
  whose run said what one thing came to as an event the stage heard, is
  heard by the stage as it now is when that run ends
  (`heard`). A handler that takes turns by a key goes on
  while its instance is given another row, and reads its params as
  they then are, so a list is handed the rows as it is called and
  nothing of them is read after: a row added meanwhile is for the next
  coming of a card.
- **A round is a list, gone through by something that keeps nothing.**
  Where cards are worked the usual things are segments already: the
  work (`std/work`), the keeping of it (`std/keep`) and the sending
  (`std/send`), which were one text and are three, with the project's
  rows laid among them. What goes through them is the round
  (`std/lane-round`), which takes the card, asks the same block what
  comes next and calls it, and hands the card on. The language has no
  going round: the round calls the first thing, and the rest is a text
  that calls the next, asks what comes after it and calls itself with
  that (`std/through`), a level of the tree for each thing done, which
  goes on through a reviewing or planning stage's list too. What bounds
  the depth is how many things at most a stage does in one going
  through (`most` of its row, 6 where cards are worked and 5 otherwise),
  which the block holds to: the asking that would call one more leaves
  the card for a person, saying how to raise the number. Neither
  the round nor what it calls keeps anything as it goes, so a thing done
  in a round costs an instance that says it began and ended and nothing
  of the stage's: no run and no step. That is the difference from the
  chain a reviewing stage goes through, where each segment is a run of
  the stage's own that is kept, and it is what a round was before it was
  a list: begun again where the daemon stopped, from the first of what
  it does, finding done what was done. For that an agent's turn has to
  be found again by something a round started and not by the round, so
  whoever asks for a turn is named by its key under the key of what
  started it wherever that keeps nothing, which is what a thing begun
  again still has. Measured with a worker
  and a reviewer (2026-11-20), a card is about 214 events and 670
  pieces of database work, in 7 instances: the round and the three
  things of it (the work, the turn it gives its agent, the keeping),
  the review, whose seats are one step of its own, the merge, and the
  tidying
  of the card's place when it is done (`std/cleared`, called by the
  stage that worked it, which says what was kept where anything was).
  A row laid into a round, or about a review, adds an instance for the
  thing and, about a review, one for the going through (some thirteen
  events and sixty pieces of database work for a command before a
  review); a row that is not for the card adds nothing. Each thing done
  after the first of a going through adds a level of `std/through`:
  measured (2026-11-20, a card alone from ready until it is done, the
  tidying after that left out, three runs each way, before and after the
  written-out askings went), a card with nothing laid in went
  from 6 instances, 200 events and 657 pieces of database work (256
  that wrote) to 7, 203 and 678 (261); with three commands laid into
  its round from 9, 218 and 892 (279) to 13, 233 and 977 (299); with two
  before its review and one after from 11, 231 and 956 (298) to 14, 243
  and 1,010 (313). So a level is an instance, a run, some four events
  and twenty pieces of database work, five of them writes.
- **A message says which model wrote it.** A package's output rules can pick
  out the model the program names as it starts; that, or failing it what the
  slot asks for, is put down with the agent's message and shown beside it.
- **A channel has three kinds of member**: people, agents and bots. A
  bot is a blueprint that joined under a name of its own (`channel.join`),
  hears messages through an ordinary handler and answers with `channel.post
  as`. It has no model, no slot and no credential beyond its instance's; it
  never answers another bot. In the client all three are added from one
  menu, and typing `@` offers the agents and bots that can be asked.

## Identity

Accounts, sessions and credentials are local to an installation. Tokens are
random, shown once and stored hashed. Every route but sign-in requires one;
being able to reach the daemon proves nothing. (The old system relied on the
network for that, which is why it could not be exposed.) Scopes are `area` or
`area:verb`. Plans gate inviting people: free is one person.

A hosted identity service (organisations, sign-in through a provider,
billing) is no part of this application, and nothing here reaches one.
The seam for it is `Identity::sign_in_external`, which no route asks
yet: when such a service vouches for somebody, they are signed in to
the account that is that service's subject's, made the first time, and
the plan the service says is written on that account. The account is
found by who vouches and for whom, and by nothing else: the address a
statement carries finds nobody, so a service that names an address an
account here already has, the owner's included, is refused and changes
nothing of that account. Joining the two would take that account's own
say, which is not built; nor is anything that says which services are
believed, which is the route's to say when there is one.

A credential says what its bearer can and cannot do, and that is the
authority: nothing is allowed for being able to reach the daemon, for where
a request came from, or for what a file in a project is called. What
follows from holding to that:

- **A request is answered only when it was addressed to this daemon**
  (`crates/served/src/guard.rs`), before any route is looked at: asked at an
  address, at `localhost`, or under a name the daemon has been told is its
  own. A page somewhere else can give its own name an address of this
  machine; the browser then takes what it finds here for that page's own,
  and a name nobody vouched for is exactly what such a page would be using.
  Whatever is put in front of the daemon is asked under a name of its
  choosing, so its owner says that name once (`mw names add <name>`), and a
  blueprint that puts the daemon behind another address says it as it does
  (`system.answer_as`, which takes `admin`).
- **A request that changes something is refused when it says it comes from
  another origin.** A browser keeps a session for a machine and not for a
  port, so the owner's session goes with a request from any other program's
  page on the same machine. A browser also says where such a request comes
  from (`Origin`, `Sec-Fetch-Site`), and one that is not this daemon's own
  page is refused whatever came with it. What says nothing of where it
  comes from (the command line, an agent, a script) is judged by its
  credential, as before.
- **The owner is made by the factory's first start, before it answers
  anybody.** Called `Unnamed` until they say what they are called (nothing
  of the machine or of whoever runs it is taken for a name), with no email
  and no password,
  in a statement that writes it only where there is no account at all; the
  session it starts is the terminal's that started it. Nobody registers to
  begin: a browser is signed in by a link the command line asks for, good
  once and for a minute, kept by its hash in memory alone; asked by a
  person with all they hold (their session, or a key that holds every
  scope they do), or for the owner by whoever proves they can read the
  factory's data folder, which only the command line on this machine, run
  as whoever runs the factory, can (`people::opened`, `api::opened`). A
  node not told to make its owner (a test's own) has its first account
  registered from the machine itself, as every factory did before: no
  factory that is run is ever found with no owner.
- **A key holds no more than whoever makes it.** Told nothing, a new key
  holds what its maker does; anything asked for beyond that is refused by
  name. Making one needs no scope of its own, since nothing can be made
  that its maker could not already do.
- **A folder that is already there is its owner's to take up as a
  project** (`admin`), and never one that holds the factory's data or lies
  inside it: every credential the factory has is a file there, and a
  project's files are read by whoever may read projects.
- **A project made of a folder that was already there is not trusted**
  until the factory's owner says so (`mw project trust <name>`, or Trust
  this project… in the gear of its page, which Stop trusting… there takes
  back; an event either way, `project.trusted` and `project.untrusted`,
  which every page follows by reading the projects again, so that the mark
  that says it is not trusted goes and comes back with nothing read by
  hand). The gear offers either line only to whoever holds `admin`, which
  is what the route asks (`trusting` in `ui/src/trusting.ts`), behind a
  question that says what it does (`views/trusted.tsx`). A blueprint is run with the authority of whoever applies
  it, and a project's own files are searched first, so a repository that
  carried a file called `std/board` would otherwise decide what setting up
  its board does. While a project is not trusted, a file of its own with
  the name of a blueprint that ships (a standard one, or a package's) is
  passed over for the one that ships, and whoever checks or applies is
  told which file and how to have it used. Its blueprints under names of
  its own are untouched: applying one of those is something somebody
  chose. A project the factory made itself is trusted from the start
  (`crates/folders/src/library.rs`, offered as `api::library`).
- **Whoever follows what happens is told what its scopes let it read.**
  What is said in a channel is in the event that says so. Each event is
  described with the scope whose holder may be told of it, and the event
  stream and the list of recent events give a caller only those
  ([blueprints.md](blueprints.md), *Who is told of an event*).
- **Starting a service takes `shell` as well as `fleet:write`**: a service
  is a command that is run.
- **Nobody gets by a deputy what they do not hold.** Much of the factory
  is an instance somebody with authority applied, acting on what others
  write: a fleet on its tables, a stage on a vote, a round on a card. Each
  place where what a lesser credential may write would decide what such an
  instance does is held to what the writer holds:
  - *the texts it runs*: a project's file is run only as vouched for
    (*Instances and runs*);
  - *the rows it follows*: a table says what changing it takes
    (`takes` on `collection.ensure`), and a fleet's `agents` and `stages`
    say `agents:write`, so that writing a record is not making an agent or
    saying how much it may do;
  - *the events it believes*: saying an event takes `emit:<topic>` from a
    step as it does from outside, and a review counts the answers of its
    own seats, turns its own step asked for, and no other's;
  - *the agents it can ask*: giving an agent work takes holding what that
    agent holds (and `shell`, of one held to nothing), and making an agent
    that is held to nothing takes `shell`;
  - *what it does for whoever did something else*: a blueprint asks
    whether who an event says did it holds a scope now (`actor.holds`,
    `Identity::holder`), and a card moved with a key says the key moved
    it (`api::state::Said`), so that a card passed by hand is merged only
    for one who could have changed the project's files themselves; what
    is said on a record with a key says the key said it the same way, so
    that something marked public is told on only for one who holds what
    telling it takes, and so does a card added or changed with one;
  - *the devices it can reach*: sending a step to another device takes
    `fleet:write`.
- **A handler hears what whoever applied it may read.** A blueprint is not
  a way to be told what its applier's credential would not be: the
  conductor asks of each event what the routes ask (`Engine::hears`).
- **An agent starts and stops nothing unless its slot says so.** What an
  agent holds by default (`kernel::principal::AGENT_SCOPES`) is what its
  work is done with, and reads what is running without `blueprints:run`:
  whoever may apply may stop any instance, a fleet among them. A fleet's
  row gives an agent more or less (`holds`), never more than whoever
  applied the fleet has.
- **An agent never changes its own slot** (`crates/agents/src/own.rs`).
  What it may ask of the factory, what its program may reach on the
  device, the keys it is handed and what it signs in with are set by a
  person, or by an agent that holds what it gives, and never by the
  agent itself, even within what it holds: a scope that lets it change
  one thing (`shell`, a wall; `secrets:use`, a key named) would let it
  hand itself what it was never shown. Its own route refuses it
  (`ensure`, `give`, `move_to`, an invitation), and so does the row of a
  fleet's table that brings it up (a table whose rows give agents what
  they hold, `takes: each`), changed or taken away: the studio asks
  whoever keeps the agents whether the row is the one something running
  holds that agent for (`Collections::own_rows_with`). Stopping itself,
  and an external agent giving up its own credential, are no raise and
  are let be. Its brief says so (`allowed.rs`).
- **A browser's cookie alone is nobody's word** (`studio::Came`). A browser
  sends a cookie to every port of the machine it was set for, and with any
  link followed there, so whatever else listens on that machine is handed
  it. A page's session is therefore good only in the cookie and only with
  a key the page was given when its person signed in, which it keeps in
  storage that is its own by port and sends with what it asks; said as a
  token, or without the key, it is refused. What the page has its
  browser fetch by itself and only read (a file or a picture shown,
  something attached, the stream of events, a page of a library and a
  mockup's address opened in a tab of their own) cannot carry the key, and
  counts with what its address says beside the cookie (`by`): something
  the page was given with its key, made from the key and not the key, so
  that the cookie reads nothing alone, an address in a log is nobody's
  credential, and neither asks as the page asks (`Identity::shows`,
  `state::shown`). What `by` is, exactly: one value for a page for as
  long as its person stays signed in, the same in every address that
  page makes, and another when they sign in again. Whoever holds it and
  not the cookie can do nothing with it. Whoever holds it and the cookie
  reads what those kinds of address read (a project's files, what is
  attached, the stream of events, a library's pages, the way in to a
  mockup), as that person and for as long as the
  session lasts, and changes nothing. So an address with it in (a link
  somebody copied, a browser's history) is to be kept as a file's
  address is, and a file or an attachment is sent with
  `Referrer-Policy: no-referrer`, so that nothing shown from there says
  its own address to what it leads to. It is compared, as the page's key
  is, digest against digest and to the end either way. A page of a
  library says `by` again in every link of it that leads somewhere on the
  daemon (`addressed::saying`), so that its reader goes on from page to
  page as they came; a library written out to a folder says nothing of
  the kind. Such a page or a mockup's address opened where it says
  nothing beside the cookie (typed, kept) is sent through the client's
  page (`api::through`, `ui/src/senton.ts`), which adds it as things now
  stand and sends the browser on, after signing in whoever is not. The
  address bar then shows it. A mockup's way in is no credential and asks
  for none. The client's page fetches and frames
  nothing that is not this daemon's (`served::client::CLIENT_POLICY`), so a
  picture in a document somebody else wrote is not a request its reader's
  browser makes.
- **An invitation to register works once and for a while**
  (`Identity::invite_for`): for the email it names, where it names one; a
  week unless whoever makes it says (`expires`), thirty days at the most;
  read from the clock when it is used, so nothing looks for the ones that
  have run out.
- **An agent the factory does not start comes by its credential with
  an invitation**, as a device comes by its place in a fleet
  (`agents::external`, `people::enrolled`). An agent the factory starts
  is handed its credential in a file the factory writes; one somebody
  runs for themselves (an *external* agent) is somewhere the factory
  writes nothing. So whoever may add agents makes an invitation for it
  (`mw agent invite <name>`): they give it nothing they do not hold,
  the invitation works once, for a day unless they say and fourteen at
  the most, and the factory keeps the hash of its secret and never the
  secret, in the small durable state under a name no copy keeps.
  Whoever holds the invitation hands it over at a route that takes no
  session (`POST /api/agent/join`, `mw agent join`), since the
  invitation is what lets its bearer in, and is answered the agent's
  token, once. A wrong invitation, a used one, one that has stopped
  working and one whose agent was stopped are answered alike. A guess
  is one in 2^256, so refusals are not counted against where they came
  from, as a hub's door counts them.
  The token is an agent's credential like any: it names the slot, and
  what it does is said to be `agent:<name>`'s doing. The factory keeps
  only its hash; the agent's side keeps it in a file of its user's
  settings, for that agent and that address, apart from any person's
  session there (`mw --as <name>`). What it holds is the slot's scopes,
  changed where any agent's is, and since the factory cannot write the
  token again the credential it has is given the new list where it is
  (`Identity::rescope`): its bearer holds that from its next request.
  Stopping the agent, or taking it away, withdraws the credential and
  whatever invitation waits; the agent gives it up itself with `mw
  agent leave`; and an invitation made again, once used, replaces it.
  Nothing is started for such an agent and nothing gives it work (the
  list of what leaves it out is in `crates/agents/src/external.rs`).
  In a channel kept to its members it is as a person is: it is
  somebody's program outside the factory, as a person with a key is,
  so nothing of what holds a started agent's program there applies (a
  wall, a home, a turn in hand), and its own credential reads the
  channel it is in whenever it asks. Which agents are such is
  remembered by whoever keeps agents and asked where whether a turn is
  in hand is (`Channels::outside_with`).
- **A device taken out of a fleet stops what it was doing for it**
  (`Gateway::remove`). On the hub the steps that waited on it end at
  once, saying so. The device is told by the hub itself: its session is
  over, and when it next asks to be let in the hub's sealed answer says
  its key is none of the fleet's (`Welcome::unknown`), at which it stops
  the steps it was running for the hub, keeps no answer for them,
  forgets the hub as it does when it leaves, and says so once
  (`device.unjoined`). What the hub's steps had started there earlier
  (`blueprint.apply` sent to it) is not stopped unasked, since that is
  the device's owner's to say: it is named, there and in what `mw device
  list` prints, with how to stop it. After that
  it asks that hub nothing more, now or after a restart, and whoever
  looks at its devices is told what happened and how to join again
  until it does. Nothing unsealed can tell it any of that. A device that
  never asks again finishes what it had begun, and nobody takes it.
- **A session that is ended, and a key that is withdrawn, are told
  nothing more.** The stream of events is answered for as long as what
  opened it would still open it: each time something that lets somebody
  in is withdrawn (`Identity::withdrawn`) every open stream asks again
  who its bearer is, and one whose bearer is nobody ends
  (`state::Bearing`). Somebody signs out of every session they have,
  wherever each was begun, with `mw logout --everywhere`
  (`identity.signed_out`); the keys they made are not sessions, and go
  on until each is withdrawn.
  A stream listens for withdrawals before it asks who its bearer is,
  and looks at them before it hands on the next event, so that none is
  missed in between and nothing is told after one.
- **A key can be withdrawn** (`mw key revoke`, by whoever made it or
  manages the installation; a key may withdraw another only where it
  holds `admin`), and making or withdrawing one is an event. A key made
  with a key keeps which (`credentials.made_with`), and is withdrawn with
  it, all the way down (`people::keys`): it holds no more than what made
  it, and would otherwise be a way to go on. Who somebody is with a key
  says which key it is (`Principal::key`), and what an instance runs
  with is kept as that: so withdrawing a key stops each instance it, or
  a key withdrawn with it, applied that is still alive
  (`Engine::applied_with`, a look over what is alive; `stop_withdrawn`,
  said as `withdrawn` of `instance.stopped`), as stopping it by hand
  would: what it started, its services and an agent's turn it asked for
  go with it, and what it did stays done (a card where it is, a merge
  made, what was said). It is withdrawn first and what it applied
  stopped after, so that nothing more is applied with it meanwhile.
  Asked to leave it (`--leave`, said as `leave` of
  `identity.key.revoked`), it goes on as it was applied until it is
  stopped. What withdrawing would come to is asked first, changing
  nothing (`GET /api/credentials/<id>/withdrawing`), and the client asks
  with it before it withdraws. That is of this device, and of what was
  applied since a key was kept with what it applied: an instance applied
  before then names no key, and what a step of the key's applied on
  another device lives in that device's own runner, where it is done as
  nobody's key (`Member::within`) and is listed there. An apply the key
  had begun as it was withdrawn whose instance is written after the
  look goes on. A key made with a key before which was kept is
  withdrawn by itself.
  **A key can be rolled** (`mw key roll`): it is given another secret,
  and the one it had is refused from then, said as `identity.key.rolled`
  in the write that changes it, with whatever answers somebody it let in
  told to ask again. It is the same key by its id, holding what it held,
  so what it applied goes on. A key may roll itself, which is how a key
  is changed on a schedule; revoking is for a key that is to be dead.

- **A channel may be kept to its members**, and is then read by nobody
  else by any way of reading (`talk::kept`). It is a flag of the
  channel's row, with what the channel is called beside it: a kept
  channel's name, among the names every channel shares, is its own id,
  so that nobody who makes a channel is told that one of that name is
  there, and two kept channels may be called the same. Whether somebody
  reads one is one expression: the channels' own scope and being in it
  (a key somebody made is its maker's and holds what it was given, so
  it is a member through its maker only where the key holds that
  scope), or, for now, the scope and what the factory's owner holds,
  who reads everything. Everything asks that expression. A route, a
  blueprint's action, the bot and a search ask it of the channel
  through a door that gives leave to read that one channel (a
  `Reader`), and nothing of a channel (itself with who is in it, a
  message, something attached) is handed out by `talk` without such
  leave: a read that asks nothing does not compile. Whoever does not
  read a kept channel is answered, by every route, in the words a
  channel that is not there is answered in, a message and a file of
  one the same; a list of channels, of a workspace's channels and of
  what a search found leaves it out. What the daemon reads for its own
  work (the search index, which agent a message is for, an answer said
  where it was asked) says so with why, and each such place is on a
  list a test holds the source to. And every event of a kept channel
  takes being a member (`member:<its id>`), which no scope answers
  for: the journal asks, as it hands each event to a stream, a handler
  or a count, who is in the channel then, of a memory `talk` keeps of
  the kept channels' members (filled from the tables at a start, never
  the only place anything is kept, and changed before the write for
  somebody leaving and after it for somebody joining, so that between
  the two it says less than the tables and never more). An event that
  says somebody left, or that the channel was taken away, names who it
  is also for, since they are no longer members when it reaches them;
  so does one that says somebody joined, whom the memory has a moment
  after the write. So a blueprint hears a kept channel only while
  whoever applied it reads it, and a run something said there set off
  is shown as any run set off by something kept is (above).
  **An agent in a kept channel is held four ways** (`agents::kept`),
  since anybody who may ask it elsewhere could otherwise have it read
  the channel out. *Its key opens the channel only while a turn asked
  there is in hand.* A turn is of a kept channel where its
  conversation is that channel's (asked there, answering there, or
  going on with what was said there), which is written on the turn as
  it is queued; while one is in hand, from just before the write that
  says it began until just before the write that says it ended,
  `agents` remembers it, and that memory is the second half of the one
  expression for an agent, asked by `talk` as synchronously as who is
  in a channel (`Channels::asked_with`). It is filled from the turns'
  rows at a start. An agent does
  one thing at a time, so no turn of a kept channel is in hand while
  its program is at work on something asked elsewhere. What the owner
  holds reads nothing for an agent, and an agent is not told of a kept
  channel by being named in what an event takes. *What is said of the
  turn is the channel's members'*: that it was queued, began and ended
  and what its program prints take being a member, a step has an agent
  go on with a channel's conversation only as somebody who reads that
  channel, and whoever does not read it is shown of the turn when it
  was, how long it took and what it cost. *Its program has a home of
  its own for each kept channel* behind its wall (*Walls*), so that
  what a program keeps of what it was asked there is found by a turn
  of that channel and by no other; its sign-in is in every home, as a
  value it is handed or as what its package says is its sign-in, shown
  where it is. *An agent that cannot be held so is in no kept
  channel*: one with no wall, and one whose program keeps its sign-in
  where it keeps what it is asked, since that place is shown to every
  agent signed in the same way (it signs in as whoever runs the daemon
  and its package says no `wall.signin`, the daemon's environment
  names a folder for it, or it may be handed a folder the factory
  holds). It is refused where it would be brought in, in a sentence
  that says the ways out, and one that comes to be so while it is in
  one (its wall, what it signs in with, its package or how the daemon
  was started having changed) leaves it, which the factory says there.
  A channel may also say that its agents are asked nowhere else
  (`channels.alone`), for what is most closely held: an agent in such
  a one is given no work anywhere but there. What it kept of a channel
  goes when it leaves it, when the channel is taken away and with the
  agent.
  **Its door is its members'** (`talk::door`, `agents::door`; `mw help
  channel`; Private in the client). A kept channel is made in one
  write with its maker, the people it was asked with and what it
  belongs to, and each agent it was asked with is brought in after,
  having been asked first whether it could be. An open channel is kept
  by somebody who is in it: what it was called is its title from
  there, its name is its own id and free for the next channel, and
  that it was kept is said to whoever may read channels, who stop
  showing it unless they are in it; the memory that answers for events
  has its members from before the write, so that nothing said of it
  after is told to anybody else, and a message is said as its channel
  is in the write that says it. It is refused while an agent in it
  could be in no kept channel, in a sentence that says which and the
  ways out, and while a turn of the channel waits or is in hand; the
  turns that have ended are written as the channel's, so that their
  words are its members', and what each agent's program remembered of
  the conversations held there is forgotten, since its next turn there
  begins in the home it has for that channel. Somebody is added by
  somebody who is in it, and nobody adds themselves; somebody leaves by
  asking, but for the last person in it; an agent is taken out by
  somebody in it who may change agents. Whether its agents are asked
  nowhere else is said afterwards too, by anybody in it, and refused
  while one of them is in another kept channel or has something asked
  elsewhere that is not over. And **only the factory's owner opens one
  again**: it takes a name among those channels share, anybody who may
  read channels reads everything said in it from there, and what each
  agent kept of it as a kept channel goes. A blueprint asks for one
  with `channel.ensure` (`kept: true`), which is the same done twice:
  an open channel is found by its name, and a kept one as the kept
  channel of the same project that is called so and has whoever
  applied the text in it, so that the one its step made, or one
  somebody kept since, is found again; it changes nothing of a channel
  that is there, and is refused a kept one where an open channel has
  the name. Every
  channel reads as it always did, but for two
  things put right for all of them: a thread is read only in the
  channel it is in, and something is said under, or in answer to, only
  a message of the channel it is said in.

- **Everybody has an inbox, which only they read** (`talk::inbox`). A
  line of one is a row of the factory's own, whatever it is about, for
  an owner who is a person (`user:<account>`) or an agent
  (`agent:<name>`): its kind, what it is about, what opens it (a
  channel), who it is from, and when it was read and put away. It
  keeps no words of a message, which are read from the message as the
  owner may read it when they look, so the box is no second place where
  what was said is kept. Everything that fills a box goes through one
  door (`Inbox::tell`, or `tell_in` inside a write of the caller's),
  which writes the row and says `inbox.told` as one write and is the
  same done twice: a telling is known by what said it, and told again
  by something else the line there is is raised (unread again, back
  from where it was put away, counted) and is never a second row. The
  first thing that fills one is a message that names a person with `@`
  (`talk::inboxed`): in the write that keeps the message, each account
  the word names (`people::handle`, one rule the client's own is held
  to by a table both read) who is not its author and reads its channel
  by the one question every reading of a channel asks is told. An
  agent the factory does not start is told by whoever hears what is
  said for agents (`Agents::tell_named`), through the same door, where
  it may read the channel; one the factory starts is given a turn and
  no line. Whose a box is, is never taken from a request: it is the box
  of whoever asks (a key's is its maker's, where the key itself holds
  `inbox:read`), so a line of somebody else's answers as one that is
  not there, and what is said of a box takes being its owner
  (`member:inbox:<owner>`), which no scope answers for, the factory's
  owner's and `*` included: `talk` answers it where it answers who is
  in a kept channel. A line of something said in a channel its owner
  does not read as things stand is passed over (not listed, not
  counted, not found by its id), and is as it was when they read the
  channel again: nothing of a box says that something is there which
  its owner may not read. How many wait unread is said with each thing
  that changes a box, counted in that write from what the owner holds
  then (`people::held`). A line goes with the channel it opens, in the
  write that takes the channel away, and a box with an agent that is
  taken away for good. One line is read by its id for a page of its own
  (`GET /api/inbox/<id>`), which is how the client shows what a
  blueprint told, and a line that was read is made to wait unread again
  by its owner (`inbox.unread`); both answer somebody else as for a line
  that is not there.
- **A blueprint tells somebody through that door, and a fleet tells of
  what waits for a person** (`talk::telling`, the action `inbox.tell`).
  A step names whom: whoever applied its instance, an actor, a handle,
  or a list; each is found in the write that tells (an account by the
  rule a word after `@` is read by, an agent by one of that name
  holding anything), so that everybody is told or, where a word names
  nobody, nobody is. The line is of a third kind (`told`), from the
  factory's bot, and keeps the blueprint's own words: it is no line of
  a message, and is never passed over. Telling takes changing a box of
  whoever applied the text, and nothing is read for whoever is told:
  what the line is about and what opens it are ids, opened by the
  routes there are. It keeps which thing done under which step of
  which run said it, as a message a step posts does, so the step done
  again tells nothing twice; and a line is one for its owner, its kind,
  what it is about and why it is there (a step of the inbox's tables:
  the one index of lines was of the first three), so another telling
  of the same thing for the same reason raises it, and one about
  nothing is a line each time. Somebody who holds no reading of a box
  has none and is passed over, which the step is told: an agent the
  factory starts. A fleet's telling is a text it attaches (`std/told`,
  by `uses`), which hears the project's questions, leavings and plans
  as events (every leaving of a card is one, `card.left` with why, said
  by its stage or, for a round's, by the run that asked for the round;
  a card's plan says whether it waits) and, being attached, what fails in the fleet as the fleet
  itself hears it (`scope: target`), with nothing said by the fleet for
  it. Whom is rows of a table it declares beside the board (`tells`),
  read at each thing that happens, and which rows a thing is told to,
  with its sentence, is a block (`deciding::told`). Its handlers take
  turns, so no two tellings of one thing cross; a telling that cannot
  be made fails its run, which the fleet reports as any failure under
  it, and that failure is not itself told, since what is attached does
  not hear its own tree.

- **A conversation is a channel kept to its members that says it is
  one** (`talk::conversed`; `channels.conversation`, a step added last).
  It is mail by where it is listed and by nothing built a second time:
  its subject is what the channel is called, whoever wrote it and
  whoever it is to are its members, and threads, attachments, search,
  who reads it and an agent's turn are the kept channel's. It is
  written as one write (`Channels::converse`, asked by
  `Agents::converse`, which says first that each agent may be in a kept
  channel): the channel's row, who is in it, its first message and the
  line each of those it is to has of it, with what is said of each or
  none of it. The memory that answers who is told an event has its
  members from before that write, since its first message is said in
  it and names nobody: named, somebody who left later would go on
  reading it back. Each message somebody else says raises the one line
  a member has (`Kind::Message`, about the channel, told by the
  message), in the write that keeps the message: a person's, and that
  of an agent the factory does not start, whatever they hold just then,
  the line being passed over while its owner does not read the
  conversation; an agent the factory starts is given a turn and no
  line, and somebody named there who is in it is told by that line and
  no other. It is in no list of channels: the route that lists them
  hands it to its members with its mark, since their page opens it, and
  what lists channels by id for a workspace leaves it out
  (`Channels::unlisted_for`), as the command line and the side panel
  do. One of its members opens it as a channel
  (`Channels::open_as_channel`): the mark comes off in one write with
  what is said of it (`channel.kept`, to its members), after it was put
  where it is to belong, and it is a kept channel like any from then;
  the factory's owner's door opens no conversation to everybody.

- **The themes somebody makes for the client are kept with their
  account** (`people::themes`), with which theme they chose, so that
  both are there in any browser they sign in to: the one thing of how
  somebody arranges the client that is not the browser's. A theme is
  a name and eight colours, with eight more for a light version where
  it says one, worn where the system asks for light as the themes that
  ship are; each version is checked on its own where it is written, by
  the rule the client draws by, held to one table of cases. Whose themes is never
  taken from a request: they are the asker's (a key's are its
  maker's, where the key holds `themes:read`), and what is said of one
  that is somebody's alone takes being them (`member:own:<actor>`),
  which the journal answers itself, for that actor and nobody else,
  whatever they hold. Whoever may manage the factory offers one of
  their own to everybody, and withdraws one; what is said of an offered
  one is for whoever may read themes. What a browser kept of themes
  before is taken up once, by the page, and taken out of the browser.
  A theme is saved as a small file and read back in by the browser
  alone, the file being sent as a theme of the reader's own through the
  route that makes one, which checks it as it checks any.
  A theme may also say how the client looks beyond its colours: its
  letters, from stacks a system already has (nothing is fetched), the
  size of text, how close things stand and how round corners are, each
  a word of a few, checked where it is written as the colours are, and
  worn as one measure each that the style sheets go by.

- **How somebody arranged the side panel is kept with their account**
  (`people::arranged`), the same way: their groups of channels (each
  an id, a name and its channels), the channels they pinned to the top
  and what they pinned in Resources (each by the address that opens
  it), so that all of it follows them to every window and browser. It
  is text and nothing more, the asker's own (`arranged:read`,
  `arranged:write`), checked where it is written, said whole to them
  alone (`arranged.kept`, `member:own:<actor>`), kept in a copy with
  the accounts and moved with no project. **The daemon stores it and
  acts on none of it**: nothing reads a group or a pin to decide who
  may read a channel or what a workspace lists, and an id that names
  something gone is kept, since it may come back from the Trash, and is
  passed over by the page. What is folded, the open tabs and what was
  lately opened stay each browser's. What a browser kept before is
  taken up once, where the account keeps nothing, and taken out of the
  browser.

- **How a time is written for somebody is kept with their account**
  (`people::clocked`), the same way: the zone, by its name in the zones'
  own list, and a clock of twelve hours or twenty-four (`12h`, `24h`;
  twelve where nothing is said). It is the asker's own (`clock:read`,
  `clock:write`), checked where it is written, said whole to them alone
  (`clock.kept`, `member:own:<actor>`), kept in a copy with the accounts
  and moved with no project. **The daemon writes no time in a zone,
  and knows a zone only as the time of day a handler fires at says
  one** (`at:`, `reader::at`, still `blueprint::at`, by `jiff`, which follows the zone's
  daylight saving; a time with no zone is refused): a person's zone is text to it, held where it is written to the
  form a zone's name has (one line, not long, letters, digits, `_`, `-`
  and `+` in parts between slashes, each part beginning with a letter:
  `clocked::checked`, held case for case with the client's
  `clockUnfit` to `ui/src/clockcases.json`), and looked up nowhere. The
  client holds a zone to those its browser knows
  (`Intl.supportedValuesOf`) before it sends it, and writes every time
  it draws through one file (`ui/src/clock.ts`: `timeIn`, `whenIn`,
  which `clock` and `when` of `util.ts` are, in the setting the page
  wears), with the zone's short name as `Intl` has it on that date and
  nothing written out. The first time it is read with nothing kept, the
  zone the browser says it is in is taken up (`/clock/first`); a window
  run under another system may say another zone, and Settings is where
  it is put right. A blueprint that writes a time in its reader's zone
  (a block over a list of zones) is later work.

- **How what somebody reads looks is kept with their account**
  (`people::appearance`), the same way: the size of its text (a theme
  look's sizes), its width (`standard` or `wide`), whether the client
  is light, dark or as the system asks (`colour`), whether a long
  article's contents are shown or hidden (`contents`), and whether an
  article and a library's document are read in sans or serif letters
  (`typeface`). It is how the client
  looks for them, so it takes what their themes take (`themes:read`,
  `themes:write`), is said whole to them alone (`appearance.kept`), and
  is kept in a copy with the accounts. The daemon acts on none of it.

What is not held, and is said so that nobody relies on it:

- **An inbox holds three kinds of line, and an agent the factory starts
  has none.** Somebody named in a channel, a conversation, and what a
  blueprint told are all that fill a box so far. An agent the factory
  starts holds no `inbox`: adding it to what
  an agent usually holds changes what a row of a fleet's `agents` that
  says nothing gives (`std/fleet`, `unsaid`), so it is for whatever
  gives an agent its box to do, with that text's next version. An
  account called what an agent or the factory's bot is called is told
  when that one is spoken to: a word after `@` is not of one kind.

- **A conversation is held as a kept channel is, and an agent from
  outside that is in one reads it where its program runs.** The
  factory's owner reads every one, for now. An agent the factory does
  not start is of a kept channel as a person is, so what is said there
  is as closely held as that agent's program, which is behind no wall
  this factory holds. Whether a message to an agent the factory starts
  is a turn at once or waits is not a setting: it is given a turn as in
  any channel. And nothing caps how many conversations an agent writes.

- **An agent's program is held in by its wall and by nothing else**
  (*Walls*, below). In everything it asks of the daemon an agent is held
  to its slot's scopes. What its program does on the machine it does as
  whoever runs the daemon, behind a wall where the system holds one
  (Linux, with bubblewrap) and its slot has not been told to go without.
  With no wall (`--no-wall`, a slot from before there were walls, a
  system that holds none) it can read what that person can read, other
  slots' credentials and the sign-ins the factory holds among it. Behind
  one it still reaches the network, and reads the sign-in it was shown.
  A command a blueprint runs, and a service it hosts, have no wall.
- **A row of agents is held to what its writer holds by what it gives
  (`holds`), and not by what its other cells let its program at.** Taking
  a wall away, showing a program more, handing it a key, having it run
  commands unasked or go on without the daemon each take what that takes
  of whoever writes that cell. Once such a cell is written, somebody who
  may change agents and holds less can still change the row's other
  cells: what the agent is told, how many there are of it. By the
  agent's own route they are refused.
- **What a stage has the fleet itself do is asked of whoever writes its
  row only through what the fleet's agents hold.** A row of `stages` is
  added or changed by whoever could write each row of `agents` as it
  stands (`steers`), so no agent that holds more than they do is given
  work on their say. But a stage also moves a card on, merges its
  branch and starts a blueprint its row names, as whoever applied the
  fleet: in a fleet whose agents were all given less than moving a card
  or changing the project's files takes, somebody who may change agents
  and may do neither can still arrange for both.
- **A device and its hub prove a key to each other, and what they say is
  sealed** (*More than one device*). What is not held: a member does what
  its hub sends, within what it was joined to allow; and whoever holds an
  invitation before it is used can be the device it lets in.
- **A channel kept to its members is held where the daemon hands
  something to somebody, and not past that.** Not held, each for
  somebody who is not in it:
  - *What the packages say of their programs' sign-ins* is written
    from the names of what each program keeps, and none has been run
    in a kept channel's home. A sign-in that is one file, which is how
    two of the three keep theirs, is written where it is from such a
    home and cannot have another file put in its place: a program that
    renews it that way fails to there, and its turns in the channel
    fail as those of one that is not signed in until it has renewed it
    in a turn asked anywhere else. A lock a program keeps beside its
    sign-in is each home's own.
  - *An agent's work and its program's reach.* What its program writes
    where it works is its project's; behind its wall it reaches the
    network with what it read; and nobody who does not read the channel
    is told, as it is said, that the agent is at work there.
  - *What a member's own blueprint makes of what it heard.* It hears a
    kept channel as its applier, a member, and what it then says itself
    (an `emit` step, what it posts elsewhere, what its instance keeps
    and starts) is said as anything of a blueprint's is, and so is
    whatever the runner says of its run that is not said as taking what
    the event took. A handler whose applier leaves the channel stops
    hearing it. And what names a kept channel by its id where a
    blueprint was applied (a param called `channel`) is read with the
    instance.
  - *The machine.* The database, the search index and the attached
    files are files; a copy of the factory holds kept channels, still
    kept, for whoever starts a factory from it and owns that one. A
    page that was attached is kept by a browser that showed it for an
    hour.
  - *What a search says besides what it found*: how many documents the
    index holds, and fewer hits than were asked for where some of the
    best were left out.
  - *What was said of a channel while it was open* is told as it was
    said, to whoever reads the journal from an earlier place: keeping
    it writes none of those events again. That it was kept is itself
    told to whoever may read channels, with what it was called until
    then. An agent that was asked there before kept what its program
    keeps of that in the home it has for everything else. And a
    blueprint that names it by the name it had finds no channel: a
    kept one is named by its id.
  - *The door's own limits.* A channel is not kept, and not opened
    again, while one of its agents is being asked something there;
    keeping one reads every turn there is, once; nobody takes a person
    out of one but that person, and its last person does not leave it;
    and a blueprint whose applier has left its kept channel is refused
    there, and is handed another of the same name by `channel.ensure`.
  - *A member* can repeat what they read, and whoever holds what the
    owner holds reads every kept channel, but for an agent.
- **A scope is for the whole factory**, not for one project: an agent
  whose slot is in one project can ask the daemon about another.
- **Whoever may write a project's files writes what is run there.** Code
  an agent wrote is run by whatever runs the project's code: a stage that
  tests, a build, a person. That is what the work is; a review is the
  check on it, and a place to run in the wall.
- **`blueprints:read` reads what instances did, and what they read only
  as far as its holder may be told what set each run off.** A run keeps
  the event that started it and what each step gave, and is handed
  whole to whoever may be told that event; anybody else who may read
  runs is handed it without what the event carried, what its steps
  gave and why any failed (`crates/api/tests/suite/told.rs`). They
  still read that it was set off, when, by an event of which topic and
  how it ended; and what an instance keeps, and why it failed, is read
  by whoever may read instances.
- **An agent told something acts on it.** Whoever may speak in a channel
  an agent listens in, or write a card it is given, has put words in front
  of it; what it then does it does with what its own slot holds.
- **A store service answers anyone on the machine** ([stores.md](stores.md)).
- **Other people on the same machine** can reach the daemon as its owner
  can before there is an owner, and can read its files if the data folder
  lets them.

## More than one device

One device is the hub and runs the blueprints; members link to it and run the
steps it sends (`device: <name>`).

- **The member calls the hub**, over HTTP. Only the hub needs to be
  reachable, and no networking product is part of this one: a LAN, a VPN
  or a tunnel is the owner's choice, made outside, and need not be one
  anybody trusts.
- **Each installation has a secret of its own**, made the first time it
  starts and kept in its data folder, readable by its owner alone
  (`link::own`). It is a seed: a key is made from it for one purpose,
  named in the making, and today there is one, the key for links. It is
  in no event, log or copy: a factory started from a snapshot makes its
  own and is another installation. The file is whole and on the disk
  before it has its name, and is never written over one that is there, so
  a start cut short leaves no empty file and two starts at once come to
  the one secret; one that others on the machine may read is not used,
  and the daemon says how to put that right and does not start. The short
  name of the key's public
  half is shown wherever a device is (`mw status`, `mw device list`, the
  client's Devices), so that two people can compare what each sees.
- **A link begins with a handshake, and everything after it is sealed**
  (`link::sealed`; the secret, the keys, the handshake and the frames are
  a crate of their own, `crates/link`, which stands on nothing else here,
  so that anything else that has to prove a key to an installation is
  built on the same handshake and not on a second writing of it). The
  handshake is Noise's (`IK`, X25519,
  ChaCha20-Poly1305, BLAKE2s), through the `snow` crate; no cipher or
  proof here is this program's own. The member must hold the hub's public
  key beforehand and is refused by the mathematics if what answers does
  not hold the private half: it says so, once, and does nothing that is
  asked of it. The hub learns the member's key from the first message,
  where only the hub can read it, and finds the member by it. What is said
  after that is sealed frames, each numbered under a counter that only
  goes forward, every record of one saying (sealed with it) which frame it
  is of: one changed, said twice, cut up or of another session is refused,
  and the link is made again with a new handshake. What carries the
  frames (HTTP here) can read nothing and change nothing.
- **A first message said before gets nobody anywhere.** Noise's first
  message can be sent again by whoever saw it cross. Each carries a number
  that only goes up for its device (kept on the device's disk before it is
  used, and never below the time); the hub keeps the highest it has taken
  from each device and turns away anything not above it before a session
  is made or anything is written down. A device keeps one such count for
  each hub, by the hub's key, and lets it go when it leaves that hub; a
  hub that tells it to go on from a number no clock will reach is not
  taken at its word. And a session counts for nothing until its device
  has said something in it: one nothing is said in is over in a minute,
  and when the device speaks in a later one.
- **What is not sealed steers nothing.** Until what answers has proved the
  hub's key, a member takes nothing from it: it reads no more of an
  answer than a hub would send, follows it nowhere it is sent on to, gives
  the asking for its link no longer than a link may be silent, and keeps a
  line of whatever was said, for the person who has to find out what is
  wrong, marked wherever it is shown as words that nothing proved. Told in
  the clear to stay away, it waits a minute at most, and never less than
  it would have waited anyway.
- **A device is let in by an invitation** (`gateway::door`): one string,
  made on the hub for one name, holding the hub's public key, where the
  hub is reached, and a secret that works once and for a while (a day
  unless said, never more than fourteen). The hub keeps a hash of the
  secret. The device hands the secret over inside the handshake's first
  message, and the key it proved there is the key its record is given:
  from then on **the credential of a device is the key it proves**, found
  in a record that says what it may do. There is nothing else a device
  can present, and no route that takes a device's word in the clear.
  Removed at the hub, a device's sessions end and its key opens nothing:
  letting a device in, giving it a session and taking it out are done one
  at a time, so that one taken out as it shakes hands is left with none.
  A device let in under a new name has left its old one, link and all. An
  invitation that was used is taken again from the device it let in, for
  the case that the hub's answer never reached it, and from no other. An
  invitation is not signed: nothing would check a signature, since it is
  redeemed at the hub that made it.
- **Who is put off, and who never is.** What is counted against an address
  is whatever was refused from it: what could not be opened at all, a key
  the hub does not know, an invitation that does not work, a first message
  that was said before. Past five in a quarter of an hour, nothing of that
  kind from there is looked at for the rest of it. Two things are never
  counted and never put off, wherever they come from: a device that proves
  a key that is a member's, in a message it has not said before; and
  whoever brings an invitation the hub holds, whose secret cannot be
  guessed. Behind a proxy in front of the hub, a tunnel or one router,
  every device has the same address, which is the address of whatever is
  nearest the hub and not the device's own, and what one stranger sends
  from there must keep neither the members nor the invited out. A refusal
  is written down once for an address in that time, and, where there are
  more addresses than are remembered, once in a while all the same.
- **A link is expected to drop.** A step sent to a member is remembered by
  the hub until it is answered; a member keeps its answer until the hub has
  taken it; a command is run once however often it arrives. A member retries
  with a growing pause, or the pause the hub asks for.
- **A member that is gone fails the step** after a minute, so the blueprint's
  own retry and failure handling decide what happens next.
- **A member decides what its hub may do on it**, and checks that itself.
  What the hub sends is done there as no more than both what the hub said
  to do it as and what the member allows: for the action, and for
  everything that comes of it on the member (a blueprint it applies, and
  that blueprint's steps). The environment a hub sends with a command
  reaches a program there only where the hub may run commands there; left
  out, the step's log says so. A hub that has been taken over can ask for
  anything; it gets what the member agreed to when it joined.
- **What is too much to say at once fails its step.** A frame holds no more
  than 1.9 MB. A command larger than that fails the step that sent it, and
  an answer larger than that comes back as the step's failure, each
  saying so: neither is sent again and again. Answers that are too much
  only together go a few at a time, and a line of a log that is too long
  is cut.
- Hosting on another device is applying a blueprint there
  (`blueprint.apply`), so what runs there has an owner there.

## What leaves the machine

Nothing, by itself. Accounts, channels, the board, blueprints, search,
measurements and logs live in the data directory and the project folders;
the daemon listens on this machine only unless told otherwise; the client is
served by the daemon and loads nothing from anywhere else. There is no
telemetry, no crash reporting and no update check, and no address of ours in
the code.

Data leaves where the person running it points it, and only there:

- **An agent's model.** The product sends nothing, but the program an agent
  runs on does: with the `claude` adapter, what the agent is asked and what
  it reads go to that service under the user's own account. A local model is
  an adapter like any other (`--command name=program`).
- **A hub**, when a device is joined to one.
- **A blueprint**, which runs the commands it is written to run.

This is checked, not promised: the `offline` gate runs the daemon's and the
command line's tests in a place with no network, where anything sent towards
the outside is counted, and fails on the first packet. That covers every
library linked in as well as the code here. It counts on Linux, where a
command can be given a network of its own; elsewhere, and on a machine that
will not make such a place, it runs the same tests with the network there
and says that it could not count.

When the hosted sign-in for paid plans exists it will be the first thing
here that talks to us, and it must stay something a free installation never
does.

## Where things run

- **In the daemon's process**: the conductor, the journal, the router, health
  checks, search indexing, the API. These restart with the daemon and are
  written so that restarting is cheap and loses nothing.
- **In their own processes**: everything a blueprint starts (commands,
  services) and every agent's program. What a blueprint starts outlives
  the daemon. An agent's program, and the program of a package's action,
  do not: each is read through a pipe the daemon holds, so it is stopped
  with the daemon, and its turn or step is done again by the next one.
- **The supervisor** (`mw up`) is a few dozen lines that start the daemon and
  start it again if it dies. This is the "fleet slot": the smallest possible
  thing that holds the conductor, so that the conductor itself can be
  replaced, or can crash, without the fleet noticing. A factory has one,
  as it has one daemon, and its data folder says who they are: a second
  `mw up` for a factory whose daemon is up says where it is and how it is
  stopped, and ends (with 3, as a daemon that was refused does); and a
  daemon that was refused because the factory is being served already,
  or because its address is something else's, did not die and is not
  started again.

## Walls

Everything the daemon starts runs as whoever runs the daemon. An agent's
scopes say what it may ask of the factory, and they are held at every
route; but its program is a process on the device, and with nothing around
it, it could read what that person can: the session they are signed in
with, the factory's own secret, every other agent's credential, their keys.
Asking would then be one way among several. **A wall is what makes asking
the only way.**

An agent's program is started behind one unless its slot says otherwise
(`Slot.wall`; `crates/agents/src/walled.rs`). The wall is held by a program
of the system's, bubblewrap's `bwrap` on Linux, which starts the command in
a view of the files made for it (`hosting::wall`). Nothing here is a
sandbox of its own making: this says what to show, and the system does the
showing.

| Shown | How | Why |
|---|---|---|
| The system (`/usr`, `/etc` and the like) and the folders on the path | To read | So that what is installed runs behind a wall as it does in front of it. A folder on the path that is another system's (`/mnt`) is left out. |
| Where it works | To change; to read for an agent held to `read` | Its project, or the folder of its own a turn names (a card's worktree), with the rest of the project to read and the other cards' folders (`.lanes`) to read. |
| A repository's own folder (`.git`) | As the slot's `git` says (`agents::repository`, over `hosting::repository`) | `read`, the usual: it is there to read, and `std/lane-round` commits what the agent changed. `commit`: the agent commits its own work; in a card's worktree that is the worktree's own part, new objects, and the branch it is on with those named beside it, so the project's own branch is not its to move. `write`: all of it. `off`: it is not there. Whichever, what git would run or go by (its settings, its hooks, where a worktree's repository is) is shown as it is and cannot be changed: whoever next asks git for something runs it, and that is the daemon, with no wall. Where a project is no repository, an empty folder is put there that cannot be written, so that none can be made. |
| A home of its own (`<data>/slots/<name>/home`; for a turn of a channel kept to its members, `<data>/slots/<name>/kept/<channel>/home`) | To change, where the home of whoever runs the daemon is | A program looks for what it keeps under the home. It finds its own, or makes it, and nothing of anybody else's. What it keeps of a kept channel it finds in that channel's turns and in no other (*Identity*): an agent that works in no project works in the channel's folder too, and that folder is hidden from its other turns. |
| Its credential, and the command-line program | To read | It acts on the factory by asking, as itself. |
| What it signs in with | To change | The folders it was handed, where it draws a sign-in the factory holds; otherwise, in each of its homes (its ordinary one and each kept channel's, laid out alike), only what its package says is its sign-in (`agent.wall.signin`), where it is, so that one renewed in any home is the one every home reads, and the rest of what its program keeps is that home's own. A package that says none has agents shown what its program keeps in the home whole (`agent.wall.keeps`), as one place: they have no home of their own, are told what they are ahead of what they are asked, and are in no kept channel. |
| The keys it is handed (`secrets`) | In its environment | Secrets the factory keeps, each under a variable (`agents::keys`): a key of another service's, read as the turn starts and scrubbed from what is written down of it. It has no other: not the daemon's environment, not the keys in the home of whoever runs it. |
| What its slot adds (`reads`, `changes`) | To read, to change | A toolchain, a cache. In one of these a file that is a credential by what it is called is not shown unless it is itself named. |
| What a table of its project keeps private (`reads`) | Not at all, unless its slot holds the scope | The folder its records are files in, and every copy of it in a card's worktree (`studio::closed` says which folders). And the repository, whatever its `git` says, where the repository holds anything of that folder (a file of it in a commit, or staged): its history would show what the wall hides. A folder that is kept out of the repository leaves the agent its git. |

What is said of a folder holds for everything in it that nothing nearer
speaks of. Nothing else of the device is there: not the rest of the data
folder, not another project, not the rest of the home. Of the daemon's
environment it inherits only what any program needs (`wall::INHERITED`).
It sees the processes it started and no others.

**Two authorities, and neither stands in for the other.** What an agent
may do to the repository on this device is its slot's `git`. What it may
do at a remote, or at any other service, is what a key of that service's
allows, and the service holds it to that. An agent has such a key only
where its slot hands it one of the secrets the factory keeps; the factory
does not read what a key allows and does not try to say it again. So a
key made for an agent (for the one repository, for what its work needs)
is what bounds it there, whatever is said here.

**What the factory does at a remote itself** (`fetch` and `push` of a
fleet, a package's actions) it does as whoever runs the daemon, with their
own sign-in, which no agent is shown, unless it is told a key to go by:
`key` of a fleet, and of a package's blueprint, names a secret the factory
keeps, which is a token of the service's own (a bot's account, an
application's). A package's program is handed it in the variable the
package wrote. Git is told two things, in this order: that it has no
helper to ask who a remote is reached as, which is what shuts out whoever
runs the daemon, and then that it has one, this program (`credential`).
The key is in the environment of the one step that speaks to the remote,
and the helper gives it to the address the repository has for that remote
and to no other, so that a remote that sends git on elsewhere is not given
it. A key the service will not take sends nothing: nobody else is tried.
A remote git reaches another way (over ssh, a folder) asks for no such
key, and a fleet told one refuses to reach it as somebody else.

**The daemon's own git runs no hooks.** A hook is a program among the
project's files or in its repository, and whatever an agent may write
there, or whatever a hook of the owner's goes on to run from the work
(a linter, a test), is run by whoever asks git. So git is asked in one
place (`kernel::git`), with `core.hooksPath=/dev/null` and no program of
the repository's to say what changed, and a test fails on anything that
asks it another way. No blueprint that ships asks git itself: what a lane
does in its repository is actions (`place.make`, `place.keep`, `place.send`,
`place.merge`, `place.clear`, `place.changed`, `place.take`, `place.follows`, and `place.gather` and
`place.forward` for a pipeline that lands work), so keeping an agent's work
and merging it runs nothing of it.

**Who commits is the project's to say.** What the fleet itself commits
(what an agent left uncommitted, a merge) is by whoever git is set up as
in the project, and after that as git knows whoever runs the daemon. Git
would go by an author and a committer named in its environment before
either, and a daemon's environment is whatever it was started in: so
the variables that name them are taken out of what the daemon's own git
is started with, in the one place it is asked. Until 2026-11-18 they
were handed on, and a daemon started where they were set committed as
whoever that named. Whose work a commit is, is said as it is made: what
an agent left is the agent's, at an address that is nobody's.

**What takes a wall away is `shell`.** A slot with no wall, or one shown
more of the device, gives what running a command here gives: making one,
or saying anything of one, takes `shell` of whoever asks, as a fleet's row
does of whoever changes its `wall`, `reads` or `changes` (a field of a
table may say what changing it takes). A slot that was here before there
were walls goes on with none until somebody says; `mw status` names those.

**Whether a program has what it needs behind its wall is found out by
starting it there.** `mw agent try <name>` (`agents::tried`) starts an
agent's program as a turn would, behind the same wall and with the same
environment, and asks it which models it offers, as its package says how:
no prompt is sent. One that answers has started, found what it is made of
and is signed in; one that does not says what it printed as it gave up.

**A wall that cannot be held is a failure, and never the open device
instead.** Where the system has no such program, or will not let it make a
namespace, an agent with a wall is not started and its turn says why and
what to do. `mw up --walls off` starts a factory that holds none, for
whoever means that.

**What it does not hold.**

| | |
|---|---|
| The network | It is the device's. What is read behind a wall can be sent anywhere, and everything on the device's loopback can be reached, the daemon among it (which is the point) and anything local that asks nobody who is there. |
| What the slot holds | An agent can read its own credential and the sign-in it was shown, and send them away. A sign-in made for agents, with a limit of its own, is what that costs least with. |
| What the daemon is asked | The API is held by the slot's scopes and not by the wall. |
| What runs with no wall | A `shell` step, a service and a package's action are started in the open, as whoever applied them may. A project's own blueprint is run for a running instance only as it was vouched for (*Instances and runs*); a command a person puts in one runs what is in the project then. |
| The kernel | A wall shares it. Another user, or another device, is still the only line against a program that sets out to break through. |

Only Linux has a program to hold one with today. [places.md](places.md) is
the design for the rest: a place a package declares (a container, a small
virtual machine), and steps and services behind one.

## Being told, not looking

The daemon follows things other programs change: the output of everything it
runs, which goes to files so that those programs outlive it, the client's
folder, and the folders of every library and of every project's mockups
(which anything may write). It does not look at them on a timer. Looking costs nothing for one
service and is a steady load for a hundred, whether or not any of them is
doing anything; that is the shape of what hammered the old hub.

- **Files** are followed with the system's own notifications (`hosting::Watch`:
  inotify on Linux, FSEvents on macOS, ReadDirectoryChangesW on Windows). One
  watcher serves the whole daemon. A service that prints nothing costs
  nothing; one that prints without pause is read at most twenty times a
  second.
- **A process the daemon started** is waited on, and the system says when it
  ends.
- **A process it inherited** from the daemon before it is nobody's child
  here, and no system call that exists everywhere says when such a process
  dies. Its files do: its exit code is written, or (on Linux) its output is
  let go of. One killed outright where the system does not report that is
  found by looking every five seconds; this is the only thing looked for.
- **If the system cannot notify** (its limit on watches is used up, or the
  folder is on a filesystem that reports nothing), output is looked at four
  times a second instead, and `system.degraded` is recorded to say so.
  On Windows the output of a program at work is looked at so as well as
  followed: of a file that whoever writes it keeps open, that system
  says the first writing and then nothing until the file is let go of,
  and what a step said after its first line was read when its program
  ended (seen there on 2026-11-19, by the hosting suite's first run).
  A library's folders are not looked at instead: a document written by
  another program is then found when the project's files are next looked
  over, after a run or when someone asks.

Measured here with 30 services doing nothing: the daemon went from 417
wake-ups a second (2.2% of a processor) to none. With a health check on each,
from 468 to 37, which is the checks themselves.

What still runs on a timer, in full:

| What | How often | Grows with |
|---|---|---|
| A service's health check | what its blueprint says (default 2 s) | services that have one |
| `every:` and `poll:` triggers, waits, timeouts, retries | what the blueprint says | blueprints that ask |
| `at:` triggers: a handler that fires at a time of day in the zone it says (`trigger/at.rs`, when it last fired kept beside its instance, a time missed while the daemon was away fired once as it is back), and so each row of a project's `triggers` that says `at` (`std/pipeline-at`) | once a day, or on the days it says | handlers that say `at`, rows of `triggers` that do |
| A command a row of a project's `segments` runs that says how long it may run before somebody is told (`warn_after` of `std/run`: a `watch` rule's `after:` on the conductor's clock, armed as the command's step begins and not kept across a restart, where the command is begun again with it) | what the row says, once | commands at work with such a row |
| A rung of a project's `chain` that says how long a card may wait at it (`std/later`, a `wait` on the conductor's clock, kept across a restart) | what the row says | cards waiting at such a rung |
| A row of a project's `triggers` that says `every` (`std/later` under `std/pipeline-timed`, a wait on the conductor's clock, kept across a restart and begun again each time it is over) | what the row says, a minute at the least | rows of `triggers` that say `every` |
| Looking for an inherited process that said nothing | 5 s | services inherited at the last restart |
| On Windows, looking at what a program at work has printed (`hosting::watch`, `Changes::or_looking`) | 250 ms | steps, services and kept programs at work |
| On Windows, a wrapper looking for the word that it is to end (`hosting::group::hears`; in the wrapper, not the daemon) | 100 ms | programs at work |
| A heartbeat from a member to its hub | 15 s | devices |
| A keep-alive on an open event stream | 20 s | pages and links that are open |
| A lock another git holds in a repository the daemon writes in (in the world's time and not on the conductor's clock: it is that program's, let go of in that time, `kernel::git::Waited`) | every 100 ms, for 10 s at most, while it is held | nothing |
| A starting daemon, for the lock on its folder, and for an address in use that a daemon of its own may still have, or was served at when it was killed (`Node::start`, `address::listen_at`) | every 25 ms, for 2 s at most, once at a start | nothing |
| A stopping daemon, for its address to be let go of by a program it was starting (`Node::stop`, `address::let_go_of`; Unix's) | every 25 ms, for 2 s at most, once as it stops and only while something answers there | nothing |
| What keeps a daemon running (`mw up`), beside another that keeps the same factory and has no daemon up (`keeps` in the command line's `serve.rs`; in the keeper, not the daemon) | every 50 ms, while that is so | nothing |
| The databases' threads letting their files go, as the daemon stops | once, a minute at most for all of them; one that is not free by then is said, with what its thread was at | nothing |
| Git that a stopping daemon waits for | once, 10 s at most, as the daemon stops | nothing |
| A daemon draining to stop (`mw down --drain`, `api::drain`): the bound it stops at whatever is still in hand; what it waits for is looked at again only when a turn or a run is said to have ended | once, ten minutes unless `--within` says, a day at most | nothing |
| Letting go of what runs' steps printed and what was said of them, older than the factory's owner said (`api::letgo`, `conductor::letgo`) | once as the daemon starts, then once an hour; where no number of days is said it reads that and does nothing more | nothing |
| A digest of what changed in something somebody follows hourly or daily (`api::followers::digests`, on the daemon's clock: it sleeps until the first digest that is due, the next whole hour or time of day after a change came to wait, and is woken as one comes; a digest due while the daemon was away is told as it is back, once) | once an hour or a day for each thing followed so in which something changed, and never where nothing did | things followed hourly or daily that changed |
| Emptying the Trash of what has been there longer than the factory's owner said (`api::trash`, `trash::Trash::empty`, measured quietly as `trash.emptied`) | once as the daemon starts, after it has finished what a stop left half done with a file, then once an hour, and when the owner says how long; it reads the rows of the Trash, which are few, and does nothing more where nothing is due | what is in the Trash |
| Writing measurements to disk | 10 s while anything worth a write was measured; at rest, once an hour | nothing |
| A sign-in that was set aside being good again | once, at the earliest such moment, and only while a turn waits for one | turns waiting for a sign-in |
| Sampling this device's load and memory | 30 s, kept in memory | nothing |
| A sandbox test that nobody asks anything, ended (`api::sandboxes`, on the daemon's clock, begun again by whatever is asked of it) | once, a quarter of an hour after it was last asked | sandboxes there are, eight at most |

Adding a timer means adding a row here.

A timer that fires is not the same as a disk that is written. A factory
nobody is using holds its instances and waits, and nothing is written to
its disk: what is measured whether or not anything happens (the device's
load, and how long the database took over its own work) is taken quietly,
kept in memory where every reader sees it, and written with the next
measurement that is worth a write, when the daemon stops, or after an
hour. Before that was so, an idle factory wrote about 150 KB every minute,
most of it the measurement of the write before.

## The client

TypeScript, Preact and signals; about 65 kB of script to start. The editor
(CodeMirror) and the Reference are separate chunks fetched on first use.

The fleet's page, which drew what was running and the agents with the
cards they had in hand, flat and in 3D (three.js), was taken out of the
application on 2026-11-23, to come back as a package somebody
chooses to add. What it was is kept on the branch `archive/fleet-view`.
What is running is under Activity, and each agent and what it is doing
under Agents; which card an agent's turn is about is still read from the
conversation the turn is part of (`heldIn` in `ui/src/heldin.ts`), for
what an agent is doing, what it writes and what a card cost.

**The client knows nothing about blueprints.** Their editor colours a
blueprint as one (the words of the language, the names somebody chose and
their own keys each look different, and a `${ }` is read as the daemon reads
it, which a YAML colourer cannot do), offers what can go where the caret is,
and says what a word is when it is pointed at. All three come from one
reader in the client (`ui/src/blueprint/scan.ts`) that is handed the
daemon's description of the language and walks it: from the file, down
through each field to what that field holds. The fields and their sentences
are the daemon's, the actions and their arguments are the device's, and the
names of steps and params are read from the text as it stands, checked or
not. A field added to the language is offered by the editor without the
client being changed. The client's tests run against a copy of that
description, which a test of the API holds to the real thing.

The client is laid out as a workbench: a rail at the far left chooses what
the side panel shows (channels, resources, extensions), the side panel can
be closed for room, and everything opened is a tab. Tabs are in groups, and
groups sit side by side or one above the other: a tab dragged to the edge of
a group splits it, one dragged among another group's tabs joins them, the
line between two groups is dragged to share the space differently, and a
group that loses its last tab gives its space back.

The resources in the side panel are in two parts that are not of a kind.
The factory is the whole installation: every blueprint it has
(the ones that ship, the ones packages bring, the shared ones), what has
happened and how it is doing. A project lists what is the project's,
each thing directly under it and nothing gathered under a word for what
kind of project it is, since one need not be software: its board once it
has one, its tables, libraries and mockups, the blueprints it *uses*,
which are the files of its own and the ones applied there that are still
alive (`GET /api/blueprints/used`, one question for every project, asked
again as instances come and go and blueprint files are written), and its
files, which every project has. A blueprint
opened from the factory's list is no project's, and applying it asks
which project it is for.

The factory's three pages (the activity, the measurements, the agents)
are of the workspace the side panel is of. One choice, in the head of each
and the same for all three, says what of it: all of the workspace, which
is what there is with nothing chosen; one project of it; or every
workspace, which is everything the factory has, what was applied for no
project and the factory's own log and figures with it. A project of
another workspace is not a line of it: the panel is put on that workspace
at its foot, which shows that workspace's own tabs, and each of the
three is of the workspace whose tabs it is among: one that is showing in
both follows the panel as it stands. What was chosen is
kept for each workspace, in the browser under who is signed in
(`ui/src/narrowed.ts`), and what a choice comes to, what a kept one comes
to as things stand and which lines are offered are functions of what was
read and what was kept (`ui/src/narrowing.ts`). The daemon is told none of
it, and no route takes a workspace: a page asks for projects. The agents
are read whole, as they always were, and held to the projects in the
page, so choosing reads nothing again; the activity asks for instances and runs with `project` said once
for each (`/api/instances?project=shop&project=blog`), which answers
what each alone answers put together, since a list that is cut at so
many cannot be held to some projects after it is read; and the
measurements ask with the label said once for each.

The arrangement is a tree (`ui/src/panes.ts`: a group, or two arrangements
sharing a space), and every change to it is a function from one tree to the
next, tested without a screen. It is drawn flat (`ui/src/workbench.tsx`):
each row of tabs and each view is placed by where the tree puts its group,
and a view is not a child of its group, so moving a tab moves its view on
the screen without making it again. A tab is dragged with the pointer, not
with the browser's drag and drop, which differs between the windows the
desktop app is drawn in and cannot be followed across a framed page.

Only the view showing in each group is on the page. What was typed in a
document and not saved is therefore kept by the store, not by the view, so
it is still there when the tab is shown again; closing such a tab asks
first. It is not kept across a reload of the page.

The open tabs, their groups and which is showing are a set for each
workspace (`ui/src/tabskept.ts`, `ui/src/tabsof.ts`), kept in the browser
under who is signed in. Showing another workspace's set closes nothing:
what a tab kept is kept by the tab and is there when its set is shown
again, and nobody is asked about what is unsaved. A set is of the
workspace it was opened in, whatever its tabs are of. What is gone
meanwhile (a project forgotten, a channel removed) is taken out of a set
when it is next shown.

A board is not built in. It is a collection of records (`studio::collections`)
that a blueprint declared, and `studio::board` is only what a board does
with one: columns, the order of cards, the events that say a card moved.
What a card is made of is the one thing about a board that the daemon's
own code depends on, and it is said once, in `studio::models`: the fields
of a card, and beside them the struct (`Card`) the board reads a record
through. The compiler holds the board's code to the struct; a test holds
the struct to the fields; and a collection that is a card's (one that
says `model: card`, or is called `board`) is held to the fields when its
blueprint is applied, because a blueprint may be written long after the
daemon was built. Nothing else reads a record's field by name: every
other field of every collection is kept, checked and handed on.
The records of any other collection are made and changed by name
(`studio::records`), each of which is an event; a board's cards are read
that way and written only through the board. Something is said on any
record the same way, a card included (`record.commented`), and that is
all the daemon knows of it: it is kept in the record's history and
changes nothing. Whoever says it may mark it public, and it is private
unless they do; the daemon keeps the mark, says it with the event and
shows it in the history, and what is told outside the factory because
of it is a blueprint's to do. That a card can have a question open, whose words answer
it and what follows is not in `studio::board` or anywhere in the daemon.
It is `std/cards`, one blueprint for all the cards of a board, over that
event and a field the board's blueprint declares (`question`); what a card
can do is that blueprint's to supply, and a card is a row. What a record is made of and
whether a value fits it is `studio::fields`: a record is checked whole
every time it is written, so only what its collection now declares is read
of it, and a collection is not declared again in a way its records would
not fit. Where the records are is the blueprint's choice, and the four
places there are (the installation's database, files in the project, a
store service, a folder of documents) sit behind the same few operations,
each an arm of the same match.

**A library** is the fourth: a collection whose records are the Markdown
files of a folder, as they are (`studio::documents`,
[library.md](library.md)). Nothing is written into a file to make it a
record. A document is known by where it is, so it is the same one on every
device with the same files; its title is the heading it begins with; what
its collection declares besides is a block of settings at its head. The
folder is read when it is asked about, and the node follows it
(`served::library`, with `hosting::Watch`) so that what another program
writes is said as any change to a project's files is. A file saved through
the daemon by someone who can be named is that document's change, in its
history; one written by another program is not, since nobody saw who did
it. The same module serves a library as pages (`/library/<project>/<name>/`):
each document as the page `studio::pages` makes of it, to whoever may
read the project's records, with a policy that lets nothing in a page run
and every path held inside the folder.

**Mockups** are the fifth: a collection whose records are the folders of
a folder that each hold an `index.html` (`studio::mockups`,
[mockups.md](mockups.md)): pages made to try an idea with, by a person or
an agent. A mockup is known by where it is, as a document is; what is
kept about it is a small file beside its page. Its folders are followed
with the libraries' (`served::library::follow`). Its page may hold scripts,
and they run, so it is never served as one of the daemon's own pages: the
node (`served::mockups`) gives whoever may read the project's records a way
in, an address with a name nobody could guess that reads that one folder
for an hour and is no credential, and everything served through it is
sent with a policy that puts it in a place of its own (`sandbox`: no
cookies, so never the session; nothing fetched but from its own way in,
so never the API). That holds in the client's frame and at the mockup's
own address (`/mockups/<id>`, which makes a way in for whoever is signed
in and sends the browser there), and for every kind of file, since a
drawing can hold a script. Ways in are kept in memory and nowhere else.

A library is also **written out as plain files** (`studio::site`, the
action `library.export`, the blueprint `std/site`). A page is made one
way, and what differs between one that is served and one that is written
out is only how what it links to is reached: `pages::body_with` asks
whoever is making the page where each link and picture leads, and a
folder of files has every link lead to a file that is there or be the
words it was. It fails closed: only documents that say `public: true` of
themselves are written unless all are asked for, and a picture or a file
only where a page that is written uses it. A page that is written out
says in itself what it may do (nothing runs, nothing is fetched from
elsewhere), since whatever serves it will say nothing. The folder is
built beside where it is to be and put in its place whole; what an
export wrote is listed in the folder, and a folder that holds anything
else is refused and left as it is. Nothing here hosts anything.

**A store service** is a database that a blueprint made and hosts, reached
at an address over a protocol of four requests that the application
defines ([stores.md](stores.md)). The application names no database: what
answers at the address is the blueprint's business, and a database it
knows nothing of is a blueprint and a small program that speaks the
protocol, with nothing here changed. The one that ships, `std/store`, is
such a blueprint; the program it hosts is the command line itself
(`store serve`), which runs as a process of its own and is no part of the
daemon. What a table is (its fields, its views) and the history of its
records stay in the installation's database whatever the store. A store
that is not answering is an error that says so, never an empty table:
"could not ask" is not "there is nothing".

A project's files are browsed with no source control behind them: a folder
at a time on the project's page, a file read (coloured, numbered) before it
is changed, a picture shown from its own bytes (`/api/projects/{id}/raw`,
which tells the browser neither to guess what a file is nor to run it). A
language's colouring is fetched the first time a file in it is opened. A
file is written as it is by the same address (`PUT`, the request's own
bytes, up to what is handed back as it is), which is how `mw file write`
puts a picture in a project byte for byte; what it is called says what it
is, as where it is read, and what is text in a library is a document
written as its page writes one.

A document is a Markdown file, and stays one. It opens as blocks to write
in, as its source, or as the page it would be published as. The page is
built by the daemon (`studio::pages`), and the one stylesheet for how a
document looks (`ui/src/page.css`) is used by the editor and put into every
page built, so what is typed and what would be published are drawn by the
same rules. The block editor is ProseMirror, fetched when a document is
first opened for writing.

The daemon serves the client from disk, so changing it never rebuilds Rust.

**Replacing the client does not touch the daemon**, or anything it runs.

- **A build takes nothing away.** Every file's name carries a hash of its
  content, new files are written beside the old, and the entry page is
  replaced last, in one step: that is the moment the new client exists. The
  builds before it stay on disk, because a page still on one of them
  fetches the editor and the Reference only when they are first opened,
  and a page is left open for days: every build replaced within a week,
  twenty at most (`ui/scripts/generations.ts`). An installed copy keeps the
  builds it served by the same rule (`xtask::generations`, `client.kept`
  beside it), so a page left open on it goes on through several installs;
  a package holds only its own build. One build at a time writes and
  tidies a folder (`dist.lock`, `ui/scripts/publishing.ts`): two side by
  side took away each other's new files.
- **The daemon is told, and says so.** The entry page names its build. The
  system tells the daemon when that page is replaced, the daemon reads it
  (as it also does when asked, by `/api/meta`), and records `client.updated`
  when the build changed, including a change made while it was down.
- **A page offers; it does not reload under you.** It shows that a new
  version is ready, with a button. The event is a stored one, so a page that
  was asleep is told when its stream resumes. A page from `npm run dev`
  reloads by itself, unless it holds unsaved edits.
- **So the client can be newer than the daemon.** A route the daemon does
  not have answers `no_such_route`, and the client says that the daemon
  needs restarting instead of failing in pieces.

**The desktop app is a shell**, not a fork of an editor: a native window
(`wry`, the system's web view) on the same client, which starts the daemon if
it is not running. Forking an editor would have meant owning an editor.

When the window starts the daemon it says why with the start (`up --note`),
and the daemon keeps that with its `system.started` event. That is the
factory's log: what it has to say for itself (started, stopped, doing
without something) is an event of its own in the journal, shown under
**Log** on the factory's Activity. Something true of the factory and of
nothing a person is looking at goes there, where it can be found later, and
not into a notice in front of whoever has a window open.

The window has no title bar of the system's. The client draws the top of the
window itself. Before anyone is signed in that is a slim bar; signed in,
there is no bar at all: the first row of tabs is the top line of the window,
the three window buttons sit at the right-hand end of that row, and the
window is moved by the empty parts of the row and the head of the side
panel. The buttons are see-through, so they are the page's colour in light
and dark and dim with the page under a dialog, and they stay above a dialog
and go on working. The client asks the window to do what a title bar does
(move, resize from the edges, minimise, maximise, close), and those are the
only things a page can ask of it. On macOS the system's own buttons are
kept, over the page's corner, and there the bar stays, to leave them room.
`--system-frame` gives the window the system's title bar instead, for a
system on which this works badly.

What the window opens is one address, and nothing else is asked or shown:
the one it is told (`--url`), else the environment's, else where its user
last signed in, else the usual one on this machine (`desktop::address`),
and a page of anywhere else is handed to the system's browser. Where
nothing answers there and the address is on this machine it starts a
daemon, which an address that only looks local makes wrong: from Windows,
a daemon in WSL is at `127.0.0.1`, which WSL passes through, and one
started on Windows in its place would be another factory. So the window
can be told to start nothing (`--no-start`): it asks the address whether
the daemon answers before anything of a window is made, and where nothing
does it says so in a sentence and ends. That is what it is run on Windows
for, against a daemon in WSL: the client is then in that system's web view,
with that system's clipboard, where the Linux window shown through WSLg is
handed nothing of a picture copied on Windows. Whether the picture is
pasted there has not been seen (below). A word the window does not take is refused
(`desktop::Asked::read`), since one misspelt would otherwise be passed over
and a window told `--no-strat` would start a daemon.

The app follows the system's light or dark setting. A web view does that by
itself everywhere but one place: a Linux app shown on a Windows desktop sees
Linux's setting, which nobody set. There the app asks Windows.

## Extending it

- A new **action**: one function registered with a spec (name, arguments,
  capability). It appears in `mw actions`, is checked in blueprints and can be
  sent to other devices with no further work.
- A new **agent**: an extension package, with no code at all; or
  `--command name=program` for something that only reads and prints.
- **Somebody else's product, set up on this device**: an extension package
  that brings actions and blueprints, again with no code. An action is one
  way of running the product's program, written down in the package and
  run by the application with no shell; the conductor asks for a package's
  actions each time one is done, so they come and go with the package.
  Opening a pull request for each branch a project's lanes send is one
  ([extensions.md](extensions.md)).
- A new **source of blueprints**: a library layer. Projects, a directory,
  the installed packages and the built-in standard library are layers
  already.
- A new **database to keep tables in**: a blueprint that makes and hosts it
  and gives out a `url`, and a program at that address that speaks the
  store protocol ([stores.md](stores.md)). Nothing in the application
  changes.
- **Existing projects**: `mw project create <name> --path <dir>` adopts a
  directory as it is. Like any project it is given a board, a fleet and
  a library unless it is added `--bare` or without one of them, and the
  one file written into it is the fleet's blueprint, where it has none:
  one it came with is left unapplied for its owner to read. Added
  `--bare`, nothing is written into it. It is the factory's
  owner's to do, and what is adopted is not trusted until they say so
  (*Identity*).

## What is not built

Stated plainly, so nobody finds out by trying.

- **An identity service** (sign-in through a provider, billing). Only the
  seam exists, and plans are a local setting. Such a service is hosted,
  and is kept apart from this application with everything else that does
  not ship in it.
- **TLS.** The daemon serves plain HTTP to the client and the command
  line. (A member's link is sealed by itself, whatever carries it.)
- **Windows and macOS are untested.** The code is written to be portable and
  the Unix-only parts are marked, and no gate runs a test anywhere but
  on Linux. On macOS nothing has been run at all. On Windows the
  program has been run by hand, once, on one machine ([status.md](status.md)
  says what was seen), and the suites have been built and
  run there by hand, on 2026-11-19 and again on 2026-11-20: that
  second day every one of the 2,047 tests compiled for it passed, the
  runner's whole suite and nearly all of the API suite's among them
  (`docs/build-times.md`, *Run on Windows*, has both runs, the first
  crate by crate). That says what held on those days and nothing of
  the next change. That it would compile for both, its tests with it, is asked of
  the compiler on Linux by hand (`docs/build-times.md`, *Checked for
  other systems, from this one*); a test that rests on what only Unix
  has is marked for Unix and is not compiled for Windows: 218 fewer
  tests are compiled there than run on Linux, each saying beside its
  mark what of Unix it needs (a wall, a program that is a file of a
  Unix shell, a daemon stopped by a signal). A path with
  its links followed is asked for in one place (`kernel::whole`), which
  on Windows says it as a person writes it and not in the system's long
  form (`\\?\C:\…`), a form that is not under the same place written
  plainly and that programs refuse; that is compiled for Windows and
  held as a function of text. A program is started, waited for and found
  again after a restart by this program itself on every system
  (`hosting::wrapped`), with no shell. Stopping a program and everything it started is a signal to
  its group on Unix (which macOS has and has not been run), and on
  Windows a job object held by whatever started the program: what a
  process holds for a program it started with no wrapper is run there
  by the hosting suite, and what a wrapper holds for itself by hand
  that once. There a stop is an ending with
  nothing asked first, nothing a program started outlives what holds
  its group, a wrapper is stopped by a word left in a file it looks for
  ten times a second, and a program is found by its name as that
  system's shell finds it (`hosting::named`). [status.md](status.md)'s *What is not
  held yet* has each in a sentence, and `crates/hosting/src/group.rs`
  the whole. A user's own `run:` is read by the
  system's shell, `/bin/sh` or `cmd`. The blueprints that ship have no shell script of
  their own (the `portable` gate), so what they run is a program and its
  arguments on any system; `std/log-feed` is the exception, and needs
  `tail` and a shell.
- **The desktop app** has been seen running on Linux only (through WSLg),
  and its own top bar has only been seen in a headless browser: whether
  dragging, resizing and maximising work in a real window is unconfirmed.
  Its macOS part is written against the windowing library and has been
  checked by the compiler from Linux, never built and never run. Its
  Windows part has been built on Windows by that system's own toolchain
  (2026-11-21) and run there only where it opens no window: asked what it
  takes (`--help`), and told to start nothing (`--no-start`) at an
  address where nothing answered, which it said before it ended. No window
  of it has been seen there, nor the client in one, nor a picture pasted
  into it. There is a package for each of the three systems (`cargo xtask
  package`: a folder to unpack and run, built on its own system when a
  version is tagged): a `.tar.gz` for Linux and for macOS and a `.zip`
  for Windows, which the tooling writes itself with its files stored as
  they are, since no program that writes one is on every system a
  package is made on. Only the one for Linux is of a system the product
  is tested on; one for Windows has been made on Linux, for trying, and
  none has been made on Windows or macOS, where the workflow that makes
  them has not been run. There are no installers and no updates,
  nothing is signed, and the package for macOS is the plain programs
  and no application. The icon is a placeholder (`desktop/icon/`): the
  window has it on Linux and Windows, and on Linux an install or the
  package's own `menu.sh` puts the product in the desktop's menu with
  it; the programs for Windows carry no icon of their own, and a
  package for Windows or macOS carries no pictures of one. Neither the
  window's icon nor the menu entry has been seen on a screen.
- **Sign-ins the factory holds have never met a real program.** The
  holding, the choosing and the handing over are tested against stand-ins.
  What the packages kept here say of their programs (`agent.signin`) is
  written from those programs' documentation and has not been run; none of
  them says how a limit shows, so no sign-in of theirs is set aside yet.
  There is nothing for sign-ins in the client: they are managed from the
  command line. Asking a program which models it offers is still done as
  whoever runs the daemon. How much of each sign-in was used is not
  counted. A daemon that is killed outright leaves a turn's program
  running until it next prints; a sign-in is taken to be free when the
  next daemon starts, which for that moment may not be so.
- **A key for a remote is a token given over https.** A fleet told `key`
  reaches its remote as that key and as nobody else. A remote reached over
  ssh has no place for one: it is reached as whoever runs the daemon, or,
  told a key, not at all. The commits the fleet itself makes (what an agent
  left uncommitted, a merge) are still by whoever git is set up as in the
  project (`git config user.name`), whoever sends them.
- **A member's events do not reach the hub's journal.** The hub knows a
  member's services and load from its reports, and gets the output of steps
  it sent; a member's own runs are visible on the member.
- **The built-in assistant understands three requests** (set up a project,
  set up its fleet, add a card). It exists so the product can be walked
  through and tested offline, not to do work.
- **The two packages** were each checked against the real program with a
  two-message conversation and its model list. Long sessions, what happens
  when the program wants a permission nobody is there to give, and rate
  limits are untested. A package can bring an agent, actions and
  blueprints; connectors are not built. An action a package brings is one
  run of its program: a package cannot yet bring an action that talks to a
  service, or keeps something running.
- **A package's actions have been run on Linux only.** They start the
  program from a list of arguments, with no shell, which is the same call
  on every system; that it behaves the same on macOS and Windows is
  expected and has not been seen.
- **Getting a package** from anywhere but the folder beside the product is
  copying its file. Nothing downloads one.
- **Search is word search** (SQLite FTS5) over files, blueprints, messages
  and cards. No embeddings.
- **A store for any database but SQLite**, and moving a table from one
  place to another. [stores.md](stores.md) says how a store for another
  database would be written, and what else of stores is not built.
- **In the client**: a tab is in one group at a time (the same document
  cannot be shown twice), and groups cannot leave the window; no editing of board
  columns, no way to create or delete files other than through an agent or
  the command line. The blueprint editor offers no topics after `event:`
  (nothing lists the topics there are) and no values for an argument beyond
  true and false; it does not rename a step where it is used, or go to
  where one is declared.
- **Updating the daemon itself** across devices. `std/rolling` runs a command
  on each device in turn and stops at the first failure; nothing downloads or
  swaps binaries.
- **The API has no version number.** A client newer than its daemon finds
  out one missing route at a time, not up front, and a route that changed
  its meaning would not be caught at all.
- **Between profiles**: nothing promotes a blueprint, or anything
  else, from one factory to another, and a copy of one goes one way, once,
  into a factory that does not exist yet; a snapshot is not a backup; a profile has no address by name;
  `profile` holds its name and plain variables and nothing else; and the line between two
  factories under one user is not enforced against what they run
  ([profiles.md](profiles.md)).
- **No MCP server.** See *Agents* above.
