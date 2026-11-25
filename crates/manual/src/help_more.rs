//! What the command line says of things that each have a file of their
//! own in the command line: a sandbox test, what a card cost, groups
//! of people and agents, and a project's feature flags.

/// What `help sandbox` says.
pub fn sandbox() -> String {
    let cli = brand::CLI;
    format!(
        "{cli} sandbox <file|name> [--project P] [--param k=v]... [--first NAME]...
        [--say TOPIC k=v...]... [--do ACTION k=v...]... [--advance 5m]...
        [--tell ACTION k=v...]... [--json]

Runs a blueprint in a sandbox and says what happened: a sandbox test. The
sandbox is a factory of its own, made for this and of nothing of the one
in use: a journal in memory, a clock only it moves, and one made-up
project called `sandbox`. The text is applied there, what is asked after
it is done in the order it is asked, and the sandbox is ended.

  {cli} sandbox std/board
  {cli} sandbox std/fleet --first std/board --do board.add title=\"Try it\" status=ready
  {cli} sandbox blueprints/release.{ext} --project shop --say release.asked version=2 --advance 10m

What a blueprint does to the factory's own tables is done for real, in
the sandbox's: a table declared, a card added and moved, a row, a channel
and what is said in it. Everything that reaches outside them is stood in:
a command (`shell`, a service, a router), an agent asked or brought up, a
repository, a project's file read or written, another device, a copy of
the factory. A stand-in runs nothing. It writes down what it was asked and
answers at once: what `--tell` said its action answers each time
(`--tell agent.task available=true decision=done`), else what its action
says a stand-in answers (a command ends with 0), else nothing. So a step
that asks an agent is answered that nobody was there, unless told.

  --first NAME     apply that blueprint first: what the text needs to be
                   there before it, such as a board before a fleet
  --say TOPIC ...  say an event, carrying what follows as `key=value`
  --do ACTION ...  do something to the tables by hand, as a made-up person:
                   board.add, board.move, board.update, record.add,
                   record.update, record.remove, record.comment,
                   channel.ensure or channel.post, each with what the
                   action of that name takes
  --advance 5m     move the sandbox's clock on: a `wait` goes on, a timer fires
  --project P      read P's own blueprints, so the text may call them (and
                   find a name among them); nothing else of P is in a sandbox

A file is read where it is; anything else is a blueprint's name, found as
an apply finds it. A text that is told which `project` it is for is told
`sandbox`. What is printed is what happened in the order it came about:
each run, its steps by what each says of itself, what was said and
started, and at the foot what was stood in. --json prints all of it as
the daemon hands it: the instances and runs as real ones are handed, every
event, the tables with their rows, the channels, and each asking of a
stand-in.

A sandbox test shows what the text does with what it is given. It does not
show what the things stood in would have done: nothing was built, nobody
was asked, nothing was sent. Passing here is not passing for real. It ends
with an error where something in it failed.",
        ext = brand::BLUEPRINT_EXT,
    )
}
/// `help cost`.
pub fn cost() -> String {
    let cli = brand::CLI;
    format!(
        "{cli} board cost <card>

What one card cost, by round. A round is each time the card was worked
and the work looked over: the turns agents took on it in that while
(working it, planning it, and each seat at its review), what each turn's
program said it took (tokens sent to it and written by it, and what that
cost) and how long it was at work, and what was decided at the round's
end, in the words of the card's history: passed, sent back with what was
said, or left for a person. Last comes what all of it came to.

Which round a turn is of is what is written of the asking it was taken
for (the event that gave the card its turn, or that asked for its
review), never when it was asked. Turns from before that say nothing of
it are put in rounds by the order they were written, which is said.

It is counted, and never scored: nothing here says that a round, an
agent or a card did well. A turn whose program said nothing of what it
took (one that has nothing to count by) says so, and is not counted as
one that took nothing. A turn asked in a channel kept to its members is
shown with its figures and not what it chose, to whoever does not read
that channel.

Nothing is kept for it: it is read from the card's turns and its history
each time it is asked, by whoever may read the board. The client shows
the same in the card's drawer. With --json, exactly what the daemon laid
out."
    )
}

