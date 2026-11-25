<!-- public -->

# Profiles

`prod` and `dev1` are two **factories**, each with a **profile**: a name,
and what its owner has said is true of it. This page says what that means,
what keeps two of them apart, and what does not.

Before 0.1.0 a profile was called an environment: `mw up --env dev1`,
`MW_ENV`, `environment.name` and `std/environment`. The command line still
takes `--env`, `MW_ENV` and `mw env`, and says once that they are the old
spelling; the list of factories on a device is still kept in
`environments.json`, which every version reads. The word environment is
kept for what a project has of its own (`dev`, `staging`, `prod`):
[environments.md](environments.md).

## What a profile is, and is not

A **factory** is one daemon with its own data folder: its own database, its
own accounts and credentials, its own projects, channels, boards, events and
measurements, and its own address. That is what there always was; a second
factory is a second daemon on a second data folder.

A factory **has a profile**: a name such as `prod` or `dev1` (a
lowercase word: letters, digits and dashes). The name is what a person, a
page and a blueprint call the factory by.

- **Factories share nothing.** There is no database, account, credential,
  event or measurement that two of them both see. Nothing here copies
  anything from one to another unless a person asks, and then one way, into
  a factory that does not exist yet (*A copy of one factory, to start
  another from*, below).
- **The name is not the factory.** Two factories may be called the same
  (two developers' `dev1` on one build machine, one day). Nothing takes a
  name for one factory: where a name is not enough, it says so and asks for
  an address.
- **A factory with no name** is as factories always were. Nothing is shown
  for it, nothing is written about it outside its own data, and every
  command does what it did.

## Starting one, and talking to it

```sh
mw up --profile dev1            # the dev1 factory: data and an address of its own
mw --profile dev1 status        # any command, to that factory: its first start made its owner and signed this terminal in
mw --profile dev1 open          # its client, signed in. Signing in is to one factory
mw profiles                     # the factories here that have a profile
```

`MW_PROFILE=dev1` in a terminal says `--profile dev1` for every command typed there.
`--url` (or `MW_URL`) says a daemon by its address instead, as always; given
both flags the command is refused, and of the two variables the address is
used, which is the more exact (it is what an agent's slot is given).

**Its data.** With `--profile dev1` and no `--data`, the data folder is the usual
one with the name added: `~/.local/share/mythwright-dev1` beside
`~/.local/share/mythwright`. So `--profile dev1` never lands on the data of the
factory with no name, or of another profile.

**Its address.** With no `--listen`, the first start takes a free port on
`127.0.0.1`, and the factory listens there every time after: whatever was
pointed at it still is. `--listen` says another, which is then what it
remembers. If something else has taken its address while it was stopped, it
says so and does not go elsewhere unasked. A factory with no name listens
where it always has, `127.0.0.1:7700`.

**The name is for good.** The first time a data folder is started with a
name, the name is stamped into it (`factory.json`, beside the database).
After that:

- started with no `--profile`, it is still what it was stamped;
- started with another name, it is refused, with a sentence saying what the
  folder is. A slip of the hand cannot turn production's data into
  development's;
- a folder from before profiles is stamped by the first name it is
  started with.

So a factory that has no name is given one by starting it once with the name,
on the data and at the address it already has:

```sh
mw down
mw up --profile prod --data ~/.local/share/mythwright --listen 127.0.0.1:7700
```

From then on `mw up` alone starts it as `prod`, at that address. Its
accounts, its data, its address and the command line's session with it are
what they were. A browser signs in to it again, once: a named factory keeps
a browser's session under a cookie of its own name (below). (Left without
`--listen` that once, it would move to a free port, as a named factory given
no address does.)

**Finding one by name.** Each named factory writes itself into a list as it
starts: `environments.json` in the user's configuration folder
(`~/.config/mythwright/`), holding its name, its data folder and its address.
`--profile dev1` looks the address up there. The list never holds a credential.
If two factories on the device are called `dev1`, `--profile dev1` is refused,
both are listed with their data folders, and the one meant is said by its
address. Before anything is sent to an address found this way, what answers
there is asked what it is: if `dev1` has stopped and something else has its
port, nothing is sent to it.

**Signing in is per factory.** `mw --profile dev1 login` keeps its session in
`sessions/dev1.json` in the configuration folder, for that factory (by its
data folder, not its name) and no other; the session a daemon found by its
address uses (`session.json`) is not touched, and is itself sent only to
the address it was saved with (`--url` to another daemon asks to sign in).
In a browser each named factory has a session cookie of its own name,
because a browser keeps cookies by host and not by port: two factories on
`127.0.0.1` would otherwise sign each other out.

## What a blueprint can read

An expression can read `profile.name`: the factory's profile as text,
and the empty text in a factory with none.

```yaml
steps:
  let_loose:
    when: ${ profile.name != "prod" }
    do: agent.task
    with: { slot: "${ profile.name == 'prod' ? 'careful' : 'builder' }", prompt: "Fix the build." }
```

`profile` is a map with two parts, `name` and `vars` (below); the
checker refuses any other and says so. Whatever a blueprint runs, a `shell`
step or a service, has `FACTORY_PROFILE` in its environment beside
`FACTORY_CLI` and `FACTORY_URL`: the same name, empty in a factory with none.

An expression is worked out where the blueprint runs. A step sent to another
device (`device:`) runs its command in that device's factory, which tells it
its own profile.

Measurements carry no label for the profile: each factory has its own
measurements, and it would say the same thing on every row.

### The variables of a profile

What differs between `prod` and `dev1` (where a service answers, how many
of something to run, whether to be careful) is said once, of each factory,
by its owner, and read by any blueprint there as `profile.vars.<name>`.
One blueprint then does rightly in both without being edited for either:

```yaml
steps:
  migrate:
    do: shell
    with:
      run: ./migrate
      env: { API: "${ profile.vars.api ?? 'http://localhost:8080' }" }
  review:
    when: ${ profile.vars.careful ?? true }
    do: agent.task
    with: { slot: careful, prompt: "Review the change before it goes out." }
```

```sh
mw --profile prod apply std/profile --param 'vars={"api":"https://api.example","replicas":3,"careful":true}'
mw --profile dev1 apply std/profile --param 'vars={"api":"http://127.0.0.1:8080","careful":false}'
```

- **A variable that has not been set is nothing** (`null`), so `??` gives
  what to use instead, and `profile.vars` with nothing set is an empty
  map. The checker does not check what follows `vars`: the names are the
  owner's. Write a blueprint so that nothing is the careful case
  (`profile.vars.careful ?? true`): a factory where nobody has said
  anything yet, and a copy of a factory, have no variables.
- **A value is plain**: text, a number, or true or false. A list, a map or
  nothing is refused with a sentence; so is a name that could not be
  written after `profile.vars.` (letters, digits and `_`, not starting
  with a digit).
- **They are not for secrets.** They are shown to whoever may read
  blueprints and instances, kept in the factory's database as they are,
  and may appear in events and in the history of what was applied.
- **They are set whole**, by the `profile.set` action, which the
  blueprint `std/profile` runs with its one param, `vars`: a map,
  given on the command line as JSON, as above (a `--param` whose value
  reads as JSON is that). Applying it again with other variables replaces
  the set: one left out is no longer set. Applying it with none
  (`mw apply std/profile`) clears them.
- **They are the owner's to set**: `profile.set` takes `admin`, as a
  snapshot does, since it changes what every blueprint in the factory
  does. No member, script or agent holds that unless the owner gave it.
- **They are the factory's, not an instance's.** They are kept in its
  database, are there when the daemon is started again before anything is
  resumed, and stay until they are set again. `std/profile` finishes
  once it has set them and leaves nothing running, so there is nothing to
  stop, and stopping does not clear them: whoever may stop an instance
  (which takes less than `admin`) cannot take them away.
- **Setting them runs nothing again.** An instance that is running reads
  the new value the next time it works an expression out.
- **A change is an event**, `system.profile`: the names that were set,
  changed and removed, and who did it; never a value.
  `mw events --topics system.profile` shows them. What the values are
  is in the params of the last `std/profile` that was applied
  (`mw instances --all`, then `mw instance <id>`).
- **A command is not handed them.** What a `shell` step or a service runs
  has `FACTORY_PROFILE` and no variables in its environment; a
  blueprint passes what it wants with `env:`, as above.
- **A copy of a factory does not carry them** (below): they are said of
  the factory they were set in.

## In the client

A page in a factory that has a profile **says which**, in words: first in
the browser tab's title; across the top on a phone (where the panel lies
over the page or is put away); beside the product's name on the sign-in
form; and in Settings, with the others on the device. The head of a side
panel says what the panel is and not which factory this is, so that
neither is cut short. On the phone's line and the sign-in form it is
written in the theme's accent so that it is seen without being looked for,
it is nothing to press, and every profile is drawn alike, whatever it
is called. A factory with no name shows nothing.

