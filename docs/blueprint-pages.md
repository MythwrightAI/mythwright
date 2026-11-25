<!-- public -->

# Blueprints in the app

Everything the factory does is set going by a blueprint: a board, a fleet
of agents, a pipeline, a bot. [blueprints.md](blueprints.md) is the
language itself. This guide is the other half: every button the app has
for blueprints, where it is, what pressing it does, what it never does,
and who may press it, each with a short example to try.

The examples use a project called `shop` and the board that ships,
`std/board`. Where a terminal does the same, the command is given beside
it. Two things hold everywhere:

- **Trying never changes what runs.** Preview, Sandbox test, Side by side
  and `mw explain` start nothing in your factory. Only Apply (and Run, and
  a comparison started on real work) does.
- **A button you may not use is not there**, or is there and closed with a
  sentence that says why. What you may do is what your role, and what you
  were given beyond it, holds: reading blueprints (`blueprints:read`),
  applying them (`blueprints:run`), writing a project's files
  (`projects:write`), managing the factory (`admin`).

## Where blueprints are

Open **Resources** on the rail at the left.

- **Blueprints**, a part of the panel of its own, lists every blueprint
  there is for the factory, in families: `std` for those that ship, and one
  for each installed package that brings some. Under each family is what
  somebody applies; under each of those, folded, its **parts**: the
  blueprints it runs for itself, with how many beside its name. A part
  that several blueprints run is listed under each. A small group called
  **misc**, at the end of a family, holds the few blueprints that are run by
  others and applied by people too.
- **Reference**, the line over the families, opens the whole language as a
  page (below).
- Under each project, **Blueprints** lists the ones it uses and those it
  keeps of its own, each with the files it links under it, and a last line,
  **New blueprint…**, for whoever may write the project's files.
- Under a project that has any, **Pipelines** opens its pipelines (below).

The box at the head of the panel narrows every list in it as you type: a
part is found under each blueprint that runs it, with what it is under held
open; Escape empties the box and leaves every fold as it was.

*Try it.* Unfold **Blueprints**, then `std`, then the parts of `std/fleet`.
Each is a blueprint of its own that the fleet runs; pressing one opens its
page, whose Summary says what it is a part of.

```sh
mw explain std/fleet
```

## A blueprint's page

Pressing a blueprint anywhere opens its page as a tab. Its head says, left
to right:

- its name, and whether its text **checks** (a green word), has problems
  (**1 problem**, **3 problems**, in red: Text shows each with its line), or
  is still **checking**;
- for something running, how it stands; for a text you have changed and not
  saved, **unsaved**;
- for one that ships, a line saying so (*ships with the product; applies to
  Shop*); for a project's own, where its file is, with **Save**;
- then the buttons: **Stop**, **Fork**, **Testing** and **Apply** (or
  **Apply changes**), each only where it can do something.

Under the head, the **views** of the page: **Summary**, **Text**, **Node**
and **Runs**, and once they mean something, **History**, **Sandbox** and
**Side by side**.

### Summary

The blueprint in plain words, made by the daemon from the text: what it is
and what you may tell it; what it makes (a table, with its fields folded
under it; a channel; an agent); what it does when it is applied; what it
hears and then does; what it may do; and what it does as it ends, fails or
takes a new text. Once it has run, what its runs have cost is said under
it.

- A line about a step or a handler is pressed to show where it is written
  (**See where this is written**).
- A blueprint that ships ends with **How to change what it does**, each way
  in the order to reach for it: tell it something else (applies it again
  with other params; nothing is copied), change the rows of a table it
  makes, and fork it. Each line that leads somewhere is pressed to go
  there.
- A fork starts with what it was forked from, where it differs, whether the
  default has moved since, and **Going back to the default** (below).
- A text that does not check has no Summary: **See them in the text** shows
  its problems.

A blueprint that ships opens on its Summary. A project's own opens on its
Text for whoever may change it, and on its Summary for anybody else.

