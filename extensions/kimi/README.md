# Kimi Code

Runs agents on the Kimi Code command line (`kimi --prompt … --output-format stream-json`).

```sh
mw extension add extensions/kimi
mw agent add builder --adapter kimi --permission full
```

`kimi` must be installed and signed in (`kimi login`) on the daemon's
device, as whoever runs the daemon. What an agent is asked, and what it
reads, go to that service under that account; the application itself sends
nothing anywhere.

- **How much an agent may do** cannot be limited. Run for one request, the
  program asks nobody's leave for anything, and its own modes (`--yolo`,
  `--auto`, `--plan`) cannot be combined with running that way. So this
  package says it can only be held to `full`, and an agent on it has to be
  given `full` on purpose: one meant only to look is refused, not let loose.
- **Models** are the ones the program is set up with, asked of it
  (`kimi provider list --json`), and the one it uses when none is named is
  the one its plain listing calls its default. It does not say, in what it
  prints for a turn, which model answered; an agent's messages are put down
  to the one chosen for it, or to that default.
- **Standing instructions** have no place of their own, so a conversation
  opens with them.
- **What it is asked** is given as an argument, because the program takes
  it no other way. It can be seen in the process list while it runs, and a
  very long conversation may be more than one argument can hold.
- **A conversation** is carried on with `--session`, from the id in the
  last line the program prints.
- **A sign-in the factory holds** cannot be handed to it: this package
  does not say how (`signin` in `extension.yaml`), because how the program
  is given a key in its environment, or pointed at another folder, is not
  known here for certain. An agent on it that is told to draw a sign-in is
  refused, and one that is not signs in as whoever runs the daemon.

- **A home of its own.** Behind its wall every turn of an agent is in a
  home of the agent's own (its ordinary one, and one for each channel kept
  to its members it is in), and it is shown in each only what
  `extension.yaml` says is its sign-in (`wall.signin`): the folders
  `.kimi-code/credentials` and `.kimi-code/oauth`, and the files
  `.kimi-code/config.toml`, `.kimi-code/device_id` and `.kimi-code/region`.
  Its sessions and their history are then that home's own, so a session it
  held in the home of whoever runs the daemon before is not carried on, and
  each conversation is begun afresh once. Written from the names of what the
  program keeps on Linux, and not run against it: to be checked by whoever
  first has such an agent behind a wall, that it starts there and is signed
  in.
- **Standing instructions.** This package names no file the program reads
  as its own standing instructions in every project, since which that is
  is not known here for certain: an agent on it is told what it is and how
  it stands ahead of what it is asked.

## What has been checked

Checked against version 2.5.3, signed in. `examples/` holds what it printed
then: a turn that uses a tool and answers, and what it prints when nobody
is signed in. Seen working through the daemon as well: an agent on it
answered in a channel, and its second answer carried on the same session.