**Settings** says which factory this is and lists the factories on the
device (`GET /api/profiles`).
Choosing another goes to that factory's address, which is another origin:
it has its own sign-in. The address offered is one that works from where the
page is, and where there is none to give honestly, the factory is listed
without a link and with the reason:

| The page was reached | Another factory that listens on | Is offered at |
|---|---|---|
| on the device itself (`127.0.0.1`, `localhost`) | the device only (`127.0.0.1:<port>`) | that address |
| on the device itself | one address of the device | that address |
| at an address or a name of the device, straight to this factory's port | every address (`0.0.0.0:<port>`) | the same host, its port |
| at an address of the device | that same address | that address |
| at an address of the device | the device only | nothing: it cannot be reached from elsewhere |
| at a name, straight to this factory's port | one address of the device | nothing: whether the name is that address is not known |
| through a proxy (another port, or a forwarding header) | anything | nothing: the proxy leads to this factory and no other |

A tunnel that brings one factory's port to another machine under
`127.0.0.1` looks, to the daemon, like a page on the device: the other
factory is then offered at an address the tunnel does not carry. The link
fails to open; nothing is sent anywhere it should not be.

## Serving each over a private network

A factory is reached from other devices by putting something in front of it
(a proxy, or what a private network of your own offers for serving a port
of the device), and each factory needs its own, on a port of its own. The
daemon answers only at an address of its own or under a name it was told,
so each is told the name it is reached under:

