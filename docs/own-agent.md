<!-- public -->

# An agent we do not ship

The packages that come with the program are for a few command-line
agents. Yours may be another: a model provider's own command line that
has no package here, a tool your team wrote, a script around a model you
host. There are three ways to bring it in, least work first:

1. **A program that reads a prompt and prints a reply.** No package: the
   daemon is told the command as it starts.
2. **A package of your own.** One file, `extension.yaml`, that says how
   your program is started and how what it prints is read. It is what the
   packages that ship are.
3. **An agent the factory does not start.** Your program runs wherever
   you run it, and works through the `mw` command under a name of its
   own.

Each comes to an agent: a name in a slot, listed with the others, in
channels, given cards, with what it may ask of the factory held by its
slot (`mw help agent`).

## 1. A program that reads a prompt and prints a reply

If your program can be started, handed what it is asked on standard input,
and print its answer, nothing more is needed. Tell the daemon about it
under a name as it starts:

```sh
mw up --command mytool="mytool --quiet"
mw agent adapters
mw channel create lab
mw agent add helper --adapter mytool --channel lab
mw post lab "@helper what is in this project?"
mw read lab
```

`--command` is `name=program and its arguments`, split at the spaces (an
argument cannot hold a space), and said once for each program. `{model}`
in an argument stands for the agent's model, where it was given one
(`mw agent add helper --adapter mytool --model large`). `mw agent
adapters` lists what this device can run, and the name you gave is among
them. The daemon has to be started with the flag every time: it is not
kept.

Each turn the program is started, handed on standard input what the
agent is told of itself and then what it is asked, and everything it
prints by the time it ends is the agent's answer. A program that ends
with a code other than 0 has failed, and what it printed on standard
error is said with the failure.

What that leaves out, beside what a package gives:

- **Nothing as it goes.** Its answer is seen once the program has ended.
- **No session of its own.** Each turn starts the program afresh: what
  it would remember of a conversation is not carried on, only what the
  agent is handed with each turn.
- **No list of models.** One is typed with `--model`, and nothing checks
  it.
- **No counts.** What a turn used (tokens, cost) is not known, so what
  shows a turn, a card or a comparison has nothing of it to count.
- **Nothing to try it by.** `mw agent try` asks a package's program which
  models it offers; this one is tried by asking it something.
- **Nothing said of its mode.** Whether the agent may only read, edit or
  do anything (`--permission`) is held by its wall, where there is one,
  and is not said to the program.

Behind its wall (`mw help wall`) the program is shown its work, a home of
its own and the folders on the path, and nothing of the home of whoever
runs the daemon. A program that keeps its sign-in or settings there finds
none of it: write a package, which says what it keeps (`wall`, below), or
add the agent with `--no-wall`, which takes `shell` and lets it see the
device as you do.

## 2. A package of your own

A package is a folder with one file, `extension.yaml`, written in the
language blueprints are written in. [extensions.md](extensions.md) says
every field; here is the least one that is worth having, for a made-up
program called `answerer`. Every flag in it is made up: put your
program's own in their place, from its documentation.

```yaml
extension: answerer
title: Answerer
description: Agents that run on the Answerer command line.

install: Install Answerer and sign in with its own command before adding an agent on it.

agent:
  program: answerer

  permissions: [full]

  args:
    - ["--print-json"]
    - when: ${ turn.model != null }
      args: ["--model", "${ turn.model }"]
    - when: ${ turn.session != null }
      args: ["--resume", "${ turn.session }"]

  input: '${ turn.session == null ? turn.system + "\n\n" + turn.prompt : turn.prompt }'

  output:
    - when: ${ event.type == "text" }
      show: ${ event.text }
    - when: ${ event.type == "done" }
      reply: ${ event.text }
      session: ${ event.session }
      input_tokens: ${ event.usage.input }
      output_tokens: ${ event.usage.output }
    - when: ${ event.type == "error" }
      failed: true
      error: ${ event.message }

  wall:
    keeps: [".answerer"]
    signin: [".answerer/auth"]

  models:
    ask:
      args: ["models", "--json"]
      answer: ${ line.models != null }
      list: ${ line.models }
      id: ${ model.id }
      label: ${ model.name }
```

Field by field (each is under *The manifest* in
[extensions.md](extensions.md)):

- `extension`, `title`, `description`: its name (lowercase letters,
  digits and dashes), what people are shown, and one line about it.
  `install` is said to somebody whose device does not have the program.
