//! What the command line says of a factory's data as a whole: the profile
//! a factory has, a project moved to a journal of its own, and the
//! database written out again. The rest of what it says about itself is
//! `help`, which asks here for a topic it does not have.

/// Help for one of these topics, or `None` if it is not one of them.
pub fn topic(name: &str) -> Option<String> {
    let cli = brand::CLI;
    Some(match name {
        "profile" | "profiles" | "env" => format!(
            "{cli} profiles                  the factories on this device that have a profile
{cli} up --profile NAME         start the factory of that profile
{cli} --profile NAME <command>  any command, to that factory

A factory is one daemon with its own data folder: its own database, accounts,
credentials and address. It can be given a profile, a name such as `prod` or
`dev1` (a lowercase word: letters, digits, dashes). Factories share nothing.
Before 0.1.0 a profile was called an environment: `--env`, {prefix}_ENV and
`{cli} env` are still taken, and say they are the old spelling. The word
environment is a project's own (dev, staging, prod): `help environments`.

  {cli} up --profile dev1         its data is beside the usual folder, with the name
                                added; its address is a free port on this device,
                                chosen the first time and the same from then on
  {cli} --profile dev1 register   signing in is to one factory: this account is dev1's
  {cli} --profile dev1 status     {prefix}_PROFILE=dev1, exported, says it for every command
  {cli} --profile dev1 open       its client; the page says dev1 at all times
  {cli} profiles                  each one's name, address and whether it is answering

A data folder belongs to one profile for good: started with another name it
is refused, and started with none it is what it was. To give a factory that
has no profile a name, start it once on its own data with the name, and
with the address it already has:
  {cli} up --profile prod --data <its data folder> --listen 127.0.0.1:7700
After that, `{cli} up` alone starts it as `prod`, where it listened.

A blueprint reads `profile.name` in an expression, and what it runs has
FACTORY_PROFILE; both are empty in a factory with no profile.

What differs between two factories is said once, of each, by its owner, and
read by any blueprint there as `profile.vars.<name>`:
  {cli} --profile prod apply std/profile --param 'vars={{\"api\":\"https://api.example\",\"replicas\":3}}'
                                the variables of prod, in place of whatever was set
  {cli} --profile prod apply std/profile
                                none: they are cleared
A value is text, a number, or true or false; one that was not set reads as
nothing. They are not for secrets: they are shown, and kept as they are. A
copy of a factory does not carry them.

A directory that is a project in a factory of one profile is refused to a
factory of another. That, and the rest of what keeps two profiles apart,
and what does not, is in docs/profiles.md.

Nothing is copied from one factory to another unless a person asks, and then
one way, into a factory that does not exist yet:
  {cli} --profile prod apply std/snapshot --param to=/somewhere/prod-copy
                                the owner asks prod for a copy of itself, in a folder
  {cli} up --profile dev2 --from /somewhere/prod-copy
                                a new factory, dev2, starts with what prod held
The copy has prod's accounts (and passwords), channels, tables, projects and
agents; it has none of its sessions or credentials, and nothing in it is
running. `--from` is refused where there already is a factory, without
`--profile`, and under the name the snapshot was made from. Every page of the
copy says across its top its profile, what it is a copy of and when it was
taken, until its owner says it may stop; `--banner WORDS` as a factory is
started has it say that instead, copy or not. The folder
holds everything said in every channel and people's password hashes: keep it
as you keep the factory's own data.",
            prefix = brand::ENV_PREFIX
        ),
        "apart" => format!(
            "{cli} apart [<project>] [--look] [--profile NAME | --data DIR] [--json]

Moves the projects of a stopped factory that an earlier program made to
journals of their own. A project this program adds has a file of its own
for what is kept of it, written by a conductor of its own, so that one
project with a great deal to do does not hold up another. A project from
before has neither: it stays whole in the factory's own journal until this
is asked. Named, one project is moved; with none named, every one that can
be.

What is moved is what is kept of what was run for the project: its
instances and what each was given, their runs and steps, and where each
of its handlers has got to; what is kept beside those: which texts its
trees are held to, what was vouched for them of its own files, and what
its handlers last read; what was said of it, each event under the number
it had; its tables with their records (a board's cards among them), each
record's history, and what was last seen of its files; and what its
agents were asked, in the order they were asked. What is kept and said of
it from then on is in its own journal. Nothing of it is changed: what was
running for it is taken up by its own conductor when the factory is
started, and goes on from where it was; whatever follows what is said
hears nothing twice and misses nothing; every card is where it was with
its history as it was.

A project whose journal keeps less than that (an earlier program made
it, or moved the project when less was moved) is taken the rest of the
way: `--look` says what its journal keeps and what is left.

A project is moved at rest. One with a run under way, or with a turn of
one of its agents waiting, running or answered and not yet ended, is left
where it is, and what is in the way is said by name: start the factory,
let it finish or stop it (`{cli} stop <instance>`, `{cli} agent stop
<name>`), stop the factory and ask again. Instances that are alive and
waiting to hear something are at rest.

It says first what it would move and how much room it needs beside the
database while it is done, and does nothing where there is not the room.
`--look` says that and stops. The factory's database is not made smaller
by it: `{cli} repack` does that afterwards.

The factory has to be stopped (`{cli} down`), and none can be started on
it meanwhile. A project's file is written whole beside the database and
read back, and then one write of the database decides it: cut off at any
moment, by anything, a project is as it was or it is moved, never between,
and asking again goes on with those that were not. What a cut left beside
the database is taken away when it is asked again, or by the daemon as it
next starts. The daemon that next starts says what was done
(`system.moved_apart`).

It cannot be gone back on: an earlier program refuses the journal as this
one keeps it, and nothing puts a project back. Take a copy of the factory
first: the data folder copied while the factory is stopped, or
`{cli} apply std/snapshot --param to=<folder>` (`{cli} help profile`)."
        ),
        "repack" => format!(
            "{cli} repack [--look] [--profile NAME | --data DIR] [--json]

Writes a stopped factory's database out again as a database is made now:
with pages of half the size, and events the journal numbers itself. Both
are decided when a database is made, and both are what every write costs
the disk afterwards; a factory made by an earlier program goes on exactly
as it was, and writes about twice what a new one does, until this is asked.
Nothing it holds is changed: every event, with its number, and every row.

It says first how big the database is, about how long it takes (a second
for every forty megabytes or so) and how much room it needs beside it
while it is done (as much again as the database takes, and a tenth), and
does nothing where there is not the room. `--look` says that and stops.

The factory has to be stopped (`{cli} down`), and none can be started on
it meanwhile. The database is written out beside the one there is and put
in its place in one move, once all of it is there and was read back: cut
off at any moment, by anything, the database is as it was, and asking
again begins again; what it had written beside the database by then is
taken away when it is asked again, or by the daemon as it next starts.
The daemon that next starts says it was done (`system.repacked`). A
factory made now has nothing to do.

Neither this nor the upgrade before it can be gone back on. The first
start of this program on a factory an earlier one made changes how its
journal is kept, and the earlier program refuses a journal that is newer
than it knows. To be able to go back, take a copy of the factory first:
the data folder copied while the factory is stopped, or
`{cli} apply std/snapshot --param to=<folder>` (`{cli} help profile`).

The search index is a file of its own beside the database (`search.db`),
and is left as it is. It is a small part of what a factory writes, and it
keeps nothing that is not read again from the projects' files, what was
said and the cards. To have it made as one is made now too, take the file
away while the factory is stopped (with `search.db-wal` and `search.db-shm`
where they are there): the daemon makes it again as it starts, reading
everything once, and what is searched for meanwhile is found as it is read."
        ),
        _ => return None,
    })
}
