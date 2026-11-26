<!-- public -->

# The first run

The first hour with a new installation, in the client, with what each
thing is as it appears. The [README](../README.md) has the same path as ten
commands. Everything the client does goes through the same HTTP API the
command line uses, and agents use that command line too (`mw help`).

Start with the daemon running: the window (`mythwright`) starts it, or
run `mw up` and then `mw open` for the client in a browser.

## 1. Signed in, with nothing to register

Nobody registers. The daemon's first start made you the factory's owner,
signed in the terminal that started it, and called you **Unnamed** until
you give a name (nothing about the machine or its user is used as a
name). You have no email and no password yet. The window opens signed
in, and so does `mw open`: it asks the daemon for a one-time link, valid
for a minute, and opens it in the browser. The bare address shows a
line telling you to run `mw open`, with a password sign-in under it.

To sign in from another device, give your account an email, a password
and a name on your **Profile** (**You** on the rail), or run `mw
register` with all three from any terminal on this machine, which also
signs that terminal in. Anybody else is invited, and registers with the
invitation.

The factory also has its first workspace, called **Unnamed workspace**
until you give your name, and then after you (Ada's workspace). The foot
of the side panel shows it, and every project you add goes into one. In
the desktop app each workspace keeps its own open tabs; in a browser one
thing shows at a time, and the browser's tabs and Back move between
things.

## 2. A channel, and an agent in it

Create a channel (**+** beside *Channels*). From the list of who is in
it, at its head, choose **Add a person, agent or bot** and add an agent.
Out of the box it runs on a small built-in assistant that needs no model
and no network. It answers in channels and sets things up when asked,
which is enough for everything below except the work on a card (step 7):
given a card, it says it cannot do that work, and what would.

A real agent comes as an extension package. **Extensions**, on the rail
at the left, has an Install button for each package that came with the
program (Claude Code, Codex and Kimi among them); `mw extension add
claude` does the same. The agent is then one of the choices when an
agent is added, with its models listed as that tool lists them (see
[extensions.md](extensions.md), and [own-agent.md](own-agent.md) for a
program that has no package here). The tool itself must be installed on
the machine and signed in. It signs in to its provider as you, unless
the factory is given sign-ins of its own to share among its agents
(`mw help signin`).

An agent's program runs as you on your machine.
[protected.md](protected.md) says what that means.

## 3. A project

Ask the agent: *set up a new project called Shop*. The project appears
under **Resources**, on the rail at the left, with nothing under it yet.
No folder was given, so the factory keeps its files in a folder of its
own and lists no **Repo**; its files are on the project's own page
(**Its repo**). A project added with a folder (the dialog's **Folder**,
or `mw project create <name> --path <folder>`) uses that folder, and
lists its **Repo**.

Added by hand (the **+** beside Resources, or `mw project create <name>`),
a project starts with a board, a fleet and a library of its documents.
The dialog adds it to the workspace the side panel shows, and asks its
name (and, for the factory's owner, a folder). It has a switch for each
of the three, with its settings folded under it (the board's columns,
the library's folder); the command line leaves one out with
`--without board`, `fleet` or `docs`. A fleet works a board, so turning
the board off turns the fleet off too. Features come later (step 10).
With the board and the fleet left on, the next two sections are already
done: read them for what was made and how to change it.

A project an agent adds is added alone, because an agent may not apply a
blueprint unless its settings allow it. What an agent may do is shown
under its name in the tools panel beside a channel, an area per line,
and changed there (`mw help agent`).

A project made over a folder that already exists (`mw project create
<name> --path <folder>`, which only the owner may do) is not trusted
until its owner says so (**Trust this project…** in the gear of its
page, or `mw project trust <name>`). Until then, a file in it with the
name of a blueprint that ships is not used in place of that blueprint.

## 4. Its board, made by a blueprint

A project has no board until a blueprint makes one. Every blueprint is
listed under **Factory**: open **Blueprints** there and choose
`std/board`. It opens on its **Summary**: what it is, what it makes and
what it may do, in plain words. Switch to **Text** for the blueprint
itself or **Node** for it as a graph, and press **Apply**. (In the text,
pointing at a word says what it means, and **Reference**, the line above
the blueprints, lists every word, action and event.) A drawer at the
side asks what to apply it with, with the text still beside it: the
project it is for, the columns (each a pill: press one to leave it out,
type to add another) and where the cards are kept. Press **Apply** at
the drawer's foot, and **Board** appears under the project, empty.

The project now has a **Blueprints** list of its own, of what it uses:
so far, `std/board`. A blueprint is a file that says how something is
done, and this is the smallest useful one.

## 5. Its fleet

Ask the agent: *set up the fleet for Shop*. `fleet` appears among the
project's blueprints, and `fleet.bp` among its files (`mw fleet init
<project>` writes the same file).

Open it. It is YAML, and as you type the editor suggests what can go
there: Ctrl-Space asks on an empty line, and pointing at a word explains
it. Ctrl-F searches. It is formatted on save, before it is applied, and
on Shift-Alt-F. The pills at the top switch between **Summary** (what it
says in plain words, kept up to date as you type), **Text**, **Node**
(the blueprint as a graph; click a node for details) and **Runs**. Press
**Apply** to start it; any params are asked in the drawer beside the
text first.

A fleet gives work to agents: who works the project's board, and how.
The file says where the fleet is and calls the one that ships
(`std/fleet`, under **std**). Who is in it and what they do with a card
are two tables of the project, **Agents** and **Stages**, which sit
beside its board once the fleet is applied. The gear at the right of the
board's head lists them under **Its tables**; each opens beside the
board (and from there in a tab of its own), and each column's head shows
the stage for that column.

At first there is an architect to talk to in the project's channel and
one worker. A card that reaches *ready* gets a lane of its own, and once
it is worked it waits in *review* for you. Open **Agents** and give the
reviewers' row a count of 1: an agent comes up and reviews the work.
Give it 3 and it is a council, which passes what two approve. Change a
row in place and the fleet follows at once, whether work is under way or
not. **Add an agent** adds a row, run by another program or on another
model. **Stages** is the path a card takes: a column, the role that has
the cards there, and where a card goes when it is passed or sent back.

## 6. In a channel

The paperclip attaches pictures and documents to what you say (or drop or
paste them); an agent you ask is told where they are. An `.html` file is
drawn in the channel as a page, which is how an agent shows a chart or a
diagram ([views.md](views.md)). Ctrl, Shift and `/` list the keyboard
shortcuts.

## 7. A card, taken by an agent

Add a card to the project's **Board** and drag it to *ready*. The fleet
opens a lane for it and hands it to a worker.

- **Who does it.** A fleet's agents run on what could run an agent when
  they were first brought up. With an agent package installed and its
  program there (step 2) that is the package. Brought up before, as a
  fleet is by the steps above, they are on the built-in assistant and
  stay on it when a package is installed later: the card is taken, stays
  in *doing*, and the project's channel says that the assistant cannot do
  this work. Say then what a row of **Agents** runs on (its `adapter`:
  `mw record set shop-agents-2 adapter=claude` for the workers, as
  `mw record list shop agents` lists them), and move the card to *ready*
  again.
- **Where the work is done.** In a project that is a git repository with
  a commit the lane works on a branch of its own, `lane/<card>`, in a
  worktree of its own, and keeps what the agent changed as a commit
  there, written by the agent and committed by whoever git is set up as.
  A project whose folder the factory made has a commit from the start:
  the files it was started with, committed as it is made. Where git has
  not been told who you are (`git config --global user.name` and
  `user.email`), nothing is committed, `mw project create` says so, and
  the work is done in the project's folder until you commit those files. Started with `push`
  (`mw apply fleet --project <project> --param push=true`) it also sends
  that branch to the repository's remote, and says in the channel whether
  it went. It is sent as you, with whatever git here is set up to use, or
  with `--param key=<secret>` as a key the factory keeps: a bot's token
  (`mw help secret`).
- **Review.** The card is then in *review*. If it is not right, open it,
  say what is wrong and press **Send back** (or `mw board move <card> doing
  --note "..."`; the button sends it where the fleet's `stages` says a
  card that did not pass goes, which is `doing` unless you change it):
  it is taken up again, with the same agent in the
  same conversation and on the same branch, told what you said, as often
  as you send it back.
- **Questions.** An agent that cannot tell what is wanted asks instead of
  guessing: the card stays in *doing* with the question on it, at the top
  when it is opened, and goes round again when you answer there (or `mw
  record comment <card> <words...>`).
- **Done.** Moved to *done*, the lane's worktree is removed, and its branch
  when the work is on the remote or in the project's own branch.
- **Pull requests.** To have one opened for each branch that is sent,
  install the package for the service that hosts the repository and apply
  its blueprint to the project once (`mw extension add github`, then `mw
  apply github/pull-requests --project <project>`;
  [extensions.md](extensions.md)).

**Columns** on the board adds, removes or reorders its columns by applying
its blueprint again; the blueprint's **History** keeps what it was applied
with each time.

## 8. What is running, and which text it runs

**Activity**, under **Factory**, lists every instance (a blueprint that
was applied) and every run. Like **Measurements** beside it, and the
agents on the **Agents** tab of **Members**, it shows the workspace the
side panel shows; the choice in its head
narrows it to one project, or widens it to **All workspaces**, which adds
what was applied for no project and the factory's own log. Each instance
says which text it runs. When the text for its name has changed it is
marked **behind**, with how it would take the new one: in place, with
nothing run again, when only what it does later changed; carried over,
when the text says how (`on_update`); or by running its steps again.
Applying what started it brings everything below it up to date.
`mw instances` says the same.

## 9. An experiment: two ways of asking

An agent is told words, and whether other words would do better is
something to try, not to argue about. The program comes with an example
that is not code work, `research` (`mw example list`): a blueprint that
puts questions to an agent of a project and keeps the answers (`ask.bp`,
and `answer.bp` for one question), and two files of what the agent is
told. `prompts/answer.md` asks for five lines from what it already knows;
`prompts/careful.md` asks it to say first what it would need to check,
and to mark what it is unsure of. The first command below adds the four
files to the project's own blueprints as `@blueprints/research/` (the
factory keeps them in its data folder, not in the project's). Read them
from each blueprint's page in the client (`research/ask` is listed under
the project in the side panel), or with `mw file read shop
@blueprints/research/ask.bp`. They are short. (In the repository they
are `examples/research`.)

With a project called Shop that has its fleet and its library (step 3;
one the agent added there has no library until it is given one, `mw apply
std/docs --project shop`), from wherever the program is, a package you
unpacked included:

```sh
# 1. The example becomes the project's own: its blueprints and what they
#    link, written into the project by the daemon. Nothing is copied by hand.
mw example add research --project shop

# 2. From now each question goes one of two ways, which differ in nothing
#    but the words: one reads the file that is linked, the other reads
#    careful.md in its place. What comes of each question is counted.
mw compare start prompts --project shop --of research/answer --per question \
    --count question=research.answered:question:came \
    --way brief \
    --way careful --prompt careful.research/prompts/answer.md=research/prompts/careful.md

# 3. Ask. Each answer is said in the project's channel by the agent that
#    gave it, and is a document of the project's library.
mw apply research/ask --project shop --param project=shop --param channel=shop \
    --param 'questions=["What should a small shop measure first?","When is a queue too long?","Which shelf sells most?","How many tills does a shop need?"]'

# 4. Side by side.
mw compare show prompts
mw read shop                     # the answers, and where each was kept
mw record list shop docs         # the documents

# 5. Yours to say. From now on every question is asked the careful way...
mw compare end prompts --goes careful
#    ...or as the example is written again.
mw compare end prompts
```

`show` has a column for each way: which text it ran, by the first letters
of its revision, which words it read, how many questions it was given,
what came of them, and what its agent chose. With four questions that
part of it is laid out like this (the times are an example: yours are
what your agent took):

```text
                            brief               careful
                            d359853f            49398979
research/prompts/answer.md  as it is linked     research/prompts/careful.md 9e7ae6af
things given                2                   2

came of them                how many  after     how many  after
question answered           2         12.0s     -
question unsure             -                   2         31.4s

chosen                      how many  average   how many  average
agent.turn answered         2         11.2s     -
agent.turn unsure           -                   2         30.6s
```

Above it is a line for each way with how many questions went that way,
and below it what was measured under each (its runs and steps, and where
the agent's program says it, what its turns cost and the tokens they read
and wrote). Nothing in it says which way is better: whether an answer
that is longer and less sure of itself is worth what it costs is what you
were trying to find out, and reading the two kinds of document side by
side is the other half of it. The same screen is in the client: the
comparison's panel under **Measurements**.

What you see depends on what the project's agents run on.

- **On the built-in assistant**, which needs no model, everything runs and
  nothing is answered: each question is given its way, `show` counts
  `question unanswered` under both, and the channel says for each that the
  assistant cannot do this work and what would. That is the whole path
  with no answers in it, and it takes a minute.
- **With an agent package installed** (step 2) the questions are answered,
  each in the words of its way, and the two columns are the experiment.
  Four questions take as long as four turns of that agent.

To try words of your own, change `@blueprints/research/prompts/careful.md`
(`mw file read shop @blueprints/research/prompts/careful.md > careful.md`,
edit it, and `mw file write shop @blueprints/research/prompts/careful.md <
careful.md`) and start another comparison, under a name of its own: what
was measured under a name is kept, so a name is used once. To ask
something else, change the questions. Nothing is rebuilt and nothing is
restarted for either.

### Two ways of working a card

What works a card is a blueprint as well (`std/lane-round`, which a stage
starts for each card that is made ready), and who works it is a row, so
two ways of working the board's own cards are set side by side the same
way, on the cards you would have added anyway. Here the two ways differ
in who is given a card: the row of workers the project started with, or a
second row that is told something more. That second row is where another
model would be said (the row's `model`), or another agent package (its
`adapter`).

```sh
# 1. A second row of those who work, under a role of its own. The fleet
#    brings its agent up at once: `mw agent list` shows shop-careful-1.
mw record add shop agents title=careful role=careful count=1 \
    told="Work the card, then read what you changed once more before you say that you are done."

# 2. From now each card that is made ready goes one of two ways and keeps
#    to it: to the workers there were, as the fleet is written, or to the
#    row just added. What becomes of each card is counted.
mw compare start workers --project shop --of std/lane-round --per item \
    --way usual \
    --way careful --with careful.role=careful

# 3. Cards, as you would add them anyway.
mw board add shop "Sweep the floor" --status ready
mw board add shop "Count the till" --status ready

# 4. Side by side: how many cards each way was given, where each went and
#    how long after, and what its agents' turns came to.
mw compare show workers
mw board shop

# 5. Yours to say: from now on every card goes to the careful row.
mw compare end workers --goes careful
```

Each way has a column, as before: the cards it was given, and under *came
of them* each move a card made by where from and where to (`moved
ready>doing`, then `moved doing>review` once it was worked, and whatever
you do with it after), with what a review came to and a question asked
where there was one. With an agent that can do the work, the two columns
are the experiment, and a card is in *review* for you when its way has
done with it.

On the built-in assistant there is this much to see and no more: each
card is given its way and taken (`moved ready>doing` is 1 under both),
each way's one turn is counted as failed, and both cards stay in *doing*
while the project's channel says, of each agent by name, the command that
gives its row a program that can work (step 7). The comparison is real
all the same: give the rows their `adapter`, put the cards back in
*ready*, and they are worked under the ways they were given.

## 10. Cards put in order together

Some cards are single pieces of work. Others belong together, were
written up one at a time, and nobody has yet said which has to be done
before which. A **feature** gathers cards that are on the board already
and has one of the fleet plan them: what order they go in, which can be
done side by side, and what else is needed. Nothing is put in order on
an agent's word: you read its plan and accept it, or say on it what
should be otherwise.

A project is given features where it is asked for (`mw project create
shop --with features`), or afterwards: from its board's page in the
client (the gear at the right of its head, **Features**), or by hand, as
here. Then three cards that
carry a label, and a feature that gathers every card with that label:

```sh
mw apply std/features --project shop --param project=shop --param channel=shop
mw board add shop "The till" --label opening
mw board add shop "The shelves" --label opening
mw board add shop "The opening day" --label opening
mw record add shop features title="Open the shop" status=backlog labelled=opening
mw record list shop features                    # its plan, once it is in `planned`
mw record set shop-features-1 status=building   # accepted
mw record list shop board                       # each card says what it waits on (`after`) and what it is part of
```

The feature is put to the fleet's architect, which is shown the three
cards as they stand and says how they would be done; its plan ends with
its tickets, where a ticket is one of those cards or a new one, each
saying which it waits on. With a plan the feature waits in `planned`
(the project's channel says so). To have it planned again, say on it what
should be otherwise (`mw record comment shop-features-1 ...`).

Moved to `building` by somebody who may change cards, the plan is taken:
each card says what it waits on and that it is a piece of the feature,
new cards are added for whatever else the plan lists, and they are made
ready. Those that wait on nothing of each other are worked side by side,
by as many as the fleet has to work them; one that waits is begun when
what it waits on is done, and says meanwhile what that is. A plan that
does not fit the board (it names a card it was not given, or has two
cards each wait on the other) is not half taken: the feature goes back to
`planned`, and its history says why.

Cards can be named outright in place of a label (`gathers=shop-1,shop-2`),
and in the client a feature is a row of the project's `features` like any
other. An agent that moves a feature on has accepted nothing: that is a
person's, unless the project's features were applied saying `accepts:
anyone`. The architect can be asked for all of this in the project's
channel, in ordinary words: it adds the feature, and leaves accepting to
you.

On the built-in assistant, which needs no model, the feature gathers its
cards and stays in `backlog`: planning it takes an agent that can, which
is an agent package (step 2).

## Things to try next

Search everything: Ctrl-K, or Ctrl-Shift-F as an editor searches its
files (⌘ on a Mac), opens one search over the page. It finds what is
written in files, the library's documents (opened at the heading nearest
what matched), every table's rows, topics and their replies (a reply
opens the topic's talk at it), messages and cards, and projects,
channels, people, agents and runs by what they are called: each with
what it is and where, and only what you may read. The pills under the
box narrow it to one kind; the arrows and Enter open one. `mw search
<words> [--kind topic]` is the same from the command line.

```sh
# A forum's topic beside the channels: one question, asked once, answered
# in replies and marked answered, found again by its title (topics.md).
mw topic make "Which database for the shop" --project shop

# A project's folder of documents as a library: listed under Docs, spoken
# on, and served as pages to whoever is signed in (library.md).
mw apply std/docs --project shop

# Say in a channel what happens on a board, as a bot called `watch`.
mw apply std/watch --project shop --param collection=board --param channel=general

# Several projects that are one piece of work, under one name: a workspace.
# It is where they are listed, and a channel may be the workspace's own;
# nothing of who may read anything (`mw help workspace`). The factory has
# one from its first start, called after you, where a project is
# unless you say another: a project is always in exactly one, and a
# channel is a project's or a workspace's own.
mw workspace create "Shop front"
mw workspace put shop-front shop
mw channel create plans --workspace shop-front

# A service hosted behind a stable address, then a canary rollout of a new
# version: 10%, 50%, 100% of traffic, watched at each stage, rolled back if
# it misbehaves. Watch it in the client's Activity, with All workspaces
# chosen in its head: it is applied for no project, so it is no
# workspace's. (From a checkout: the example program is built there.) A
# version is judged by what it was sent: rolled out with nothing
# connecting, it is promoted and the slot says nothing was learnt of it
# (`slot.unwatched`).
cargo build -p kv-cache
mw apply examples/blueprints/kv-cache.bp --param build=false --param binary=$PWD/target/debug/kv-cache
mw apply examples/blueprints/kv-cache.bp --param build=false --param binary=$PWD/target/debug/kv-cache --param version=v2
mw apply examples/blueprints/kv-cache.bp --param build=false --param binary=$PWD/target/debug/kv-cache --param version=v3 --param fault=refuse   # rolls back

# What was measured while that happened: every step, and every connection
# the router carried once something has connected to the cache.
mw metrics step --by blueprint,step
mw metrics router.connection --by backend

# A database of the project's own, made and hosted by a blueprint, and a
# board whose cards are kept in it instead of in the installation
# (stores.md).
mw apply std/store --project shop      # its outputs say the url it answers at
mw apply std/board --project shop --param store=service --param database=<that url>

# A second factory beside the first, as development beside production. Each
# has a profile (a name), and they share nothing: its own data, accounts
# and address. The page of a named factory says which it is at all times
# (profiles.md).
mw up --profile dev1             # data in ~/.local/share/mythwright-dev1, a port of its own
mw --profile dev1 open           # signing in is to one factory: its first start made its owner
mw profiles                      # the factories here that have a profile
# What differs between them is said once, of each, by its owner, and read by
# any blueprint there as profile.vars.<name>. Not for secrets.
mw --profile dev1 apply std/profile --param 'vars={"api":"http://127.0.0.1:8080","replicas":1}'
# A new factory that starts with what another holds: nothing signed in,
# nothing running, and no way back into the one it was copied from.
mw apply std/snapshot --param to=/somewhere/copy    # asked of the factory to copy, by its owner
mw up --profile dev2 --from /somewhere/copy

# A second device, wherever it can reach this one. It is a factory of its
# own first: the package unpacked there and its daemon started, which makes
# its owner and signs that terminal in (`mw up`), since joining is asked by
# whoever is signed in there. The hub is reached at the address it listens on,
# which is this machine alone unless it was started saying another (`mw up
# --listen <address>:7700`, an address the other device can see). On the hub:
mw device invite builder-1       # prints an invitation, and the command to run on the other machine
# From then on a step marked `device: builder-1` runs there (`mw help
# blueprints`, *More than one device*, has one written out). What crosses
# between the two is sealed; `mw device list` on each shows the other's key.

# Kill the daemon outright; `mw up` restarts it and it finds its services
# where it left them.
```
