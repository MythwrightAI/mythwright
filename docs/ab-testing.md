# A/B testing a card

An A/B test has one card worked several ways at once, and judged. Two
agents on different models, say, each work the same card in a worktree of
their own; a reviewer agent, or a council of them, looks at what each did
and picks one. The winner's work becomes the card's and goes on to the
usual review. The losers' work is kept as branches, or thrown away. What
each contender spent, and what the judges said, is counted under its
name, so the same pairing can be added up over many cards.

A test is set up in the card's **A/B testing** tab, a form made from what
the test's table declares. Everything a test does is blueprints; what it
decides is blocks with tables of cases; and it counts, never scores.

The same tooling underneath (*comparisons*: one blueprint started several
ways, counted side by side) can also be used by itself, over many cards
at once, and can roll a new setup out gradually. That is the second half
of this page.

## Turning A/B testing on for a project

A project needs its board and its fleet. In the client, open the board's
page, press its gear and turn on **A/B testing**. From a terminal:

```
mw project create shop --with ab
mw apply std/ab --project shop --param project=shop
```

The first makes a new project with it; the second gives it to a project
that is there. Either way `std/ab` declares two tables beside the board:

| Table | Called | What a row is |
|---|---|---|
| `contests` | A/B tests | One test of a card, set up by a person |
| `entries` | A/B attempts | What one contender of a test did, written by the test and never by hand |

## Setting up a test

### In the client

Open a card. Its form has a tab **A/B testing**, which lists the card's
tests, each with what it says and the attempts under it. **+ New test**
adds a test about the card and opens it whole in the drawer. Fill in at
least the contenders, then make the card ready as usual.

### From a terminal

```
mw record add shop contests card=shop-12 contenders=otter,slate
mw board move shop-12 ready
```

### What a test says

