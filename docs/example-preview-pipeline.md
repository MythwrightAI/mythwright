<!-- public -->

# Example: every branch tried before it lands

Say a branch holds a handful of cards your agents finished: a fix, a new
page, a change to a query. Before any of it reaches `main`, you want to
use it: the branch built and running somewhere of its own, the cards it
holds in front of whoever tries it, each passed or failed where they read
about it, and a record of every time that was done. This is how
Mythwright's own work is tried before it lands, and every part of it is a
blueprint or a row of a table that you can read and change.

The shape:

1. A branch is pushed (by you, or by the agents' merge of their cards).
2. The project's `dev` environment is **run** for that branch. Its
   blueprint builds the branch and starts what was built at an address of
   its own.
3. The cards the branch holds move to a `staging` column and are given to
   whoever tries the preview.
4. That person is told in their inbox: the address, and the cards, each
   with **Pass** and **Fail** beside it. A card failed goes back to
   `ready` with the reason, which its agent is told when it next has it.
   A card passed goes on to `done`.
5. Each run of the environment is kept as a row of a `deployments` table:
   when, which branch and commit, whether it came up, how long it took
   and where it answers.
6. When everything passes, you land the branch.

## What is yours, and what is the factory's

**Yours**: how your project is built and started. Mythwright does not
know your build system. You give it two commands, as an environment's
variables:

- `BUILD`: a program of yours that checks the branch out (it is handed the
  branch) and builds it.
- `SERVE`: the program that runs what was built, given the port it is to
  answer on.

**The factory's**: everything around them. Running the blueprint for a
branch, starting and watching what was built as a service, moving and
giving cards, the inbox line and its buttons, the table, and the events
anything else can listen for.

## Set it up

The environment, its variables, and the blueprint it is loaded by:

```
mw project environment shop dev add
mw project environment shop dev set BUILD=./scripts/build-preview SERVE=./target/release/shop PORT=7760 TRIED_BY=user:you
mw project environment shop dev deploy preview
```

`preview` is the blueprint below, saved among the project's own
(`mw file write shop @blueprints/preview.bp --from preview.bp`, or in the
client under the project's Blueprints). A project's own blueprint of a
name is found before one that ships.

Then, each time there is something to try, apply it for the environment
with the branch and the cards it holds:

```
mw apply preview.bp --project shop --environment dev \
  --param branch=origin/dev --param 'cards=["shop-12","shop-14"]'
```

**Run** in the environment's drawer in the client (or `mw project
environment shop dev run origin/dev`) does the same with the branch alone:
it hands the blueprint `branch` and nothing more, so the preview is built
and served and no card is moved. To have Run bring the cards too, let the
blueprint find them itself: a `board.list` step of the column your merged
cards wait in, in place of the `cards` param.

## The blueprint

```yaml
blueprint: preview
version: 1
description: Build the branch this environment is run for, start it at the environment's port, and hand the cards it holds to whoever tries it.
permissions: [shell, fleet, projects:write, inbox:write, records, board]

params:
  branch:
    type: string
    description: The branch to try, as the environment's Run hands it.

  cards:
    type: list
    default: []
    description: The cards the branch holds.

steps:
  table:
    description: The table each deploy is kept in, made the first time.
    do: collection.ensure
    with:
      name: deployments
      by_hand: false
      fields:
        branch: { type: text }
        result: { type: choice, of: [up, failed] }
        address: { type: text }
        why: { type: text }

  before:
    description: The preview shown before, put away.
    allow_failure: true
    do: service.stop
    with: { service: preview }

  built:
    description: The branch checked out and built, by the project's own command.
    needs: [before]
    do: shell
    with: { cmd: ["${environment.vars.BUILD}", "${params.branch}"] }

  serve:
    description: What was built, started at the environment's port.
    needs: [built]
    do: service.start
    with:
      name: preview
      cmd: ["${environment.vars.SERVE}", "--port", "{port}"]
      port: "${environment.vars.PORT}"
      restart: never
      health: { tcp: true }

  up:
    description: The preview answering.
    needs: [serve]
    do: service.wait_healthy
    with: { service: preview }

  staged:
    description: The cards the branch holds, moved to where they are tried from.
    needs: [up]
    for_each: ${params.cards}
    allow_failure: true
    do: board.move
    with:
      id: "${item}"
      to: staging
      note: "On the preview at http://${steps.serve.out.address}/"

  given:
    description: Each of them given to whoever tries the preview.
    needs: [staged]
    for_each: ${params.cards}
    allow_failure: true
    do: board.update
    with: { id: "${item}", assignee: "${environment.vars.TRIED_BY}" }

  told:
    description: Whoever ran it told that the preview is ready, with the cards to pass or fail.
    needs: [given]
    do: inbox.tell
    with:
      to: applier
      why: preview
      about: "${instance.id}"
      says: "The preview of ${params.branch} is ready to try at http://${steps.serve.out.address}/"
      parts:
        - { kind: cards, column: staging, pass_to: done, fail_to: ready, tab: To test }
        - { kind: runs, key: "${instance.key}", tab: Deployment }
        - { kind: links, links: ["http://${steps.serve.out.address}/"], tab: Links }

  kept:
    description: This deploy kept as a row of the project's deployments.
    needs: [told]
    allow_failure: true
    do: record.add
    with:
      collection: deployments
      fields:
        title: "${params.branch}"
        branch: "${params.branch}"
        result: up
        address: "http://${steps.serve.out.address}/"

on_fail:
  - do: record.add
    with:
      collection: deployments
      fields:
        title: "${params.branch}"
        branch: "${params.branch}"
        result: failed
        why: "${ run.error ?? '' }"

outputs:
  address: "${steps.serve.out.address}"
  cards: "${params.cards}"
```

Read it from the top: each step says in its `description` what is true
once it has run, and `needs` says what it waits for. `mw explain
preview.bp` says the same in plain words, `mw graph preview.bp` draws it,
and `mw sandbox preview.bp` runs it where nothing is real, before you
trust it with your board.

## What you see

- **Your inbox**: one line, *The preview of origin/dev is ready to try at
  …*. Opened, it has three tabs. **To test** lists the cards with Pass and
  Fail. **Deployment** lists every run of this environment, and each opens
  its steps and what they printed. **Links** holds the address.
- **The board**: the cards in `staging`, given to you, each with a note
  saying where they can be tried.
- **The `deployments` table**: a row for each run, up or failed and why,
  so the recent history of an environment is read in one place.
- **The environment's row** in the project's environments: its last run in
  one sentence, and **Run** again for another branch.

## Going further

Everything above is a starting point; each of these is a step or a
blueprint more:

- **Show what was checked.** Have your CI or your own test command write
  each run as a row of a table (`test-runs`: what was run, on which
  commit, how long, passed or not), and add a `rows` part to the inbox
  line that shows the rows for the branch's commits. Whoever tries the
  preview sees what already passed beside what is left to try.
- **Try it on real data.** When what you build is Mythwright itself, as
  ours is, `factory.snapshot` takes a copy of the factory in use and the
  preview serves that copy: everything you have, nothing you can break.
  For a service of your own, the same place in the blueprint is a step
  that restores last night's database copy into the preview's.
- **Do something when it is ready.** The load emits
  `project.environment.ready` with the address and the cards
  ([Environments](environments.md), *Events of a load*). A blueprint of
  yours that hears it can have an agent try each card against the
  address and write what it found under the card, before a person looks.
- **Land when everything passes.** A blueprint that hears a card moved to
  `done` can check whether any card of the branch is still in `staging`,
  and when none is, merge the branch (`merge`, as the fleet does) and run
  the next environment for it.
- **Keep several.** `dev` for every branch, `staging` for what is about to
  ship, each its own port and variables, each with its own row in the
  deployments table.

The factory decides none of this on its own: which branch, which cards,
who tries them and what happens when they pass are all in the text above,
so a team that works differently changes the text.

See also [Environments](environments.md) for variables, secrets and
loading from a branch, and [Approvals](approvals.md) for a go-ahead a run
waits on.