/// `help docs`: the program writing out what it says of itself.
pub fn docs() -> String {
    let (cli, version) = (brand::CLI, crate::VERSION);
    format!(
        "{cli} docs export <folder>
{cli} help --json
{cli} version [--json]

The program writes out everything it says of itself, as pages of Markdown
that a library reads and a site is written from:

  {cli} docs export site/reference

writes, in that folder: every command and every topic of this help; every
keyword of the language blueprints are written in, every action and every
event, each with its sentence and, where it has one, a whole blueprint that
uses it; the summary of every blueprint that ships, with what each may be
told; what each release was; and the guides that say they are public at
their head, which the program carries in it. `index.md`, at the top, lists
and links every one of them. Each page says `public: true` and this
program's version ({version}) at its head.

No daemon is asked: the actions and events are those this program has,
worked out where it runs. Run it from an installed copy and it writes
what that copy does.

The folder is replaced as a whole, and only where an export wrote what is
in it: what one wrote is listed in it (`.exported`), and a folder holding
anything else is left as it is and the export refused. Written again with
nothing changed, nothing is written.

A blueprint does the same with the action `docs.export`, into a folder of
the project it is applied for (`to`), as whoever applied it, who holds
`projects:write`: put it in a library's folder (`std/docs`) and `std/site`
writes it out as web pages with the rest.

  {cli} help --json      every command, the ways each is written, the flags
                         it takes, the flags every command takes, and every
                         topic of this help, as values
  {cli} version          this program, and where it reads what it ships from;
                         with --json, the version as a value"
    )
}

/// What `help group` says.
pub fn group() -> String {
    let cli = brand::CLI;
    format!(
        "{cli} group list
{cli} group make <name> [--scope S]...
{cli} group add <group> <who>... | group remove <group> <who>...
{cli} group hold <group> <scope>... [--not]
{cli} group rename <group> <name> | group forget <group>

A group is people and agents together, with what its members may ask of
the factory beside what each holds itself: `{cli} group make deploys
--scope environments:secrets`, then `{cli} group add deploys grace
helper`, a person by their handle or email and an agent by its name. Each
holds what the group holds from their next request, an agent with the
credential it had. `group hold` gives the group more scopes, or with
`--not` takes them away; `group list` says who is in each, and to whoever
manages the factory what each holds.

Groups are made and changed by whoever manages the factory, and nobody
gives by one what they do not hold: a scope added to a group is held by
whoever adds it, and somebody is put in a group only by whoever holds all
it holds. An agent never changes a group it is in, nor puts itself in one
or takes itself out. A group holds no group.

Somebody who holds less by a group (taken out of it, a scope taken from
it, the group forgotten) holds less at once, and so do the keys they
made, which hold no more than they do. A key is not given more by a
group its maker is put in: it holds what it was made with. An agent taken
away leaves every group it was in."
    )
}

/// `help topic`: a forum's topics beside the channels.
pub fn topics() -> String {
    let cli = brand::CLI;
    format!(
        "{cli} topic list
{cli} topic make <title...> [--body TEXT] [--status open|standing] [--workspace W | --project P | --under <page>]
{cli} topic show <topic>
{cli} topic reply <topic> <words...> [--to <reply>]
{cli} topic answer <topic> <reply> | --none
{cli} topic move <topic> --under <page> | --top [--before <page>]
{cli} topic move <wiki> --workspace W | --project P
{cli} topic follow <topic> [--with-pages] [--often once|hourly|daily|off] [--at HH:MM] [--not]

A topic is made for one purpose and carries what makes it useful: a title,
a body written as a document, a summary of what is known now, and, where
it is a question, where it stands (open, answered, closed or standing). It
is a workspace's or a project's, as a channel is, and is named by its
title, whatever its capitals, or by its id (`@-topics-3`).

A topic has no status unless one is said: a wiki's page or a thread that
gathers what is known needs none. `make --status open` makes a question,
`--status standing` one that is kept up to date and never answered;
`{cli} record set <topic> status=open` gives one later, and `status=` with
nothing takes it away. `show` and `list` leave its column blank.

A topic may hold topics under it, to any depth: each is a page of a wiki,
which is a page with nothing above it. `make --under` makes one under a
page; `move --under` moves one there and `--top` makes it a wiki of its
own, before the page `--before` names or at the end, with every page
under it. A page is where its wiki is: a wiki is put in a workspace or a
project with every page under it, and moves only within its workspace
otherwise. `show` says where a page is and the pages under it.

What is said in it is its replies, each under the topic or under the reply
it answers (`--to`, the number `show` prints before it), nested eight deep;
a reply to one at the eighth stands beside it. `answer` marks the reply
that answered the topic, which is then answered; `--none` opens it again.
A topic with no status is answered by nothing until it is given one.

A page links another as `[words](topic:<id>)`, or one of its replies as
`topic:<id>#<reply>`; a new title or a move leaves it leading there, and
the page linked to says it is linked from this one. A source is a
footnote: `[^1]` after what it supports, and `[^1]: …` on a line of its
own, listed at the end of the page as its References.

Its title, body, summary and status are changed as any record's are:
`{cli} record set <topic> body=...`, from any door alike. Each change of
what it says is a revision of it, kept whole. Where it is and the page it
is under (`move`, `make --under`), and who follows it (`follow`), are
written by these commands and by nothing else.

Whoever follows a page has one line of it in their inbox, raised by each
change somebody else makes: a reply, a new title, body or summary, its
being answered, closed, opened again or marked standing, and its being
moved under another page, to the top or (a wiki) elsewhere. It says who
changed it last and what that did, and opens the page at the reply where
that was a reply. Following a page follows no page under it, unless it is
followed `--with-pages`: then a change of any page under it, and a page
made under it later, raises the same line, opening the page that changed.
A follower named in a reply is told by that line alone. How often you are
told, and everything else that can be followed, is `{cli} help follow`.

The first topic is made by whoever manages the factory: it sets the
topics up, applying `std/topics`, which takes `admin`."
    )
}

