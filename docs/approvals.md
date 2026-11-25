# Asking for a change you may not make

An agent often needs a change it does not hold the scope for: a cell of a
table that takes `shell`, a row of a table that takes `admin`. Giving the
agent that scope would let it make every such change from then on.
Instead it **asks**. Nothing changes. A person who could make the change
is told in their inbox, sees exactly what would change, and approves or
declines it. Approved, the change is made as the agent, once, and the
record's history says who approved it.

A run of a blueprint can ask the same way: a step waits until a person
lets the run go on (*A run that waits for a go-ahead*, below).

Two rules hold throughout:

- **An agent never acts on behalf of a person.** An approved change is
  made as the agent who asked, and credited to it. The approver lends
  their authority to that one write and to nothing else.
- **Asking never widens what the asker holds.** Its credential, its slot
  and its scopes stay as they were before, during and after an approval.

## Asking

```
mw ask <record> <field=value>... [--to WHO] [--why WORDS] [--channel C [--thread M]]
```

For example, a worker that needs a command run after its work, which only
somebody holding `shell` may set:

```
mw ask shop-segments-1 command="cargo test" --why "the card's tests are not run before review"
```

The daemon first works out what the change takes, as the asker, without
writing anything. Then one of three things happens:

| What the asker lacks | What happens |
|---|---|
| Nothing | The change is made there and then, as any change by the asker is. No ask is kept. |
| Only scopes (a cell that takes `shell`, a table that takes `admin`) | An ask is kept, unmade, and people who could make it are told. |
| Anything else (a value that does not fit, a record that is not there, a table kept from the asker) | It is refused, in the same sentence the change itself would be refused with. Nothing is kept. |

Rules for an ask:

- **Whom it is told to.** The person `--to` names (an id, a name or an
  email), where they could make the change as things stand. Otherwise
  everybody with an account who could make it and may read the record.
  Where nobody could, the ask is refused and nothing is kept.
- **Who may ask.** A person or an agent who may read the record. Asking
  takes no scope of its own.
- **How many.** An asker has at most 20 asks waiting at once.
- **How long.** An ask waits seven days. After that it has expired and
  nothing can approve it.
