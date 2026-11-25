# Codex

Runs agents on the Codex command line (`codex exec --json`).

```sh
mw extension add extensions/codex
mw agent add builder --adapter codex
```

`codex` must be installed and signed in on the daemon's device, as whoever
runs the daemon. What an agent is asked, and what it reads, go to that
service under that account; the application itself sends nothing anywhere.

- **Models** are the list Codex itself offers for the account, asked of the
  program (`codex app-server`, marked experimental by its makers). Asking
  sends no prompt.
- **How much an agent may do** maps to its sandbox: `read` is `read-only`,
  `edit` is `workspace-write` with the network allowed, `full` is no sandbox
  at all. The network is allowed in `edit` because an agent acts on the
  fleet with the command-line client, which talks to the daemon.
- **Standing instructions** have no place of their own, so a conversation
  opens with them.
- **A conversation** is carried on with `codex exec resume`.

## A sign-in the factory holds

An agent told to draw its sign-in (`mw agent add builder --adapter codex
--signin subscription --signin key`; `mw help signin`) is handed one of the
factory's for each piece of work, as `extension.yaml` says under `signin`:

| Kind | How the program is handed it |
|---|---|
| `key` | `CODEX_API_KEY`, which `codex exec` reads |
| `folder` | `CODEX_HOME`: a folder the program was signed in with (`CODEX_HOME=<folder> codex login`). It keeps and renews its sign-in there, and its conversations and settings |

There is no `token`: the program has no long-lived token of a subscription
to be handed. With a key, `CODEX_HOME` is a folder of the slot's own
(`<data>/slots/<slot>/home`), so that the sign-in under the home of whoever
runs the daemon cannot be used instead; the program then also has none of
that user's `config.toml`.

**None of this has been run against the program.** It is written from its
documentation. To be checked by whoever first uses it: that `codex exec`
with `CODEX_API_KEY` and an empty `CODEX_HOME` starts and uses the key; and
that the program keeps its sign-in in a file in `CODEX_HOME` and not in the
system's keyring (its `cli_auth_credentials_store` setting), or a `folder`
is not a sign-in of its own.

## Standing instructions

What an agent is and how it stands (its name, its project and working
folder, the factory's command, what its wall shows it, the variables its
keys are under, and the standing instructions it was given) is written
before each turn into `.codex/AGENTS.md` in its home, which the program reads
as its global guidance (`wall.instructions` in `extension.yaml`), and the turn then
hands the program what it was asked with one line more. The file is in the
agent's own home, where of `.codex` only the sign-in is shown, and is
never written in the home of whoever runs the daemon. With a key
handed, the program reads `CODEX_HOME` in place of `.codex`, and the agent is
told all of it ahead of what it is asked, as before.

**None of this has been run against the program.** It is written from its
documentation. To be checked by whoever first tries it (`docs/extensions.md`,
*Whether a program reads `instructions`*): that the
agent does as standing instructions it cannot miss say, and says them when
asked what its instructions are.

## A home of its own

Behind a wall every turn of an agent is in a home of the agent's own
(`<data>/slots/<agent>/home`), and one that signs in as whoever runs the
daemon is shown there, of what the program keeps in that person's home,
only its sign-in (`.codex/auth.json`, `.codex/config.toml` and `.codex/installation_id`), where it is. Everything else the program keeps
under `.codex` (its settings, its history, its conversations) is that
home's own: it sees nothing else of that person's, and what it remembered
of a conversation held in their home before is not carried on, so each
agent begins its conversations afresh once.

## In a channel kept to its members

A channel may be kept to its members, and an agent in one has a home of its
own for it behind its wall, so that what its program keeps of what it was
asked there is found by no turn asked anywhere else
([docs/architecture.md](../../docs/architecture.md), *Identity*). An agent
that signs in as whoever runs the daemon is shown, in that home, only what
`extension.yaml` says is its sign-in (`wall.signin`): the files
`.codex/auth.json`, `.codex/config.toml` and `.codex/installation_id`.
Everything else the program keeps under `.codex` is then that home's own.

**None of this has been run against the program.** It is written from the
names of what it keeps on Linux. To be checked by whoever first has such an
agent in a kept channel: that it starts there and is signed in; and how it
renews its sign-in. A file that is shown is written where it is and cannot
have another file put in its place from behind the wall, so a program that
renews its sign-in by putting a new file in its place will fail to from a
kept channel's home: its turns there then fail as those of a program that
is not signed in, until it has renewed it in a turn asked anywhere else.
An agent handed a key the factory holds keeps no sign-in in a file, and is
not so.

**A limit is not declared.** What the program prints when a subscription
has reached its limit, and how it says when the limit lifts, is not known
here for certain, so `signin.limit` is left out: a turn that meets a limit
fails as any failed turn does, and the sign-in is not set aside. Add it
once the wording is known ([docs/extensions.md](../../docs/extensions.md),
*`signin`*).

Checked against version 0.145.0. `examples/` holds what it printed then.