*Try it.* Open `std/board` from the factory's list. Its Summary says it
makes a table called `board` with the fields of a card, and lists the
columns it is told (`backlog`, `ready`, `doing`, `review`, `done`) under
what you may tell it.

```sh
mw explain std/board
mw explain std/board --json
```

### Text

The text itself, in the editor: coloured, with what can be written next
offered as you type, and a word explained where the pointer rests on it
(with a link into the Reference). A blueprint that ships is only read here.

- **Save** (or Ctrl and S) writes a project's own blueprint. It is laid out
  first, the one way every blueprint is laid out: the same indentation and
  spacing, with nothing it means changed. Shift, Alt and F lays it out
  without saving.
- Ctrl and F finds in the text, and a line under the editor for each
  problem takes you to it.
- A file you save that has the name of one that ships is said, as it is
  saved, to be in use, or why not.

```sh
mw check blueprints/greet.bp
mw fmt --check blueprints
mw fmt blueprints
```

### Node

The graph of the text: each step a tile, saying what it says it is for
with its key under it, joined to what it waits on; each handler with the
event it hears. **Zoom in**, **Zoom out** and **Fit** are under it. A tile
pressed opens what is known of that step in the drawer at the right.

### Runs

Every run of it, the newest first, each with how it went, when and how
long. A run pressed shows its steps beside the list: what each gave, why
one failed, and its log under it. The run's id is a link to its own page
(below).

```sh
mw runs --instance in_1m7gm1c0h511x8qr
mw run run_1m7gm1c4e2bd9x7f
mw logs run_1m7gm1c4e2bd9x7f
```

### History

On the page of something running: each time it was applied, newest first,
said as which time it was (with the blueprint's own version where its text
says one), what it was applied with, in words and folded until you open it,
and the text it ran then. **Apply it like this again** applies that text
with those params again; it is offered only for what somebody applied
themselves, since what a blueprint started is its own to start.

## Testing

**Testing** in the head is one button with a menu of four lines. Each says
under it what it does, and a line that cannot be used yet says why (most
often: *Once its text has been checked*). **None of them changes anything
that runs in your factory but the last.**

### Preview

*Plays what it does on its graph; runs nothing.* The drawer opens beside
the page with the graph over the Summary's parts. As you scroll to a part,
what it says is played on the graph once: the steps in the order they wait
on each other, a handler's event arriving and its steps after it, how it
ends. Under the graph are **Play** and **Pause** (Space), **Previous frame**
and **Next frame** (the arrows, or `,` and `.`), and a track you can press
or drag. A line of a part pressed (**See this on the graph**) goes to its
first frame. Nothing is asked of the daemon to play it, so it follows the
text as you write it, before it is saved. Pressing Preview again, or
Escape, puts it away.

Who may: whoever reads blueprints. A text that does not check has no
preview; the line says why in the check's own words.

*Try it.* Open `std/board`, then Testing and Preview. Scroll to what it
does when it is applied: the step that declares the table lights, then the
one that starts `std/cards`.

### Sandbox test

*Runs this text in a factory of its own, with stand-ins.* The page shows
the **Sandbox** view. The text, as it stands on the page whether or not it
is saved, is applied in a sandbox: a factory of its own, made for this, with
a journal in memory, one made-up project and a clock only it moves. What it
does to tables, channels and cards is done for real there. Anything that
reaches outside (a command, an agent, a repository, a file, another device)
is a **stand-in** that runs nothing and answers.

The view has three parts:

- **Run it**: what it is told, **What goes first** (a blueprint applied
  before it, chosen and added with **Add**: a fleet needs the board first),
  whether stand-ins wait for you or answer as usual, and **Run** (**Run
  again** once it has run). Running again plays everything you did to the
  last one again, against the text as it is now, and says what differs.
