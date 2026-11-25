# Who is next when a card waits

A card that waits on somebody should never sit with nobody told and
nobody next. A project's **chain** says who is next: a list of rungs, each
naming a person to tell or an agent to look at the card, and how long the
card may wait there before the next rung is tried too. The fleet keeps it
beside the board, next to `tells` (who is told what waits for a person)
and `segments` (what more is done with a card where it is).

With no rung, nothing more is done than `tells` does: whoever it names is
told once, and nobody is next.

## The kinds of wait

A card waits on somebody in one of these ways. Each is a word a rung's
`waits` may name.

| Kind | When | Said by |
|---|---|---|
| `review` | It came to review and nobody is there to review it (the role has no agents, or fewer than the seats or votes a review needs), or its review ended with nothing said of how it came out. | `card.waits`, from `std/reviewing` |
| `left` | It was left for a person: sent back as often as allowed, nobody decided its review, a merge was in the way, its round was left or failed, a plan was left. | `card.left` |
| `blocked` | Its worker could not do what it needed and ended its turn with the word `blocked`. | `card.left` with `as: blocked` |
| `asked` | Its worker, a plan's round or a feature's planner asked a question. | `card.asked` |
| `held` | Something its column offers to be pressed has not been pressed. | `board.item.held` |
| `plan` | Its plan waits to be accepted. | `card.planned` that says `waits` |
| `idle` | It came to a column no stage works, which an `idle` rung names. | the card's move |

A rung that says no `waits` is for every kind but `idle`. A card in a
column no stage works waits on somebody only where a rung says `idle` and
names that column: a column like that may be a place a person meant to
leave cards in.

## Setting up a chain

The chain is a table of the project's, `chain`, listed under the board's
tables in the board's gear in the client, where its rows are added and
changed as any table's are. From a terminal, a row is added as any record
is:

```
mw record add shop chain title=lead waits=review who=@grace after=4h step=1
mw record add shop chain title=owner waits=review who=applier step=2
mw record add shop chain title=parked waits=idle column=draft who=@grace
```

Each row is one rung:

| Cell | What it says |
|---|---|
| `title` | What the rung is called. |
| `waits` | Which kinds of wait it is for. Every kind but `idle` where it says none. |
| `column`, `labelled` | Held to cards in one column, or with a label. Any card where it says none. |
| `step` | Where it stands among the rest. The lowest is first. |
| `who` | Whom it names: `applier` (whoever applied the fleet), a handle after `@`, or an actor (`user:<id>`, `agent:<name>`). A rung that looks may name one of a role: `role:<role>`. |
| `does` | What it does: `tell` (the usual), `look` or `review`. |
| `told` | The row's own words to an agent that looks or reviews. |
| `after` | How long the card may wait at this rung before the next is tried too, as `4h`. Never, where it says none. |

With the rows above, a card that comes to review with nobody to review it
puts a line in Grace's inbox at once. Four hours later, if the card has
not moved, whoever applied the fleet is told too.

## What a rung does

**`tell`** puts a line in the inbox of whoever `who` names, saying what
the card waits for. A second rung that tells the same person raises the
same line, which says it was told twice.

**`review`** names one agent (`agent:<name>`) and is only for `review`
waits: a `review` rung that names no agent, or says other kinds in
`waits` (or none), is refused. The card's stage reviews it with that agent as the one reviewer
(a council of one). What the agent decides is the stage's to act on as any
review's is, merge and all, and is said as the agent's.