```sh
mw --profile prod names add box.example.net
mw --profile dev1 names add box.example.net
```

On a phone, each page then lists the other without a link (it came through
a proxy that leads to one factory); the other is opened at its own port.

## A copy of one factory, to start another from

To try something against what a real factory holds, with no chance of
touching it, a new factory is started with the same content. It takes two
commands, and both are a person's to give:

```sh
mw --profile prod apply std/snapshot --param to=/somewhere/prod-copy   # prod writes a copy of itself to a folder
mw up --profile dev2 --from /somewhere/prod-copy                       # a new factory, dev2, starts from the folder
```

**The first** is asked of the factory to copy, by its owner (the
`factory.snapshot` action, which `std/snapshot` runs, takes `admin`: no
member, script or agent holds that unless the owner gave it). That factory
is only read, and carries on with what it is doing meanwhile. `to` is a
folder said by its whole path, that does not exist yet or is empty, outside
the factory's data and its projects. It is made readable by its owner only.

**The second** starts a factory that does not exist yet. `--from` is refused,
with a sentence saying what to do instead:

- where the data folder already holds a factory, or anything else. A copy
  is never put over, or into, a factory that exists;
- without `--profile`: the new factory needs a name of its own;
- under the name of the factory the snapshot was made from;
- from a folder that is not a snapshot, or from one made by a newer version
  than the one starting.

Once it has started, the new factory is a factory like any other, and is
started from then on with `mw up --profile dev2`. It says what it was started
from to whoever started it, and once in its own journal (`system.copied`);
the factory copied says that a snapshot was made (`system.snapshot`), and
measures how long it took (`mw metrics snapshot`).

**A snapshot is a plain folder**, which can be handed to another user or
another device and started there:

```text
snapshot.json    what it is, when and from which factory it was made, and what is not in it
database.db      the factory's own database
journals/        the journal of each project that has one of its own
attachments/     what was attached to messages
extensions/      the installed extension packages
projects/        the files of the projects kept in the factory's own projects folder
```

The database and the journals are each as they were at one moment, the
same for all of them, whatever the factory was doing as the copy was
taken: every journal stands still for an instant (a millisecond, measured
as `mw metrics snapshot.stood`) while each file begins to be read, and
goes on while they are written out. A factory with a great deal under way
is not waited for past five seconds; the snapshot then fails, saying which
journal was busy, and is asked for again.

**Treat the folder as you treat the factory's own data.** It holds
everything said in every channel, every card and every file of the projects
copied, and people's password hashes. And **it is not a backup**: a factory
cannot be put back from it as it was, because its credentials and what it
was running are not in it.

### What a copy holds, and what it leaves out