- **What is done to it**, once it is up: **Say an event** (a topic and what
  it carries, then **Say it**), **Do something by hand** (**Do it**),
  **Move the clock on** (**Move it on**: a step that waits goes on, a time
  runs out), and for each stand-in that waits to be answered, how it
  answers (as usual, **It fails**, or **An answer written out**) and
  **Answer it**, or **Answer every one as usual from here on**.
- **What happened**: each run and what it did in order, the graph lit by
  what ran (a mark on each tile that did), the tables it made with their
  rows, what was said in its channels and who was told what; and at the
  foot, in the daemon's own sentences, that it was a sandbox test and what
  was stood in, how often.

Nothing of a sandbox is ever in the factory, the side panel, Activity or
anybody's inbox. Closing the tab ends it.

Who may: whoever reads blueprints.

*Try it.* Open `std/fleet`, add `std/board` under What goes first, and
press Run. The fleet asks its stand-in agents a few things as it comes up;
answer them as usual. Then do `board.add` by hand with a title and
`status=ready`, and watch the card be taken up.

```sh
mw sandbox std/board
mw sandbox std/fleet --first std/board --do board.add title="Try it" status=ready
mw sandbox greet --project shop --param who=Ada
```

### Side by side

*The same played against each text there is of it.* The page shows the
**Side by side** view: under **The texts**, each text of the blueprint the
page knows (as it stands on the page, as it was saved, the default that
ships, each project's fork of it), chosen with a tick. **Play them** (or
**Play them again**) plays the same things against each in a sandbox of its
own, and lays what came of each out in a column: what ran, how it stands,
and the figures it was measured by, counted and never scored. Under a
column, **Apply** applies that text in the factory, and **Fork**, under the
default's column, forks it. With two texts or more, **Compare on real work**
starts a comparison with them as its ways.

Where there is only one text, it says so, and what would make a second
(change the text on the page, or fork it).

Who may: whoever reads blueprints; Apply and Compare on real work are for
whoever may apply.

### Compare on real work

*Splits real work between ways in this factory, and counts what each comes
to.* This is the one line of Testing that changes what runs. The drawer
asks what the comparison is **Called**, what **A thing is** (a card, an
item), what is **Counted of each thing**, and each way: a name, its share,
and what it starts in place of the blueprint (another blueprint, a version
of this one, or **Start it as it is written instead**). **Another way** adds
one. **Start comparing** starts it; **Not now** leaves the drawer with
nothing started.

From then, wherever the project starts this blueprint, it is started one of
the ways, and what comes of each is counted. The comparison is shown on
**Measurements**, under Factory: each way in a column with what came of its
things, how many were accepted and what each cost. There **Give it a rule…**
has the shares follow what is counted, and **End it** ends it, saying which
way goes on from then (or none).

Who may: whoever may apply. The line is closed until the text checks.

```sh
mw compare start trial --project shop --of std/lane-round --per item --way one --way two
mw compare show trial
mw compare end trial --goes one
```

## Apply

**Apply** starts the blueprint, or updates what is already running under its
name to this text; on a page of something running it says **Apply
changes**. It is closed until the text has been checked, since what it takes
is not known before then. A project's own is saved first.

Where the blueprint takes params, or the project has environments, or it was
opened from the factory's list and is no project's yet, the drawer asks
first: the **Project**, the **Environment** it runs under (a project with
environments always asks), and each param, as a field with what it means
under it. **Apply** in the drawer applies; **Not now** leaves it. What you
wrote is kept if you go to another tab and back.

When what ships with this program has moved on from what is running, the
page says so under its head, with what moved by version first, and two
buttons: **Take what ships now** moves what is running to it (refused,
saying why, where your text does not fit it), and **Leave it** goes on with
what it has.

Who may: whoever may apply (`blueprints:run`). It runs as you, with what you
hold.

*Try it.* Open `std/board` from Shop's Blueprints and press Apply. The
drawer asks for the columns; leave them as they are and press Apply.