- `agent.program`: the program, found on the path of the daemon's device,
  or a whole path.
- `permissions`: the modes it can be held to. `[full]` says it can only
  be run asking nobody's leave, so an agent on it is added with
  `--permission full`, knowingly. A program that can be held to looking
  only, or to changing only its work, leaves this out and says what each
  mode is in `args`, with an entry each `when: ${ turn.permission ==
  "read" }` (*`permissions`* there).
- `args`: how one turn is started. A list is always passed; a mapping
  with `when` is passed when it holds. `turn` is what the agent asks for:
  its model, the conversation to carry on (`turn.session`), its mode
  (*`args` and `input`* there).
- `input`: what it is handed on standard input. Put what the agent is
  asked here and not in an argument, which anything else on the machine
  can read. A program that has no place for what the agent is told of
  itself (`turn.system`) is handed it at the head, on a new conversation.
- `output`: each line it prints that is JSON is `event`, and every rule
  whose `when` holds is applied. `reply` is the answer (the last one seen
  is the turn's), `show` is what is shown as it works, `session` carries
  the conversation on next turn, the counts say what it used, and
  `failed` and `error` say it went wrong (*`output`* there). Your
  program's own lines say what each `when` looks for.
- `wall`: what it keeps in the home of whoever runs it (`keeps`), and of
  that what is its sign-in (`signin`), so that it is shown that much
  behind its wall and no more (*`wall`* there).
- `models.ask`: how the program is asked which models it offers, so that
  they are chosen from a list (*`models`* there). It is also what
  `mw agent try` asks.

Install it from its folder, add an agent on it, and try it:

```sh
mw extension add path/to/answerer
mw extension list
mw agent add helper --adapter answerer --permission full --channel lab
mw agent try helper
mw extension models answerer
mw post lab "@helper what is in this project?"
```

`mw agent try` starts the program where it will work, behind its wall,
and asks it which models it offers: no prompt is sent. A program that
answers was found, started and is signed in; one that does not says what
it printed as it gave up. In the client the package is under
**Extensions** on the rail, and **+** there takes an `extension.yaml` too.

What to look at when it does not answer:

- `mw agent list` says what each agent runs on and how it stands.
- What each turn was asked and answered, and why one failed, is in the
  agent's history in the client: **Members** under Factory, its
  **Agents** tab, the menu at the end of the agent's line, **History**.
- A rule that reaches for something a line does not have is skipped, not
  an error. Run your program by hand with the arguments your `args` come
  to, and read what it prints against what your rules look for.

A package may hold examples (lines your program really printed, and what
they should come to), which keep it honest as your program changes
(*Examples keep a package honest* in [extensions.md](extensions.md)). To
have a fleet's agents run on it, install it before the project is made
(an agent that says nothing runs on the first package installed whose
program is on the device), or say it in the fleet's row: `mw record set shop-agents-2
adapter=answerer` (`mw record list shop agents` lists the rows).

## 3. An agent the factory does not start

The factory can also leave your program alone: you run it however and
wherever you like, here or on another machine, and it acts on the
factory with the `mw` command as an agent of its own. Nothing is started
for it and no work is handed to it: it takes work up itself.

Invite it, on the factory's side, as somebody who may add agents:

```sh
mw agent invite walls --project shop
```

That prints an invitation once, which lets the agent in once, within a
day unless `--expires` says. Where its program runs, the agent joins with
it, and its credential is kept there, readable by that user alone:

```sh
mw agent join --url http://127.0.0.1:7700
```

`agent join` asks for the invitation on standard input, so that it is in
no shell's history. From then on every command it asks is asked as it,
with `--as` (or `MW_AGENT=walls` in its environment):

```sh
mw --as walls board shop
mw --as walls board give shop-4 walls
mw --as walls board move shop-4 doing
mw --as walls record comment shop-4 "Started on it."
mw --as walls inbox
```

What it does says it did it. What it may ask is what its slot holds: with
nothing said, what an agent usually holds, and changed as any agent's is
(`mw agent add walls --scope ...`, or the Tools panel beside a channel).
A message that names it, and a conversation written to it, waits in its
inbox; a card given to it is its to take up. `mw agent stop walls`
withdraws its credential, and inviting it again lets it in again.

Its credential is a bearer token in a file: whoever reads that file is
that agent. `mw help invite` says the rest.