| | In a copy |
|---|---|
| Accounts | As they were, password hashes among them: the same people sign in with the passwords they have. |
| Sessions, credentials, invitations | None. Everyone signs in again; a key for a script, a device's credential and an invitation are not good in the new factory. |
| Channels, messages, what was attached | As they were. Something attached to a message that was never posted is not copied. |
| Tables, their records and histories; boards | As they were, for a table kept in the factory's database or as files in a project that was copied. |
| A table kept by a store service | The table is declared, and **has no store**: its address is not copied, since it is the other factory's store, and the copy's board would read and write that factory's cards. It says so until a store is hosted in the new factory and the table's blueprint is applied again with its address. |
| Projects inside the factory's projects folder | With their files, in the new factory's own projects folder. |
| A project made from a directory elsewhere (`--path`) | It is a project in the new factory, with **an empty directory of its own**. Its files are not copied: two factories never share a directory, and the directory is the other factory's. `snapshot.json` and the new factory name the project and the directory. |
| In a copied project | No working directories of lanes (`.lanes`), no list of a repository's other working directories (`.git/worktrees`), no `.git` file that says the repository is elsewhere, no link that leads by its whole path into the factory copied, and nothing that is not a file, a folder or a link. Branches stay. A link is copied as a link and never followed. `snapshot.json` lists what was left. |
| Events | As they were: the journal is the history, and the sequence the rest refers to. |
| Agents | Each slot as it was set up, and what each has done. Not its credential: each is given one of the new factory's by itself, when it next works. Not the folder of its slot, so what an agent with no project wrote there is left behind (`snapshot.json` says which agents). |
| Instances, runs and steps | As history. An instance that was live is `stopped`, saying that a copy starts nothing; a run or step that had not finished is `cancelled`. No hook runs and nothing is torn down: nothing is running in a copy. |
| Services and routers | None. A daemon finds its processes again from those rows, and the processes are the other factory's. |
| Agents' turns | As history. One that was waiting or under way is `failed`, saying so: the new factory's agents do not take up the other's work. |
| Where things had read to in the journal | Forgotten, so that nothing that already happened is acted on again: a message that was answered is not answered twice, and no event from before the copy fires a handler. (Where the keeper of records' histories had read to is kept: it writes each line once, and carries on.) |
| Conversations agents' programs remember, the models they said they offer | Forgotten: said of the account each was signed in with there. |
| The sign-ins the factory holds for its agents' programs (`mw signin`) | None: not their values (keys and tokens, each a file in the data folder's `signins`, which is not copied), not where a provider's program keeps its own, and no row that says which there are, who held which or what was set aside. A sign-in in two factories would be held by more programs than it says. What each agent draws its sign-in from goes with the agent: in the new factory its work fails, saying the factory holds none of those, until some are added there. It does not fall back to the sign-in of whoever runs the daemon. |
| Measurements, logs, the search index | None. The index is built again in full; the others start clean. |
| The link to a hub, the devices that had joined | None. |
| The variables of the profile (`profile.vars`) | None. They were said of the factory copied (where its services answer), and a copy that read them would take that factory's addresses for its own. They are set in the new factory by applying `std/profile` there. |
| Extension packages | As installed. |

What is left out is left out **when the snapshot is made**: sessions and
credentials are not in the folder at all, not even in the unused parts of
the database file. It is taken out **again on arrival**, in case the folder
was changed or made some other way, and then the new factory says that it
had to be.

**What to do in the new factory.** Sign in again. Say what is true of its
profile, if its blueprints read variables: `mw --profile dev2 apply
std/profile --param 'vars={...}'`. Apply again the blueprints
you want running there: `mw --profile dev2 apply std/board --project <project>`
gives a project its board's life back, and each unfinished card its own
blueprint. Put files into a project that came without them.

**Agents, and what they spend.** An agent in the copy answers what is said
to it there. Its credential for the new factory is made by itself. What its
program signs in to its model's provider with is one of two things, and
neither is copied:

- **Nothing said** (an agent that draws no sign-in): it is whoever runs the
  daemon. So under the same user as the factory copied, the copy's agents
  use the same sign-in, and what they do is charged where that factory's
  is; under another user, or on another device, each program has to be
  signed in there before its agents can work.
- **A sign-in the factory holds** (`mw help signin`): the copy holds none.
  An agent that draws one says so when it is next given work, and works
  again once the new factory has been given sign-ins of its own (`mw --profile
  dev2 signin add ...`). That is how two factories are given different
  ones: what `dev2` spends is then charged to what `dev2` holds.

**What stays behind in the history.** The journal, and the params and
outputs of what was applied, are copied as they were, and may name
addresses of the other factory (where its store answered, the port of a
service, the variables `std/profile` was applied with there). Nothing
in the copy reads them to act on; a person who applies one of those
blueprints again gives it addresses of the new factory.

**Files are copied as they are found.** The database is taken whole at one
moment, whatever is being written to it. A project's files are not: one
being written at that moment is copied as far as it had been. That includes
a store's database kept in a project (`std/store` keeps its in the
project's `.data`), which goes with the project's files; stop what writes
there first if the copy must be exact.

## The boundary: what it is, and what it is not

**What the product holds to.**

- Factories keep nothing in common: database, accounts, credentials,
  sessions, events, measurements, logs, extensions and addresses are each
  factory's own.