/// What `help follow` says.
pub fn following() -> String {
    let cli = brand::CLI;
    format!(
        "{cli} follow <kind>:<thing> [--with-pages] [--often once|hourly|daily|off] [--at HH:MM]
{cli} unfollow <kind>:<thing>
{cli} following

Following something tells you in your inbox when it changes: a page of a
wiki (`page:<page>`, by its title or id; `--with-pages` for every page under
it, one added later included), a project's board (`board:<project>`), one of
its tables or its library (`table:<project>/<table>`,
`library:<project>/<library>`), a channel (`channel:<channel>`) or a whole
project (`project:<project>`: its tables, its library, its channels' messages,
and its being renamed or moved). You follow only what you may read, and are
told only what you may read when you are told. Nothing you do yourself is
told you.

How often is yours to say, for each thing:

  --often once     at once (what nothing said means): one line for the thing,
                   raised by each change, saying what changed since you read
                   it (\"Shop board: 3 cards changed\"); a page's line says
                   who changed it last and how, as it always has
  --often hourly   one line an hour, once the hour is over, summing up what
                   changed in it; nothing changed, no line
  --often daily    the same once a day, at `--at` (09:00 where nothing is
                   said) in the zone your clock setting keeps, else UTC
  --often off      followed, and told nothing

Following what you follow already changes how, and keeps what is not said.
`{cli} following` lists everything you follow, each with how often;
`{cli} inbox` shows what each told, and a line of something followed lists
the changes it told of in the client. `{cli} topic follow` follows a page
the same way."
    )
}

/// What `help flag` says.
pub fn flag() -> String {
    let cli = brand::CLI;
    format!(
        "{cli} flag list <project>
{cli} flag set <project> <flag> <environment> <value>
{cli} flag unset <project> <flag> <environment>
{cli} flag ways <project> <flag> --environment E --blueprint B --per P --param F

A feature flag is a row of a project's `flags` table: what it is called, what
it is for, whose it is, the day it is to be taken out, and what kind of value
it says (`switch`, on or off; `percentage`, a whole number from 0 to 100; or
`word`). A project is given the table by `std/flags`, one of what it can be
set up with (`--with flags`). Rows are added as any are (`{cli} record add
shop flags name=new-checkout kind=switch until=2027-01-31`).

What a flag says in each environment is that environment's variable
`FLAG_<NAME>` (`FLAG_NEW_CHECKOUT`), which an app the project deploys reads
as it reads any setting, and a blueprint run under the environment as
`environment.vars`: nothing is fetched and nothing is sent. Where the
environment says nothing it is off. It is changed as a flag and never as a
variable by itself, and each change is a line of the flag's row's history.

Changing one takes writing the project. Turning it on, or wider (a larger
percentage, another word), where the environment's rule names who approves
(`{cli} project environment shop prod approve grace`) waits for one of them,
unless you are one of them: they are told in their inbox, and it is written
once one approves (`{cli} approve <ask>`). A key that holds less is itself,
and asks. Turning one off, or narrower, never waits.

A flag past its day is told to whose it is, once.

`ways` starts a percentage flag as a comparison on real work: the
blueprint started for each thing its param `--per` names, given two ways,
`on` for the flag's part of the things in that environment and `off` for the
rest, each handed its word as the blueprint's param `--param`. What comes
of each way is counted and never scored (`help compare`)."
    )
}