```sh
mw apply std/board --project shop
mw apply std/board --project shop --param 'columns=["todo","doing","done"]'
mw apply std/board --project shop --take
```

## Fork, and going back

A blueprint that ships is the same for every project. To change what it does
for one project only, fork it: its text is copied into the project, and
from then that copy is what runs there under the name. The default stays as
it is, and the fork remembers what it was forked from.

- **Fork** in the head of one that ships, for whoever may write a project's
  files, while the project has no fork of it. On a page of a project the
  fork is made for that project and opened in the editor at once; from the
  factory's list a dialog asks **For which project**, then **Fork it** (or
  **Cancel**). It is not offered where the project keeps no blueprints of
  its own.
- The default's Summary then says that the project runs its own fork, and
  that line opens it.
- The fork's Summary says where it differs from what it was forked from, and
  whether the default has moved since, by version first. Nothing is ever
  merged into a fork: its text is the project's to change.
- **Going back to the default**, the last part of a fork's Summary, opens a
  question, **Go back to the default?**, that says what goes, what runs then
  and what is lost. **Go back to the default** (or **Remove the fork**, where
  the fork says what ships does not) takes the fork away; **Keep the fork**
  leaves it.

Before you fork, look at the ways above it under How to change what it does:
telling it something else, or changing a row of a table it makes, is often
enough.

```sh
mw own std/board --project shop
mw own std/board --project shop --say
mw own std/board --project shop --back
mw explain std/board --project shop
```

## Stop

**Stop**, in the head of a page of something running, stops it and what it
started. Its tables and records stay. It is not undone: apply it again to
start it again.

Who may: whoever may apply.

```sh
mw instances
mw stop in_1m7gm1c0h511x8qr
```

## Something running, and behind

Everything running is listed under **Activity** (under Factory). A line
pressed opens its page: the same page as a blueprint's, of the text it runs,
with its key, how many things it started and the environment it runs under
in its head, and **Details**, which opens the whole of what it says of
itself in the drawer.

Something marked **behind**, on its line under Activity and in the head of
its page, runs an older text than the one there now for its blueprint
(somebody changed the file, or a new version of the program shipped
another). The mark in its head, pressed, says in the drawer which text it
runs, which it is behind, what moved between them by version first, and
what leaving it comes to, with the one button that brings it up to date,
saying what that does:

- **Apply it again (its steps run again)**;
- **Apply it again (it is carried over: its steps do not run again)**, where
  the new text says how something running takes it;
- **Apply it again (it takes the text where it is: nothing runs again)**,
  where only what it does later changed.

```sh
mw instances
mw apply --instance in_1m7gm1c0h511x8qr
```

## A blueprint of your own

A project's own blueprints are kept by the factory, not in the project's
repository, unless the project says otherwise.

- **New blueprint…**, the last line under a project's Blueprints in the side
  panel (and a line of the project page's gear), asks a **Name** (it may
  have folders: `tools/build`) and makes it with **Make the blueprint**: a
  file with one step that does nothing, opened to be written in. A name the
  project has, or one that ships, is refused before anything is asked.
- Write it in Text and **Save**. Its page then has everything a blueprint
  that ships has, but Fork.

*Try it.* Make one called `greet` in Shop and write:

```yaml
blueprint: greet
description: Say hello in a channel of its own.
permissions: [channels:write]

params:
  who: { type: string, default: everybody, description: "Whom to say hello to." }

steps:
  room:
    description: Make sure the channel is there.
    do: channel.ensure
    with: { name: greetings }

  hello:
    description: Say hello to whoever was named.
    needs: [room]
    do: channel.post
    with: { channel: greetings, body: "hello, ${params.who}" }
```

Save it, read its Summary, Preview it, and run it in a sandbox: the
sandbox's `greetings` channel gets the message, and nothing of yours does.
Then Apply it, and `#greetings` is in the side panel's Channels.

```sh
mw example list
mw example add research --project shop
```

### Where a project keeps them

