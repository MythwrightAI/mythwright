<!-- public -->

# Extension packages

Out of the box, an agent runs on the built-in assistant, which needs no
model and understands only a few requests. A real agent is a command-line
program from someone else, and the application knows none of them by
name. Each one comes as an **extension package**: a folder with one file,
`extension.yaml`, that says everything the application needs to run that
program as an agent. For a program that has no package here,
[own-agent.md](own-agent.md) walks through the three ways to bring it in,
least work first.

A package can also, or instead, bring **actions** and **blueprints**.
Actions are named ways to run a product's program; blueprints set that
product up on this device, in the same language as everything else. This
is how the application uses a product it does not know, even when no
agent is involved. See [Actions a package brings](#actions-a-package-brings)
and [Blueprints a package brings](#blueprints-a-package-brings).

## Installing and listing packages

In the client, open **Extensions** on the rail at the left. The packages
that came with the product are listed with an **Install** button each.
The **+** at the top installs any other package from its
`extension.yaml`. Each extension opens as a page of its own, listing the
models its program offers. An installed package that is not the one that
came with this version says so in its line and on its page, with an
**Update** button for whoever may install packages (see
[Updating an installed package](#updating-an-installed-package)).

From a terminal:

```sh
mw extension list                     # what is installed, and what came with the product and is not
mw extension add codex                # install one that came with the product, by name
mw extension add path/to/package      # or any package, from its folder or file (it is checked first)
mw extension update codex             # take the one that came with this program in place of the installed copy
mw extension update --all             # the same for every installed package that is not the one that came
mw extension models codex             # the models it offers, as the program itself lists them
mw extension remove codex
```

`mw extension list` says what each package brings (`agents`, the names of
its blueprints, or both), and under that, the actions the installed
packages bring, each with what it takes and gives. `mw actions` lists
those among all other actions, and `mw actions <name>` shows one.

The program must be installed on the daemon's device. It signs in to its
model's provider as the user who runs the daemon, unless its agent is
told to use a sign-in the factory holds and its package says how its
program takes one (see [`signin`](#signin-a-sign-in-the-factory-holds);
`mw help signin`). Installing a package needs `admin`, since it decides
what the daemon runs.

Some packages are kept with the source, in `extensions/`, and are what a
daemon built from source offers. A packaged product offers whatever is in
the `extensions` folder beside its program (`--packages DIR` names
another). Being offered is not being installed: nothing runs on a package
until someone installs it. The packages are short; read one alongside
this page.

### Updating an installed package

A package is installed as a copy of its one file. The copy keeps saying
what it said when installed, even as the one that ships with the product
changes: a newer one may say what its program needs behind a wall, which
of the things it keeps is its sign-in, or how to ask it what it offers.
An installation that is upgraded keeps the copy. **A package is never
replaced unless someone asks**: not at a start, not by listing, not by
opening a page. The copy may be one you changed.

What you are told is that the two differ, and where:

- `mw extension list` marks each installed package that is not the one of
  its name that came with this installation, and says where the one that
  came differs from the copy (`says agent.wall.signin besides`: a place
  in the manifest, by its keys). `mw status` names them in one line.
- In the client, such a package says so in its line under **Extensions**
  and on its own page, to whoever may see what agents run on.
- An error that refuses something because of it names the command. For
  example, an agent whose installed package does not say which of the
  things its program keeps is its sign-in is refused a private channel;
  where the one that came does say it, the error says
  `mw extension update <name>`.

That is all that is known. A manifest has no version, and nothing is kept
of what was installed but the copy itself. So a copy you changed and a
copy of what an earlier program shipped look the same, and nothing here
says which is older. Updating a package means taking the one that came,
whichever that is.

`mw extension update <name>`, and **Update** in the client, replace the
installed copy with the one that came, whatever was changed in the copy.

- It is installing the one that came over the copy (`mw extension add
  <name>` does the same and says less), so it needs what installing
  needs: `admin`.
- It keeps the replaced copy beside the new one, once
  (`extension.was.yaml`, in the package's folder under the data folder's
  `extensions`; the next update overwrites it).
- It says where the two differed.
- A package that is already the one that came is left alone, and says
  so. A name that nothing shipped has, and a shipped package that is not
  installed, are refused with a sentence.
- `--all` does it for every installed package that differs.
- The client asks first, in a dialog that says what is replaced and what
  becomes of the copy.

Anything that depends on what a package says is checked again as the
update is taken. An agent that was refused a private channel because of
its package can be brought in by adding it again.

## What a package can run

A package can run a program that is started once per turn, is given its
request on standard input (or as arguments), and prints **one JSON value
per line** while it works. Most agent command lines have such a mode.

A program that only reads a prompt and prints a reply needs no package:
`mw up --command name="program args"`.

## The manifest

The manifest is written in the blueprint language: the same strict subset
of YAML and the same `${ }` expressions ([blueprints.md](blueprints.md)).
Every field is listed here; any other field is an error, reported with
where it is.

```yaml
extension: example          # its name: lowercase letters, digits, dashes; not `builtin`, `std` or `external`
title: Example Agent        # what people are shown
description: One line about it.

install: Get it from example.com, and sign in with `example login`.   # optional: shown to someone who does not have the program

agent:                      # optional: a package brings an agent, actions, blueprints, or several
  program: example          # found on the PATH, or a path

  args:                     # how one turn is started
    - ["run", "--json"]                       # always
    - when: ${ turn.model != null }           # only when this holds
      args: ["--model", "${ turn.model }"]

  input: ${ turn.prompt }   # what it is given on standard input

  output:                   # what to take from each line it prints
    - when: ${ event.type == "done" }
      reply: ${ event.text }

  models:                   # optional: which models it offers
    ask: { ... }            # ask the program, or
    list: [ ... ]           # list them here

  signin:                   # optional: how it is handed a sign-in the factory holds
    key: EXAMPLE_API_KEY    # the variable it reads a key from
    limit: { ... }          # how a limit shows

  wall:                     # optional: what its program needs behind a wall
    keeps: [".example"]     # what it keeps in the home of whoever runs it
    signin: [".example/auth"] # of that, what is its sign-in and no conversation
    instructions: ".example/INSTRUCTIONS.md"  # the file it reads as its own standing instructions

actions:                    # optional: ways its program is run, each under a name
  status:
    description: Say whether it is signed in.
    args: ["status", "--json"]
    ...

blueprints:                 # optional: blueprints it brings, each a name and its text
  setup: |
    blueprint: example/setup
    ...

prompts:                    # optional: files those blueprints link, each a name and its text
  ask.md: |
    Look at ${item}.

repository:                 # optional: what it shows of a repository it serves, on a project's Git page
  serves: ${ contains(remote.url, "example.com") }
  parts:
    - { name: open, title: Open requests, action: open, columns: [{ field: title, title: Title }], link: url }
```

A package with none of `agent`, `actions` and `blueprints` brings nothing
and is refused.

A package with no agent names the program its actions run at the top
level, `program: example`, where an agent's would be under `agent`. That
is what is looked for on the daemon's device to tell whether the package
can be used there, and what [the rule](#the-rule) at the end of this page
keeps out of the application. A package whose blueprints run no program
need not name one.

### `args` and `input`: one turn

`turn` is what the slot asks for:

| Field | What it is |
|---|---|
| `turn.prompt` | What the agent is asked. |
| `turn.system` | What it is told about itself and the fleet. Where that was written as its standing instructions (`wall.instructions`, below), it is one line saying that what follows was asked through the factory. |
| `turn.model` | The model chosen for the slot, or `null` for the program's own choice. |
| `turn.session` | The program's handle on the conversation so far, or `null` for a new one. |
| `turn.permission` | How much it may do without asking: `read`, `edit` or `full`. |
| `turn.slot`, `turn.workdir` | The slot's name, and the folder it works in (also the program's working directory). |
| `turn.web` | `true` where the slot says its program may use the web without asking (`mw help web`), else `false`. Which of the program's own tools that covers is the package's to say; a package that says nothing of it leaves the program as it is. |
| `turn.cli`, `turn.cli_path` | The factory's own command: its name, and its full path as the program finds it (it is first on the program's `PATH`, and visible behind a wall). `turn.cli_path` is `null` where there is no factory around the turn. |

An entry of `args` is either a list of arguments that are always passed,
or a mapping with `when` and `args`. Map each permission to the program's
options with three such entries. A program with no place for
`turn.system` can be given it at the start of `input`.

Put what the agent is asked in `input`, not in `args`, wherever the
program allows: arguments are visible to everything else on the machine.

An agent acts on the factory with the factory's own command, signed in as
its slot, and the factory itself checks what the slot's key holds. A
program that asks permission for each command it runs has nobody to ask
during a turn. So a package for such a program says, in the program's
own terms, that this one command needs no asking (`turn.cli` and
`turn.cli_path` are for that), and nothing wider: every other command
stays the program's to ask about or refuse.

### `output`: reading what it prints

Each line that is JSON is `event`. Every rule whose `when` holds is
applied, in order. Lines that are not JSON are ignored.

| Field | What it does |
|---|---|
| `when` | Which lines the rule is for. Left out: all of them. |
| `each` | A list in the line; the rest of the rule is applied once per element, as `item`. |
| `session` | The program's handle on this conversation, carried to the next turn. |
| `model` | The model the program says it is using. It is shown beside the agent's messages; without it, the model the slot asked for is shown. |
| `show` | Text for whoever is watching, as it happens. |
| `reply` | The agent's answer. The last one seen is the turn's. At least one rule needs this. |
| `failed`, `error` | Whether the turn failed, and what to say about it. |
| `cost_usd`, `input_tokens`, `output_tokens` | What the turn used, where the program reports it. A cost it does not report is kept as not reported, never as zero: whatever shows a turn, a card or an A/B test's columns says so. |
| `refused` | Something the program was refused as it went (say, a tool it asked permission for and was not given), as a word. Each time a rule gives one, it is added to the turn's list of refusals. It is shown with the turn's result and decides nothing by itself, unless the stage's row says `refused_means: blocked`. |

A field that reaches for something a line does not have is skipped, not
an error: lines differ.

### `models`: what can be chosen

So that people choose from what the program really offers, not from a
list in a file that goes stale, a package can say how to **ask** the
program:

```yaml
  models:
    ask:
      args: ["models", "--json"]              # how it is started for this
      send:                                   # written to its standard input, one JSON value a line
        - { request: models }
      answer: ${ line.models != null }        # which line it prints holds the answer
      list: ${ line.models }                  # the list in that line; each element is `model`
      id: ${ model.id }                       # what the program is given to select it
      label: ${ model.name }                  # what a person is shown
      note: ${ model.description }
      hidden: ${ model.retired == true }      # optional: leave it out
      default: ${ model.isDefault == true }   # optional: the one it uses when none is named
```

- The answer is a line the program prints. A program that prints one
  value over many lines and then ends has answered with all of it.
- `list` is a list of models, or a map of them by name, in which case
  each `model` is its `key` and `value`.
- The program is stopped as soon as it has answered. The answer is
  remembered until the daemon restarts or someone asks again.
- A program that cannot be asked can have its models written out under
  `list` instead, each with `id` and optionally `label`, `note` and
  `default`. With neither, a model is typed in by hand.
- Choosing the program's default stores no model at all, so the slot
  follows the program when its default changes.

A program whose list does not say which model it uses when none is named,
but says so somewhere else, can be asked that too:

```yaml
      usual:
        args: ["models", "--default"]
        when: ${ starts_with(line, "Default: ") }   # which line it prints holds the answer
        id: ${ replace(line, "Default: ", "") }
```

Here each line the program prints is `line`: its JSON value, or for a
line that is not JSON, its text.

An agent is shown with the name its provider gives its model: the one
chosen for it; else the one its program last said it was using; else the
one marked `default`. Where a program does not say which model it uses
when none is named, an agent left on the default is shown as running on
the package, with no model named.

### `permissions`: what its program can be held to

An agent is given one of three modes: `read`, `edit` or `full`. A package
maps them to its program's own options in `args`. A program that cannot
be held to all three lists the ones it can:

```yaml
  permissions: [full]       # it asks nobody's leave, whatever it is told
```

An agent on such a package that is asked to run in another mode is
refused, with what it can be given instead. If `permissions` is left out,
the program is taken to honour every mode, so leave it out only when
`args` really does hold it to each.

### `signin`: a sign-in the factory holds

Started with nothing said, a program signs in as the user who runs the
daemon. The factory can hold sign-ins of its own (`mw signin add`) and
hand one to an agent for each piece of work (`mw agent add ... --signin
subscription --signin key`). The factory knows nothing about any
program, so the package says how its program takes a sign-in:

```yaml
  signin:
    key: EXAMPLE_API_KEY          # the variable its program reads a key for the API from
    token: EXAMPLE_TOKEN          # the variable it reads a subscription's long-lived token from
    folder: EXAMPLE_HOME          # the variable that says which folder it keeps its own sign-in in
    home: EXAMPLE_HOME            # optional: see below
    without: [EXAMPLE_PROFILE]    # optional: other variables it would sign in by
    limit:                        # optional: how a limit shows
      when: ${ matches(error, "limit reached") }
      until: ${ int(split(error, "resets at ")[1]) }
      for: 5h
```

| Field | What it does |
|---|---|
| `key`, `token`, `folder` | For each kind of sign-in the program can be handed, the environment variable it is handed in. A `key` is a key for the provider's API, and a `token` is a subscription's long-lived token: the variable is set to the value. A `folder` is a directory where the program keeps and renews its own sign-in: the variable is set to its path. The folder is used in place, never copied. At least one is needed. A kind left out cannot be added for this package or used by its agents. |
| `home` | A variable set to a folder of the slot's own (`<data>/slots/<slot>/home`, created empty the first time) whenever a `key` or `token` is handed. It is for a program that would otherwise find, in its usual folder, the sign-in of the user who runs the daemon and use that. It is often the variable `folder` names. The program then has none of that user's settings either. |
| `without` | Variables removed from the program's environment whenever it is handed a sign-in, whatever the daemon has them as: other ways it would be told who to sign in as. Variables of the kinds it was *not* handed are always removed. |
| `limit.when` | A condition on what a failed turn gave. When it holds, it was not the work that failed but the sign-in, which has hit its limit: the sign-in is set aside, and the turn starts again on another, from what it was asked. |
| `limit.until` | When the sign-in is good again, in seconds since 1970, where the program says. If it is left out, or gives nothing that reads as a future time, `for` is used. |
| `limit.for` | How long the sign-in is set aside otherwise, such as `5h`. Needed with `when`. |

`limit` can read:

- `error`: the sentence the turn failed with (what an `output` rule gave
  as `error`, or how the program ended);
- `reply`: what the agent had answered by then;
- `code`: the code the program ended with;
- `stderr`: the last 16 KB of what it printed on standard error;
- `now`: the time, in seconds since 1970.

It is only checked for a turn that failed and was handed a sign-in.

A package with no `signin` cannot be handed a sign-in: none can be added
for it, and an agent on it that is told to use one is refused, with a
sentence. A value is never put in an argument, and no expression of a
manifest can read one.

The variable names must not be the application's own (`PATH`, and names
beginning `MW_` or `FACTORY_`).

### `wall`: what its program needs behind a wall

An agent's program is started behind a wall unless its agent is told
otherwise (`mw help wall`; [architecture.md](architecture.md), *Walls*).
Behind the wall it sees the system, the folders on the path, its work, a
home of its own, and its own program file, wherever that is once links
are followed. A package says what else its program cannot do without:

```yaml
  wall:
    beside: ["../.."]
    keeps: [".example", ".example.json"]
    signin: [".example/auth"]
    instructions: ".example/INSTRUCTIONS.md"
```

#### `beside`

Folders of the program's own, for a program that is one file among
several (a script in a package of scripts). Each is written relative to
the program's file, with links followed, and shown read-only. One that
would show a folder that is never shown whole (the home, the factory's
data) is left out.

#### `keeps`

What the program keeps in the home of the user who runs it, written
relative to that home: its sign-in, its conversations. One that is not
there is not shown.

- An agent that signs in as the user who runs the daemon, on a package
  that says no `signin`, is shown all of these, writable. Its sign-in
  cannot be told apart from the rest, so it has no home of its own, is
  in no private channel, and is told what it is ahead of what it is
  asked.
- Any other agent is shown, of these, only its sign-in (`signin`,
  below); the rest is its own home's.
- An agent that uses a sign-in the factory holds is shown none of them:
  it has the folders it was handed, which is the point of handing it
  one.

#### `signin`

Of what the program keeps, the part that is its sign-in and nothing it
was asked or answered: one of `keeps`, or a folder or file within one
(`.example/auth`), written the same way. A path written from the top,
containing `..`, or outside what `keeps` lists is refused when the
package is read.

Every turn of an agent behind a wall runs in a home of its own: its
ordinary home, plus one for each private channel (a channel kept to its
members) it is asked in, so what it keeps of one is seen by no turn
asked anywhere else ([architecture.md](architecture.md), *Identity*). In
each of these homes, an agent that signs in as the user who runs the
daemon is shown only its sign-in, writable, where it is (the folder it
is in is created in that home). The rest of what the program keeps (its
settings, its history, its conversations) belongs to that home. So a
sign-in the program renews in one home is the one every other home
reads, and the program sees nothing else of that user's. A sign-in that
is not there is not shown, and that is not an error.

**A package that says no `signin` has agents that are in no private
channel** while they sign in as the user who runs the daemon: its
program keeps its sign-in in the same place as what it is asked, which
every agent signed in that way is shown.

Name a folder where the program keeps its sign-in in one. A file that is
itself what is shown can be written in place, but no other file can be
put in its place from behind the wall, and that is how many programs
renew a sign-in.

#### `instructions`

The file the program reads as its own standing instructions in every
project, written relative to the home (`.example/INSTRUCTIONS.md`). A
path written from the top, containing `..`, or inside its sign-in is
refused when the package is read.

Before each turn, the factory writes into this file what the agent is
and how it stands:

- its name, its project and the folder it works in;
- the factory's command, and what its wall shows it;
- the variables its keys are under;
- what its slot lets it do, in words: by area of the factory, on the
  device, and in what it signs in with (never a value);
- that its slot is set by others and never by itself;
- the standing instructions it was given.

The text goes under a comment saying that the factory writes it for each
turn and that edits to it are overwritten, and it is written only when it
differs from what is already there. `turn.system` is then one line.

The file is written only where it belongs to the turn's own home: behind
a wall, in the agent's own home. It is not written:

- where what is there is shown from the home of the user who runs the
  daemon (what `signin` lists, or everything `keeps` lists for a package
  that says no `signin`);
- where the program is pointed at a folder instead of its home (a
  sign-in handed with `home`, or a `folder`; a variable `signin` names
  that was set where the daemon was started);
- where the agent was told it may read or change what is there.

Everywhere else, and for a package that says no `instructions`, the agent
is told all of it ahead of what it is asked, as `turn.system`.

A file in a home that the factory wrote (recognised by the comment it
starts with) and that no turn uses any more (the package no longer names
it, or names it and it could not be written) is removed. A file that
does not start with that comment is never touched. A key's value is
never written there: what the agent is handed is named by the variables
it is under, and whatever it was handed is removed from what is written,
as it is removed from what its program prints. Changed standing
instructions (in the agent's row, or the Tools panel) are in the next
turn's file.

#### Whether a program reads `instructions`

The field is written from the program's documentation. To check it, take
an agent that has a wall, and give it standing instructions it cannot
miss (`mw agent add <agent> --instructions "End every answer with the
word Noted."`). Ask it something. Check that it does as they say, and that the file is in
its home (`<data>/slots/<agent>/home`, and for a private channel
`<data>/slots/<agent>/kept/<channel>/home`). Change the instructions and
ask again: it follows the new ones. Ask it, too, what its standing
instructions are: it should say them, not go looking for where they came
from.

A conversation the program held in the home of the user who runs the
daemon, before its agent had a home of its own, is not carried on in the
agent's home: each such conversation starts afresh once.

#### Writing `signin` for a program, and testing it

The field is written from where the program keeps what, and only running
the program shows whether it holds. Take an agent of the package that
signs in as the user who runs the daemon and has a wall, and go in this
order:

1. Install the package version that has `signin`, if an older copy was
   installed (`mw extension update <name>`). Ask the agent something in
   an open channel: it answers as before.
2. Bring the agent into a private channel
   (`mw channel create <name> --kept`, then
   `mw channel add <channel> <agent>`). Try its program there without
   asking it anything (`mw agent try <agent> --channel <channel>`: it
   starts in the home it has for that channel, and reports whether it
   started and is signed in). Then ask it something there. It should
   answer. If it says it is not signed in, something it signs in by is
   missing from `signin`: look in the home of the user who runs the
   daemon at what the program keeps beside its sign-in, and add it.
3. Ask it again there, then in the open channel. What it was asked in
   the private channel is not something it remembers in the open one,
   and the other way round.
4. Wait until its sign-in is due to be renewed (for a subscription,
   usually some hours), then ask it in the private channel first. If
   that turn fails as not signed in, and a turn in the open channel then
   works and the private one after it too, the program renews its
   sign-in by putting another file in its place, which it cannot do to a
   file that is shown. Name the folder the file is in instead, if that
   folder holds nothing the program was asked; or give the agent a key or
   token the factory holds (`mw help signin`), which is a value and is
   kept nowhere.

#### The environment behind a wall

Behind a wall, the program inherits none of the daemon's environment
except what any program needs (`PATH`, `HOME`, the language and the
proxies) and, for an agent that signs in as the user who runs the
daemon, the variables `signin` names: those are how that user told the
program who they are.

A program that needs something on the device no package could know (a
toolchain it builds with) is given it by its agent (`reads`,
`changes`), not by its package.

## Actions a package brings

A blueprint that sets up someone's product has to run that product's
program. It could use `shell`, but a `shell` step is a script: it is
written for one system's shell and tools, it reads output by searching
text, and it needs permission to run any command at all. Instead, a
package lists under `actions:` each way its program is run, and a
blueprint calls one by name: `do: example.serve`. The application runs
the program itself, with the arguments the package wrote and **no
shell**, the same on every system, and reads its output with the
package's own expressions.

```yaml
program: example

actions:
  serve:                    # done in a blueprint as `do: example.serve`
    description: Serve something on a port.   # what `mw actions` and the editor show
    makes: { what: something served on the port, named: port }   # what is there afterwards, for a summary
    with:                   # what a step gives it, checked like any action's arguments
      port: { type: int, required: true, description: "The port to serve on." }
      to: { type: string, default: "${ daemon.url }", description: "What is served; the daemon itself unless said." }
    refuse:                 # looked at before the program is run
      - when: ${ !starts_with(with.to, "http://") }
        says: "`to` must be a full local address such as http://127.0.0.1:7700; it is ${ with.to }."
    args: ["serve", "--port=${ with.port }", "${ with.to }"]
    answer: json            # how what it prints is read: `json`, or `text`
    fails:                  # looked at after it ran
      - when: ${ code != 0 && contains(stderr, "--operator") }
        says: "It does not take this from ${ daemon.user }. Allow it once with `example allow`, and do this again."
    out:                    # what the step gives out
      address: ${ answer.address }
    repeatable: true        # it may be run again after an interruption
```

### The fields of an action

| Field | What it does |
|---|---|
| `description` | One line saying what it does. Required. It is also how a step that calls the action, and has no description of its own, is described (up to its first full stop) wherever a blueprint is described in plain words (`mw explain`, the Summary on a blueprint's page). Write it as a sentence that someone who has never seen the program can read. |
| `makes` | What the action creates that lasts after the run that called it, if it creates such a thing: `what` it is, in a few words with an article (`an issue`, `a pull request`), and `named`, the argument under `with` that gives its name. A blueprint's summary lists it under *What it makes*, as `An issue called …`. Left out, the action is taken to create nothing that outlasts it. |
| `with` | Its arguments: a name each, with `type` (as a blueprint's param has; any, unless set or implied by the default), `required`, `default` and `description`. A step that passes anything else, or the wrong type, is refused when the blueprint is checked. A default may read `daemon`. |
| `first` | Another action of this package that runs before this one, every time, with the arguments of this one that it also takes. Its output is `first` in this one's expressions; if it fails, this one has failed, in its words. Use it to do something only when the program first says it should: `refuse` then decides by `first`. Only one action may run before another, no more. |
| `refuse` | A list of `when` and `says`, checked before the program runs (and after `first`). The first whose `when` holds is the failure, with the sentence `says`, and the program is not run. |
| `args` | The arguments the program runs with, in the forms `agent.args` takes: a list that is always passed, or entries with `when` and `args`. Each is passed as it is: nothing splits it, expands it or treats it as more than one argument. |
| `answer` | `json`: the program prints one JSON value, and `answer` is that value. `text`, or left out: `answer` is what it printed, as text. |
| `fails` | A list of `when` and `says`, checked after the program has run. The first whose `when` holds is the failure. If none holds, a program that ended with a non-zero code has failed, with the last line it printed. |
| `out` | What the step outputs: a name each, and the expression for its value. `steps.<id>.out.<name>` in the blueprint. |
| `repeatable` | `true` if running it twice does no harm. A step interrupted by a daemon restart is then run again; otherwise it fails, saying so. |

### What its expressions can read

| Name | Is |
|---|---|
| `with` | What the step passed, with what it left out filled in from `default`. |
| `daemon.url` | Where the daemon answers, such as `http://127.0.0.1:7700`: what a command a blueprint runs gets as `FACTORY_URL`. |
| `daemon.user` | The user the daemon runs as on its device, from its environment; empty if the environment does not say. |
| `first` | The output of the action named by `first`. |
| `answer`, `code`, `stdout`, `stderr` | Only after the program has run, so only in `fails` and `out`: its output read as `answer` says, its exit code (`null` if it was killed), and everything it printed on each stream. `answer` is `null` where `json` was asked for and the output was not JSON, so a package can say for itself what an answer it does not understand means. |

These are the expressions of a blueprint ([blueprints.md](blueprints.md)),
with the same functions. A manifest that reads a name which does not
exist where it is read (`answer` in `refuse`, an argument the action does
not take) is refused when the package is checked, with where.

### How the program is run

- **Directly, from the list of arguments.** There is no shell on any
  system, so there is nothing to quote and nothing that works on one
  system only. The program is found the same way as an agent's: on the
  daemon's `PATH`, or by its path.
- **With the same environment a `shell` step's command gets**:
  `FACTORY_URL`, `FACTORY_CLI`, `FACTORY_ENVIRONMENT`, and the `env` of
  the blueprint and the step. Its standard input is empty. It runs in the
  folder of the project its instance was applied to, which is how a
  program that finds a repository from where it runs finds the
  project's.
- **All its output is read**, up to 4 MB on each of its two streams. A
  program that prints more is stopped, and the step fails, saying so.
  Each line goes into the step's log as it is printed
  (`mw logs <run> <step>`), so a program that is waiting for something
  has said what by then.
- **It ends with its step.** A step that is cancelled, or runs past its
  `timeout:`, takes the program and everything it started with it. So
  does the daemon stopping: unlike a `shell` step's command, the program
  does not outlive the daemon, and when the daemon is back the step is
  run again if the action is `repeatable`.
- **A program that is not on the device** is reported as such, with the
  package it belongs to and then the package's own `install` text.

### What using one needs

- **`extensions:<package>`**, not `shell`. `shell` is permission to run
  any command; a package's action can run only that package's program,
  in the ways the package wrote down, so it needs less. `extensions` on
  its own covers every package. People have `extensions` unless it was
  taken from them; an agent has neither unless it was given one. A
  blueprint that lists `permissions` must list `extensions:<package>` (or
  `extensions`).
- **The package, installed.** Its actions exist from the moment it is
  installed: in `mw actions`, in what the editor offers after `do:`, and
  in what a blueprint is checked against. They are gone the moment it is
  removed. A blueprint that calls `example.serve` where `example` came
  with the product but is not installed is told to install it. A step of
  an instance still running after its package was removed fails, saying
  that the package is not installed here any more.
- **A name of its own.** A package that brings actions cannot be named
  after one of the device's own action groups (`board`, `agent`,
  `service`, `shell`, ...): installing it is refused.

## Blueprints a package brings

A package stays one file. Under `blueprints:` it has a name for each
blueprint and, after `|`, the blueprint's text, exactly as it would be in
a file of its own ([blueprints.md](blueprints.md)):

```yaml
extension: github
title: GitHub
description: Open a pull request on GitHub for each branch a project's lanes send there, and keep linked cards and issues in step.
program: gh

actions:
  account: ...
  find: ...
  open: ...

blueprints:
  pull-requests: |
    blueprint: github/pull-requests
    version: 3
    description: Open a pull request on GitHub for each branch this project's lanes send to the repository's remote.
    permissions: [extensions:github, board:read, channels:write, secrets]
    params:
      base: { type: string, default: "", description: "The branch each pull request asks to be merged into." }
    steps:
      account: { do: github.account, timeout: 1m }
      ...
```

- **Its name is `<package>/<name>`**, such as `github/pull-requests`, and
  the text must use that name. Names are lowercase letters, digits and
  dashes.
- **It is checked with the package.** A blueprint that does not check
  stops the package from being installed, and the error is the
  blueprint's own problems with their line numbers. It is checked against
  the package's own actions: calling `<package>.<name>` where the package
  declares no such action, passing an argument the action does not take,
  or not listing `extensions:<package>` under `permissions` is such a
  problem.
- **It exists while the package is installed**, not while the package is
  merely offered: among the blueprints that can be applied
  (`GET /api/blueprints`, the factory's **Blueprints** in the client),
  applied by name (`mw apply github/pull-requests`), and called from
  other blueprints (`call: github/pull-requests`). The package's page in
  the client lists the blueprints it brings.
- **Lookup order**: after a project's own blueprints and before the ones
  that ship as `std/...`. A project changes one for itself the way it
  changes a standard one: with a file of that name among its own
  blueprints, which the factory keeps for it
  (`@blueprints/github/pull-requests.bp`). A package cannot bring
  anything under `std/`: no package may have that name.
- **Linked files are in the package too.** A blueprint's long text is a
  file it links (`from:` of `text.put`), which for a project is a file in
  `prompts/` beside its blueprints. A package is one file, so those go
  under `prompts:`, each a file name and, after `|`, its text. `ask.md`
  there is what a blueprint of the package links as `from: ask.md`, and
  it is in the library as `<package>/prompts/ask.md`. These are checked
  with the package: a linked file that is missing, or one with a blank
  the blueprint passes nothing for, stops the package from being
  installed. They are listed among no blueprints, and a package with no
  `prompts:` is unaffected.
- **A blueprint that another of the package runs is listed under it.**
  The factory's list of blueprints shows what someone applies and, under
  each, its parts ([blueprints.md](blueprints.md), *Which runs which*). A
  blueprint of the package becomes a part by being named from the one
  that runs it (a `call`, a `spawn`, a `uses`, or its name as a value). A
  package that ships here lists which of its blueprints someone applies,
  one per line with the reason, in `contract/applied.txt`, and a test
  holds that file to the texts: a new blueprint that nothing names must
  either be named from what runs it or get its line.
- **Removing the package removes them.** They can no longer be applied or
  called. An existing instance of one keeps the definition it was given,
  as instances always do, and keeps running; stop it before removing the
  package if it should not.
- **It has no authority of its own.** It is applied like any other
  blueprint and can do what whoever applied it can. One that calls its
  package's actions needs `extensions:<package>`; one that runs a
  program with `shell` needs `shell`.

A blueprint that sets something up on the daemon's own account often
needs to know where the daemon is. A package's action reads it as
`daemon.url`; a command run with `shell` gets it as `FACTORY_URL`
([blueprints.md](blueprints.md), *Built-in actions*).

### Example: a pull request for each branch a lane sends

A project's lane commits what its agent changed on a branch of its own,
and with `push` sends that branch to the repository's remote
([blueprints.md](blueprints.md), *Built-in actions*). What happens to the
branch there is up to the service that hosts the repository, and the
application knows none of them: their names are kept in
`extensions/reserved`, and each is reached only through a package written
for it. All the application does is emit an event saying the branch was
sent: `card.pushed`, with the `card`, the `project`, the `branch`, the
`remote` and the card's `title`.

The package in `extensions/github/` is the one for GitHub, and its
program is that service's own command-line tool, `gh`. It declares three
actions (`github.account`, `.find` and `.open`). Its blueprint, applied
once to a project, listens for `card.pushed` for that project and opens
a pull request for the branch, with the card's title as its title and the
card's body and id in its description, unless the branch already has
one: a card sent back from review and pushed again stays in the pull
request it had. Where the pull request is gets posted in the project's
channel and said of the card (`card.proposed`), so the card's history
keeps it. A pull request that could not be opened is reported in the
channel, with why.

```sh
mw extension add github
mw apply fleet --project shop --param push=true        # the lanes send their branches
mw apply github/pull-requests --project shop           # and each branch sent is given a pull request
```

It needs `extensions:github`, plus `board:read` to read the card and
`channels:write` to post where the pull request is; no `shell`. Read
`extension.yaml` and the README beside it.

Both act as the user who runs the daemon unless they are given a key: a
secret the factory keeps that is a token of a bot account or an app,
passed as `--param key=shop-bot` to each. The branch is then sent, and
the pull request opened, by that account. A package's program is handed
the secret as a variable its own blueprint names in `env`, as any step's
program is handed a secret ([blueprints.md](blueprints.md), *Secrets*);
nothing in the application knows what that program calls it.

### Example: cards kept in step with issues

The same package keeps the cards of a project that are **linked** to an
issue of its repository in step with that issue, one ticket at a time
(`github/issues`; its README says how to turn it on). It is the example
of a package that reaches a service and is reached by it, and it shows
how little of that is the application's:

- **Nothing about the service is the application's.** How an issue is
  read, created, changed and commented on, what the service says of who
  wrote it, and what counts as a trusted author, are the package's
  actions and their expressions. The blueprint declares a table of links,
  listens for what happens to a card, and does what it is told.
- **Decisions are made by a block that knows no service.** Which side of
  a ticket moved since the two last agreed, whose changes are taken, and
  what happens to a change that is held: that is `sides.cross`. Which
  comments have not crossed yet: that is `sides.owed`
  ([blueprints.md](blueprints.md), *Built-in actions*, describes each).
  The blueprint that calls them is held to the `plain` rule like any that
  ships.
- **The service reaches the factory as a caller**, with signed requests
  ([blueprints.md](blueprints.md), *Callers*). Where it cannot, the
  blueprint polls on a timer (a handler's `poll`). Of what a call says,
  only which issue it was about is used: the issue is read again, as it
  is now.
- **Nobody gets through it what they do not hold.** A link is a row of a
  table whose fields `takes` what running the package's program takes,
  since adding one makes a card's words public. A comment on a card goes
  out only where whoever marked it public holds that too
  (`actor.holds`). Naming a record in a row requires being able to read
  it.
- **Incoming text steers no agent unless asked.** Anyone who can write an
  issue is nobody the factory knows. Whose changes to a title or body are
  taken into a card is a setting of the blueprint; a change by someone
  the service gives no standing is held until someone here says which
  side stands. A comment on an issue is posted on the card by the
  blueprint, and so answers nothing an agent asked: what a blueprint says
  on a card never does (`std/cards`). So a blueprint that relays words
  that may answer an agent asks the board itself (`board.said`), for
  those whose words it was told to take.

## What a package shows of a repository

A project whose folder is a git repository of its own has a **Git**
page: the branch the folder is on and how it stands against the branch
it tracks, what is changed and what of that is staged, the commits of a
branch, its branches and its remotes. That page is the application's,
read from git, and names no service.

What the hosting service keeps about the repository (pull requests,
issues, how checks went) is for a package to show, under `repository:`:

```yaml
repository:
  serves: ${ contains(remote.url, "example.com") }   # which remotes it serves
  parts:                                              # what it shows, in this order
    - name: open                                      # what the part is known by
      title: Open requests                            # its heading
      action: open                                    # the action of this package that reads it
      columns:                                        # what of a row is shown, in this order
        - { field: number, title: "#" }
        - { field: title, title: Title }              # `title` left out is the field's name
      link: url                                       # the field of a row that is where it is read
```

- **`serves`** is a condition checked against each of the repository's
  remotes. It reads `remote.name` and `remote.url` (the address as git
  has it, with any user name and password removed) and nothing else. A
  package whose condition holds for no remote is not offered.
- **Each part is an action of the package** that needs no arguments and
  outputs `rows`: a list, each row a mapping. It runs as a step would, in
  the project's folder, with no shell and nothing passed to it, so its
  program acts as the user who runs the daemon, signed in there or not.
  Its `fails` say what a program that is not signed in, or does not know
  the repository, comes to, and that sentence is shown in the part's
  place. A part that answers nothing (no `rows` at all; an empty list
  counts as rows) is reported as such, followed by the package's
  `install` text; it is never shown as having nothing to show. A program
  that can shape its output (`--jq`, `--json`) is told to in its `args`,
  so that each row is flat.
- **A row is passed on as its columns say and no more.** Each cell is
  text or a number; a list becomes its items joined; true and false
  become yes and no. The link is kept only if it is an address a browser
  opens as a page (`http` or `https`). At most two hundred rows are
  passed on, with a note of how many more there were.
- **Each part is a tab of the page**, after git's own (Changes, Commits,
  Branches, Remotes), titled by its `title`. It is known by its `name` in
  the page's address (`/<project>/git#pulls`), or by the package's name
  and its own (`sample.pulls`) where another tab has that name.
- **Nothing is read unless someone asks.** A part reaches the service,
  so it is read when its tab is first shown and when the person looking
  asks to read it again. Never on a timer, never because the repository
  changed, and never for a tab nobody opened. Reading one needs what
  using the package's actions needs (`extensions:<package>`), as well as
  reading the project. Someone who does not hold it is told so where the
  parts would be, and the program is not run for them.
- It only reads. Doing something from the page (a comment, a merge)
  would be a package action checked against whoever asks, and is not
  built.

`mw project git <project> --parts` reads the same parts from a terminal.

## Examples keep a package honest

A package's `examples/` folder holds lines the program really printed
(`name.jsonl`) beside what they should come to (`name.expect.json`: the
reply, the session, the model, what was shown, what was used, the
failure). The application's tests hold every package in `extensions/` to
its own examples, so a change in the program's output shows up as a
failed example rather than a silently wrong answer.

A package that brings an agent also says what it starts its program
with, in `examples/starts.json`: a list of cases, each a turn
(`permission`, and where they matter `model`, `session` and
`"web": true`; `"cli": false` for a turn with no factory around it) and
the `args` it should produce, in order. What a turn is told is written
as placeholders: `<system>`, `<prompt>`, and the factory's own command
as `<cli>` and `<cli-path>`. There is a case for each mode its program
can be held to, and the tests check every one, so a dropped or reworded
flag shows.

A package that brings blueprints shows what they do in the same folder,
with two kinds of file:

- **A stand-in for its program** (`stand-in.sh`): a script that answers
  as the program does, changes nothing outside its own folder, and
  records how it was called.
- **Cases** (`<what is true>.case.json`): each a list of steps and what
  must then hold. A step can apply a blueprint with some params and say
  what it outputs or the sentence it is refused with; stop an instance;
  restart the daemon; run the command line (which is how a case creates
  the project or card a blueprint needs, and emits the event a blueprint
  waits for); and check how the program was called and what the stand-in
  is left holding.

The tests play every case against a daemon of its own that is the real
binary and finds the stand-in under the program's name and nothing else,
so the real program never runs. A package that relied on a shell, or on
a tool of one system, would fail its own cases.

The steps a case may have are listed at the top of
`crates/cli/tests/suite/packages.rs`. Besides those above, they include:
a key that holds less than the case's owner, with the command line run as
it; a command that is expected to be refused; a call to a caller of the
case's daemon, as an outside service makes one, signed with the secret
the case gives; the daemon stopped as a step begins, or while the
stand-in is in the middle of what it was asked, and then started again;
and a stand-in for an agent's program, where a case is about what an
agent is then asked. Long text a package's program is asked can be
written once, in `examples/words.json`, and referred to by name in its
cases.

## The rule

Nothing outside a package names what a package is for. `cargo xtask
check` fails if the name of a package here, or of the program it runs
(`program:`, at the top level or under `agent`), appears anywhere in the
application but its documentation. A name shorter than four letters is
matched as a whole word, since a two-letter program name is inside half
the words there are; a longer one is matched anywhere. That applies to
the tests too, which is why what a package's blueprints do is tested in
the package, not in the application's tests.
