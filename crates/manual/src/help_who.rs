//! What the command line says of who works in a factory and where: agents
//! and what they are let do, sign-ins, devices, keys, channels, the names a
//! factory answers to, and projects. The rest of what it says about itself
//! is `help`, which asks here for a topic it does not have.

/// Help for one of these topics, or `None` if it is not one of them.
pub fn topic(name: &str) -> Option<String> {
    let cli = brand::CLI;
    Some(match name {
        "agent" | "agents" => format!(
            "{cli} agent list | adapters | add <name> [...] | try <name> | stop <name> [--idle] | remove <name> [--look]
{cli} agent move <name> <project>|--none     another project to work in, and nothing else of it changed
{cli} agent invite <name> [...] | join [<invitation>] --url ADDRESS | leave     one this factory does not start

An agent lives in a slot: a name, a project to work in, channels to listen in,
and an adapter that runs it. `{cli} agent adapters` lists what this device can
run. An agent acts on the factory with this same command, signed in as its
slot, and can do only what its slot allows.

What its slot allows is what it holds, as scopes: an area (`board`), or one
thing in it (`blueprints:read`). `--scope`, said once for each, is the whole
of what it holds. Said of an agent that is there, it changes what that agent
holds and leaves the rest of it as it was, from the next thing the agent asks
of the factory, in a piece of work it has in hand too; like any `add`, it
brings in one that was stopped. Nobody gives an agent what they do not hold
themselves: one that is not held is refused by name. And an agent changes
nothing of its own slot, by this command or by its row in a fleet, even
within what it holds: a person, or another agent, sets it. With no `--scope` an
agent that is there holds what it held, and a new one what an agent usually
does: it reads and changes projects, channels, the board and records, reads
blueprints, agents and the fleet, and searches. It starts and applies no
blueprint, which setting a project up takes. This gives it that:

  {cli} agent add helper --scope projects --scope channels --scope board --scope records --scope blueprints --scope agents:read --scope fleet:read --scope search

An agent of a fleet holds what its row in the project's `agents` says, and
is changed there. In the client, what any agent holds is under its name in
the tools panel beside a channel, a line for each area, and is changed there.
What a fleet's agents run on is their row's too, the cell `adapter`: said of
one agent with `--adapter`, the row's others and any it brings up later
would not have it. `{cli} record set <project>-agents-2 adapter=<extension>`
(`{cli} record list <project> agents` lists the rows; `{cli} help extensions`).

Its program signs in to its model's provider as whoever runs the daemon,
unless it is told what to draw its sign-in from, in order, of what the
factory holds: `--signin subscription --signin key`, or a sign-in's name.
`--no-signin` goes back to the daemon's own. `{cli} help signin` has the rest.

Its program works behind a wall: of this device it is shown its work, a home
of its own, and what its agent is told it may read or change, and nothing
else. `{cli} help wall` has the rest.

`{cli} agent try <name>` starts its program as a piece of work would, behind
the same wall, and asks it which models it offers: no prompt is sent and
nothing is asked of a model. A program that answers has started, found what
it is made of and is signed in; one that does not says what it printed as it
gave up, which is usually what it could not find.

An agent this factory does not start (a program somebody runs for
themselves, with this command) is external: it is invited, joins with its
invitation, and is listed with the others as `external`. Nothing is started
for it and no work is given to it; it is given cards, and what it does says
its name. `{cli} help invite` has the rest.

`{cli} agent move <name> <project>` has it work in another project, and
`--none` in none: from its next piece of work its wall shows it that project's
folder, or a home of its own, and it is listed there. Nothing else of it
changes: one that was stopped stays stopped, and it holds what it held. It
is refused for an agent with work in hand, until that is over, and for one a
fleet counts, whose project is its fleet's; and it takes holding what the
agent holds. `add --project` says the same with the rest of it.

`{cli} agent stop <name>` stops an agent and keeps it: it is listed as
stopped, and `add` brings it in again. `{cli} agent remove <name>` takes it
away for good, and cannot be gone back on: it is a person's to do, who
manages the factory (`agents:write` and `admin`), and never an agent's.
`--look` says what it would come to and changes nothing.

  what goes     its slot, with what it was told and held; its credential,
                which is nobody's from then on; its place in every channel;
                what its program remembered of its conversations; and the
                folder that was its own on this device: the home its program
                was shown and, where it worked in no project, what it wrote
                there
  what stays    what it said and did, which names it as text: what it said in
                channels, the history of what it changed, what it was asked
                and answered, what was measured of it, and a card given to
                it, which says so until somebody gives it to another. The
                secrets and sign-ins it was handed are the factory's, and stay

It is refused for an agent with work in hand, which is stopped first or let
finish (`{cli} agent stop <name> [--idle]`): taking one away calls no work
off. And it is refused for an agent that something running counts. An agent
of a fleet is changed in its row of the project's `agents` table: lower the
row's count, or take the row away, and one that is no longer counted can be
taken away. An agent that is later called what one that was taken away was
called is another agent: the name is free, and nothing of the first is its."
        ),
        "wall" | "walls" => format!(
            "{cli} agent add <name> [--reads PATH]... [--changes PATH]... [--no-reads] [--no-changes]
{cli} agent add <name> --no-wall | --wall
{cli} agent add <name> --git off|read|commit|write
{cli} up --walls off

An agent's program is started as whoever runs the daemon. With no wall it
could read what they can: their keys, the sign-in of the person at the
client, the factory's own secret, every other project. So it is started
behind a wall, and of this device it is shown:

  the system and the folders on the path           to read
  where it works                                   to change
      its project, or a card's own worktree with the rest of the project
      to read. An agent held to `read` may change neither.
  a home of its own                                where the home of whoever runs the daemon is
  its credential and this command                  to read: it acts on the factory by asking it
  what its program signs in with                   the sign-in it draws, or what its package
      says is its sign-in in the home, where it is (`{cli} help extensions`)

It inherits nothing else of the daemon's environment. The network is the
device's: it reaches the daemon, and whatever else this device reaches.

What its program keeps of what it is asked (its settings, its history,
its conversations) is that home's own, and in a channel kept to its
members it has another home of its own for that channel, laid out the same
way, so that what it keeps of what it was asked there is found by no piece
of work asked anywhere else. Its sign-in goes with it: a key or a token the
factory holds is handed to it each time (`{cli} help signin`), and one
that signs in as whoever runs the daemon is shown, in each of its homes,
only what its package says is its sign-in (`agent.wall.signin` in its
manifest: a folder or a file of what its program keeps; `{cli} help
extensions`). Where its package names the file its program reads as its
own standing instructions, what the agent is and how it stands is written
there before each piece of work. An agent whose package says no sign-in,
which signs in as whoever runs the daemon, is shown what its program keeps
in that person's home whole, keeps what it is asked beside its sign-in,
where every agent signed in that way finds it, and is told what it is
ahead of what it is asked: it is in no kept channel, and neither is an
agent with no wall. A package installed before it said so says it once
the one that came with this installation is taken (`{cli} extension update
<name>`). `{cli} agent try <name> --channel <kept channel>` starts its
program as a piece of work asked there would, in the home it has for that
channel, and asks it nothing: whether it starts and is signed in there.

What it may do with the project's repository is said of the agent, with
`--git` (in a fleet, the `git` cell of its row):

  off      the repository is not there
  read     it is there to read (the usual): what the agent changed is
           committed for it when its work is done
  commit   it may commit its own work: in a card's worktree, on the card's
           branch and not on the project's own
  write    all of it: branches, tags, merges

Whichever it is, the repository's settings and hooks are not the agent's to
change: git runs them for whoever asks next, and the factory asks with no
wall. For the same reason the factory's own git runs no hooks at all. To
send anything to a remote an agent also needs what the remote takes: a key
of the remote's own, kept by the factory and handed to the agent
(`--secret REMOTE_TOKEN=remote-bot`; `{cli} help secret`). What that key allows
is the remote's to say, and holds whatever is said here. The fleet sends a
card's branch itself when it is told to (`push`), as whoever runs the daemon.

What else it needs is said of the agent, by whoever holds `shell`:

  {cli} agent add builder --reads ~/.rustup --reads ~/.cargo      a toolchain it builds with
  {cli} agent add builder --changes ~/.cache/sccache              a cache it may write to

A path is written from the top or from `~`, the home of whoever runs the
daemon. A program on the path that is only a link into another folder of the
home is found and does not run until that folder is added. In a folder that
is added, a file that is a credential by what it is called
(`credentials.toml`, `.netrc`, `.npmrc`, `auth.json`, a private key) is not
shown unless it is itself named. In a fleet these are cells of the agent's
row: `wall`, `reads` and `changes`.

`--no-wall` starts its program in the open, seeing this device as whoever
runs the daemon does; `--wall` gives it back. An agent that was here before
there were walls has none until it is told `--wall`; `{cli} agent list` says
which have one.

`{cli} agent try <name>` starts an agent's program behind its wall and asks
it which models it offers, sending no prompt: the quickest way to see
whether a program has what it needs there, and what it says it lacks.

A wall is held by a program of the system's (bubblewrap's `bwrap`, on
Linux). Where there is none, or the system will not let it, an agent with a
wall is not started, and says so; `{cli} status` says whether walls are held
here. `{cli} up --walls off` starts a factory that holds none at all."
        ),
        "web" => format!(
            "{cli} agent add <name> --web | --no-web

An agent's program reaches the network as this device does: a wall is about
the device's files, and does not hold the network. What keeps such a
program from the web is that it asks leave before it looks something up or
fetches a page, and in a piece of work there is nobody to give it, so it is
refused.

With `--web` it is told it need not ask: it may search the web and fetch
pages by itself, with whatever of its own does that. Which of a program's
tools that means is said by the program's package; a package that says
nothing of it leaves the program as it is.

It is off unless said. A page may say anything to an agent, and what an
agent looks for, or the address it asks for, carries whatever it puts in
it. Turning it on takes `shell`, as showing a program more of this device
does; turning it off again takes no more than changing the agent. In a
fleet it is the cell `web` of the agent's row, and `{cli} agent list` says
which agents may.

An agent held to `full` asks leave for nothing, the web included, whatever
is said here."
        ),
        "outlives" | "restart" | "restarts" => format!(
            "{cli} agent add <name> --outlives | --no-outlives

Every agent's program is started so that it could go on without the
daemon, and this says whether it does when the daemon stops, as the agent
is set then: a change made while it has something in hand holds for that.
Unless this is said its program is ended as the daemon stops, and the next
daemon asks the agent again for whatever it had in hand.

With `--outlives` its program goes on working while the daemon is down,
behind the wall it had and with the keys it was handed, and what it says is
kept in the daemon's own folder. A daemon that is started again takes the
turn up where it is, still at work or finished, and the agent is not asked
again; whatever was waiting for it has its answer. (`{cli} down --drain`
stops the daemon once what agents have in hand is done: `help down`.) It is off unless said, because
such a program works with nobody watching while the daemon is down, and
turning it on takes `shell`, as taking a wall away does; turning it off
again takes no more than changing the agent. In a fleet it is the cell
`outlives` of the agent's row, and `{cli} agent list` says which agents go
on so.

What becomes of work that was cut off, for an agent whose program does not
go on, is its stage's to say, in two cells of its row in `stages`:
`after_restart` (`again`, which is as it is unless said: the work is begun
again; or `leave`: it is left where it is for a person, and the channel
says why) and `restarts` (how many times at most a daemon is started in the
middle of the same work before it is left for a person whatever was said;
3 unless said)."
        ),
        "signin" | "signins" => format!(
            "{cli} signin list
{cli} signin add <name> --for <extension> --kind key   [--file FILE] [--at-once N]     the key, on standard input or in FILE
{cli} signin add <name> --for <extension> --kind token [--file FILE] [--at-once N]     a subscription's long-lived token
{cli} signin add <name> --for <extension> --kind folder --folder DIR [--at-once N]     where its program keeps its own sign-in
{cli} signin remove <name>
{cli} signin restore <name>            one that was set aside is good again, now

An agent's program signs in to its model's provider as whoever runs the
daemon, unless its agent is told what to draw its sign-in from. The factory
holds sign-ins for that, each for the program of one extension package:

  key      a key for the provider's API
  token    the long-lived token of a subscription; nothing renews it
  folder   a directory in which the provider's program keeps and renews its
           own sign-in. It is used where it is and never copied

A key or a token is a secret. It is given on standard input or in a file,
never as an argument; it is kept in a file only the daemon's user can read,
is in no event, log, measurement or copy of the factory, and is never shown.
`--at-once` is how many agents may hold the sign-in at once: 1 unless said.

An agent draws when told to, in order:

  {cli} agent add builder --adapter <extension> --signin subscription --signin key

`subscription` is any token or folder held for its extension, `key` any key,
and a sign-in's own name is that one. Before each piece of work the agent is
handed, of the first of those with one free, the one used longest ago; it
gives it back when the work ends. With none free it waits for one. A sign-in
whose provider says it has reached its limit is set aside until it is good
again, and the work begins again on the next. `--no-signin` goes back to the
daemon's own. Managing sign-ins takes `admin`; telling an agent to draw one
takes `secrets:use`.

Only an extension whose package says how its program is handed a sign-in
can be given one (`agent.signin` in its manifest; `{cli} help extensions`).

In a channel kept to its members an agent's program has a home of its own
for that channel (`{cli} help wall`). A key or a token is a value it is
handed and keeps nowhere, so an agent that draws one is at home in any such
channel. A folder is where its program keeps everything it is asked, beside
its sign-in, for every agent that is handed the folder: an agent that may be
handed one is in no kept channel. Tell it to draw a key or a token by its
name, and no folder."
        ),
        "device" | "devices" => format!(
            "{cli} device list | invite <name> [--expires 3d] [--at http://host:port]... | remove <name>
{cli} device join <invitation> [--allow shell,fleet] | leave

One device is the hub: it runs the blueprints. `{cli} device invite builder-1`
on the hub prints an invitation and the command to run on the other device;
from then on a step marked `device: builder-1` runs there, and what it prints
comes back. An invitation lets one device in, once, for a day unless
`--expires` says (never more than fourteen): pass it on as a password is.
`{cli} device join` with nothing after it reads the invitation from standard
input, so that it is in no shell's history.

The other device is a factory of its own first, with its daemon running and
its first account made (`{cli} up`, then `{cli} register`): joining is asked
by whoever is signed in there, and one with no account yet says so.

The member always calls the hub, so only the hub's address needs to be
reachable (start the hub with `--listen` on an address the member can see;
`--at` says where that is when the hub cannot tell). The network between
them need not be one anyone trusts: each proves a key to the other and what
crosses is sealed, so nothing on the way can read it, change it or stand in
for either. `{cli} device list` shows the short name of each one's key, to
compare with what the other side shows. A member can limit what its hub may
do on it with `--allow`.

A lone step leaves nothing behind. To host something on another device,
apply a blueprint there: `do: blueprint.apply` with `device:`."
        ),
        "key" | "keys" | "give" => format!(
            "{cli} key <name> [--scope S]... | key revoke <id> [--leave] | key roll <id>
{cli} keys
{cli} give <email> <scope> [--not]

Makes a key: a credential for a script, which it gives as `--token` or in
`{prefix}_TOKEN`. It is printed once and kept nowhere it can be read again.
`{cli} keys` lists the keys there are (yours; all of them for the factory's
owner), and `{cli} key revoke <id>` withdraws one: from then it is nobody's
credential, nor is any key made with it, and what each was being told of
stops. What they applied is stopped, as `{cli} stop` stops it; what it did
stays done. `--leave` leaves it running with what the key held, and the
answer lists it. That is of this device and of what was applied since a
key was kept with what it applied: an instance applied before then names
no key, and what a step applied on another device is that device's
(`{cli} instances` there). `{cli} key roll <id>` gives a key another
secret, printed once, and the one it had is refused from then; it is the
same key, so what it applied goes on. A key may roll itself.

A key holds no more than whoever makes it. With no `--scope` it holds what
you hold, `admin` included when you are the factory's owner; name the scopes
a script needs and it holds only those: `{cli} key ci --scope board:read
--scope search`. One you do not hold yourself is refused by name. A key made
with a key holds no more than that key.

Whoever holds a key is told only of the events its scopes let it read.

`give` gives a person a scope beyond what their role holds, such as
`environments:secrets` to somebody who looks after a project's deploys, and
`--not` takes it away. It is given by whoever manages the factory and holds
it, and nobody gives what they do not hold; the owner holds every scope
already. It is held from their next request, without signing in again.
Taken away, it goes from the keys they made too, which keep the rest of
what they held.",
            prefix = brand::ENV_PREFIX
        ),
        "channel" | "channels" | "speaker" | "post" | "read" | "thread" | "threads" => format!(
            "{cli} channel list | channel create <name> [--project P | --workspace W] | channel remove <name> [--force]
{cli} channel create <name> --kept [--alone] [--topic T] [--member WHO]...
{cli} channel keep <channel> | channel open <channel> [--name NAME]
{cli} channel archive <channel> | channel unarchive <channel>
{cli} channel rename <channel> <name> | channel describe <channel> [words...]
{cli} channel manager <channel> <who> [--not]
{cli} channel docs <channel> [add|remove <document>]
{cli} channel end <channel> <message>
{cli} channel add <channel> <who> | channel leave <channel> [--agent NAME] | channel alone <channel> on|off
{cli} channel from <conversation> [--name NAME] [--project P | --workspace W]
{cli} channel belongs <name> --project P | --workspace W
{cli} channel speaker [name]
{cli} read <channel> [--limit N] | read <channel> --thread <message>
{cli} post <channel> <message...> [--thread <message> | --top] [--attach FILE]... [--copy] [--mark plan|discuss]

`{cli} post` says something in a channel, with whatever is attached to it
(`--attach`, once for each file). `--thread <message>` says it under that
message, which is how a thread is begun and gone on with; `{cli} read`
says which messages have something under them. An agent's own post, made
while it is answering somebody in a thread of that channel, is said in
that thread without being told so, as its answer is: `--top` says it in
the channel itself. Anybody else, and an agent that is not answering
there, posts in the channel unless a thread is said.

A file attached that is one of the channel's project's files, which you
may read, is posted as that file and not as a copy: the message names
it by its project and path, and whoever reads the message reads the file
as it is then, as far as they may read that project's files. `--copy`
posts a copy of it all the same.

`--mark` says what the message is, where the channel's agents talk things
over (`std/discuss`): `plan`, a plan or proposal of an agent's, which the
others discuss before anything is done; `discuss`, a person's asking the
agents it names (two or more) to talk it over in a thread. Nothing reads
either from what a message says. `channel end <channel> <message>` ends
the discussion whose thread hangs from that message, going or waiting for
you, for whoever may write in the channel: what its agent has in hand is
called off, and it is said to have been ended by you.

A channel belongs to a project or to a workspace (`{cli} help workspace`):
one or the other, never both and never neither. It is said as the channel
is made, and one made with neither said is the own of the usual workspace,
the one that was made first; `channel belongs` changes it afterwards. It
says where the channel is listed and whose the bot is there, and nothing of
who may read it. `channel belongs <name> --nothing` is refused: a channel
is put in another project or workspace, and never taken out of all. It is
refused, too, for a project's own channel while that project's fleet is
running and posts there: stop the fleet first. A project that is forgotten
leaves its channels to the workspace it was in.

A channel may be kept to its members (`--kept`; in the client, Private):
read by whoever is in it and by nobody else, by any way of reading there
is. To everybody else it is not there: not listed, not found, and asked
for by its id it is answered as a channel that was never made. It is
called what it was made as, which another may be called too, and is named
by its id (`{cli} channel list` says it; these commands also take what it
is called, where one channel you read is called so). `--member` puts
somebody in it from the first, once for each: a person by name or as
`user:<id>`, an agent as `agent:<name>` or by its name.
  `channel keep`   keeps an open channel, by somebody who is in it. Its
                   name is free from then, and what was said in it is its
                   members' to read. Refused while an agent in it could be
                   in no kept channel (`{cli} help wall`), or is being
                   asked something there: the sentence says which.
  `channel add`    adds a person or an agent, by somebody who is in it;
                   nobody adds themselves. They read what was said so far.
  `channel leave`  takes you out; `--agent NAME` takes an agent out, with
                   what it kept of the channel. The last person in a kept
                   channel does not leave it: add somebody, or have the
                   owner open it or remove it.
  `channel alone`  says its agents are asked nowhere else (`on`), for what
                   is most closely held: an agent in it is given no work
                   anywhere but there, and is in no other kept channel.
  `channel open`   opens it again, which is the factory's owner's to do
                   and nobody else's: anybody who may read channels then
                   reads everything said in it. It takes a name among
                   those channels share, `--name` where its own is taken.
  `channel from`   opens a conversation (`{cli} help inbox`) as a channel,
                   by somebody who is in it: kept to its members as it
                   was, with everything said so far, and listed among
                   their channels from then, called `--name` or what it
                   was about. It is not undone.
`{cli} read` and `{cli} post` take a kept channel, and a conversation, by
its id.
What is not held is in docs/status.md: whoever has this device reads its
files, what was said while a channel was open was said to everybody, and
a member can repeat what they read.

`{cli} channel archive <channel>` archives a channel, by whoever may write
in it (of a kept one, one of its members, or the owner): it is read and
searched by whoever read it, as before, and nobody says anything there, a
person, an agent, the bot and a blueprint's step alike, each told that it
is archived. No agent is asked anything there, and nothing that waits in
it is counted as unread. It is refused while something that is running
says it posts there, or an agent is being asked something there: stop
that first, or wait for the agent. `channel unarchive` gives it back as
it was, with everything it held.

`{cli} channel rename <channel> <name>` calls a channel something else, by
whoever manages it: an open one by its name, which is made of the
words as any name is and is refused where another channel has it, and a
kept one, or a conversation, by what it is called, its id staying as it
was. What was said there, who is in it and where it is listed stay, and
what was written as its name in a message stays as it was written. It is
refused for an archived channel (give it back first), while something
that is running finds it by what it is called (a fleet in its project's
channel: stop that first, or it would make another of the old name), and
while an agent is being asked something in an open one.

`{cli} channel describe <channel> [words...]` says what a channel is for,
on one line shown under its head; with no words it says nothing. It and
a rename are its managers' to do: whoever made it, whoever a manager
names one (`{cli} channel manager <channel> <who>`, and `--not` for no
longer one, the last manager staying), every person in a conversation,
and whoever manages the factory. A channel made before channels had
managers has none until whoever manages the factory names one.

`{cli} channel docs <channel>` lists the documents a channel keeps to
hand, the same for everybody in it: a library's record by its id
(`shop-docs:guides:install`), or a project's file as `<project>/<path>`
(`shop/README.md`). `add` and `remove` change the list, by whoever may
write in the channel, and somebody adds only what they may read. Each is
listed as whoever looks may read it: by what it is called, or as gone, or
as kept from them. An agent asked something in the channel is told those
it may read, by what each is called and where it is, and reads them for
itself.

`{cli} channel remove <name>` takes a channel away into the Trash: its
messages go with it, what was attached to them, and what the journal and
the search index kept of them. `{cli} trash restore <id>` gives it back
whole (`{cli} help trash`); where the factory keeps nothing that is
deleted (`trash --keep 0`) it is gone at once, for good. It is the owner's
to do. It is refused while something that is running says it posts there
(a fleet in its project's channel, a bot), and says which: stop that
first, or say `--force`, and what it says there next fails.

A channel's members are people, agents and bots (`channel add`, `channel
leave`; an agent is also brought in as it is added, `agent add --channel`).
What this factory's
blueprints say in one is said by one bot, `conductor` unless it has been
called something else: a fleet reporting, a stage, a card's round. Which
blueprint it was, and which run and step, is kept on each message:
`{cli} read <channel> --json` has it as `said_by`, and in the client it is
where the bot is read about, not in the channel. A blueprint that says `as:` speaks as the bot it names, and an
agent in its own name.

`{cli} channel speaker` says what the bot is called, and `{cli} channel
speaker <name>` calls it something else, which is the owner's to do. What
was said before keeps the name it was said under. The name is the bot's
own: a new agent cannot be given it. An agent that had it before is kept,
and the two are told apart by kind (an agent, a bot).

The bot takes commands and nothing else: it has no model. A message that
names it (`@conductor board`) is one of

  status | instances | instance <id> | stop <id> | apply <name> ...
  board <project> | board columns | add | move | give | set | press | history | cost
  record list | show | add | set | comment | remove
  agent list | agent stop <name> [--idle] | agent move <name> <project>|--none
  read <channel> | post <channel> <message...> | channel rename <channel> <name>
  inbox [--all] | inbox read <line>|--all | inbox away <line>
  project environment <project> [<env> run <branch>]
  compare [list] | compare show | compare end | sandbox <name> | search <words...>
  emit <topic> [k=v]... | help

each as `{cli} help` writes it (`@conductor help` lists them whole), with or
without `{cli}` before it, and in a project's channel with the project left
out. Each is done as whoever said it, with what their key holds and never
more than the bot may ask: nobody is let do by asking the bot what `{cli}`
would refuse them. Anything else it answers once, with this list. To be understood in
ordinary words, ask an agent in the channel by its name.

In a project's channel the bot is that project's. What is running and who
is up are that project's there; another project's board is not read, its
card not moved and its agent not stopped, and an event said there says
whose it is. In a channel that is no project's it is the factory's."
        ),
        "inbox" | "to" | "conversation" | "conversations" => format!(
            "{cli} inbox [--all]
{cli} inbox --follow [--json]
{cli} inbox read <line> | inbox read --all
{cli} inbox unread <line>
{cli} inbox parts <line>
{cli} inbox pass <line> <card> [--reason TEXT] [--part N]
{cli} inbox fail <line> <card> --reason TEXT [--part N]
{cli} inbox away <line>
{cli} to <who>[,<who>...] [--subject S] <words...>

Your inbox is the box of what was addressed to you: each conversation you
are in, each message that names you with `@` in a channel you may read,
and what a blueprint told you. `{cli} inbox` lists what waits there, the
latest first, each line with its id, whether you have read it, where it was
said and by whom, and the words as you may read them now: a line of
something said keeps no words of its own. One of a channel that has been
kept from you since is not listed and not counted, as the channel is not,
and is back if you come to read the channel again. `--all` lists what you
put away too.

`{cli} inbox --follow` prints what waits unread, then stays and prints each
line as it is put in your box or told again, the oldest first and one line
each: who said it, where, and the words as `inbox` shows them. It looks for
nothing on a timer: it follows what the daemon says of your box, which only
you hear, and reads the words through your box. `--json` prints each as one
object (`id`, `kind`, `place`, `message`, `thread`, `from`, `who`, `where`,
`words`, `count`) for a program to read. Where the daemon is lost it says so
on its error stream, asks again until the daemon is back, goes on from where
it had got to and prints what was told meanwhile, once. A line told twice
before it is read is printed once, as it was told last. It ends well when it
is interrupted or asked to stop, and with why where it is refused or nothing
answers when it begins. An agent this factory does not start follows its
own the same way, `{cli} inbox --follow --as <name>`.

What a blueprint told you is from the factory's bot, in the blueprint's own
words, with what it is about where the words do not say. A fleet tells
whoever applied it when a card's agent asks, a card is left for a person, a
plan waits, or something in it fails; who else, and of what, is the rows of
its project's `tells` (`{cli} record list <project> tells`).

`{cli} inbox read <line>` says you read one, `inbox read --all` every one,
`inbox unread <line>` has one you read wait unread again, counted with the
rest, and `inbox away <line>` puts one away: it is listed no longer and waits
unread no longer, and is back if the same thing is told you again. A line
goes with the channel it was said in when that is taken away.

What a blueprint told may show more beside its words, as its parts:
addresses, and a board's cards in one column or with one label.
`{cli} inbox parts <line>` lists them, each card with the last thing said
on it. `inbox pass` and `inbox fail` pass or fail one of those cards: a
card failed goes where its part says (`ready` unless it says), with why
as what was said as it was moved, which its agent is told when it next
has it; one passed goes where its part says, or stays and that it passed
is said on it. It takes changing the board, and an agent does not pass
or fail its own work.

`{cli} to` writes a conversation: a message to one or several people and
agents, each by a handle (`nell`, `@nell`) or as an actor (`user:<id>`,
`agent:<name>`), with commas between and never a bot. It is a channel
underneath, kept to whoever wrote it and whoever it is to, and is listed
in the inbox of each of them and not among the channels: one line a
conversation, with what it is about (`--subject`, or its first words), who
is in it, the latest thing said and the id it is read and answered by,
`{cli} read <id>` and `{cli} post <id> <words...>`, as any channel is,
threads and what is attached with the rest. Each thing somebody else says
there raises your line: unread again, and back if you had put it away.
Putting it away does not leave the conversation; `{cli} channel leave
<id>` does, but for the last person in it. The same people written to
again are another conversation. Writing one takes `inbox:write` and
`channels:write`; an agent this factory starts is written to by whoever
may change agents, is in it as in any channel kept to its members
(`{cli} help channel`), and is given a turn by what is said to it. One of
its members opens it as a channel (`{cli} channel from <id>`).

A box is its owner's and nobody else's: nobody reads another's, the
factory's owner included, and a line that is not yours is one that is not
there. An agent has one too. One this factory does not start
(`{cli} help invite`) is asked nothing by a message that names it, or by
a conversation written to it: each is put in its box, which it reads as
itself, `{cli} inbox --as <name>`, and it reads and answers a
conversation it is in, and writes one, the same way (`{cli} read <id> --as
<name>`, `{cli} to <who> <words...> --as <name>`). An agent the factory
starts is given a turn by what names it, and no line: it holds no inbox
yet.

Who a word after `@` names is every account called so and every agent
called so: an account that is called what an agent, or the factory's bot,
is called is told when that one is spoken to.

Reading a box takes `inbox:read` and marking or putting away `inbox:write`.
People hold both. A key holds them where it was given them, and then reads
the box of whoever made it; a key and an agent made before there was an
inbox hold neither until they are given it (`{cli} help key`,
`{cli} help agent`).",
            cli = brand::CLI
        ),
        "ask" | "asked" | "approve" | "decline" => format!(
            "{cli} ask <record> <field=value>... [--to WHO] [--why WORDS] [--channel C [--thread M]]
{cli} asked [<ask>]
{cli} approve <ask>
{cli} decline <ask> [--why WORDS]

A change of a record you may not make yourself is asked for, and somebody
who could make it decides. `ask` makes the change there and then where you
may; where you lack only scopes (a cell that takes `shell`, a table that
takes `admin`) it is kept unmade and told, as a line of their inbox, to the
person `--to` names (an id, a name or an email) where they could make it,
and otherwise to everybody who could and may read the record. Anything else
is refused as the change would be, and so is the row an agent is brought up
by: that is a person's to change. You have twenty waiting at most.

An agent that asks while it is answering in a channel is told there, in the
thread it was asked in, what came of it; otherwise its inbox is. `--channel`
and `--thread` say where to tell you yourself.

`{cli} asked` lists what you asked for and what waits on you to decide,
each cell from what it held to what it is to hold, with what it takes and
whether you hold it. `approve` makes the change in the one write that says
it was approved: it is made as whoever asked, with every check asked of
you as things stand, and its history says who approved it. `decline` makes
nothing, with why where you say. An ask whose record was changed since is
stale, and one seven days old has expired: either makes nothing, and is
asked again. Pressed twice, either is as it was decided.

A run may wait for a person to let it go on (a step of `approval.ask`):
that is an ask too, listed by `asked` among the rest, and `approve` lets
the run go on and `decline` fails its step with why. It is decided by a
person who holds `blueprints:run` and did not start the run, or, where it
names an environment that says who approves, by one of them, whoever
started it among them only where they are named (`help environments`). It
waits as long as its step is given, and seven days where it says nothing."
        ),
        "names" | "name" => format!(
            "{cli} names | names add <name> | names remove <name>

The daemon answers a request only when it was asked at an address, at
`localhost`, or under a name it has been told is its own. A page somewhere
else can give a name of its own an address of this machine; asked under such
a name, the daemon does not answer.

Whatever is put in front of the daemon (a tunnel, a proxy, this machine's
name on a network of your own) asks it under a name too. Say that name once,
as the factory's owner: `{cli} names add box.example.net`. A blueprint that
puts the daemon behind another address says it as it does (`system.answer_as`).

Say only names that are yours. `{cli} names remove <name>` takes one back."
        ),
        "git" => format!(
            "{cli} project git <project> [--branch <branch>] [--parts]

A project whose folder is a git repository of its own, as its Git page shows
it: the branch the folder is on and how it stands against the one it follows,
what is changed and what of that is staged, the last fifty commits of what the
folder is on, or of `--branch` (whole, as it is listed: `refs/heads/dev`,
`refs/remotes/origin/dev`), its branches and its remotes, each address said
without a name and password written into it. Git is asked only to read, and
takes no lock that reading can do without. It takes reading the project.

A package may say it shows parts of a repository it serves (the pull
requests, issues and checks of the service the repository is hosted on, for
one), each read by its own program as whoever runs the daemon, signed in
there. `--parts` reads each of those now, which reaches the service; reading
one takes what using the package's actions takes (`extensions:<package>`).
What a package shows is the package's (`docs/extensions.md`, *What a package
shows of a repository*)."
        ),
        "environment" | "environments" => format!(
            "{cli} project environment <project>                    its environments
{cli} project environment <project> <env>              one, with its variables
{cli} project environment <project> <env> add          made
{cli} project environment <project> <env> remove       taken away, with its variables
{cli} project environment <project> <env> set K=V... [--agents | --not]
{cli} project environment <project> <env> unset K...
{cli} project environment <project> <env> agents K... [--not]
{cli} project environment <project> <env> deploy <blueprint>|--none
{cli} project environment <project> <env> approve <who>...|--none
{cli} project environment <project> <env> branches <branch>...|--any
{cli} project environment <project> <env> run <branch>
{cli} project branches <project> [--fetch]

An environment is a project's own: a named set of variables, such as `dev`,
`staging` and `prod` (lowercase letters, digits and `-`), one of which the
project's instances are to be applied under, so that one blueprint does the
right thing in each. Each project says its own; one may have none. It is
not the profile, which is a copy of the whole factory (`help profile`).

A variable is a name (letters, digits and `_`) and a plain value: text, a
number, or true or false; a value that reads as one of those is that, and
anything else is text. `set` sets those it names and leaves the rest.

An agent that works under an environment is handed only the variables
marked for agents, under their own names, and is shown nothing of the rest:
`set ... --agents` or `agents K...` marks them, and `--not` takes the mark
off. Nothing is marked unless somebody says so, and a name the application
sets itself (`PATH`, `HOME`, `FACTORY_...`) is never handed on. An agent
works under the environment of what brings it up, or the one said as it is
added by hand (`{cli} agent add <name> --project P --environment E`); its
program is handed the marked variables, the marked secrets as the variable
each is marked with, and the keys its slot names from that environment
first, never the factory's in place of one it lists and does not hand on.

Reading them takes reading the project. Changing them takes writing the
project, `shell` and `secrets:use`, since what is said there reaches the
commands and the secrets of what is applied. Every change is said by name,
never by value (`project.environment.*`). They go with the project into a
copy of the factory and when it is moved to a journal of its own.

A variable is not for secrets: it is shown to whoever may read the
project. An environment has secrets of its own, set by name and never
shown (`{cli} secret set <name> --project P --environment E`, `help
secret`); taking away an environment that lists any takes what setting one
takes, and their values go with it.

What is applied for a project that has environments is applied under one
of them, which is said (`{cli} apply <name> --project P --environment E`),
or it is refused naming them; a project with none runs as it always did.
What a step starts is under its parent's, and an instance runs under the
one it was applied under for its life: applied again saying another, it is
refused. It reads `environment.name` and `environment.vars.<name>` as they
are when each is worked out, its programs are told `FACTORY_ENVIRONMENT`,
and `{{ secret: stripe }}` is the environment's `stripe` where it lists one
and the factory's otherwise. What an agent applies reads, and is handed,
only what is marked for agents. An environment is not taken away while
anything runs under it.

An environment may say how it is loaded from a branch: `deploy <blueprint>`
names a blueprint, the project's own or one that ships, and `run <branch>`
applies it for the project under the environment, as you, handed the branch
as its param `branch` (and the commit as `commit`, where it declares one).
It is applied under the key `deploy-<env>` with its steps run again each
time, taking the project's own blueprints as they are now; one that does not
declare `branch` is refused. What it does is the blueprint's. What the
environment was last loaded with (the branch, the commit, who and when) is
kept with it and said with it, and how that went is its run's to say. Where
nothing was loaded so, its last run is the latest run of anything applied
under it, said as what was applied.

An environment may say who approves what goes under it: `approve <who>...`
names people, each by their name or email, and `--none` nobody. A run that
waits for a person to let it go on (`approval.ask`) and names the
environment, or runs under it, is let go on by one of them and by nobody
else, whoever started it among them only where they are named; with nobody
named, anybody who may apply lets it go on but whoever started it. Saying
who approves takes what changing its secrets takes (`environments:secrets`).
A run that waits is decided from the line it tells each of them in their
inbox, or with `{cli} approve <ask>` and `{cli} decline <ask>`.

Who approves is held where something comes to run under it: anything
started under it by somebody it does not name (Run, an apply, a step of a
pipeline that says `environment: <env>`) is refused unless it is started
by a run that holds a go-ahead one of them gave (a step of `approval.ask`
with `environment: <env>` before it). And `branches <branch>...` says
which branches may be run under it, each by its name (`main`,
`origin/main`) or a beginning ended by `*` (`release/*`), `--any` for any:
what is handed another (Run's branch, a step's `branch`) is refused.
Saying it takes what saying who approves takes.

`{cli} project branches <project>` lists the branches of the project's
repository, its own and as each remote holds them (`origin/dev`); a project
that is no repository has none. `--fetch` asks its remotes what they have
now, as whoever runs the daemon and prompting nobody, and says what came;
that is the network, and takes writing the project and `shell`, since a
repository's settings may name a program git starts to reach a remote."
        ),
        "project" | "projects" => format!(
            "{cli} project list | project tree <project> | project forget <project>
{cli} project create <name> [--path DIR] [--workspace W] [--setup NAME] [--bare | --without <name> | --with <name>]...
{cli} project trust <name> | project distrust <name>
{cli} project rename <project> <name>
{cli} project blueprints <project> [<folder> | --none | --usual | --move-in [--from <folder>]]
{cli} project outline <project> [--folder <path>] [--put <thing>... [--in <path> | --top]] [--before <thing>]
{cli} project outline <project> --rename <path> --to <name> | --remove <path> | --forget
{cli} project outline <project> --name <thing> --to <name>

`project create <name>` makes a project in the factory's own place.
`--path DIR` takes up a directory that is already there, as it is; that is
the factory's owner's to do, and never a directory that holds the factory's
own data. In the client a project lists its Files under it only where its
directory was said this way: one in the factory's own place lists none, and
its files are on its own page.

A project is added with what one starts with:

  board   where work is written up (`std/board`)
  fleet   who works the board: its two tables, `agents` and `stages`
          (the project's own fleet blueprint, which the factory keeps for it)
  docs    a library: the project's documents as pages (`std/docs`), with
          two to begin with in its `docs` folder

and, where it is asked for (`--with features`):

  features  work bigger than a card, and cards that are there already put
            in order together (`std/features`): one of the fleet says how
            it would be done and which cards wait on which, and you accept
            it. It needs the board and the fleet.

Each is a blueprint applied as you, the same as applying it by hand. What
could not be given is said, with why. `--without <name>` leaves one out, and
may be said more than once: without `docs` no document is written either,
and without `board` there is no fleet, which works one. `--bare` adds the
project alone, as it always was, and writes nothing into a directory that
was already there. A directory that came with a fleet blueprint of its own
is given the rest, and its blueprint is left for you to read and apply.

Setting a project up is a blueprint, `std/project`, whose rows are what it
starts with: what is listed above is what ships, and a factory may keep
setups of its own, saved from the dialog that adds a project, one of them
its default. `--setup NAME` sets it up as the setup the factory keeps
under that name, `--with` and `--without` naming rows of it.

`--workspace W` adds it to a workspace that is there (`{cli} help workspace`):
which projects belong together, and nothing else of the project. With none
said it is in the usual one, the workspace that was made first.

`project forget <project>` takes a project out of the factory. Its files
are left exactly where they are. What is running for it is not stopped by
it, and its tables and agents are not taken away: stop its fleet and its
board first (`{cli} instances`, `{cli} stop <id>`).

A project made of a directory that was already there is not trusted at first.
Its blueprints under names of its own are applied as any are. A file of its
own with the name of a blueprint that ships (`std/board`, or a package's) is
passed over for the one that ships, and whatever checks or applies says which
file. `{cli} project trust <name>` has its own files used instead; read them
first. `{cli} project distrust <name>` goes back. Both are the owner's, and
`project list` says which projects are not trusted.

A project's own blueprints (its forks, its fleet's, the examples put there)
are kept by the factory, in its own data folder and never in the project's:
a project is often a repository other people read. Each is named by a path
of the project's that begins `@blueprints/`, which is read and written as
any file of the project is (`{cli} file read <project> @blueprints/fleet.{ext}`).
`project blueprints <project> <folder>` has a folder of the project's
repository read as well, before the factory's place, for blueprints kept
with its code: a name with a file in both is refused until one is taken
away. `--none` has it keep none, with no forks and nothing read in place of
what ships, and `--usual` goes back to the factory's place alone. With
nothing after the project it says which. Its fleet's blueprint at its top
(`fleet.{ext}`), which a project from before was set going by, is read
whatever it says, where the factory keeps none for it. `--move-in` copies
the blueprints of the folder it says (or `--from <folder>`, or `blueprints/`
where it said none) and those at its top into the factory's place, with
what each fork was forked from, and has it keep them there alone: the
repository's files are left as they were, for you to take away. It is the
owner's to change. Where texts are followed (`{cli} help texts`) a change is
taken as a save of those files would be; otherwise what was applied goes on
with what was vouched for then, until it is applied again.

`project rename <project> <name>` calls a project something else, for
everybody who reads it: what it is called, and not its id, which
everything knows it by and which stays as it was. It is refused where
another project is called the same. It takes writing the project's files.

`project outline <project>` says how the project's things are arranged in
the side panel, for everybody who reads it: folders of the arrangement's
own, on no disk, with its board, tables, libraries and the rest put in
them. Nothing a thing is known by changes by where it is shown. A thing is
said by its key (`board`, `table:<name>`, `library:<name>`,
`mockups:<name>`, `environments`, `pipelines`, `blueprints`, `git`,
`repo`), and a folder by its names from the top (`Insights/Runs`).
`--folder <path>` makes each folder of the path that is not there; `--put`
puts the things after it in the folder `--in` says, or at the top
(`--top`), at the end or `--before` a thing or folder there; `--rename`
and `--remove` change a folder, what was in one removed taking its place;
`--name` shows a thing as something else for everybody (`--name board
--to Roadmap`), its key and what blueprints call it as they were; `--to ''`
shows it as it always was, and a table is renamed by its own title instead;
`--forget` has the panel go back to how it is with none. Whatever nobody
placed is drawn after what was, in the usual order, so a folder made from
here on a project nobody arranged stands first. It takes writing the
project's files to change.",
            ext = brand::BLUEPRINT_EXT
        ),
        "example" | "examples" => format!(
            "{cli} example list
{cli} example add <name> --project <project>

Examples are blueprints to read and to try, which came with the program.
`example list` says which there are, what each is, and the blueprints it
brings.

`example add <name>` puts one among a project's own blueprints: its files
are written where the factory keeps them for the project, as
`@blueprints/<name>/` (`{cli} help project`), by the daemon. From then they are the project's own
files like any other. Its blueprints are applied by their names
(`{cli} apply <name>/<blueprint> --project <project>`), and what they link,
the words an agent is told among it, is beside them to be read and changed,
in the client from each blueprint's page, or with
`{cli} file read <project> @blueprints/<name>/<file>`.

Asked again it writes nothing that is there as it came. A file that is there
and says something else is yours, or somebody's: then nothing at all is
written, and it says which file. Take that file away to have the example as
it came.

It takes what writing a project's files takes (`projects:write`), and
whoever asks has vouched for the blueprints as they are written, as for
any blueprint they save."
        ),
        "workspace" | "workspaces" => format!(
            "{cli} workspace list | workspace create <name> | workspace rename <workspace> <name>
{cli} workspace remove <workspace> [--to <workspace>]
{cli} workspace put <workspace> <project> | workspace take <project>
{cli} project create <name> [--workspace <workspace>]
{cli} channel create <name> --workspace <workspace> | channel belongs <name> --workspace <workspace>

A project is one folder, with its board, its fleet and its channels. A
workspace is a name for the projects that belong together: an application,
what is served for it and its scripts, say. A factory always has a
workspace, and every project is in exactly one. The first is made when the
factory's owner registers, called after them. A channel is a project's, or
a workspace's own, for talk that is of no one project of it: none belongs
to nothing.

It is grouping and nothing else. Being in a workspace changes nothing of a
project: not its id, its cards' ids, its folder, its fleet or its channels.
And it lets nobody in and keeps nobody out: whoever may read channels reads
the channels of every workspace, and whoever may read projects sees every
project.

`workspace list` says each with its projects and its own channels, by name,
and which is the usual one: the workspace made first that is still there,
where a project goes when `project create` says no `--workspace`. One with
nothing in it is listed. `workspace rename` changes what it is called and
not its id, so nothing that names it moves.

`workspace put` puts a project that is there in a workspace, out of
whichever it was in. `workspace take` is refused: a project is never in no
workspace, and leaves one by being put in another.

`workspace remove` takes a workspace away and nothing that was in it. One
with projects, or channels of its own, goes only with where they go
(`--to <workspace>`), and each is said to have moved. The last workspace
there is cannot be removed: call it something else instead.

A factory an earlier version made, where a project might be in no
workspace, is brought to this when the daemon first starts on it: a
workspace is made where there was none, called after the factory's owner,
and each project that was in none, and each channel that belonged to
nothing, is put in the usual one. Nothing else of any project or channel
changes.

In a workspace's own channel the factory's bot is the factory's; in a
project's it is that project's (`{cli} help channel`).

Making, renaming and removing one, and putting a project in another, take
what adding a project takes (`projects:write`). Removing one that holds
channels of its own moves them, and takes what saying what each belongs to
takes as well. Saying what a
channel belongs to takes what changing a channel takes (`channels:write`)."
        ),
        // The first things anybody types have a file of their own.
        _ => return crate::help_first::topic(name),
    })
}
