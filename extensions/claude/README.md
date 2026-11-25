# Claude Code

Runs agents on the Claude Code command line (`claude`), in its headless mode.

```sh
mw extension add extensions/claude
mw agent add builder --adapter claude --model <one of its models>
```

`claude` must be installed and signed in on the daemon's device, as whoever
runs the daemon. What an agent is asked, and what it reads, go to that
service under that account; the application itself sends nothing anywhere.

- **Models** are the list Claude Code's own picker shows for the account,
  asked of the program. Asking sends no prompt.
- **How much an agent may do** maps to its permission modes: `read` is
  `plan`, `edit` is `acceptEdits`, `full` is `bypassPermissions`.
- **The factory's own command is not asked about.** Run for one request
  the program has nobody to ask, so a command it would ask leave for is
  refused; an agent held to `read` or `edit` could not run `mw board` to
  see its own board. Held either way it is started with
  `--allowedTools "Bash(mw *),Bash(<data>/bin/mw *)"`: that one command
  under its name and under its whole path (a rule is matched against the
  command as it is written), and nothing wider. What the agent may do
  with it is what its slot's key holds, which the factory checks. Every
  other command is as it was.
- **A conversation** is carried on with `--resume`.

## A sign-in the factory holds

An agent told to draw its sign-in (`mw agent add builder --adapter claude
--signin subscription --signin key`; `mw help signin`) is handed one of the
factory's for each piece of work, as `extension.yaml` says under `signin`:

| Kind | How the program is handed it |
|---|---|
| `key` | `ANTHROPIC_API_KEY` |
| `token` | `CLAUDE_CODE_OAUTH_TOKEN`: what `claude setup-token` prints, for a subscription |
| `folder` | `CLAUDE_CONFIG_DIR`: a folder the program was signed in with (`CLAUDE_CONFIG_DIR=<folder> claude`, then `/login`). It keeps and renews its sign-in there, and its conversations |

With a key or a token, `CLAUDE_CONFIG_DIR` is a folder of the slot's own
(`<data>/slots/<slot>/home`), so that the sign-in under the home of whoever
runs the daemon cannot be used instead. The program then also has none of
that user's settings, memory files or MCP servers. `ANTHROPIC_AUTH_TOKEN`,
`CLAUDE_CODE_USE_BEDROCK` and `CLAUDE_CODE_USE_VERTEX` are taken out of its
environment, with the variables of the kinds it was not handed.

**None of this has been run against the program.** It is written from its
documentation. To be checked by whoever first uses it: that a turn with a
folder of the slot's own and a key, or a token, starts without asking
anything; that on macOS, where the program keeps its sign-in in the
keychain, a `folder` is a sign-in of its own; and that a settings file
with an `apiKeyHelper` does not override what was handed.

## Standing instructions

What an agent is and how it stands (its name, its project and working
folder, the factory's command, what its wall shows it, the variables its
keys are under, and the standing instructions it was given) is written
before each turn into `.claude/CLAUDE.md` in its home, which the program reads
as the user's memory file (`wall.instructions` in `extension.yaml`), and the turn then
hands the program what it was asked with one line more. The file is in the
agent's own home, where of `.claude` only the sign-in is shown, and is
never written in the home of whoever runs the daemon. With a key or a token
handed, the program reads `CLAUDE_CONFIG_DIR` in place of `.claude`, and the agent is
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
only its sign-in (`.claude/.credentials.json`, and `.claude/settings.json`), where it is. Everything else the program keeps
under `.claude` (its settings, its history, its conversations) is that
home's own: it sees nothing else of that person's, and what it remembered
of a conversation held in their home before is not carried on, so each
agent begins its conversations afresh once.

## In a channel kept to its members

A channel may be kept to its members, and an agent in one has a home of its
own for it behind its wall, so that what its program keeps of what it was
asked there is found by no turn asked anywhere else
([docs/architecture.md](../../docs/architecture.md), *Identity*). An agent
that signs in as whoever runs the daemon is shown, in that home, only what
`extension.yaml` says is its sign-in (`wall.signin`): the file
`.claude/.credentials.json`, and `.claude/settings.json`, where how it signs
in may also be said. Everything else the program keeps under `.claude`, and
`.claude.json`, is then that home's own.

**None of this has been run against the program.** It is written from the
names of what it keeps on Linux. To be checked by whoever first has such an
agent in a kept channel: that it starts there and is signed in; and how it
renews its sign-in. A file that is shown is written where it is and cannot
have another file put in its place from behind the wall, so a program that
renews its sign-in by putting a new file in its place will fail to from a
kept channel's home: its turns there then fail as those of a program that
is not signed in, until it has renewed it in a turn asked anywhere else.
An agent handed a key or a token the factory holds keeps no sign-in in a
file, and is not so.

**A limit is not declared.** What the program prints when a subscription
has reached its limit, and how it says when the limit lifts, is not known
here for certain, so `signin.limit` is left out: a turn that meets a limit
fails as any failed turn does, and the sign-in is not set aside. Add it
once the wording is known ([docs/extensions.md](../../docs/extensions.md),
*`signin`*).

- **The web is asked about unless the agent's slot says otherwise.** An
  agent whose slot says `web` (`mw agent add <name> --web`, or the cell
  `web` of its row in a fleet; it takes `shell`) is started with the
  program's two tools for it allowed as well, `WebSearch` and `WebFetch`,
  each by its name alone, which is every use of it. Without that the
  program asks leave for each search and each page, and in a turn is
  refused. Held to `full` it asks about nothing, the web included,
  whatever the slot says.

## What has not been tried

The rule for the factory's own command is written from the program's
documentation and **has not been run against the program**. To be checked
by whoever first uses it:

- Held to `edit`: that an agent asked to run `mw board <project>` runs it
  and reads the answer. The documentation says a rule given with
  `--allowedTools` takes effect in `acceptEdits`.
- Held to `read`: the documentation does not say whether a rule lets a
  command that is not read-only run in `plan`. If the agent is still
  refused there, the rule did not take, and what would is another way of
  holding it to reading: `--permission-mode dontAsk` with the reading
  tools and the factory's command allowed and the editing tools denied
  (`--allowedTools "Read,Glob,Grep,Bash(mw *),…" --disallowedTools Edit
  Write NotebookEdit`). That is a change to what `read` means for this
  program, and is not made here.
- With `web` on: that an agent asked to look something up does, held to
  `edit` and held to `read`. The documentation gives a tool's name alone
  as the rule for every use of it and shows it for `WebFetch`; for
  `WebSearch` it names the tool and shows no rule of its own. A page the
  program fetches is read by the agent as anything it reads is.
- What the rule does not cover, on purpose: a command joined to another
  (`mw board shop && ls`: every part must be allowed), one under `env` or
  after a variable of its own (`X=1 mw …`), and one whose output is sent
  to a file outside what the agent may change. An agent that writes the
  command plainly is not asked; one that wraps it is refused as before.
- A data folder whose path has a space, a comma or a bracket in it: the
  rule for the whole path is then not one the program will read as meant,
  and the rule under the command's name is the one that counts.

A factory that had this package before goes on with its own copy of it:
`mw extension update claude` takes the one that came with the program.

Checked against version 2.1.321. `examples/` holds what it printed then.