The gear at the right of a project's page (and the head of its own page)
says where its own blueprints are, and, for whoever manages the factory,
offers:

- **Kept by the factory**, the usual: in the factory's data folder, never in
  the project's repository.
- **Also a repo folder…**: blueprints kept with the project's code as well,
  in a folder of its repository you name, then **Read it**. Those are read
  first, and one name in both is refused.
- **None of its own**: no blueprints of its own and no fork.
- **Move them in**, where a folder is said: each blueprint there is copied
  into the factory's place, and the repository's files are left for you to
  take away.

Changing it runs nothing again: what is running goes on as it was applied.

```sh
mw project blueprints shop
mw project blueprints shop --usual
mw project blueprints shop tools/blueprints
mw project blueprints shop --move-in
```

## The Reference, and Run it

**Reference**, over the factory's blueprints in the side panel, is every
word of the language, every action and every event, as the daemon describes
them: three parts, **Keywords**, **Actions** and **Events**, each counted,
found by what you type in its box. Each entry says what it is for and has an
**Example**, a whole small blueprint written with it, with a button to copy
it, and **Used in**, the blueprints that ship that use it, each opening at
the line. The button beside an entry's name copies an address that opens the
Reference there.

Where a word is easier seen working than read, its example has **Run it**: the
drawer runs the example in a sandbox, does to it what its scenario says, and
shows **What was done to it** and **What happened**, with a sentence on what
to notice. Nothing of it is in your factory, and putting the drawer away
ends it.

*Try it.* Find `retry` in the Reference and press Run it: the step fails
twice and goes on the third time.

```sh
mw actions
mw actions channel.post
mw schema
```

## Pipelines

A pipeline is a project's own blueprint kept under `pipelines/`. Its
**Pipelines** page (a line under the project once it has one, and a line of
the project page's gear before then) lists each with what it says it is
for, a mark for each of its latest runs (each opening that run's page), and
**Run**. Under them, folded, is **Everything else that runs here**.

- **New pipeline…** asks a **Name** and makes it with **Make the
  pipeline**: one stage to begin from, opened to be written in.
- **Run** runs it as you, its steps run again whatever changed. Where it
  is handed something, or the project has environments, the drawer asks
  first. Where the project keeps triggers, they are asked, and the run they
  begin opens as a tab; one they refuse says why on the page and in your
  inbox.

Who may: Run is for whoever may apply.

```sh
mw apply pipelines/check --project shop --force
mw runs
```

## A run's page

Every run of every blueprint has a page of its own, opened from its id
anywhere one is named, from Runs, and from Activity. It says what ran, under
which environment and with which branch, what set it off, how it stands and
how long it took, and its stages: each step at the top of the run, a task
with the steps inside it, something it called unfolded into its own run.

- **Stages** and **Graph** are its two views.
- **Copy link** copies an address that opens the run for whoever is signed
  in here.
- A step's output is drawn from its end; **Show 500 lines earlier** (and the
  like) reads what came before.
- A run that waits for somebody to let it go on has **Approve** and
  **Decline** under its head, for whoever may decide it.

```sh
mw run run_1m7gm1c4e2bd9x7f
mw logs run_1m7gm1c4e2bd9x7f build
```

## How what is running comes by a changed blueprint

In **Settings**, for whoever manages the factory, **Blueprints that are
running** says how something running comes by a blueprint that changed:
**Blueprints are** *As usual here*, *Held until somebody takes them*, or
*Followed as they change*, and **One that is taken lands** in place or
again. Following asks first what it gives up, with **Have them followed**
and **Not now**.

```sh
mw texts
mw texts held
```

## Where to read more

- [blueprints.md](blueprints.md): the language, every action and event.
- [first-run.md](first-run.md): a board and a fleet set up step by step.
- [environments.md](environments.md): what runs under `dev` and `prod`.
- `mw help sandbox`, `mw help own`, `mw help compare`, `mw help explain`.
