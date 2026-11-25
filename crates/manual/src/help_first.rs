//! What the command line says of the first things anybody types: starting
//! the daemon, opening the client, and the first account. Each had a line
//! of `help` and no topic, so that `help up`, the first thing asked of a
//! program just unpacked, answered that there was no help on it.

/// Help for one of these topics, or `None` if it is not one of them.
pub fn topic(name: &str) -> Option<String> {
    let cli = brand::CLI;
    Some(match name {
        "up" | "serve" => format!(
            "\
{cli} up [--listen ADDR] [--data DIR] [--profile NAME] [--note TEXT] [--walls on|off] [--workers N] [--command NAME=PROGRAM]...
{cli} serve [the same]
{cli} down

`up` starts the daemon and keeps it running: it stays where it was started,
and starts the daemon again whenever it ends without having been asked to.
So it is run in a terminal of its own, or in the background (`{cli} up &`).
`serve` is the daemon itself, once, in the foreground. `down`, from any
terminal, stops both; what the daemon hosts keeps running, and the next `up`
finds it.

The daemon says where it is as it starts: the address the client and every
command reach it at, its data folder, and which program it is. Told nothing
it listens at http://127.0.0.1:7700, on this machine only.

  --listen 127.0.0.1:7701     another address. One that other machines can see
                              (this machine's address on your network) is
                              what a second device needs (`{cli} help device`)
  --data DIR                  where the factory keeps everything, in place of
                              its usual folder
  --profile NAME              a second factory beside the first, with its own
                              data and address (`{cli} help profile`)
  --note TEXT                 why it is being started, kept with the start
  --walls off                 no wall around agents' programs (`{cli} help wall`)
  --command NAME=PROGRAM      a program that reads a prompt on standard input and
                              prints its reply, which an agent can then run on
                              (`{cli} agent add <name> --adapter NAME`): its
                              arguments after the program, `{{model}}` among them
                              standing for the agent's model; once for each program

A factory has one daemon, and one `up` that keeps it running. A second `up`
for a factory that is being served says where it is and the command that
stops it, and ends; beside an `up` whose daemon is not up just then, it
waits. An address that something else has is said to be somebody else's.
`{cli} status` says what is running, and where.

Every other command finds the daemon at `--url`, at the address this
terminal last signed in to, or at http://127.0.0.1:7700; a factory started
somewhere else is told to each command (`--url http://127.0.0.1:7701`) or
once in the environment, as `{cli} help` says in its last line."
        ),
        "open" => format!(
            "\
{cli} open [--link] [--data DIR]

Opens the client in your browser, signed in: the daemon is asked for a link
that signs you in once, within a minute, and the browser is opened at it.
Nobody registers to begin: the first start made the factory's owner, called
Unnamed until they say otherwise, and signed this terminal in. The link is asked
as whoever this terminal is signed in as; signed in as nobody, as the
owner, by proving this machine can read the factory's data folder (its
usual one, or `--data DIR`). `--link` prints the link and opens nothing.
On a device with no screen, or where no browser comes up, the address it said is
what to open. The address alone shows a page saying to run this, and a
sign-in for an account that has a password (`{cli} help register`)."
        ),
        "status" => format!(
            "\
{cli} status

Whether a daemon is running where commands look for one, which program and
version it is, and what it has: how many instances, services and agents. Of
a factory nobody has registered at yet (one an earlier program started) it
says so, with the command that makes the first account."
        ),
        "register" | "login" | "logout" | "whoami" => format!(
            "\
{cli} register [--email E --name N --password P] [--invite CODE]
{cli} login [--email E --password P] [--here [--data DIR]]
{cli} logout [--everywhere]
{cli} whoami

Nobody registers to begin: the first start of a factory makes its owner,
called Unnamed, with no email and no password, and signs this terminal in
(`{cli} open` opens the client signed in). `register` asked by that owner,
from any terminal on this machine that can read the factory's data folder,
gives the account an email, a name and a password of eight characters or
more, to sign in another way (from another device), names the workspace
made with them after them, and signs that terminal in; each can be given as
a flag instead. The email is only what the account is
known by here: nothing is sent to it, and nothing leaves this machine.

Anybody else registers with an invitation (`--invite CODE`), which
somebody who may invite makes in the client's Settings, and signs this
terminal in to the account made.

`login` signs this terminal in to an account that has a password; with
`--here`, as the factory's owner, by proving this machine can read its data
folder (its usual one, or `--data DIR`). `logout` signs it out, and with
`--everywhere` ends every session of that account. `whoami` says who is
signed in and what they may do. A factory an earlier program started that
nobody has registered at has no account to log in to: there it is
`register`, on the machine the daemon runs on.

Signing in is to one factory. What is kept is which daemon this terminal is
signed in to, and that is where its commands go until it is told another
(`--url`, or `--profile NAME` for a factory with a profile)."
        ),
        "invite" | "join" | "leave" | "external" => format!(
            "\
{cli} agent invite <name> [--project P] [--scope S]... [--expires 12h]
{cli} agent join [<invitation>] --url ADDRESS
{cli} --as <name> [--url ADDRESS] <any command>
{cli} --as <name> [--url ADDRESS] agent leave

An external agent is one this factory does not start: a program somebody
runs for themselves, here or on another machine, that works with this
command. It has a slot like any agent: a name, listed with the others, a
project, what it may ask. So what it does says `agent:<name>` did it, and a
card can be given to it. The factory starts nothing for it and gives it no
work: nothing is asked of it by a blueprint, a role or a message that names
it. What names it, and a conversation written to it, is put in its inbox
(`{cli} help inbox`), and in a channel kept to its members it is as a
person is: it reads what it is in. It takes work up itself:

  {cli} --as walls board give shop-4 walls     the card is its own
  {cli} --as walls board move shop-4 doing     and in hand, saying who moved it
  {cli} --as walls record comment shop-4 ...   and what it says is said by it

`agent invite` makes the agent, or invites one that is there again, and
prints an invitation once. Whoever invites may add agents (`agents:write`)
and gives nothing they do not hold; with no `--scope` it holds what an
agent usually does (`{cli} help agent`). The invitation lets that agent in
once, within a day unless `--expires` says (fourteen days at most). Pass it
on as you would a password: the factory keeps only a hash of it.

`agent join`, run where the agent's program runs, hands the invitation over
and is answered the agent's credential, which is never printed. Given
nothing after `join` it reads the invitation from standard input, so that
it is in no shell's history. The credential is kept under this user's
settings, in a file for that agent and that address that only this user
can read (on Windows, in the user's profile, with nothing more set on it),
apart from the session of whoever signs in on the machine: a person's
commands and the agent's never use each other's.

A command is asked as the agent with `--as <name>`, or with `{prefix}_AGENT`
in the environment. With no address said it goes to the one factory that
agent joined from this machine. Where no credential is kept for it, that is
said and nothing else is used. (`--token` and `{prefix}_TOKEN` are still a
credential given whole.)

What it holds is changed where any agent's is (`{cli} agent add <name>
--scope ...`, the Tools panel), and is so from the next thing it asks: its
credential is not replaced. `{cli} agent stop <name>` withdraws the
credential and whatever invitation waits; `agent invite` again lets it in
again, and joining replaces the credential it had, which is how a lost one
is replaced. `agent leave`, asked as the agent, gives its credential up and
forgets it here. `{cli} agent remove <name>` takes it away for good.

A message that names it waits unread: nothing here hands it messages yet.
The credential is a bearer token in a file: whoever reads the file is that
agent. (`{cli} invite [email]` is another thing: an invitation for a
person to register.)",
            prefix = brand::ENV_PREFIX
        ),
        "file" | "files" => format!(
            "\
{cli} file write <project> <path> [--from FILE]
{cli} file read <project> <path>
{cli} file remove <project> <path>
{cli} file mkdir <project> <path>

A file of a project, as the client writes, reads and deletes one. `write`
takes the file from `--from`, or else from standard input
(`{cli} file write shop docs/logo.png < logo.png`), byte for byte: a
picture arrives as it was. It makes the folders the path needs and writes
over a file that is there; a document of the project's library is changed as
it is from its page. `read` writes the file to standard output as it is
(`{cli} file read shop docs/logo.png > logo.png`), or with `--json` the
daemon's answer for a file of text. `remove` moves it to the Trash, which
says how long it is kept there; `{cli} trash restore <id>` gives it back.
`mkdir` makes an empty folder, with the folders it is in, and is refused
where anything is there under its name already.

Each is done as whoever is signed in, or as an agent with `--as <name>`:
writing, making a folder and removing take what writing a project's files takes
(`projects:write`), reading what reading them takes, and a file a table
keeps private is refused to whoever may not read that table, in the same
words for each. `{cli} project tree <project>` lists what there is."
        ),
        _ => return None,
    })
}