| Field | What it says | Where it says nothing |
|---|---|---|
| `card` | The card. | (needed) |
| `contenders` | Two or more, as many as you want. Each is a role of the fleet (`otter`: the role's row of `agents` says its model), an agent by name (`@shop-slate-1`), or a way of the comparison named in `compared`. | (needed) |
| `compared` | The comparison it is counted under. | One of its own, `ab-<test id>` |
| `judging` | The blueprint that judges: `std/judge`, `std/judge-scores`, a fork of one, or the project's own. | `std/judge` |
| `judges` | A role, or an agent as `@` its name. | The `reviewer` role |
| `seats` | How many judge. 1 is one judge picking. | 1 |
| `needed` | How many judges must agree on one entry. | 0: the one picked most, alone |
| `council` | How several judges come to one answer: `votes` (each for itself) or `talks` (they talk it over). | `votes` |
| `stretches` | How many stretches a council that talks may take. | 3 |
| `told` | Your own words to the judges. | |
| `then` | What scores come to: `person` (a person picks, the scores beside them), `highest` (the highest mean wins) or `most` (the entry most judges scored above every other wins). | `person` |
| `scale` | The highest score there is. | 10 |
| `known_by` | How judges know the entries: `letters` (blind) or `ways` (by name). | `letters` |
| `losers` | What becomes of the work that did not win: `branch` (kept) or `discard`. | `branch` |
| `alone` | What a test with one contender left comes to (the others failed or kept nothing): `person` or `take`. | `person` |
| `undecided` | What a judging that decides nothing comes to: `person` (the card asks which) or `left`. | `person` |
| `shadow`, `challengers` | A shadow test (*Shadow tests*, below). | |

The test writes `state` (`waiting`, `working`, `judging`, `asking`,
`came`, `left`), `winner`, `agent` and `why` itself.

Rules the test is held to before anything runs:

- at least two contenders, and none written twice;
- each a role the fleet has, an agent that is there, or a way of the named
  comparison;
- the judges are not all agents that work an entry: nobody judges their
  own work;
- setting up a test takes `agents:write`, and holding what each row of
  the fleet's `agents` gives: a test chooses what the fleet's agents are
  given, run as whoever applied the fleet (`steers: agents`).

A card may have several tests. They are done one after another, the
oldest first, one round each.

## What a test does

1. While a test of a card is to be done, `std/ab` lays a row into the
   project's `segments`, held to that card, at order 10, naming
   `std/contest`. So the card's next round does the test where it would
   have done the usual work (`std/work`), and keeping, sending, the review
   and the merge come after it unchanged.
2. Each contender works the card at the same time, in a place of its own
   cut from the card's own work so far: `.lanes/shop-12--a` on the branch
   `lane/shop-12--a` (by letter where judging is blind, by its name where
   it is not). Each is a turn of the contender's agent, in a conversation
   of its own (`contend:shop-12:<way>`).
3. Behind a wall, a contender is not shown the other contenders' places.
   Where no wall holds (an agent with `--no-wall`, a factory started with
   `--walls off`, Windows and macOS) it could read them: its attempt is
   marked **not held apart**, judges are told so beside its work, and the
   test's history line names it. Nothing is decided by it.
4. The judging named is called with each attempt (*Judging*, below), and
   each attempt's verdict (`won`, `lost`, `undecided`) and what the judges
   said is written on its row.
5. The winner's place becomes the card's: its worktree becomes
   `.lanes/shop-12` on `lane/shop-12`, so everything after it goes by the
   card as usual. The losers' branches are kept, or deleted where the test
   says `discard`.
6. The card is given to the winner's agent, and from then on its rounds
   are that agent's: a card sent back from review is worked again by the
   winner, told what the reviewer said.
7. The card's history says it in one line, such as `its A/B test of otter
   and slate was judged by shop-reviewer-1: otter`, then the review's
   lines as usual.

A project that is no git repository has no test done in it: its
contenders would work over one another in one folder.

## Judging

### One judge, or a council that votes

`std/judge` with `council: votes`: `seats` judges, each in a turn of its
own (`judge:<card>:<seat>`), side by side. Each is shown the same thing
and ends with the letter of the entry it picks. `seats: 1` is one agent
picking. What the picks come to, with `needed`, is a winner or
undecided.

### Scores

`std/judge-scores`: each judge scores every entry out of `scale`, ending
with a line an entry (`A: 7`). The scores are kept as said, a judge's
word each, on each attempt, and counted. What they come to is `then`. A
tie always goes to a person, whatever the rule: no rule picks between two
that came out alike.

### A council that talks

`std/judge` with `council: talks`: the seats talk it over a stretch at a
time. In each stretch every seat is shown the entries and what the
council said so far, says what it thinks, and ends with the letter it
would take, or nothing while it is not settled. A stretch where `needed`
seats (every seat, where it says none) would take one entry decides it;
otherwise another stretch, up to `stretches`, after which nothing is
decided. The talk is the test's own and is said in no channel.

### What a judge is shown

The card (title, body, plan), the test's `told`, and for each attempt, by
its letter (or its way, where judging is not blind): what its agent
answered, what it changed, a patch of the change against the card's work,
and its place to read more. Letters are given in an order turned by the
card, so no contender is always `A`; past `Z` they go on `AA`, `AB`. No
judge is an agent that worked an entry.

### When a person decides

Where judging comes to nothing and the test says `undecided: person`, or
scores are left to a person, the card asks which entry to take. The
question says what each entry is in words you can act on from the card
alone: its letter, the way that made it, its agent, its branch, how much
it changed, and why others were not kept. Answer it as any question on a
card, in its **Comments** or from a terminal:

```
mw record comment shop-12 B
```

Where one entry's work was kept, answer `y` (or `take`, `ok`) to take it,
or `n` (or `leave`) to leave the card for a person. Where several were,
answer with a letter, or no.

### What a test comes to

| Judging came to | The card |
|---|---|
| A winner | The winner's work is the card's and goes on (`on`). |
| Undecided, `undecided: person` | Asks which (`asks`). |
| Undecided, `undecided: left` | Left for a person, saying why (`left`). |
| One attempt left, `alone: person` | Left for a person, saying which failed. |
| One attempt left, `alone: take` | That one is taken, with nothing judged. |
| Every attempt failed | Left for a person. |

## Shadow tests

A shadow test has a challenger work the same card beside the usual setup,
and only judges or counts it: the card's work is always the
**incumbent's**.

```
mw record add shop contests card=shop-12 contenders=slate shadow=worker
```

- `shadow` names the incumbent, written as a contender is. It is one of
  the contenders, put first where `contenders` leaves it out, and its
  attempt is marked `incumbent`.
- `challengers` says what the others are: `judged` (the usual: judged
  beside the incumbent, every verdict counted) or `counted` (no judge is
  asked; what each spent and took is what tells them apart).
- The incumbent's place is taken whatever the judges said, and nobody is
  asked: a judging that decided nothing leaves each attempt `undecided`,
  and a person's answer takes nothing. Each challenger's branch is kept or
  thrown away as `losers` says.
- An incumbent whose work could not be kept leaves the test for a person,
  with every branch kept. No challenger is taken in its place.
- The test's `winner` is the incumbent, and the card is counted as the
  incumbent's. What the judges took is each attempt's verdict, the test's
  `why`, and the history line: `its shadow test of otter and slate was
  judged by shop-judge-1: slate; its incumbent, otter, goes on`.
- Judges are not told which entry is the incumbent.

## Reading the results

[A real A/B run](ab-run-opus-sol.md) shows all of what follows read
back from one factory: five cards, two models, judges' scores and costs.

**On the card.** The A/B testing tab shows each test with its state and
winner, and under it each attempt: its contender, agent, state, verdict,
what the judges said, **not held apart** where it was so, and its branch,
which opens to its last commit.

**What it cost.** The card's cost, in its drawer or with
`mw board cost shop-12`, lists a round's turns by contender and judge.

**Side by side.** Each test is counted under a comparison, its own
(`ab-<test id>`) unless it names one in `compared`:

```
mw compare show ab-shop-contests-1
```

shows a column for each contender: what came of it (`judged won`,
`judged lost`, each score), what each turn chose, what it spent (tokens
and money), how long it took, how many of its cards were accepted, and
what it spent per accepted card.

**Over many cards.** A test that names a shared comparison
(`compared=models`) adds its card to it, so `mw compare show models` is
the same contenders side by side over every card they met. One test is
one card; a shared comparison is what A/B testing is for.

**After the test.** The card is counted as the winner's (`compare.went`),
so its review passing, and work on it after review, are counted under the
winner's way. A test's own comparison goes on counting until the card's
review passes, then ends; one taken away ends then. And once the card is
merged, what comes of its work later is counted under its way too (next
section).

## Comparisons

A test is built on comparisons, which you can also use directly: one
blueprint, started several ways in one project, with everything each way
does counted apart. From a terminal:

```
mw compare start models --project shop --of std/lane-round --per item \
    --way one --with one.role=first --way two --with two.role=second
mw compare show models
mw compare end models
```

From the client, open a blueprint's page, press **Testing**, and choose
**Compare on real work**.

- From when it is started, every start of `--of` in the project goes one
  of the ways. With `--per item` each thing (a card) goes one way and
  keeps to it; the way with least of its share is given the next new
  thing.
- A way may hand other params (`--with`), start another blueprint or a
  version in its place (`--way b=my-round`, `--way before=std/lane-round@3`),
  or read other words where a prompt is linked (`--prompt`).
- `--asked` makes the ways named by the start: only a start that says
  `compared: <name>` and `way: <way>` is its, and goes that way whatever
  the shares. Any number of these may be in force for one blueprint. A
  test's comparison is one.
- What is counted is events about each thing, with how long after it was
  given its way (`--count`). `--count cards` is the usual one for a
  board's cards. `show` then says how many of each way's things were
  accepted, over how many it was given, and what it spent per accepted
  one (what was spent on work not accepted is in that).
- It counts; which way is better is yours to say. Nothing marks a column
  as better.

### Counts after merge

What came of a card's work after it was merged is counted under its way:

- `card.reverted`: a later merge reverts its commit.
- `card.changed_again`: a later merge, within `within` days (14, a param
  of `std/merge`), changes the same files.
- `card.fixed`: a later card says it fixes it, in its `fixes` field.

```
mw record set shop-31 fixes=shop-12
```

Each is said at the later card's merge, and counted once for each card
that did it. `compare show` lays them out as `after reverted`,
`after changed_again` and `after fixed`. A revert changes the same files
again, so it is counted as both, on purpose.

### Gradual rollout

A comparison that shares its things out can have its shares said again,
by hand or by a rule:

```
mw compare share models slate=3 --why "it held for a week"
mw compare rule models --grows slate --against usual --holds verdict=failed --over 20 --step 10 --most 50 --least 10
mw compare rule models --none
```

The rule above reads: once `slate` and `usual` have each had 20 things
whose `verdict` was counted since the shares were last set, `slate` is
given 10 more of every hundred, up to 50, where its things came to
`failed` no more often than `usual`'s (`--at-least`: at least as often,
for what is wanted). Where they did not, its share stays, or with
`--least 10` goes back a step, to 10 at least. Either way what comes next
is read afresh.

- A share is of what comes next: a way raised to half is given half of
  the next things, not everything until it catches up.
- The rule is checked when it is given: its ways and count are the
  comparison's, `--most` leaves every other way one at least, and the
  comparison shares things out and is going.
- `show` says each way's part of a hundred, the rule in a sentence with
  what it has read, and each time the shares were said again and why.
- In the client, the Compare drawer has **Its shares grow by a rule**
  under the ways, and a comparison's view under Measurements has
  **Give it a rule…** or **Change its rule…** for whoever may.

## What it does not do yet

- **Benchmarks of finished cards**, replayed by each contender from the
  commit each began at. Not built.
- **Prompt and blueprint tests set up in the client**: a contender can
  already be a way of a shared comparison that starts a fork, a version
  or other words, but the client has no form for making such ways for a
  test.
- **Two judgings of the same entries** compared (reviewer A/B). Not built.
- **Environments as ways**, two versions deployed to two environments
  under the same work. Not built: it needs the runner to start a child
  under another environment than its parent's.
- A card tested twice under one shared comparison is counted as the first
  winner's.
- The winner's work after review is under its way only while no other
  comparison that shares its things out is in force for `std/work` in the
  project.
- A board rearranged after a comparison starts is counted by the last
  column it had then.
- A test whose card is never approved keeps its own comparison counting
  until somebody takes the test away or ends it.
- A gradual rollout reads a thing only once its count says something of
  it; a rule that has taken its way to the most is done and does not fall
  back from there; and the shares are said again by hand only from a
  terminal or a blueprint, not from the client.
- Counts after merge see nothing that comes to the project's branch
  without a card's merge, nor merges a program from before made, nor a
  `fixes` changed after the card was merged; of more than a thousand
  merges in the window, only the newest thousand are looked at.
- The factory's own tests try up to about ten contenders. Many more (the
  pool, what a judge can read, the client's tab) has not been tried.

## How it works

**A contender is a way of a comparison.** The comparison tooling already
knows how to hand a start other params, a fork, a version or other words,
and labels everything started a way (`compared`, `way`, `text`) so that
runs, turns and measurements are told apart. A test's comparison is an
*asked* one: the test names each contender's way at its start, so the
card, not the shares, decides who works it.

**The test is laid in, not added on.** A test is one row of `segments`
held to the card, so the card's round does the test where it would have
done the work, and nothing after it changes. The card itself stays a card:
that row and its assignee are all that change.

**What runs is blueprints; what decides is blocks.** `std/ab` declares the
tables and lays the row; `std/contest` finds the card's next test;
`std/contended` does it (and is what anything else that wants a card tried
several ways can call); `std/entry` is one contender; `std/judge`,
`std/judge-seat`, `std/judge-talk` and `std/judge-scores` judge. Which test
is next, contenders to ways and letters, picks, scores, a talking
council's stretch, what a judge is shown and what it all comes to are
blocks (`contest.*` in `deciding`), each with its table of cases.

**Any number at once.** Each contender, and each judge of a council, is
spawned, and all are waited for together by the language's `wait` with
`ended` (the keys of children of the instance), which gives what each
gave. So nothing fixes how many contenders there are.

**The daemon gained only general actions.** A place cut from another
item's branch (`from` of `place.make`), a place taken by another item
(`place.take`), a branch thrown away (`discard` of `place.clear`), a patch
of a place (`patch` of `place.changed`), folders a turn is not shown and
agents that may not take it (`hidden`, `not` of `agent.task`). None names
a card or a test.

**Stopped anywhere, a test comes to the same.** Everything a test does is
the same done twice: the comparison, the rows, the places (each half-way
state of `place.take` is found and finished), each agent's turn by its
asking, the verdicts (written again with the same values is no write, so
nothing is counted twice), and the history line, said only with the write
that changed the test's `came`. A test taken up again starts no contender
whose attempt says its work is over. The simulation stops a test at every
place and holds that each agent is asked once a turn, one winner's work is
merged, and the loser's branch is kept.

**The card is counted as one thing of one way.** The comparison's things
are the attempts, so the card is written down as the winner's way's
(`compare.went`) and what is said of it later (approved, reverted, fixed)
goes by the card. Were the card a thing of every attempt, its contenders
could not each have their own way.

**It counts and never scores.** Each attempt's verdict and scores are
counted under its way; what each contender spent and took is measured by
the labels the comparison gives. Where judges give scores and nobody said
how a winner is had from them, a person picks.
