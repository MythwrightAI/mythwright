# Where commands run

The factory starts programs on your device: an agent's program for each
piece of work, a `shell` step's command, a service, a package's action.
This page says where each runs today, how an agent's program is held
behind a wall, and what is planned for running work somewhere other than
the device itself (a container, a small virtual machine): a **place**.

Of places, only one kind is built: the system's own wall around an agent's
program, on Linux. Everything under *Planned: places a package declares*
is design, not built.

## Where each program runs today

| What | Folder it runs in | Environment | When the daemon stops |
|---|---|---|---|
| An agent's program | Its project, or a card's own worktree; else the agent's own folder (`<data>/slots/<agent>`) | Behind a wall: only what any program needs, its credential, its sign-in and the keys its slot hands it | Ended, unless its slot `outlives` the daemon; its turn is begun again or taken up |
| A `shell` step | The project's folder (`cwd:` under it) | The daemon's, without `MW_*`, plus `FACTORY_CLI`, `FACTORY_URL`, `FACTORY_PROFILE` and the blueprint's and step's `env:` | Left running, and found again |
| A service | The same | The same, plus its port as `PORT` | Left running on purpose, and found again |
| A health check's `cmd:` | The service's folder | The daemon's | Ended |
| A package's declared action | The project's folder | As a `shell` step | Ended; done again if it is `repeatable` |

The built-in assistant runs inside the daemon and starts nothing.

Only an agent's program is behind a wall. A `shell` step, a service and a
package's action run in the open, as whoever runs the daemon, with
whatever authority whoever applied them has.

## The wall around an agent's program

An agent's program is started as whoever runs the daemon. With nothing
around it, it could read what that person can: their keys, the session of
whoever is signed in to the client, the factory's own secret, every other
project. So it is started behind a wall, held on Linux by bubblewrap
(`/usr/bin/bwrap`). Of the device it is shown:

| Shown | How |
|---|---|
| The system (`/usr`, `/etc`) and the folders on the path | To read |
| Where it works: its project, or a card's worktree with the rest of the project | To change; to read only for an agent held to `read` |
| A home of its own, where the home of whoever runs the daemon is | To change |
| Its credential and the `mw` command | To read |
| What its program signs in with | As its package says (`agent.wall.signin`) |
| Folders its slot adds (`reads`, `changes`) | To read, or to change |

Nothing else of the device is there, and it inherits nothing else of the
daemon's environment. The network is the device's.

Commands:

```
mw agent add builder --reads ~/.rustup --reads ~/.cargo     a toolchain it builds with
mw agent add builder --changes ~/.cache/sccache             a cache it may write to
mw agent add builder --git commit                           it may commit its own work
mw agent add builder --no-wall                              started in the open
mw agent add builder --wall                                 behind a wall again
mw agent try builder                                        start it behind its wall, ask it nothing
mw up --walls off                                           a factory that holds no walls at all
```

Taking a wall away, or showing a program more of the device, takes
`shell`. In a fleet the same are cells of an agent's row: `wall`, `reads`,
`changes` and `git`. `mw help wall` says the rest, and
[architecture.md](architecture.md), *Walls*, says why each is shown as it
is.

A wall that cannot be held is a failure, never the open device instead:
where there is no `bwrap`, or the system will not let it make a namespace,
an agent with a wall is not started and its turn says why. Nothing holds
a wall on Windows or macOS.

### What a wall does not hold

| | Why |
|---|---|
| The network | It is the device's. What is read behind a wall can be sent anywhere, and everything on the device's loopback can be reached. |
| The daemon's API | That is held by the agent's scopes, not by the wall. |
| The agent's own credentials | It can read its credential and its sign-in, and send them away. |
| What runs with no wall | A `shell` step, a service, a package's action. |
| The kernel | A wall shares it. Another user, or another device, is the only line against a program that sets out to break through. |

## What it does not do yet

None of this is built:

- places a package declares (a container, a small virtual machine);
- `shell` steps, services and health checks behind any wall or place;
- a step, a project or the factory saying which place work runs in;
- a socket for a place with no network to reach the daemon by;
- any wall at all on macOS or Windows.

## Planned: places a package declares

This is the design, written to be settled before it is built. Nothing in
it is built, and the YAML below has not been run.

A **place** is somewhere a command runs that is not the device itself. The
application knows a place by name only. A package says how its program
enters one, runs a command in it and leaves, as it already says how an
agent's program is run. The device itself is the place that answers "run
the same, and do nothing before or after".

### What a package would declare

A place is named `<package>.<name>`, as an action is. For a container
program:

```yaml
extension: podman
program: podman

places:
  container:
    description: A container for one command, removed when the command ends.
    with:
      image: { type: string, required: true, description: "It must hold a shell and the programs the work runs." }
    bin: /usr/local/bin           # a folder on the PATH inside; the command-line program is given there
    run:
      args:                       # put before the command, which follows as it is
        - ["run", "--rm", "--name", "${ place.name }", "--workdir", "${ command.dir }"]
        - when: ${ command.input }
          args: ["--interactive"]
        - ["--network", "${ given.network ? 'host' : 'none' }"]
        - each: ${ given.dirs }
          args: ["--volume", "${ item.path }:${ item.path }:${ item.write ? 'rw' : 'ro' }"]
        - each: ${ command.env }  # names only: the values are in the program's own environment
          args: ["--env", "${ item }"]
        - ["${ with.image }"]
      fails:                      # when the outcome is the place's failure, not the command's
        - when: ${ code == 125 }
          says: "The container could not be started. The step's log has what the program said."
    leave: { args: ["rm", "--force", "--ignore", "${ place.name }"] }
    find: { args: ["container", "exists", "${ place.name }"], there: "${ code == 0 }" }
```

A small virtual machine has the same shape. It takes seconds to start, so
it would be kept between commands (it has `enter`), its folders are
shared folders, and its network is its own, so it says what the device is
called from inside (`host`).

| Field | What it does |
|---|---|
| `description`, `with`, `refuse` | As an action's. `with` is filled by whoever defines a place, not by a step. |
| `run` | Needed. `args` go before the command. `fails` says when the outcome is the place's own failure; otherwise the exit code is the command's. Standard output is the command's alone. |
| `enter` | One run before the first command. A place with `enter` is kept; one without lives for one command. |
| `leave` | One run after the last command, harmless to do twice. Needed with `enter`. |
| `find` | One run when the daemon starts again, to ask whether a kept place is still there. |
| `host` | What the device is called from inside. |
| `bin`, `shell` | A folder on the `PATH` inside, and how a script is run there (`["sh", "-c"]` unless said). |
| Expressions | `with`, `place.name`, `entered`, `given.dirs`, `given.network`, `given.ports`, `command.dir`, `command.input`, and `command.env`, which holds variable **names** only, so no value can reach an argument or a log. |

### Rules the design holds to

- **Inside a place nothing is inherited.** A command has the folders and
  variables it was given and no others. A place that cannot be entered is
  a failure, never the device instead.
- **What a place is given:** the project's folder (to read only for an
  agent held to `read`), or an empty folder of its own with no project;
  the `mw` program in `bin`; `FACTORY_*` and the blueprint's `env:`; for an
  agent's turn, its own folder, credential, home and the sign-in handed to
  it. Never the rest of the data folder, another project, or the home of
  whoever runs the daemon.
- **Who says where work runs:** whoever manages the factory says what a
  named place is and what runs in it, for the factory and for a project,
  and then for an agent. A word on a step (`place: <name>`, beside
  `device:`) would come last and could only add a wall, never leave one:
  otherwise a blueprint an agent wrote into its project could ask for the
  device when somebody applies it.
- **How long a place lives:** one command first. A kept place is held by
  an instance or an agent and left when that stops. Leaving is what stops
  things for certain.
- **After a restart:** a row is written before a place is entered and
  removed after it is left; a daemon that starts asks `find` of a kept
  place, and takes it up or enters it afresh.
- **Reaching the daemon:** a place shares the device's network first, so
  the daemon's address and services' ports work as now. A socket in the
  agent's folder comes later, and with it a place with no network.
- **Cards of one project** share its folder and its repository: a place
  holds nothing between them.
- **Sending a branch** from a place needs a key that can only push to that
  repository, added to the project's place.
- **Linux first.** On macOS containers run in a Linux machine where the
  device's program does not run; on Windows paths are written otherwise.
  Neither is designed here.

Events would say a place was entered, left, lost or failed
(`place.entered`, `place.left`, `place.lost`, `place.failed`), never a
value; entering, leaving and finding would be measured. No timer would be
added: a kept place that died is found at its next use.

The order of building: the description of a command with the device as
the only place; `places:` with `run` and `leave` for `shell` steps; rows
and the sweep at start; agents' turns; the sign-in from the factory's
pool; kept places; services; the socket; the word on a step, and other
systems.

## How it works

**One description of a command, and one question asked of a place.** Every
program the daemon starts would be described the same way (its program
and arguments, folder, whole environment, the folders it may see, the
ports it is reached at), and a place is asked what to start instead and
what to do before and after. That keeps a container or a virtual machine
out of the application, as an agent's program is: the package says how.

**The wall is the system's, not ours.** The application says what to show;
bubblewrap does the showing. Nothing here is a sandbox of its own making.
Every Linux system has the means, so the application asks for this one
place itself and needs nothing from a package.

**Asking is the only way.** An agent's scopes are held at every route of
the factory. A wall is what keeps its program from going around them by
reading what whoever runs the daemon can read.

**A package's declared action does not go through a place.** It runs a
product's program on the device, which is what it is for, and entering and
leaving a place are themselves such runs.
