<!-- public -->

# Blueprints

A blueprint is a text file that describes a piece of work as a graph of
steps, and what to do when things happen around it. The conductor runs it.
Everything in the factory is set up and kept running by blueprints: a
service, a rollout, a board, a fleet, a research task.

This page is the whole language. `mw help blueprints` prints it. What each
button of a blueprint's page in the app does (Summary, Testing, Fork, Apply,
Stop, the Reference, pipelines), with an example to try for each, is in
[blueprint-pages.md](blueprint-pages.md).

## Start here

A blueprint is a YAML-like file. You apply it, the factory makes an
**instance** of it, and the conductor runs the instance: first its steps,
once, then its handlers, each time something they listen for happens.
Each time steps run is a **run**.

The words you need first:

| Word | What it is |
|---|---|
| `blueprint:` | The blueprint's name. The only required field. |
| `params:` | What it is started with. Read as `${params.name}`. |
| `steps:` | What it does once when it starts. Each step names an action with `do:` and gives its arguments in `with:`. A step waits for the steps named in its `needs:`. |
| `on:` | Handlers. Each listens for an `event:` (or a time, `every:`), may test it with `when:`, and then does an action. |
| `${ ... }` | An expression: a param, an event's data, what an earlier step gave out. |
| `permissions:` | What the blueprint may use. It is checked before anything runs. |

A blueprint that says hello in a channel, and says so again each time a
card reaches `done`:

```yaml
blueprint: hello
permissions: [channels]
params:
  channel: general
steps:
  greet:
    do: channel.post
    with: { channel: "${params.channel}", body: "Hello from a blueprint." }
on:
  done:
    event: board.item.moved
    when: ${ event.data.to == "done" }
    do: channel.post
    with: { channel: "${params.channel}", body: "${event.data.title} is done." }
```

Save it as `hello.bp`, check it, and apply it for a project:

```sh
mw check hello.bp
mw apply hello.bp --project shop
mw instances
```

Where to read on:

- *The shape*, *Steps* and *Handlers*: the fields of a blueprint, a step
  and a handler.
- *Expressions*: what `${ ... }` can read and the operators it takes.
- *Built-in actions*: what a step can do, from `wait` and `emit` to
  tables, boards, channels, agents and fleets. `mw actions` lists every
  action this factory has, and `mw actions <name>` shows one.
- *Events*: every event there is and what it carries.
- *Permissions* and *Secrets*: what a blueprint may use, and how it is
  handed a secret.
- *Writing one* and *Checking and running*: the editor, `mw check`, the
  sandbox and what a run looks like.

## The shape

```yaml
blueprint: kv-cache                 # required: the blueprint's name
description: Build and host the cache.
permissions: [shell, fleet]         # what it may use; checked before it runs

params:                             # what it is started with
  port: 7379                        # shorthand: a default; the type follows from it
  profile: { type: string, default: release, enum: [debug, release] }
  owner:                            # nothing after the name: required

state:                              # durable properties of an instance
  builds: 0

steps:                              # runs once, when an instance starts
  build:
    do: shell
    with: { run: "cargo build --profile ${params.profile}" }

on:                                 # runs each time something happens, while the instance is alive
  unhealthy:
    event: service.unhealthy
    do: channel.post
    with: { channel: ops, body: "${event.data.service} is unhealthy" }

outputs:                            # what a caller gets back
  address: "127.0.0.1:${params.port}"
```

Six more fields may be at the top:

| Field | Meaning |
|---|---|
| `version:` | A version of your own choosing, kept with the blueprint. |
| `env: { K: V }` | Environment variables for every action it runs. |
| `max_parallel: 4` | How many steps may run side by side. Default 8. |
| `one_at_a_time: true` | Its handlers take turns (*Handlers*, below). |
| `durable: false` | An instance of it keeps nothing (*Instances that keep nothing*, below). |
| `on_update:` | What a running instance does when it takes this text, in place of running its steps again (*Carrying a running instance over*, below). |

A blueprint needs at least one of `steps`, `on`, `uses` and `outputs`. One
with only `outputs` runs nothing: it gives out what its outputs make of its
params. That is what a rule is. `std/signin` is one: it says which sign-in
an agent is handed.

A param written out in full may have:

- `type`: `string`, `int`, `number`, `bool`, `list`, `object`, `duration`
  or `any`;
- `default`;
- `required`;
- `description`;
- `enum`: the values it may have.

### The syntax

A blueprint looks like YAML, and any YAML-aware editor colours it. The
grammar is a strict subset of YAML:

- block mappings and lists, `[inline, lists]`, `{ inline: maps }`;
- quoted and plain strings, `|` and `>` blocks;
- `#` comments.

There are no anchors, aliases, tags or merge keys. Only `true`, `false` and
`null` are special words (`no` and `on` are plain strings). A `${ ... }`
expression is one token: it may contain `:`, `#`, `,` and braces without
quoting.

Every mapping has a closed set of fields. A field that is not on this page
is an error. The error names the field and suggests the nearest real one.

## Steps

A step does exactly one thing:

| Field | Meaning |
|---|---|
| `do: <action>` | Run an action (see `mw actions`). |
| `call: <blueprint>` | Run another blueprint as a child and wait for its steps. Its `outputs` become this step's output. |
| `spawn: <blueprint>` | Start another blueprint as a child and carry on. |
| `task: { <steps> }` | Run the steps under it as one group. If one fails, the rest do not run and the task has failed. What is written beside it (`when`, `retry`, `timeout`, `on_fail`) applies once, to the whole group. See *Tasks*. |

It may also have any of these:

| Field | Meaning |
|---|---|
| `with: { ... }` | Arguments. For `call`/`spawn`, the child's params. A mapping may start from another one: `...: ${params}` hands on everything in `params`, and keys written beside it override it (*Handing on*, below). |
| `needs: [a, b]` | Steps that must settle first. Without it, a step follows the one above it. `needs: []` starts at once. |
| `when: ${ expr }` | A guard. When false, the step is `skipped` and the steps after it still run. |
| `timeout: 5m` | Fail the step if it runs longer. |
| `retry: 3` or `retry: { max: 3, backoff: 10s, factor: 2, max_backoff: 5m }` | Try again after a failure, waiting longer each time. |
| `allow_failure: true` | A failure here does not fail the run. Later steps see `steps.<id>.failed`. |
| `again: true` | If the daemon stopped in the middle of this step, run it again from its start instead of failing it as not safe to repeat (*Done again after a restart*, below). Only on a step that does an action. |
| `for_each: ${ list }` | Run once per element, in order. `item` and `index` are available. Given a whole number instead of a list (`for_each: ${params.count}`), it runs that many times, with `item` going from 1 to the number. The output is a list. If the daemon stops in the middle, the loop continues from the element it was on, over the list as it was when the loop began. Elements already done are not done again. |
| `watch: [ ... ]` | Rules checked while the step runs (below). |
| `on_start`, `on_success`, `on_fail`, `on_timeout`, `on_finish` | Hooks (below). |
| `device: <name>` | Run on another device, through the gateway. |
| `measure: <name>` | Also report the step's duration under this name, e.g. `build`. |
| `labels: { ... }` | For `call`, `spawn` and `task`: names and values carried by everything measured under the step, so two ways of doing the same work can be compared. See *Labels*. |
| `as: <key>` | For `call`/`spawn`: the child's name within its parent, exactly as written. In a `for_each` it must tell the elements apart (`as: "card-${ item.id }"`); one that is the same for every element is refused. |
| `environment: <name>` | For `call`/`spawn`, when the blueprint runs under none of its project's environments: the environment the child is started under (`environment: prod`). The child is held to that environment's rules (who approves what runs under it, which branches it lets run). A blueprint already running under an environment starts its children under that one only, and naming another is refused: a `dev` run never reaches `prod` through a step. See [environments.md](environments.md), *Rules*. |
| `env: { K: V }` | Environment variables for the action, added to the blueprint's `env`. |
| `description: ...` | What the step is for, in a sentence. The node view and the list of a run's steps show the step by this, up to its first full stop, with the key underneath in smaller type. The key is still what the text, the runs and the logs call the step. |

Steps with no dependency between them run side by side. The first failure
that is not tolerated stops the run: nothing new starts, running steps are
cancelled, and steps that never ran are marked `cancelled`.

A step may only read `steps.<id>` of a step it depends on. This is checked,
so what a step sees never depends on timing.

### Done again after a restart

When the daemon starts again, it takes up every step that was under way when
it stopped. What happens depends on the step's action:

| The action... | What happens |
|---|---|
| kept a place to continue from | The step continues from there. |
| kept no place, but is idempotent (safe to retry; it says so in `mw actions <name>`) | The step is done again from its start. The action is told it is being done again, so it looks for what it left the first time. |
| neither | The step fails, saying it was interrupted and is not safe to repeat automatically, so a person looks at it. |

A command (`shell`) is of the last kind: the program may have done half its
work. When repeating it does no harm (a build, a test run), say so with
`again: true`:

```yaml
build:
  do: shell
  with: { cmd: [make] }
  again: true
```

The step is then done again from its start instead of failing. `again`
covers only what the action does not say about itself, and it is read only
when the step is taken up after a stop.

- **With `retry`.** `retry` repeats after the step's own failure; `again`
  covers the stop, which is not a failure. Without `again`, an interrupted
  step fails, and its `retry` then tries again, using up one try. With
  `again`, the stop uses no try: the try that was under way is done again
  (`attempt 1 (resumed after a restart)` in its log).
- **With an idempotent action** it changes nothing: such a step is done
  again anyway.
- **With `durable: false`** it changes nothing. An instance that keeps
  nothing is not taken up at a restart, so none of its steps is. It is
  over, and whatever wanted the work applies it again (*Instances that keep
  nothing*, below).
- **On a `call`, `spawn` or `task`** it is refused. A step that starts a
  child finds the child it made when it is taken up, and the steps of a task
  each say `again` for themselves.

Use it only for work that is safe to do twice. A deploy, a push or a sent
message is not: it fails as interrupted if the daemon stopped in the middle
of it, unless its action says it is idempotent.

## Hooks

A hook is a list of actions to run at a point in a step's life. Each entry
may have `with` and `when`. `emit: { topic: ... }` is short for `do: emit`.

```yaml
build:
  do: shell
  with: { run: cargo build }
  on_start:
    - emit: { topic: build.started }
  on_success:
    - do: set
      with: { builds: ${ state.builds + 1 } }
  on_finish:
    - emit: { topic: build.over, went: "${ self.status }" }
  on_fail:
    - do: agent.consult
      with: { prompt: "The build failed: ${self.error}. Fix it if you can.", options: [retry, fail] }
```

| Hook | Runs |
|---|---|
| `on_start` | As the step starts, before its action. |
| `on_success` | When the step went well. |
| `on_fail` | When it failed. |
| `on_timeout` | When it timed out, in place of `on_fail`. |
| `on_finish` | When it has ended, after either of `on_success` or `on_fail`. |

"Finished" means any ending; it is not the opposite of failed. A step that
will be tried again has not ended, so `on_finish` runs once, after the last
try. A step that was cancelled, or never began, runs none of them.

Inside a hook, `self` is the step it belongs to: `self.out`, `self.error`,
`self.duration_ms`, and `self.status` (or `self.ok`, `self.failed`), which
is how `on_finish` knows how the step went. A hook that fails is reported
(`hook.failed`) and does not change the step's outcome.

A step is under way from the first entry of `on_start`. Its own action
begins when the last entry of `on_start` is done. If the daemon stops among
those entries, the step continues from the entry it had reached when the
daemon is back. Its action then runs for the first time, so an action that
is not safe to do twice is not refused for that.

### Deciding what happens next

`on_fail` (and `on_timeout`) can decide what happens next:

| Decision | Meaning |
|---|---|
| `retry` | Run the step again, at most twice. |
| `continue` | Treat the failure as tolerated. |
| `fail` | The default. |

An entry can be the bare word (`on_fail: [retry]`), or an action whose
output has a `decision`. That is how an agent decides: `agent.consult`
returns the option the agent chose. Every entry of the list runs, in order,
each time the step fails. A decision does not end the list; if several
entries decide, the last one stands.

### The blueprint's own hooks

A blueprint itself has `on_success`, `on_fail`, `on_finish` and `on_stop`
hooks.

- `on_success`, `on_fail` and `on_finish` are about the run of its steps as
  a whole, named as a step's are. In them, `run` is that run: `run.status`
  (`ok` or `failed`), `run.error` and `run.id`.
- They belong to the steps only. A handler's run that fails does not
  trigger the blueprint's `on_fail`: that hook may undo a rollout, and would
  undo one that had nothing wrong with it. Any run that fails says
  `run.failed`, and then `run.finished`, as a run does after any ending. To
  act on a failed handler, write another handler that listens for it
  (*Handlers*).
- `on_stop` runs when an instance that is still alive is stopped, before
  what it holds is given back. It is where a blueprint undoes what it set up
  outside the daemon. Its entries run in turn, each seeing what the one
  before `set`, and each once: if the daemon stops in the middle, it
  finishes the stop when it starts again, from the entry it had reached.
- An instance whose steps have finished stays alive only if something keeps
  it (*Handlers*). So a blueprint that is meant to be stopped has a handler.
- One more list has the same shape, `on_update`. It is not about a run of
  the steps; it stands in for one (*Carrying a running instance over*,
  below).

### Asking an agent when a step fails

The most common use of `on_fail`, asking an agent, has a short form on the
step:

```yaml
build:
  do: shell
  with: { run: cargo build }
  if_fail_ask_agent: builder      # or "@builder"; the quotes are needed with the @
  else: build-broken
```

When the step fails, after any `on_fail` of its own, the named agent is
told what failed and may fix it. It then says whether the step runs again
(at most twice). If it is not fixed, or there is no such agent, the step
fails, and `else` is the code it fails with: `steps.build.out.code` for the
steps after it, and `code` in the `step.failed` event.

## Watch rules

Rules that fire while a step is still running:

```yaml
watch:
  - after: 5m                          # once, when it has run this long
    emit: { topic: build.slow, msg: build slow, code: 5 }
  - every: 1m                          # repeatedly
    do: channel.post
    with: { channel: ops, body: "still building (${self.duration_ms} ms)" }
  - log: "^error"                      # when a line of output matches (once per attempt)
    emit: { topic: build.error }
    fail: true                         # and stop the step
```

A step that is done again after the daemon stopped in the middle of it
takes its whole time again, and its rules count from the new start. A rule
still says its event only once: firing again as it fired before the stop,
it finds the event it said then, wherever that is kept, and says nothing.

## Tasks

A task is a group of steps that succeeds or fails as one, with one policy
for all of them. It is a step whose one field is `task:`. Under it are steps,
written exactly as under `steps:`:

```yaml
steps:
  build:
    description: Everything it takes to have something to ship.
    task:
      version: { do: shell, with: { run: git describe --tags } }
      compile: { do: shell, with: { run: "make VERSION=${steps.version.out.stdout}" } }
      test: { do: shell, with: { run: make test }, timeout: 5m }
    when: ${ params.build }
    retry: 2
    timeout: 30m
    on_fail:
      - emit: { topic: build.broken, why: "${self.error}" }
  ship:
    do: shell
    with: { run: "ship ${steps.build.out.version.stdout}" }
```

The steps inside are ordinary steps and run as they would under `steps:`,
in the same run: each follows the one above it unless its `needs` says
otherwise. When one fails, after its own `retry` and `on_fail`, the task has
failed at that step: the steps after it do not run and are marked
`cancelled`.

The fields beside `task:` belong to the task. Each means what it means on
any step, for the group as a whole:

- `when` false skips the whole task: none of its steps begins.
- `retry` runs the whole group again, from its first step.
- `timeout` bounds the whole group. The step it was on is cancelled.
- `on_fail` (and `if_fail_ask_agent`) runs once, when the group has failed
  for good. A `retry` decided there runs the group again.
- `allow_failure`, `needs`, `for_each`, `measure`, `on_start`, `on_success`,
  `on_finish`, `on_timeout`, and `watch` rules with `after` or `every` work
  the same way.
- `env` is handed to every step inside. A step that sets the same variable
  keeps its own value.
- `labels` are carried by every step inside, and by what those steps start
  (*Labels*, below).

A task runs no action of its own, so `with`, `device`, `as`, `again` and a
`log` watch rule are refused on it. Put them on the steps inside.

#### Reading what its steps gave out

Inside a task, steps read each other as steps always do
(`steps.version.out.stdout`). They read the steps outside that the task
waited for the same way.

Outside, a task outputs what each of its steps output, by name, the way
`call` outputs a blueprint's `outputs`: `steps.build.out.version.stdout`. So
`steps.<task>.out.<step>` is that step's `out`, and is `null` for a step that
was skipped. When a task failed and the failure is tolerated,
`steps.<task>.out` holds the outputs of its steps up to the failure.

Both are read as `steps.<name>`, so a step in a task may not have the same
name as a step outside it.

#### In a run

In a run, a step inside a task is called `<task>.<step>`: `build.compile`.
That is its name in `step.started`, `step.finished` and `step.failed`, in
`mw run`, in `mw logs <run> build.compile`, and as the `step` of its
measurement. The task is a step there too, `build`: it starts before its
steps and finishes after them. In the node view the task is a node, and its
steps are the nodes after it, under the same names.

A task may go wherever a step may: under `steps:`, in a handler's `steps:`,
and inside another task, where its steps are named `release.build.compile`.
After a restart, a task continues from the step it was on. So does a task
that is an element of a `for_each`.

## Handlers

`on:` handlers run while an instance is alive. An instance stays alive after
its steps finish if it has handlers, holds something (a service, a router,
an agent), or has live children. Each handler has one source:

| Source | Fires |
|---|---|
| `event: board.item.*` | On an event whose topic matches. `*` is one word, `**` any number of words. |
| `every: 5m` | On a timer. |
| `poll: { every: 30s, do: <action>, with: {...} }` | When the action's result changes. For things that cannot push. The action is first asked when the handler begins to listen; that answer is only the baseline for the next. Never more often than every 5s. `event.data` is what it read, `event.previous` what it read the time before. |
| `at: "02:00 Europe/Zagreb"` | At a time of day in the zone written after it. Days go before it when it is not every day (`mon,thu 09:30 UTC`). A time with no zone is refused. `event.data.at` is the moment it fired for. If times passed while the factory was stopped, it fires once when the factory is back, for the last of them. It may contain `${ }` holes, filled from `params` when the handler starts. |

A handler also has a body: `steps:` (a graph, like the main one), or a
single `do:` or `call:`. Optional fields:

| Field | Meaning |
|---|---|
| `for:` | Which events are addressed to it (*Who an event is for*, below). |
| `when:` | A guard over `event`. |
| `scope:` | Whose events it hears (below). |
| `debounce:` | `debounce: 2s` collapses a burst into one run with the latest event. |
| `turn:` | What its events take turns by (*A turn for each*, below). |
| `outside:` | Its event is said from outside (*Events*, *From outside*). |

```yaml
on:
  card_ready:
    event: board.item.moved
    when: ${ event.data.to == "ready" }
    steps:
      weigh: { spawn: weighed, as: "weighed-${event.data.id}", with: { item: "${event.data.id}" } }
```

### Scope

| `scope:` | Hears |
|---|---|
| nothing, or `scope: project` | Events of its own project, and the factory's own events, which belong to no project. |
| `fleet` | Every event in the factory, whichever project's. |
| `own` | Only events from this instance and its children. |
| `target` | Only events from the instance this one is attached to. |

Whose an event is, and what an instance in no project hears, is in *Whose
an event is*, below.

What an instance says as it ends (`instance.failed`, `instance.finished`,
`instance.stopped`) reaches its parent's handlers however late they get to
it, after a restart too. That is how a fleet hears that something it started
failed.

### A handler does not hear itself

A handler does not hear what its own runs say. One that listens for
`chatter.*` and answers with `chatter.reply` is not answering itself. So a
handler cannot wake itself by saying the event it listens for; another
handler can hear it, or `every:` runs it again.

The check warns of a handler that says the very topic it listens for. An
apply repeats whatever the check warned of: `warnings` in its answer, which
the terminal prints after the result of the apply. An apply also says what
only it can know (a handler that will hear nothing where the text was
applied: *Whose an event is*). The editor lists that under the text with
what the check says.

### When a handler fails

A handler whose run fails does not fail its instance. The instance keeps
running and hears the next event. The run says `run.failed`, with its flow
(`on.<handler>`) and why. A second handler can act on it:

```yaml
on:
  trouble:
    event: run.failed
    scope: own
    do: channel.post
    with: { channel: ops, body: "${ event.data.flow } failed: ${ event.data.error }" }
```

### Chains that run away

Two handlers can answer each other for ever, and nothing in either text says
so. To stop that, every event knows how deep it is in a chain: events each
said while the one before was being handled (what a step says, what an
action it runs says, what a child it starts says as it ends).

- The hundredth event of a chain within ten seconds is not handled. That
  ends the chain.
- `handler.runaway` reports it against the instance, with the handler, the
  topic and a sentence.
- Nothing is stopped or failed: the next event from outside is handled as
  usual.
- A thousand events from one cause are not a chain. A round slower than
  that goes on for as long as it likes. A fast loop that is meant wants a
  `wait`, or a `for_each` in one run.

### Runs side by side

Runs of one handler never overlap. Two handlers of one instance do run side
by side, and each `set` adds to what the other left (*Built-in actions*,
`set`). They cannot decide something together: a `when:` reads the state as
its step began, so a handler that looks and then sets may be looking at what
another is about to change. Events that happen while the conductor is down
are handled when it returns, each once.

#### One at a time

A blueprint that says `one_at_a_time: true` at the top has handlers that
take turns: one run of any of them at a time, the next when that one has
ended. Then looking and setting is one atomic thing, whichever handlers do
it:

```yaml
blueprint: stock
one_at_a_time: true
params: { item: { type: string } }
state: { left: 3 }

on:
  wanted:
    event: stock.wanted
    for: { item: "${params.item}" }
    steps:
      sold_out:
        when: ${ state.left < 1 }
        do: emit
        with: { topic: stock.refused, order: "${event.data.order}" }

      keep: { when: ${ state.left > 0 }, do: set, with: { left: ${ state.left - 1 } } }

  returned:
    event: stock.returned
    for: { item: "${params.item}" }
    do: set
    with: { left: ${ state.left + 1 } }
```

Use it when an instance stands for something there is one of, with state
that must stay true: a count of what is left, who holds a lock, how many
workers are out.

- A handler's own `when:` is checked again when its turn comes, since what
  it waited behind may have changed what it reads.
- The cost: a handler that waits (a `wait`, a `call`, an agent's turn) holds
  up every other handler of the instance until it ends. One that waits for
  something only another of its handlers can do waits for ever. Have such a
  handler `spawn` the work and hear its end as an event.
- A text that differs from the one an instance runs only in this field is
  taken where the instance is.

#### A turn for each

An instance that looks after many things of one kind (the rows of a table,
the orders of a shop) is one instance for all of them, and none of them
should hold it up. A handler that says `turn:` names what its events take
turns by, worked out from each event as it arrives:

```yaml
blueprint: orders
on:
  paid:
    event: order.paid
    turn: "${event.data.order}"
    steps:
      pack: { call: packing, with: { order: "${event.data.order}" } }

  changed:
    event: order.changed
    turn: "${event.data.order}"
    do: record.update
    with: { id: "${event.data.order}", fields: { seen: true } }
```

- Events with the same key are handled one at a time, across every handler
  of the instance that says `turn`, and in the order they were said. While
  order 12 is being packed, the next event about order 12 waits, and an
  event about order 13 is handled beside it. (When two handlers hear of
  order 12 at the same instant, they take turns; which goes first is not
  defined.)
- So what a handler reads of one thing and then writes is atomic, whichever
  handlers do it. A handler that waits (a `call`, an agent's turn) holds up
  only events about the same thing.
- Nothing is started for this. A thing nobody has said anything about costs
  nothing, however many there are. What the instance has to keep about one
  thing it keeps on that thing's own record.
- Each event is still handled once. If the daemon stops while runs are
  under way, each is taken up again when it starts, beside the others: the
  run that was in progress for a key goes on first, and the ones that waited
  behind it start after, in the order they got their turn.
- A run that has begun is not cut off when its instance takes another text,
  however that comes about. It finishes on the text it began with, and only
  stopping its instance ends it.
- A handler that says `turn` takes no part in `one_at_a_time`, and its
  `when:` is checked once, as the event arrives.
- An event whose key comes to nothing takes turns with the others whose key
  does.

`turn` is a field of the handler and not a kind of instance, because the key
is the event's to say: the same instance may hear a row by its `id` in one
event and by its `card` in another, and may hear events about no row at
all, which take no turn.

### A limited number at once

Some work is heavy enough that only so much of it fits at once: an agent
working a card, a render, a build. `std/pool` is an instance that says how
many go at once. Whatever wants to do such work asks the pool first, and
begins when it is told it may. The rest wait their turn in the order they
asked, and are told that they are waiting and why.

```yaml
steps:
  renders:
    spawn: std/pool
    as: renders
    with: { name: renders, limit: 2 }
```

One piece of the work is a blueprint of its own whose first step takes its
place:

```yaml
blueprint: render
params: { scene: { type: string } }
steps:
  place:
    do: wait
    with:
      event: pool.granted
      where: event.data.to == instance.id
      say: { topic: pool.asked, pool: renders, about: "${params.scene}" }

  work: { needs: [place], do: shell, with: { cmd: [render, "${params.scene}"] } }
```

`say` asks once the `wait` is listening, so the answer cannot arrive before
it can be heard (*Built-in actions*, `wait`). The asker is an instance, and
the pool knows it by that. When its steps end, however they end, the pool
hears that the instance has ended and its place is free. Nothing has to give
a place back for the pool to stay correct, and no place is ever held by
something that is gone. Something long-lived that holds a place only for a
while says `pool.released` when it is done.

#### Asking by name

Something that looks after many things of one kind asks for each by a name
of its own (`as`), so nothing has to be kept waiting for a thing whose turn
has not come:
`{ topic: pool.asked, pool: renders, about: "${event.data.scene}", as: "${event.data.scene}" }`.
The place then belongs to that name.

1. Whoever waits is a name in the pool's list, nothing more.
2. When its turn comes, `pool.granted` says the name (`as`). A handler hears
   it (`for: { pool: renders }`, `turn: "${event.data.as}"`) and begins the
   work.
3. The place is given back by saying `pool.released` with the same `as`.
   Nothing else ends such a place: whoever asks for one gives it back,
   however the work went.

| Said | By | Carries |
|---|---|---|
| `pool.asked` | whatever wants a place | `pool`; `about`, what the work is about, said back to whatever shows it; `as`, where the place is to be a name's and not the asker's; and `priority`, a number, which a pool told `order: priority` goes by |
| `pool.granted` | the pool | `pool`, `to` (the instance that may begin, or `key:<name>`), `as` (the name, where it was asked by one), `waited` (whether it had to) |
| `pool.waiting` | the pool, to one that has to wait | `pool`, `project` (whose pool it is, where it is a project's), `to`, `about`, `limit`, `why` (the reason while it is paused) |
| `pool.released` | whatever gives its place back before it ends | `pool`, and `as` for a place that was a name's |
| `pool.pause`, `pool.resume` | a person, or whatever watches what the work costs | `pool`, and `why` |
| `pool.paused`, `pool.open` | the pool, to everyone | `pool`, and `why` or `limit` |
| `pool.listening` | the pool, to everyone, once it is listening: when it is first there, and each time it takes another text or another limit where it is | `pool`; `held` and `waiting`, who holds a place and who waits as it says so |

#### Asking before the pool listens

A pool hears requests only from when it is listening, like anything
(*Handlers*). A place asked for before then was asked of nobody. Something
started beside its pool, or before it, may ask too soon. So:

- The pool says when it is listening (`pool.listening`).
- Whatever asks by a name asks again, when it hears that, for whatever is
  due (`std/working` says each card of its column is due again). A name that
  is waiting is kept once however often it asks. A name that already holds
  its place by then waits for its next one, which whoever asked gives back.
- An instance that waits for its own place (`wait ... say:`) must not ask a
  second time beside its `wait`: the answer could arrive before the `wait`
  is there to hear it. Where it has handlers, one of them ends it when the
  listening pool does not have it (`held`, `waiting`), and whatever began it
  begins it again (`std/review` has its stage do so).
- Something that asks in its steps hears nothing while they run: start it
  once its pool is there.

#### Pausing, limits and order

`mw emit pool.pause pool=renders why="the bill is being looked at"`
pauses the pool: what is being done goes on, nothing new begins, and those
waiting are told why. `pool.resume` has it go on. Applied again with another
`limit`, it gives out the places there now are. Who holds a place and who
waits is its `state` (`mw instance <id>`).

A free place goes to:

| `order` | Whoever... |
|---|---|
| (default) | asked first. |
| `order: last` | asked last. |
| `order: priority` | said the highest `priority` when asking; among equals, the first to ask. One that said none said 0. |

The action `pool.turn` works out who is next, from who holds a place, who
waits, what each said and what has just happened.

A pool is only the policy, in a file: places in the order they were asked
for, decided in one handler, one event at a time (`one_at_a_time`). To give
places in another order, or to hold some back, copy `std/pool` into the
project's blueprints and edit it. Each stage of a project's fleet has one
(below, *a card is worked on in a lane*).

### Instances that keep nothing

Everything an instance does is written down as it does it. That is what lets
a restart pick it up at the step it was on. It is right for something that
stands for a thing that matters (a card's lane, a rollout, an order). It is
more than needed for short work that can simply be done again: a receipt
sent, a picture made smaller, a line counted.

```yaml
blueprint: receipt
durable: false
permissions: [shell]
params: { order: { type: string } }

steps:
  send: { do: shell, with: { cmd: [send-receipt, "${params.order}"] } }
```

An instance of a blueprint that says `durable: false`:

- **does what any instance does**: steps, tasks, loops, calls, handlers,
  hooks, and its own `state`, which it keeps in memory from one run to the
  next for as long as it lives;
- **has its runs recorded and measured**, with how each went, and says
  `run.*` and `instance.*` as any instance does, so it is seen, counted, and
  can be waited for or called;
- **records nothing for a step as it goes**: no saved place to pick a step
  up from, no place kept in a loop. What is said of a step (`step.started`,
  `step.finished`) is passed on to whoever is watching and not kept,
  whatever hears it;
- **records what each step did when the run ends**: whether it went well,
  how long it took, its outputs or why it failed, as for the steps of any
  run. This is written once, with the run's ending, however many steps
  there were. While the run is going, anyone looking at it sees how far it
  has got. A run cut off by a restart says nothing of its steps;
- **is over at a restart**: it is not picked up, what it started and what
  it held is stopped, and it says so. A run that was under way is cancelled.
  Whatever wanted the work applies it again.

Use it for what can be done twice without harm. What must happen exactly
once, or must not be lost when the daemon stops, is what the default is for.

### Who an event is for

Often many instances wait on one topic, each for its own events: there is
an instance for each card on a board, and each waits for its own card to
move. `for:` says which events are a handler's: fields of the event's data,
and the value each must have.

```yaml
on:
  moved:
    event: board.item.moved
    for: { id: "${params.item}" }        # only events whose data.id is this card's reach this handler
    when: ${ event.data.to == "done" }   # `when` stays, for anything else
    steps:
      say: { do: emit, with: { topic: card.finished, card: "${params.item}" } }
```

Every field named must have its value, compared as `==` would. A field the
event does not have matches nothing. The handler runs for the same events
it would run for with the same test in `when:`, and everything else about it
is the same: `scope`, `debounce`, one run at a time, each event once.

The difference is who decides:

- With `when:`, every handler waiting on the topic is woken for every event
  on it, and works out for itself whether it wants the event.
- With `for:`, the conductor decides who an event is for and hands it to
  those handlers only. Among a thousand cards, a move wakes one.

For that, the conductor must know what a handler waits for before any event
arrives. So:

- A value in `for:` is worked out once, when the handler starts to listen.
- It may read only what is fixed for the handler's life: `params`,
  `instance` and `target`, and plain values. `event`, `state`, `steps`,
  `self` and `item` are refused there, each with the reason. A condition on
  those goes in `when:`.
- `for:` goes with `event:` only. A timer or a poll is not addressed to
  anyone.
- If a value cannot be worked out when the time comes (`int()` of something
  that is not a number), the handler hears nothing and `handler.failed` says
  why.

#### Sending to one instance

Sending to one instance needs no field of its own. The sender names who the
event is for in the event, by convention as `to`, and the receiver says that
is its address:

```yaml
# the sender
ask:
  do: emit
  with: { topic: review.asked, to: "card-shop-12", about: the price }

# the receiver: an instance whose key is card-shop-12
on:
  asked:
    event: review.asked
    for: { to: "${instance.key}" }
    do: channel.post
    with: { channel: reviews, body: "asked about ${event.data.about}" }
```

An address is data in the event like any other. The field's name is agreed
between the two blueprints; `to` is only what the standard ones use.

### Whose an event is

A factory has many projects, and what happens in one is not the business of
what runs in another. So every event is some project's or nobody's, and a
handler or a `wait` hears by that:

1. **An event belongs to the project it names**: `project` among its data.
   Everything said of a card, a record, a table, a channel that belongs to a
   project, an agent that works in one, and a file of one names it. An event
   a blueprint says with `emit` belongs to a project if you write `project:`
   beside the topic.
2. **Otherwise, it belongs to the project of the instance that said it.**
   What a run, a step and an instance say of themselves, and an event a
   blueprint says without naming a project, belong to the project the
   instance was applied for. The project named comes first: an instance of
   one project that says an event naming another has said an event of the
   other.
3. **Otherwise, it is nobody's**: the factory's own. For example: the daemon
   starting, somebody signing in, a package installed, something said in a
   channel that is a workspace's own, an event said from outside that names
   no project.

#### Workspaces and channels

- A workspace is above projects, so what is said of one (made, renamed,
  taken away) is nobody's. So is what is said in a channel that belongs to
  a workspace.
- A project put in a workspace, when it is made or moved from another, is
  that project's news (`project.placed`).
- A channel said to belong to a project is that project's from then on
  (`channel.placed`). One made a workspace's own is nobody's again,
  including the event that says so.
- A channel always belongs to one or the other. `channel.ensure` makes one
  for the project it is told, or for the project its blueprint is applied
  for. One made for no project belongs to the factory's usual workspace.
- Told `kept: true`, `channel.ensure` makes a channel kept to its members
  ([architecture.md](architecture.md), *Identity*), with whoever applied the
  blueprint in it. Then `name` is what it is called, and another channel may
  have the same name; its `id` is what names it in every later step
  (`out.name` is the id too). Each time the step runs, it finds the channel
  again as the kept channel of that project with that name and with whoever
  applied the blueprint in it.
- A step that says nothing of `kept` finds such a channel the same way when
  no open channel has the name, so a channel somebody made private since
  stays the one.
- Where an open channel has the name, a step that asks for a kept one is
  refused: making sure of a channel never changes one that is there.

#### Hearing every project

A handler that says no `scope` hears its own project's events and what is
nobody's. An instance applied for no project has no project of its own, and
hears only what is nobody's. To hear every project's events, a handler says
so:

```yaml
on:
  any_card_done:
    event: board.item.moved
    scope: fleet                              # every project's cards, not only this instance's project's
    when: ${ event.data.to == "done" }
    do: channel.post
    with: { channel: ops, body: "${event.data.project}: ${event.data.id} is done" }
```

`for:` narrows what a handler hears and never widens it. A handler applied
for `shop` that says `for: { project: blog }` and no `scope` hears nothing:
the event is the blog's. It needs `scope: fleet` beside the `for:`. A `wait`
is the same: `scope: fleet` among its arguments hears every project's, and
with nothing said it hears as a handler does.

#### Order

- What is said of one project is heard in the order it was said, and so is
  what is nobody's.
- Between an event of a project and one of nobody's, or between two
  projects' events where a handler hears the whole fleet, no order is
  promised. A blueprint that needs one thing to have happened before another
  checks what is so (a row, what a step answered), and does not rely on
  which it heard first.
- What a service says of itself and what a router does belong to its
  instance's project, and are heard in order with everything else of that
  project.
- What is said of a sign-in an agent's turn holds (`agent.signin.*`) is
  nobody's, whichever project the agent works in.

#### The `unheard` warning

An apply warns, and goes ahead, when a handler or a `wait` will hear nothing
for these reasons. The warning has the code `unheard`, at the place it is
written:

- an instance applied for no project that listens for something that always
  belongs to some project;
- a handler that names another project in `for:`.

`mw check --project` reports the second as the text is written. With no
project given, the check does not know where the text will be applied, and
says neither. Neither can say anything of a topic or a project filled in
when the blueprint runs (`${ … }`).

#### What was running before this

An instance applied by an older program, in which a handler that said no
`scope` heard every project's events, keeps hearing that way until it is
applied again. Nothing changes under a running factory just because the
program was replaced.

- `mw instances` names the ones that still hear every project's under its
  list, and `mw instance <id>` says it of one.
- `heard` in what `/api/instances` answers is `null` for them and `project`
  for the rest.
- Applying the instance again (with `--force` where its text has not
  changed) makes it hear by project from then on. If it relied on hearing
  another project's events, add `scope: fleet` to its handler first.
- The two blueprints that ship and hear every project's say so (`std/bot`,
  `std/watch`).

## Composition

`call` and `spawn` make child instances. A child is identified by its key
within its parent: what the step says with `as:`, or the step's name.
Applying a blueprint whose instance already exists brings that instance to
what is asked, and its children are matched by key. That is how a rollout
works: apply the blueprint again.

### Children of a `for_each`

In a `for_each` there is a child for each element:

- With no `as:`, they are named for the step and their place in the list
  (`existing#0`, `existing#1`).
- With `as:`, each is named exactly what it says. `as: "card-${ item.id }"`
  is `card-shop-3` wherever that card comes in the list. So it is the same
  child when the blueprint is applied again with the list in another order,
  and the same child a handler elsewhere started under that name.
- An `as:` that does not tell the elements apart would be one child applied
  over for each of them, and is refused: when the blueprint is checked, if it
  has no `item` or `index` in it, and when the step runs, if two elements
  come to the same name.

### `call` versus `spawn`

The two ask different things of a child that is already there:

- A `spawn` says the child should be there. It is an apply of the child like
  anyone's (*How it comes to run the text now there*, below).
- A `call` asks for the child's steps to be run, and gives back their
  outputs. They are run, unless the child is already running that very text
  with those very params; then nothing runs and the caller gets what they
  output last time.

A `call` that is cut short (its run cancelled, its instance applied over,
its `timeout` run out) stops the child it made, and no other. A child that
was there before the call is left as it is, and the steps the call asked of
it go on, with nobody waiting for their outputs.

### Calling itself

A blueprint may call itself where the step says `when`. Each call is one
more level of the tree, under the one that called it, and it ends when the
step's `when` no longer holds. That is how a list is gone through one item
at a time with no loop construct (`std/through`). Its `when` says something
that is sure to stop holding.

- A call of itself with no `when`, and a `spawn` of itself, are refused when
  the blueprint is checked: neither would end.
- However a tree comes to be deep (a `when` that keeps holding, texts that
  call each other in a ring), it goes 128 levels deep at most, counted from
  the instance at its top. A step that would start a deeper child fails,
  saying which text, how deep it would be and which texts went round, and
  nothing is started.
- The deepest that ships is eleven levels, which leaves room for a list of a
  hundred and twenty items and for your own texts that nest a good deal
  more.

### When an update fails

If an update fails, the instance goes back to the text and the state it
had, stays up, and says so. A state key the failed text no longer declared
is there again, and what its steps set is as it was.

What the failed update started is stopped: a service, router or agent the
instance came to hold during it, and a child made during it. What it held
before is left running. Two things are not put back, because nothing here
knows how:

- what the failed steps did to something already there under the same name
  (a service started again with another command);
- a child that was there and was given another text or other params.

Name what an update starts by its version (`web-${ params.version }`), as
`std/slot` does, and a failed update costs nothing that was working. A first
apply that fails has nothing to go back to and is torn down.

#### What a blueprint reaches is checked with it

Applying a blueprint reads and checks every blueprint it calls, spawns or
attaches, and those they name in turn. What is wrong in one of them is
reported when you apply, not when the step runs, against the step that
reaches it:

```text
fleet.bp:9:7: error: this step spawns `lane`, whose step `go` calls `round`, which has 1 problem(s): round.bp:14:5: error: `agent.task` has no argument `options`; it takes: prompt, slot, thread. `round` may have been written for a newer version than this one, which is 0.1.0
```

#### Handing on

A blueprint that starts another often passes on most of what it was given.
Written key by key, that is a list as long as its own params, repeated
wherever the other is started. `...` in a mapping hands on a whole mapping,
and keys written beside it override it:

```yaml
call: std/lane-round
with:
  ...: ${params}                       # everything this one was handed
  back: true                           # and what is its own to say
  note: "${ event.data.note ?? '' }"
```

- What is handed on is known when the step runs. The receiver then reports
  anything it has no param or argument for, as it does for anything else.
- What is written beside `...` is checked when the blueprint is read, as
  usual.
- Something that comes to nothing hands on nothing. Anything else that is
  not a mapping is refused.

What a step passes to a blueprint (`with:`) is checked against its `params`
in the same way: a param it does not have, or a required one it is not
given. A name with a `${ }` in it is checked when the step runs. A blueprint
that is not found is a warning: a package may bring it later.

### Which text a running instance runs

#### A running instance runs a project's own blueprints as they were applied

A blueprint runs with the authority of whoever applied it. A project's own
blueprints are files that anybody who may write the project's files can
change, an agent among them. So for a name, a `call`, a `spawn` or a `uses`
gets the project's file as it stood when someone who holds what the instance
runs with last:

- *applied* it (the instance itself, or whatever it was started under), or
- *saved* the file through the daemon, which the editor in the client does.

Whoever first applies something in a project takes its blueprints as they
are at that moment.

A file changed since by anything else (a merge, an agent, an editor outside
the daemon) is run as it was. One removed since is also run as it was. One
that was not there is not taken up; if a blueprint ships under its name, that
one is run. Each time, the instance says so: the event `instance.call_kept`
(`blueprint`, `key`, `error`: why, `kept`: the revision run), and a note on
the instance itself until a call finds the file as it was vouched for.

Applying again takes up the blueprint you apply, which is what you asked
for, and leaves the others as they were, saying which have changed
(`vouched.left` in the answer, and in words in the terminal). To run them as
they are now, read what changed and say so with `--take` (`take` in the body
of the request). The answer lists what it took (`vouched.taken`). From then
on they run as whoever applied.

```sh
$EDITOR <data>/blueprints/projects/shop/review.bp   # changed outside the daemon, where the factory keeps it
mw apply fleet --project shop           # says that `review` changed, and leaves it as it was
mw apply fleet --project shop --take    # takes the project's blueprints as they now are
```

#### Blueprints that ship are held too

A blueprint that ships is not a project's file. It changes when the program
does, and what is running was not written for the new text. So an instance
goes on with each blueprint that ships **as it was when somebody applied
it**, and so does everything it starts, whatever version of the program runs
it. Upgrading the program moves nothing.

```sh
mw instances                            # which are behind, and what changed in each, in words
mw apply fleet --project shop           # says which that ship have changed, and leaves them
mw apply fleet --project shop --take    # moves this tree to what ships now
```

Every blueprint that ships says its version (`version:` after its name), and
a version has one meaning:

- A text that means something different gets the next number. A test holds
  this (`contract/versions.txt`). Re-laying a text out or adding a comment is
  not a new version. `WRITE_VERSIONS=1 cargo test -p api the_versions` gives
  a changed text its number.
- So what moved is said by version first, then in words, wherever it is
  said: under an instance that is behind, in `mw instances` and in the
  client, and in an apply's answer (`reached.left`, `reached.taken`: each
  text's `name`, the version it `was` and is `now`, and what `changed`).

```text
std/lane-round 3 to 4: the param `asking` is new; the step `work` is changed
std/prompts/review-seat.md: its words changed; the blank `${among}` is new
```

#### Linked files

What a blueprint links (the words an agent is told, `from:` of `text.put`)
ships with it and is held with it:

- A tree goes on with each such file as it reached it, and is told when the
  file changed, where it is told a blueprint did.
- `--take` moves it to the words that ship now, together with the
  blueprints, and `--back` moves it back.
- A file says no version, so it is named with a note that its words changed,
  and any blank that is new or gone, which is what the blueprint linking it
  has to fill.
- Held words that the linking text can no longer fill are not used: the
  words there now are read instead.
- A changed prompt is not a new version of the blueprint that links it. Its
  versions are its own, a line each in `contract/versions.txt` beside the
  blueprints' (and in a package's own). The file's version is the last of
  its lines, since a Markdown file has nowhere to say a number. The same
  command writes the next line when its words change.

#### Checking an apply against the texts

- Applying again without `--take` is checked against the texts the tree
  has, not the new ones. So you can keep changing your own params without
  taking anything.
- `--take` is checked against what ships now. Where the text you apply was
  not written for it, it is refused with what does not fit. When it goes
  through, it answers with what it moved to, and from then the tree runs
  those.
- Applying a blueprint that ships by its own name (`mw apply std/board`)
  moves that instance to the text now there, and what it names with it.
- A development checkout's blueprints are read where they are, so editing
  one is trying it.

#### Going back

What a tree used before it was moved is kept beside what it was moved to,
and the texts themselves are kept, so it can be put back without the old
program:

```sh
mw apply fleet --project shop --back --say   # what that would put back, by version; nothing is changed
mw apply fleet --project shop --back         # the tree is on the blueprints it had
mw apply --instance <id> --back              # the same, for one something else started
```

It is an apply, on the texts the tree had. An instance whose own text ships
goes back to the one it ran, and everything below it is brought to what it
then reaches. A file of your own that you changed for the new texts is
checked against the old ones, and refused with what does not fit: change the
file back first. Put back twice, a tree is where it was put back from.

#### How what is running comes by a blueprint that has changed is a setting of the factory

Different places want different things. Where work is being done for
somebody, nothing should move until a person says so. Where blueprints are
being written, saying so for every change gets in the way. The setting has
two parts, and only the factory's owner changes it:

```sh
mw texts                         # says both parts, and who said them
mw texts held                    # what is running goes on with what it has until somebody applies with --take
mw texts follow                  # a blueprint that changes is taken as it changes, and nobody is asked
mw texts usual                   # nothing said: as the factory started
mw texts --lands again           # a text that is taken lands by its steps being run again
mw texts --lands in-place        # or where the instance is, with nothing run again that need not be
```

| Part | Value | Meaning |
|---|---|---|
| How texts are come by | `held` | Everything above about `--take` and `--back`: a tree goes on with what it reached, and a take moves everything it reaches that ships, together. A project's own blueprints run as they were vouched for. Said for a factory run from its checkout, the checkout's blueprints are held too. |
| | `follow` | A blueprint that changes is taken as it changes by everything running that reaches it: a project's file, and the blueprints of a checkout the program runs from. Nothing is held, so there is nothing to take or go back to. A text that does not check is not taken: what runs it goes on with the last one that did, and says so once, beside itself (`instance.text_left`) and in the channel its tree reports in. |
| | `usual` | Nothing said. This is how a factory starts. What came with the program is held, so an installed factory holds. The blueprints of a checkout the program runs from are followed, so a factory run from its checkout follows them unless told `held`. A project's own run as they were vouched for either way; they are followed only where somebody said `follow`. |
| How a taken text lands | `in_place` | By the rule below: carried over, or taken where the instance is, with nothing run again that need not be. Something whose steps would have to run again goes on, behind, until it is applied itself. The default. |
| | `again` | Its steps run again, whoever asked: a fleet that takes new texts is started again. |

`follow` gives something up, and says so when it is turned on. A blueprint
runs with the authority of whoever applied it, and a project's blueprints
are files that whoever may write the project's files can change, an agent
among them.

- Held, such a file runs only as someone who holds what the instance runs
  with applied or saved it.
- Followed, it is taken as it changes. Whoever turned `follow` on has
  vouched for every version of every file, with what they held then and no
  more. An instance that runs with more goes on with its file as it was
  vouched for.
- So turning it on takes `shell` (as removing a wall does) as well as
  `admin`. It is never how an installed factory starts, and it is said as an
  event (`system.texts`, with `gives_up` in words). It is for where
  blueprints are being written.

The same setting is `GET` and `POST /api/system/texts` (`come`, `land`), and
in the client's Settings. A copy of a factory takes the setting with it.

#### An instance runs the text it was applied with, and says when that is not the one now there

Editing a file does not change what is already running from it. For each
instance, `mw instances` shows:

- the start of the revision of the text it runs;
- the `version:` that text gives itself;
- `behind`, when the text now under its blueprint's name is another, with
  why under it.

The API and the client say the same (`version`, `behind` with `revision`,
`version`, `takes` and `why`).

##### How it comes to run the text now there

This depends only on the two texts. It is the same whichever way the
instance comes to the text:

- you apply the blueprint again under the instance's name and key;
- you apply the instance again as it is (`mw apply --instance <id>`, or its
  button in the client; this is how you apply one that something else
  started, with what it was started with);
- a step spawns a child that is there.

| The two texts | What happens |
|---|---|
| **Only what it does later differs**: its `on:` handlers, `on_stop`, `one_at_a_time`, `permissions`, its `version` or a description (comments and spacing never count) | It takes the text **where it is**. None of its steps runs again, its state is kept, a run under way finishes on the text it began with, and the next time a handler fires it is the new text's. This is how an instance already at work gets a handler you added. |
| **The text says `on_update`**, whatever else changed | It is **carried over** where it is, with that list done in place of its steps (*Carrying a running instance over*, below). |
| **Its steps, params, state or anything else at the top differ**, and the text says no `on_update` | Its steps run again. |
| **`--force`** | Its steps run again, whatever the two texts are. |
| **The same text and params it already has** | Nothing happens. If its steps are still going they go on, and are not started over: applying twice, or spawning a child twice, does the steps' work once. You are given the run in progress, and `mw apply` waits for it and says how it ended. (Steps that failed are run again by applying again.) |
| **A text that does not check on this device** | It goes on with the text it has, and says what is wrong with the new one. |

Whoever applies is told which it was: `took` in the answer (`in_place`,
`carried`, `again`), and in words in the terminal and the client.

Applying an instance also looks at everything below it that its handlers
started, as whoever applied: each child that is behind takes the text now
there where it is, or is carried over, by the same rule. This look never
runs a child's steps again. A child whose steps differ stays as it is and is
shown as behind, until it is applied itself.

A text taken because of an event hears that event. When a handler spawns a
child that is already there, because something was said, and the child takes
a text that adds a handler for exactly that event, the new handler is handed
the event that brought the text. A handler the text adds otherwise listens
from when the text was taken.

A `call` asks for more: it wants the steps run. So a child that is there
with another text or other params has its steps run again, and a call never
takes a text in place or carries a child over. A child whose steps are
running that very text with those very params is waited for, and the call
gets their outputs. So a call that was under way when the daemon stopped
goes on waiting for its child when the daemon starts again, and the child
goes on from the step it was at.

### Carrying a running instance over

Running the steps again is what you want for something that is rolled out.
It is not what you want for something long-lived in the middle of its work:
a stage's steps would start it again with cards in hand. Instead, a text can
say how a **running** instance takes it. `on_update` is a list like
`on_stop`, done *in place of* the steps:

```yaml
blueprint: lane
version: 3
state:
  rounds: 0

on_update:
  - do: set
    when: ${ was.version == "2" }
    with: { rounds: ${ was.state.builds } }
```

The instance carries on from where it is, on the new text. `was` is what it
was: `was.version` (what the old text called itself, or nothing),
`was.revision`, `was.state` and `was.params`. In order:

1. Its handlers take no more events, and each ends when it is next between
   runs: **a run under way finishes on the text it began with**, and nothing
   of the instance runs beside `on_update`. Until then the instance is as it
   was. Events that happen meanwhile wait, and the new text hears them.
2. The text, the params and the authority are replaced together, as in any
   update. State is carried as any update carries it: a key the new text
   still declares keeps its value, a new key starts at what the text says,
   and a key it no longer declares is gone from `state` but still in
   `was.state`. That is how a key is renamed.
3. `on_update` runs, each entry in turn, each seeing what the one before
   `set`. An entry is an action with `with`, and may have a `when`, so one
   list can say how to come from each of several versions. The first entry
   that fails ends the list. An entry cannot ask anything of another entry,
   and none can ask a building block and act on the answer. What needs that
   is said as an event here and done by a handler; a handler that the text
   itself brings hears what its `on_update` said. If the daemon stops in the
   middle of the list, it continues from the entry it had reached when it
   starts: what the earlier entries said and set is said and set once.
4. If it failed, the instance is back on the text, the authority **and the
   state** it had, and says so, as after any failed update. If not, it is
   active, its handlers are the new text's, and they listen from where the
   old ones had got to.

What the steps did is not undone or done again. Their outputs are still what
a `call` of the instance gets back. `steps` cannot be read in `on_update`,
since no step ran. The run is one of the instance's runs, called
`on_update`.

#### When it applies

- **Only to a running instance** whose steps have settled. A first apply
  runs the steps and never `on_update`; so does an apply over an instance
  that has not settled yet.
- **Whichever way the instance comes to the text**: you apply it; a step
  spawns the child that is already there; what started it is applied again
  (it is shown as `behind`, with `takes: carried`); or
  `mw apply --instance <id>`. Not by a `call`, which asks for the steps and has them run. Not
  with `--force`, which asks for them too.
- **Whenever the text says it**, even when it differs from the last only in
  its handlers, since `on_update` may be there to fix what the instance
  holds. Written with nothing under it (`on_update: []`), there is nothing to
  do, and the steps still do not run.
- **Not when the steps are asked for**: `mw apply --force` runs them, as an
  ordinary update.
- The same text applied with other params is carried too: `params` are the
  new ones and `was.params` the old.

#### After a restart

If the daemon stops while `on_update` is running, it continues when the
daemon comes back, from the entry it had reached, with what the entries
before it set. An entry that was done is not done again. The entry it was in
the middle of may run once more, so write each entry so that running it
twice is the same as running it once (a `set` is kept with how far it had
come, and an `emit` looks first for what it said). An instance that was
still waiting for a run to finish is carried over when the daemon is back
and that run has finished: whoever asked for it was told it would be.

#### History

Every definition an instance is given is kept, in order: the text and the
params it was applied with, who applied it and when, and whether it held (an
update that failed and was undone says so).

- In the client this is the **History** of a blueprint that has been
  applied. Each entry is called by the version its text says of itself (the
  blueprint's version) and by which time the instance was applied. The same
  text applied again with other params is the same version a second time.
  The list's own count is never called a version there.
- Each entry says what it changed from the one before, can be read as it
  was, and can be applied again, which is an apply by whoever does it
  (*Permissions*, below).
- `GET /api/instances/<id>/versions` is the same list.

### Attached blueprints

`uses:` attaches other blueprints to every instance of this one. An attached
blueprint is started as a child and can watch its target with
`scope: target`:

```yaml
uses:
  logs: std/log-feed
```

or, with params: `logs: { blueprint: std/log-feed, with: { to: "nc logs.internal 5140" } }`.

### Where blueprints are found

A blueprint is looked up by name, nearest first:

1. the project's own;
2. the ones an installed extension package brings (`<package>/<name>`;
   [extensions.md](extensions.md));
3. the ones that ship as `std/...`.

#### A project's own blueprints are kept by the factory

A project's own blueprints are kept in the factory's data folder
(`<data>/blueprints/projects/<project>/`), never in the project's folder. A
project is often a repository other people read, and a blueprint you wrote
for your factory is yours, not the repository's.

- Each is named by a project path that begins `@blueprints/`. It is read,
  written, deleted and opened in the client like any file of the project:
  `@blueprints/std/board.bp` is the project's `std/board`.
- To change how something standard works for one project, fork it (below),
  or write `@blueprints/std/<name>.bp`
  (`mw file write shop @blueprints/std/board.bp < board.bp`). One from a package likewise, as
  `@blueprints/<package>/<name>.bp`.
- Each may also be called `.bp.yml` (*In an editor of your own*, below).
- A project's fleet blueprint is kept there too. A `fleet.bp` at the
  project's top, which older projects were set up with, is read regardless
  of what the project says, when the factory keeps none for it.

#### Blueprints in the project's repository

A project may also keep blueprints in a folder of its repository, to version
them with its code:

| Command | Effect |
|---|---|
| `mw project blueprints <project> tools/blueprints` | That folder, and the project's top, are read before the factory's place. A name with a file in both is read from neither until one is removed, as with two files for one name. Forks are still written where the factory keeps them. |
| `mw project blueprints <project> --none` | `--none`: keep none: nothing is read in place of what ships, and the project has no forks. |
| `mw project blueprints <project> --usual` | `--usual`: back to the factory's place alone, which is what a project that says nothing has. |
| `mw project blueprints <project> --move-in` | Copy the blueprints of the folder it names (or `--from <folder>`, or `blueprints/` where it names none), and those anywhere else in the repository by their whole path, as they were read, into the factory's place, with what each fork was forked from, and keep them there. The repository's files are left for you to remove. |

- **A factory from before**: a project reads no folder of its repository
  that the project did not name. So a project that kept its own under
  `blueprints/` with nothing said reads none of them. (A checkout of this
  program's own source keeps every blueprint that ships there, and each
  would otherwise be taken for the project's fork.) What was running that
  reached one of them is let go of it once, when this version first starts,
  and its note about it goes in that start. From its next call it gets what
  is there now.
- In the client, a project's own blueprints are listed under its Blueprints
  in the side panel, with the files each links under it. *New blueprint…*
  there, and in the gear of its page, makes one where the factory keeps
  them.
- A file of a project's repository under a name that ships is used only
  once the project is trusted (`mw project trust`). One the factory keeps is
  always used.
- Changing what a project says takes what taking a folder up takes (the
  factory's owner).
- Where texts are followed (`mw texts follow`), a change is what a save of
  the files would be: files read before are as though removed, and those
  read now as though saved. Otherwise it changes nothing that is running:
  what an instance runs of the project's own is what was vouched for when it
  was applied, until it is applied again.
- A project in the Trash keeps what it said and what the factory keeps for
  it, and gets both back. One deleted for good takes both with it. A project
  made later under its name says nothing and has nothing kept.

## Labels

A label marks what was measured, so that two ways of doing the same work
can be compared: one card given to two agents, two versions of a
deployment, two ways of handing work out. The measurements can only be told
apart if something says which group each belongs to. The blueprint that
started the work knows, and says so with `labels:` on the step that starts
each:

```yaml
steps:
  card_a: { do: board.add, with: { title: "${params.title} (a)" } }
  card_b: { do: board.add, with: { title: "${params.title} (b)" } }
  a:
    call: std/lane-round
    labels: { experiment: "cart-${params.item}", variant: a }
    with: { item: "${steps.card_a.out.id}", agent: "${params.agent_a}" }
  b:
    needs: [card_a, card_b]
    call: std/lane-round
    labels: { experiment: "cart-${params.item}", variant: b }
    with: { item: "${steps.card_b.out.id}", agent: "${params.agent_b}" }
```

`labels:` is a mapping from a label's name to its value. It goes on a step
that is a `call:`, a `spawn:` or a `task:`. Everything measured under that
step carries the labels:

- **Under a `call` or a `spawn`:** the instance it makes, and everything
  that instance does and starts. That is its steps and its runs; the runs
  of its handlers, however long afterwards; its polls; what a step names
  itself with `measure:`; the blueprints it calls, spawns and has attached
  with `uses`; and every turn an agent takes for one of its steps
  (`agent.task`, `agent.consult`, `if_fail_ask_agent`), with what the turn
  cost. The labels are kept with the instance, so they survive a restart
  and still hold for a handler that runs days after the `call` finished.
- **Under a `task`:** the steps inside it and what they start, for as long
  as the task runs.

The step that says `labels:` is not itself under them. It is a step of the
blueprint it is written in, and is measured as one.

Labels are inherited. A step further in that says `labels:` of its own adds
to the labels it is under. Where both say the same name, the one nearest
the step wins:

```yaml
steps:
  trial:
    labels: { experiment: checkout }
    task:
      old: { call: deploy, labels: { variant: v1 }, with: { version: v1 } }   # experiment=checkout variant=v1
      new: { call: deploy, labels: { variant: v2 }, with: { version: v2 } }   # experiment=checkout variant=v2
```

### Label values

A value may contain `${ }`. It reads what the step's `with:` may read,
including `item` in a `for_each`, so one step can start a variant for each
element of a list: `for_each: ${ params.agents }` with
`labels: { variant: "${item}" }`.

- The value is worked out when the step starts.
- It must come out as text or a number, of at most 100 characters.
- A list, a mapping, `true` or nothing fails the step. The error names the
  label.
- A value that comes out as empty text says nothing: the label the step was
  under, if any, stays as it was.

### Label names

A label's name is a lowercase word: letters, digits and `_`, starting with
a letter. It may not be a name that measurements already use to describe
themselves, because a query by that name could not tell which was meant:
`action`, `adapter`, `backend`, `blueprint`, `caller`, `collection`,
`db`, `decision`, `device`, `extension`, `flow`, `handler`, `kind`,
`method`, `name`, `op`, `project`, `route`, `router`, `service`,
`slot`, `status`, `step` and `topic` are taken. The standard blueprints
would use `variant` and `experiment`; any other word works as well.

`labels:` on a step that is a single `do:` is refused, because that step
starts nothing under it. Put the step in a `task:`, or put the labels on
the `call` that the step is part of.

### Labels from outside

A blueprint that is applied directly, not called, carries no labels unless
whoever applies it gives some:

```sh
mw apply trial.bp --label experiment=cart-12
```

Over HTTP, send `labels` in the body of `POST /api/apply`. Applying an
instance again gives it the labels of that apply.

### Reading them

A label is queried like anything else a measurement says:

```sh
mw metrics agent.turn --by variant --label experiment=cart-12   # each variant's turns in that experiment
mw metrics --labels                                             # every label there is, with its values
```

In the client, Measurements has **Compare by**. Choose a label, and each
measure becomes a row with a group of columns for each of the label's
values. With exactly two values it also shows the difference between them.
**where** narrows the view to one value of another label, so that two
variants of one experiment are not mixed with another experiment's.

### Comparing without writing it

`labels:` is for a blueprint you are writing. To compare ways of doing
something that a blueprint you did not write already does, start a
comparison from outside, and the daemon does the rest:

```sh
# Is the work better done by one kind of agent or another? Two roles, each
# a row of the project's `agents` with its own model; each card goes to one.
mw compare start models --project shop --of std/lane-round --per item --also std/review \
    --way one --with one.role=first \
    --way two --with two.role=second

# Or by a blueprint of your own, against the one that ships.
mw compare start rounds --project shop --of std/lane-round --per item \
    --way theirs --way mine=my-round --share mine=1 --share theirs=3

# Or what ran before against what is there now: a way held to a version.
mw compare start upgrade --project shop --of std/lane-round --per item \
    --way before=std/lane-round@3 --way now

mw compare show models                  # one screen: each way with what came of its things, side by side
mw compare end models --goes two        # from now on everything goes that way
mw compare end models                   # or as it is written again
```

From when a comparison is started, wherever a step in the project calls or
spawns the blueprint named by `--of`, the daemon starts the child one of
the ways instead.

| Flag | What it does |
|---|---|
| `--way one` | A way that says nothing is the work as it is written. |
| `--way mine=my-round` | Starts another blueprint in its place, as its text is at that moment. |
| `--with` | Hands a way something on top of what the step hands. |
| `--per` | Names the param that says which thing a start is for (a card's id, for a round). Each new thing is given the way that has had least of its share, and keeps that way however often it is started and across restarts. |
| `--also` | Names another blueprint started for the same things. It is started as written and measured under the thing's way, so that what came of the work (a review's verdict) is counted beside it. |

#### What is counted

`--count` says what is counted of what comes of each thing: an event, the
field of the event that says which thing it is about, and the fields that
say what it came to.

- `--count made=shop.made:id:how` counts each `shop.made` under the way of
  the thing its `id` names, by what its `how` says.
- Several fields are joined: `:from+to` gives `ready>doing`.
- With no fields, an event is counted by the last word of its name.

Each count is a measurement, `system.comparison.came`, with the labels
`counted` (the name you gave) and `came` (what it came to). Its value is the
time since the thing was given its way: how long a card took from being
taken up to being done is the time under `moved review>done`.

Only events you may be told of are counted, and only those of the
comparison's project. Count by fields that take one of a few values (a
column, a verdict), not by one that is different every time: each distinct
value is a line of its own.

A field is a key of the event's data, or a dotted path through it:
`--count judged=record.updated:id:chose.verdict` counts a record's change by
what it now says of `verdict`.

**The usual counts for cards.** Where a thing is a card (`--per item`) and
nothing is said, the usual counts are used (`--count cards` asks for them
by name; `--count none` counts nothing):

| Count | What it counts |
|---|---|
| `moved` | A card's moves, by where it was and where it came to: done, sent back and handed on are each a count. |
| `verdict` | What a review came to: `passed`, `failed`, or `undecided` and left for a person. |
| `asked` | An agent asking a question instead of working. |
| `left` | Work that could not be kept or merged, and a card left for a person for any reason: `unkept`, `unmerged`, `left` (the last once each time its stage says `card.left`). |
| `after` | What came after the card's work was merged, each said of it at a later card's merge by `std/merge`: its work reverted by that card (`reverted`); its files changed again by it within fourteen days (`changed_again`, where `within` of `std/merge` sets the days; said once at the later card's merge and counted for each card it names, since a count's thing that is a list counts each element); and that card saying it fixes it (`fixed`, from the later card's `fixes`). |

Each of these is counted once for the card that did it, however often it is
said. A count may say what makes an event one (`once`: the fields read as
its thing is, in `compare.start` and the route), and these say `by`.

#### Accepted things

A count may say which of its outcomes means the thing was **accepted**,
that is, the work the thing was given was taken:
`--count verdict=review.passed:card::passed` (`accepts` of a count in
`compare.start`). Where several fields are given, it is the last:
`:from+to:done` is a move that came to `done`.

The usual counts for cards say that a card is accepted when its review
passed, or when it came to the board's last column. That column is read
from the project's board when the comparison is started, and is not
followed if the board is rearranged later. With no project it is the last
column of the board that ships; for a project with no board there is none.

A thing is counted as accepted once, however often that is said of it, as
`system.comparison.came` with `counted=accepted`. `show` then lays out two
more kinds of row beside what was added up:

- `accepted`: how many of each column's things were accepted, and that
  over how many things it was given.
- `per_accepted`: each figure added up again, in all and over how many
  were accepted. What a way spent on things that were not accepted is in
  that figure: it is what the way spent for each thing that was taken.

A comparison that says nothing of accepting lays out neither. A comparison
counts; nothing in it says which way is better.

**Work that something else goes on with.** When something else carries on
with a way's work, that thing is written down as one of the way's things,
with nothing started for it (`compare.went`, for a comparison whose ways
are named by the start). What is said of it from then on is counted under
the way. A card's A/B test writes the card down this way as the winner's,
so its review passing is counted as accepted under the way whose work it
continued. What is started for the card after that (`also`, by the param
`also_per` names: `item`) is counted under that way too, and is not one
more thing given to it.

A comparison that ends still counts what was said before it ended that had
not been counted yet, and nothing said after.

#### A comparison's shares said again, and a rule that says them

This is how a gradual rollout works, as a service's canary is rolled out.

A comparison that shares its things out gives each new thing the way that
has had least of its share since the shares were last set. To set them
again:

```sh
mw compare share rounds mine=3 --why "the new round reviews well"
```

From a blueprint of the project, the action is `compare.share`. Each way is
then given its part of what comes next, whatever went before.

A rule sets the shares itself. Give it at `compare start`, or with
`compare rule <name>` on a comparison that is already going:

```sh
mw compare rule rollout --grows slate --against usual --holds verdict=failed --over 20 --step 10 --most 50
```

This gives `slate` 10 more of every hundred things, up to 50, each time
both of these hold:

- each way has had at least 20 things whose `verdict` count said something
  since the shares were last set, and
- `slate`'s came to `failed` no more often than `usual`'s.

`--at-least` asks for at least as often instead, for an outcome that
is wanted. Where `slate`'s came to it more often, the shares stay where
they are; with `--least 10` the way goes back a step instead, to no less
than 10 of every hundred.

Either way, what comes next is read afresh. Every time the shares are set
again, by the rule or by a person, it is said as
`system.comparison.shared`, with why: what was read of each way and what
that came to. A thing is read once for each time the shares are set,
however often the count says something of it, and it counts as having come
to the outcome if it did so once. `show` lays out each way's part of a
hundred, the rule with what it has read, and each time the shares were set
again.

The rule is yours: the daemon counts, and a block of the kernel's
(`kernel::compared::rolled`) says what the counts come to. A comparison
whose ways are named by the start has no shares to set.

#### A comparison whose ways are named by the start

A comparison started with `--asked`, or by `compare.start` in a blueprint,
shares nothing out. It applies only to a start whose params say
`compared: <its name>` and `way: <one of its ways>`, and gives that start
the way it names, whatever the shares.

- A start that names neither, or only one of the two, is not this
  comparison's.
- Any number of these may be in force for one blueprint, beside one that
  shares its ways out, and each may have any number of ways.
- A start that names one it cannot be given (no such comparison here, one
  that has ended, a way it does not have, or a thing that already went
  another way of it) is started as written, and its instance says why
  (`instance.call_kept`).

A blueprint starts one with `compare.start`, as whoever applied it and in
its own project. It is idempotent: asked again, it answers with the
comparison as it is, and adds any requested ways that are new (by whoever
holds what its starter held). So a text that keeps nothing can ask for it
each time it runs. `compare.end` ends one of its project's comparisons.
Nothing goes one way after one has ended.

Everything under a way carries the labels `compared` and `way`, which are
read like any label: `mw metrics agent.turn --by way,decision --label
compared=models`, or **Compare by** in the client.

#### What show lays out

`show`, like the comparison's panel under Measurements in the client, is
one screen to decide from. There is a column for each way, with how many
things it was given, and under it four kinds of row:

| Rows | What they say |
|---|---|
| came of them | What came of the things, in the order it comes about: how many, and how long after a thing was given its way. |
| chosen | What was chosen: a turn by what its agent decided. |
| added up | Whatever is not a length of time (what turns cost, the tokens they read and wrote), in all and for each thing the way was given. |
| measured | Everything else that was measured, with how many failed. |

```text
                      big                          small
                      version 3 784df2b9           version 3 784df2b9
things given          12                           12

came of them          how many            after    how many            after
moved ready>doing     12                  0ms      12                  0ms
moved doing>review    15                  4m 10s   13                  2m 05s
moved review>doing    3                   5m 30s   1                   3m 10s
moved review>done     11                  9m 40s   12                  3m 50s
verdict failed        3                   5m 30s   1                   3m 10s
verdict passed        11                  9m 40s   12                  3m 50s

chosen                how many            average  how many            average
agent.turn approve    11                  41.0s    12                  38.2s
agent.turn done       15                  2m 12s   13                  58.3s
agent.turn send_back  3                   52.4s    1                   40.1s

added up              in all              a thing  in all              a thing
agent.cost_usd        $4.12               $0.3433  $1.02               $0.0850
agent.input_tokens    1,204,000           100,333  940,100             78,342
agent.output_tokens   88,200              7,350    61,000              5,083

measured              how many            average  how many            average
run                   58                  1m 02s   52                  31.0s
step                  190 (1 failed)      3.1s     186                 1.6s
```

It does not say which is better, and nothing in it is a score: that depends
on what you were trying to find out. `--json` gives the same as the daemon
laid it out (`beside`: its `columns` and `rows`), with every series under
it (`measured`).

#### Which text a way runs

A way follows the factory's setting for texts (`mw texts`, under
*Composition*):

- **Held:** a way's text is the one from when the comparison was started.
  One that changed half way would be two ways counted as one.
- **Followed:** a way follows its file as everything else does, and what is
  counted is kept apart by the text that was run. Everything under a way
  also carries the label `text` (the start of the text's revision).
  `compare show`, like the client, gives each text of each way a column of
  its own, with the version it says, which one the way runs now, and how
  many things were given to it. What one text did is never added to what
  another did.

```text
            theirs              mine      mine
            version 2 784df2b9  1ec21650  version 2 9a0c44e1 (now)
things given  4                 2         2
```

A way may be held to a version, `<blueprint>@<version>`. It starts the
text that calls itself that version, as the factory kept it, whatever its
file has become and however texts are taken.

- A text that ships is kept for as long as a running tree was started with
  it or can be put back on it (`apply --back`). So the version a tree was
  moved from with `--take` is there to compare with the one it was moved
  to.
- Where that version is not kept, the comparison is refused, and the error
  lists the versions there are. To compare with a text that is not kept,
  save it among the project's own blueprints (`blueprints/`, which the
  factory keeps as `@blueprints/`) under a name of its own, and name that
  as the way.
- Such a way starts that one text. What the text calls by name is what the
  running tree is held to now.

#### Two prompts for one step

What an agent is told is a file its blueprint links. Whether other words
would do better is a comparison whose ways differ in nothing else.
`--prompt <way>.<file>=<other file>` has a way read another file where one
is linked, each named as it is in the library. To try other words for
whoever reviews a project's cards:

```sh
# 1. The other words, a file of the project's own under a name of its own.
#    It may have the blanks the one that ships has, and no others:
#    ${item}, ${title}, ${body}, ${changed}, ${where}, ${among}.
$EDITOR strict.md
mw file write shop @blueprints/prompts/strict.md < strict.md

# 2. Each card's review goes one of two ways; one reads the other words.
mw compare start prompts --project shop --of std/review --per item \
    --way usual \
    --way strict --prompt strict.std/prompts/review-seat.md=prompts/strict.md

# 3. Work goes on as it does. Then, side by side: how many reviews each way
#    was given, what they came to and how long after, what the reviewers
#    chose, what it cost.
mw compare show prompts

# 4. Yours to say. From now on every review reads the other words...
mw compare end prompts --goes strict
#    ...or the ones that ship again.
mw compare end prompts
```

```text
                            usual               strict
                            version 6 0c1d2e3f  version 6 5a6b7c8d
std/prompts/review-seat.md  as it is linked     prompts/strict.md 91f0aa27
things given                14                  14

came of them                how many  after     how many  after
verdict failed              2         1m 10s    6         1m 25s
verdict passed              12        58.0s     8         1m 02s

chosen                      how many  average   how many  average
agent.turn approve          12        40.2s     8         44.9s
agent.turn send_back        2         51.0s     6         47.3s
```

The comparison is refused when it is started where:

- the other file is not there;
- nothing the comparison starts links the file it stands in for (the error
  says what they do link); or
- the other file has a blank the step hands nothing for.

Which words a thing reads is settled when it is started under its way, for
everything started for it. Where texts are held, the words are the ones from
when the comparison was started. Where they are followed (`mw texts
follow`), it is the file as it is then, and words that changed half way get
a column of their own, as a way's text does. To keep the words for good,
end with `--goes`, or copy them over the project's own
`@blueprints/std/prompts/review-seat.md`.

Ending with `--goes` lets a blueprint of your own stand in for one that
ships for as long as you like, without the project having a file under the
shipped name. End a comparison naming the way; end it again with no way to
have everything run as written.

### An example, put into a project

The program comes with example blueprints to read and to try. They are no
part of what runs until somebody adds one:

```sh
mw example list                               # which there are, what each is, and the blueprints it brings
mw example add research --project shop        # its files, written among the project's own as @blueprints/research/
```

From then on they are files of the project like any other, vouched for by
whoever put them there: applied by their names (`research/ask`), and read
and changed in the client or with `mw file`. Adding it again writes nothing
that is already there as it came. Where one of its files says something
else, nothing at all is written: those words are somebody's.
[first-run.md](first-run.md) tries one, in step 9.

### Changing what a blueprint that ships does

A blueprint that ships is the default. There are three ways to change what
it does; reach for them in this order. Its Summary lists, last, which of
them it has (`mw explain std/fleet`, or the Summary on its page in the
client, where each is a line you can press):

1. **Tell it something else.** Its params are what whoever applies it may
   tell it. Apply it again with other values
   (`mw apply std/board --project shop --param 'columns=["todo","doing","done"]'`,
   or Apply on its page). Nothing is copied. It is still the default, and
   takes what a new version of the program brings.
2. **Change a row.** A blueprint that makes a table whose rows set what it
   does works from those rows. Who is in a fleet, and what is done with a
   card in each column, are rows of `agents`, `stages` and `segments`; who
   is told what waits is a row of `tells`. Change them in the client or
   with `mw record set`, and the fleet follows. It is still the default.
   Such a table is one that sits beside another (`beside` of
   `collection.ensure`, which the action reports about itself as `setting`
   in the contract). A board's own rows are its cards, and changing them
   changes nothing the board does, so its Summary offers no row.
3. **Fork it.** Its text is copied into the project. The copy is what runs
   there under the name, and is the project's to change:

```sh
mw own std/lane-round --project shop          # forks it: copies it for the project, as @blueprints/std/lane-round.bp
mw explain std/lane-round --project shop      # the fork in plain words: what it was forked from, where it differs
mw own std/lane-round --project shop --say    # where the fork and the one that ships now differ, and the same parts
mw own std/lane-round --project shop --back   # takes the fork away: the one that ships runs again
```

In the client, Fork is in the head of a blueprint that ships, beside
Apply, for whoever may write the project's files. The fork is made and
opened in the editor. On a page opened from the factory's list, it asks
which project.

What is copied is the text that what is running in the project goes on
with, where it is held to one; otherwise it is the one that ships now. The
command says which version. From then on the fork is a file of the
project's like any other:

- What runs there reads it in place of the one that ships, as whoever
  applies or saves it vouches for it.
- It no longer changes when a new version of the program brings another
  text under its name.
- It is kept by the factory (`@blueprints/`), so it is used whether or not
  the project is trusted. A project made of a folder that was already
  there has to be trusted (`mw project trust`) only for a file of its own
  folder to stand in for one that ships.

#### There is a record of the default and a record of the fork

The default is in the program, listed with every blueprint that ships.

For the fork, the daemon records what it was forked from when it is made:
which blueprint, the version, that text to the letter, and who forked it
and when. This record:

- is kept for as long as the fork is, whatever version of the program is
  installed by then;
- goes into a copy of the factory, and with a project moved to a journal
  of its own;
- is forgotten when the fork is taken away.

Both are announced as events (`project.forked`, `project.unforked`). The
fork's own page is the rest of its record: its text, and under History
what was applied when and by whom.

A fork's Summary says, in this order:

1. **What it was forked from**, first of all:
   `Forked from std/lane-round version 6 by Ada on 2026-11-20.` In the
   client the line opens the default.
2. **Where it differs from the default it was forked from:** each param,
   step and handler that was added, removed or changed, and under a handler
   or a task the steps in it that were. In the client a line shows that
   place in the fork's text. A fork nobody has changed says it is the
   default as it was forked.
3. **That the default has moved since**, only where it has: the version
   that ships now beside the one it was forked from, what changed since
   (in the same words), and that the fork takes none of it by itself.
   Nothing is merged: the text is the project's to change, with what moved
   beside it.
4. **How to go back to the default**, last: the fork is taken away, and the
   one that ships runs again once what is running is applied. `--back` is
   refused, with nothing taken away, for a fork that no longer says what
   ships. `--back --force` takes it away anyway, and what it said is then
   gone unless the project keeps it somewhere else. In the client this is
   behind a question that says which of the two it will be.

The Summary of the default, viewed for a project that has a fork of it,
says first that the project runs its own fork.

A fork made by an earlier version of the program has no record of where it
came from, and none is made up for it. It says it was forked from the
blueprint; it gives the version only where its own text names one that the
default had; and it says that by whom and when is not known.

### Measurements of your own

The system measures how its own parts went: a step, a run, a turn. What
only your program or your blueprint knows (that a checkout was completed,
what an order came to) is reported by them, under a name of their own. It
is kept and compared the same way.

**From a service.** A service reports it in what it prints, whatever
language it is written in. A line that begins `::measure` is a measurement,
and one that begins `::event` is an event (*Events*, below). Each is
followed by a JSON mapping:

```text
::measure {"name": "checkout.done", "value": 812, "ok": true, "labels": {"page": "cart"}}
::event {"topic": "shop.paid", "order": "o-17", "total": 40}
```

- `value` is a number (how long, how much; 0 if left out).
- `ok` says whether it went well (true if left out).
- It is kept with the `service` that said it and with the labels of the
  step that started the service. So two versions started under
  `variant: a` and `variant: b` are compared with `mw metrics
  checkout.done --by variant`, whatever each says of itself.
- `labels` in the line add to those.
- A line that is wrong is not taken, and the service's output says why.

**From a blueprint.** A blueprint reports it with `measure`. This is also
how an event said from outside becomes a measurement:

```yaml
on:
  paid:
    event: shop.paid
    do: measure
    with: { name: shop.order, value: ${ event.data.total }, labels: { plan: "${ event.data.plan }" } }
```

Neither may use a name the system measures under (`step`, `run`,
`service.health`, ...). Use two or more words of your own, such as
`checkout.done`.

**Units.** A measurement is a length of time, in milliseconds, unless how
its name ends says what it is of:

| Name ends in | Is |
|---|---|
| `_usd` | an amount of money |
| `_tokens` | tokens |
| `_bytes` | bytes |
| `_mb` | megabytes |
| `_cpu` | load per processor (`device.load_per_cpu`) |
| `_count` | a count |

For example `shop.order_usd`, `shop.read_tokens`. Such a measurement is
written as what it is wherever it is shown, `mw metrics` gives its total,
and a comparison adds it up under each way and gives it for each thing the
way was given. That is how what an agent's turn cost (`agent.cost_usd`)
and what it read and wrote (`agent.input_tokens`, `agent.output_tokens`,
where its program reports them) are read. Nothing that shows measurements
keeps a list of names.

**Not carried yet.** These do not carry the labels of the step that started
them:

- a service's health checks and a router's connections (they know their
  project and nothing else of the instance that started them);
- requests to a store;
- a turn an agent takes because of a message a labelled blueprint posted
  in a channel;
- a step run on another device with `device:`.

## Expressions

`${ ... }` may appear in any string value. A string that is exactly one
expression keeps its type, so `port: ${params.port}` is a number.

Names an expression can start with:

| Name | Is |
|---|---|
| `params` | What the instance was started with. |
| `state` | The instance's durable properties. Change them with `do: set`. |
| `steps.<id>` | An earlier step: `.ok`, `.failed`, `.skipped`, `.out`, `.count` (how many things `.out` is, where it is a list, as a `for_each` step's is), `.error`, `.reason`, `.duration_ms`, `.attempts`. |
| `self` | In a hook or watch rule, the step it belongs to. |
| `event` | In a handler, what fired it: `.topic`, `.data`, `.source`, and `.seq`, its number, which no other event in the factory has (`as: "review-${event.data.card}-${event.seq}"` names a child after what asked for it). Of two events of one project the later has the greater; the numbers have gaps, and say nothing of the order of two projects' events (*Events*, *An event's number*). |
| `was` | In `on_update`, the instance as it was before it took this text: `.version`, `.revision`, `.state`, `.params`. |
| `run` | In the blueprint's own `on_success`, `on_fail` and `on_finish`, the run of its steps: `.status`, `.error`, `.id`. |
| `item`, `index` | In a `for_each` step. |
| `instance` | `.id`, `.name`, `.key`, `.project`, `.device`. |
| `target` | The instance this one is attached to. |
| `profile` | The profile of the factory this runs in, the copy of it (`prod`, `dev1`): `.name`, and `.vars.<name>`, what its owner has said is true of it. |
| `environment` | The environment of its project's that the instance was applied under (dev, staging, prod): `.name`, and `.vars.<name>`. Both are empty for an instance applied under no environment. |

### What follows a name is checked

What follows each of these names is checked against what it has when the
text is checked:

- A name that `event`, `instance`, `target`, `self` or a step does not have
  (`event.dat`, `steps.build.output`) is refused, with the nearest name it
  does have. Without this check it would read as nothing, silently.
- What is read of a step's outputs is checked against the step. For a task,
  against the steps it has. For an action, against the outputs the action
  says it gives, as a warning only (`steps.build.out.stdot`: `shell` does
  not say it gives `stdot`), since an action may give more than it says.
- What an event carries (`event.data.<name>`) is checked against the event,
  as *Handlers* says.

### `profile` and `environment`

**`profile.name`** is what the factory was started as
(`mw up --profile prod`), as text. It is the empty text for a factory that
was given no name. It describes where a blueprint runs, not what it was
started with, so the same file can behave differently in each profile
without a param for it:

```yaml
steps:
  let_loose:
    when: ${ profile.name != "prod" }
    do: agent.task
    with: { slot: builder, prompt: "Find out why the build failed, and fix it." }
  review:
    do: agent.task
    with: { slot: "${ profile.name == 'prod' ? 'careful' : 'builder' }", prompt: "Review the change." }
```

**`profile.vars.<name>`** is a **variable** of the profile: something the
factory's owner has said is true of it (where a service answers, how many
of something to run), so that the same file reads the right value in
`prod` and in `dev1`.

- It is text, a number, or true or false.
- It is nothing (`null`) if it was not set, so use `??` to give a
  fallback.
- `profile.vars` with nothing set is an empty map.

```yaml
steps:
  migrate:
    do: shell
    with:
      run: ./migrate
      env: { API: "${ profile.vars.api ?? 'http://localhost:8080' }" }
```

The variables are not for secrets: they are shown, stored as they are, and
may appear in events. They are set with `std/profile` (*Built-in actions*,
below).

`profile` by itself is a map of those two parts, `name` and `vars`. The
checker does not check what follows `vars` (those names are yours),
and refuses any other part (`profile.region`), saying so.
[profiles.md](profiles.md) explains what a profile is.

**`environment`** is a map of the same two parts, for something else: the
environment of its project's that the instance was applied under (`dev`,
`stage`, `prod`), which a project defines for itself. For an instance
applied under no environment, `environment.name` is the empty text and
`environment.vars` an empty map.

In texts written before 0.1.0, `environment` meant what `profile` means
now; the release's notes say what such a text reads. `mw check` warns of a
text that sets the profile's variables and reads `environment`
(`environment-renamed`).

### Operators and functions

Operators: `!`, `-`, `* / %`, `+ -`, `== != < <= > >=`, `in`, `&&`, `||`,
`??` (the left side unless it is null), `a ? b : c`. A path that leads
nowhere is `null`, not an error. `+` joins text and lists.

Functions: `len`, `contains`, `starts_with`, `ends_with`, `lower`, `upper`,
`trim`, `string`, `int`, `float`, `json`, `keys`, `join`, `split`, `replace`,
`matches`, `slug`, `min`, `max`, `where`, `sort`, `without`, `range`, and
`pluck`, `at`, `sum` and `from_json`, described below.

**`matches(text, pattern)`** says whether a pattern is found in some text.
The pattern is a regular expression, as a `log:` watch rule's is. It is
looked for anywhere in the text unless it says where: `^` is the start and
`$` the end, so `matches(params.to, "^[a-z0-9.:/-]+$")` holds for text made
of those characters only. A pattern written out that does not parse is an
error when the blueprint is checked.

**`where(list, field, value)`** is the mappings of a list whose `field` is
that value. **`sort(list, field)`** is the list with the smallest `field`
first (`sort(list)` for a list of plain values). The sort is stable:
mappings that are equal keep their order, so sorting by one field and then
by another orders by the second and, within it, by the first. The first of
what is left is `[0]`, which is `null` when the list is empty:
`sort(where(params.signins, "free", true), "last_used")[0].name` is the
name of the free sign-in that was used longest ago.

**`without(list, value)`** is the list with that value left out, wherever
it stood. `without(list, other)` leaves out every value of a second list.
What is left keeps its order: `without(state.waiting, state.held)[0]` is
the first that waits and holds nothing.

**`range(n)`** is the whole numbers from 1 to `n` as a list, up to 1000 at
most. To do something so many times, a `for_each` needs no list: handed the
number itself (`for_each: ${params.count}`), it counts from 1 to it, by the
same rule.

**`pluck(list, field)`** is that field of each mapping of a list
(`pluck(steps.added.out, "id")`: the ids of what a loop added).
**`at(list, places)`** is the elements at those places, counted from 1, in
the order asked. A mapping with no such field, and a place where nothing
is, are left out.

**`sum(list)`** is the numbers of a list added up, and 0 for an empty one:
`sum(pluck(steps.roles.out, "agents"))`.

**`from_json(text)`** is what JSON text says, for something an agent or a
program wrote as text. Text that is not JSON fails the step, saying where
it stops being JSON.

There are no loops, assignments or user functions, and an expression always
terminates. Anything that needs more belongs in an action.

### Words that may be taken out after the preview

Nothing that ships uses these (no standard blueprint, example or package).
They may be removed once the preview is out, so do not build on them:

- The functions `ends_with`, `upper`, `int`, `float`, `keys`, `min`,
  `max`, `where`, `sort`, `without`, `range`, `pluck`, `at`, `from_json`
  and `sum`.
- The operators `-`, `*`, `/` and `%`.
- Four fields: `max_parallel` of a blueprint, `if_fail_ask_agent` of a
  step, `debounce` of a handler, and `max_backoff` of `retry`.

They are read and run today as described here. Everything else of the
language stays: from the preview on, nothing is removed from it, since a
factory that is running goes on with the texts it was applied with. A
handler's `poll` stays even though nothing that ships uses it yet: it is
how a factory that cannot be reached from outside hears of changes
somewhere else.

## Built-in actions

These actions are part of the language itself. Every other action is
registered by the device (see the end of this section).

| Action | Does |
|---|---|
| `wait` | `for: 5m` pauses. `event: <topic>` (with optional `where:` and `timeout:`) waits for an event and returns `timed_out` and `event`. It hears its own project's events and the factory's own, as a handler with no `scope` does; `scope: fleet` hears every project's (*Whose an event is*). A wait survives a restart and still ends when it was going to. With `say: { topic: ..., ... }` it publishes that event once it is listening, as `emit` would. Use it to ask something and wait for the answer: an event published by an earlier step could be answered before the wait began to listen. The event is published once, however the wait is interrupted. |
| `emit` | Publishes an event: `topic`, plus any data. |
| `set` | Changes the instance's `state`. The new value is worked out from the state at the moment the `set` runs, and an instance's `set`s run one at a time, so `n: ${ state.n + 1 }` in two handlers, or in two steps side by side, counts both. Outputs what it set. |
| `keep` | Adds an entry to a list of the instance's `state` (`in`), once. `what` is the entry: a word, a number or a mapping. An equal entry is replaced, not added again, so keeping it twice leaves one entry, with what it now says. Mapping entries are equal when a field the step names is equal (`by: item`); with no `by`, when the whole entry is. Outputs `kept`, how many entries the list then holds, and `was`, whether an equal entry was there. Use it for a list of work in hand kept by handlers that run side by side: each changes its own entry and none removes another's, which setting the whole list from what one handler read would do. |
| `drop` | Removes from a list (`from`) every entry that `is` the value given: the entry itself or, with `by`, the entry whose field of that name has that value. Nothing to remove is not a failure. An entry that is not a mapping is compared whole, whatever `by` says. Outputs `dropped` and `left`. |
| `measure` | Keeps a measurement of your own: `name`, and optionally `value` (a number), `ok` and `labels`. Not a name the system measures under. See *Labels*, *Measurements of your own*. |
| `end` | Stops the run here: `status: ok` (the default) or `failed`, and a `reason`. |
| `fail` | Fails the step, with a `message`. |
| `log` | Writes a line to the run's log. |
| `stop` | Stops an instance (this one, or a child by key). With `missing_ok: true` and a child's key, nothing alive under that key is not a failure, and `stopped` says whether anything was stopped. |
| `noop` | Does nothing. |

### Records and tables

A blueprint can give a project **records of its own**. `collection.ensure`
declares a collection (a table) with its `fields`, its `views` and its
`store`:

```yaml
steps:
  sprints:
    do: collection.ensure
    with:
      name: sprints
      fields:
        title: { type: text, required: true }
        status: { type: choice, of: [planned, running, done] }
        size: { type: number }
        starts: { type: date }
      views:
        rows: { kind: table, hides: [size] }
        by_status: { kind: board, by: status }
```

#### Field types

| Type | Holds |
|---|---|
| `text` | words |
| `choice` | one of the words its `of` lists |
| `actor` | somebody: a person or an agent |
| `list` | several words |
| `number` | a number |
| `flag` | yes or no |
| `date` | a day, written `2026-11-30` |
| `record` | the id of one record of another collection (*Fields that name records*, below) |
| `records` | the ids of several records of one collection, kept as a list: `spans: { type: records, of: years }`. Each id must be a record that exists when it is named. |

#### Views

A collection's `views` are the ways it is laid out to begin with. Every
view shows the same records; whoever looks at the table saves more views.
A view's `kind` is one of:

- `table`;
- `board`, with `by:` the choice whose values are its columns;
- `list`;
- `gallery`;
- `calendar`, with `by:` the date that puts each record on a day;
- `order`, which lays records out by what comes first (below).

What a view may also say:

| Option | What it does |
|---|---|
| `hides` | The fields it leaves out: `rows: { kind: table, hides: [notes] }`. |
| `group` | (`board`, `order`, `table`) A field the records are gathered by, a band for each thing it names; in a table, a line across it with its rows under it. A field that holds several things (a `list`, `records`) puts a record in the band of each: `group: labels`. |
| `columns` | Its columns in the order they stand: `columns: [status, due]`. A column it does not name stands after those it names, where it always stood among the rest, so a column added later is at the end. A name that is no column is ignored. What a record is known by comes first whatever is said; in a table that says `groups`, the order applies inside each group. |
| `freezes` | (`table`) The column it is frozen up to: that one and those before it stay while the rest scroll sideways: `freezes: title`. |
| `figures` | (`table`) A figure at the foot of a column, over the rows the view shows: `figures: { points: sum, done: ticked }`. |
| `widths` | (`table`) How wide some columns are: a map from a column to whole pixels at the usual text size, 60 to 2000, as in `widths: { title: 320, done: 70 }`. |
| `only` | (any kind) What the view is held to: a map from a field to the things a record must hold one of to be shown, or, for a field that holds one thing, the values it must say one of. With two fields named, a record must match both: `urgent: { kind: board, by: status, only: { labels: [urgent, soon] } }`. |

**An `order`** lays records out by what comes first. `by:` is the field in
which a record lists the records that come before it (a `list` of their
ids, or a `records` of its own collection). `done:` is the choice whose
last value means a record is done; where none is said, the collection's
first choice. Its columns are what can be begun now, then what waits only
on those, and so on. What is done is folded away, and records that wait
on each other in a ring are set apart. `std/board` gives a board this as
its second view:

```yaml
steps: { kind: order, by: after, done: status, group: part_of }
```

**Figures.** A figure counts; it never scores.

| Figure | For |
|---|---|
| `filled`, `empty` | any column: how many rows say something there, and how many nothing |
| `sum`, `mean`, `least`, `greatest` | a `number` |
| `ticked` | a `flag` |
| `different` | anything that is neither: how many different things are said |

`freezes` and `figures` must name columns the table has, and a figure must
be one its column's kind can show; anything else is refused where the view
is declared or saved.

**Widths.** A page draws a width as that many times the reader's text size,
so a column holds as much for everybody. A column with no width is as wide
as its content. A width outside the range is refused; a width of a column
the table does not have is ignored, as in `columns`. On a phone's layout
widths are not drawn.

In the client, all of this is done from a column's head: pressed for a
menu, dragged to another place, and made wider or narrower by the edge at
its right. What is done is saved with the view in these same words.

#### Where records are kept

`store` says where the records are kept:

| `store` | Records are kept |
|---|---|
| `local` | in this installation's database |
| `files` | one file per record, in the project |
| `service` | in a database another blueprint made and hosts, at the address given as `at`. `std/store` is such a blueprint; see [stores.md](stores.md). |
| `documents` | as the Markdown files of a folder of the project, the folder given as `at`. `std/docs` is such a blueprint; see [library.md](library.md), which also covers `library.export`, the action that writes a library out as web pages for any host (`std/site` runs it each time a document changes). |
| `mockups` | as the folders of a project folder that each hold an `index.html`: pages made to try an idea. `std/mockups` is such a blueprint; see [mockups.md](mockups.md). |

Where a collection is kept is settled when it is made. The address of a
service and the folder of a library may be said again. The instance that
declared the collection holds it.

#### Who may change a table: `takes` and `steers`

- **`takes`** on a collection is a scope that changing its records needs,
  on top of what changing any record needs. Use it for a table that
  something with more authority acts on: a fleet's `agents` says
  `agents:write`. Only whoever holds that scope changes the records, or
  declares the collection again.
- **`steers`** names another collection of the project whose records this
  one's records steer: a fleet's `stages` says `agents`. What those records
  do, and for whom, follows what a record of this one says, with the
  authority of whatever reads the two. So a record of the steering table
  is added, or changed in any cell (on a board that says it, a card
  moved), only by whoever could write every record of the steered
  collection as it stands: each scope its cells need, and each scope a
  `takes: each` list of it gives. Nobody gets something that holds more
  than they do to act for them by writing beside it.
  - Removing a record needs no more than changing the table does.
  - A collection that does not exist has no records to steer.
  - Declaring the table again so that it steers nothing, or something
    else, needs `admin`.

#### How a table is shown: `beside`, `about`, `groups`, `by_hand`

These change only how the client shows a table. They let nobody in and
keep nobody out, nothing the factory does follows them, and saying them
(or removing them again) needs no more than declaring the collection.

**`beside`** names another collection of the project that this one sits
beside (a fleet's three tables say `board`). With it, **`about`** names the
field that says which thing of that collection a row is about: a fleet's
`stages` say `column`, so a row is about a column of the board. A row that
leaves the field empty is about whatever it is called. In the client such
a table has no line of its own under its project. The page of the table it
sits beside lists it (under **Its tables**, in the gear at the right of its
head), opens it beside itself, and shows at the head of each column the
rows that are about that column.

- One that names a collection the project does not have sits beside
  nothing and is shown as any table is.
- Naming itself, a name that is not a collection name as one is written
  (another project's), or a field it does not have is refused when the
  blueprint is applied.
- `beside` is refused on a board, a library or mockups, which each have a
  page of their own kind under their project. Only a table of plain
  records sits beside another.

**`groups`** puts the columns under headings. It is a list of headings in
the order they are drawn, each a mapping from the heading to its fields in
order:

```yaml
groups: [{ "Who they are": [role, count] }, { "What they are told": [told] }]
```

A collection's fields are a map, which keeps no order, so the order of the
groups and of the columns in each is said by this list and not by a word
on each field. The table is drawn with a band of those headings over its
column heads, each group's columns kept together under its heading, and a
record's form has a section for each. A field in no group is drawn before
the groups, and so is what a record is known by. A field named in two
groups, or one the collection does not have, is refused.

**`by_hand: false`** is for a table whose rows are written only by the
commands and blueprints that keep it (a row for each heavy run of a
script, or for each deploy). The client then offers no new row, no cell or
form field to change, no column to add, retype, rename or remove, and no
CSV file to read in. It says in a few faint words at the table's head that
its rows are written by what keeps it. Everything about how somebody looks
at the table stays: sorting, holding a view to some rows, grouping, a
figure at a column's foot, freezing, widths, hiding a column, saved views.
So do opening a row to read it, commenting on it and moving it to the
Trash. Nothing else is refused: commands and blueprints add and change its
records through the same doors as every table's. Only a table of plain
records may say it; a board, a library and mockups are refused it.

#### Declaring a table again

A collection that has records, declared again with other fields, becomes
what it now says only if those records can still be written:

- A field no longer declared is no longer read. What a record held of it
  is gone for good the next time that record is written.
- A field given another `type` is refused while records hold values that
  would not fit (`"big"` where a number is now wanted).
- A field newly `required` is refused while records lack it.

Each refusal says how many records and what to do first.

#### Models

A collection may say it is a **model**: `model: card`. A model is a record
the daemon itself reads field by field. The models are `card` (the board
moves a card by its `status`), `action` (what a column offers to be
pressed on a card, *A button on a card*) and `topic`.

- A collection that is a model's writes the model's fields among its own.
- A field of the model that is declared must be what the model says it is
  (its `type`, and whether it is `required`). Otherwise the blueprint is
  refused when it is applied, with the field, what the model has it as and
  what was declared.
- What a choice can be is still the blueprint's: a card's `status` is a
  choice, and `of` what (the board's columns) is said in the blueprint.
- A field of the model that is left out and that a record can do without
  (a card's `due`) is there all the same, so an older copy of a board's
  blueprint still applies and its cards have what cards now have. A field
  a record cannot do without (`title`, `status`) must be declared.
- Fields beyond the model's are the blueprint's own.
- The collection called `board` is a card's model whether or not it says
  so.

A card's fields:

| Field | Type |
|---|---|
| `title` | text, required |
| `body` | text |
| `status` | choice, required |
| `assignee` | actor |
| `labels` | list |
| `due` | date |
| `moved` | list: how the card came to be where it is. The board writes it with the move that puts the card there, and nobody else writes it: the column it was moved from, the kind of actor that moved it, what was said as it was moved, and how many times it has been moved. |
| `pressed` | list: what was pressed on it of what its column offers. The board writes it with the press, and nobody else writes it. |

`std/board` writes them all out, so that it states the whole of what a
board is made of, and a test holds every blueprint that ships to that. It
declares more fields of its own, which are no part of the model:
`question` (what is asked on a card and not yet answered), and what is kept
on a card by whatever looks after it (`std/cards` and a fleet's stages):
who asked, what it waits for, how often it was sent back, where it was
taken from to be worked, and what its agent is to be told next.

#### Field options

| Option | Example | What it does |
|---|---|---|
| **`quiet`** | `question: { type: text, quiet: true }` | A change to this field alone is not a line of the record's history. |
| **`hidden`** | `taken: { type: text, quiet: true, hidden: true }` | Left out of a view of the collection to begin with. |
| **`said`** | `sprint: { type: text, said: true }` | Every event about a record carries the field's value. |
| **`suggests`** | `does: { type: text, suggests: [plan, work, review] }` | Words offered to whoever fills the field. |
| **`icon`** | `due: { type: date, icon: calendar-days }` | The icon its column is drawn with. |
| **`indexed`** | `title: { type: text, indexed: true }` | Keeps the column in order by an index from the first row. |

**`quiet`** is for a field a blueprint sets to show something its own
event has already said (that a question was asked, then answered), so that
the history does not say it a second time as "changed its question". The
change is still an event, and whatever draws the record still hears it.

**`hidden`** is for what a blueprint keeps on a record for itself, which a
reader seldom wants as a column (`std/board` says it of who asked a card's
question, where a card was taken from, and what its agent is to be told
next). It is not privacy: anyone looking at the table can show the column
from the view's options, a record opened shows every field, and so does
any command or request. A view follows `hidden` until it says for itself
which fields it hides (`hides`), which it does from the moment somebody
shows or hides a column in it. A view saved before views could say this
follows `hidden` too, since nobody who saved one chose to see what its
table has come to keep since.

**`said`** puts the field's value into every event about a record of the
table, under `fields` of the event's data (`fields: { sprint: autumn }`):
`record.created`, `record.updated`, `record.removed`,
`record.commented`, what is attached, and for a board
`board.item.created`, `board.item.moved`, `board.item.updated`,
`board.item.removed` and what is pressed on a card. It is under `fields`
so that nothing a blueprint adds can come to mean what the event already
says.

- A handler can then be chosen by the field with nothing run: by `for:`,
  which names a field inside one by its path
  (`for: { project: shop, fields.sprint: autumn }`), or by a plain `when`
  (`event.data.fields.sprint == "spring"`). Whatever follows the events
  from outside has it too, with no rebuild for a field a blueprint adds.
- Whoever hears the event hears the field: an event of a private table
  needs what reading the table needs.
- A field that is `quiet`, or says what giving it a value `takes`, is
  refused `said`.
- A text longer than a kilobyte is cut, ending in `…`; anything else that
  long (a long list) is left out. An event is no copy of a document.
- A table that says it of no field sends events as before, with no
  `fields`.
- A document's file being removed says nothing of its fields, since it is
  gone when that is said.

**`suggests`** is for a `text` field with a few usual words that the
blueprint's author knows and whoever fills it may not (`std/fleet` says it
of what a stage does with a card, and of a role). The field takes any word;
the client offers these and the user may write another. Nothing checks
them, when a record is written or when the collection is declared again
with others. A field held to its words is a `choice`, which says them with
`of`.

**`icon`** names the icon: lowercase letters, digits and dashes, 48 at
most. The client has drawings for the names of the Lucide set it is built
with, less any that names a package or what a package is for. A name it
has no drawing of is kept and drawn as the mark of the field's kind, so
nothing fails on a client with another set. Whoever may save a view of the
collection may choose another icon in the client. That choice stands over
the declared one for everybody and is kept when the collection is declared
again, as a saved view is; removed there, the declared icon is drawn again.

**`indexed`** makes a table of many rows sort and filter by the column at
once. A table is read a window at a time
(`GET /api/collections/{project}/{name}/rows`, with what is counted of a
view at `…/rows/counted`). The daemon indexes any column of a table of
10,000 rows or more the first time the table is sorted or filtered by it.
That holds the project's journal for about a second and a half per million
rows, once, and is announced (`collection.indexed`). `indexed` says it of
a column before anybody asks (`std/run` says it of when a run kept by a row
of `segments` began). At most eight columns of a table are indexed, so a
declaration says it of eight at most, and never of a field that holds
several things (a `list`, `records`), which no index keeps in order.

#### What a declaration means once people look at the table

The fields a blueprint declares are the ones it reads; they are not all
the columns the table has. In the client:

- Whoever holds `columns`, beside what changing the table's records needs,
  may add a column of their own, change what kind of thing a column they
  added holds, and remove it.
- Whoever may save a view may rename any column and reorder a choice's
  options.

All of it belongs to the table, kept with it for everybody, and applying
the blueprint again removes none of it: every column somebody added stays,
after the declared ones, with what the records hold; what was chosen of a
declared field (its label, its icon, the order of its options) carries on
by the field's name. What a blueprint can rely on:

- A field it declares is as it declared it. Nobody retypes it, changes
  what it offers or removes it from the client. Only its label, its icon
  and the order of a choice's options are somebody's choice, and none of
  those is what a record holds. A record keeps the field under its
  **name**, which never changes: a label stands over the name only where
  the column is headed, so `rows.read`, an expression and a file go on
  using the name.
- A record may hold fields the blueprint never declared. An action that
  reads a row by the names it knows (`rows.read`'s `fill`, a segment's
  cells) ignores the rest; one that hands a row on whole hands them on too.
- A column somebody adds says nothing a declaration can say about
  authority: no `takes`, never `required`, never `quiet` or `hidden`, and
  it is one of `text`, `number`, `date`, `flag`, `choice`, `list`,
  `record` and `records`. So a record a blueprint adds without knowing of
  the column is never refused because of it.
- A declaration that comes to declare a field under the name of
  somebody's column takes the column over: it is declared from then on,
  keeping what the records hold where that fits the declaration, and
  refused as any change of type is where it does not. A field a
  declaration drops takes what was chosen of it away.
- `collection.fields` is published for each change, with the `field` and
  what was `did` (`added`, `called`, `retyped`, `options`, `removed`).

A table kept as `files` has somebody's column written into each record's
file as a declared field is. One kept by a store `service` holds it in the
record it hands the store. A library and mockups take no added columns.

#### Private tables

A collection may be **kept private**: `reads: private:journal` is the
scope reading its records needs, on top of what reading any record needs.
It is for what is in a project but is not for everyone who works there,
such as a person's notes in a project whose agents help with the rest.

It holds at every way of reading:

- the records and their history;
- the files a library or mockups keep them in (listed, read, written over,
  removed, or reached by a link), and the library's pages;
- what searching finds;
- the events about them (each says what hearing it needs);
- the run of a handler that heard one of those events: somebody else who
  reads the run sees that it ran and which steps, not what the event
  carried, what the steps output, why any failed or what they printed;
- what is said of that run as it goes: `run.*`, `step.*`, `hook.failed`
  and each `log.line` of it need what hearing the event needed, so your
  handler on `run.failed` hears such a run fail only where you may be told
  what set it off;
- the folder itself, behind the wall of an agent that does not hold the
  scope, with the repository beside it, whose history has those files too.

What a blueprint that heard such an event goes on to say with a step of
its own (`emit`, or an action that publishes something), or to keep, is
its own doing, as whoever applied it. A table kept from somebody is not
among the tables they are shown. A board cannot be kept private.

#### What giving a field a value takes

A field may say what giving it a value **`takes`**: a scope, on top of
what changing the record needs, or a list of scopes, every one of which is
asked for:

```yaml
reads: { type: list, takes: shell }
with: { type: list, takes: [shell, secrets:use] }   # a command, or a secret's name
```

Use it for a field that something with more authority reads and acts on,
where whoever may change the rest of the record should not get that done
by changing a cell. Where only one value is of that kind, `takes` is said
of that value:

```yaml
outlives: { type: choice, of: [on, off], takes: { on: shell } }
```

(or `{ on: [shell, secrets:use] }`). Giving the field that value needs the
scope or scopes; any other value, or nothing, needs no more than changing
the record. Only a `choice` or a `flag` says it of a value, since their
values are written one way. A refusal names every scope the field takes
and which of them the writer lacks.

**`takes: each`** is for a `list` whose values are scopes that something
then gives to somebody else (`holds: { type: list, takes: each }`). The
record gives what the list holds, so whoever adds the record, or changes
anything of it, must hold each scope it then lists. Nobody gives through a
table what they do not hold, or acts through whatever was given it.

- A record that holds more than somebody does is not theirs to change in
  any cell. Made to say no more than they hold, in the same write, it is,
  since what it gives from then on is theirs to give.
- Removing the record needs no more than changing the table does.
- Such a list may say what it stands for where a record leaves it empty,
  `unsaid: [board, records]`: what is given all the same, by whatever
  reads the record. A record that says nothing is held to that.

Declaring the table again with less asked:

- with `takes` left off a field, a scope left off its list, or said of
  fewer values: by whoever holds what it no longer asks;
- with `each` left off, or less in `unsaid`: by whoever holds `admin`.

#### The history of records

A collection may also say what is kept as the **history** of its records:
`history`, a list of event topics or groups of them
(`[board.item.*, lane.*]`), or a map from each to its sentence
(`board.item.moved: "{actor} moved it from {from} to {to}"`):

```yaml
history:
  board.item.moved: "{actor} moved it from {from} to {to}[, saying: {note}]"
  board.item.created.restored: "{actor} gave it back from the Trash, to {status}"
```

Braces hold what the event says. A part in square brackets is said only
when the event has what the part names, so the first line above ends at
the column for a move nobody said anything about.

- A record given back from the Trash is announced as added again, with
  `restored`. Its line is the sentence of `<topic>.restored` where there
  is one, and otherwise `… gave it back from the Trash`.
- Each such event that names a record is written against it as it
  happens. A line once written is never changed or removed.
- What a blueprint's step does to a card (`board.add`, `board.move`,
  `board.update`) or to any record (`record.add`, `rows.start`) is the
  factory's doing. It is said as the factory's bot, by the name the bot
  has where the line is read (`conductor moved it from ready to doing`),
  not as whoever applied the blueprint, who lent the authority and never
  touched the card. The event keeps both: the person as its actor, and
  the blueprint as `through`. The blueprint is shown after the line as
  detail, never as who did it (` · via std/lane-round`, in the client and
  in `board history`).
- Where the step that adds a record is told whose work it is (`wrote` of
  `board.add` and `record.add`: an agent, as `agent:<name>`, and nothing
  else), the line names that agent too, `conductor added it to backlog from what shop-architect-1 wrote`, and so does the card's drawer. A
  blueprint passes on only what it knows from what is written:
  `std/feature` names the agent whose round wrote its plan as it stands
  (somebody who changed the plan since is who wrote it); `std/plan` names
  whoever wrote the document through the factory, which a fleet reads from
  the file's change. A document an agent's program wrote on the disk is
  found there, by nobody, and its cards name the bot alone.

### Boards and cards

A project's board is a collection called `board`: a table underneath, of
which the board is one layout. It exists once `std/board` has been
applied. These actions act on its cards:

| Action | Does |
|---|---|
| `board.add` | Adds a card. |
| `board.move` | Moves a card to a column. It may carry a `note`. |
| `board.update` | Changes a card's fields. |
| `board.get` | Reads one card. |
| `board.list` | Reads the cards. With `ids` it reads only the cards named, each found by its id, leaving out those that are gone, and outputs their `ids` beside the `items`. It always outputs the board's `columns`, with the `first` and the `last` (where cards begin and end). |

A move's `note` is what is said as the card is moved. It goes with the
`board.item.moved` event as `note`, into the card's history, and onto the
card as part of `moved`, in the same write as the move. So whatever reads
the card knows why it is where it is, whether or not it heard of the move.

A card is a row of that table and nothing more: nothing is started for a
card, and a card nobody is working on costs what a row costs. What a card
can do beyond being moved is one blueprint for the whole board,
`std/cards`, which the board starts. It is an instance that hears what is
said of any card, handles what is said of one card in order and of another
card beside it (`turn`), and keeps what it has to keep of a card on the
card. That a card can hold a question, and what answers one, is defined
there and not in the daemon (*Lanes*, below). Copy it into a project to
change what happens to every card there at some stage, or what a question
is there.

#### Fields that name records

A field may **name a record of another collection**:
`sprint: { type: record, of: sprints }` on a board says that a card may be
in a sprint.

- `of` is one collection of the same project. It must be declared in a
  step before this one, or be the collection being declared
  (`after: { type: record, of: board }`: a card that comes after another
  card). Naming one that does not exist fails the step when the blueprint
  is applied.
- The field holds the other record's id, as text (`shop-sprints-3`; a
  card's is `shop-12`).
- It is checked when it is written, wherever either collection is kept: an
  id that is not a record of that collection, in that project, is refused
  with a sentence saying so. It is not checked again.
- Removing the other record is not refused, and with records kept as files
  nobody is asked. The field goes on holding the id, the record that holds
  it is still moved and changed, and whatever shows it says the named
  record is gone. It can be given another record, or nothing, like any
  field.
- On a card, a field its board declares beyond a card's own is given among
  `fields` of `board.add` and changed among `fields` of `board.update`
  (`fields: { sprint: shop-sprints-3 }`; empty clears it). A card gives
  what it holds of them as `more`.

#### Records of other collections

The **records of any other collection** are made and changed by name:

| Action | Does |
|---|---|
| `record.add` | Takes `collection` and `fields`, each by name (`fields: { title: November, size: 5 }`). Outputs the record with its `id` (`shop-sprints-1`) and what it holds as `data`. |
| `record.update` | Takes `id` and `fields`. Changes the fields it names and leaves the others; an empty one is cleared. |
| `record.get` | Takes `id`; reads one record. |
| `record.list` | Takes `collection`. Outputs `items` and their `count`, in the order they were made. |
| `record.remove` | Takes `id`; removes the record. |

`record.add` given `declaring` (fields, as `collection.ensure` takes them)
makes the table with those fields where the project has none of that name.
Where the table exists, it writes only the fields that table has, which
keeps its own. It never writes into a table that says `takes`, `reads` or
`steers`, or is a model's.

What is written is checked as the collection declares it, wherever it is
kept. Each change is an event:

- `record.created`;
- `record.updated`, with `changed` (the names of the changed fields),
  `chose` (what each changed choice now says) and `flags` (what each
  changed flag now says). A handler that acts because a record was moved,
  or a flag ticked, goes by these, which describe the change itself, and
  not by how the record looks when it hears of it. A document's file saved
  through the daemon says the same.
- `record.removed`.

Each says the record's `id`, its `project` and its `collection`, and, when
done by a blueprint's step, that blueprint as `through`. A record changed
from outside with a key somebody made says the key changed it
(`source.actor` is `key:<id>`), as a card's change and a comment on a
record do. A collection that counts them in its history
(`record.updated: "{actor} changed its {changed}"`) keeps them like a
card's. They are the system's to publish, not a blueprint's to `emit`. A
blueprint that uses them lists `records` under its `permissions`.

A board's cards are read this way but not written this way: moving a card
is an event of its own and the board keeps its own mark on a card. So
`record.add`, `record.update` and `record.remove` on a `board` say to use
`board.add`, `board.update` and `board.move`.

The same is done over the API (`POST /api/collections/{project}/{name}/records`
and `PATCH /api/records/{id}` with `fields`, `GET` and
`DELETE /api/records/{id}`) and from the terminal:

```
mw record add shop sprints title=November
mw record set shop-sprints-1 size=8
```

and `list`, `show`, `remove`.

### Lanes

A card is worked on in a **lane**: a worktree, a branch and a conversation
with one agent that belong to the card for as long as it is worked.
Nothing runs for a lane itself. The stage of the project's fleet whose
column the card is in (`std/working`) looks after every card there by its
row. What is started for one card is a **round**, `std/lane-round`, which
lasts as long as its agent is at work:

1. When the card's turn has come, the card is taken (moved to `doing`).
2. It is given a git worktree and a branch of its own (`.lanes/<card>`,
   `lane/<card>`) when the project is a repository with history to branch
   from. A `.gitignore` in that folder tells the repository that `.lanes`
   is not the project's files, and ignores itself too. The place is made
   now and not when the card became ready: a card that waits has none, and
   a card that waited on other cards begins from what they left.
3. It is given to an agent, which works in that worktree.
4. What the agent left uncommitted is committed there, with the agent as
   author (`<agent>@agents.invalid`, an address that can belong to nobody)
   and whoever git is configured as the committer. A commit the agent made
   itself is left as it is.
5. With `push`, the branch is sent to the repository's remote.
6. The agent says what it did in the channel, in its own name.
7. The card goes to `review`.

#### Sending a card back

Whoever reviews the card **sends it back** by moving it from `review` to
`doing` with a note saying what is wrong: `mw board move <card> doing --note "..."`, **Send back** on the card in the client, or `board.move`
with `note` from a blueprint. Where the row of the stage that reviews says
a card that did not pass goes somewhere else (`fail`), **Send back** puts
it there: the client reads the rows that are about the board's columns
(the project's `stages`) for the stage that has the card's column, and
names no column where a row does.

The stage that worked the card hears that and asks for another round: the
same agent, the same conversation, worktree and branch. The agent is told
the note or, where nothing was said, that nothing was and to look at the
card and at what went before. What a person says as they put a card where
it is worked for the first time (`mw board move <card> ready --note "Mind the totals."`, or the words given with a move in the client) is told to
whoever works it, as what was said when it was put there.

- A card put back in `ready` after it was worked from there is taken again
  the same way: moved to `doing` for another round.
- This happens only as often as someone sends the card back. A stage that
  works never sends a card back, and does not take a round's own moves for
  one.
- A round that fails is reported, and the card stays in `doing`. Made
  ready again, or sent back again by moving it to `review` and back, it is
  taken up with the worktree and the conversation where they were left.
- Whatever is said or done to the card while its agent is at work is heard
  by the stage and taken up, in the order it was said, when the round is
  over (`turn`).

What the stage must remember of a card between rounds it writes on the
card:

- `taken`: the column it took the card from, which is how it knows the
  card is its own;
- `again`: what it keeps of the card's rounds: which of the card's moves a
  round has been told of, that what it asked was answered, that what was
  done could not be kept, or that its round is over and it waits for a
  person.

Why the card is where it is, and what was said as it was put there, its
row says for itself (`moved`), written by the move. What a turn that comes
for the card comes to is worked out from those words by a block the stage
asks (`work.turn`), whichever handler heard of the move first.

#### When the card's place cannot be made

In a repository where the card's place cannot be made (something else is
at `.lanes/<card>`, or git refuses), the card is not worked in the
project's own folder instead. The round fails saying what git said, nobody
is asked, and the card stays in `doing` until that is fixed and the card
is made ready again. Only a project with no repository, or no history yet,
is worked where it is.

#### Moves the daemon never announced

When the daemon starts, what was written as it went down and never
announced is announced. Where the board is kept in the factory's own
database, a card's move and its event are one write, so a daemon stopped
at any point leaves a card where it was said to be. A board kept as files
or by a store service is written and then announced (as every board was by
older programs), so a daemon stopped between the two left a card where
nobody was told it is.

The board looks for those itself: `board.settle`, which a stage that works
asks through `std/owed`, and which `std/cards` asks too, both as the daemon
starts and as each comes up (one coming up then was not yet listening when
the daemon said it had started). Every card whose row is in another column
than it was last said to be in has the move announced then, late (`late`
on `board.item.moved`), and whatever would have heard it hears it once. So:

- a card finished at that instant has its place tidied, and the cards that
  waited on it are asked again;
- a card made ready is worked;
- a card put in review is reviewed;
- a card *sent back* goes round again and is counted once, as sent back
  with nothing said. What was said went with the event that was never
  kept, and its agent is told to look and to ask.

Who made such a move is not known, so nothing is done on their authority:
a card passed by hand at that instant is not merged for them.

#### Work that cannot be committed does not go on

Where git will not make the commit (it is set to sign with a program that
is not there, say), the round says so of the card (`card.unkept`, with
`why`, what git said: a line of the card's history) and in the channel.
It sends nothing and moves nothing: the card stays in `doing`.

- Where that is the agent's to fix, whatever asked for the round gives the
  card to its agent again, in the conversation it had, told what git said,
  as a card sent back is told its note. That happens as often as the
  stage's `rounds` says (twice for a round asked for with nothing said of
  it: the round's `most`). After that the card is left where it is for a
  person, and why is said.
- Where git does not know who to commit as, nobody is asked again, since
  no agent can fix that. The channel says so at once, with what to tell
  git in the project.

The action that answers instead of failing is `place.keep` with
`unkept_ok: true`. It outputs `unkept`, why, in a line; `said`, all that
git said (which is what the agent is told); and `named`, whether git knows
who commits there.

#### A round across a restart

A round keeps nothing as it goes (`durable: false`, *Instances that keep
nothing*). If the daemon is stopped while an agent has a card, the round
is over. When the daemon starts again, the stage, which is kept, takes up
what it was doing for that card and asks for the round again, with the
card's place in the pool still its own. The round asks for what it asked
for before, and is given the turn that was left in hand, not a second one:
the agent is asked once, in the conversation it had, and the card goes on
to `review` without anyone moving it. Its history says so
(`card.went_on`). What becomes of the turn itself, and how a stage says to
leave the card for a person instead, is *When the daemon is stopped in the
middle*, below.

#### Questions on a card

An agent that cannot tell what is wanted, or why the card came back to it,
**asks** instead of working, in any round. It changes nothing, says what
it needs to know, and ends with `ask` (its task is offered the choice,
`options: [done, ask]`). The card is said to be asked (`card.asked`, with
the `card`, the `question` and the `asker`) by the stage that asked for
the round, and nothing is committed, sent or moved. The agent says its
question in the channel as itself.

What a question on a card is belongs to the board's cards, and is text:
`std/cards`, the one blueprint that looks after them.

- It hears `card.asked` and puts the question on the card, in the field
  `question` that `std/board` declares beside a card's own (a card with
  something there is waiting, and the board shows it so), with who asked
  in `asker`.
- It hears comments on a card (`record.commented`). While a question is
  open, it takes what someone other than the asker says as the answer:
  the question comes off the card and it publishes `card.answered`, with
  the `card`, the `asker` and the `body`.
- It hears a card moved by a person, which is an answer too, and takes the
  question off.
- What a blueprint says on a card answers nothing, and neither does a
  comment on a card nobody asked anything on.

The stage that worked the card hears `card.answered` and asks for another
round, with the answer as what was said (`mw record comment <card> <words...>`, or the box in the card's drawer). A project that wants a
question to be something else (answered only by whoever the card is
assigned to, say) copies `std/cards` and changes its handler `said`:
nothing of it is in the daemon.

#### Pushed branches

When the branch has reached the remote, the round publishes an event of
the card: `card.pushed`, with `card`, `project`, `branch`, `remote` and the
card's `title`. A blueprint that proposes the branch for merging waits for
that. The application knows no repository hosting service; a package
written for one brings such a blueprint ([extensions.md](extensions.md)),
and says where the pull request is as `card.proposed`, with `card`,
`project`, `branch` and `url`, which the card's history keeps.

#### Where a card's branch comes from, and whether it goes anywhere

This is said to the round, by the project's fleet (`fetch` and `push`).
By default it cuts a new branch from the project's folder as it is, and
sends nothing anywhere. Neither is done unless it is said.

With `fetch: true`, the repository's remote (`remote`, `origin` unless
said) is asked first, when the card's place is made:

- A branch the remote already has for the card (`lane/<card>`) is pulled
  down, and the card taken up where that branch is.
- A new branch is cut from the remote's `base` (the branch the project's
  folder has checked out, unless said) when that has everything the folder
  has and more, and from the folder otherwise. So work merged here and not
  yet sent is never left out.
- A place that was already there takes in what the remote has added to
  the card's branch, where that follows on.
- The project's own folder is not updated by any of this.
- A remote that cannot be reached is reported in the channel, and the card
  begins from what is here.

With `push: true`, the card's branch is sent to the remote each time its
agent has finished.

#### As whom the remote is reached

By default, as whoever runs the daemon, with whatever git on the device is
set up to use. The fleet may instead be told a `key`: the name of a secret
the factory keeps (`mw secret set shop-bot`), which is a token of the
hosting service's own, a bot's account or an application's, and says what
may be done there.

```sh
mw secret set shop-bot < token.txt
mw apply fleet --project shop --param push=true --param key=shop-bot
```

The remote is then asked and sent to as the key and as nobody else: git is
told to ask this program how to reach the remote, and no helper of
whoever runs the daemon.

- A key the remote will not take sends nothing; a key the factory does not
  keep is reported as missing, by name.
- The key is given only to the address the repository has for the remote,
  which must be `https`. A remote reached another way is refused when a key
  is set, and is not reached as somebody else.
- A service that cares which user a key is given under says so, and that
  user is written into the remote's address (`https://<user>@host/...`).
- The commits themselves are still by whoever git is set up as in the
  project (`git config user.name`).

A project's `fleet.bp` written before there was a `key` does not take one,
and says so when given one. It is the project's own file and is not
written over: add `key: { type: string, default: "" }` under its
`params`, and `key: "${params.key}"` to what it hands `std/fleet`, and
apply it again.

#### A card may wait on other cards

The standard board keeps `after` on a card: the ids of the cards that must
be done before it is begun.

```
mw board add shop "Shelves" --field after=shop-1,shop-2
mw board set shop-3 after=shop-1,shop-2
```

(the second for a card that is already there), or the field in the card's
drawer.

- The stage that would work the card asks first (`after.asked`), and what
  looks after the board's cards answers when none of them is left
  (`after.clear`): at once, or when the last of them is finished or taken
  off the board.
- Meanwhile the card stays in `ready`, holds no place, and says what it
  waits for, on the card and in its history (`it waits its turn: after shop-1, not done yet`, then `what it waited on is done`).
- A card that names nothing is asked nothing. Cards that do not wait on
  each other are worked side by side, as many as the fleet works at once.
- Two cards that wait on each other both wait, each saying so.

#### How many cards are worked at once

A fleet works a limited number of cards at once: as many as the role that
works them has agents, since an agent works one card at a time, and taking
more than there are agents would only make cards wait where it cannot be
seen. Each stage of the fleet has a pool for that (`std/pool`, *A limited
number at once*), named after the project and the stage's column
(`shop/ready`). The stage asks it for a place for each card, by the card's
id, before the card's round begins.

- A card whose turn has not come stays in `ready` and says that it waits
  and what for, on the card and in its history (`it waits its turn: one card is worked at a time here`). It is a row that says so and a name in
  the pool's list; nothing is running for it.
- When a card being worked goes to review, asks something or fails, the
  next is taken (`its turn came`).
- `mw emit pool.pause pool=shop/ready why="..."` has the stage start
  nothing new, and say so in the fleet's channel and on every card that
  waits. `pool.resume` has it go on.
- A card moved out of `ready` while it waits is not worked when its turn
  comes.

#### When a card is done

When the card is `done`, the stage that worked it removes its worktree,
unless it holds changes that were never committed. It then removes the
branch, when nothing would be lost with it: everything on it is in the
project's own branch, or on the remote as far as the repository knows.
Otherwise the branch is kept and the channel is told why. Nothing is done
to the remote.

To change what is done with a card in a round for one project, add a row
to its `segments` (*Something more done with a card where it is*, below).
A round is the work, the keeping of it and the sending, each a small
blueprint, and a row goes between two of them or in the place of one. To
change what a stage itself is, make the text for its kind the project's
own (`mw own std/working --project shop`).

### Channels

#### Bots

A blueprint can take part in a channel as a **bot**. `channel.join` puts it
there under a name of its own, a handler on `channel.message` tells it what
is said, and `channel.post` with `as: <name>` answers as that bot. `std/bot`
is the smallest one: when mentioned, it answers with what a command
prints. What was said is given to the command in its environment, never
put into the command.

#### The factory's bot

A post that names no bot is said by the factory's own bot, `conductor`:
one voice for everything its blueprints say, a member of the channel from
the first thing it says there. What the factory itself has to say of an
agent is said by it too: that an agent could not answer and why, that one
was not asked, that agents have been answering each other and will wait
for a person. An agent says only what its program said.

- Which blueprint said a message, and which run and step, is kept on the
  message (`said_by`, in `mw read <channel> --json` and on
  `channel.message`). The channel does not show it: there the message is
  the bot's, as anybody's is theirs.
- `detail:` adds more than the message says: the whole of an error whose
  reason is the message. In the client such a message opens to that, with
  the way to what was running.
- What the bot is called is a setting of the factory
  (`mw channel speaker <name>`, for whoever manages the factory). What was
  said before keeps the name it was said under.
- The name belongs to the bot: a new agent cannot be given it, and the bot
  cannot be given an agent's name. A factory that had an agent of that
  name before there was a bot keeps the agent as it is. The two are then
  told apart by kind wherever members and messages are shown
  (`agent:conductor`, `bot:conductor`), and a message that names them is
  said to both.

#### A failure is said once

`error` on `run.failed` and `instance.failed`, and `steps.<id>.error`, is
the whole failure: the reason, with each step it came up through named in
front (``step `again` failed: step `roles` failed: ...``). `reason` beside
each is the reason alone, which is what a person is told.

An instance that a `call` is waiting on says so when it fails: which
instance called it (`called_by`) and whether that step lets it fail
(`let_be`, its `allow_failure`). Its failure is that step's too: where the
step fails with it, whatever is above reports it; where the step lets it
be, it stops there. So a blueprint that reports failures in everything it
started (`scope: own`) reports the instances nobody called, those a call
let be, and handlers' runs, each once (`trouble`, `let_be` and `slipped`
in `std/fleet`). What it called itself and let be is its own to report: a
row of the fleet's tables that the fleet refuses is one message, with the
row, the cells that were changed and the reason.

#### Commands to the factory's bot

The factory's bot has no model, so what is said to it is a command or
nothing. It takes:

- what is running, and applying or stopping it;
- a board and its cards and columns;
- a project's records;
- the agents (listed, stopped, moved);
- a channel: read, posted in, renamed;
- the asker's own inbox;
- a project's environments and their runs;
- comparisons, sandbox tests and search;
- `emit` and `help` (`@conductor help` lists them).

Each is written as the command line's help writes it (a leading `mw` means
the same) and read by that line alone. What has no place in a channel is
not among them: workspaces, projects, accounts, keys, scopes, secrets,
signing in, the factory itself, profiles, copies and extensions. Nothing is
said in a channel that would tell its members what they may not read: an
inbox is counted, not read out; a channel kept to its members is read only
from inside it; a private table is not listed; and search finds no
messages.

Each command runs as whoever said it, by asking the route the command line
asks, so a key is refused through the bot exactly what it is refused
directly. Anything else is answered once with the list. A message that
names a bot and no agent is the bot's: the channel's one agent does not
take it for itself.

In a project's channel the bot is that project's:

- `status`, `instances` and `agent list` say what is the project's;
- `board` is its board and no other's;
- a card or an agent of another project is left alone, and the bot says
  so;
- an event published there carries `project`, the channel's, and may not
  name another.

That is on top of what the asker holds, never in place of it. In a channel
that belongs to no project, the bot is the factory's. In the client the
commands are offered as you write to the bot, and the bot is described,
with them, under Channel members in the panel beside the channel.

#### Removing a channel

A channel can be removed by whoever manages the factory (`admin`) and
nobody else (`mw channel remove <name>`, or its row's menu in the client).
It is announced as `channel.removed`. It goes into the Trash, with its
messages, what was attached to them and the inbox lines that open it, and
can be given back from there; where the factory keeps nothing that is
deleted, it goes at once. What agents were asked there goes with it, and
so does what the journal and the search index kept of what was said, so a
copy of the factory made afterwards holds nothing of it beyond what the
Trash keeps, and nothing once it is deleted for good.

While a running instance says it posts there (it has a param or a word of
state called `channel` that names it: a fleet in its project's channel, a
bot), the removal is refused and names the instance, since its next post
there would fail. Stop it first, or pass `--force`.

### The inbox

A blueprint tells somebody something by putting a line in their inbox
(`inbox.tell`, which needs `inbox:write` of whoever applied it):

```yaml
  told:
    do: inbox.tell
    with:
      to: applier            # or an actor, a handle (`@ada`), or a list of them
      says: "The night's build is green."
      why: built             # a word of the blueprint's own
      about: "${params.release}"   # the id of what it is about, where it is about something
```

| Argument | Meaning |
|---|---|
| `to` | Whoever applied the instance that asks (`applier`: for a key, its maker), an actor (`user:<id>`, `agent:<name>`), a handle with or without its `@` (which tells everybody it names), or a list of those. A word that names nobody, or an actor that is a bot, is refused, and then nobody is told. |
| `about` | The id of a card, a record, a channel or something running: what the line opens. |
| `place` | What opens the line, where that is something else. |

It outputs `told`, each actor in whose inbox the line is, and `passed`,
each that has no inbox. An agent the factory starts has none yet: it is
told nothing, and the step goes on.

- The line is from the factory's bot, as what a blueprint says in a
  channel is, and keeps which step said it.
- The same step run again (the daemon stopped in the middle of it) is safe
  to retry: it tells nothing twice.
- Told again by another step with the same `about` and `why`, the existing
  line is raised: unread again, counted, showing the latest words. So a
  card that asks three times is one line.
- Told about nothing, each telling is a line of its own.

The words are the blueprint's, said as whoever applied it, exactly as what
it says in a channel is. Nothing is read for whoever is told, so a
blueprint tells somebody only what its applier could have told them, and
what it says of a card is said even to somebody who may not read that
card's board. What a line opens is opened by the existing routes, which
check whether its reader may read it: a line never opens what its owner
may not read, and that is all that is enforced. In the client a line that
is pressed shows its words whole, is marked read, and shows what it is
about: a card on its board with its drawer open, a row of another table
over its table, a channel, something running. `mw inbox` prints it with
its words.

#### Parts of a line

What a line shows beside its words is the blueprint's to choose, as
`parts`: a list, each part a mapping with a `kind`, which a fork of the
blueprint adds to, changes or removes.

- `links`: addresses, each shown as a link.
- `cards`: the cards of a project's board in one `column` or holding one
  `label` (one of the two), each with what was said on it since it came
  there, and with Pass and Fail. A part of a column keeps which cards the
  column held when the line was told, and lists those wherever each is
  when the line is read: one that moved on says where it is, with nothing
  to press, and one taken off the board says it is gone. Told again, the
  line keeps the column as it is then. A part of a label, and a line told
  by an older program, list what is there when the line is read.

```yaml
  told:
    do: inbox.tell
    with:
      to: applier
      says: "The preview is ready to try."
      parts:
        - { kind: links, links: ["http://127.0.0.1:7760/"] }
        - { kind: cards, column: staging, fail_to: ready }
```

Three more kinds are read for whoever reads the line, as they may read
them, and say so in their place where they may not:

- `rows`: rows of a project's `table`, by their `ids` or by what a cell
  says (`saying`, written as `rows.read`'s is; `{ in: [...] }` for any one
  of several), with the `columns` named, in their order (the table's own
  where none are named). Each is drawn as the table draws it, a number
  called `seconds` or ending in `_seconds` as a length of time, and each
  opens its form. A table kept from the reader shows none of it.
- `runs`: the runs of an `instance`, of the instances under a `key` (a
  pipeline applied again and again), or of what ran under an `environment`
  of the project, newest first (`most`, 10 unless said): what each is a
  run of, how it went, when and how long, each opening its run's page.
  Reading them needs `blueprints:read`.
- `measures`: measurements by their names and `labels`, each its latest
  value beside the median of its `last` few (10 unless said), written as
  the measurements page writes what each is of. Reading them needs
  `fleet:read`. It counts, and says nothing of whether either is good.

Every part may name the tab of the line's page it is in (`tab`). Parts
that name none are on a first tab with no name. Tabs come in the order
each is first named, the first showing, and a page with one tab draws no
row of tabs. What the line is about stays under its words, over the tabs:

```yaml
      parts:
        - { kind: cards, column: staging, pass_to: done, tab: To test }
        - { kind: runs, key: "${instance.key}", tab: Deployment }
        - { kind: rows, table: test-runs, saying: { commit: abc1234 }, columns: [title, result, seconds], tab: Insights }
        - { kind: links, links: ["http://127.0.0.1:7760/"], tab: Links }
```

#### Passing and failing a card

- A card failed is moved to `fail_to` (`ready` unless said), with why it
  failed as what was said as it was moved. Its history keeps that, and its
  agent is told it when next given the card. Failing asks for a reason
  unless the part says `reason: false`.
- A card passed is approved by whoever passed it (`card.approved`, a line
  of its history and no comment: words are said on it only where somebody
  writes some). It is then moved to `pass_to`, or has `pass_press` of its
  column pressed on it, or, where the part says neither, stays where it is.
- Both go through the board's own doors as whoever presses, who needs
  `board:write`. An agent does not pass or fail a card it worked on or is
  given. Over the API: `GET /api/inbox/<line>/parts`,
  `POST /api/inbox/<line>/parts/<n>/cards/<card>` with `verdict` and
  `reason`.

The discussion on a card is its comments. An agent given a card again is
told what others said on it since it last had it (`record.said`, which
`std/work` asks), and answers by ending with `ask` with what it says,
which puts the question on the card for a person or another agent to
answer.

The cards a part lists, and what passing or failing one does, are asked of
the cards as they are (`kernel::parts`), never worked out by the
blueprint's text. Behaviour by label lives wherever something names a
label: a view held to one, a row of `segments` held to cards that are
`labelled`, a feature that gathers cards by `labelled`, the rule that says
who gets work (`std/dispatch`, shown each card's labels), and a part of
cards. What a stage does with a card, what is pressed on it before it is
begun (`actions`), and where a card failed from review goes, are by
column.

#### A column only a person moves cards on from

Failed cards can be sent to a column of their own, where a person looks at
them before they are worked again. Add it to the board (`std/board`'s
`columns`, say `[backlog, needs-analysis, ready, doing, review, staging, done]`) and say `fail_to: needs-analysis` in the part. No stage has that
column, so nothing is begun for a card there: it waits, with why it failed
in its history, until somebody moves it to `ready`, saying what they
found, which is then what its agent is told. Not built: holding that move
to a person. A row of a fleet's `actions` with `who: person` holds a card
in its column from being *begun* until a person presses it, but nothing
holds a card from being *moved out* of a column by an agent that may
change the board.

#### What a fleet tells people

A fleet tells of what waits for a person (`std/told`, which the fleet
attaches with `uses` and hands its `project` and where it reports). Who
is told is the rows of a table of the project that sits beside its board,
`tells`. Each row says `whom` (`applier`, a handle or an actor; writing it
needs `inbox:write`) and `of`, a list of which things, all seven where it
says none:

| `of` | what happened | heard as |
|---|---|---|
| `asked` | a card's agent, or whoever plans a feature, asked a question | `card.asked`, `feature.asked` |
| `left` | a card was left for a person: by its round or by a round that failed, by those who looked at it (sent back as often as its stage allows, nobody decided, something in the way of its merge), or by what its stage does with a plan | `card.left`, which says `why`, or how many `rounds`, and `as`, which way where the round says (`nobody`, `external`, or the word its agent ended with) |
| `blocked` | a card's agent could not do what it needed, something it needed being refused it, and ended its turn with `blocked` (`std/work` offers it the word); the card is left for a person in its column. A row that tells of `left` is told of it too | `card.left` that says `as: blocked`, with what the agent said as `why` |
| `review` | a card waits in a column where cards are reviewed for a person to review it: nobody in the stage's role, or fewer than it has look at each card or have to approve. Said once each time the card comes there, and once more after the stage's row changes while it still does | `card.waits`, which says `kind: review`, the `stage` and `why` |
| `held` | a card waits in its column for something the column offers to be pressed (*A button on a card*, below), once each time it comes there | `board.item.held`, which `says` what and who may press it |
| `plan` | a plan waits to be accepted: a feature's, or a card's where its stage sends a planned card nowhere | `feature.planned`, and `card.planned` where it says `waits` |
| `failed` | something of the fleet failed, as the fleet says it where it reports | `instance.failed` and `run.failed` of the fleet it is attached to |

```
mw record list shop tells
mw record add shop tells title=lead whom=@grace of=left,failed
```

**A card that asks is told by being named.** The question is posted on the
card, among its Comments, by whoever asked (its agent, with `std/told`
after it, or the factory's bot for a record that asks, such as an A/B
test), naming with `@` each person the rows name (`record.ask`: given the
words, whom as `inbox.tell` takes them, who asked, and which asking it is,
`once`, so it is said once wherever the daemon stops). Being named is what
tells them: a line in their inbox where they may read the card, which
opens it at the question. There is no separate fleet line for it. It is
answered in Comments, since anything said on a card that asks is its
answer, and the card's Details says only whom it waits on and leads there.
Asked of nobody (no row names anybody with an inbox), the question says so
on the card and where the fleet reports.

`tells` starts with one row, `applier`: whoever applied the fleet.

- A row emptied or removed tells nobody. With no row, nobody is told and
  nothing is run for a card that waits for nobody.
- Which rows a thing is told to, and the sentence, is a block
  (`tells.whom`). It is given the `rows`, which thing it is as `of`, what
  the event said as `said`, the instance that said it as `from`, and the
  `project`; it outputs `to`, `says`, `about`, `why`, and `tells`, whether
  there is anybody.
- A row that names nobody fails the telling it was part of, which the
  fleet reports where it reports.
- A fleet applied by somebody who may put nothing in an inbox (a key made
  before inboxes existed) cannot say whom. What tells says so once where
  the fleet reports, with what to do, and stops; it is attached again when
  the fleet is next applied.
- A fleet from before goes on as it was, telling nobody, until it is
  applied again with `--take`.

#### The chain: who is next when nobody acts

Who is next when nobody acts is the project's `chain`, a table beside its
board that `std/chain` keeps (the fleet attaches it beside what tells). A
row is a rung:

| Cell | Meaning |
|---|---|
| `waits` | which kinds of wait it is for: the words of `tells` except `failed`, plus `idle` (a card in a column no stage works, for which a rung must name the `column`). Every kind but `idle` where it says none. |
| `column` | holds the rung to a column |
| `labelled` | holds the rung to cards with a label |
| `step` | where it stands among the rest, lowest first |
| `who` | whom it tells, as `whom` of `tells` |
| `after` | how long the card may wait at it before the next rung is tried too, as `4h` |

```
mw record add shop chain title=lead waits=review who=@grace after=4h step=1
mw record add shop chain title=owner waits=review who=applier step=2
mw record add shop chain title=parked waits=idle column=draft who=@grace
```

- A card that waits is told to the first rung for it at once, and each
  time a rung's time is over and the card has not been moved since, to the
  next.
- Where on the chain a card is, is kept on its own row (`rung`, a quiet
  field of `std/board`), so a wait ends when the card is moved, and a time
  that comes for it after that does nothing.
- What is done for a card that waits is one blueprint that keeps nothing
  (`std/chain-wait`), called by a handler of `std/chain` for each kind of
  wait.
- Which rung is next, and the sentence, are a block (`chain.next`).
  `chain.read` says whether there is a rung at all, and which columns
  `idle` rungs name, so that a chain with no row runs nothing for a card.
- A rung's time is waited out by `std/later`, which waits and then
  publishes `later.over` with what it is given, kept across a stop of the
  daemon.
- A chain with no row adds nobody: whom `tells` names is told, and a
  `tells` somebody emptied tells nobody.

A rung may also name an agent, and then does more than tell (`does`):

- **`review`** names one agent (`agent:<name>`) and waits for `review`
  alone. The card is said to be due a review by that agent (`review.due`
  with `by`). Its stage begins the review that ships with that one agent
  looking (`agent` of `std/review`, `looker` of `agent.task`: a council of
  one), and what it decides is acted on by the stage as any review's is,
  merge and all, said as that agent's (`by`). A project's own review is
  given no agent.
- **`look`** names an agent or one of a role (`role:<role>`), told the
  row's own words (`told`) and shown the card and what was said on it
  (`prompts/chain.md`). It ends with a word:
  - `back`: the card goes back to the column it was taken from, with what
    the agent said as what its worker is told;
  - `ask`: whoever applied the fleet is asked, in its words;
  - `up`: the next rung is told now;
  - `stay`: it waits as it is;
  - `answer`, where the card waits on a question: the question is answered
    as the agent's.

A move is made only where the agent itself holds what moving a card needs
(`actor.holds`), and is said to be what it decided (`decided` of
`board.move`, a line of the card's history: `std/chain-wait (as shop-lead-1 decided) moved it…`). One it may not make is put to a person.
What its word comes to for the card as it now is, is a block
(`chain.decided`), which leaves a card that was moved meanwhile as it is.
A card it sends back is counted as one sent back from review is, so it is
left for a person once it has been sent back as often as its stage allows.
The table steers the fleet's agents (`steers: agents`).

#### Tools a worker was refused

What a worker's program says it was refused as it went (a tool it asked
permission for, where its package says how its program reports that:
`refused` of a package's output rules) is recorded with the card, as a
line of its history (`card.worked`), and decides nothing: the card goes on
as the agent said. A stage's row that says `refused_means: blocked` has
such a card left for a person, as one whose agent ended `blocked`.

#### How a card is finished

How a card is finished where it is worked is also its stage's row, so that
the agent is never left to reconcile what the card says with what it was
told:

| Cell | Meaning |
|---|---|
| `finish` | each step its agent sees to before it says the card is done (`its tests pass`, `the docs it changes say what is new`) |
| `later` | what somebody else checks afterwards, which is never the agent's to wait on or to run (`the whole gate, run when the work lands`) |
| `lasts` | how long one turn may take (`45m`, given to `agent.task` as `within`) |

Its turn (`std/turn`) begins with `finish` and `later` as a checklist,
before what it is asked; where the card's own words name something on
`later`, it is told that part is checked later. It ends by saying, a line
for each step, `STEP <n>: done`, `STEP <n>: later` (it waits only on
something checked later) or `STEP <n>: not done: <why>`. What that comes
to is a block's (`finished` in `deciding`, asked by `work.worked`):

- every step done or later: the card goes on;
- some not done: the same agent is given it again once, in the same
  conversation, told what is left; after that it is left for a person,
  saying what is left;
- none done: it is left.

Past `lasts` the program is stopped, and the agent is asked once, briefly,
what is done and what is left, which is read the same way. One that says
nothing even then is left, saying its time ran out.

How long each turn took, whether it was cut and whether it was given again
is measured (`turn.ended`, by `came`, `time` and `given`), so that a
comparison counts how often each way needed more. The contenders of an A/B
test are told and held the same way (`std/contest` hands them what its
stage says); those who look at a card at once (a council) are told none of
it. Nothing of it applies unless the row says it.

#### Following a record's history

One thing can follow another without either knowing of the other. Each
line written into a record's history is announced once
(`collection.history`, with the record, what kind it is, and the line), so
a blueprint that hears that follows anything that keeps a history.
`std/watch` is that blueprint: it posts each line in a channel, for one
record (`--param record=shop-12`) or for every record of a kind in a
project (`--project shop --param collection=board`). It does not know what
a card is; a sprint or a release that a blueprint declares with a
`history` (`history: [record.*]`, say) is watched the same way, with
nothing added here: `--project shop --param collection=sprints`. What it
watches is said as an address (`for: { record: ... }`,
`for: { project: ... }`), so a watch is woken only for the lines of what it
watches, however many records there are.

### Actions the device registers

Everything else is an action registered by the device: `shell`,
`system.check`, `service.start`, `router.route`, `channel.post`,
`board.move`, `fs.write`, `agent.task`, `agent.consult` and so on.
`mw actions` lists them with their arguments; `mw actions <name>` shows
one.

```sh
mw actions
mw actions shell
```

### Running commands

`shell` runs a command and waits for it to finish. A service
(`service.start`, `std/slot`) is a command that is kept running. Both
say what to run in one of three ways:

- **`cmd: [git, add, -A]`**: a program and its arguments, handed to the
  system as they are, with no shell in between. **This works on every
  system**, and nothing in an argument is ever run. In
  `cmd: [git, commit, -m, "${steps.card.out.title}"]` the card's title is
  one argument, whatever it contains. A service's program that takes its
  port as an argument has no shell to read `$PORT` for it, so in an
  argument of a service's `cmd`, `{port}` is the port the service was
  given and `{name}` is the service's name:
  `cmd: [serve, "--listen=127.0.0.1:{port}"]`.
- **`cli: [store, init, --db, "${state.file}"]`**: the same, for this
  program's own command line, wherever it is installed and whatever it
  is called there.
- **`run: make test`**: a script, run by the shell of the system the
  daemon is on: `sh` on Linux and macOS, `cmd` on Windows. It is for a
  command of your own, written for the system you run it on. On Windows
  it is one line, read as `cmd` reads a line typed at its prompt. A
  script of several lines is refused there: put it in a file of that
  shell and name the file (`run: build.cmd`).

`cwd` is the directory a command runs in, relative to the top of the
project.

#### Decisions are steps, not script

What a script would decide is said in the blueprint instead:

- A command whose exit code is an answer, not a failure, lists the codes
  that count as success (`ok_codes: [0, 1]`). The step that depends on
  the answer reads the code (`when: ${ steps.staged.out.code == 1 }`).
- A step that may fail says `allow_failure: true`.
- Each thing asked for is its own step, so it shows in the node view and
  in a run.

```yaml
blueprint: commit-if-staged
permissions: [shell]

steps:
  staged:
    do: shell
    with: { cmd: [git, diff, --cached, --quiet], ok_codes: [0, 1] }

  commit:
    needs: [staged]
    when: ${ steps.staged.out.code == 1 }
    do: shell
    with: { cmd: [git, commit, -m, Work in progress] }
```

#### Shipped blueprints run no shell script

The blueprints that ship (`std/`, and those a package brings) contain no
script of their own, so they behave the same on every system the daemon
runs on. They use `cmd:` and `cli:` only. A gate holds them to this
(`portable`). Where one runs a script, it is a command you gave it as a
param (`std/bot`, `std/rolling`): `run:` there is only ever your own
command. `std/log-feed` is the one exception that still has a script,
because no program that follows a file exists on every system.

#### Shipped blueprints decide nothing themselves

A shipped blueprint does not work anything out either. An expression in
one names a thing, asks one plain question of it, or joins up to three
such questions with one word (all `&&`, or all `||`). Whatever is
decided is an action the blueprint asks, which answers with a word
(`came`) and what goes with it. A gate holds them to this too (`plain`).

This is a rule for what ships, not for the language. A blueprint of your
own may call functions, do arithmetic and choose with `? :`. If one of
yours grows hard to read, you can make the same move: put the deciding
in a package's actions.

### Prompts

#### What an agent is told is a file beside the blueprint

A prompt is long prose. It is not typed out inside a blueprint, and it
is not built into the program. It is a Markdown file in a `prompts/`
folder beside the blueprints of its place:

| Place | Folder |
|---|---|
| what ships | `blueprints/std/prompts/` |
| a project's own blueprints | `@blueprints/prompts/` (or `prompts/` at the project's top, or under the repository folder the project named) |

A step builds the text with `text.put`, from parts in the order they are
said:

```yaml
asked:
  do: text.put
  with:
    parts:
      - from: review-seat.md                 # prompts/review-seat.md
        with:
          item: "${params.item}"             # fills ${item} in the file
          among:                             # a blank handed parts of its own
            - { if: "${ params.seats > 1 }", from: review-seat/among.md, with: { among: "${params.seats}" } }
      - "\n"                                 # words said here
```

A part is one of:

| Part | What it says |
|---|---|
| plain words | the words as written (`"\n"` above) |
| `say:` | words, or a list of parts of its own, each with its own `if:` |
| `from:` | a file of the `prompts/` folder beside this blueprint |
| `own:` | a file of the `prompts/` folder of the project's own blueprints, whichever blueprint names it. This is for a file a table's row names, as a fleet's `agents` row does with `prompt` |

Any part may carry a plain `if:`, for words said only sometimes, and
`else:`, for what is said when the `if:` does not hold.

Rules for a prompt file:

- It leaves blanks, `${name}` or `${name.part}`, which the part fills
  under `with:`.
- A blank handed a list gets those parts joined together. This is how a
  sentence that is only sometimes said keeps its place in a file that
  reads as a whole.
- Nothing is worked out in a file. Any other hole is refused, and so is
  a blank handed nothing. Both are refused when the blueprint is checked,
  naming the file and the blank.
- The file's one last newline is not said.

The step outputs `text`, and `said`: whether any of it said anything.

#### How a prompt file is found

A linked file is a text of the library, like a blueprint that is called:

- It is found by name (`std/prompts/review-seat.md`; a project's
  `prompts/worker.md`), never by a path a step puts together.
- A project's own file is read for a running instance only as it was
  vouched for. So an agent that may write the project's files cannot
  rewrite what it is told and have that taken up unasked.
- A project may stand its own file in for one that ships
  (`@blueprints/std/prompts/review-seat.md`), as with a blueprint. To
  change what your reviewers are told, copy the file there and edit it.
- A blueprint links the `prompts/` of the place it is in. A round of
  your own, made from the one that ships and kept beside it under
  another name (`@blueprints/std/my-round.bp`), is told what that one is
  told. One kept anywhere else needs its files beside it; the refusal
  says so when it is applied.

### Services and rollouts

#### A rollout is evidence only of what the new version was sent

`std/slot` hosts a service behind an address that does not change. It
replaces the service by giving the new version a growing share of the
connections (`weights`) and watching it at each stage
(`std/canary-stage`, over `router.observe`). A stage fails, and the slot
rolls back, when:

- more than `max_fail_ratio` of the new version's connections failed, or
- its process ended while it was watched, whatever it had answered until
  then.

A stage in which no connection reached the new version has nothing to
judge it by. By default the slot promotes the new version anyway, since
a service nobody uses yet still has to be rolled out, and says that it
did:

- `slot.promoted` carries `stages` and `unwatched` (in how many stages
  the new version was sent nothing).
- The slot outputs `unwatched`.
- A new version watched through no stage at all is reported by an event
  of its own, `slot.unwatched`, so that nobody takes "not rolled back"
  as a sign that it works.

`min_connections: 20` makes a stage that saw fewer connections fail
instead, and the new version roll back. Set it for a service that has
traffic, where a silent stage means something is wrong.

```yaml
steps:
  site:
    spawn: std/slot
    as: site
    with:
      name: site
      service: { cmd: [serve, "--listen=127.0.0.1:{port}"] }
      weights: [10, 50, 100]
      max_fail_ratio: 0.05
      min_connections: 20
```

### Agents

#### Asking an agent: `agent.task`

`agent.task` gives an agent a piece of work and waits for it.

- **Conversations.** Tasks with the same `thread` are one conversation,
  which the agent's program carries on. By default each instance has one
  thread.
- **Who gets it.** With no `slot` named, the task goes to the agent the
  conversation was last held with. A conversation not yet held goes to
  whichever agent has least in hand. With `role`, only an agent of that
  role is chosen (*Roles*, below).
- **Different agents for one thing** (`apart`). Tasks on the threads
  `review:7:1` and `review:7:2` that both say `apart: "review:7:"` are
  not given to the agent the other is held with, while there is anyone
  else of the role.
- **A council** (`seats`). Asked of several at once, the task goes to
  that many agents. Each has a conversation of its own
  (`<thread>:<seat>`), is apart from the others, and is an asking of its
  own (`<asking>:<seat>`). Answers are counted as they come: the council
  passes when `needed` of them choose `yes` (more than half, by default).
  `yes` and `no` are `approve` and `send_back` unless said. It is decided
  as soon as it can be; seats still looking are called off. The step
  outputs `came` (`passed`, `failed` or `undecided`) with `by`, `words`
  and `votes`. A seat that could not be asked, or whose agent could not
  do it, chose neither. `std/review` asks its seats this way, and so does
  `std/turn` given `seats`. With `seats` 0, one agent is asked, as with
  none.
- **Where it works** (`cwd`). The directory the agent's program runs in,
  relative to the top of its project. `cwd: ".lanes/${params.item}"` is
  how a lane's agent works in the lane's worktree and not beside it. A
  directory that is not there, or that leads out of the project, fails
  the step before the agent is asked anything. A conversation is carried
  on only in the directory it was held in, because an agent's program
  keeps what was said by where it was said: the same `thread` with
  another `cwd` starts again.

```yaml
steps:
  looked:
    do: agent.task
    with:
      role: reviewer
      thread: "review:${params.item}"
      seats: 3
      needed: 2
      prompt: Look the change over. Approve it, or send it back saying what is wrong.
```

#### An agent

An agent is a slot: a name, the program that runs it, what it may do,
and the instructions it is given on every turn. Every agent the factory
starts is brought in by `std/agent`, whose params are those settings,
each with what it means:

- Applied, the agent is as the params say.
- Applied again with other params, it follows them.
- Stopped, it is stopped and kept.

An agent added by hand (`mw agent add`, the client's Add agent) is an
instance of `std/agent` keyed by the agent's name. It is applied for its
project, under its environment, as whoever asked, and is changed by
being applied again. A role's agents are each one instance under the
role.

So a project that wants to set up its agents another way forks
`std/agent`: a param more, a step more. An agent added by hand in that
project is then set up by the fork, with what the fork's own params
were given (`params` of `POST /api/agents`) kept as the agent is
changed.

#### Roles

The factory knows nothing of what an agent is for. A fleet with several
agents of a kind says so with a *role*: a word of its own (`worker`,
`reviewer`, anything) that those slots share. `std/role` brings a role
up: so many agents, each told and allowed the same, called `<role>-1`,
`<role>-2` and so on. Where it says a `name`, they are called
`<name>-1` instead; a role may be made of several of these, each with a
name of its own and run by something else.

```yaml
steps:
  reviewers:
    spawn: std/role
    as: reviewers
    with:
      role: reviewer
      count: 3
      permission: read
      instructions: "You review changes. You change nothing yourself."
```

Work is given to the role, not to one agent by name: `do: agent.task`
with `role: reviewer` goes to whichever reviewer has nothing in hand, and
a conversation begun with one of them is carried on with that one.

**Changing the count.** Applied again with another `count`, the role has
that many. Those past the count leave, and come back when counted in
again. How one leaves is `leave`:

| `leave` | What happens | Same as |
|---|---|---|
| `finish` (default) | takes no more work, and stops when it has finished what it has in hand | `agent.stop` with `idle: true` |
| `now` | stops where it is; what it was doing stops with it, and what waited behind that is not begun | `agent.stop` without `idle` |

An agent told to leave when idle is given nothing more: not by name, not
as whoever is available, not as whoever a conversation was held with. It
says `agent.slot.leaving`, and is stopped when it has nothing left.
`mw agent list` says `leaving` meanwhile; from a terminal it is
`mw agent stop <name> --idle`.

Stopped either way, an agent is kept, and is back when it is counted in
again. No blueprint takes an agent away for good. That is a person's
job (`mw agent remove <name>`, or its line under Members, Agents); it is refused for an
agent a role still counts, and says `agent.slot.removed`.

`agent.ensure` takes `role` too, for a slot brought up by itself, and
`mw agent list` shows each agent's role. A role nobody is in fails the
step that asks it, saying so, unless the step says
`if_available: true`.

#### External agents

An agent may be one the factory does not start: an *external* agent, a
program somebody runs for themselves that works with the factory's own
command (`mw help invite`). No step gives one work:

- `agent.task` and `agent.consult` that name one fail, saying that it
  takes its work up itself.
- With `if_available: true` they answer that nobody was there, with
  `named` and `external: true`. This is how a card given to one, in a
  column a fleet works, is left in hand for it (`work.worked` takes
  `external`, and the fleet says so where it reports).
- `agent.ensure` brings up nobody of its name, a role is never asked of
  it, and a message that names it gives nobody a turn.

### Fleets

#### The fleet that ships

Nothing above is about architects, workers or reviewers, and nothing in
the factory is. Which role gets what, when, and what it is told, is text
and table rows. `std/fleet` is that text as it ships.

A project does not apply `std/fleet` by name. `mw fleet init shop` writes
the project a blueprint of its own, `fleet.bp`, which says where the
fleet is (its channel, and what may be asked of or sent to the
repository's remote) and calls `std/fleet`. A project added by hand has
had both done for it, unless it was added with `--bare` or
`--without fleet`:

```sh
mw fleet init shop
mw apply fleet --project shop
```

Who is in a fleet, and what they do with a card, are in neither file.
They are two tables of the project. The fleet declares them
(`collection.ensure`) and fills them the first time. You change them
where tables are changed: in the client, from the board's page, or from
the terminal.

```sh
mw record list shop agents
mw record set shop-agents-3 count=3                  # three who review
mw record add shop agents title=second role=reviewer model=...
mw record set shop-stages-2 seats=1                  # one of them to a card
```

#### `agents`: who is in the fleet

A row of **`agents`** is so many agents of one role:

| Field | Says |
|---|---|
| `title` | what they are called. Each name gets its project before it and its number after: `shop-worker-1`. An agent's name is unique in the whole factory, so two rows may not have the same title |
| `role` | the word their work is given by, which a stage names. A row that says none is of the role its title names |
| `count` | how many there are. One if not said; 0 means none, the row waiting for when there should be some |
| `rank` | how they rank among everyone of their role, higher first; 0 if not said. Read only by a stage whose `picks` is `rank` |
| `adapter`, `model` | what runs them, and on which model; the device's default if not said |
| `permission` | `read`, `edit` or `full`: how much one may do unasked. `full` means running commands as whoever runs the daemon. It needs whoever applied the fleet to hold `shell`, and whoever gives the cell that value to hold it too (`takes: { full: shell }`) |
| `holds` | what each one's credential holds, as a list of scopes (`board`, `channels:read`): what it may ask of the factory. If not said, what an agent usually holds (*Permissions*). Never more than whoever applied the fleet holds, and no scope that whoever wrote the row (any cell of it) does not hold themselves (`takes: each`) |
| `outlives` | `on` or `off`: whether each one's program keeps working when the daemon is not running. `off` if not said: the program ends with the daemon, and the next daemon starts it again for the same turn. `on`: it is started so that it does not end, and the next daemon takes it up where it is, still working or finished, without asking it again. Such a program works unwatched while the daemon is down, and nobody can stop it through the daemon. So setting `on` takes `shell` of whoever changes the cell, beside what changing the row takes, as removing a wall does; setting `off` again takes only what changing the row takes (*When the daemon is stopped in the middle*) |
| `web` | `on` or `off`: whether each one's program may use the web unasked, searching and fetching pages with its own tools. `off` if not said: a program that asks permission for it is refused, since nobody is there to give it. A wall does not hold the network, a page may say anything to an agent, and a search carries whatever the agent puts in it. So setting `on` takes `shell` of whoever changes the cell, beside what changing the row takes, as showing a program more of the device does; setting `off` again takes only what changing the row takes. Which of a program's tools this covers is its package's to say (`mw help web`) |
| `wall` | `on` or `off`: whether each one's program works behind a wall, shown its work and a home of its own and nothing else of the device. `on` if not said. `off` starts it seeing the device as whoever runs the daemon sees it (`mw help wall`) |
| `reads`, `changes` | what else its program may read behind its wall, and what else it may change: lists of folders or files, each written from the top or from the home of whoever runs the daemon (`~/.cargo`). A toolchain it builds with goes in `reads` |
| `secrets` | the keys its program is handed: secrets the factory keeps (`mw secret set`), each under a variable of the program's environment, written `VARIABLE=name` (`GH_TOKEN=github-bot`). This is how an agent gets a key for another service, and that key says what it may do there. Changing the cell takes `secrets:use` |
| `git` | what its program may do with the project's repository behind its wall: `off` (the repository is not there), `read` (it reads it, and what it changes is committed for it; the default), `commit` (it commits its own work, in a card's worktree, on the card's branch and not the project's) or `write` (everything). In every case the repository's settings and hooks are not its to change |
| `listens` | whether they listen in the project's channel |
| `plans` | whether they are told how to write a plan so that its tickets become cards |
| `told` | what each is told on every turn |

#### Who may change the fleet's tables

Both tables say `takes: agents:write`. A row is changed by whoever may
change agents, not by whoever may write a record, because changing a
row changes what agents there are, what they are told and how much each
may do. An agent does not hold `agents:write` unless its row's `holds`
gives it.

`holds` says `takes: each`. The fleet brings agents up as whoever
applied it, so a row is added, or changed in any cell, only by somebody
who holds every scope the row then gives. What an agent is told and how
many of it there are are cells of the same row: somebody who holds less
than an agent cannot steer it or multiply it by changing them. A row
that says nothing of `holds` gives what an agent usually holds, which
the table states (`unsaid`), and is held to that.

Stopping an agent gives nobody anything: a row is removed, as an agent
is stopped, by whoever may change agents.

Some cells take `shell` on top:

| Cell | Takes `shell` for | Why |
|---|---|---|
| `reads`, `changes` | any value (`takes: shell`) | showing a program more of the device is what running a command there gives |
| `wall` | `off` (`takes: { off: shell }`) | removing a wall is the same; saying there is a wall is not |
| `outlives` | `on` (`takes: { on: shell }`) | a program working unwatched is the same kind of thing; stopping that is not |
| `web` | `on` | as `outlives` |
| `permission` | `full` | as `outlives` |

`stages` also says `steers: agents`. A stage gives the fleet's agents
their work, as whoever applied the fleet: whichever agents its row comes
to, by the role it says, by its title where it says none, or by a
blueprint its row names. So a stage is added, or changed in any cell,
only by somebody who could have written each row of `agents` as it
stands: who holds each scope those rows give, and what their cells take.
An agent that holds more than you do is not given work on your say, just
as it is not by a blueprint you apply (`agent.task`). If that is in the
way, first change the row of agents, as somebody who holds what it
gives.

#### Where the fleet's tables are shown

All three of the fleet's tables say `beside: board`, and `stages` also
says `about: column`. They are how the board is worked, so the client
reaches them from the board's page and lists none of them under the
project, and a column's head shows the stage that is about it. This
affects only how they are shown. A fleet applied by an older program
says neither, and its tables are listed under the project until the
fleet is applied again with `take`.

#### How `agents` groups its columns

`agents` puts its columns in five `groups`, because a row says three
kinds of limit that look like one when they stand side by side:

| Group | Columns | What it is |
|---|---|---|
| *Who they are* | `role`, `count`, `rank` | which role, how many, and how they rank |
| *What they are told* | `told`, `listens`, `plans` | their instructions, and what they hear |
| *What they may ask* | `holds` | what each one's credential lets it ask of the factory, which the factory enforces |
| *How their program works* | `permission`, `adapter`, `model`, `outlives`, `web` | what the agent's own program does unasked, which the program keeps to |
| *What their program can reach* | `wall`, `reads`, `changes`, `git`, `secrets` | what of the device the wall around the program lets through |

`permission` and `holds` are not two words for one thing. An agent held
to `read` by its program may still hold `board` at the factory. One its
program lets run anything still asks the factory only what its
credential holds. In the client `holds` is shown and filled by area, one
line per area with what is held there and what is not, as anybody's
scopes are shown. The cell says `the usual` where the row says nothing,
which is what `unsaid` lists.

A role with a count, and one agent with a program and a model of its
own, are both rows. A role may be made of several rows: three
reviewers on three models are three rows of one role. A cell left empty
says nothing; the table above says what that means for each. A row
titled the same as an earlier row brings up nobody, and the channel is
told: an agent is known by its row's title.

#### `stages`: what is done with a card

A row of **`stages`** is a column of the board, and what the agents of a
role do with the cards there. Each kind is a blueprint of its own
(`std/working`, `std/reviewing`, `std/planning`), which is handed the
row:

| Field | Says |
|---|---|
| `title` | what the stage is called |
| `column` | the column whose cards are its; the one its title names, if not said |
| `does` | `plan`: an agent of the role says how the card would be done, or asks what it needs to know. `work`: an agent of the role does what the card asks. `review` (the default): the role's agents look the work over and pass it or send it back. Any other word is the name of a blueprint, the project's own or a package's: the stage starts it for each card that comes, as `<word>-<card>`, and hands it `project`, `item`, `channel`, `role`, `picks`, `from` (the column), `to` (`pass`), `pool` and `note` (what was said as the card was put there). One that moves the card on itself is done with it; one that says what it came to and moves nothing (`stage.came`, below) has its stage hand the card on |
| `role` | the role whose agents do it; the one its title names, if not said |
| `picks` | which of the role's agents gets a card, or a seat at its review, among those with equally little in hand: `first` (default), the one brought up first; `turn`, each in turn; `least`, the one given least so far; `rank`, the one its row ranks highest |
| `pass` | the column a card goes to when it is planned, worked or passed. A planning stage that says none leaves a planned card where it is, for a person to accept by moving it on. A review that says none leaves a card that passed where it is, saying who approved it |
| `fail` | for a review, the column a card goes to when it is sent back: `doing` by default, where whoever worked it takes it up again at once. Sent to the column it is worked from, it waits its turn again; sent anywhere else, it waits there for a person |
| `seats` | for a review, how many of the role look at each card; all of them, with 0 or nothing |
| `needed` | how many of those must approve; more than half, with 0 or nothing |
| `next` | which waiting card in the column is taken next when there is room: `first` (default), the first to arrive; `last`, the last; `priority`, the one whose `priority` stands highest, and among equals the first to arrive. See *Priority*, below this table |
| `at_once` | how many cards are taken at once: as many as the role's agents can take (the default, or with 0), and never more. So `1` with two agents is one at a time |
| `review` | for a review, the blueprint that reviews one card here: the fleet's `review` (`std/review`) by default. One of the project's own takes the params `std/review` takes and says `review.passed`, `review.failed` or `review.undecided`. It runs only as it was vouched for: saved by somebody who holds what the fleet was applied with, as for a blueprint `does` names |
| `proposal` | for a planning stage, the blueprint that plans one card here: the fleet's `proposal` (`std/proposal`) by default, held to the same rule as `review` |
| `rounds` | how many times a card may be sent back from here before it is left for a person: 2 by default. The stage counts it wherever it sends the card, so workers and reviewers never go round forever. A card left that way is reported where the stage reports and as an event, `card.left` (`card`, `project`, `rounds`), once each time it is left; that is also a line of the card's history |
| `most` | the most things done with a card here in one pass through its list (*How many things at most*, under *Writing a segment*): 6 where cards are worked and 5 otherwise, by default; with 0, no bound but the list's own length. A card whose list holds more is left for a person, and the message gives the command that raises it on this row |
| `merge` | whether a card's branch is merged into the project's when it passes here |
| `by_hand` | what happens to a card's branch when a person passes the card themselves, moving it on from here: `leave` (default), the branch is left as it is; `merge`, it is merged as an agent-approved one is, if whoever moved the card holds `projects:write` |
| `in_flight` | what happens to a review in session when the row is changed: `finish` (default) or `restart` |
| `reports` | what the stage says where the fleet reports, beyond what goes wrong: `trouble` (default), nothing more; `verdicts`, also what each review of a card came to, with who looked and what each said |
| `after_restart` | what happens to what an agent had in hand here when the daemon is stopped and started again: `again` (default), it carries on; `leave`, it is left for a person, and the channel is told why. Work whose program kept going by itself but was killed together with what kept it is cut off after all, and is begun again or left as this says |
| `restarts` | how many times the daemon may restart in the middle of the same piece of work before it is left for a person, whatever `after_restart` says: 3 by default |
| `moved_away` | where cards are worked, what happens to a card's round when somebody moves the card out of `doing` while its agent is at it (back to waiting, further back, on, or to done). `stop` (default): the round is stopped and the agent's turn called off, which is not a failure. What it changed is left where it was, committed by nobody, and the card records that it was taken out of its agent's hands (`again: [away, <column>]`). Put back, it is taken up where it was left, and its agent is told so and what was said as it came back. `finish`: the round runs to its end and hands nothing on, and a card put back is worked again as new. A move a blueprint made (a round handing its card on) stops nothing |

**Priority.** With `next: priority`, a card's `priority` is a word of the
board's choice (`urgent`, `high`, `normal`, `low` on the board that
ships; nothing means `normal`). It ranks by where that word stands among
the choice's words as they are ordered now, first highest (`ranked` of
`board.get`, `kernel::ranked`). A board whose words somebody reordered
ranks by the new order. A number written in `priority` before it was a
word becomes the word as far above `normal` as the number was above
zero (below `normal` for less), once, when the board is applied; the
card's history records it (`record.reworded`).

The order is the order of the stage's pool (`std/pool`'s `order`). It
holds for every kind of stage whose cards take places in that pool: a
card's turn to be worked, and a review that ships (`std/review`). Each
asks with where the card's `priority` stands, read from the card at the
moment it asks. A plan (`std/proposal`) takes no place in a pool. A
review or blueprint of the project's own naming is handed `pool` and
asks with no priority unless it says one itself (`priority` of
`pool.asked`), so it is taken first come, first served.

#### What a new fleet starts with

When the tables are first made, the fleet is close to the smallest it
can be:

- **`agents`**: an architect that listens and plans, one worker, and a
  row of reviewers with a count of 0, so a worked card waits in review
  for a person.
- **`stages`**: `ready`, worked by a `worker`, going on to `review`; and
  `review`, looked over by the `reviewer`s, going on to `done` merged, or
  back to `doing`.

The starting rows are params of `std/fleet` (`agents`, `stages`: each a
list of rows), read once.

#### What each kind of stage does

**Planning.** The agents of a row that `listens` hear the project's
channel. With one such agent, anything said there that names nobody is
for it. An agent whose row says `plans` is told, when asked for a plan,
to write it as a document in the project (`docs/plans/`) that ends with
its tickets. It writes a file and runs nothing. A plan written there, by
it or by a person, has its tickets added to the board as cards by
`std/plan` (below), each saying which cards it waits on. The fleet's
`planned` says where they land: `backlog`, where they stay until someone
moves them to ready, or `ready`.

**Working.** A card that reaches a stage that works belongs to that
stage, by its row. Nothing is started for it until an agent of the
stage's role takes it (a round, `std/lane-round`), which happens when
the cards it waits on are done and a place is free: there are as many
places as the role has agents. Worked, the card goes to the stage's
`pass`. If nobody is in the role, the card stays where it is, the channel
is told, and it is taken up when somebody is.

**Planned** (`does: plan`, `std/proposal`). A card added to or moved into
the stage's column is put to an agent of the role, which keeps it for as
long as it is there: the same agent, in one conversation. The agent
reads the card and ends its turn one of three ways:

- It *asks*: its question is put on the card, which waits; what somebody
  then says on the card is passed back to it.
- It *proposes*: what it wrote is kept on the card as its `plan`, which
  whoever works the card and whoever reviews the work are shown with
  it. The card goes to the stage's `pass`, or stays for a person to
  accept by moving it. (Where the project's `segments` adds something
  after the plan, the planner says the card has a plan and its stage
  hands it on once that is done.)
- It *leaves* the card as it is.

The agent is told again when what the card asks changes, when something
is said on it, and when the card is sent back to the column with what
was wrong. `rounds` is how many times before the card is left for a
person (8 by default for a planning stage). What it is told about how
to plan is its row's `told` plus the `told` of `std/proposal`. For a
different blueprint altogether, set the stage's `proposal`, or the
fleet's for every stage. A planning stage in the first column, a stage
that works in `ready` and one that reviews give a card that is planned,
accepted, worked and reviewed, each by its own role:

```sh
mw record add shop stages title=plan column=backlog does=plan role=architect
```

**Who it is given to.** An agent does one thing at a time, so a card
goes to an agent of the role with nothing in hand before one that is
busy. Which of several free agents gets it is the stage's `picks`: the
one brought up first, each in turn, the one given least so far, or the
one ranked highest (`rank`, a number on the `agents` row: the better
model, the cheaper one, whatever you make it mean). A review that seats
fewer than there are chooses who sits the same way.

A card that names an agent as its assignee (set on the board, or with
`mw board give <card> <agent>` before it is made ready) is worked by
that agent whatever the stage says. If that agent is not up, it is given
to nobody else, and the channel is told. A stage takes the name only
where it is one of its own role in its own project: a card given to the
agent that planned it is worked by one of the workers, as if it named
nobody. Once a card has been begun, every round of it is the same
agent's.

**The rule for it is a blueprint that is asked** (`std/dispatch`). Where
work is asked of a role and nobody is named, the daemon puts those who
could take it in the order `picks` says and asks this rule which of them
gets it. It is asked, never run: it has `params` and `outputs` and no
steps, and as it ships it picks the first. A project that has a file of
that name is asked its own instead, which may go by what the work is
(`about`: for a card, its id, title and labels) and by each agent's
name, role, rank, model, how much it has in hand and how much it was
ever given. Whoever it names must be one of those it was shown; naming
nobody means the first.

```yaml
# @blueprints/std/dispatch.bp, kept for the project: cards about the till to one agent
blueprint: std/dispatch
params:
  about: { type: any, default: {} }
  agents: { type: list, default: [] }
outputs:
  take: "${ contains(lower(params.about.title ?? ''), 'till') ? 'shop-worker-2' : params.agents[0].name }"
```

When the rule takes effect:

- Saved through the factory (the client's editor, or `PUT
  /api/projects/<project>/file`) by somebody who holds what the fleet
  runs with, it is asked from the next piece of work on: the running
  fleet was vouched for with the file as it now is.
- Written in an editor outside the factory, it is not. A running fleet
  runs a project's file only as somebody who holds what it runs with
  applied or saved it. It is asked once the fleet is applied again with
  the project's files taken as they are (`mw apply --instance <the
  fleet> --take`).
- In a project made from a folder somebody gave, a file of its
  repository's folder is asked only once the factory's owner trusts the
  project (`mw project trust <project>`). One the factory keeps
  (`@blueprints/`) is asked whether or not it is trusted.

Where the file stands is said first in the Summary of `std/dispatch` for
the project, in the answer to a save, and by `mw status`, which lists
each such file not in use with what would put it in use.

**Reviewing.** A card that reaches a stage that reviews is looked at by
the agents of its role, side by side, each on its own, each a different
agent, and each in a conversation that is its seat's on that card
(`std/review`, or the blueprint the fleet's `review` names). They are a
council: of one if the role has one agent, of three if it has three, and
fewer than there are if `seats` says so. It passes when `needed` of them
approve, more than half by default. It is decided as soon as it can be,
and the seats still looking are called off: the seats are one step of
the review (`agent.task` with `seats`), and the review keeps which turn
is whose.

Reviews take turns, as many at once as there are councils to seat, and
a card waiting for its review says so. A role with nobody in it, or too
few for the stage's `seats` and `needed`, reviews nothing. The card
waits there for a person, which is said once each time it comes
(`card.waits`, `kind: review`), written as a line of its history, and
told to whoever the project's `tells` says (`review`). When the row is
changed and there is somebody, they take up what was waiting.

**Sent back** (`review.failed`: too few approved, and someone said what
is wrong). The card goes to the stage's `fail` with what each reviewer
said. The stage that worked it treats this as a person sending it back,
and the agent that did the work takes it up again. How many times a
card was sent back is counted on it (`rounds`, by `std/cards`): when it
is put where cards are in hand, or from `review` into any column the
board has before that, by anyone. After the stage's `rounds`, nobody is
asked and the card is left there for a person, and the channel is told.
A review that did not pass with nobody saying what is wrong
(`review.undecided`) is left for a person too.

**Passed** (`review.passed`). The card records who approved it and why
(`card.approved`, a line of its history). Where the stage says `merge`,
its branch is merged into the branch the project's folder has checked
out, as a commit that names the card. Then it moves to the stage's
`pass`. A branch that no longer merges is not forced: the merge is
called off and the card goes back to whoever worked it, who is told to
bring the branch up to date. Anything else blocking a merge is said in
the channel and left to a person.

**Passed by a person**, who moves the card on from the stage's column
themselves. The card is where they put it, and what happens to its
branch is the row's `by_hand`:

- By default the branch is left as it is, for that person to merge.
- With `merge` it is merged for them (`std/pass-by-hand`), as an
  agent-approved card is, and the card's history says so
  (`card.merged`).

The merge is done by the fleet, with what whoever applied the fleet
holds, so the person who moved the card is checked first: the block
`actor.holds` says whether the actor the move's event names holds
`projects:write` now. If they do not, they have passed the card and
merged nothing: the card stays where they put it, its history says it
was not merged (`card.unmerged`), and the channel says why and what to
do. A card moved with a key somebody made counts as the key's doing,
and is checked against what the key holds, which may be less than its
maker holds. The card's place is tidied after that is done, not
alongside it.

#### Counted

What comes of all this is measured, and what the figures mean is up to
whoever asks: nothing here defines a score. Each turn an agent takes
records what it chose, in the words it was given to choose between, and
each reviewing stage records what it came to each time it was shown a
card:

```sh
mw metrics agent.turn --by slot,decision --project shop    # how often each approved, and sent back
mw metrics review.verdict --by stage,verdict --project shop  # how often each stage passed what it was shown
```

Each is also an event (`review.passed`, `review.failed`, with who
approved or sent it back and what they said; `card.worked`, with who did
the work). So a blueprint of the project's can keep its own tally in a
table, or do something when a figure is reached.

#### More stages

Anything that should look at a card before it is done is a row. Give the
board a column for it (`std/board`, `columns`), give the role a row of
`agents`, give the column a row of `stages`, and in the stage before it
say that a card goes on to it:

```sh
mw apply std/board --project shop --param 'columns=["backlog","ready","doing","review","test","done"]'
mw record add shop agents title=tester permission=read
mw record add shop stages title=test role=tester pass=done merge=true
mw record set shop-stages-2 pass=test merge=false
```

A card that passed review is then looked at by the testers, and merged
and done when they pass it. Sent back from there, it goes to whoever
worked it, and counts as one more round. A fleet has one stage that
works cards, taken from its column into `doing`; any number may review
them after.

What is wrong with a row is said in the channel each time the row is
changed, for as long as it is wrong:

- a stage that names a column the board does not have;
- a stage whose role no row of `agents` has;
- a stage that asks more agents to look or to approve than there are;
- two stages with one column.

#### A button on a card: actions

A column may offer a button a person presses on a card in it:
`Approve`, `Signed`, whatever you call it. Each is a row of the
project's `actions`, a fourth table of the fleet's, which starts empty
and sits beside the board, about a column:

| Cell | What it says |
|---|---|
| `title` | what the button is called |
| `name` | what it is pressed by on a command line and in a text; its title written as a name, if it says none |
| `column` | the column a card must be in to be offered it |
| `who` | who may press it: `anyone` who may change cards (the default), or a `person` and not an agent |
| `scope` | a scope whoever presses it must also hold, if one is said (`admin`) |
| `holds` | whether the column's stage waits for it: a card there is taken up only once it is pressed |
| `again` | whether a press counts only for this time the card came to the column, so a card sent back must be pressed again; if not said, a press counts for good |
| `says` | what pressing it writes in the card's history; that it was pressed, if it says nothing |

```sh
mw record add shop actions title=Signed column=ready who=person holds=true
mw board press shop-4 signed            # as whoever this terminal is signed in as
mw board press shop-4 signed --back     # taken back, while the card is where it was pressed
```

The hold is the fleet's and the runner's; the client is just one place
to press from. The board keeps what was pressed on the card's own row
(`pressed`, beside `moved`), in the same write that says so
(`board.item.pressed`). Every way of pressing goes through that one
door as whoever presses, never as whoever applied the fleet: the client,
the command line, the factory's bot in a channel
(`@<bot> board press shop-4 signed`) and a blueprint's step
(`board.press`).

Before a stage begins anything for a card in its column, it asks whether
the card is held. The answer comes from the rows, the card's presses,
and the count of its moves, which says which time it came there
(`work.due`, `review.owed`, `work.owed` answer `held`). A held card asks
for no place and nothing is started for it. It says so once each time
it comes (`board.item.held`, in its history and to whoever the project's
`tells` says is told of `held`), and is due at once when the last thing
holding it is pressed.

- A row that holds nothing is a button whose press is kept and
  announced, for a blueprint to hear.
- A card deleted and restored from the Trash keeps what was pressed on
  it, since its row is kept whole and a restore is not a move. A press
  made for the time it came still counts, it is held by what is left as
  before, and it is not announced as held a second time.
- With no rows, nothing changes.

### Something more done with a card where it is: segments

A stage does a usual thing with each card in its column. To do more
there, add a row to the project's third table, `segments`. A row names
one small thing (a *segment*), the stage whose list it joins, and where
in that list it goes.

For example, run the project's tests before anybody looks at a card:

```sh
mw record add shop segments title=Tests stage=review order=5 does=std/run command='cargo test'
```

A card that comes to `review` is now handed to `std/run` first, which
runs the command where the card was worked:

- If the command passes, the card goes on to be reviewed.
- If it fails, the card goes back to whoever worked it, with the end of
  what was printed. It is counted as a card sent back is, and nobody is
  asked to look.
- With `with=fails=left`, a failing card is left in the column for a
  person instead, and the channel says why.

`std/run` keeps nothing as it goes. If the daemon stops while the
command runs, the command is run again from its start when the daemon is
back. A command that was cut off has not said that the card fails.

#### Around a review

Where a stage reviews cards, its usual list is two things:

| Order | Segment | What it does |
|---|---|---|
| 10 | `std/review` | Those who look. The stage begins it and hears what they decide. |
| 20 | `std/merge` | Puts approved work with the rest, where the stage's row says `merge`. |

Rows go around either. Here: something run on approved work before it
is merged; a second look by two others who both have to approve; and,
in place of the usual look, three people who look at cards labelled
`payments`, two of whom have to approve:

```sh
mw record add shop segments title=Licences stage=review order=15 does=std/run command='reuse lint'
mw record add shop segments title=Second stage=review order=12 does=std/review role=auditor seats=2 needed=2
mw record add shop segments title=Payments stage=review order=10 does=std/review labelled=payments seats=3 needed=2
```

The rules:

- A row that names what the stage usually begins for a review
  (`std/review`, or the blueprint its row of `stages` names) is one more
  review. It is begun and heard from the same way. Who looks and how
  many have to approve come from the row (`role`, `picks`, `seats`,
  `needed`), and from the stage's own row where the segment's row says
  nothing.
- Each review that passes has what comes after it in the list done: the
  next review begun, a thing called, the merge.
- A review that sends the card back, and any thing called that says
  `back`, sends it where the stage sends a card back, counted.
- A thing that says `left` leaves the card in the column and says why.
- A row at 20 that names a blueprint of the project's own puts approved
  work with the rest another way.

#### Where cards are worked

Where a stage works cards, its usual list is three things in a round
(`std/lane-round`):

| Order | Segment | What it does |
|---|---|---|
| 10 | `std/work` | An agent works the card. |
| 20 | `std/keep` | What it changed is kept as a commit. |
| 30 | `std/send` | The card's branch is sent to the remote, where the stage's row says `push`. |

Rows go among those the same way:

```sh
mw record add shop segments title=Tidy stage=work order=15 does=std/run command='cargo fmt'
mw record add shop segments title=Docs stage=work order=10 does=std/work labelled=docs with=agent=shop-worker-2
mw record add shop segments title=Look stage=work order=15 does=std/turn role=reviewer ends='approve=on send_back=back' with='told=Look at what was changed here with git diff and say whether it may be kept'
mw record add shop segments title=Asked stage=work order=10 does=std/work ends='done=on ask=asks'
mw record add shop segments title='Second look' stage=work order=15 does=std/turn role=reviewer seats=2 needed=2 ends='passed=on failed=back'
mw record add shop segments title=Release stage=work order=25 does=std/turn role=reviewer seats=2 needed=2 yes=ship no=hold
```

1. **Tidy** tidies what an agent leaves with the project's own command
   before it is committed. It runs in the card's place between the work
   and the commit, so what it leaves is in the commit. If it fails, the
   card goes back to whoever worked it, told what was printed, as often
   as the stage gives a card again (`rounds`). After that the card is
   left in `doing` for a person.
2. **Docs** sends cards with a label to one agent. It stands in for the
   usual work for cards that hold the label, and hands `std/work` what
   `std/work` has a param for.
3. **Look** has a second agent look at the work before it is kept. It
   is an agent's turn on the card (`std/turn`, which is also how
   `std/work` gives its agent the card), here of the role `reviewer`.
   The agent works in the place the card was worked (`dir`, which
   `std/work` hands on), is told the row's own words (`told`), and is
   offered the words it may end with, each with what it comes to
   (`ends`). `send_back` sends the card back to whoever worked it with
   what was said, as a failing command does.
4. **Asked** takes away `blocked`, the third word an agent working a
   card is offered when nothing is said. With `blocked`, an agent that
   could not do what it needed, because something it needed was refused,
   ends with `blocked`. Its card is left in `doing` for a person, with
   what it said, and is told of as `blocked` (`left_as`, which
   `std/work` gives out and the card's `card.left` says as `as`).

   Each word comes to one of the words a segment ends with (`on`,
   `back`, `again`, `asks`, `left`; see *Writing a segment*). If the
   agent chooses none of them, the first is taken.
5. **Second look** is a council in the work stage. Two agents of the
   role look at the work at once, each on its own, in the card's place
   before anything is kept. Each is offered `approve` and `send_back`,
   and both have to approve (`needed`). `ends` says what they come to
   together:
   - passed: the card goes on to be kept;
   - failed (as soon as one sends it back, where both are needed): the
     card goes back to whoever worked it, told what those who sent it
     back said, each as `<agent>: <words>`, and is counted as a card
     sent back by a command is;
   - undecided (neither, once all have spoken): the card is left for a
     person, unless `ends` says otherwise.

   Each who looks is asked once. If the daemon stops while they look,
   each is given the turn it had when the round begins again.
6. **Release** has two look once the work is kept. They are offered the
   words the row says (`yes`, `no`) and counted by them: both choosing
   `ship` passes it.

A round takes the rows as they are when it begins. A row added while a
card is in hand runs from the next round.

#### What a command is handed

A command a row runs (`std/run`) gets the card in its environment:

| Variable | What it holds |
|---|---|
| `CARD` | The card's id. |
| `CARD_TITLE` | What the card is called. |
| `CARD_LABELS` | Its labels, with a comma between; empty where it has none. |

It runs where the card was worked: the card's own worktree, or the
project's folder where the card is worked there. A row that says
`in=project` in `with` runs it in the project's own folder instead. For
example, a line in `CHANGELOG.md` for each card that is done, laid after
the merge where cards are reviewed:

```sh
mw record add shop segments title=Changelog stage=review order=25 does=std/run with=in=project command='echo "- $CARD_TITLE ($CARD)" >> CHANGELOG.md'
```

#### A table of test runs, by saying where

A row that runs a command may name a table to keep each run in
(`record_to`). Here the project's tests run before anybody looks at a
card, and each run, passed or failed, is a row of `test-runs`:

```sh
mw record add shop segments title=Tests stage=review order=5 does=std/run command='cargo test' record_to=test-runs
```

The table is made the first time a run is kept, as a table of the
project, listed under it in the side panel. It fills as cards come to
the command. Its rows are the command's and are not written by hand
(`by_hand: false`), so the client offers no row or column to write
there. Each row is one run:

| Field | |
|---|---|
| `title` | A run id: when the command began, in UTC, and the card (`20261122-140312-shop-7`). It is what a run is known by, and sorts by when it began. |
| `day` | The day it began, in UTC, as a date (`2026-11-22`), which a view filters and sorts by. |
| `time` | The time it began, in UTC (`14:03:12 UTC`). |
| `card` | The card it ran for, which opens it. |
| `result` | `passed` or `failed`, as the command's ending said. |
| `code` | The exit code; nothing where it ended with none. |
| `seconds` | How long it took. |
| `command` | The command, as the row says it. |
| `commit` | The commit it ran on, where it ran in a repository with history; nothing otherwise. |
| `printed` | The last forty lines of what it printed, four thousand characters at most. Left out of the table's view until somebody shows it there, and read whole in a row's form. |

Rules:

- It counts and never scores. Every run is kept, passed or failed. The
  card goes on, back, or is left exactly as it would with nothing kept.
  A run that cannot be kept (the table refuses it) changes nothing of
  that either.
- If the project already has a table of that name, it keeps its own
  fields, and only the fields of these it has are filled.
- A table is never written to if it says what writing or reading it
  takes, says what its rows steer, or is a model's (the board, a
  fleet's own tables). A row is kept there only by whoever writes it.
- A run cut off by a stop of the daemon ended nowhere and is not kept.
  The command runs again when the daemon is back, and that run is kept.
  A run stopped after it was kept and before the card went on is run
  again too, and kept again: each is a run that was made.
- What the row comes to is a block (`segments.recorded`). The commit is
  asked of git by an action (`place.head`).
- Saying where a command's runs are kept takes `shell`, as saying the
  command does: what the command printed is written there as whoever
  applied the fleet.

#### A command that runs long, told once

A row that runs a command may say how long it may run before somebody
is told (`warn_after`, a time such as `20m` or `1h30m`; nobody is told
where it says none). Here a run of the whole test suite that is still
going after twenty minutes is told to whoever the project's `tells`
names for `slow`. That is whoever applied the fleet, unless a row of
`tells` says otherwise:

```sh
mw record add shop segments title=Tests stage=review order=5 does=std/run command='cargo test' warn_after=20m
mw record add shop tells title=lead whom=@grace of=slow
```

- The line says which card, which command, and how long it has run so
  far.
- Nothing stops the command. It goes on and comes to what it would have
  come to.
- The time is a rule that watches the step that runs the command (a
  `watch` with `after:` from the row, on the conductor's clock). It says
  `card.slow` of the card, and `std/told` tells of it.
- It is told once for each time the card comes. If the daemon stops
  while the command runs, the command runs again from its start when the
  daemon is back; if that run also passes its time, nobody is told a
  second time (`once` of `inbox.tell`, which `tells.whom` gives from the
  card, the coming and the command).
- A card that comes to the command again (sent back and worked again)
  is another coming, and is told of again, which raises the line that is
  there.
- A row whose `warn_after` is not a time is refused when the card comes
  to it, and the card is left for a person, saying so.

#### Where a command runs

Where a card is to be worked in a worktree of its own and has none (a
row before the work, at an order under 10 where cards are worked, or one
after its worktree was tidied away), the command is not run. The card is
left for a person, saying so. A row meant for the project's folder says
`in=project`.

The command is never run in the project's folder instead: somebody may
be working there, and a command meant for a card's own copy of the work
(one that tidies or rewrites files) would run over what they have in
hand. Where a card is never worked here (it is planned, or reviewed
having come there by hand), the project's folder is where it runs.

On Windows the command is read by that system's shell, which puts what
a `%NAME%` holds into the line before it reads the line: what a card is
called would be read as part of the command. So a command there reads
the variable in a program of its own (a script, or the project's tool),
and is never `echo %CARD_TITLE%`. On Linux and macOS the shell reads the
line first, and nothing a `$NAME` holds is run ([status.md](status.md),
*Systems*).

#### Where cards are planned

Where cards are planned, and where a stage starts a blueprint of its
row's naming for each card, the usual list is one thing, at 10: whoever
says how the card would be done (`std/proposal`), or that blueprint. The
stage begins it, and it has the card for as long as the card is there.
Rows go before or after it the same way. A check that a card says enough
to be planned, and a look at the plan before the card goes on:

```sh
mw record add shop segments title=Triage stage=plan order=5 does=std/run command='./triage'
mw record add shop segments title=Lint stage=plan order=15 does=std/run command='./lint-plan'
```

- What is before is called first, and the usual thing is begun once it
  has passed.
- What is after is called once what was begun says what it came to and
  leaves the handing on to its stage. `std/proposal` does this when its
  stage tells it something is laid after (`hands`). A blueprint of your
  own does it by saying so, and moving nothing itself (below).

```yaml
  done: { do: emit, with: { topic: stage.came, card: "${params.item}", project: "${params.project}", stage: "${params.from}", came: on, says: sorted, hands: true } }
```

- The stage then goes through what is laid after. The card is moved to
  the stage's `pass`, with what was said (`says`), by something that
  first checks the card is still there as it was (`std/onward`). A card
  somebody moved meanwhile is theirs.
- Nothing is sent back from such a column. A thing that says `back` or
  `left` leaves the card where it is, and the channel says why.
- One thing is begun for a card there, so a row that names the usual
  thing again after itself leaves the card for a person.

#### A row's cells

| Cell | |
|---|---|
| `title` | What it is called, in what is said of it. |
| `stage` | The stage whose list it is in, by what that row of `stages` is called. A row that names none runs nowhere. |
| `order` | Where in the list. Where cards are reviewed, the review is at 10 and the merge at 20: a row at 5 runs before anybody looks, one at 15 once the card is approved and before it is merged. Where they are worked, the work is at 10, the keeping at 20 and the sending at 30. Where they are planned, or handed to a blueprint of the row's naming, that one thing is at 10. |
| `does` | The blueprint that does it: one that ships (`std/run`, `std/work`, `std/turn`, `std/keep`, `std/send`, `std/merge`, `std/review`) or the project's own. |
| `command` | A command of the project's own, for `std/run`. It runs as whoever applied the fleet, so writing one takes `shell`. |
| `record_to` | A table each run of `command` is kept in as a row, for `std/run`, made the first time as a table of the project (*A table of test runs, by saying where*, above). What the command printed is written there as whoever applied the fleet, so writing it takes `shell`, as `command` does. |
| `warn_after` | How long `command` may run before whoever `tells` names for `slow` is told, once, for `std/run`: a time such as `20m` (*A command that runs long, told once*, above). It stops nothing. It is a time, so writing one takes no more than writing the row. |
| `with` | Anything else the blueprint has a param for, each `name=value`. Not a cell the row has (that goes in its cell), nor what the stage hands it of itself (below). It is handed to what runs as whoever applied the fleet, and may be a command or the name of a secret the factory keeps, so writing it takes `shell` and `secrets:use`. |
| `role`, `picks`, `seats`, `needed` | For a review a row names: who looks and how many have to approve. Also for a segment that asks agents, where it has params of those names (`std/turn` with `seats` has that many look at once, a council). The stage's, where a row says none. |
| `ends` | For a segment that asks an agent (`std/work`, `std/turn`): what each word it may end with comes to, as `done=on ask=asks`. For a council, what those who look come to together, as `passed=on failed=back` (`undecided=left` where it says nothing). Words, so writing one takes no more than writing the row. |
| `yes`, `no` | For a council (`std/turn` with `seats`): the two words each who looks is offered and counted by, for the work and against it; `approve` and `send_back` if not said. Words, as `ends` is. |
| `rounds` | How often its segment is given again at most before the card is left for a person; 2 if not said. |
| `item`, `labelled`, `part_of` | Held to one card, to cards with a label, or to the cards of one feature. Of two rows at one order, the one held to more stands. |

The rows are changes to the usual list, not the list. So a project with
no row does exactly what it did:

- a row at an order between two usual things goes between them;
- a row at the order of a usual thing stands in for it;
- a row there with no `does` takes it out.

Writing a row takes what writing a row of `stages` takes
(`agents:write`, and being able to have written every row of `agents`).

#### Writing a segment

A segment is a blueprint that does one thing with one card. It is handed
`project` and `item`, and beyond those only what it declares a param
for:

| From | Params |
|---|---|
| Its row | The cells `role`, `picks`, `seats`, `needed`, `ends`, `yes`, `no`, `command`, `record_to`, `warn_after`, and whatever `with` says. |
| The stage's row | `role`, `picks`, `after_restart`, `restarts`, `remote` and `key`, and `fetch` and `base` where cards are worked. Its own row does not say these again in `with`. |
| The card's journey | `channel`; `note` (what was said as the card came); `title` (what the card is called); `labels` (what it holds, as a list, or with a comma between to a param that is text); `asking` (a word for this asking alone, for a segment that keeps nothing as it goes). |
| Where cards are reviewed | `from`, `to` and `fail` (the stage's column, and where it sends a card on and back). Once the card is approved: `by`, `words`, `approved`, `against` and `seats` (who approved it, what they said, and how the looking came out). |
| Where cards are worked | `back`, `answer`, `unkept`, `tries` and `most` (why the card comes to its agent, and how often it has); `worktree` (whether it is worked in a worktree of its own); `place` (the pool a place was taken in for the card); and whatever a thing done before it in the same round handed on (`by`, who worked it, and `dir`, where). |

Rows that are not run:

- A row that says something its blueprint has no param for is not run.
  The card is left for a person, and the channel says what the blueprint
  takes.
- A row that names something that itself goes through such a list
  (`std/through`, `std/segments`, `std/lane-round`) is not run either:
  it would go through the list inside itself. This is said where the
  fleet reports when the row is looked at, and a card that comes to it
  is left for a person with the same sentence. A round of your own under
  a name of its own is a thing done like any other.

A segment ends by giving out one word, `came`, and what goes with it:

| `came` | Means | With |
|---|---|---|
| `on` | Done: what comes next in the list runs, or the card goes on. A segment that gives no word came to this. | `says` |
| `back` | The card goes back. Where cards are reviewed, where its stage sends one (`fail`), counted. Where they are worked, to whoever worked it, in another round, as often as the stage gives a card again. | `says`, the words it goes back with (or `why`) |
| `again` | The same segment once more, as often as its row's `rounds` allows; then the card is left. | `why` |
| `asks` | It asked instead of doing. Where cards are worked, the question is put on the card, which waits, and the round its answer begins starts at the segment that asked. Where they are reviewed nobody takes a question up, and the card is left for a person with the question said. | `asker`, `question` |
| `left` | Left for a person. | `says`, which the channel is told; `why`, which the card keeps |
| `nothing` | The card was gone from under it: nothing more is done for it. | |

Any other outputs of a segment are handed on to whatever comes after it
in the same round and declares a param of that name.

A segment that cannot be called at all (it does not read, or the call is
refused) came to `left`, with why. So the least a project writes is two
params and its steps:

```yaml
blueprint: licence
params:
  project: { type: string }
  item: { type: string }
steps:
  look: { do: shell, allow_failure: true, with: { cmd: [reuse, lint] } }
outputs:
  came: "${ steps.look.ok ? 'on' : 'back' }"
  says: "The licence check did not pass."
```

#### `std/turn`: a segment that asks an agent

`std/turn` is an agent's turn on the card, and keeps nothing as it goes.
It takes:

- who: `role`, `picks`, `agent`;
- where: `dir`;
- what the agent is asked: `prompt` whole, or the row's own words
  (`told`) with the card, in `prompts/turn.md`;
- which conversation it is: `thread`, put before the card's id;
- what becomes of it at a restart: `after_restart`, `restarts`,
  `asking`;
- the words it may end with, each `word=what it comes to`: `ends`
  (`done=on ask=asks` if not said). What a word comes to is decided by a
  block (`turn.came`), never in a text.

If the daemon stops in the middle of it, whatever called it calls it
again, and its agent is given the turn it had: the agent is asked once.

A project's own segment runs only as it was vouched for: saved through
the factory, or applied by somebody who holds what the fleet runs with.
One that nobody vouched for is not run, and the card is left with a
sentence that says so.

#### How a list is gone through

Where cards are reviewed, the stage that has the card's column has the
list gone through, in the card's turn, by something that keeps nothing
(`std/segments`). It calls each thing in order until there is a review
to begin, and once that review has passed, goes on from where the review
was.

- A card with nothing laid before the review has none of the first
  part: its review is begun at once. So a row costs only the cards it is
  for, and a row laid after the review costs nothing before it.
- Where cards are planned, or handed to a blueprint of the row's naming,
  it is the same around the one thing begun there.
- What the things done before came to is said as an event of the
  stage's own, `stage.came`. It carries the `card`; `came`, which is
  `review` where what is usual begins; the `row` and `order` of that, or
  of what said the word; and `says`. Where a stage that begins one thing
  for a card is to hand the card on itself, it also carries `hands`.

Where cards are worked, the round calls the things one after another and
keeps nothing as it goes. If the daemon stops in the middle of one, the
stage begins the round again from the first thing in its list, which
finds done what was done: its agent is given the turn it had, and what
is committed is not committed again. A failure in one of them fails the
round, and the card is left in `doing` saying why.

Either way, the first thing is called by what goes through the list, and
the rest by `std/through`. That calls the next thing, asks what comes
after it, and calls itself with that: one level of the tree for each
thing done.

#### How many things at most

A stage does so many things at most with a card in one going through its
list. Its row says how many (`most` of `stages`):

- 6 where cards are worked and 5 where they are reviewed or planned, if
  it says none;
- 0 for no bound but the list's own length. A list is gone through one
  level of the tree per thing, so a list of more than about a hundred
  and twenty meets how deep a tree may go first (*Composition*).

A card whose list holds more is left for a person, and what is said
tells how to raise the number on the stage's row:

```text
shop-7: there is more to do with it in ready than is gone through at once (6 things at most), and Announce was not begun: it is left for a person. Take a row of `segments` away, put two of them in one blueprint, or raise how many are done: `mw record set shop-stages-1 most=7`
```

A review, begun and heard from by the stage, is not counted: it ends the
going through, and what is done once it has passed is another going
through. The number is read when a going through begins, as the rows
are (below). A stage changed while a card's list is gone through holds
the next card to the new number, and the card under way to the one it
began with. A round begun again after the daemon stopped starts again
from its first thing, so it is held to the number as it is then.

#### What a going through is fixed to

Three rules follow, and hold for a segment of your own:

- **What a list is gone through with is fixed when the going through
  begins**: the rows as they then are, and for each thing its name and
  what it is handed, which are one answer (`segments.next`). A row
  changed or added meanwhile is for the next coming of a card: the next
  round, the next card's review. What is done once a review has passed
  is what was laid after it when that review began; where nothing was,
  what is usual is done by itself.
- **Stopped in the middle, it is like any called blueprint**: one that
  keeps nothing (`durable: false`) is called again under the same name
  when the daemon is back, and is handed the same `asking`; one that is
  kept goes on where it was.
- **A card moved while its list is gone through has left the list.**
  What the list came to is then nothing's: the card's move is what
  counts, as it is for a round.

A whole round of a project's own, written when a round was one text (a
copy of `std/lane-round` as it was then), is named in a row at the place
of the work (`order: 10`) under a name of its own. It is handed what a
round was always handed, does all of it, and hands its card on itself.
Nothing after it in the list is done for that card.

Not there yet:

- A review whose way of reviewing is changed while it is in session
  begins again at the first of the card's list, so what a row lays
  before it is done once more.
- With nobody in a reviewing stage's role, nothing is run at all: the
  card waits for a person.

### A plan's tickets become cards

`std/plan` takes a document of the project that ends with its tickets as
a block,

````markdown
```tickets
[
  { "title": "Lay the floor", "body": "What is wanted, and how one would know it is done." },
  { "title": "Put the shelves up", "after": [1] }
]
```
````

and adds a card for each ticket to the board's first column:

- Each card says which cards it waits on. `after` lists tickets by their
  place in the block, counted from 1, and becomes the ids of their
  cards.
- With `ready`, the cards are then made ready, once each says what it
  waits on and not before.
- The block is replaced, in the document, by the list of cards it came
  to. So the plan says which cards are its own, and is never added
  twice. A block written into it later is added the same way.
- A block that cannot be read is said in the channel, with why, and the
  document is left as it was written.

The fleet calls it for whatever is written under `docs/plans/`. To do
the same for one document by hand: `mw apply std/plan --project shop
--param project=shop --param path=docs/plans/cart.md`.

### Changing a fleet while work is under way

Change a row. Each change to either table is an event
(`record.created`, `record.updated`, `record.removed`, with the
`project` and the `collection`), which the fleet hears. It brings itself
to what the tables now say (`std/arrangement`), then says
`fleet.arranged`, with the `project`, how many `agents` and `stages` it
now has, and what it was `asked` for (`apply`, or `row` and the number
of the change it heard). Anything else that is to follow a fleet being
changed listens for either: for what was written, or for the fleet
having taken it. The tables are read whole each time, so a change is
taken by the arrangement it asked for or by a later one, which says a
greater number. Two changes made together may be taken by one.

```yaml
on:
  rearranged:
    event: fleet.arranged
    for: { project: shop }
    do: channel.post
    with: { channel: ops, body: "shop now has ${event.data.agents} agents." }
```

What is asked for from then on is done the new way:

- **More of a row**, or a new row: its agents are brought up, and the
  stages of their role make room for as many cards as there are now
  agents for.
- **Fewer of a row**, or a row taken away: an agent no longer counted
  takes no more work and leaves when it has finished what it has in
  hand. Nobody's work is cut off because there are to be fewer of them.
  (To have one gone at once, stop it by name.)
- **A stage changed**: it takes what its row now says, where it is, when
  it has finished what it was in the middle of. A card that is being
  merged is merged.
- **A stage taken away** is stopped, and so is what was under way there:
  this one does stop work, and the channel is told.

A review that was waiting its turn is begun as now arranged. What
becomes of a review *in session*, when how many look or how many are
needed has changed, is its stage's `in_flight`:

- `finish`, the usual: it is finished as it was begun. A council that is
  looking at a card reaches its decision as a council, and the next card
  is reviewed the new way.
- `restart`: it is stopped where it is, and its card is reviewed again
  from the start as now arranged.

The stage says the new arrangement as an event (`review.arranged`, with
its `stage` and `in_flight`), and each review takes it from there; one
of your own does the same, or ignores it.

A blueprint named as `review` takes the params `std/review` does and
says one of `review.passed`, `review.failed` (`card`, `project`, `by`,
`words`) or `review.undecided`. One that says which review it is with it
(`review`, its key) is ended by the stage once that is taken in, as
`std/review` is; one that does not ends itself.

Whatever reviews, what it comes to is on the card and in its history.
It is said in the project's channel too, where the stage's row says
`reports: verdicts`. This is the same for the fleet's own agents and for
a blueprint that has cards looked at somewhere else and says what was
said there as `by` and `words`.

What the tables start with is the fleet's to say and no more: applying
the fleet again changes neither. To change what a stage *is*, copy the
text: `mw own std/reviewing`, or whichever kind it is.

### When the daemon is stopped in the middle

A daemon stopped while agents have cards (by `down`, by an upgrade, by
the machine going off) is started again and finds their turns in hand.
Three cells say what becomes of each. Unless one is changed, everything
goes on.

#### An agent's `outlives`

`outlives` says whether the agent's program is tied to the daemon.

- `off`, the default: the program ends when the daemon does.
- `on`: the program is started by a wrapper of its own that the daemon
  only follows. What it prints goes to files in the daemon's data folder
  (`run/turn.<id>.*`). These are readable by whoever runs the daemon and
  nobody else, and are written with every key and sign-in the turn was
  handed already taken out, as anything a daemon writes down is. How the
  program ended is left there too. It is behind the wall it would have
  been behind, shown nothing more, and is not told where those files
  are. The next daemon reads what it said from the first line and waits
  for it where it is, or finds it finished and takes its answer.
  Nothing a program leaves there is believed further than what it
  prints: it is read as its answer, by the same rules.

Saying `on` takes `shell`, in a fleet's row and for an agent made by
hand alike (`mw agent add --outlives`): it lets a program at this device
with nobody watching. Saying `off` again takes no more than changing the
agent does.

#### A stage's `after_restart`

`after_restart` says what becomes of work that was *cut off*: a turn
whose program ended with the daemon.

- `again`, the usual: the turn is taken again by its agent, which is
  asked again in the conversation it had, and the work goes on from
  there. For a review, each seat that was waiting for its reviewer goes
  on waiting, and nobody chooses again who looks. A seat that was called
  off because enough had decided chose neither, and is not asked again.
- `leave`: it is not begun again. The step that asked for it fails,
  saying that the daemon was stopped while the agent had it and that it
  was asked to be left. A card that was being worked stays in `doing`,
  one in review stays in `review`, and the channel is told why. Moved by
  a person, it is taken up as any card is.

An agent whose program outlives the daemon was not cut off, so its work
is taken up whatever `after_restart` says.

#### `restarts`

`restarts` caps both. Each time a daemon starts and finds the same turn
in hand is counted on the turn. After more than the stage's `restarts`,
the turn is left as `leave` leaves it, and its program is stopped if it
was still going by itself. So something that brings the daemon down
every time it is tried does not go round for ever.

#### Timeouts and programs that went on

- A step's `timeout` still holds for a program that went on by itself.
  If that time has passed when the daemon is back and the program is
  still at work, it is stopped and the step fails saying so. If it
  finished within its time, its answer is taken however long ago that
  was.
- A program that went on by itself and is gone without a word (killed
  outright with what kept it, or the machine was restarted) was cut off
  after all, and `after_restart` applies: begun again, or left.
- A program that ended by itself, however badly, said how, and its turn
  ends as it would have with the daemon there.
- A program nobody asks for again (a person finished or stopped its card
  while the daemon was down) may finish, and is stopped when the time it
  was given runs out. It moves nothing.

A copy of a factory (`up --from`) takes up nothing. Turns that were in
hand in the factory it was copied from are ended in the copy, saying so,
and the other factory's programs are not looked for.

#### What is shown

- `agent.turn.started` says `again` (how many times now) and `taken_up`
  (whether its program had gone on by itself).
- `instance.stopped` says `why`: `asked`, `parent`, or `restarted` for
  one that keeps nothing and was over at a restart.
- The card's history has a line for it.
- `mw agent list` says of each agent whether it goes on without the
  daemon.

#### `agent.task` in a blueprint of your own

Your own blueprint says the same of a turn it asks for, with
`after_restart` and `restarts` of `agent.task`; the step's `again` is how
many times it happened. A blueprint of your own that a stage starts (a
word of its own in `does`, or a `review` or `proposal` that is not the
one that ships) is started with what it was always given, and not these
two.

A daemon stopped while the agent has the task finds it in hand when it
starts again:

- With `after_restart: again`, the usual, the task goes on. The step
  waits for the same turn, which the agent takes again (or, where its
  program outlives the daemon, is found still at it or finished), and
  gets its answer as if nothing had happened. A step of an instance that
  keeps nothing, run again, is given that turn too, not a second one.
- With `after_restart: leave`, the step fails instead, as something not
  safe to retry.
- `restarts` is how many such stops at most before it is left whatever
  was said (3). The step's `again` says how many there were.

With `channel`, the agent says its reply in that channel when it has
finished, as itself: the message is the agent's, not a notice from the
blueprint that asked. The blueprint needs `channels` as well as
`agents`, and a channel that is not there fails the step before the
agent is asked anything. A task that fails says nothing there: the step
failed, and what is said of that is the blueprint's to say. Without
`channel`, what the agent said is in the step's `reply` and nowhere
else.

With `options`, the work may end more than one way and the agent says
which. It is asked to end on a line `DECISION: <done | ask>`. The choice
is the step's `decision`, and the line is not part of `reply` or of what
is said in the channel. An agent that chose nothing has not failed, as
with `agent.consult`: `decision` is null, and the blueprint treats that
as whichever it takes as usual.

### Saying something on a record: `record.comment`

`record.comment` says something on a record of any collection, a card
included (`id`, `body`). Nothing of the record is changed. It is an
event, `record.commented`, with:

- the record's `id`, `project` and `collection`;
- the `body`;
- `by`, who said it as the journal names them: `user:<id>`,
  `agent:<slot>`, or `key:<id>` where it was said with a key somebody
  made, which may hold less than the account it is of. The event's
  `source.actor` says the same, and `actor.holds` asked of it answers
  for the key;
- `through`, when a blueprint's step said it.

It is a line of the record's history where the collection's `history`
counts that topic (`record.commented: "{actor} said: {body}"`). A
collection that counts it nowhere refuses it, saying so, since it would
be kept nowhere. It needs what changing that kind of record needs:
`board:write` for a card, `records:write` for any other. This is asked
when it is done, not when the blueprint is applied.

Over HTTP it is `POST /api/records/{id}/comments`, and from the terminal
`mw record comment <record> <words...>`. What follows from something
being said is not the daemon's to decide: that a card has a question
open, and that what is said then is the answer, is `std/cards` (*Lanes*,
above).

What was said on a record is read in its history (`GET
/api/records/{id}/history`), among what happened to it. How many things
were said on each record of a table is one question, answered for the
table: `GET /api/collections/{project}/{name}/commented` gives
`commented`, each record's id with how many, and no record that nothing
was said on. It is told to whoever may read the table's records and to
nobody else. It is how a list of records shows which were commented on
without asking for each one's history.

#### Public and private

What is said is private unless whoever says it marks it public:

- `public: true` beside the words, in a step and over HTTP;
- `mw record comment <record> --public <words...>` from the terminal;
- in the client, a switch beside sending in a card's drawer, off until
  it is turned on, and off again for the next thing said.

The event then says `public: true`, and says nothing of it otherwise, so
a handler that is to hear only what was marked says
`for: { public: true }`. The line of the record's history keeps the
mark and says so (`Ada said: It ships on Friday. (public)`), whatever
words the collection has for something said.

That is all the daemon does with it. The mark is for a blueprint that
also keeps a record somewhere outside the factory, which tells nobody
there what was not marked. It asks nothing more of whoever says a thing
than saying it does, and it lets nobody read anything. What is said on a
record of a table kept private (`reads`) is heard, and read in its
history, by whoever holds what the table is kept under and by nobody
else, marked or not.

### Asking what an actor holds: `actor.holds`

`actor.holds` says whether whoever an event says did something holds a
scope now.

- `actor` is the event's `source.actor` (`user:<id>`, `key:<id>`,
  `agent:<slot>`). The route that took the request sets it, and nothing
  anybody sends can.
- `scope` is `area` or `area:verb`.

It gives:

- `holds`;
- `known`, whether there is such an actor now. An account that is gone,
  a key that was withdrawn, a caller, the factory itself and nobody at
  all hold nothing. A key is answered for what the key was given, not
  for what its maker holds;
- `kind`, what the actor says it is before its colon (`user`, `key`,
  `agent`);
- `name`, what such an actor is called, for a blueprint that says who
  did something by name.

Asked `who: person`, an agent is answered as not holding the scope
whatever its slot was given, for something that is a person's to do.
`refused` then says which it was: `agent`, or `scope` for anybody else
who does not hold it. (`std/feature` asks it so of whoever moved a
feature on, unless the project's features say `accepts: anyone`.)

It is for a blueprint that does something, with its own authority, for
whoever did something else. The instance holds what whoever applied it
holds, and this is how it asks whether the one it acts for could have
done it themselves (`std/pass-by-hand` asks it before it merges for
whoever moved a card). Whoever asks has to hold the scope asked about,
so that nobody learns by asking what they could not do.

The events that name the key, where something was done from outside
with one, are a card being added, changed and moved
(`board.item.created`, `board.item.updated`, `board.item.moved`) and
something being said on a record (`record.commented`); each says the
same in `by`. Any other event names the account the key is of, and
asked of one of those, a key that holds less is answered as its maker.

### Keeping two sides in step: `sides.cross` and `sides.owed`

`sides.cross` and `sides.owed` are for something kept in two places (a
row here, and whatever stands for it somewhere else) that has to be kept
the same in both. Where the two differ, nothing in them says which
moved. So `sides.cross` is handed both sides (`here`, `there`: the same
fields by the same names) and what the two last agreed on (`last`), and
says what is to cross:

- `goes`: the fields to be said there;
- `comes`: those to be said here;
- `sends` and `takes`: their names;
- `agreed`: what to write down as agreed once that is done;
- `came`: one word for it (`same`, `out`, `in`, `both`, `held`).

A field moved on the side that no longer says what was agreed. Where
both moved, or nothing was ever agreed, the side `prefer` names is taken
(`here` or `there`: it is asked every time, and the block has no
opinion). What was sent is what is agreed, so it never comes back as a
change.

`takes` says whose changes on the other side are taken:

- `everyone`, unless said;
- `trusted`: only where `trusted` says the other side, as it stands, was
  written by somebody who is; a change by anybody else is held;
- `nobody`: this side only goes out, and what moves there alone is
  neither taken nor written over.

While a change is held, nothing crosses either way. `holds` is `waiting`
(what to keep beside what was agreed, in the same write), and `newly`
says it came to be held just now, to be said once. `held` is what is
kept, handed back each time; whoever may says which side stands by
writing `take` or `keep` there.

`sides.owed` does the same for things that are said and never changed.
It is handed `things`, each naming itself in `by` (`id`, unless said),
and the names of those that have crossed (`had`, and `also` for those
that crossed the other way). It gives `those` that have not, each with
`kept` beside it: the names to write down once that one has crossed. So
a row's list is written one thing at a time, with nothing worked out in
the text.

Neither knows what the sides are. A package's blueprint that keeps a
project's cards in step with a service asks them
([extensions.md](extensions.md)).

### A copy of the factory: `factory.snapshot`

`factory.snapshot` writes a copy of the whole factory to a folder, for a
new factory to be started from
(`mw up --profile <name> --from <folder>`). `std/snapshot` is the
blueprint that runs it: `mw apply std/snapshot --param to=<folder>`.
The factory is only read. It takes
`admin`, which only the factory's owner has: the folder holds everything
the factory does. [profiles.md](profiles.md) says what a copy holds and
what it leaves out.

### Restarting the factory: `factory.restart`

`factory.restart` stops the factory and has what keeps it running start
it again, so that a program installed meanwhile is the one that runs:
for example, a pipeline's last stage after it installed one.

- It first waits until nothing else is under way (`at_rest`, the
  usual): no run but its own and those that wait on it, and no agent's
  turn in hand. It looks again each time one of them ends, on no timer.
  So a run that waits for something nobody does keeps it waiting for as
  long as the step's own `timeout` allows.
- It is refused where nothing keeps the factory running (a daemon
  started with `mw serve`, not `mw up` or the desktop app), since a
  stopped factory would not come back.
- The run that asked is not cut short by it. The daemon that comes back
  takes it up, and the step answers `came: restarted` there, with the
  `version` that runs now.
- Stopped anywhere from the step's beginning on, it comes to one start
  of the factory after it, and is said (`system.restarting`) where it
  was asked.
- It takes `admin`, as `mw down` does.

### Landing work: `place.gather` and `place.forward`

Work is landed by two actions of a piece of work's place (`places`).

`place.gather` gathers other items' branches (`items`, each
`lane/<item>`), and any branch the repository has (`branches`), into an
item's place of its own, cut from the project's branch (`onto`). Each is
merged in turn as a commit of its own; an item's commit says which item
in its last line, as `place.merge`'s does. It is taken whole or not at
all: where one does not go with what was gathered before it, nothing of
that asking is kept, and it answers `conflict`, `with` the branch and
what git `said`, for whoever's work it is to put right.

`place.forward` then brings a branch of the project (`onto`, or the one
its folder has checked out where none is said) forward to what was
gathered:

- only where that is a fast-forward (`diverged` otherwise);
- keeping what somebody changed in the folder and never committed
  (`stuck`, with git's words, where the change would be overwritten);
- and sends it to a `remote` where one is said, as the key the step is
  handed where it is handed one.

Each is idempotent (the same done twice), so a pipeline stopped anywhere
in either comes to the same commits.

Whether all the items gathered were passed by whoever looks at them, and
so may be landed, is a block's to say. `items.passed` is handed the
items, the rows of the table where they are (`rows.read`), where an item
goes once it is passed, and where it waits. It answers `passed`, `back`
or `waiting`.

### The profile's variables: `profile.set`

`profile.set` sets the variables of the factory's profile, which every
expression reads as `profile.vars.<name>`: `with: { vars: { api:
"https://api.example", replicas: 3 } }`.

- Each value is plain (text, a number, true or false); anything else is
  refused with a sentence.
- The whole set is replaced: a variable left out is no longer set, and
  `vars: {}` clears them.
- `std/profile` is the blueprint that runs it: `mw apply std/profile
  --param 'vars={"api":"https://api.example","replicas":3}'` (a
  `--param` whose value reads as JSON is that, which is how a map is
  given).
- It takes `admin`, which only the factory's owner has: it changes what
  every blueprint in the factory does.
- The variables are the factory's and stay until they are set again,
  whatever becomes of the instance that set them. Nothing is run again
  because they changed.
- Each change is an event, `system.profile`, naming what was set,
  changed and removed, and by whom.
- They are not for secrets.

[profiles.md](profiles.md), *The variables of a profile*, has the whole
of it.

### Actions a package brings

An installed extension package may bring actions too, named
`<package>.<name>`. Each is one way of running that package's program,
with no shell, as the package wrote it down
([extensions.md](extensions.md), *Actions a package brings*). They are
listed and checked with the rest for as long as the package is
installed. A blueprint that does one where the package is not installed
is told which package to install.

### What a command finds in its environment

Whatever a blueprint runs, a `shell` step or a service, has these in its
environment:

| Variable | What it holds |
|---|---|
| `FACTORY_CLI` | The path of `mw` itself, wherever it is installed and whatever it is called there, so that a script of your own can run it (`"$FACTORY_CLI" store serve ...`). A step or a service that is this program and nothing else says so without a script, as `cli:` (above), which is how `std/store` hosts a database. |
| `FACTORY_URL` | Where the daemon itself answers, as `http://127.0.0.1:7700`, for a blueprint that puts the daemon within reach of something else (a proxy in front of it, another device) and has to say what is to be reached. A daemon that listens on every address gives its device's own. |
| `FACTORY_PROFILE` | What the factory's profile is called, as `profile.name` is to an expression; empty for a factory with none (`[ "$FACTORY_PROFILE" = prod ]`). |

No credential comes with any of them: a command that talks to the daemon
has to sign in as someone. A step that sets one of them in its own `env`
is given what it set. The variables of the profile (`profile.vars`) are
not put there: a blueprint passes a command what it wants it to have,
with `env:`.

### A project's pipelines

A project's **pipelines** are its own blueprints kept under
`pipelines/` (`@blueprints/pipelines/<name>.bp`, called
`pipelines/<name>`).
`mw apply std/pipeline --project shop --param project=shop` runs them
as the rows of the project's `triggers` say. A row names the pipeline
and what runs it:

| `on` | What runs it | What the run is handed, where the pipeline takes it |
|---|---|---|
| `hand` | Somebody saying `pipeline.asked` from outside (`mw emit pipeline.asked project=shop pipeline=pipelines/gates`), who holds `blueprints:run` and what the pipeline says it runs with (`permissions`; managing the factory where it says none) | what they said as `with` |
| `branch` | A branch of the project's repository moving: the one `branch` says, or any where it says none (`project.branch.moved`) | `branch`, `commit` |
| `column` | A card coming to the column `column` says | `item`, `column` |
| `every` | So long after it last ran: `every: 6h`, a minute at the least | nothing |
| `at` | A time of day with its zone: `at: 02:00 Europe/Zagreb` | nothing |

#### What a row may say

- **`with`**: what more its pipeline is handed, each `name=value`, read
  as the kind of thing the pipeline's param of that name is
  (`with: [target=prod, times=3]`). So a pipeline that has to be told something
  can be triggered. What happened is handed over it, and what somebody
  who asks by hand says over that.
- **`environment`**: the project's environment its runs are under
  (`environment: prod`). The pipeline is called under it (the step word
  `environment:`), and that environment's rules hold as it starts (who
  approves, the branches it runs). Saying it takes
  `environments:secrets` beside `blueprints:run`, as saying the rules
  does.
- **`key`**: the key a run is called under. Where it says none, it is
  the pipeline's name with each `/` a `-`, and the environment's name
  after it where it says one: an instance is under one environment for
  its life.
- **`paused`**: a paused row runs nothing.

Where the environment's rule says who approves, the run first asks one
of them to let it go on (`approval.ask`), unless each of these is one of
them: whoever applied `std/pipeline`, whoever last wrote the row's
`environment` (which the factory keeps on the row as `environment_by`, a
field that says `writer_of: environment`), and whoever asked for it by
hand. Declined, it runs nothing.

#### How runs go

- A run is the pipeline called under the row's `key`.
- A run asked for while another under the same key goes on waits for
  it, in the order they were asked.
- No more than `at_once` keys run at once: a param of `std/pipeline`,
  two unless said. They take places in the project's pool,
  `<project>/pipelines`.
- Writing a row takes `blueprints:run`: what it runs is run as whoever
  applied `std/pipeline`.
- What cannot be run (a pipeline that is not there, a param it does not
  take, somebody who does not hold what it runs with) is said as
  `pipeline.refused`, with why. Whoever asked by hand is told it in
  their inbox.

#### Finding the run you asked for

Somebody who asks by hand may name their asking (`asked`, any text, as
the Pipelines page's Run does). It is carried on what fires the run and
on a refusal, and the run the pipeline's call began is then found by it
(`GET /api/pipelines/begun?project=&asked=`). So two askings at the same
moment each find their own.

### Features: work that takes several cards

`mw apply std/features --project shop --param project=shop` gives a
project **features**: a table of its own that is a board of its own,
for what is wanted that takes several cards to do. Each feature has a
blueprint of its own (`std/feature`), which is its pipeline:

| Where it is | What happens |
|---|---|
| `backlog` | An agent of the role that plans (`role`, `architect` unless said) reads it and asks what it needs to know, or says how it would be built: a plan that ends with its tickets, the cards it comes to and which wait on which. What it asks is the feature's `question`; what somebody then says on the feature is put to the same agent. |
| `planned` | It waits with its plan for somebody to accept it by moving it on, or to say on it what should be otherwise. |
| `building` | Accepted, where whoever moved it there holds what changing a card takes and, unless `accepts` says `anyone`, is a person. A card is added to the project's board for each ticket that is new, each saying which cards it waits on (`after`) and which feature it is a piece of (`part_of`), and with `ready` (unless said not) made ready, to be worked and looked over as any card is. A plan that lists no tickets, of a feature that gathers nothing, comes to one card, with the plan on it. |
| `done` | Every one of its cards is done. |

- The columns are the blueprint's `columns`, in that order.
- The tickets a plan ends with are the block a plan that is a document
  has (`std/plan`).
- What an agent is told about how a feature is planned is the `told` of
  `std/feature`. A project that wants another pipeline altogether has a
  file of its own under that name.
- The features are a page in the client like any table, drawn first as
  their board. A feature is accepted there by being carried to
  `building`.

#### Cards that are there already

A feature may gather cards that were written up before anybody put them
in order:

- `gathers`: their ids;
- `labelled`: a label. Every card with it that is not done is gathered
  when the feature is first planned, and written into `gathers` then, so
  that the plan and whoever accepts it speak of the same cards (the
  feature's history says which those were).

```sh
mw record add shop features title="Open the shop" status=backlog gathers=shop-3,shop-4,shop-7
mw record add shop features title="Open the shop" status=backlog labelled=opening
mw record set shop-features-1 status=building     # accept its plan
```

Its agent is shown them with the feature, each as it stands (where it
is, what is wanted, what it waits on and is part of). A ticket of its
plan may be one of them: `{ "card": "shop-3", "after": [1] }` is the
card shop-3, which then waits on the first ticket, where a ticket with a
`title` is a new card.

- `after` lists what a ticket waits on, each a place in the block or the
  id of any card of the board (`[1, "shop-9"]`).
- A ticket that is a card and says no `after` leaves what that card
  waits on as it is. One that says `after` has the card wait on exactly
  that, and on whatever it already waited on outside the plan.
- A gathered card no ticket names is taken as it stands.
- Accepted, each card there already is given what it waits on and says
  which feature it is a piece of, and nothing else of it is changed. It
  is made ready only where it had not been begun, and one that is done
  is left, with nothing waiting on it.

#### When a plan does not fit

The plan is checked against the board before anything is written
(`tickets.fit`), and is taken whole or not at all. It does not fit
where:

- a ticket is a card the feature does not gather, or one that is gone,
  or one named twice;
- a ticket is neither a card nor has a title;
- it waits on a place no ticket is at, or a card that is not there;
- a card is already part of another feature;
- anything would wait on itself by any road, counting what cards
  outside the plan already wait on.

Then, as where whoever moved it on may not accept it, the feature says
why (`feature.unaccepted`, a line of its history, and where it reports),
goes back to where it waits with a plan, and no card is added or
changed. Somebody says on it what should be otherwise, or puts the board
right, and moves it on again.

A plan being taken up when the daemon stops is taken up again when it is
back, and makes nothing twice. A ticket whose card was added before the
stop is that card (`tickets.fit` finds it by its title among what is
already part of the feature), and what a card is told it waits on is the
same told twice.

#### The blocks that decide

Three blocks do the deciding. Each is handed plain values, and none
knows a feature or a card:

- `tickets.named` says in words the rows a plan is to put in order, for
  whoever plans. It takes `rows`, `ids`, `label`, and `over` (what the
  state cell says of one that is done). It gives `shown`, `ids`,
  `count`, `listed`, `missing`, and `fix` (whether the label brought any
  that are to be written down).
- `tickets.fit` says whether a plan fits. It takes `read` and `unread`
  as `tickets.read` gave them, `tickets`, `may`, `rows` and `of`. It
  gives `came` (one of `fits`, `unfit` and `whole`), `why`, and `new`
  (the tickets a row is to be made for).
- `tickets.placed` says, once those are made, what every one waits on
  and is part of. It takes `made` beside the rest, and `begins` (what
  the state cell says of a row not yet begun). It gives `links`, `ids`,
  `listed`, `count`, `ready`, and `told` (what changed for the rows
  there already, in sentences).

Each reads a row by the cells whoever asks names (`waits`, `within`,
`state`, where they are not `after`, `part_of` and `status`).

### What is written inside an argument

An argument is often a mapping with words of its own: the `fields` of a
table, each by a name with its `type` and whether it is `quiet`; a
service's `health`, which is `tcp`, `http` or `cmd`. The action reads
those words, and the language knows none of them. So an action says what
it reads there: each key with a sentence, the few words a key may be,
and what a key holds in turn. It is said once, where the action is
declared, and everything that shows it is made from that:

- the editor in the client says what a key or a word means when it is
  pointed at, and offers the keys a mapping does not have yet as it is
  written (below, *Writing one*);
- `mw actions <name>` lists it under the action's arguments;
- the schema for an editor of your own carries each sentence;
- this table is held to it by a test.

It describes and never decides. Nothing is checked by it when a
blueprint is read, checked or run, and no editor marks a key for not
being listed. A key that is not here is one its action says nothing of,
and whether it is taken is the action's to say when the step runs. A
blueprint of your own may be older than what an action now says of
itself.

How to read the table:

- **Written** is a path from the argument down, its parts joined by
  dots: a key by its name, and `*` for a thing known by a name of your
  own choosing (a field, a view, a cell).
- **Is** is the kind of value a key takes, or one of two words: `each`,
  of a path that ends in `*`, whose sentence says what each such thing
  is; and `word`, one of the few words the key before it may be
  (`fields.*.type.list`), or that each thing of the `*` before it is
  (`views.*.figures.*.sum`).
- A list of mappings is described as one of its mappings is.

| Action | Written | Is | What it is |
|---|---|---|---|
| `wait` | `say.topic` | string | where it is published; whatever else is written is what the event carries |
| `inbox.tell` | `parts.kind` | string | what the part is |
| `inbox.tell` | `parts.kind.links` | word | addresses, each shown as a link |
| `inbox.tell` | `parts.kind.cards` | word | the cards of a board in one column as it is told, each listed where it is when it is read, or the cards holding one label; each to be passed or failed where it is read while it is where the part says |
| `inbox.tell` | `parts.kind.message` | word | one message of a channel, opened where it was said: in its thread where it is in one |
| `inbox.tell` | `parts.kind.rows` | word | rows of a table of a project's, by their ids or by what a cell says, with the columns named, each drawn as its table draws it, as whoever reads the line may read the table |
| `inbox.tell` | `parts.kind.runs` | word | the runs of an instance, of what ran under a key, or of what ran under an environment: what each is a run of, how it went, when and how long, each opening its run's page |
| `inbox.tell` | `parts.kind.measures` | word | measurements by their names and labels, each its latest value beside the median of its last few |
| `inbox.tell` | `parts.tab` | string | the tab of the line's page it is in, by its name; the parts that say none are on a first tab with no name |
| `inbox.tell` | `parts.links` | list | of `links`: the addresses, each a page on the web (http or https) |
| `inbox.tell` | `parts.channel` | string | of `message`: the channel's id |
| `inbox.tell` | `parts.message` | string | of `message`: the message's id |
| `inbox.tell` | `parts.project` | string | of `cards`, `rows` and `runs`: whose board, table or runs; the project the text is applied for where it says none |
| `inbox.tell` | `parts.table` | string | of `rows`: the table, by its name |
| `inbox.tell` | `parts.ids` | list | of `rows`: the rows, each by its id, in the order listed; or say `saying` |
| `inbox.tell` | `parts.saying` | object | of `rows`: what a row is to say, by cell, to be listed, as `rows.read`'s `saying` is written; or say `ids` |
| `inbox.tell` | `parts.columns` | list | of `rows`: the columns shown, in order, each by its name (default: the table's own) |
| `inbox.tell` | `parts.instance` | string | of `runs`: the instance whose runs are listed, by its id; or say `key` or `environment` |
| `inbox.tell` | `parts.key` | string | of `runs`: the key whose instances' runs are listed, of the project: a pipeline's, applied again and again |
| `inbox.tell` | `parts.environment` | string | of `runs`: the environment of the project whose runs are listed |
| `inbox.tell` | `parts.most` | int | of `runs`: how many of the latest are listed (default 10, at most 50) |
| `inbox.tell` | `parts.measures` | list | of `measures`: the measurements, each by its name |
| `inbox.tell` | `parts.labels` | object | of `measures`: the labels each is to be measured by, each with its value as a word |
| `inbox.tell` | `parts.last` | int | of `measures`: how many of each's latest values its median is of (default 10, at most 100) |
| `inbox.tell` | `parts.column` | string | of `cards`: the column whose cards are listed; or say `label` |
| `inbox.tell` | `parts.label` | string | of `cards`: the label the cards listed hold; or say `column` |
| `inbox.tell` | `parts.fail_to` | string | of `cards`: the column a card that fails is moved to, with why (default `ready`) |
| `inbox.tell` | `parts.pass_to` | string | of `cards`: the column a card that passes is moved to, once it is approved; or say `pass_press`, or neither, and it stays |
| `inbox.tell` | `parts.pass_press` | string | of `cards`: what its column offers to be pressed that is pressed on a card that passes, once it is approved |
| `inbox.tell` | `parts.reason` | bool | of `cards`: a card is failed only with why (default true) |
| `service.start` | `health.tcp` | bool | `true`: it is healthy while its port takes a connection |
| `service.start` | `health.http` | string | a path, such as `/health`: it is healthy while asking for that on its port is answered well |
| `service.start` | `health.cmd` | string | a command your shell reads: it is healthy while the command ends well |
| `service.start` | `health.interval` | duration | how often it is asked (default 2s, and never more often than every 100ms) |
| `service.start` | `health.timeout` | duration | how long an answer is waited for before the asking counts as failed (default 2s) |
| `service.start` | `health.unhealthy_after` | int | how many askings in a row fail before a healthy service is said to be unhealthy (default 3) |
| `rows.read` | `fill.*` | each | a cell, with what it is given: a value, or a form that says how the value is come by |
| `rows.read` | `fill.*.like` | string | another cell of the row: what that one says |
| `rows.read` | `fill.*.else` | any | with `like`, what is given where that cell says nothing, or a number not above nothing: a value or a form |
| `rows.read` | `fill.*.slug_of` | string | another cell of the row: what that one says, as a name is written |
| `rows.read` | `fill.*.by` | string | another cell of the row: the form written beside what that one says is taken, as in `{ by: does, review: 3, otherwise: 1 }` |
| `rows.read` | `fill.*.otherwise` | any | with `by`, the form taken where nothing is written beside what the cell says |
| `rows.read` | `fill.*.share` | any | with `each`, how many times the one goes into the other, never fewer than one: what is shared out, a value or a form |
| `rows.read` | `fill.*.each` | any | with `share`, what each is given of it: a value or a form |
| `rows.read` | `fill.*.least` | list | forms, the least of whose numbers is given: one that says nothing, or a number not above nothing, is passed over |
| `rows.read` | `counting.*` | each | a figure over the rows of another list, by the name each row is given it under |
| `rows.read` | `counting.*.of` | list | the rows it is worked out over |
| `rows.read` | `counting.*.by` | string | the cell in which those rows say what the row says: it is worked out over those that say the same there |
| `rows.read` | `counting.*.sum` | string | the cell of those rows whose numbers are added up; how many of them there are, where it is not said |
| `rows.read` | `counting.*.unless` | string | a cell of those rows: one in which it is so is left out |
| `rows.read` | `giving.*` | each | a cell, with what it is given: a value, or a form that says how the value is come by |
| `rows.read` | `giving.*.like` | string | another cell of the row: what that one says |
| `rows.read` | `giving.*.else` | any | with `like`, what is given where that cell says nothing, or a number not above nothing: a value or a form |
| `rows.read` | `giving.*.slug_of` | string | another cell of the row: what that one says, as a name is written |
| `rows.read` | `giving.*.by` | string | another cell of the row: the form written beside what that one says is taken, as in `{ by: does, review: 3, otherwise: 1 }` |
| `rows.read` | `giving.*.otherwise` | any | with `by`, the form taken where nothing is written beside what the cell says |
| `rows.read` | `giving.*.share` | any | with `each`, how many times the one goes into the other, never fewer than one: what is shared out, a value or a form |
| `rows.read` | `giving.*.each` | any | with `share`, what each is given of it: a value or a form |
| `rows.read` | `giving.*.least` | list | forms, the least of whose numbers is given: one that says nothing, or a number not above nothing, is passed over |
| `rows.read` | `gathering.*` | each | a list of what one cell says, by the name it is given out under |
| `rows.read` | `gathering.*.of` | string | the cell |
| `rows.read` | `gathering.*.saying` | object | what a row is to say to be in it, by cell, as `saying` is written; every row, where it is not said |
| `rows.read` | `saying.*` | each | a cell, with what a row is to say in it: a value for exactly that, or a form |
| `rows.read` | `saying.*.not` | any | anything but this value |
| `rows.read` | `saying.*.in` | list | any one of these values |
| `rows.read` | `saying.*.said` | bool | `true` for something, `false` for nothing |
| `collection.ensure` | `fields.*` | each | a field of the collection, by its name |
| `collection.ensure` | `fields.*.type` | string | what it holds: `text` where it is not said |
| `collection.ensure` | `fields.*.type.text` | word | words |
| `collection.ensure` | `fields.*.type.choice` | word | one of the words `of` says can be chosen |
| `collection.ensure` | `fields.*.type.actor` | word | somebody: a person or an agent, as in `agent:builder` |
| `collection.ensure` | `fields.*.type.list` | word | several words |
| `collection.ensure` | `fields.*.type.number` | word | a number |
| `collection.ensure` | `fields.*.type.flag` | word | true or false |
| `collection.ensure` | `fields.*.type.date` | word | a day, written `2026-11-30` |
| `collection.ensure` | `fields.*.type.record` | word | a record of the collection `of` names, by its id |
| `collection.ensure` | `fields.*.type.records` | word | several records of the one collection `of` names, by their ids |
| `collection.ensure` | `fields.*.type.document` | word | a document: its Markdown, kept in the row, as a topic's body is |
| `collection.ensure` | `fields.*.required` | bool | a record cannot be without it |
| `collection.ensure` | `fields.*.of` | any | for a choice what can be chosen, in order; for a record or records the collection whose records it names |
| `collection.ensure` | `fields.*.quiet` | bool | its changes are not news: no line of a record's history is written for a change to it alone |
| `collection.ensure` | `fields.*.hidden` | bool | it is left out of a view of the collection until somebody shows it there |
| `collection.ensure` | `fields.*.said` | bool | every event about a record of the collection carries what it holds, under `fields` of the event's data, so that a handler is chosen by it with nothing run: a text longer than a kilobyte cut there, and anything else that long left out. Not of a field that is quiet or says what giving it a value takes |
| `collection.ensure` | `fields.*.indexed` | bool | its column is kept in order by an index from the first row, so that a table of many rows is sorted and held by it at once: any column of a large table is indexed when it is first sorted by, and this says it of one before anybody asks. Not of a field that holds several things |
| `collection.ensure` | `fields.*.suggests` | list | for a text that takes any word, the usual ones, which are offered to whoever fills it |
| `collection.ensure` | `fields.*.zoned` | bool | for words that are a time of day with the zone it is in after it (`02:00 Europe/Zagreb`): whoever writes one with no zone is given their own |
| `collection.ensure` | `fields.*.calls` | number | where it is among the fields a record is called by, from 1, wherever the record is named: an attempt called by its card and its contender, where its title is what something else knows it by |
| `collection.ensure` | `fields.*.writer_of` | string | another field of the table, whose writer it keeps: the factory writes into it who wrote that field each time it is written, and nobody writes it themselves |
| `collection.ensure` | `fields.*.icon` | string | the name of the icon its column is drawn with, until somebody who looks at the table chooses another |
| `collection.ensure` | `fields.*.takes` | any | what giving it a value takes beside what changing the record does: a scope, or a list of scopes every one of which is asked, or for a choice or a flag the scopes of each value that takes any, or `each` for a list of scopes that the record gives somebody, where adding the record or changing anything of it takes holding each scope it then says |
| `collection.ensure` | `fields.*.unsaid` | any | with `takes: each`, the scopes such a list stands for where a record leaves it empty; of a choice, the one word of `of` nothing chosen stands for, which is where nothing ranks and what a number written before the field was a choice is counted from |
| `collection.ensure` | `fields.*.takes.*` | each | a value of the field as it is written, with the scope, or the list of scopes, giving the field that value takes |
| `collection.ensure` | `views.*` | each | a way of looking at the collection, by its name |
| `collection.ensure` | `views.*.kind` | string | how its records are laid out: `table` where it is not said |
| `collection.ensure` | `views.*.kind.table` | word | rows and columns |
| `collection.ensure` | `views.*.kind.board` | word | a column for each thing the choice `by` names can be |
| `collection.ensure` | `views.*.kind.list` | word | a line for each record |
| `collection.ensure` | `views.*.kind.gallery` | word | a tile for each record |
| `collection.ensure` | `views.*.kind.calendar` | word | each record on its day: the date field `by` names, or `created` or `updated`, and when it was made where none is said |
| `collection.ensure` | `views.*.kind.order` | word | its records by what comes first: what can be begun now, what waits only on those, and so on |
| `collection.ensure` | `views.*.by` | string | what it goes by: for a board the choice field its columns come from, for a calendar the date its records are put on the days of, for an order the field in which a record says which others come before it |
| `collection.ensure` | `views.*.hides` | list | the fields it leaves out |
| `collection.ensure` | `views.*.done` | string | for an order, the choice whose last value says a record is done: the collection's first choice where none is said |
| `collection.ensure` | `views.*.group` | string | for a board, an order or a table, the field its records are gathered by |
| `collection.ensure` | `views.*.only` | object | what it is held to: a map from a field to the things a record has to hold one of there to be shown |
| `collection.ensure` | `views.*.columns` | list | its columns in the order they stand |
| `collection.ensure` | `views.*.freezes` | string | for a table, the column it is frozen up to |
| `collection.ensure` | `views.*.figures` | object | for a table, a map from a column to the figure at its foot |
| `collection.ensure` | `views.*.widths` | object | for a table, a map from a column to how wide it is |
| `collection.ensure` | `views.*.only.*` | each | a field, with the things a record has to hold one of there: a list, or one alone |
| `collection.ensure` | `views.*.figures.*` | each | a column, with the figure at its foot |
| `collection.ensure` | `views.*.figures.*.filled` | word | how many rows say something there |
| `collection.ensure` | `views.*.figures.*.empty` | word | how many rows say nothing there |
| `collection.ensure` | `views.*.figures.*.sum` | word | what a number's rows add up to |
| `collection.ensure` | `views.*.figures.*.mean` | word | what a number's rows come to on average |
| `collection.ensure` | `views.*.figures.*.least` | word | the least a number's rows say |
| `collection.ensure` | `views.*.figures.*.greatest` | word | the greatest a number's rows say |
| `collection.ensure` | `views.*.figures.*.ticked` | word | how many rows a flag is ticked in |
| `collection.ensure` | `views.*.figures.*.different` | word | how many different things are said, of what is neither a number nor a flag |
| `collection.ensure` | `views.*.widths.*` | each | a column, with how wide it is: whole pixels at the usual size of text, from 60 to 2000 |
| `record.add` | `declaring.*` | each | a field of the collection, by its name |
| `record.add` | `declaring.*.type` | string | what it holds: `text` where it is not said |
| `record.add` | `declaring.*.type.text` | word | words |
| `record.add` | `declaring.*.type.choice` | word | one of the words `of` says can be chosen |
| `record.add` | `declaring.*.type.actor` | word | somebody: a person or an agent, as in `agent:builder` |
| `record.add` | `declaring.*.type.list` | word | several words |
| `record.add` | `declaring.*.type.number` | word | a number |
| `record.add` | `declaring.*.type.flag` | word | true or false |
| `record.add` | `declaring.*.type.date` | word | a day, written `2026-11-30` |
| `record.add` | `declaring.*.type.record` | word | a record of the collection `of` names, by its id |
| `record.add` | `declaring.*.type.records` | word | several records of the one collection `of` names, by their ids |
| `record.add` | `declaring.*.type.document` | word | a document: its Markdown, kept in the row, as a topic's body is |
| `record.add` | `declaring.*.required` | bool | a record cannot be without it |
| `record.add` | `declaring.*.of` | any | for a choice what can be chosen, in order; for a record or records the collection whose records it names |
| `record.add` | `declaring.*.quiet` | bool | its changes are not news: no line of a record's history is written for a change to it alone |
| `record.add` | `declaring.*.hidden` | bool | it is left out of a view of the collection until somebody shows it there |
| `record.add` | `declaring.*.said` | bool | every event about a record of the collection carries what it holds, under `fields` of the event's data, so that a handler is chosen by it with nothing run: a text longer than a kilobyte cut there, and anything else that long left out. Not of a field that is quiet or says what giving it a value takes |
| `record.add` | `declaring.*.indexed` | bool | its column is kept in order by an index from the first row, so that a table of many rows is sorted and held by it at once: any column of a large table is indexed when it is first sorted by, and this says it of one before anybody asks. Not of a field that holds several things |
| `record.add` | `declaring.*.suggests` | list | for a text that takes any word, the usual ones, which are offered to whoever fills it |
| `record.add` | `declaring.*.zoned` | bool | for words that are a time of day with the zone it is in after it (`02:00 Europe/Zagreb`): whoever writes one with no zone is given their own |
| `record.add` | `declaring.*.calls` | number | where it is among the fields a record is called by, from 1, wherever the record is named: an attempt called by its card and its contender, where its title is what something else knows it by |
| `record.add` | `declaring.*.writer_of` | string | another field of the table, whose writer it keeps: the factory writes into it who wrote that field each time it is written, and nobody writes it themselves |
| `record.add` | `declaring.*.icon` | string | the name of the icon its column is drawn with, until somebody who looks at the table chooses another |
| `record.add` | `declaring.*.takes` | any | what giving it a value takes beside what changing the record does: a scope, or a list of scopes every one of which is asked, or for a choice or a flag the scopes of each value that takes any, or `each` for a list of scopes that the record gives somebody, where adding the record or changing anything of it takes holding each scope it then says |
| `record.add` | `declaring.*.unsaid` | any | with `takes: each`, the scopes such a list stands for where a record leaves it empty; of a choice, the one word of `of` nothing chosen stands for, which is where nothing ranks and what a number written before the field was a choice is counted from |
| `record.add` | `declaring.*.takes.*` | each | a value of the field as it is written, with the scope, or the list of scopes, giving the field that value takes |
| `compare.start` | `ways.name` | string | what the way is called: a word, such as `otter`, which a start names as its `way` |
| `compare.start` | `ways.share` | int | its part of the things where they are shared out; a start that names its way is given it whatever the shares (default 1) |
| `compare.start` | `ways.blueprint` | string | a blueprint started in place of the one compared |
| `compare.start` | `ways.version` | string | the version of what it starts that it is held to |
| `compare.start` | `ways.with` | object | what it hands, over what the start hands |
| `compare.start` | `ways.prompts` | object | for a file a step links, by its name in the library, the file it reads instead |
| `compare.start` | `counts.name` | string | what the count is called: a lowercase word, such as `judged` |
| `compare.start` | `counts.topic` | string | the event's topic, whole |
| `compare.start` | `counts.thing` | string | the field of the event that says which thing it is about; a dotted path reads within it |
| `compare.start` | `counts.by` | list | the fields whose values say what it came to, each read as `thing` is; the last word of the topic where none |
| `compare.start` | `counts.accepts` | list | what it came to that says the thing was accepted (the last of several fields where several say); a thing is counted as accepted once |
| `record.comment` | `anchor.quote` | string | the words commented on, as they read in the document, on one line |
| `record.comment` | `anchor.before` | string | a few words just before them in the same block, so that the same words said twice are told apart |
| `record.comment` | `anchor.after` | string | a few words just after them in the same block |
| `record.comment` | `anchor.under` | list | the headings the block stands under, outermost first, so that words that moved are looked for there first |

## More than one device

A fleet can span devices. One device is the hub: blueprints run there. The
others are members that have joined it (`mw help device`). Put
`device: <name>` on a `do:` step to send that one action to a member:

```yaml
steps:
  build:
    do: shell
    device: builder-1
    timeout: 30m
    with: { run: cargo build --release }
```

The step still belongs to the hub's run, so:

- what the action prints comes back into the step's log, and `watch:` rules
  see it;
- the step's `timeout`, `retry` and hooks work as they do anywhere.

The device's own name, or `local`, means this device. `device:` may be an
expression (`device: "${item}"` in a `for_each`).

#### When the link drops

A dropped link costs time, not the step. The member carries on, and the hub
gets the result when the link is back. If a member is not linked, the step
waits for it for a minute and then fails with `unreachable` in its error
data. `retry`, `allow_failure` or a hook can then decide what happens next.

A command reaches a member once, however often it is delivered. The one
exception: when a member's daemon restarts in the middle of the work, the
member is asked again to do what it forgot. So a step sent to another
device should be safe to repeat.

#### Hosting something on another device

A step leaves nothing running after it. To *host* something on another
device (a service, a router, anything that must be owned), start a
blueprint there. That gives it an instance on that device:

```yaml
steps:
  text: { do: fs.read, with: { path: blueprints/web.bp } }
  host:
    do: blueprint.apply
    device: builder-1
    with: { source: "${steps.text.out.content}", key: web, params: { version: v2 } }
```

`blueprint.apply` takes `blueprint` (a name known on that device, such as
`std/slot`) or `source` (its text), and `params`, `key` and `wait`.
Applying again with the same `key` updates what is there, exactly as
`mw apply` does. `blueprint.stop` takes the `key`.

To write an expression that the inner blueprint evaluates, rather than this
one, double the dollar: `$${ ... }`.

`std/rolling` runs the same command on each device of a list, one at a
time. It checks each device before going on to the next, and stops at the
first that fails.

#### Wherever there is room

`device:` takes an expression, and `device.pick` gives it one. Among this
device and the devices of its fleet that have what is asked, it picks the
least busy for its size (its load per processor), then the one with the
most free memory.

```yaml
steps:
  where: { do: device.pick, with: { mem_mb: 8000, here: false } }
  job:
    needs: [where]
    do: blueprint.apply
    device: ${ steps.where.out.device }
    with: { blueprint: render, key: "render-${params.id}", params: { id: "${params.id}" } }
```

`device.pick` takes:

- `mem_mb` and `disk_mb`: how much must be free;
- `cpus`, `arch` (`x86_64`, `aarch64`) and `os`: what the device must have;
- `among`: only these devices;
- `here`: whether this device itself may be picked (true unless said).

When no device fits, the step fails and says, for each device, what it
lacks. `device.list` gives everything known of every device, for a
blueprint that chooses by a rule of its own.

What is known of another device is what it last reported about itself, a
few times a minute. Until its next report, a pick is counted against the
device it chose, as one more thing running there and as the memory it
asked for. So ten picks in the same moment are spread out, instead of all
going to the device that looked emptiest. This is not a reservation.
Nothing limits how much work a device takes, except a blueprint that counts
what it placed: one instance that everything asks (`one_at_a_time`), with
the count in its `state`.

A member decides what its hub may do on it (`mw device join ... --allow`).
That is checked on the member.

## Secrets

A program that takes payments needs the payment service's key; one that
sends mail needs a password. A blueprint never holds either. The factory
keeps each secret under a name, and the blueprint names it in `env` (of the
blueprint or of a step) in place of a value, where the program reads it.

```yaml
blueprint: checkout
permissions: [fleet, shell, secrets]

steps:
  serve:
    do: service.start
    env:
      STRIPE_KEY: { secret: stripe }
      MAIL_PASSWORD: { secret: "mail-${ profile.name }" }
    with: { name: checkout, cmd: [./checkout] }
```

```sh
mw secret set stripe < key.txt      # read from standard input or --file, never an argument
mw secret list                      # the names, and when each was set
mw secret list --where              # for whoever manages the factory: who set each, and what names it
mw secret remove stripe             # says what still names it
```

#### Secrets in the client

In the client this is the *Secrets* page, reached from Settings. It is
offered only to whoever may manage secrets. For each secret it shows the
name, who set it and when, and what names it:

- the agents whose rows hand it over;
- the services that were started with it;
- the running instances whose texts name it in an `env`.

So you see what would fail before you remove one. You add or replace a
secret by typing it once into a field that hides it. Nothing reads a secret
back: not that page, no route, no command. If a value is lost, set it
again. A name that something uses but nobody has set is listed too, as not
set.

What names a secret is read from the texts as they stand. A name that a
blueprint works out any other way than from a param
(`"mail-${ profile.name }"`) is not known until the step runs.

#### How a secret is kept

The value is sealed with a key made from the installation's own secret. It
is kept in a file only the installation's owner can read, in a folder of
the data folder that no copy of the factory carries. A copy of the file is
not the value: it opens on no other installation and under no other name.

The value goes from that file into the environment of the program that a
`shell` step or a service runs, as the program starts. It is in no event,
log, database, argument or answer of the daemon. For a service, what is
kept so that it can be started again is the name. The value is read each
time, so a restarted service gets the secret as it was last set.

#### The rules

- **Setting and removing a secret needs `admin`**. Seeing which
  secrets there are, and using one, takes `secrets:use`. A blueprint lists
  that as `secrets` under `permissions`.
- **`env` is the only place a secret can be named.** An expression cannot
  read a secret, so nothing written in a blueprint can put one in an
  argument, in `state`, in an event or in a line of a log.
- **The name is text, and may be worked out**: the one for this profile, as
  above, or the one a param says (`{ secret: "${params.key}" }`). A name
  that comes to the empty text names no secret, and the variable is not
  set: that is how a blueprint that can be told which key to use is told
  none. What cannot be worked out is *whether* a variable is a secret: an
  expression that comes to `{ secret: ... }` is refused.
- **A package's program is handed one too**, where the step that runs it
  names one in `env`. Whoever applied then also needs `shell`, as for any
  environment given to such a program.
- **A missing secret** fails the step that wants it, saying which one and
  how to set it. A new factory started from a snapshot has no secrets until
  it is given its own.
- **A step for another device** (`device:`) is handed the secret that
  device keeps. Only the name is sent with the step. The value never leaves
  the device it was set on (`mw secret set` there), and a hub that keeps a
  secret of the same name sends nothing of it. A device that does not let
  its hub use its secrets, or run commands on it (`--allow`), refuses the
  step and says which.
- **What the program does with a secret is up to the program.** It can send
  it anywhere it can reach, and nothing here stops that.
- **What the program prints of a secret is written down by name.** See
  below.

#### What a program prints of a secret

A program that prints its environment, or fails with the refused key in its
last line, has said its secret. What a program says is kept: a step's log
and its outputs (`stdout`, `stderr`), a service's log, an error's sentence,
and from those every event, every message a blueprint posts of them, and
every copy of the factory.

So wherever a program that was handed a secret prints it again, it is
written as `` [the secret `stripe`] ``, never as the value. It is taken out
before anything reaches the disk: the program is started with this program
standing between it and the files it writes to, so a service that keeps
running without the daemon has a clean log too. An agent's program, a
package's program and git asked as a key are held to the same rule.

It does not catch everything:

- A value of fewer than six letters is left as it is: it is too short to
  tell from an ordinary word.
- A value the program prints changed (in base64, in halves on two lines, a
  letter at a time) is not recognised.
- What a program writes anywhere but its output (a file of its own, another
  machine) is not read at all.

One thing behaves differently for a step that was handed a secret: it is
over only when everything it started has let go of its output. A step that
leaves a program running behind it (`server &`) goes on until that program
ends. Something that is meant to keep running is a service
(`service.start`).

## Permissions

An action may need a capability (`shell`, `fleet:write`, `agents:write`,
...).

- A blueprint that lists `permissions` may use only those. That is checked
  before anything runs.
- A blueprint that lists none runs with everything its caller has, and
  gets a warning saying so.
- Either way, an instance never has more authority than whoever applied
  it. A blueprint applied by an agent cannot run a shell unless that agent
  can.
- Listing a permission does not grant it. Where a blueprint lists one that
  whoever checks or applies it does not hold, a warning is shown on the
  list (`unheld-permission`): the blueprint will run without it, and a step
  that needs it will fail.

```yaml
blueprint: nightly-build
permissions: [shell, fleet:read]
steps:
  build: { do: shell, with: { cmd: [cargo, build] } }
```

#### Applying again

The same holds every time a blueprint is applied. Applying over a running
instance is an apply like the first:

- From the moment it takes, the instance acts as whoever applied it this
  time, with what they hold and no more. That covers its steps, its
  handlers, what it does as it stops, and the children it starts from then
  on, whoever started it originally.
- A blueprint that lists something the asker does not hold is not applied
  over somebody else's instance at all: it is refused, and nothing changes.
- An update that fails puts the previous version back, with the authority
  that version had.
- Applying what is already running changes nothing, whoever asks, and it
  goes on as whoever applied it. `--force` applies it anyway, as the asker.

#### Areas and scopes

A capability is an area (`fleet`) or one thing within it (`fleet:write`).
Listing or holding the area is listing or holding everything in it. The
areas:

- `projects`, `channels`, `board`, `records`, `blueprints`, `agents`,
  `fleet`: each with `:read` and `:write` (`:run` for blueprints);
- `search`, `columns`, `secrets:use`, `shell`, `extensions`, `private` and
  `admin`.

Some of these need a word more:

| Scope | Leave to | Who holds it |
|---|---|---|
| `columns` | Change what columns a table has, from the client: add one, change what kind of thing one somebody added holds and what it offers, and remove it. This is beside what changing the table's records takes. It changes nothing a blueprint declared. | A person, unless it was taken away. An agent only if its row gives it. |
| `private` | Read a table that is kept private (`reads` of `collection.ensure`). `private:<word>` is what is kept under one word (`private:journal`). | The factory's owner. Nobody else until given it: a person's key with `--scope private:journal`, an agent in its row's `holds`. |
| `shell` | Run any command on the device. | A person, unless it was taken away. An agent only if its slot was given it. |
| `extensions` | Run the programs of the installed extension packages, each only in the ways its package wrote down. An action a package brings needs it. `extensions:<package>` is one package's (`permissions: [extensions:github]`). | A person, unless it was taken away. An agent only if its slot was given it. |

An agent's slot is given `shell` or `extensions` with `scopes` on
`agent.ensure` (or in the body of `POST /api/agents`), by someone who holds
it themselves.

Unless its slot says otherwise, an agent holds what its work needs:
`projects`, `channels`, `board`, `records`, `blueprints:read`,
`agents:read`, `fleet:read` and `search`. It can read what is running, but
start and stop none of it. A fleet's row says otherwise with `holds`.

#### Scopes that no action names

Some things take a scope even where no action's capability says so:

- **Saying an event** (`emit`, and `wait ... say:`) takes `emit:<topic>`,
  or `emit`. Whatever hears an event acts on it with the authority of
  whoever applied that, and trusts that the sender was allowed to say it.
- **A handler and a `wait` hear what whoever applied them may read** (the
  table under *Who is told of an event*). What the instance itself, or
  anything it started, says is its own to hear, whatever it holds.
- **Sending a step to another device** (`device:`) takes `fleet:write`.
- **Giving work to an agent** (`agent.task`, `agent.consult`) takes holding
  everything that agent's credential holds, plus `shell` if the agent is
  held to nothing (`full`). An agent is not a way round what the asker
  holds.
- **Making an agent that is held to nothing** takes `shell`. Making one
  that draws a sign-in or is handed a key (`secrets`), or changing anything
  of one that does, takes `secrets:use`.
- **Making an agent with no wall, or showing it more of the device**
  (`wall: off`, `reads`, `changes` of `agent.ensure`), or changing anything
  of one that is so, takes `shell`.
- **Letting an agent's program use the web unasked** (`web: on` of
  `agent.ensure`, `--web`) takes `shell`. A wall does not hold the network.
  A program that need not ask before it looks something up or fetches a
  page brings in whatever a page says, and sends out whatever it puts in a
  search or an address. Turning it off again takes no more than changing
  the agent.
- **Letting an agent's program keep running without the daemon**
  (`outlives: on` of `agent.ensure`, `--outlives`) takes `shell`. Turning
  it off takes no more than changing the agent, and neither does changing
  anything else of such an agent.
- **Reading a record of a table that says `reads`** takes that scope. So
  does everything else that would show it: changing it or commenting on
  it, reading the files it is kept in, finding it by search, hearing an
  event about it, and reading its folder from behind an agent's wall.
  Declaring such a table, or declaring it again, takes holding the scope.
- **Changing a record of a table that says `takes`** takes that scope too
  (`collection.ensure`), and so does declaring such a table again. Giving
  a value to a field that says `takes` needs each scope the field names,
  or each that the field says that one value takes. Writing a scope into a
  list that says `takes: each` needs holding that scope. Declaring a table
  again so that a field asks less than it did takes holding what it asked,
  or `admin` where it asked for each scope written.
- **Adding or changing a record of a table that says `steers`** takes what
  writing each record of the steered table would take, as each stands. For
  example, a stage of a fleet is arranged by whoever holds what the fleet's
  agents hold. Declaring the table again so that it does not steer takes
  `admin`.
- **Keeping a table in a store service** (`store: service`) takes
  `fleet:write`: it is an address the daemon then sends requests to.
- **Giving a package's program an environment of the step's own** (`env`)
  takes `shell`.

`secrets:use` is leave to have a secret the factory keeps be used. It is
what `signins:` on `agent.ensure` takes: that tells an agent to draw what
its program signs in with from the sign-ins the factory holds
(`mw help signin`). A person has `secrets:use`; an agent does not. Adding,
listing and removing those sign-ins takes `admin`.

## Events

Everything that happens is an event with a topic: `run.started`,
`step.finished`, `step.failed`, `instance.active`, `service.unhealthy`,
`board.item.moved`, `channel.message`, `agent.turn.finished`. Handlers
react to events, the client is drawn from them, and every measurement is
derived from them. `mw events --follow` shows them as they happen.

A **topic** is what an event is published to. An event reaches the
handlers waiting on its topic that hear its project's events (*Whose an
event is*): all of them, or, with `for:`, the ones it is addressed to
(*Handlers*, above).

#### An event's number

An event's number (`seq`) is unique: no other event in the factory has it.
Of two events of one project, the later has the greater number. That is
all a number says:

- Numbers are not consecutive. There are gaps, and more of them as a
  factory grows.
- The numbers of two events of different projects say nothing about which
  came first. Each project's events are kept in their own order; two
  projects' events are ordered by when each was said (`ts`), as closely as
  a clock can.

So a number names an event (`review-${event.data.card}-${event.seq}`) and
orders one project's events. Never work anything out by adding to a
number, and never wait for "the next number".

#### Where you have read to

Where you have read to is not a number. Anything that follows events from
outside is handed a *place* to go on from, and hands it back unchanged:

- `cursor` in what `/api/events/recent` and `/api/fleet` answer;
- the `id` of each event in the stream (`/api/events`), which a browser
  sends back by itself as `Last-Event-ID`.

`since=<place>` opens the stream just after that place. `after=<place>`
reads what is kept from there.

A place is text written by the daemon for the daemon. It is not an event's
number, nothing should be read out of it, and its form may change (it is a
place in each journal the factory keeps: its own and each project's). The
`id` of a stream event is the place just after that event, not a name for
it: two readers handed the same event may get different places with it.

A stream opened from now first says where now is, in a frame with an `id`
and nothing else. A browser that reconnects goes on from there.

What the runner says of its runs and steps is passed on and not always
kept (*What is said of steps and runs is passed on*). So a stream that
goes on from a place cannot hand over what was passed on since then. It
says so first, in a frame of its own kind, `event: behind`, whose data
names the topics that may be missing (`{"topics": ["run.started", …]}`).
It says so again whenever it falls too far behind the live events.
Anything that shows something it follows on one of those topics reads it
again on that frame.

An event's number is still accepted where a place is. It means every event
numbered above it: for that event's own project, what was said after it;
for another project, what was numbered later, which is not the same thing.
`last_seq` is still answered beside `cursor`, as the highest number any
event has. It is not a place. `mw events --follow` and the client keep the
place, never the number.

#### Ids are names

A run's id, an instance's, a channel's, a message's, a turn's, a key's:
each is text that names one thing. Read nothing out of how it is written,
which may change. The one id that is read is a record's, which says where
the record is: `shop-12` is the twelfth record of the shop's board, and a
document's or a mockup's id is where its file is
(`shop-docs:guides:install`). Where a thing's project is wanted, it is in
what is said of the thing (`project`), never part of its id.

#### How things say they ended

`finished` is said after any ending of something that began, with how it
went (`status`), and it is said last. It is not the opposite of `failed`:

| What ended | When it goes wrong | Then, always |
|---|---|---|
| A step | `step.failed` | `step.finished` |
| A run | `run.failed` | `run.finished` |
| An instance | `instance.failed` or `instance.stopped` | `instance.finished` |

What went well says only that it finished. So:

- to act on something going wrong, listen for `failed`;
- to act whenever something is over, listen for `finished`;
- to act only on success, listen for `finished` and read
  `event.data.status`.

A step that never began (its `when` was false, or the run ended before its
turn) says `step.skipped` or `step.cancelled`, and nothing more.

A run whose instance was stopped or applied over while one of its steps
was under way, or still to come, finishes as `cancelled` and never says it
failed. A run whose steps had all ended by then went well: a handler whose
last step stops its own instance (`do: stop`) is not cancelled by that.
Where runs are measured (`mw metrics run`), a cancelled run is counted, and
not among those that failed, which are the runs that said `run.failed`.

A step that is passed over costs nothing by itself: it is among its run's
steps, written with the run's next write.

#### What is said of steps and runs is passed on

Everything the runner says of a step and a run (`step.started`,
`step.finished`, `step.failed`, `step.retrying`, `step.waiting`,
`step.skipped`, `step.cancelled`, `run.started`, `run.resumed`,
`run.failed`, `run.finished`) is sent as it happens, in order and with its
number, to whoever is watching. It is kept in the journal only while the
text of some instance there hears it or has a step that waits for it:

- A handler that hears one, or a `wait` for one, gets every one from its
  place, as with any event.
- A text that names one through a hole (`event: ${ params.topic }`) is
  taken to hear them all.

Where nothing could hear them, what is read afterwards (`mw events`, what
happened lately) does not have them. What a step and a run came to is in
their rows, which a run's page, Activity, `mw runs` and `mw logs` read.

#### A blueprint's own events

A blueprint's own events (`emit`) may use any topic except the families
the system publishes under (`run`, `step`, `instance`, `service`, `board`,
`channel`, `collection`, ...).

#### From a service

A service the factory hosts says an event of its own by printing a line
that begins `::event`, with the topic and what it carries:
`::event {"topic": "shop.paid", "order": "o-17"}`. The event is said for
the instance that owns the service (*Labels*, *Measurements of your own*).

#### From outside

Something that is not a blueprint and is not hosted here can also say an
event of a blueprint's own topic: a script, another program, or a service
elsewhere that calls an address when something has happened (a payment
taken, a form sent). A handler hears it like any other event.

```sh
mw emit shop.paid order=o-17 total=40          # as you
mw key payments --scope emit:shop.paid         # a key that can say that topic and do nothing else
curl -X POST -H "Authorization: Bearer <key>" -d '{"order": "o-17"}' http://127.0.0.1:7700/api/emit/shop.paid
```

The sender holds a key for the topic: `emit:<topic>` is one topic; `emit`
is any topic a blueprint may say, and a person signed in holds it. What the
system says of itself cannot be said this way by anyone. What is sent is a
mapping, written as JSON, and is what the event carries (`event.data`).

A handler that waits for such an event says so with `outside: true`. The
check looks for a blueprint that says each topic a handler hears, to catch
a misspelt name, and warns when it finds none. For an event said from
outside there is none to find, and what it carries is the sender's choice:

```yaml
on:
  paid:
    event: shop.paid
    outside: true
    do: set
    with: { last: "${ event.data.order }" }
```

The event says who said it, in `event.source.actor`:

- `user:<id>` for a person signed in;
- `key:<id>` for a key;
- `caller:<name>` for a service that signed (below).

A key's id is given by the factory when the key is made, and is in the
answer that makes it. Nobody chooses it (unlike a key's name). Nothing a
sender writes sets the actor. So a handler that must know whose word it is
acting on reads the actor, never a field of what was sent:

```yaml
on:
  submitted:
    event: job.submitted
    when: ${ event.source.actor == "key:cred_01k2…" }   # that customer's key, and no other
```

A service that can only be given an address to call, with no way to send a
header, is given `/in/<key>/<topic>`. A key in an address is written down
wherever the address passes, so a key is accepted there only if saying
events is all it can do: make it with `--scope emit:<topic>` and nothing
else. There, the address is all that says who is calling.

#### A service that signs what it sends

A service that signs what it sends proves more, and is let in by that
instead. It shares a secret with you and sends, in a header, a signature of
each call made with that secret. Only a holder of the secret could have
made it, and only over those bytes. A *caller* is such a service, described
once by whoever manages the factory. What it sends to `/in/<name>` is said as the one topic
named, when the signature is one its secret makes; otherwise it is refused.
No key is in the address, so reading the address lets nobody in.

```sh
mw secret set pay-signing < signing-secret.txt
mw caller set payments --topic shop.paid --secret pay-signing --header X-Signature --prefix sha256=
mw caller list
```

Services differ only in where they put things, and their own pages say
where. A description says only that:

| Field | What it says |
|---|---|
| `topic` | the one topic its calls are said as |
| `secret` | which of the factory's secrets it signs with, by name (*Secrets*) |
| `header` | the header its signature is sent in |
| `prefix` | what comes before the signature there: `sha256=`, `v1=`, `v1,`. A header that holds several signatures (as one does while a service changes its secret) is accepted when any of them is right |
| `encoding` | how the signature is written: `hex` (unless said) or `base64` |
| `signed` | what was signed: `{body}` (unless said) is what was sent, `{time}` when the service says it signed, `{header:<name>}` a header. `{time}.{body}` and `{header:msg-id}.{time}.{body}` are two that services use |
| `time_header`, `time_prefix` | where the time is: a header of its own, or after a prefix (`t=`) in the header the signature is in |
| `within` | how old a call may be: `5m` unless said |
| `key` | how the secret is written: `text` (unless said), or `base64` or `hex` for a service whose secret is a key written that way |

The signature is HMAC-SHA256. A description that would prove nothing is
refused when it is given: one whose `signed` leaves out the body, or that
reads a time nobody signed.

A call that says when it was signed is refused once it is older than
`within`. A call that was accepted is not said a second time while it
could still be sent again: it is answered as the first time, with
`again: true`. That is remembered while the daemon runs; a daemon that
restarts could accept such a call once more within what is left of
`within`. A service that signs no time can have its calls replayed by
anyone who saw them pass, for as long as its secret stands, and nothing
here can make up for that. So a handler of such calls should be safe to run
twice, as it should anyway: a service that did not hear the answer sends
again.

What a caller sends is a mapping written as JSON, as from a key. Each
caller's calls are measured, accepted and refused (`mw metrics caller.call
--by caller`), so you can see when a service's calls start being refused.
A copy of the factory has no callers: it must not answer for the factory it
was copied from, and it has none of its secrets to check with.

### What a handler hears is checked

An action is described, so a step is checked before it runs. An event is
described too, so a handler is checked as well: what it listens to, the
fields it names under `for:`, and what it reads as `event.data.<field>` are
held to what such an event carries, when the blueprint is checked and when
it is applied. Otherwise a misspelt field would be a handler that never
fires, with no error anywhere.

- **An event of the system's own** is held to the table below.
  `event: board.item.move` is refused, with the one that was meant;
  `for: { card: … }` on `board.item.moved` is refused, since a move carries
  `id`; so is `event.data.too`.
- **An event blueprints say to each other** is held to what says it. An
  `emit` step with its topic written out says what the event carries: the
  names it writes beside `topic`. The blueprints that can be seen (the
  standard ones, the packages', the project's own, and the one being
  checked) are read for them. A handler on a topic that none of them emits
  gets a warning, since something outside may say it.
- **A group of topics** (`board.item.*`) may read what any event it takes
  carries. Such a handler usually tells its events apart by `event.topic`
  and reads from each what that one has. A field that none of them carries
  is wrong, whichever event comes. A group that takes every family (`**`)
  is held to nothing.
- **An event that is passed on and not kept** (a line of a log, a program's
  output as it works) reaches whoever is watching, and no handler.
  Listening for one is refused.
- A topic or a field that is filled in when the blueprint runs (`${ … }`)
  cannot be read off its text, and is held to nothing.

`mw events --kinds` lists every kind of event, and `mw events --kinds
<topic>` what one carries. The editor offers them: topics after `event:`,
and an event's fields under `for:` and after `event.data.`.

The system is held to the same descriptions. Wherever it runs under test,
every event it emits is checked against its description, and a test fails
on one that is not described, carries something not listed, or lacks
something said to be always there.

A kept text (an instance's own, taken up again when the daemon starts) is
not held to this. It was checked when it was applied. If an event has
changed under it since, the result is a handler that does not fire, not an
instance that cannot run.

### The events there are

What each event carries is read as `event.data.<name>`. A name followed by
`?` is there only some of the time. (A test holds this table to the
descriptions.)

| Topic | What happened | Carries |
|---|---|---|
| `agent.output` | Something an agent's program printed as it worked. Passed on, not kept. | `slot`, `turn`, `project`?, `text` |
| `agent.signin.added` | A sign-in was given to the factory to hold. | `signin`, `kind`, `for`, `at_once` |
| `agent.signin.held` | A sign-in was handed to an agent for a turn. | `signin`, `kind`, `slot`, `turn` |
| `agent.signin.released` | A sign-in was handed back at the end of a turn. | `signin`, `slot`, `turn` |
| `agent.signin.removed` | A sign-in was taken away. | `signin` |
| `agent.signin.restored` | A sign-in that was set aside can be handed out again. | `signin` |
| `agent.signin.set_aside` | A sign-in that was refused is not handed out for a while. | `signin`, `slot`, `turn`, `until`, `why` |
| `agent.signin.waiting` | A turn is waiting for a sign-in to be free. | `slot`, `turn`, `until` |
| `agent.slot.created` | An agent was brought into the fleet. | `slot`, `adapter`, `project`?, `role`?, `wall`? |
| `agent.slot.invited` | An invitation was made for an agent this factory does not start to join by. The invitation is in no event. | `slot`, `expires`, `project`? |
| `agent.slot.joined` | An agent this factory does not start handed its invitation over, and has its credential. | `slot`, `project`? |
| `agent.slot.unjoined` | An agent this factory does not start gave its credential up: it is let in again by an invitation. | `slot`, `project`? |
| `agent.slot.stopped` | An agent was stopped taking work. | `slot`, `project`? |
| `agent.slot.leaving` | An agent is given no more work, and is stopped when it has finished what it has. | `slot`, `in_hand`, `project`? |
| `agent.slot.removed` | An agent was taken away for good, by a person: its slot, its credential and its place in every channel. What it said and did stays. | `slot`, `turns`, `channels`, `project`? |
| `agent.slot.updated` | An agent's slot was changed, or its program said which model it is. | `slot`, `adapter`?, `project`?, `role`?, `wall`?, `model`? |
| `agent.turn.finished` | An agent's turn ended. | `slot`, `turn`, `project`?, `kind`, `channel`?, `ok`, `error`?, `called_off`?, `duration_ms`, `cost_usd`?, `input_tokens`?, `output_tokens`?, `decision`?, `refused`? |
| `agent.turn.queued` | A turn was put in an agent's queue. | `slot`, `turn`, `project`? |
| `agent.turn.started` | An agent began a turn. | `slot`, `turn`, `project`?, `kind`, `channel`?, `thread`?, `again`?, `taken_up`? |
| `board.columns` | A board's columns were set. | `project`, `columns` |
| `board.item.created` | A card was added to a board. | `id`, `project`, `fields`?, `title`, `status`, `by`?, `late`?, `restored`?, `through`?, `wrote`?, `doing`? |
| `board.item.moved` | A card was moved to another column. | `id`, `project`, `fields`?, `title`, `from`, `to`, `by`, `note`?, `decided`?, `late`?, `through`? |
| `board.item.removed` | A card was taken off its board. | `id`, `project`, `fields`? |
| `board.item.pressed` | Something a card's column offers was pressed on it, or a press was taken back. | `id`, `project`, `fields`?, `status`, `action`, `title`, `by`, `name`, `coming`, `back`, `did`, `released`, `waits`, `through`? |
| `board.item.held` | A card waits where it is for something its column offers to be pressed: said once each time it comes there. | `id`, `project`, `fields`?, `card`, `status`, `coming`, `waits`, `who`, `says`, `through`? |
| `board.item.updated` | Something on a card was changed, or its place in its column. | `id`, `project`, `fields`?, `title`?, `status`, `assignee`?, `changed`, `reworded`, `by`, `through`?, `decided`?, `approved_by`?, `asked`? |
| `caller.removed` | A caller was taken away, and its address answers nobody. | `name`, `topic`, `by_name` |
| `caller.set` | A caller was described, or described again: a service outside that signs what it sends. | `name`, `topic`, `by_name` |
| `card.approved` | Somebody approved a card: a review that passed it, or a person who passed it where a told line lists it. | `card`, `project`, `by`, `words` |
| `channel.created` | A channel was made, or given back from the Trash. | `channel`, `name`, `project`?, `restored`? |
| `channel.speaker_held` | What the factory's bot may ask was narrowed, or given back to everything its commands reach. | `name`, `holds`? |
| `channel.removed` | A channel was taken away, with what was said in it. | `channel`, `name`, `project`?, `messages`, `attachments`, `trash`? |
| `channel.archived` | A channel was archived: read by whoever read it, and written in by nobody until it is given back. | `channel`, `name`, `by`, `by_name`, `at`, `title`?, `project`? |
| `channel.attachment.placed` | Something attached to a message as a copy was written among a project's files, and the message names that file from then on. | `channel`, `name`, `message`, `attachment`, `project`, `path`, `file_name`, `by`, `by_name`, `title`? |
| `channel.unarchived` | An archived channel was given back, as it was. | `channel`, `name`, `by`, `title`?, `project`? |
| `channel.renamed` | A channel was called something else: an open one by its name, the one it had free from then, and one kept to its members by its title. Its id is as it was. | `channel`, `name`, `was`, `kept`, `by`, `by_name`, `title`?, `project`? |
| `channel.described` | What a channel says it is for, shown under its head, was changed by one of its managers. | `channel`, `name`, `topic`, `was`, `by`, `by_name`, `title`?, `project`? |
| `channel.managers` | Somebody in a channel was made one of its managers, who change what it is called and says it is for, or was made no longer one. | `channel`, `name`, `member`, `manages`, `managers`, `by`, `by_name`, `title`?, `project`? |
| `channel.docs` | A document was added to what a channel keeps to hand, or taken from it: a library's record or a project's file, the same for everybody in the channel. | `channel`, `name`, `doc`, `did`, `by`, `by_name`, `title`?, `project`? |
| `channel.placed` | What a channel belongs to was said: a project or a workspace, and never both. Said as one that is no project's is made, as it is said to be another's, and as what it was of is taken away. | `channel`, `name`, `project`?, `workspace`? |
| `channel.joined` | Someone or something joined a channel. | `channel`, `name`, `member`, `member_name`, `project`? |
| `channel.kept` | A channel was kept to its members or opened again, says another thing of whether its agents are asked nowhere else, or was a conversation and is opened as a channel. | `channel`, `name`, `kept`, `alone`, `title`?, `was`?, `conversation`?, `project`? |
| `channel.left` | A member left a channel. | `channel`, `member`, `project`? |
| `channel.message` | Something was said in a channel. | `id`, `channel`, `channel_name`, `project`?, `author`, `author_kind`, `author_name`, `body`, `reply_to`?, `thread`?, `model`?, `attachments`?, `mentions`, `mark`?, `said_by`? |
| `channel.discussion.offered` | A channel came to offer its agents discussions, or said them otherwise. | `channel`, `settings`, `instance` |
| `channel.discussion.withdrawn` | A channel offers its agents discussions no longer. | `channel` |
| `channel.discussion.begun` | Agents of a channel began talking something over in a thread. | `channel`, `channel_name`, `project`?, `root`, `message`, `why`, `agents`, `spoken`, `asker`?, `stretch`, `first` |
| `channel.discussion.resumed` | A person answered a discussion that waited for them, which goes on. | `channel`, `channel_name`, `project`?, `root`, `message`, `why`, `agents`, `spoken`, `asker`?, `stretch`, `first` |
| `channel.discussion.turned` | An agent took a turn in a discussion. | `channel`, `root`, `stretch`, `nth`, `slot`, `word`, `turns` |
| `channel.discussion.ended` | A stretch of a discussion ended: agreed, needing a person, at its cap, or ended by a person. | `channel`, `root`, `stretch`, `came`, `status`, `turns`, `agents`, `by`?, `by_name`? |
| `client.updated` | The client's files were replaced by another build. | `build`, `was` |
| `collection.ensured` | A collection was declared, or declared again. | `project`, `name`, `store`, `at`?, `owner`?, `made` |
| `collection.exported` | A library was written out as pages. It is said when something was written, not when what was there was already so. | `project`, `name`, `to`, `pages`, `files`, `left_out` |
| `collection.indexed` | A column of a table was kept in order by an index from here on, so that a table of many rows is read a window at a time by it: as it was first sorted or held by it, or as the table was declared. | `project`, `name`, `field` |
| `collection.history` | A line was written in a record's history. | `project`, `name`, `record`, `seq`, `topic`, `who`, `said` |
| `collection.forgotten` | A project's table was taken away, with its records and what was said of each, as the project was deleted for good. | `project`, `name`, `store` |
| `collection.icon` | Somebody chose the icon a field's column is drawn with, or took their choice away. | `project`, `name`, `field`, `icon`? |
| `collection.titled` | Somebody called a table something else for people, or took that away: its name, which everything else knows it by, is as it was. | `project`, `name`, `title`?, `by` |
| `collection.fields` | Somebody who looks at a collection changed its columns: added one, called one something else, changed what one holds or offers, or took one away. | `project`, `name`, `field`, `did` |
| `collection.views` | The views kept of a collection changed. | `project`, `name`, `view` |
| `device.added` | A device was let into this fleet: it redeemed its invitation. | `device`, `id` |
| `device.changed` | What a device says of itself changed. | `device` |
| `device.connected` | A device of this fleet opened its link. | `device` |
| `device.disconnected` | A device of this fleet lost its link. | `device` |
| `device.invited` | An invitation was made for a device to join this fleet. | `device`, `expires` |
| `device.joined` | This device joined a hub's fleet. | `hub`, `id` |
| `device.left` | This device left a hub's fleet. | `hub` |
| `device.refused` | A device that shook hands with this hub was not let in. | `id`, `from`, `why` |
| `device.removed` | A device was taken out of this fleet. | `device` |
| `device.sample` | How loaded this device is. Passed on, not kept. | `host`, `os`, `arch`, `cpus`, `total_mem_mb`, `free_mem_mb`, `free_disk_mb`?, `load1`? |
| `device.unjoined` | This device is no longer joined to the hub it was, and was not asked. | `hub`, `why` |
| `device.unproven` | What answers where this device's hub was is not its hub. | `why` |
| `extension.added` | A package was installed. | `extension`, `program`?, `actions`, `blueprints` |
| `extension.broken` | A package that is installed could not be read. | `problem` |
| `extension.removed` | A package was taken away. | `extension` |
| `handler.failed` | A handler could not start listening. | `handler`, `error` |
| `handler.runaway` | An event was not handled: it was at the end of a long chain of events, each said while the one before was being handled, in a short time. | `handler`, `topic`, `seq`, `depth`, `within_ms`, `error` |
| `hook.failed` | A hook could not be done. | `action`, `error` |
| `identity.given` | A person was given a scope beyond what their role holds, by somebody who holds it and `admin`. | `account`, `scope`, `given` |
| `identity.group.held` | What a group's members may ask of the factory beside what each holds itself was said: heard by whoever manages the factory. | `group`, `name`, `holds`, `keys` |
| `identity.group.joined` | A person or an agent was put in a group. | `group`, `name`, `member` |
| `identity.group.left` | A person or an agent was taken out of a group. | `group`, `name`, `member`, `keys` |
| `identity.group.made` | A group of people and agents was made, by whoever manages the factory. | `group`, `name` |
| `identity.group.removed` | A group was taken away, with everybody's place in it. | `group`, `name`, `members`, `keys` |
| `identity.group.renamed` | A group was called something else. | `group`, `name`, `was` |
| `identity.key.made` | A key was made: a credential for a script. | `key`, `name`, `account`, `scopes`, `with`? |
| `identity.key.revoked` | A key was withdrawn, and is nobody's credential any more; so is every key made with it, each said too. What it applied is stopped unless whoever withdrew it said to leave it running. | `key`, `name`, `account`, `with`?, `leave`? |
| `identity.key.rolled` | A key was given another secret: the one it had is refused from then, and it is the same key, holding what it held, so what it applied goes on. | `key`, `name`, `account` |
| `identity.password` | An account that had no password was given one, with an email to sign in by where it had none, to sign in another way than by a link. | `account` |
| `identity.registered` | An account was made. | `account`, `role`, `made`? |
| `identity.renamed` | Somebody called themselves something else. | `account`, `name`, `was` |
| `identity.signed_out` | Somebody ended every session they had, wherever each was begun. | `account`, `sessions` |
| `identity.taken` | A scope a person was given was taken away from them, and from the keys they made. | `account`, `scope`, `given`, `keys` |
| `identity.tier` | The plan this installation is on was set. | `tier` |
| `theme.chosen` | Somebody chose another theme: heard by them alone. | `owner`, `theme` |
| `theme.gone` | A theme somebody made was taken away: heard by them alone, and by whoever may read themes where it was offered to everybody. | `owner`, `theme`, `offered` |
| `theme.kept` | A theme somebody made was made or changed: heard by them alone, and by whoever may read themes while it is offered to everybody. | `owner`, `theme` |
| `theme.offered` | A theme was offered to everybody, beside the ones that ship. | `owner`, `theme`, `by` |
| `theme.withdrawn` | A theme offered to everybody was withdrawn, and is its owner's alone again. | `owner`, `theme`, `by` |
| `arranged.kept` | Somebody changed how they arranged the side panel (their groups of channels, the channels pinned to the top, what is pinned in Resources): heard by them alone. | `owner`, `arrangement`, `nth` |
| `clock.kept` | Somebody changed how a time is written for them (the zone and whether on a clock of twelve hours or twenty-four): heard by them alone. | `owner`, `clock`, `nth` |
| `appearance.kept` | Somebody changed how what they read looks for them (the size of its text, its width, and light or dark): heard by them alone. | `owner`, `appearance`, `nth` |
| `inbox.away` | Lines of somebody's inbox were put away by their owner, or went with what they opened: heard by their owner alone. | `owner`, `lines`, `place`?, `unread` |
| `inbox.follows` | Somebody follows something now, or follows it otherwise (how often they are told, a page with its pages or alone), or follows it no longer: heard by them alone. | `owner`, `kind`, `thing`, `follows`, `pages`?, `often`?, `at`? |
| `inbox.read` | Lines of somebody's inbox were read by their owner, who alone hears it. | `owner`, `lines`, `place`?, `unread` |
| `inbox.told` | Something was put in somebody's inbox, or a line there was told again: heard by its owner alone. | `owner`, `line`, `kind`, `place`, `about`, `count`, `unread` |
| `inbox.unread` | A line of somebody's inbox that was read was made to wait unread again by its owner, who alone hears it. | `owner`, `lines`, `place`?, `unread` |
| `instance.active` | An instance's steps ended well and it stays, listening. | `outputs` |
| `instance.text_left` | Where texts are followed: the text for an instance changed and does not check, so it was not taken. Said once for a text. | `blueprint`, `error`, `kept`, `left` |
| `instance.call_kept` | A step went on with another text of a blueprint than the one now there. | `blueprint`, `key`, `error`, `kept` |
| `instance.failed` | An instance's first run failed, or it could not be taken up again. | `error`, `reason`?, `called_by`?, `let_be`?, `run`? |
| `instance.finished` | An instance has ended, however it did: its steps went well and it had nothing to stay for, or it failed, or it was stopped. Said after `instance.failed` or `instance.stopped`. | `status`, `key`?, `outputs`?, `error`? |
| `instance.started` | A blueprint was applied and an instance of it began. | `blueprint`, `key`, `parent`?, `project`?, `revision` |
| `instance.state` | An instance's state changed. | names of its emitter's own |
| `instance.stopped` | An instance was stopped. | `blueprint`, `why` |
| `instance.update_failed` | An instance could not take what was applied over it. | `error`, `kept` |
| `instance.updated` | An instance came to run another text, or other params. Nothing said after this is handled by the text from before. | `blueprint`, `revision`, `from`, `in_place`?, `carried`? |
| `log.line` | A line something printed. Passed on, not kept. | `stream`, `text`, `service`? |
| `poll.failed` | What a handler asks on a timer could not be asked. | `handler`, `error` |
| `project.created` | A project was made, or a folder was taken up as one. | `project`, `name`, `path`, `brownfield`, `restored`? |
| `project.file.changed` | A file of a project was added, changed or removed. | `project`, `path`, `folders`, `kind`, `change` |
| `project.folder.made` | A folder was made in a project, empty, through the factory. | `project`, `path`, `folders` |
| `project.removed` | A project was taken away. | `project` |
| `project.renamed` | A project was called something else: what people read it as. Its id, which everything knows it by, is as it was. Kept with its row, in the factory's own journal. | `project`, `name`, `was`, `by` |
| `project.trusted` | A project's own files may stand in for the blueprints that ship: its factory's owner said so. | `project` |
| `project.untrusted` | That was taken back: the blueprints that ship are used there. | `project` |
| `project.rescanned` | So many files of a project changed that they are not each said. | `project`, `changed` |
| `project.branch.moved` | A branch of a project's repository moved, was made or was taken away, as its repository says: said only for a project something asked to have its branches followed (`branches.follow`), and a move made while the daemon was away is said once it is back. | `project`, `branch`, `from`, `to` |
| `project.blueprints` | Where a project keeps blueprints of its own was said: in the factory's place for them, in a folder of its repository read before that, or nowhere. Kept with the setting, in the factory's own journal, and heard as the project's. | `project`, `said`, `folder`?, `was`?, `was_said`?, `usual` |
| `project.git.changed` | Something of a project's repository may have changed: a commit, a branch made, moved or switched to, a file staged, a fetch. Said as the system says its files changed. Passed on, not kept. | `project` |
| `project.placed` | A project was put in a workspace: as it was made, out of another, or out of none by a factory that an earlier program made. It is always in one. Kept with its place, in the factory's own journal, and heard as the project's. | `project`, `workspace`?, `was`? |
| `project.arranged` | How a project's things are arranged in the side panel was changed: a folder made, renamed or removed, something put in one or out of it, a thing shown as something else, or the arrangement forgotten. Kept with the outline, in the factory's own journal, and heard as the project's. | `project`, `nth`, `by`, `forgot` |
| `project.setup.saved` | A setup new projects may be set up with was saved: rows of blueprints, under a name, or as the factory's default. | `name`, `rows`, `by` |
| `project.setup.removed` | A setup new projects could be set up with was taken away; the factory's default taken away is what ships again. | `name`, `by` |
| `topic.revised` | What a topic says was changed (its title, its body or its summary): a revision of it, kept whole, numbered from its first, which was its making. | `record`, `title`, `project`?, `workspace`?, `by`?, `nth` |
| `topic.answered` | A topic was answered: a reply was marked as what answered it, or its status said so. | `record`, `title`, `project`?, `workspace`?, `by`?, `answer`? |
| `topic.closed` | A topic was closed. | `record`, `title`, `project`?, `workspace`?, `by`? |
| `topic.opened` | A topic is open: made so, opened again once it was answered, closed or standing, or given a status where it had none. | `record`, `title`, `project`?, `workspace`?, `by`? |
| `topic.standing` | A topic is standing: it is never answered, and is kept up to date. | `record`, `title`, `project`?, `workspace`?, `by`? |
| `topic.untracked` | A topic's status was taken away: it is not tracked as a question, and stands nowhere until it is given one again. | `record`, `title`, `project`?, `workspace`?, `by`? |
| `topic.placed` | A topic was put somewhere else: in a workspace of its own, or in a project. | `record`, `title`, `project`?, `workspace`?, `by`?, `was`? |
| `topic.moved` | A page of a wiki was moved: under another page, or to the top, where it is a wiki of its own, or to another place among the pages beside it. Every page under it went with it. | `record`, `title`, `project`?, `workspace`?, `by`?, `parent`?, `position`, `was` |
| `topic.followed` | Somebody follows a topic now, or follows it no longer. | `record`, `title`, `project`?, `workspace`?, `by`?, `follows` |
| `project.flag.asked` | Somebody asked for a project's feature flag to be turned on, or wider, in an environment whose rule says who approves what goes under it, and is not one of them: what looks after the project's flags (`std/flags`) asks one of them to let it go on, and writes it once one does, only where it is still the change asked last of that flag there and the flag says what it said when it was asked. Kept on the environment in the write that says it. | `project`, `flag`, `name`, `record`, `kind`, `environment`, `variable`, `asked`, `value`, `was`?, `shown`, `by`, `by_name` |
| `project.flag.changed` | What a project's feature flag says in one of its environments changed: the flag's variable of the environment (FLAG_<NAME>) was set, or taken away, in the write that says so. It names the flag's row, whose history keeps it. | `project`, `environment`, `flag`, `name`, `record`, `variable`, `value`?, `was`?, `shown`, `by`, `by_name`, `asked_by`?, `asked_by_name`?, `approved_by`? |
| `project.flag.unchanged` | A change of a project's feature flag that waited for a go-ahead was let go on after the flag was changed, or asked for again, in that environment: nothing was written, so that an approval never undoes a later change. It names the flag's row, whose history keeps it. | `project`, `environment`, `flag`, `name`, `record`, `variable`, `value`?, `was`?, `now`?, `overtaken`, `shown`, `by`, `by_name`, `asked_by`?, `asked_by_name`?, `approved_by`? |
| `project.forked` | A blueprint that ships was forked for a project: its text was copied into the project, where the copy is what runs under the name from then on. What it was copied from is kept. | `project`, `blueprint`, `version`?, `revision`, `path`, `by`, `by_name` |
| `project.unforked` | A project's fork of a blueprint that ships was taken away: the one that ships is what runs under the name there again, once what is running is applied. What the fork was made from is forgotten. | `project`, `blueprint`, `path`, `version`?, `lost`, `by`, `by_name` |
| `project.environment.changed` | The variables or the secrets of a project's environment, the blueprint it is loaded by, or what it holds to, were changed. What changed is said by name, and no value is said. | `project`, `environment`, `set`, `changed`, `removed`, `marked`, `secrets`, `secrets_removed`, `secrets_marked`, `deploy`, `rules`, `by`, `by_name` |
| `project.environment.made` | An environment of a project was made: a named set of variables the project's instances are to be applied under, such as dev, staging or prod. It has no variables yet. | `project`, `environment`, `by`, `by_name` |
| `project.environment.ran` | A project's environment was loaded from a branch: the blueprint it says it is loaded by was applied for the project under it, handed the branch. How that goes is the run's to say. | `project`, `environment`, `branch`, `commit`, `blueprint`, `instance`, `run`?, `by`, `by_name` |
| `project.environment.ready` | The run a project's environment was last loaded by ended well: what it is loaded as is ready, with where it is reached and the cards it holds where the blueprint it is loaded by gave them out. | `project`, `environment`, `branch`, `commit`, `blueprint`, `instance`, `run`?, `took_ms`, `address`?, `cards` |
| `project.environment.failed` | The run a project's environment was last loaded by did not end well: it failed, or was cut short. | `project`, `environment`, `branch`, `commit`, `blueprint`, `instance`, `run`?, `took_ms`, `cards`, `reason`? |
| `project.environment.down` | What a project's environment was loaded as, once ready, was taken down: stopped, or failed, by whatever ended it. | `project`, `environment`, `branch`, `commit`, `blueprint`, `instance`, `run`?, `up_ms`, `address`? |
| `project.environment.removed` | An environment of a project was taken away, with its variables and its secrets. | `project`, `environment`, `vars`, `secrets`, `by`, `by_name` |
| `project.approval.asked` | A run waits for a person to let it go on (`approval.ask`), and whoever may decide was told in their inbox. | `ask`, `run`, `step`, `instance`, `blueprint`, `environment`, `asker`, `project`?, `to` |
| `project.approval.decided` | A run that waited to be let go on waits no more: somebody let it go on or declined, or it waited its time. The step that asked hears this. | `ask`, `run`, `step`, `instance`, `blueprint`, `environment`, `asker`, `project`?, `status`, `by`, `why`? |
| `record.ask.approved` | A change somebody asked for was approved and made, as theirs, in the same write as this is said. | `ask`, `id`, `project`, `collection`, `cells`, `asker`, `by` |
| `record.ask.decided` | A change somebody asked for came to something and waits no more: approved, declined, stale (its record was changed since), expired or withdrawn. Said for whatever waits on one, beside the event that says which. | `ask`, `id`, `project`, `collection`, `cells`, `asker`, `by`, `status`, `why`? |
| `record.ask.declined` | A change somebody asked for was declined, and nothing was made. | `ask`, `id`, `project`, `collection`, `cells`, `asker`, `by`, `why` |
| `record.asked` | Somebody asked for a change of a record they may not make themselves, and whoever could make it was told. | `ask`, `id`, `project`, `collection`, `cells`, `asker`, `why` |
| `record.commented` | Something was said on a record. | `id`, `project`, `collection`, `through`?, `fields`?, `body`, `by`, `public`?, `anchor`?, `decided`?, `once`?, `doing`?, `asker`?, `parent`? |
| `record.attached` | Something was attached to a record: a file kept beside it, read by whoever may read the record. | `id`, `project`, `collection`, `through`?, `fields`?, `attachment`, `name`, `media`, `size`, `by`, `restored`? |
| `record.detached` | Something attached to a record was taken away, and its file with it. | `id`, `project`, `collection`, `through`?, `fields`?, `attachment`, `name`, `media`, `size`, `by`, `trash`?, `into`? |
| `record.created` | A record was added to a collection. | `id`, `project`, `collection`, `through`?, `fields`?, `restored`?, `with`?, `wrote`?, `doing`? |
| `record.removed` | A record was taken away. | `id`, `project`, `collection`, `through`?, `fields`?, `with`?, `trash`?, `into`? |
| `record.reworded` | A number a record held came to a word, as its field was declared a choice that says what nothing stands for: once, as the table is declared so. | `id`, `project`, `collection`, `through`?, `fields`?, `field`, `was`, `now` |
| `record.updated` | A record's fields were changed. | `id`, `project`, `collection`, `through`?, `fields`?, `changed`, `approved_by`?, `asked`?, `chose`?, `flags`? |
| `router.failed` | A router could not be brought back at start. | `router`, `error` |
| `router.listening` | A router began listening. | `router`, `host`, `port` |
| `router.routed` | A router was told where to send new connections. | `router`, `backends` |
| `run.failed` | A run failed. It is said just before the run says it finished. Passed on as it is said, and kept in the journal only while some instance's text hears it or waits for it. | `flow`, `blueprint`, `error`, `reason`?, `duration_ms`? |
| `run.finished` | A run ended, however it went. Passed on as it is said, and kept in the journal only while some instance's text hears it or waits for it. | `flow`, `blueprint`, `status`, `duration_ms`?, `error`? |
| `run.resumed` | A run that was under way when the conductor stopped was taken up again. Passed on as it is said, and kept in the journal only while some instance's text hears it or waits for it. | `flow`, `blueprint` |
| `run.started` | A run of a flow began. Passed on as it is said, and kept in the journal only while some instance's text hears it or waits for it. | `flow`, `blueprint` |
| `search.indexed` | What can be searched was brought up to date. | `docs`, `written`, `took_ms` |
| `search.progress` | How far reading for the search has got. Passed on, not kept. | `phase`, `done`, `total`, `docs` |
| `secret.removed` | A secret was taken away. | `name`, `by_name` |
| `secret.set` | A secret was set, or set again. Its name is said and never what it is. | `name`, `by_name` |
| `service.exited` | A service's program ended by itself. | `service`, `code`?, `while`? |
| `service.failed` | A service was given up on. | `service`, `reason` |
| `service.healthy` | A service answered its health check, or has none to answer. | `service` |
| `service.restarting` | A service that ended is to be started again. | `service`, `attempt`, `in_ms` |
| `service.started` | A service's program was started. | `service`, `pid`?, `port`? |
| `service.stopped` | A service was stopped, as it was asked to be. | `service` |
| `service.unhealthy` | A service stopped answering its health check. | `service`, `reason` |
| `step.cancelled` | A step was stopped before it ended, or never came to run. Passed on as it is said, and kept in the journal only while some instance's text hears it or waits for it. | `attempt`? |
| `step.failed` | A step failed. Passed on as it is said, and kept in the journal only while some instance's text hears it or waits for it. | `attempt`, `error`, `code`?, `timed_out`?, `tolerated`?, `duration_ms`? |
| `step.finished` | A step that began has ended, however it went. Said after `step.failed` or `step.cancelled`. Passed on as it is said, and kept in the journal only while some instance's text hears it or waits for it. | `attempt`, `status`, `duration_ms` |
| `step.retrying` | A step that failed is to be tried again. Passed on as it is said, and kept in the journal only while some instance's text hears it or waits for it. | `attempt`, `error`, `in_ms`, `by` |
| `step.skipped` | A step was not run. Passed on as it is said, and kept in the journal only while some instance's text hears it or waits for it. | `when`? |
| `step.started` | A step began, or began again. Passed on as it is said, and kept in the journal only while some instance's text hears it or waits for it. | `attempt`, `verb`, `target`, `resumed` |
| `step.waiting` | A `wait` step began waiting. Passed on as it is said, and kept in the journal only while some instance's text hears it or waits for it. | `event`?, `ended`?, `until`? |
| `system.copied` | This factory was started from a snapshot of another. | `snapshot`, `from`, `profile`, `note`, `made`, `arrived`, `files`, `projects`, `left_out`, `removed`, `without_files`, `announced` |
| `system.degraded` | Something this device does less well than it could. | `what`, `reason`, `effect` |
| `system.experiment` | Something of the program's that is not yet whole was turned on or off for this factory, in Settings, Experiments. | `name`, `on`, `by`, `by_name` |
| `system.move_cleared` | What a move of a project to a journal of its own left when it was cut off (a file that no project's journal is kept in) was found and taken away, as the daemon started. | `files`, `bytes` |
| `system.moved_apart` | A project of a factory from before was moved to a journal of its own, while the daemon was stopped: what is run for it is kept there from now on, by a conductor of its own, with its tables, their records and what its agents were asked. Said by the daemon that next starts, once for each project. | `moved`, `journal`, `rows`, `beside`, `at`, `took_ms` |
| `system.logs` | How long the detail of runs' steps is kept, or where their output is written, was said. | `days`?, `was`?, `folder`?, `by`, `by_name` |
| `system.logs.let_go` | What runs' steps printed and what the runner said of them as they went, older than the factory keeps, was let go of. What runs came to is kept. | `before`, `days`, `events`, `files`, `bytes`, `held`? |
| `system.profile` | The variables of this profile, the copy of the factory, were changed. | `set`, `changed`, `removed`, `by`?, `by_name` |
| `system.texts` | How what is running comes by new texts, or how a taken one lands, was changed. | `come`, `land`, `was`, `landed`, `by`, `by_name`, `gives_up`? |
| `system.comparison.added` | Ways or counts were added to a comparison whose ways are named by the start, as a blueprint asked for it again. | `name`, `project`?, `of`, `per`?, `also`?, `also_per`?, `counts`?, `ways`, `by`, `held`, `started`, `ended`?, `goes`?, `until`?, `asked`?, `rule`?, `rolling`?, `shared`? |
| `system.comparison.ended` | A comparison was ended, or which way everything goes after it was said again. | `name`, `project`?, `of`, `per`?, `also`?, `also_per`?, `counts`?, `ways`, `by`, `held`, `started`, `ended`?, `goes`?, `until`?, `asked`?, `rule`?, `rolling`?, `shared`? |
| `system.comparison.ruled` | A comparison was given a rule its shares change by, or had it taken away. | `name`, `project`?, `of`, `per`?, `also`?, `also_per`?, `counts`?, `ways`, `by`, `held`, `started`, `ended`?, `goes`?, `until`?, `asked`?, `rule`?, `rolling`?, `shared`? |
| `system.comparison.shared` | A comparison's shares were said again: by its rule, as what it counts came to, or by somebody; the same where the rule held them. | `name`, `project`?, `at`, `by`, `was`, `now`, `why` |
| `system.comparison.started` | A comparison was started: from now, one blueprint is started one of several ways. | `name`, `project`?, `of`, `per`?, `also`?, `also_per`?, `counts`?, `ways`, `by`, `held`, `started`, `ended`?, `goes`?, `until`?, `asked`?, `rule`?, `rolling`?, `shared`? |
| `system.named` | The daemon was told a name it is asked under, and answers to it. | `name` |
| `system.reindexed` | The journal of a factory from before had its index of when events were said made anew, as this program first started on it. Said once. | `took_ms`? |
| `system.repack_cleared` | A copy of the database that a `repack` was cut off writing was found beside it and taken away, as the daemon started. | `bytes` |
| `system.repacked` | The factory's database was written out again as one is made now, while the daemon was stopped. Said by the daemon that next starts. | `before`, `after`, `events`, `took_ms` |
| `system.restarting` | A step asked for the daemon to be stopped and started again by what keeps it running. | `at_rest` |
| `system.snapshot` | A snapshot of the factory was written. | `snapshot`, `to`, `profile`?, `note`, `made`, `files`, `bytes`, `holds`, `left_out`, `took_ms` |
| `system.started` | The daemon started. | `address`, `version`, `resumed`, `note`?, `profile`?, `runs_from` |
| `system.stopped` | The daemon stopped. | nothing |
| `system.across` | What every page says across its top changed: whoever manages the factory said it may stop, or what it is to say. | `across`?, `quiet` |
| `system.draining` | The daemon was asked to stop once what is in hand is done: it begins nothing new from now, and stops when the turns in hand and the runs under way have ended, or at the bound. | `since`, `until`, `turns`, `runs` |
| `system.stopping` | The daemon was asked to stop. | `at`?, `of`?, `drained`?, `cut`? |
| `system.unnamed` | A name the daemon answered to was taken back. | `name` |
| `trash.gone` | Something in the Trash was deleted for good. | `id`, `kind`, `thing`, `project`?, `why` |
| `trash.put` | Something was deleted, and is kept in the Trash until it is given back or goes for good. | `id`, `kind`, `thing`, `project`? |
| `trash.restored` | Something was given back from the Trash, where it was. | `id`, `kind`, `thing`, `project`?, `to`?, `with`?, `said`? |
| `workspace.created` | A workspace was made: a name for the projects that belong together. | `workspace`, `name` |
| `workspace.renamed` | A workspace is called something else. Its id is what it was. | `workspace`, `name`, `was` |
| `workspace.removed` | A workspace was taken away. What was in it is in another, and is otherwise as it was. | `workspace`, `name`, `projects`, `channels`, `to`? |

### Who is told of an event

A handler hears everything that is its own, and of the rest only what
whoever applied its blueprint may read. It runs as the instance it belongs
to, with that instance's authority and no more.

Anything outside that follows what happens (the client, a script reading
`/api/events`, `mw events --follow`) is likewise told only what its
credential lets it read. This matters because events carry content: what
is said in a channel is in the event that says so, and a line a step
printed is in the event that passes it on.

Each event is described with the scope whose holder may be told of it
(`heard` in `contract/offered.txt`):

| Events | Told to whoever holds |
|---|---|
| `project.*`, `workspace.*` | `projects:read` |
| `channel.*` | `channels:read` |
| `inbox.*` | `inbox:read`, and only the owner of that inbox: nobody else is told, whatever they hold |
| `theme.*` | `themes:read`, and of a theme that is somebody's alone, and of what they chose, only them: nobody else is told, whatever they hold |
| `arranged.*` | `arranged:read`, and only the one whose arrangement it is: nobody else is told, whatever they hold |
| `board.*` | `board:read` |
| `record.*`, `collection.*` | `records:read` |
| `run.*`, `step.*`, `instance.*`, `handler.*`, `hook.*`, `poll.*`, `log.line`, `system.profile`, and any event a blueprint says for itself | `blueprints:read` |
| `agent.*`, `extension.*` | `agents:read` |
| `service.*`, `router.*`, `device.*`, `system.degraded`, `system.named`, `system.unnamed` | `fleet:read` |
| `secret.*` | `secrets:use` |
| `search.*` | `search` |
| `identity.*`, `caller.*`, `system.snapshot`, `system.copied` | `admin` |
| `system.started`, `system.stopping`, `system.draining`, `system.stopped`, `system.restarting`, `client.updated` | anyone signed in |

### What is offered is kept, and held to

Everything a blueprint can depend on is written down in one file kept with
the code, `contract/offered.txt`: every action with what it takes, gives
and needs, and every event with what it carries. What a package that ships
brings is written down the same way in the package's own folder
(`extensions/<name>/contract/offered.txt`), since nothing in the
application names a package. A test writes down what is offered now, the
same way, and compares:

- Something **added** fails the test, showing what was added and the
  command that writes the file again:
  `WRITE_CONTRACT=1 cargo test -p api the_contract`.
- Something a blueprint written before **could break on** fails the test,
  showing exactly what. That covers an action, an argument, an output, a
  topic or a field removed or renamed; a type changed; an argument that
  must now be given; an event no longer kept. It is written down only by
  saying that it is meant, and why:
  `BREAK_CONTRACT="why" cargo test -p api the_contract`. What was broken is
  added to `broken.txt` beside the file, with the day and the sentence.
  Nothing is ever taken off that list.

When the daemon starts and a kept text no longer checks, because an action
it uses is gone, the instance fails as before. Its error points at that
list: what was removed, when, and why. Where the list says nothing of it,
the error says the text may be from an earlier version.

## Writing one

#### Colours and completion

In the client a blueprint is coloured as a blueprint. The words of the
language, the names you chose (steps, params, handlers) and your own keys
(arguments, state) each look different, so a misspelled or misplaced field
does not look like a field. What is inside `${ }` is set apart.

As you type, what can go where the caret is is offered:

- the fields a step or a handler may still have;
- the actions this device has, after `do:`;
- an action's arguments under `with:` (or the params of the blueprint a
  `call:` names);
- the other steps, after `needs:`;
- inside `${ }`, the names an expression may start with: your params and
  state, the steps above, and the outputs of each step's action.

Ctrl-Space asks for suggestions on an empty line. Pointing at a word says
what it is.

The same goes inside an argument, where its action says what it reads
there (*What is written inside an argument*, above):

- in a field of a table, the keys a field takes are offered, less those it
  already has, in braces or on lines of their own;
- after a key that takes one of a few words (`type:`, a view's `kind:`),
  those words are offered;
- pointing at a key says what it means and what it is a key of; pointing
  at one of its words says what that word means.

A key that no action names gets no description and no mark. Whether it is
accepted is the action's to say when the step runs.

#### The Reference

All of this is on one page in the client, the **Reference**: every word of
the language with what goes after it; every action this device has, with
what it takes (down to what is written inside an argument), gives and
needs; and every event, with what it carries and who hears it.

- It is the line over the blueprints in the side panel, and a line of
  Help's menu.
- The card about a word you point at in a blueprint's text has a link that
  opens the Reference at that word.
- A box at its top finds what you type, in a name or in a sentence.
- Each entry has a mark that copies an address that opens the page there.

It is made from the same descriptions as the tables on this page and as
`mw actions`, `mw events --kinds` and `mw schema`, and writes none of its
own. A package you add is on it once it is installed.

#### Examples

Every word there, and every action you would write in a blueprint of your
own, says what it is for and is shown by an **example**: a whole
blueprint, a few lines long, written with it, coloured as the editor
colours it, with a mark that copies it. Tests hold each example: it reads,
checks with no warning against every action there is, and is laid out as
the formatter lays it out. So one copied from the page can be applied as
it stands.

Under each example are the blueprints that ship that use the word, the
shortest first; each line opens that blueprint's text at the place. The
card about a word you point at shows the example too.

A word's example is written where the word is
(`crates/blueprint/src/vocabulary/shown.rs`), and an action's beside the
action (`.example(...)`; the studio's in `crates/kernel/src/asked.rs`). A
new word or action without one fails a test that says where to add it.
Where the blueprints use each word is worked out from their texts, and is
written nowhere.

#### Finding and replacing

Ctrl-F (⌘F on a Mac) finds in the whole text, not only the lines on
screen. A bar opens over the text's top right, the matches are marked as
you type, and Enter and Shift-Enter go to the next and the previous one. It
can match case, whole words, or a regular expression.

Where the text is yours to change, Ctrl-H (⌘⌥F on a Mac, and Ctrl-Alt-F
anywhere) adds a line that replaces one match or all of them. A blueprint
that ships is read-only, and offers no replacing. Escape closes the bar.
It works the same in any file opened as text.

None of this is a second copy of the language. The daemon hands the editor
this page's fields, as it hands `mw actions` the actions.

### In an editor of your own

A blueprint is written as YAML, and the editors people already have
complete and check YAML against a schema. Three things make yours do that
for blueprints.

#### Call the file so that the editor knows what it is

A blueprint's file may be called `<name>.bp` or `<name>.bp.yml` (or
`.bp.yaml`). It is the same blueprint either way, wherever one is found by
its name: a project's own, its `fleet`, what `mw apply`, `mw check` and
`mw fmt` take, the client's tree. With the second name, an editor takes it
for YAML with nobody telling it.

- A file that is `.yml` and nothing more is never a blueprint: a project
  is full of YAML that is somebody else's.
- One name has one file. A folder that holds both `fleet.bp` and
  `fleet.bp.yml` has not said which is meant, so neither is read, and
  whoever asks for `fleet` is told of both.

#### Give it the schema

`mw schema` prints the language as a JSON Schema. It has every word and
what goes after it, with the sentence that says what it is for, and the
actions on this device with what each takes. That goes down to what an
action says is written inside an argument: each key with its sentence, and
each of a key's few words. These are described and held to nothing, so a
key your editor is not told of is not marked.

```sh
mw schema > blueprint.schema.json
```

The common language server for YAML (behind the YAML support of most
editors) reads which schema a file is held to from a first line of the
file itself:

```yaml
# yaml-language-server: $schema=./blueprint.schema.json
blueprint: greeter
```

`mw fmt` keeps that line first. Or say it once for a project, in the
editor's own settings: map the schema's file to `*.bp` and `*.bp.yml`. In
an editor that keeps a project's settings in `.vscode/settings.json`, with
its YAML extension:

```json
{
  "files.associations": { "*.bp": "yaml" },
  "yaml.schemas": { "./blueprint.schema.json": ["*.bp", "*.bp.yml", "*.bp.yaml"] }
}
```

Nothing writes into an editor's settings for you. The schema of the
program alone, with no package's actions, is kept in the repository as
[`contract/blueprint.schema.json`](../contract/blueprint.schema.json). A
factory's own (`mw schema`, `GET /api/blueprints/schema`) adds what its
packages bring: run it again when you add a package.

#### Lay it out

`mw fmt` writes a blueprint so that an editor reads all of it. The reader
here accepts a few things written plain that YAML does not, among them a
value with `: ` inside it and a `${ ... }` written bare inside `{ }`. An
editor marks each of those as it reads the file, before any schema is
asked. `mw fmt` puts them in quotes, where each is the same to the reader
and valid YAML to an editor:

```yaml
description: "A board: its cards, and what moves them"
with: { rounds: "${ params.rounds + 1 }" }
```

Every blueprint that ships is written so. A test reads each with a YAML
reader that is not this program's, and holds what that reader made of it
to the schema. Written the plain way, a blueprint still checks and still
runs. For a file called `.bp.yml`, `mw check` gives a warning for each line
an editor will mark, and says that `mw fmt` fixes them.

#### The schema helps; `mw check` decides

The schema says less than the check, on purpose:

- whatever may be worked out (`${ ... }`) is allowed to be text;
- the name of an action or a blueprint is not held to what exists;
- what the reader refuses that YAML allows (an anchor, a tag, a second
  document in the file) no schema can say.

So something your editor accepts may still be refused when it is checked.
The other way round is a bug in the schema: what checks, once laid out,
must not be marked by an editor. A test holds every blueprint that ships to
that.

## Laying one out

A blueprint is laid out one way, by one formatter, whoever wrote it:

```sh
mw fmt                       # every blueprint under this folder
mw fmt blueprints/fleet.bp   # one file, or a folder
mw fmt --check               # rewrites nothing; says which would change, and exits 1 if any would
```

The editor does the same when a blueprint is saved, before it is applied,
and on Shift-Alt-F. It is never done as you type. The cursor stays where it
was in the text, and one undo takes the whole of it back.

#### What it changes

Layout, and nothing else:

- indentation; the spaces after `:` and `,` and inside braces and
  brackets; spaces at the ends of lines; one line break at the end;
- blank lines: a run of them becomes one. A step, a handler, a param or a
  top-level section written over several lines is set apart from its
  neighbours by one blank line. Things that are each on one line stay
  together as you left them;
- the spaces just inside `${ }`: none around a bare name
  (`${params.item}`), one on each side of anything else
  (`${ event.data.to == "ready" }`);
- a map or a list written in braces that no longer fits its line becomes a
  block, each item on a line of its own. This keeps a long
  `with: { ... }` on the screen. One that fits is put back on one line;
- quotes, around a value written plain that YAML does not read the way it
  is read here: one with `: ` in it, or a `${ }` written bare inside
  braces. The value is the same quoted or not, and the file is then YAML
  to everything else that reads YAML, an editor first (*In an editor of
  your own*, above). The reader goes on accepting both unquoted.

#### What it never changes

- what the blueprint means;
- the words of a comment, and what it stands before or at the end of;
- how a quoted string is quoted;
- a long sentence (a `description` is not broken to fit);
- a block, which stays a block however short;
- the lines of a `|` or `>` block, which keep their indentation past the
  first;
- the order things are written in.

It checks the first of these every time it runs: a text that would come to
mean anything else is handed back unchanged. A text that does not read as
a blueprint is left exactly as it is, and `mw check` says why.

Laying a blueprint out is not a change to it. An instance running a text is
not behind one that differs only in layout and comments, and applying that
one does nothing. The instance goes on showing the text it was applied
with, as it was written then.

#### Settings of a project's own

A project can set a few things in a file called `blueprint-format.yaml`. It
is looked for in the blueprint's own folder and then in each folder above
it, up to the top of the project (from the terminal: the top of the
repository). The nearest one counts, by itself. It is written the way a
blueprint is, one setting a line:

```yaml
width: 120
expressions: spaced
```

| Setting | Unless said | May be | What it does |
|---|---|---|---|
| `width` | `100` | a whole number from 40 to 1000 | How wide a line may be before a map or a list written on it becomes a block. Comments and sentences are never broken to fit. |
| `indent` | `2` | a whole number from 1 to 8 | How many spaces each level is set in by. |
| `blank_lines` | `1` | a whole number from 0 to 3 | How many blank lines set apart a step, a handler, a param or a section at the top that is written over several lines. With 0 none is added, and one the author left stays. |
| `one_line` | `fits` | `fits`, `never` or `always` | When a map written in braces stays on its line: while it `fits` the width, `never` (it is always written as a block), or `always`, however long. A list in brackets goes by the width unless this is `always`. |
| `expressions` | `mixed` | `mixed`, `spaced`, `tight` or `written` | The spaces just inside `${` and `}`: `mixed` is none around a bare name and one each side of anything else, `spaced` one each side always, `tight` none, `written` as the author left them. |
| `order` | `written` | `written` or `vocabulary` | The order of the fields of a step, a handler, a param and the top of the file: as `written`, or as the language lists them (`vocabulary`). Names of the author's own (steps, params, handlers) always stay in the order written. |

Anything else in that file is refused, with a list of what can be set. The
blueprints that ship have no such file, and `cargo xtask check` fails when
one of them (`blueprints/std`, `examples`) is not laid out this way: run
`mw fmt` on that folder to fix it. Blueprints written inside another file
(a package's `extension.yaml`, the fleet's in the daemon) are not laid out
by this.

## Checking and running

```sh
mw check fleet.bp            # what is wrong with it, with line numbers
mw explain fleet.bp          # what it is and does, in plain words (*In plain words*)
mw sandbox fleet.bp          # run it where nothing is real, and see what happens (*In a sandbox*)
mw fmt fleet.bp              # lay it out the one way (*Laying one out*)
mw graph fleet.bp            # the graph, as text
mw apply fleet.bp            # start it, or update the instance that exists
mw apply fleet.bp --param port=8080 --project shop
mw instances                 # what is alive, the text each runs, and which are behind it
mw apply --instance <id>     # apply one again as it is (see below)
mw run <id>                  # one run, step by step
mw logs <run> <step>         # a step's output
mw logs <run>                # what each step printed, and its outputs (*Playing one to see what it does*)
mw stop <instance>           # stop it, its children and what it holds
```

`mw apply --instance <id>` says how the instance took the text: where it
is, carried over (`on_update`), or with its steps run again.

#### How long step detail is kept

What a step printed, and what the runner said of each step as it went
(every `step.*`, `run.started` and `run.resumed`), is kept for as long as
the factory's owner says (`mw logs --keep <days>`, and in Settings). Unless
said, that is for ever. Those runner events are kept in the journal at all
only where a text hears them (*What is said of steps and runs is passed
on*).

- A run whose step detail was let go is still listed with how it ended,
  and each step with how it ended and its outputs. It says that its detail
  went (`let_go`, in the daemon's words).
- A handler or a `wait` that hears those events never loses one it has not
  been handed yet.

#### Errors

`check` and `apply` report each mistake on one line, as
`file:line:col: error: message`. A name read in an expression that does
not exist (a param, a step, a key of `state`, `event` outside a handler) is
pointed at where it is written, with the name that was probably meant
where one is close. Applying to a project that does not exist is refused,
and starts nothing.

### A run as a pipeline run

A pipeline is a blueprint, and every run of every blueprint is a pipeline
run: the steps of a text, in the order its `needs` make, are the stages of
getting something to some state. Nothing is added to the language for it,
and nothing in the runner changes.

"Stage" is the client's word only, as a fleet's stages are a card's. The
language says steps and tasks, and the runner says neither.

#### A run's page

A run has a page of its own in the client, at the address `/run/<run>`
(opened as any address is, once whoever opens it has signed in). The page
shows:

- what ran, under which environment, and with which branch and commit
  where it was handed them;
- what set it off: applied, and by whom, or the event its handler heard;
- how it stands, and how long it took;
- its **stages**.

A stage is one of:

- a step at the top of the run's flow, on its own (a stage of one);
- a `task`, with the steps inside it;
- a `call` or a `spawn`, unfolded in place into its child's own run.

Each stage is shown by what it says it is for (`description`), with its
key under it, how it stands, when it began, how long it took and its share
of the run's time. Stages the graph puts in one layer run side by side and
are drawn so.

Under each stage is every step's status and time, why it failed, and what
it wrote. Output is read from its end through
`/runs/<run>/steps/<step>/log?tail=<lines>` and drawn a window at a time;
Show earlier reads what came before (`?after=<line>&limit=<lines>`). The
run's graph, lit by how each step went, is a view of the same page. A run
of a text that can no longer be read is shown in stages by its steps'
names.

A run is a link to its page wherever one is named. The address
`<factory>/run/<run>` in a told line's part of `links` opens it as a tab of
the page that shows the line, and so does the older form from before
addresses were paths (`#/run/<run>`, as `<factory>/#/run/<run>`). Every page of the client is
named by a path this way (`/<project>/<table>#<view>`; see
`ui/src/tab.ts`).

#### A project's pipelines

A project's pipelines are its own blueprints kept under `pipelines/` in
the factory's place for them (`@blueprints/pipelines/<name>.bp`, called
`pipelines/<name>`). The folder only says which of the project's
blueprints are listed first and offered Run.

Its Pipelines page is a line under the project in the side panel once it
has any pipeline, and a line of its page's gear before then. It lists:

- each pipeline, with what it says it is for, its latest runs (each
  opening that run's page), and Run;
- under them, folded, everything else applied for the project, with its
  latest runs, since those are pipeline runs too.

Run applies the pipeline as whoever presses it, through the apply route,
with `force` so that its steps run again. Where it is handed something, or
the project has environments, it asks first in the drawer what it is
handed and which environment it runs under. New pipeline… makes one, with a
stage to begin from.

A pipeline's runs are found by
`GET /api/instances?project=<p>&blueprint=pipelines/<name>`, with
`environment=<name>` for what ran under one of the project's environments
(the empty name for under none).

```yaml
blueprint: pipelines/check
description: Builds the shop and checks it.
steps:
  build:
    description: Build it.
    task:
      compile: { do: shell, with: { cmd: [cargo, build] } }
      test: { do: shell, with: { cmd: [cargo, test] }, needs: [compile] }
  deploy: { call: pipelines/deploy, as: deploy, needs: [build] }
```

### In plain words

A blueprint's text is written for whoever writes blueprints. Somebody who
does not can still see what one is and does. `mw explain <file|name>`
prints it; in the client it is the Summary of a blueprint's page. A
blueprint that ships opens on its Summary, and so does a project's own for
somebody who may not change it.

```sh
mw explain std/board
mw explain blueprints/release.bp --project shop
```

It has these parts, each left out where there is nothing to say:

| Part | What is under it |
|---|---|
| What it is | Its `description` and `version`; whether what it is doing is written down as it goes; what it keeps and gives out; and what whoever applies it may tell it: each param by its `description`, with its default where it has one. |
| What it makes | What its steps bring into being that is still there after the run: a table, a channel, an agent, a service, another blueprint started or called under it. An action says this of itself, with the argument that names the thing. A name that is worked out is said by what says it (`called what the param table says`). |
| What it does when it is applied | Its steps in order, each in a sentence. |
| What it hears, and what it then does | Each handler: the event, as the program describes it; whose events it hears; which of them (`for:`, `when:`); and its steps. |
| What it may do | That it runs as whoever applies it; what its actions need leave for, each in the words of the scope; and, marked, each command it runs and each secret it hands to one. |
| What it does as it ends or is changed | `on_fail`, `on_finish`, `on_update` and `on_stop`. |

It is worked out from the text and from what each action and each event
says of itself. No model is asked and nothing is run, so it is the same
every time, and you can have it for a blueprint that was never applied.
What a blueprint says of itself is therefore what is read:

- A step is said by its `description`, up to its first full stop. Write
  that sentence as what the step is for. A step with no description is
  said by its action's own sentence (the one `mw actions` shows). A step of
  the language's own (`emit`, `set`, `wait`, `stop`) is said by what it is
  handed: `Say shop.told.`
- A param is said by its `description`, whole.
- A `when` is said as the question it asks, where it is plain: a name, one
  plain question of it, or up to three joined by one word (`Only when the
  param count is more than 3`). One that works something out is said to be
  a condition, as written.
- A handler's event is said by its description in the table under *The
  events there are*; a topic of your own, by its name.

Nothing in it is a judgement. In the client, what a blueprint's runs have
cost is shown under its Summary, from what was measured under its name
(how many runs, how many failed, how long one typically took, and
anything else measured under it that is not a length of time), and only
where something was measured.

### In a sandbox

`mw explain` says what a blueprint would do. `mw sandbox <file|name>` has
it done somewhere it cannot matter: a *sandbox test*. The daemon makes a
sandbox, applies the blueprint in it, and says what happened. A sandbox is
a factory of its own with nothing of yours in it:

- a journal in memory;
- a clock that only the test moves;
- one made-up project called `sandbox`, owned by a made-up person who may
  do anything there.

```sh
mw sandbox std/board
mw sandbox std/fleet --first std/board --do board.add title="Try it" status=ready
mw sandbox blueprints/release.bp --project shop --advance 10m --say release.asked version=2
```

#### What is real and what is stood in

What a blueprint does to the factory's own tables is done for real, in the
sandbox's tables: a table declared, a card added and moved, a row, a
channel and what is said in it, somebody told something (`inbox.tell`),
and every block that only works something out. Whoever is told something
there is the made-up person: `applier` is them, and a handle names nobody,
since the sandbox has no account but theirs. Every table is kept in the
sandbox's journal: one kept as files, as a library's documents or by a
store service is refused.

Everything that reaches outside those tables is **stood in**: a command
(`shell`, a service, a router), an agent brought up or asked, a
repository, a file of a project read or written, a library written out,
another device, a copy of the factory. A stand-in has the same name and
takes the same arguments, so a text that checks still checks there. It
runs nothing. It writes down what it was asked, and answers with the first
of:

1. what you said its action answers (`--tell shell code=3`);
2. what the action says a stand-in answers: a command ends with 0 and says
   nothing, a device has room, nobody of a fleet is there yet, a card's
   place is as in a project that is no repository;
3. nothing.

So a step that asks an agent is answered that nobody was there, unless it
is told otherwise: `--tell agent.task available=true decision=done`.

#### What you can do to it

What is asked after the blueprint is done in the order it is written:

| Flag | What it does |
|---|---|
| `--first NAME` | Apply that blueprint first: what the text needs to be there before it, such as a board before a fleet. |
| `--param k=v` | What the blueprint is told. A blueprint that asks which `project` it is for is told `sandbox`. |
| `--say TOPIC k=v...` | Say an event, carrying what follows. One this program describes is held to what it carries. A handler hears it once its instance's steps have ended, as anywhere. |
| `--do ACTION k=v...` | Do something to the tables by hand, as the made-up person: `board.add`, `board.move`, `board.update`, `record.add`, `record.update`, `record.remove`, `record.comment`, `channel.ensure` (a channel for the text to say things in) or `channel.post`, each with what the action of that name takes. |
| `--advance 5m` | Move the sandbox's clock on: a `wait` goes on, a timeout runs out, an `every:` fires. Nothing else moves it. |
| `--tell ACTION k=v...` | What a stand-in for that action answers, each time it is asked. |
| `--project P` | Read P's own blueprints, so the text may call them and a name is found among them. Nothing else of P is in a sandbox. |

#### What it prints

What happened, in the order it came about:

- each run under a heading that names its instance;
- each step by what it says of itself, with its key and what it does, and
  its outputs;
- what was said and started between steps;
- under a step that was stood in, what it was asked and what it answered.

What an action says of a table or a channel is under the run it was said
for. What was said for no run (an event said by hand) stands apart. At the
foot are the tables and channels the sandbox ended with, what was told to
whom, and the actions that were stood in. `--json` prints all of it as the
daemon hands it. The command exits with an error where something in the
sandbox failed.

#### Through the API

Four routes, for whoever may read blueprints (`blueprints:read`). That is
all a sandbox takes, since nothing it does is real.

| Route | What it does |
|---|---|
| `POST /api/sandboxes` | Makes one and applies the text. Takes `source` or `name`, and `project`, `params`, `first`, `told`, and `answers`: `asked` (a stand-in waits to be answered; the default) or `default` (it answers at once). |
| `POST /api/sandboxes/<id>/acts` | Does one thing to it: `apply`, `say`, `do`, `advance`, `answer` a waiting stand-in with what it `gives` or why it `fails`, or `answers`. Answers when what came of it is at rest. |
| `GET /api/sandboxes/<id>` | Hands back everything. |
| `DELETE /api/sandboxes/<id>` | Ends it (`DELETE` on the same address). |

Limits:

- A sandbox belongs to whoever made it, alone.
- A daemon has at most eight at once.
- One that is asked nothing for a quarter of an hour is ended.
- It is held to a few thousand steps and a few megabytes: what its
  journals keep and what its steps write to their logs, together. It is
  meant to be light.

#### Several texts side by side

`POST /api/sandboxes/beside` tries several texts of one blueprint against
the same things done. It takes:

- `texts`, two to four, each `called` what heads its column, and either
  its `source` or `ships` (what ships under `name`);
- what one sandbox is made with, and the `acts` to do to each, as the
  route above takes them. An `answer` that says `usual` is answered in
  each sandbox with what a stand-in for its action usually answers.

Each text gets a sandbox of its own, and the acts are played in each. An
answer for a stand-in a text does not ask is skipped there, and said so
(`skipped`). An act one sandbox refuses ends its playing there
(`refused`).

What each came to is handed back whole (`sandboxes`), and laid side by
side as a comparison's columns are (`beside`): a column for each text, and
rows by kind. `came` is each event said that is not the runner's own
report of its work; `took` is what was measured of what ran, and each
action stood in, by how often. It counts and never scores: no column is
ordered, marked or named as the better one. The sandboxes are kept like
any others and ended by their own route. Where a text does not check, what
is wrong with each is said and no sandbox is made.

#### In the client

On a blueprint's page, **Testing** in its head holds what tries a text
without changing what runs: Preview, Sandbox test, Side by side and Compare
on real work, each with a few words on what it does.

**Sandbox test** runs the text as it stands there, saved or not, and shows
a view called Sandbox.

- On one side is what is done to it: what goes first, what it is told, and
  Run. Then: an event to say (its topic, with a box for each thing it
  carries), something done by hand, the clock moved on, and each stand-in
  that waits, with what it was asked. A waiting stand-in can be answered
  three ways: with what a stand-in for it usually answers, that it fails,
  or with an answer written out as JSON. Or every stand-in can be told to
  answer as usual from then on, which is also how a test can be begun.
  Which project a blueprint is for is not asked: it is told the sandbox's
  one.
- On the other side is what happened: how it stands, in a sentence; the
  blueprint's graph, with a mark on each step that ran, was stood in, was
  passed over or failed; each run with its steps in the order they came
  about; the tables with their rows; what was said in channels; and what
  somebody was told.
- At its foot, always in view, is what it was: a sandbox test, what was
  stood in and how often, and that passing there is not passing for real.

Everything done to it is kept in order. After you change the text, **Run
again** does the same things to the new text and says what differs from
the run before. The sandbox is ended when its tab is closed, and by the
daemon when nobody has asked it anything for a quarter of an hour; the
page then says so and runs it again. Nothing of a sandbox is listed
anywhere else in the client: it is not the factory's.

**Side by side**, under it, plays the same things against two or more
texts of the blueprint, each in a sandbox of its own:

- the text as it stands on the page;
- the file as it was saved;
- the default, as it ships;
- each project's fork of it, as many as there are to read.

A text that is the same, to the letter, as another is no second column.
What is played is what the tab's sandbox test was run with and had done to
it, so what you tried by hand on one text is tried on the others.

Each text is a column: what it ran, how it stands, what was counted of it
(in the rows a comparison has, in the same order in every column), and
under that its own sandbox, as the Sandbox view draws one, with the
sentence that it was a sandbox test at its foot. Nothing is marked as
better. From a column, its text can be applied, and the default can be
forked where the page offers a fork. **Compare on real work** opens the
drawer that starts a comparison, with each text a way of its own, started
as it was played whatever its file comes to.

#### What a sandbox does not show

A sandbox test shows what the text does with what it is given. It does not
show what the stood-in things would have done: nothing was built, nobody
was asked, nothing was sent, and what those would have said as they worked
is not said. Passing in a sandbox is not passing for real.

### Which runs which

Most of the blueprints that ship are never applied by anybody: they are
*parts*, which a fleet, a board or a rollout runs. Nothing is renamed or
moved to say so, since everything that runs a blueprint finds it by its
name. Which is a part is worked out from the texts.

A blueprint is a part of another where that one names it:

- it attaches, calls or spawns it;
- a param of it defaults to that name (the fleet's `review`);
- the name is written whole as a value anywhere else in its text (the list
  of what a round does, the table of what a row's `does` comes to);
- an action it asks reaches the blueprint by itself, which the action says
  (`reaches`, on the Reference: giving an agent work reaches
  `std/dispatch` and `std/signin`).

A name in a sentence, a comment, an expression or a key names nothing.
What a part runs is a part too, and what nothing names is something
somebody applies.

Three blueprints are applied by people though another also runs them:
`std/plan`, `std/pool` and `std/slot`. The list shows these for now in a
group of their own at the end of `std`, called **misc**, as well as under
what runs them.

So the client's list under **Blueprints** shows, for `std` and for each
package, what somebody applies, and under each, folded, its parts and how
many. The box over the list finds one by its name or by what it says it
is, a part under each thing it is a part of. A part's Summary says what it
is a part of (`mw explain std/cards`: `A part of std/board.`).
`GET /api/blueprints` hands, with each blueprint that is not a project's,
what it names (`runs`).

A blueprint of your own names what it likes and makes nothing a part: a
project's list is what the project uses, as before.

### Playing one to see what it does

What a step was given is not what was written for it. Every `${ }` has
been worked out by then, from params that came from whoever applied the
blueprint, or from the step of another blueprint that called it. To see
what each step was actually handed, play the blueprint in a factory of its
own ([profiles](profiles.md)), started to say so:

```sh
mw up --profile try --handed                  # a factory to one side; nothing in it reaches the one you work in
mw --profile try apply greeter.bp --param who=Grace
mw --profile try runs                         # its runs, the newest first
mw --profile try logs <run>                   # that run, step by step
```

```
greet (ok)
  # handed: {"greeting":"hello, Grace"}
  gave: {"greeting":"hello, Grace"}
child (ok)
  # handed to hello as child: {"said":"hello, Grace","who":"Grace"}
  gave: {"instance":"in_1m7gm1c0h511x8qr"}
```

- A step that does something shows the `with` it was handed, as it came
  out.
- A step that calls or spawns shows which blueprint it started, as what,
  and the params it handed it.
- What the step printed is between that and its outputs (`gave`).

The blueprint that was started has runs of its own (`mw runs --instance
<id>`), which show in the same way what its steps made of what they were
handed.

A factory not started with `--handed` writes none of this: it would be a
line for every step there is. A value longer than a log line holds (16 KB)
is cut. No secret is ever in it: a secret is named under `env`, and is
never what a `with` comes to.

#### Stopping the daemon at a step

To see what a blueprint does when the daemon goes away in the middle of
it, have the daemon stop at a step:

```sh
mw --profile try down --at tell --of hello    # stop as a step called `tell` of `hello` next begins
mw --profile try apply greeter.bp --param who=Grace --no-wait
mw up --profile try --handed                  # once it has stopped: go on from there
mw --profile try logs <run>
```

The journal keeps that step's beginning and nothing after it, said or
written. So the daemon started again finds what a daemon that went at that
instant would have left: the step under way, with nothing of what it did
kept. Whoever was asking the daemon something at that moment (the `apply`
that set it going, if it waits) may get no answer.

Started again:

- the step is taken up (`# attempt 1 (resumed after a restart)` in its
  log, `resumed` in its `step.started`) if it is safe to do twice
  (idempotent) or says `again: true`;
- otherwise it fails, saying it was interrupted;
- an instance of a blueprint that says `durable: false` is ended, so that
  whatever called it can begin it again.

`--of` may be left out; then the daemon stops at the first step of that
name in any instance. A step that never begins stops nothing, and plain
`mw down` stops the daemon as usual.