- **Not its own row.** The row an agent is brought up by (its row of a
  fleet's `agents`, say) is never asked for: that is a person's to change.
  An agent is told so in what it is told of itself.
- **Asked twice.** The same asker asking the same change of the same
  record again (a turn begun again after a restart) gets the ask that is
  already waiting.

`--channel` and `--thread` say where you want to be told what came of it.
An agent asking from a turn in a channel needs neither (*How the asker is
told*, below).

## Deciding

### In the client

The ask is a line in the inbox of each person it was told to, from the
agent: `shop-worker-1 asks to change shop-segments-1 (segments): command`.
Its page shows a table with one row per cell, headed **Cell**, **Was**,
**To be** and **Takes**: what the cell held when it was asked, what it
holds now where that is different, what it is to hold, and each scope it
takes with whether you hold it. Under it are why the agent asked, in its
words, and how the ask stands.

**Approve** and **Decline** are offered only where the daemon says you
may. Decline asks **Why not (they are told)**, which you may leave empty,
and **Decline it** sends it. Whoever may not read the record is shown
that it is kept from them and nothing of it.

Once one person has decided, the others' lines read as decided, by whom,
with nothing to press.

### From a terminal

```
mw asked                     what you asked for, and what waits on you to decide
mw asked <ask>               one ask
mw approve <ask>
mw decline <ask> --why "count it per role instead"
```

`mw asked` prints each cell from what it held to what it is to hold, with
what it takes and whether you hold it.

### Who may approve

- A person. Never an agent, a device or the system, and never the asker.
- Somebody who passes every check of the change as things stand, against
  the record as it is now: so nobody gives by approving what they could
  not do themselves, and a scope taken from them since they were told is
  not given.
- A key may approve, within what it was given. The event then names the
  key (`key:<id>`).

### What approving does

Approve is the change. In one write:

1. The ask is read, and refused unless it is still waiting.
2. The record is read as it is now. If it changed in any way since the
   ask was made, the ask is **stale** and nothing is made.
3. The cells the ask names are set to what it says. Nothing else is
   touched.
4. The record and the ask (now `approved`, with who and when) are written,
   and both events are said.

The change is said as the asker's: `record.updated` with the agent as its
actor, and `approved_by` and `asked` in its data. The record's history
line for the change ends with who approved it, by their name as it is
now: `… · approved by Ada`.

Anything that later asks what the actor of that change holds (a blueprint
acting for whoever changed a record) is answered for the agent, with what
the agent holds.

### Decided once

- Approve or Decline pressed again, by the same person or someone else,
  after a double press, a retry or a restart, answers what the ask came
  to and changes nothing.
- A stale or expired ask makes nothing, and is asked again if the change
  is still wanted.
- **An asker that cannot act** (an agent that was stopped, one that was
  taken away, a key that was withdrawn) cannot have its change approved:
  Approve is refused, saying to start it again or to decline. Decline
  still works.
- **An agent taken away** has every ask of its that waits withdrawn by the
  factory, saying why. Nobody is told back, and the people who were told
  read it as withdrawn.

### Cards

A card is a record and is asked for the same way. Its cells are changed
through the board's own write, so the board's history and its stages hear
the change, and `board.item.updated` says who approved it. Two things
differ:

- A card's column is never asked for. Moving a card is not a change of a
  cell, and asking for one is refused as it is for anybody.
- Whom a card is given to goes by the board's own check of whom a card may
  be given to.

## How the asker is told

| Where it asked from | How it is told what came of it |
|---|---|
| An agent in a turn in a channel | The factory's bot says it in that thread, naming the agent (`@shop-worker-1 Ada approved your change of shop-segments-1, and it is made.`), which gives the agent a turn. Only while the agent may still write there; otherwise in its inbox. |
| A person, or an agent with no turn in hand | The bot says it in the channel and thread `--channel` and `--thread` named, where given and while they may write there; otherwise a line in their inbox. |
| An agent the factory does not start | A line in its inbox. |

Stale and expired asks are told the same way, saying to ask again.

## A run that waits for a go-ahead

A step of `approval.ask` makes an ask of its own kind and waits until a
person decides it:

```yaml
blueprint: deploy-site
description: Build the site, and put it live once somebody lets it.
permissions: [shell]

steps:
  build:
    do: shell
    with: { cmd: [make, site] }

  approve:
    needs: [build]
    do: approval.ask
    with: { says: The site is built and waits to go live. }

  live:
    needs: [approve]
    do: shell
    with: { cmd: [make, live] }
```

| Argument | What it says |
|---|---|
| `says` | What is to go on, in the blueprint's words. Required. |
| `to` | Whom to tell, of those who may decide: people by id, name or email. Where it names none of them, all of them are told. |
| `environment` | The environment what follows goes under, whose rule says who approves. The run's own where it names none. |
| `links` | Addresses to look at before deciding, each `http` or `https`. |

**Who may decide:**

- a person, never an agent;
- holding `blueprints:run`;
- where the environment says who approves what goes under it, one of
  them. That list is kept with the environment:
  `mw project environment shop prod approve grace ada`;
- whoever started the run only where that list names them. With no list,
  somebody else lets it go on.

Each person who may decide is told in their inbox. The line's page shows
what is to go on, the links, how long it waits, and **Approve** and
**Decline**; Decline asks **Why not (the run fails saying it)**. The run's
own page shows the same buttons where its stage waits. From a terminal it
is listed by `mw asked` and decided with `mw approve` and `mw decline`.

**What it comes to:**

- Approved, the step gives `approved_by` (an actor), `approved_by_name`
  and `ask`, and the run goes on.
- Declined, the step fails with why. `on_fail` and `allow_failure` take it
  as any failure.
- It waits as long as its step's `timeout` says, and seven days where it
  says none. Past that it has expired wherever it is read, and nobody can
  approve it.
- A go-ahead is decided, never withdrawn: to call it off, stop the run.

The step is the same done twice: done again after a stop, it finds its ask
and goes on waiting or answers what came of it.

An environment's rules are held wherever something comes to run under
it, and are shown in its drawer in the client: *Rules* in
[environments.md](environments.md).

## Events and history

Each event of a change is said in the write that keeps the ask, with
`project`, and is heard as the record's own events are: a table kept
private keeps its asks to whoever may read it. None carries the cells'
values; those are read through the line's page, by whoever may.

| Event | Said when | Carries |
|---|---|---|
| `record.asked` | an ask is kept | `ask`, `id`, `project`, `collection`, `cells`, `asker`, `why` |
| `record.ask.approved` | in the write that makes the change | the same, and `by` |
| `record.ask.declined` | it is declined | the same, `by` and `why` |
| `record.ask.decided` | it came to anything: approved, declined, stale, expired or withdrawn | the same, `by` (`system` for stale or expired), `status`, and `why` where declined |
| `project.approval.asked` | a run waits for a go-ahead | `ask`, `run`, `step`, `instance`, `blueprint`, `environment`, `asker`, `project` |
| `project.approval.decided` | a go-ahead is decided | the same, with the decision |

The go-ahead events are heard by whoever may read blueprints.

A record's history keeps an approved change as the change's own line,
ending `· approved by <name>`. An ask and a decline are lines of the
history only where a table's history counts their topic. The board and
the fleet's tables do:

```yaml
history:
  record.asked: "{actor} asked to change its {cells}[: {why}]"
  record.ask.declined: "{actor} declined a change of it[: {why}]"
```

A table whose blueprint says nothing of them keeps no line of them.

## What it does not do yet

- **Only a record's cells, and a run's go-ahead, can be asked for.** A
  card's move, a card given to somebody and a blueprint applied cannot
  be asked for yet. Each would be a kind of its own, asked for by name.
- **No blueprint step asks for a change of a record.** An agent asks with
  `mw ask`. (The action called `record.ask` is something else: it asks a
  question on a record.)
- **A table that keeps its records outside the factory's database** (as
  files, through a store service, or as a library's documents) cannot
  have a change approved: Approve is refused, saying so.
- **Withdrawing** an ask is done by the asker through the API
  (`POST /api/asked/<ask>/withdraw`). There is no `mw` command or client
  button for it yet.

## How it works

**There is no standing grant.** Approving gives, checks and spends the
grant in one call, inside the daemon. Nothing is handed to the agent: no
token, no scope, nothing in its session or its slot. There is nothing it
could carry to another change, and nothing that could be spent twice or
never.

**The change is fixed before anybody is asked.** The ask keeps the
record's id, each cell's value before and after, and when the record was
last changed. Approve writes those cells to those values and nothing else,
and refuses where the record moved meanwhile. What the approver saw is
what is written. Any change of the record makes it stale, not only a
change of the asked cells: a cell beside them can change what they give
(a row's `holds`, say).

**An ask lives beside its record.** It is a row of a table `asked`, kept
in the same database as the record it is about: the project's own file,
or the factory's for a project from before projects had files of their
own. So the write that changes the record is the write that spends the
grant. Kept apart, two writes could leave a change made with its grant
unspent, or a grant spent with no change made. That is also why a table
kept outside the database cannot have a change approved.

**Approve races are settled by the database.** The write checks that the
ask is still waiting in its own statement, so two Approves at once give
one change and one no-op, even where neither saw the other.

**What is told after the write is owed.** The inbox lines and what is said
back to the asker are kept in the factory's file, not the project's, so
they are told after the write. The ask's row says it owes them until they
are done, and a daemon that starts tells what is owed. Each telling is
the same done twice, so nothing is told twice after a stop. The
simulation stops an ask, an approval and a decline at every place and
holds that the change is made once and the asker told once.

**The approver's authority is used for the checks only.** It never
becomes the actor of anything else: no blueprint runs as the approver,
nothing is said as them, no turn is given.

**Limits are worked out, not timed.** Twenty waiting, and seven days, are
worked out where an ask is read or acted on, from when it was asked. No
timer runs for them.

**Whom to tell is not a channel's people.** A record belongs to no
channel, and asking which channel an agent was in to decide who may change
a table would make a channel a gate it is not. So an ask goes to everybody
who could make the change, or to one of them the asker names.

**Each kind is its own door.** A kind of ask (`kernel::approvals::Kind`:
a change of cells, a go-ahead) has its own dry run and its own one-write
door. Nothing can be asked for that has none.