**`look`** names an agent, or one of a role. The agent gets one turn on
the card, in a conversation of its own (`chain:<card>`, counted in the
card's cost). It is told why it is asked, the card, and what was said on
the card lately (`prompts/chain.md`), and ends with one word:

| Word | What becomes of the card |
|---|---|
| `back` | It goes back to the column it was taken from, with the agent's words as what its worker is told. It counts as sent back, as one sent back from review does. |
| `ask` | Whoever applied the fleet is asked, in the agent's words, in their inbox. |
| `up` | The next rung is told now. |
| `stay` | It waits as it is, and the next rung is tried when its time comes. |
| `answer` | Only where the card waits on a question: the agent's words answer it, as `card.answered`, and whoever asked goes on with them. |

**An agent never judges a card it worked.** Who worked it is read from the
card's history. A `look` or `review` rung that names one of them is
passed over for the next rung; a `look` rung that names a role is given to
one of the role that did not work the card, while there is one. Where
there is nobody else, a person is asked instead.

**What an agent decides is credited to it.** The move is made through the
board, and the line of the card's history is the agent's, with the
blueprint after it as detail. It is made only where the agent itself
holds what moving a card takes (`board:write`); otherwise a person is
asked to make it. And it is made only where the card has not moved since
the wait began: a card another stage has taken meanwhile is left where it
is. The agent may also act with `mw` in its turn, as itself; a move it
makes there ends the wait the same way.

## When a wait ends

A wait stands while the card has not been moved since it began and is
the same kind of wait. Once the card is moved the wait has ended, and a
rung's time that comes after that does nothing.

## A worker that could not do what it needed

A worker's turn may end with `blocked`: it needed something it was refused
(a command, a file, the network) and says what and why. The card is then
left for a person with `as: blocked`, and the chain and `tells` can tell it
apart from a round that failed. No words of a report are read to decide
this: the word is the agent's.

A package may also say how its program reports what it was refused (`refused`
in the package's output rules). What a program was refused is said with
the card as a line of its history (`card.worked`) and decides nothing by
itself: the card goes on as the agent said. A stage's row that says
`refused_means: blocked` has such a card left for a person, as one whose
worker ended `blocked`:

```
mw record set shop-stages-1 refused_means=blocked
```

A rung cannot give an agent a scope it lacks. Where the work needs more
than the worker holds (`shell` to run a check, say), the agent ends with
`ask`, and a person decides. A change of a record the agent may not make
is asked for with `mw ask` ([approvals.md](approvals.md)). The chain grants
nothing itself.

## Who may

- Writing a row that names an agent or a role takes what writing each of
  the fleet's `agents` rows takes: the table steers the fleet's agents.
- `who` takes `inbox:write`. A person rung tells as whoever applied the
  fleet, as `tells` does. What a line opens is read as its reader may.
- An agent rung's turn runs with the agent's own key.

## Fleets already running

A running fleet keeps the blueprint texts it was applied with until it is
applied again taking what ships (`--take` of `mw apply`, or **Apply it
again** on the fleet's page in the client, where it is said to be behind). Until then nothing of the chain applies. A board from
before that keeps no `rung` field on its cards has a rung act and tells as
before, but cannot climb to the next rung.

## What it does not do yet

- A rung cannot start a discussion among agents.
- An answer to a question a `look` agent asks does not come back to that
  agent.
- An agent cannot accept a feature's plan; that stays a person's.
- A rung grants nothing (see [approvals.md](approvals.md) for asking).
- Rungs are not counted or measured.

## How it works

**Every decision is a block.** Which rung is next, what it does, the
sentence it says and when the next rung is due are one block
(`chain.next`, in `deciding`, with its table of cases). What an agent's
word comes to for the card as it is now is another (`chain.decided`),
which looks at the card first. The blueprints (`std/chain`, attached by
the fleet as `std/told` is, and `std/chain-wait`, one call per wait) only
hear and do what the blocks say.

**Nothing runs for a card nobody waits on.** With no rung, `std/chain`
runs nothing for any card. Where a card is on the chain is kept on its own
row (`rung`, a quiet field of the board: the kind, the count of the card's
moves it began at, and how many rungs it has reached). Nothing else is
kept per card, and nothing is started for a card while no rung has a time.

**A wait goes by the card's moves, never by its column.** A wait stands
while the card's move count is the one it began at. A card moved and put
back is a new wait.

**Time is waited out by `std/later`**, begun only for a rung that says
`after`, which waits on the daemon's clock and then says `later.over`. Its
wait is kept across a restart. The rung's time that comes for a card that
moved does nothing.

**An emptied `tells` stays empty.** The chain adds no rung of its own: a
project that emptied `tells` and has no chain tells nobody, as it chose.

**Stopped anywhere, a climb comes to the same.** `std/chain-wait` keeps
where it is as it goes, so a daemon stopped in the middle of it takes it up
where it was: whoever a rung names is told once, an agent that looks is
given the turn it had, and a card is moved once. The simulation holds this
by stopping a climb at every place.
