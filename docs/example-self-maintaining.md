<!-- public -->

# Example: a repository that looks after itself overnight

Say your team keeps a service called `shop`, and every week somebody spends
a morning on the same chores: dependencies that have new versions, a test
that started failing, a warning that crept in. You would rather find those
done, or waiting for one decision, when you come in. This is how Mythwright
does that: a check of your own runs at two in the morning, each thing it
finds becomes a card, agents work the cards and a council of agents reviews
them, what passes is merged, and in the morning the project's channel says
what happened and your inbox holds only what needs you.

Each part is a blueprint or a row of a table, and you can read and change
every one of them.

## What is yours, and what is the factory's

**Yours**: the check itself. Mythwright does not know how your project is
built, which dependencies it has or how its tests are run. You write one
command that looks, and prints one line for each thing it found, such as:

```
Update serde to 1.0.215
Test checkout::totals fails since yesterday
Warning in src/cart.rs: unused variable `rest`
```

It can be a script that runs your package manager's "what is outdated"
and your test suite, and prints what it found in those words. Each line
becomes a card's title, so write it as the work to be done. Mythwright
keeps the last 40 lines a command prints, so one night's findings are 40
at most.

**The factory's**: running it on time, turning lines into cards without
adding one twice, working and reviewing the cards, merging, and telling
you.

## 1. The project, with pipelines

```sh
mw project create shop --path ~/src/shop --with pipelines
```

This gives the project its board, its fleet (an architect, a worker, and a
row of reviewers with nobody in it yet), its library, and the table of what
runs its pipelines, `triggers`. Put your check in the repository, for
example as `scripts/nightly-check`.

## 2. The pipeline

A project's pipelines are its own blueprints under `pipelines/`. This one
runs your check, has a card added for each line, and says in the project's
channel what it found:

```yaml
blueprint: pipelines/nightly
description: Run the project's own nightly check and add a card for each thing it found.
permissions: [shell, channels:write, board:write, records:read, blueprints:run]

steps:
  check:
    description: Run the check, which prints one thing it found on each line.
    do: shell
    with: { cmd: [scripts/nightly-check] }

  found:
    description: Add a card for each line, where the board has none of that title yet.
    when: ${ trim(steps.check.out.stdout) != "" }
    for_each: ${ split(trim(steps.check.out.stdout), "\n") }
    call: nightly-found
    as: "found-${index}"
    with: { title: "${item}" }

  channel:
    do: channel.ensure
    with: { name: "${instance.project}", project: "${instance.project}" }

  said:
    description: Say in the project's channel what the check found.
    do: channel.post
    with:
      channel: "${steps.channel.out.id}"
      body: "Nightly check: ${ steps.found.count ?? 0 } found. Cards labelled nightly are on the board."
```

And the small blueprint it calls for each line, which looks for a card of
that title first, so that a thing found again tomorrow is not a second
card:

```yaml
blueprint: nightly-found
description: Add one thing the nightly check found to the board, unless a card is called that already.
permissions: [board:write, records:read]

params:
  title: { type: string, description: "What was found, one line." }

steps:
  board:
    description: Look for a card called that.
    do: rows.read
    with: { collection: board, saying: { title: "${params.title}" } }

  add:
    description: Add it, ready to be worked.
    when: ${ steps.board.out.those[0] == null }
    do: board.add
    with: { title: "${params.title}", status: ready, labels: [nightly] }
```

Save each as a project's own blueprint, from the client (the project's
Blueprints, in the side panel) or from a terminal:

```sh
mw check nightly.bp
mw file write shop @blueprints/pipelines/nightly.bp --from nightly.bp
mw file write shop @blueprints/nightly-found.bp --from nightly-found.bp
```

`mw sandbox pipelines/nightly --project shop --first std/board --tell shell
code=0 stdout="..."` plays it in a factory of its own with your check stood
in, so you can see the cards it would add before it touches the board.

## 3. Run it at two in the morning

A row of `triggers` says when. A time of day is written with its zone, so
it means the same through summer time:

```sh
mw record add shop triggers title=nightly pipeline=pipelines/nightly on=at at="02:00 Europe/Berlin"
```

To try it now, add a row that runs it by hand, and ask for it:

```sh
mw record add shop triggers title=nightly-now pipeline=pipelines/nightly on=hand
mw emit pipeline.asked project=shop pipeline=pipelines/nightly
```

The run is listed with the others under the project's **Pipelines**, with
what each step printed.

## 4. Agents work the cards, and a council reviews them

Each card the check added is in `ready`, and the fleet's worker takes it,
works it on a branch of its own and puts it in `review`. Give the
reviewers a count, and the review stage a council: three look at each card
and two have to approve.

```sh
mw record list shop agents
mw record set shop-agents-3 count=3
mw record set shop-stages-2 seats=3 needed=2 reports=verdicts
```

(`mw record list shop stages` says which row is the review stage's.) A card
the council passes is merged into the project's branch and moved to `done`;
one it fails goes back to the worker that did it, with what those against
it said, as often as the stage's `rounds` allows (two, as it starts). One
whose branch no longer merges goes back to its worker too. `reports=verdicts`
has the stage say in the channel what each review came to and who looked.

The agents need a real program to do any of this: out of the box they run
on a small built-in assistant that cannot work a card
([extensions.md](extensions.md) says how to add one). Each works behind a
wall that shows it the project and little else of the machine.

## 5. What you wake up to

**In the channel**, in order: what the check found, what each review came
to and who looked, and anything of the fleet that failed. Each card's
history says the rest: who worked it, each round, and the merge.

**In your inbox**, only what waits for you, each line opening the card:

- a card **left for a person**: sent back as often as it may be, a council
  that could not decide, or an agent that could not do what it needed;
- a **question** an agent asked in place of working, said on the card and
  naming you;
- and, if you want a say before anything is worked, a **go-ahead**. Add a
  button that holds the cards in `ready` until a person presses it:

  ```sh
  mw record add shop actions title="Go ahead" column=ready who=person holds=true
  mw board press shop-14 go-ahead
  ```

  The cards the check added overnight then wait for you in the morning,
  each a line of your inbox, and are worked once you press it. Leave the
  button out and they are worked overnight.

Who is told what is a table too, `tells`: it starts by telling whoever
applied the fleet, and a row names somebody else and which things they are
told (`mw record add shop tells title=lead whom=@grace of=left,failed`).

## What it does not do

- It does not write your check. What is outdated, what failed and what is
  worth a card is your command's to say.
- It merges what the council passes. If nothing is to be merged without a
  person, take the reviewers out (`count=0`): each card then waits in
  review for one of you.
- A card is matched by its title. A finding whose line changes from night
  to night (a version number that moves on) is a new card each time it
  changes: write the line so that it names the thing, and leave out what
  moves.
