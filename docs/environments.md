<!-- public -->

# A project's environments

An **environment** is a named set of variables that belongs to one
project, such as `dev`, `staging` and `prod`. What is applied for the
project runs under one of them, so one blueprint can do the right thing
in each. Each project has its own list, and a project may have none.

An environment is not a **profile**. A profile is a whole separate copy
of the factory: a daemon with a data folder of its own
([profiles.md](profiles.md)). Before 0.1.0 a profile was called an
environment.

You can make environments, give them variables and secrets, mark what
agents may see, read them and remove them, from a terminal or the
client. Whatever is applied for a project runs under one of its
environments, and so does an agent working there. An environment can
also say how it is loaded from a branch of the project's repository, and
be run for a branch ([Loaded from a branch](#loaded-from-a-branch)).

## Starting a project with environments

A new project can start with environments. Ask for them when you add it
(the Environments switch in the dialog that adds a project, or
`mw project create shop --with environments`): `std/environments` then
makes `dev` and `prod`, with no variables, or the names its `names`
param gives.

This is off unless asked for, because from then on everything applied
for the project must say which environment it runs under. What the
project starts with is applied under none.

## From a terminal

```
mw project environment shop                          # its environments
mw project environment shop staging add              # one made, with no variables
mw project environment shop staging set api=https://staging.example replicas=2
mw project environment shop staging agents replicas  # handed to agents (`--not` takes the mark off)
mw project environment shop staging                  # one, with its variables
mw project environment shop staging unset replicas
mw project environment shop staging remove           # removed, with its variables and secrets
mw secret set stripe --project shop --environment staging < key.txt
mw secret agents stripe STRIPE_KEY --project shop --environment staging
mw secret list --project shop --environment staging
mw secret remove stripe --project shop --environment staging
```

- An environment's name is lowercase letters, digits and `-`, at most 40
  characters.
- A variable has a name (letters, digits and `_`, not starting with a
  digit) and a plain value: text, a number, or `true` or `false`. On the
  command line, a value that reads as a number or `true`/`false` is that;
  anything else is text.
- `set` sets the variables it names and leaves the rest as they are.

## In the client

A project's environments have a page of their own. Open it from the gear
of the project's page (**Environments**, with how many it has) or, once
the project has one, from its line under the project in the side panel.

The page is a table with a row per environment, showing how many
variables and secrets it has, how many of those agents are handed, how
many things run under it, and its last run. A row opens the environment
in the drawer beside the table:

- its variables, edited in place, each with a **For agents** switch;
- its secrets by name, with who set each and when. A secret is set or
  replaced in a field that hides what you type, and is never shown again.
  Each can be marked with the variable an agent is handed it as, and
  removed behind a question that says what uses it;
- at the foot, **Remove this environment…**, which says what runs under
  it where something does, and is then closed.

Nothing on the page can show a secret's value: the daemon never sends
one.

When you apply a blueprint for a project that has environments, the
drawer beside its text asks which, and **Apply** stays closed, saying
why, until you choose. Applying over something that already runs keeps
the environment it runs under, and asks nothing. Activity and an
instance's page show which environment something runs under, or
*under no environment* where its project has some.

## Loaded from a branch

An environment is loaded the way a deploy service loads one: from a
branch. The environment names the blueprint it is loaded by (the
project's own blueprint of that name is found first, else one that
ships). Someone then runs it for a branch of the project's repository,
either a local branch (`dev`) or one as a remote holds it
(`origin/dev`):

```
mw project branches shop                         # local branches and each remote's
mw project branches shop --fetch                 # ask the remotes what they have now
mw project environment shop dev deploy preview   # loaded by the blueprint `preview`
mw project environment shop dev run origin/dev   # load it from that branch
mw project environment shop dev deploy --none
```

Run applies that blueprint for the project under the environment, as
whoever pressed it, through the same path as every apply: it needs what
applying needs, and what the blueprint asks for is asked of that person.

- The blueprint is handed the branch as its param `branch`, and the
  branch's commit as `commit` if it declares that param. A blueprint
  that does not declare `branch` is refused, with a sentence saying to.
- It is applied under the key `deploy-<env>`. Its steps run again each
  time, with the project's own blueprints taken as they are then.
- What it does with the branch (check it out, build it, serve it) is
  entirely the blueprint's. The application knows nothing of deploying.

```yaml
blueprint: preview
params:
  branch: { type: string }
steps:
  built:
    description: The branch checked out and built.
    do: shell
    with: { cmd: [./build, "${params.branch}"] }
outputs:
  address: "${steps.built.out.stdout}"
```

### What is kept of the last load

The environment keeps what it was last loaded with: the branch, the
commit, the blueprint, who and when, and the run it began. Keeping it
emits `project.environment.ran` in the same write. How the load went
belongs to the run, and is read from the run whenever the environment is
shown; it is never stored twice.

What is kept is written once the apply has answered, so a daemon stopped
between the two leaves it naming the run before. So when the newest run
of the deploy (the instance under `deploy-<env>`) is newer than what is
kept, that run is shown, with the branch it was handed and its commit
where its blueprint takes one. What is shown is never older than what
ran.

### The last run, in one sentence

What an environment last ran is said in one sentence. The daemon works it
out (`environments::last`, with its cases) and sends it with the
environment as `last`. The client's table shows it under **Last run**,
and `mw project environment` prints it, each with when beside it. In
order, it is:

1. what was last loaded through Run, if anything;
2. else the latest run of anything applied under the environment, however
   it was started (`apply --environment dev` under a key of its own, or
   something a step started under it), said as what was applied, its key
   and who applied it;
3. else, where something runs under it but none of its runs was read,
   that;
4. else that nothing ran, with the command that names the blueprint it is
   loaded by, if it names none.

It never says that nothing ran while something runs under it. What is
read for it is bounded like a list of runs: the project's latest 200 runs
and the instances alive or recently made (500). A project with more says
what it found among those, never "never run". Someone who may not see
what runs (`blueprints:read`) is told only what was loaded through Run,
or what the environment says of itself, and nothing of what was applied
under it.

### Run in the client

In the client, the environments' table shows the sentence under **Last
run**. The environment's drawer has a branch to choose (the last one run
chosen first), **Fetch**, **Run**, and the run's steps as they go, each
shown by its description, with its output when it is over.

A project that is not a repository of its own has no branches and no
Run. Fetching talks to the remotes, which is the network: it is done
only when someone asks, as the user who runs the daemon, and never
prompts. A remote that wants a password is reported as having refused,
in git's own words.

## Events of a load

A load emits events any blueprint can listen for. Each carries the
project, the environment, the branch, the commit, the blueprint, the
instance, and the run the load began:

- `project.environment.ready`: the run ended well. It says how long the
  load took (`took_ms`), where it can be reached (`address`: what the
  blueprint outputs as `address`), and the cards it holds (`cards`: what
  the blueprint outputs as `cards`, a list or ids separated by commas).
- `project.environment.failed`: the run failed or was cut short, with
  `reason`.
- `project.environment.down`: what was ready was later taken down
  (stopped, or failed, by whatever ended it), with how long it was up
  (`up_ms`). A blueprint that ends well and leaves nothing running is
  never taken down: it was never up.

Each is emitted once, for the load the environment keeps. Pressing Run
again before the first load is over replaces it, and the first emits
nothing. Each is emitted in the same write that records it was emitted,
so a daemon stopped mid-load emits it when it is back, as the run is
taken up and ends.

How long each load took is measured as `project.environment.loaded`
(marked failed where it did not end well), and how long it was up as
`project.environment.up`, with the labels `project`, `environment` and
`branch`. They are counted, never scored.

Anything done when an environment is ready (say, an agent that tests
each card it holds) is a blueprint of the project's own that listens for
the event. The factory does nothing on its own.

## Agents

An agent working under an environment is handed only the variables
marked for agents, under their own names, and sees nothing of the rest:
what the daemon answers to an agent's credential holds only the marked
variables. **Nothing is marked unless someone marks it.** An environment
filled for a deploy must not put `prod`'s database in a working agent's
way just because nobody thought of agents. A name the application sets
itself in a program's environment (`PATH`, `HOME`, `FACTORY_…`, `MW_…`)
is never marked.

An agent works under one environment of its project, or none:

- An agent brought up by a fleet, or by anything applied under an
  environment, works under that environment.
- An agent added by hand says which:

  ```
  mw agent add helper --project shop --environment prod
  ```

  In a project that has environments, naming none is refused, with the
  names listed. `mw agent list` shows which environment each agent is
  under.

As each turn starts, the agent's program is handed what of its
environment is marked for agents: each marked variable under its own
name, and each marked secret as the variable it is marked with
(`mw secret agents deploy DEPLOY_KEY …`).

A key the agent's own slot names (`--secret GH_TOKEN=github-bot`) is
looked up in its environment first. If the environment lists it, it is
handed only when marked for agents; otherwise the turn does not start,
and the error gives the command that marks it. The factory's secret of
the same name is never handed in its place.

What the program prints is written down with each secret's name in place
of its value. An environment cannot be removed while an agent works
under it.

## Running under an environment

Anything applied for a project that has environments is applied under
one of them, named as it is applied:

```
mw apply deploy --project shop --environment prod
```

- Naming none is refused, with a sentence listing the environments there
  are. A project with no environments runs as before.
- An instance runs under the environment it was applied under for its
  whole life (`mw instances` shows which), and so does everything it
  starts: what a step calls or spawns, and what it applies for itself.
- Applying it again under another environment is refused: stop it, or
  apply it under another key.
- An instance applied before its project had environments goes on under
  none.
- What a project starts with (its board, fleet, library, features),
  given when it is added or later, runs under no environment, and says
  so, even in a project that has some. It looks after the project's
  tables, which belong to the project whatever it deploys to.

A blueprint reads the environment it runs under:

```yaml
steps:
  deploy:
    when: ${ environment.name == "prod" }
    do: shell
    env:
      API: "${ environment.vars.api ?? profile.vars.api }"
      STRIPE_KEY: { secret: stripe }   # the environment's `stripe`, else the factory's
    with: { cmd: [./deploy] }
```

- `environment.name` is the environment's name, empty for none, and
  `environment.vars.<name>` is one of its variables, as it is when the
  expression is worked out. A changed value is read next time; nothing
  runs again because of it.
- Every program a step starts gets `FACTORY_ENVIRONMENT`, the
  environment's name.
- `{ secret: stripe }` is the environment's `stripe` if it lists one,
  and the factory's otherwise. A secret name containing `/` is refused,
  so nothing can name another project's secret. What a program prints of
  one is written as `[the secret `stripe` of the `prod` environment]`.
- What an agent applies reads only the variables marked for agents, and
  is handed only secrets marked for agents. An unmarked secret is
  refused, and the factory's secret of the same name is never handed in
  its place.
- A step sent to another device is refused an environment's secret: the
  device keeps none of them.
- An environment cannot be removed while anything runs under it.
  `GET /api/projects/<p>/environments/<e>/secrets` tells whoever holds
  `environments:secrets` where each of its secrets is named: by what runs
  under it, and by a service started with it.

## Rules

An environment can set rules. Each rule does nothing while it is empty.

```
mw project environment shop prod approve grace@example.com   # who approves what runs under it (`--none` for nobody)
mw project environment shop prod branches main release/*     # the branches it runs (`--any` for any)
```

- **Who approves.** Anyone the rule names may start what they like under
  the environment. Anything started by someone else is started only by a
  run that holds a go-ahead one of them gave: a step of `approval.ask`
  with `environment: prod` before the step that starts it. Someone not
  named who presses Run, or applies under it directly, is refused, since
  there is nobody to wait for. The go-ahead belongs to the run and is
  read, not used up: a run taken up after the daemon was stopped finds it
  again, and nothing else can use it.
- **Branches.** Anything handed a branch (Run's, or a step's
  `with: { branch: … }`) is refused for a branch the rule does not name.
  A branch is named whole, as it is chosen (`main`, or as a remote holds
  it, `origin/main`), or by a prefix ending in `*` (`release/*`).

Both rules are checked where something comes to run under the
environment: an apply that names it, and a step of a run under no
environment that names it (`call: deploy`, `environment: prod`,
`as: deploy-prod`). What is started by something already running under
it is already under it, and is not checked again. Setting the rules needs
`environments:secrets`, as setting a secret does.

The environment's page lists the runs that acted under it, newest first,
each opening its run's page; its last run is the first of them. A
pipeline run under no environment that started its child under this
environment is listed as that child, which is what acted there.

## Secrets

An environment has secrets of its own, beside its variables: values a
program needs and nothing else should see.

- Each is set by name, read from standard input or a file, never from an
  argument.
- No route, command or page ever shows its value again. What is listed
  is the name, who set it and when, and the variable an agent is handed
  it as.
- It is kept in the factory's one store of secrets, in a folder of its
  own that is never listed with the factory's
  (`secrets/@projects/<project>/<environment>/<name>`). It is sealed with
  a key of this installation under that whole name, so a file moved into
  another environment's or another project's place does not open.
- A name a blueprint gives is never taken for one of these: whatever
  reads an environment's secret says which project and environment it is
  for.

What an agent is handed a secret as is set per secret
(`mw secret agents <name> STRIPE_KEY …`), and nothing is handed unless
someone says so. One name in an agent's program is one thing: a secret
is not handed under a name that a marked variable, or another secret,
already uses.

A secret is listed by its environment, and its value is a file. The file
is written before the list names it, and the list stops naming it before
the file is removed, so a stop between the two leaves a file nothing
reads. A listed secret with no value (a copy of the factory carries no
values) shows as not set, and is set again there.

## Feature flags

A project can keep feature flags with `std/flags`, one of the things a
project can be set up with: `mw project create <name> --with environments
--with flags`, or its switch in the dialog that adds a project.

Each flag is a row of the project's `flags` table: its name, what it is
for, its owner, the day it is due to be removed, and the kind of value it
holds:

- `switch`: on or off;
- `percentage`: a whole number from 0 to 100;
- `word`: a plain word.

### Where a flag's value lives

What a flag says in each environment is that environment's variable
`FLAG_<NAME>` (`FLAG_NEW_CHECKOUT` for `new-checkout`). It is written in
one write with its event, `project.flag.changed`, which names the flag's
row, so the row's history keeps each change.

That variable is the only place the value is kept. An app the project
deploys reads it like any setting, and a blueprint run under the
environment reads it as `environment.vars`, so nothing is fetched and
nothing is sent. Where an environment says nothing of a flag, it is off.

A flag's variable is changed only as the flag (`mw flag set`, or the
**Flags** part of the project's Environments page), never directly: the
route that sets a variable refuses a name starting with `FLAG_`. Like any
variable, it is not handed to agents unless marked for them.

### Approvals

Turning a flag on, or wider (a larger percentage, another word), in an
environment whose rule names who approves, waits for one of them, unless
the person asking is one of them:

1. The change is reported to what looks after the project's flags
   (`project.flag.asked`).
2. That asks for a go-ahead (`approval.ask`) under the environment.
3. Once one of the approvers lets it through, the change is written, as
   long as whoever asked still holds what changing a flag needs.

**An approval never undoes a later change.** The asked change is kept on
the environment, with what the flag said at the time, in the same write
that records the ask. When approved, it is written only if, as that
write reads the environment, it is still the latest change asked of that
flag there, and the flag still says what it said when it was asked.
Every change of the flag written in between replaces it (even turning it
off when it was already off), and so does a later ask of the same flag
there, so two approvals cannot land out of order. If it was overtaken,
nothing is written, and the person who asked and the flag's history are
told (`project.flag.unchanged`: "approved after the flag was changed:
nothing written").

A key that holds less than its maker counts as itself, never as its
maker, so it has to ask. Turning a flag off, or narrower, never waits:
that is what a flag is for when something goes wrong. What is decided,
and the wording, is `kernel::flags`, with its table of cases.

### Flags past their day

A flag past its removal day is reported once to its owner (or, where its
row names nobody, to whoever applied the flags). The check runs when the
flags are first looked after, whenever a row is written, and daily at
the time `std/flags` is given (`looks_at`, `09:00 UTC` by default).
Flags that are never removed are the cost of having them.

### A percentage flag as a comparison

A percentage flag can be started as a comparison on real work:

```
mw flag ways <project> <flag> --environment E --blueprint B --per P --param F
```

The blueprint is started for each thing its `per` param names, in two
ways: `on` for the flag's share of the things and `off` for the rest,
each handed its word as the blueprint's param `param`. What comes of each
way is counted, never scored, as in any comparison (`mw compare show`).

## Who may

| Action | Needs |
|---|---|
| See a project's environments and variables | `projects:read`; an agent sees only the marked variables |
| Make an environment; set, remove or mark a variable; remove an environment | `projects:write`, `shell` and `secrets:use`: a variable's value reaches the commands and secret names of what is applied, run as whoever applied it |
| See an environment's secrets by name | `secrets:use` as well as reading the project; an agent sees those marked for it |
| Set the blueprint it is loaded by | what changing it needs |
| Set who approves what runs under it, and which branches it runs | `environments:secrets` |
| See the project's branches | `projects:read` |
| Fetch them from its remotes | `projects:write` and `shell`: git starts whatever the repository's settings name to reach a remote |
| Run it for a branch | `blueprints:run` and `projects:read`, and whatever the blueprint asks of whoever applies it |
| See the project's flags and what each says in each environment | `projects:read` and reading its records; an agent sees only what is marked for agents |
| Set what a flag says in an environment, or remove its variable | `projects:write`, nothing more: a flag only ever says on, off, a percentage or a plain word. Turning one on or wider where the rule names approvers waits for one of them |
| Start a percentage flag as a comparison | `blueprints:run` |
| Set, replace, remove or mark a secret, or remove an environment that lists any | `environments:secrets`. The factory's owner holds it without being given it, and can give it to anyone (Settings, People; `mw give <email> environments:secrets`). Holding it never shows anyone a value |

## Where it is kept

Each environment is kept whole in the journal of the conductor that runs
what is applied for the project (`runs::beside::PROJECT_ENVIRONMENTS`),
so it goes with the project:

- into a copy of the factory, with its secrets' names and none of their
  values;
- into the project's own journal when the project is moved apart
  (`mw apart`);
- away with the project when the project is deleted for good, from the
  Trash or at once, along with its secrets' values. While the project is
  in the Trash they stay, so it comes back whole.

Every change is reported by name, never by value:
`project.environment.made`, `.changed` (variables and secrets set,
changed, removed or marked, and the blueprint it is loaded by), `.ran`
and `.removed`, heard by whoever may read the project.

Variables are not for secrets: they are shown to whoever may read the
project, stored as they are, and may be printed by whatever reads them.