- A data folder belongs to one profile for good (the stamp, above).
- Two daemons cannot run on one data folder (the lock there always was).
- **A directory belongs to one profile.** A project can be made from an
  existing directory (`mw project create <name> --path <dir>`), and two
  factories that adopted the same one would share its files, and with them
  whatever is kept as files there: blueprints, a board kept as files. So a
  directory that a factory of one profile holds is refused to a factory
  of another, with a sentence; so is a directory inside it or around it, and
  a directory inside another factory's data folder. A factory with no name
  counts as a profile of its own here. Two factories of the *same*
  profile are let be.

  How it is known: a factory writes the directories it has adopted (those
  outside its own data folder) into the list in the configuration folder,
  when it adopts one and again each time it starts; a data folder is known
  on disk by the database in it. **Nothing is written into the directory
  itself.** What this does not catch: a factory that has not been started
  since it was updated to a version that writes the list has written
  nothing yet; a directory that *contains* the data folder of a factory with
  no name; two users' factories, whose lists are separate; and a directory
  reached under two names that are not links to each other (two mounts of
  one disk).
- The command line sends a saved session only to the daemon it was saved
  with: the one found by that name, or the one at that address. A page's
  cookie for one factory is not taken by another.

**What it does not hold to, and cannot from inside.**

On one device, under one user account, both factories run as the same
operating-system user. An agent's program is started behind a wall
([architecture.md](architecture.md), *Walls*), which shows it its work and
nothing of another factory. What is started with no wall is not kept in.
So:

- **A `shell` step in `dev1`, or an agent there that was told to work with
  no wall, can read `prod`'s files on disk**: its database, its projects,
  the credentials of its slots, the values of the sign-ins it holds
  (`signins` in its data folder: files only their owner can read, and the
  owner is the same user), and the session files in your configuration
  folder, with which it could act in `prod` as you. An agent behind its
  wall cannot: none of that is there for it. The permission modes (`read`,
  `edit`, `full`) are what an agent's program is asked to keep to; the
  wall is what it is held to, and holds `read` to reading.
- **An agent need not sign in to its provider as you.** One that is told
  what to draw is handed a sign-in its own factory holds, for the length of
  each piece of work, and the variables it would otherwise sign in by are
  taken out of what its program is started with; the daemon's own
  credentials for the command line are taken out for every agent. That
  decides what its program *is given*, and so what its work is charged to.
  Behind its wall, an agent that was handed one is not shown the sign-in of
  whoever runs the daemon at all. The one it was handed it can still read,
  and send anywhere: a wall does not hold the network.
- A browser sends a host's cookies to every port on that host. Each factory
  takes only its own, but each receives the other's, as does anything else
  that listens there. A cookie alone is therefore taken only for what a
  browser fetches by itself and reads; for anything else it counts with
  the key its own page keeps (architecture.md, *Identity*).
- Processes of both share the device: its ports, its processor and memory,
  its disk.

**A real hard line between production and development needs a different
user or a different device.** The product does not enforce either; it works
the same under both, since everything a factory keeps is under the home of
whoever runs it:

```sh
# Another user on the same device: its own home, so its own data, list and sessions.
sudo -u factory-dev -i mw up --profile dev1
sudo -u factory-dev -i mw --profile dev1 register

# Another device: the same commands there.
ssh builder mw up --profile dev1 --listen 0.0.0.0:7700     # then reach it at its address
```

Under another user the two factories do not see each other's lists, so
neither offers the way to the other and the directory check does not reach
across; file permissions are what keeps them apart, which is the point.

## What is not built

- **Promoting** a blueprint, or anything else, from one profile to
  another. Nothing copies between factories unless a person asks, one way,
  into a factory that does not exist yet (a snapshot, above); nothing goes
  back, and nothing is brought up to date afterwards.
- **A backup.** A snapshot leaves out credentials and what is running, on
  purpose; a factory cannot be put back from one as it was.
- **Choosing what goes into a snapshot.** It is the whole factory, less
  what is listed above: not one project, and not without the channels.
- **Moving a snapshot.** It is a folder; getting it to another user or
  device is copying the folder, by whatever means is trusted with it.
- **An address by name** for each profile (`dev1.` something). A factory
  is reached at its address, or by name on its own device.
- **More of the variables of a profile** than is said above: one
  that is secret, or a list or a map; setting or removing one without
  saying the rest; a place to read them other than a blueprint and the
  history of what was applied; and clearing them by stopping something.
  And other properties of a profile than its name and its variables:
  the map is there so that they can be added without what it holds now
  changing its meaning.
- **Enforcement of the hard line** described above.
- **Removing a factory from the list** by a command: one whose data folder
  is gone is dropped the next time the list is read.
