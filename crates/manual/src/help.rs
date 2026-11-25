//! What the command line says about itself.
//!
//! Written to be read by agents as much as by people: short, and with the
//! detail a command away rather than loaded up front.

use crate::guides::{body, guide};

pub fn overview() -> String {
    let (cli, product) = (brand::CLI, brand::DISPLAY);
    format!(
        "{product}: a software factory you run yourself.

usage: {cli} <command> [arguments] [--json]

running it
  up [--data DIR] [--listen ADDR]     start the daemon and keep it running (help workers for --workers N)
     [--note TEXT] [--profile NAME]   why, kept with its start; and which profile's factory it is (--env before 0.1.0)
     [--from FOLDER]                  with --profile: a new factory, started from a snapshot of another (help profile)
  serve | open                        the daemon itself, in the foreground | open the client in a browser, signed in
  down [--drain [--within T]]        stop the daemon, at once or once what is in hand is done (help down)
  repack [--look] | apart [<project>] [--look]     a stopped factory's database written out as one is made now | its projects from before moved to journals of their own (help repack, help apart)
  status | names [add|remove <name>]  what is running here | the names this daemon answers to besides its address
  profiles                            the factories here that have a profile, as prod and dev1 (help profile)

signing in
  register | login | logout [--everywhere] | whoami | key <name> [--scope S]... | keys | key revoke <id> [--leave] | key roll <id> | give <email> <scope> [--not] | group list|make|add|remove|hold ... | invite [<email>] [--expires 3d]   keys for scripts | more than a role holds (help key) | people and agents given scopes together (help group) | a person invited to register

projects
  project list | project trust|distrust <name> | project blueprints <name> [<folder>|--none|--usual] | project environment <project> [<env> [add|remove|set K=V...|unset K...|agents K...|deploy B|approve WHO...|branches B...|run BRANCH]] | project branches <name> [--fetch] | project git <name> [--branch B] [--parts] | flag list|set|unset|ways <project> ...     whether its own files may stand in for what ships | where they are (help project) | its environments, as dev and prod, each loaded from a branch | the branches of its repository (help environments) | its repository as it stands, with what its packages show (help git) | its feature flags in each environment (help flag)
  project create <name> [--path DIR] [--workspace W] [--setup NAME] [--bare | --without board|fleet|docs | --with features|environments|flags]...   a new project with a board, a fleet and a library, or an existing directory (the owner's to adopt; not trusted at first)
  project tree <project> | project rename <project> <name> | project outline <project> [--folder F] [--put KEY... [--in F|--top]] ... | project forget <project> | workspace list|create|rename|remove|put|take     its files | what people call it | how its things are arranged in the side panel (help project) | take it out of the factory, its files left where they are | projects that belong together (help workspace)
  fleet init <project> | file write|read|remove|mkdir <project> <path> [--from FILE]     write the project's fleet blueprint, which starts the fleet that works its board | one of its files, byte for byte from standard input, to standard output, or to the Trash, or a folder made (help file)

blueprints
  check <file>... | explain <file|name> | sandbox <file|name> | graph <file> | schema     what is wrong with a blueprint | it in plain words (help explain) | it run where nothing is real (help sandbox) | as a graph, in text | the language, for an editor (help schema)
  fmt [--check] [file|folder]...      lay blueprints out the one way; --check only says which would change (help fmt)
  apply <file|name> [--project P [--environment E]] [--param k=v]... [--label k=v]... [--key K] [--force] [--take] [--no-wait]
  instances [--all] [--limit N] | instance <id>   what is alive, the text each runs, and which are behind it | one instance, its runs and children
  texts [held|follow|usual] [--lands in-place|again] | own <blueprint> --project P [--say|--back]   how what is running comes by new blueprints (help texts) | one that ships made the project's own (help own)
  apply --instance <id> | stop <id>   apply one again as it is (it says how it took the text) | stop it and what it holds
  runs [--instance ID] | run <id> | logs <run> [step] | logs [--keep DAYS|always] [--folder DIR|usual]     a run step by step: what each was handed and gave (help play) | how long what steps printed is kept, and where (help logs)
  actions [name] | example list | example add <name> --project P     what a blueprint can `do` on this device | blueprints to read and try that came with the program, and one put among a project's own (help example)

working together
  channel list | channel create <name> [--project P | --workspace W] [--kept] | channel keep|open|add|leave|alone|from|archive|unarchive|rename|docs|end <channel> ... | channel belongs <name> ... | channel remove <name> [--force] | channel speaker [name]     (help channel)
  read <channel> [--limit N] | read <channel> --thread <message> | inbox [--all] | inbox read <line>|--all | inbox away <line> | inbox parts|pass|fail <line> ... | to <who>[,<who>...] [--subject S] <words...>     recent messages | a message and what was said under it | what was addressed to you | a conversation written to somebody (help inbox) | topic list|make|show|reply|answer|move|follow ...: a forum's topics beside the channels (help topic) | follow <kind>:<thing> [--with-pages] [--often once|hourly|daily|off] [--at HH:MM] | unfollow <kind>:<thing> | following: be told when a page, a wiki, a board, a table, a library, a channel or a project changes, and how often (help follow)
  post <channel> <message...> [--thread <message> | --top] [--attach FILE]... [--copy] | attachment <id> [--out FILE]     say it, under a message or in the channel itself; a project's file attached is posted as that file (help channel) | what is attached to a message or a record, as you may read it
  board <project> | board columns <project> [<column>...] | board add <project> <title...> [--body TEXT] [--status COLUMN] [--label L]... [--field name=value]...
  board move <card> <column> [--note TEXT]          with what is said as it is moved; from review to doing sends it back
  board give <card> <agent>|<person>|nobody | board set <card> <field=value>... | board history <card> | board press <card> <action> [--back] | board cost <card>     who works a card; what it holds; what has happened to it; what its column offers, pressed (help record); what each round of it cost (help cost)
  record list <project> <collection> | add ... | show <record> | set ... | remove <record> | title <project> <collection> <words>|--usual | trash [restore <id> [--beside] | forget <id>] [--keep DAYS|always]     what else a project keeps, and what a table is called for people (help record) | what was deleted, given back or deleted for good, and how long it is kept (help trash)
  record comment <record> <words...> | attach <record> <file>... | attachments <record> | detach <record> <id> | ask <record> <field=value>... [--to WHO] [--why W] | asked [<ask>] | approve <ask> | decline <ask> [--why W]     say something on a record or a card; on a card that asks something, the answer | what is attached to it (help record) | a change you may not make, asked of whoever could (help ask)
  agent list | agent adapters | agent try <name> [--channel C] | agent stop <name> [--idle] | agent remove <name> [--look] | agent move <name> <project>|--none | agent invite <name> | agent join --url U     try: start its program where it will work, and ask it nothing; --idle: when it has finished what it has in hand; remove: take it away for good (help agent); invite, join: an agent this factory does not start, asked as with --as <name> (help invite)
  agent add <name> [--adapter A] [--project P [--environment E]] [--channel C] [--model M] [--permission read|edit|full] [--scope S]... [--signin S]... [--secret VAR=NAME]... [--role R] [--no-wall] [--outlives] [--web] (help agent, wall, outlives, web)
  signin list | add <name> ... | remove <name> | restore <name>     what agents' programs sign in with (help signin)
  extension list | add <folder> | update <name>|--all | remove <name>     what agents can run on, and blueprints for other products
  extension models <name> [--fresh]                 the models it offers

finding and measuring
  search <words...> [--kind KIND] [--project P]     everything you may read: topic, document, record, file, message, card, blueprint, project, channel, person, agent, run
  events [--topics PATTERNS] [--limit N] [--follow] | events --kinds [topic] | emit <topic> [k=v]...   what happened | what can | say one
  metrics <name> [--since 1h] [--by label,label] [--label k=v]... [--project <id>] | metrics --labels   measurements | the labels blueprints gave, to split and narrow by
  compare [list] | start <name> --of <blueprint> --way <way>... | show | share | rule | end <name>   one blueprint started several ways, counted side by side (help compare)
  services | secret list | set <name> | remove <name> | agents <name> VAR|--not [--project P --environment E] | caller list | set <name> ... | remove <name>   hosted | handed to programs (help secret) | let in by signature (help caller)

more than one device
  device list                         this device's fleet, and its hub if it has one
  device invite <name>                on the hub: an invitation for a device, with its join command
  device join <invitation>            on the device: join that hub's fleet
  device leave | device remove <name>

  help blueprints                     how blueprints are written
  help views                          how to show a chart or a page in a channel
  help stores                         keeping a table in a database that a blueprint hosts
  help extensions                     how agents other than the built-in one, and other products, are added
  help <command> | help --json | version | docs export <folder>     this, for one command | every command, its flags and topics | this program | all of this written out as pages (help docs)

Add --json to any command for exactly what the daemon answered.
The daemon is found at --url, ${prefix}_URL, the saved session, or http://127.0.0.1:7700; or, by its profile, with --profile NAME.",
        prefix = brand::ENV_PREFIX
    )
}

/// Help for one topic, or `None` if there is no such topic.
pub fn topic(name: &str) -> Option<String> {
    let cli = brand::CLI;
    Some(match name {
        // The blueprint reference, how to show a page in a channel, and
        // keeping a table in a database a blueprint hosts: the guides of
        // those names, as they are written out.
        "blueprints" | "blueprint" => body(guide("blueprints")).to_string(),
        "views" | "view" => body(guide("views")).to_string(),
        "stores" | "store" => body(guide("stores")).to_string(),
        "docs" | "version" => crate::help_more::docs(),
        "topic" | "topics" => crate::help_more::topics(),
        "follow" | "unfollow" | "following" => crate::help_more::following(),
        "extensions" | "extension" => format!(
            "{cli} extension list | add <folder> | update <name>|--all | remove <name> | models <name> [--fresh]

Out of the box an agent runs on the built-in assistant, which needs no model
and understands only a few requests. A real agent is somebody's command-line
program, and this product does not know any of them: each comes as an
extension package, a folder with an `extension.yaml` that says which program
it is, how a turn is put to it, how what it prints is read, and how to ask it
which models it offers.

  {cli} extension add <folder>    install a package (it is checked first)
  {cli} extension add <name>      install one that came with this installation; over one of
                                that name, it replaces it and keeps the old copy beside it
  {cli} extension list            what is installed, whether its program is here, and which are
                                not the ones that came with this installation
  {cli} extension update <name>   take the one that came with this installation in place of the
                                installed copy of that name, and say where the two differed
  {cli} extension update --all    the same for every installed package that is not the one that came
  {cli} extension models <name>   the models it offers, as the program itself lists them
  {cli} extension remove <name>

A package is installed as a copy, and an installation that is upgraded goes on
with the copy: what a newer program's package says is not said here until the
one that came is taken. Nothing takes it unasked, since the copy may be one
its owner changed, and nothing tells such a copy from an earlier one: `list`
says only that the two differ. `update` replaces the copy whatever was changed
in it, and keeps it beside the new one (`extension.was.yaml`).

Then `{cli} agent add <name> --adapter <extension> [--model <id>]`, or pick
both when bringing an agent into a channel. The program must be installed and
signed in on the daemon's device, as whoever runs the daemon.

An agent of a fleet runs on what its row of the project's `agents` says, in
the cell `adapter`, and is changed there:

  {cli} record list <project> agents                       the rows, each by its id
  {cli} record set <project>-agents-2 adapter=<extension>  the agents of that row, from their next piece of work

A row that says none has its agents run on what could run an agent when
they were first brought up: a package that was installed with its program
there, else the built-in assistant. Installing a package later does not
move a fleet that is up, so its rows are told.

A package may bring blueprints as well as, or instead of, an agent: how
somebody's product is set up. While it is installed they are applied by name,
`{cli} apply <extension>/<name>`, and `{cli} extension list` says which there
are. One of them has no more authority than whoever applies it.

It may bring actions too: each a way its program is run, which a blueprint
does as `<extension>.<name>`. The program is run directly, with no shell, and
doing one takes `extensions:<extension>` rather than `shell`. `{cli} extension
list` says which there are, and `{cli} actions <extension>.<name>` shows one.

Some packages are kept with the source, in `extensions/`. Writing one is
described in docs/extensions.md."
        ),
        "record" | "records" => format!(
            "{cli} record list <project> <collection>
{cli} record add <project> <collection> <field=value>...
{cli} record show <record>
{cli} record set <record> <field=value>...
{cli} record comment <record> [--public] [--on <quote> [--before W] [--after W] [--under H]...] <words...>
{cli} record attach <record> <file>...
{cli} record attachments <record>
{cli} record detach <record> <id>
{cli} attachment <id> [--out <file>]
{cli} record remove <record>
{cli} record title <project> <collection> <words...> | --usual

A project keeps what its blueprints declare with `collection.ensure`: its
board, and whatever else, such as its sprints. These are the records of
anything but the board; a board's cards are read here and written with
`{cli} board`, because moving a card is an event of its own. What a card
holds is set with `{cli} board set <card> <field=value>...`, in the words
`set` takes here; where it is and whose it is are `board move` and
`board give`, which takes an agent by its name, a person by their handle
or email, or `nobody`.

What a card's column offers to be pressed (a row of a fleet's `actions`:
`Signed`, say, which a person presses before a card in ready is worked)
is pressed with `{cli} board press <card> <action>`, by its name, as
whoever this terminal is signed in as, and taken back with `--back` while
the card is where it was pressed. Who may is what the row says; one that
holds keeps the card where it is until it is pressed.

A table is called something for people with `title`, for everybody who
opens it: what its blueprint calls it (`title` of `collection.ensure`)
until somebody calls it something else, and `--usual` goes back to that,
or to its name. Its name is what blueprints, scripts and its address
know it by, and never changes. It takes writing the project's files, and
changing the table; a board, a library and mockups keep their words.

Something is said on any record, a card included, with `comment`: it is
kept in the record's history (`{cli} board history <card>` for a card) and
changes nothing of the record. On a card whose agent asked something
instead of guessing, what someone else says is the answer, and the card
goes round again with it. What is said is private unless it is said with
`--public`, which marks it: the mark is kept and shown in the history, and
is what a blueprint that keeps the record somewhere outside the factory as
well goes by, to tell nobody there what was not marked. Words after `--`
are words, whatever they look like. `--on <quote>` says the comment is
about those words of a document and not the whole of it, with `--before`
and `--after` the few words either side of them and `--under` each heading
over them, the outermost first: the document's page marks them where it
finds them, and where they are no longer there, says what it was about.

A file is attached to any record, a card included, with `attach`, by
whoever may change it: a screenshot, a short video or a log that shows
what was done, or what is wrong. Each is a line of its history where its
table keeps one, `attachments` lists them, `detach` takes one away, and
`{cli} attachment <id>` writes one out (`--out <file>`, else standard
output), for whoever may read the record; the same reads what is attached
to a message, for whoever may read the channel. An agent working a card
is told what is attached to it, by name and size, with that command.

  {cli} record add shop sprints title=November size=5
  {cli} record set shop-sprints-1 size=8 goals=ship,rest
  {cli} record set shop-sprints-1 size=          `field=` with nothing after it clears the field
  {cli} record set shop-sprints-1 'goals+=Look at it, then say'

A record is named by its id, `<project>-<collection>-<number>`, which `add`
and `list` print. Each `field=value` is made what the field keeps: a number
for a number, `true` or `false` for a flag, the words between commas for a
list, and the words as they are for anything else, an id for a field that
names a record of another collection. `field+=value` adds one thing to a
list, as it is written, commas and all: after what the same command said of
the field, and where it said nothing, after what the record holds (a card's
`labels+=` in `board set` too). What is written is checked as the
collection declares it, and what is wrong with it is said. `--json` gives a
record as the daemon keeps it."
        ),
        "apply" => format!(
            "{cli} apply <file|name> [--project P] [--param k=v]... [--label k=v]... [--key K] [--force] [--take | --back [--say]] [--no-wait]

Starts an instance of a blueprint, or updates the one that already exists under
the same key. Given a file, its text is sent; given a name, it is looked up in
the project's blueprints and then the standard ones.

  --project P     the project it belongs to (default: ${prefix}_PROJECT)
  --param k=v     a param; the value is read as JSON if it parses, else as text,
                  so a map is given as `--param 'vars={{\"api\":\"https://api.example\"}}'`
  --label k=v     a label that everything measured under the instance carries,
                  as `labels:` gives to what a blueprint calls: `--label variant=b`
  --key K         the instance's name (default: the blueprint's); a different
                  key starts a second instance beside the first
  --force         run the steps again even if nothing changed
  --take          take the blueprints it reaches as they are now (below)
  --no-wait       return as soon as it has started

A blueprint runs as whoever applied it, and the other blueprints it calls are
files of the project that others may have changed since: an agent, a merge. So
what is running goes on with those as they were when you last applied, and
applying again takes up the one you name and leaves the rest as they were,
saying which have changed. `--take` takes them all as they now are: read what
changed first, since from then it runs as you.

The blueprints that ship are held the same way for another reason: a new
version of this program has new texts under the old names, and what is running
was written for the old ones. So an instance goes on with each blueprint that
ships as it was when you applied it, and so does everything it starts, whatever
program is running it. Applying again says which have changed and leaves them;
`{cli} instances` says what changed in each, by the version each says first
(`std/lane-round 3 to 4: the step `work` is changed`). `--take` moves it to what
ships now, and is refused with what does not fit where what you apply was not
written for them.

  --back          put it back on the blueprints it had before it was last moved
                  with `--take`. It says what it put back, and is refused where
                  the text you apply does not fit them (a file of your own that
                  you changed for the new ones: change it back first). The
                  texts are kept, so this needs no earlier program
  --back --say    only say what that would put back

{cli} apply --instance <id> [--force] [--take] [--no-wait]

Applies an instance that is running again as it is: the text now there for its
blueprint, with the params, the key and the place it was started with. It is
for one that `{cli} instances` shows as behind and that something else started
(a lane, started when a card became ready), which nobody can apply by name. It
is an apply like any other, and says which of three things happened. If the
text now there says how one that is running takes it (`on_update`), it is
carried over, with that done in place of its steps. If the text differs only in
what it does later (its handlers), it takes the text where it is and nothing is
run again. Otherwise its steps run again; `--force` runs them whatever the text
says.

Exits 1 if the run fails.",
            prefix = brand::ENV_PREFIX
        ),
        "check" => format!(
            "{cli} check <file>...

Checks blueprints against everything this device can do: syntax, the step
graph, what expressions read, the actions and their arguments, permissions.
Problems are printed as `file:line:col: error: message`. Exits 1 if any file
has an error. Without a daemon to ask, only what can be known from the file
alone is checked, and it says so.

A file called as YAML (`<name>.{ext}.yml`) is one an editor reads as YAML,
and this reader takes a few things written plain that YAML does not: a
value with `: ` in it, a `${{ }}` bare inside braces. Each is a warning
there, with its line; `{cli} fmt` puts it in quotes, where it is the same
to both. A file called `.{ext}` is told nothing of it.",
            ext = brand::BLUEPRINT_EXT,
        ),
        "sandbox" => crate::help_more::sandbox(),
        "cost" | "costs" | "spent" => crate::help_more::cost(),
        "group" | "groups" => crate::help_more::group(),
        "flag" | "flags" => crate::help_more::flag(),
        "explain" => format!(
            "{cli} explain <file|name> [--project P] [--json]

Says a blueprint in plain words, for somebody who does not write
blueprints: what it is and what whoever applies it may tell it; what it
makes (a table, a channel, an agent, a service, another blueprint started
under it); what it does when it is applied, step by step; what it hears
and what it then does; what it may do; and what it does as it stops,
fails or takes a new text.

  {cli} explain std/board
  {cli} explain blueprints/release.{ext} --project shop

A file is read where it is. Anything else is a blueprint's name, found as
an apply finds it: one of the project's own with --project, then a shared
one, then one that ships. Nothing is run and nothing is written, and the
blueprint need not be applied anywhere.

A blueprint that ships says last how what it does is changed: by telling
it something else, by the rows of a table it makes, and by forking it
(`{cli} help own`). A project's fork of one, asked for with --project by
its name or as the file it is, says first what it was forked from, then
where it differs from that and how the one that ships has moved since,
and last how to go back.

It is worked out from the text and from what each action and each event
says of itself, with no model asked. A step is said by its `description`,
as far as its first full stop, and one that says nothing by what its
action does. After a line, in brackets, is what the text calls what the
line is about: a step's key, a handler's name, a scope. Under *What it may
do*, a line that begins with `!` says that a command is run or a secret
is handed to one: read those before applying a blueprint somebody else
wrote.

A blueprint that does not check has no summary: what is wrong with it is
printed as `{cli} check` prints it, and the command exits 1. --json
prints the summary as the daemon hands it out. Without a daemon to ask, a
file is said from its text alone, with no action known but those of the
language, and it says so.",
            ext = brand::BLUEPRINT_EXT,
        ),
        "schema" => format!(
            "{cli} schema

Prints the blueprint language as a schema (a JSON Schema), for the editors
people already have: the words there are and what goes after each, with
the sentence that says what each is for, and the actions there are on this
device with what each takes. A blueprint is written as YAML, so an editor
that is given the schema completes one as it is written and marks what
does not fit.

  {cli} schema > blueprint.schema.json

A blueprint's file may be called `<name>.{ext}` or, so that an editor takes
it for YAML with nobody telling it, `<name>.{ext}.yml`: the same blueprint
either way, and never two files for one name. Which schema a file is held
to is said to an editor by a first line of the file, which `{cli} fmt`
keeps where it is, or once for a project in the editor's own settings, by
mapping `*.{ext}` and `*.{ext}.yml` to the schema's file. The lines to write
are in `{cli} help blueprints`, under *In an editor of your own*. `{cli} fmt`
writes a blueprint so that an editor reads all of it.

The schema helps whoever is writing. It says less than `{cli} check`,
which is what decides: whatever may be worked out (`${{ ... }}`) is let be
text, a name is not held to what there is, and what the reader refuses
that YAML allows (an anchor, a tag, a second document) no schema can say.
With no daemon to ask it prints the language's words and only the actions
that are part of the language, and says so.",
            ext = brand::BLUEPRINT_EXT,
        ),
        "workers" | "worker" | "threads" => format!(
            "{cli} up --workers N        {cli} serve --workers N        {env}=N

How many threads the daemon's work is done on. Told nothing, it is one for
each processor of the device, and eight at most: nearly all a daemon does
is wait (on its database, which has a thread of its own; on the programs
it started; on the network), and a thread that waits for work is a stack
and a share of memory all the same. Say more for a device with many
processors and a great deal going on at once; say fewer to hold it to less.
`{cli} up` hands the number on to the daemon it keeps running.",
            env = brand::env_var("WORKERS"),
        ),
        "fmt" => format!(
            "{cli} fmt [--check] [file|folder]...

Lays blueprints out the one way: the same indentation and spacing, a blank
line where one sets things apart, a map written in braces opened into a
block when it has outgrown its line. What a blueprint means is never
changed, and neither are its comments, how a string is quoted, a sentence,
or the order things are said in. A value written plain that YAML does not
read as it is read here (one with `: ` in it, a `${{ }}` bare inside braces)
is put in quotes, where it is the same value: what is laid out is YAML to
an editor too. Given a folder it takes every blueprint
under it, but for folders that are other tools' (`target`, `node_modules`,
and any whose name begins with a dot). Given nothing it takes this folder.

--check rewrites nothing: it says which files would change and exits 1 if
any would. A file that does not read as a blueprint is left as it is, said,
and exits 1.

A project may say a few things of its own in `{settings}`, in the folder
its blueprints are in or any above it, up to the top of its repository:

{table}

It needs no daemon and nobody signed in. The editor lays a blueprint out
the same way when it is saved.",
            settings = blueprint::format::SETTINGS_FILE,
            table = blueprint::format::SETTINGS
                .iter()
                .map(|setting| format!(
                    "  {}: {}    ({})\n{}",
                    setting.name,
                    setting.default,
                    setting.may_be,
                    wrapped(setting.help, 6, 76)
                ))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        "trash" | "restore" => format!(
            "{cli} trash | trash restore <id> [--beside] | trash forget <id> | trash --keep DAYS | always

What is deleted goes to the Trash: a row of a table or a card, a file or a
folder of a project (a document, a mockup), a project forgotten, and a
channel removed with everything said and attached in it. It is left out of
everything that lists or reads what it was of, and is given back where it
was: a card in its column and its place there, or at the bottom of the
column where its place is taken; a file at its path, or with `--beside`
beside whatever is there now, as `name (restored).md`; a project in its
workspace, or the usual one where that is gone; a channel whole, a private
one to the same members, as `#name-2` where its name is taken. What was
said on a record or a document is kept with it, and is its again.

`{cli} trash` lists what whoever asks could read, the latest first, each
with who deleted it and when it goes for good. Giving it back, and
`{cli} trash forget <id>`, which deletes it for good, are for whoever could
have deleted it.

`--keep 7` has what has been there seven days deleted for good, at once,
as the daemon starts and once an hour after that; thirty unless said.
`--keep 0` deletes at once, with nothing kept, as before there was a Trash,
and `--keep always` keeps it until somebody deletes it for good. Each is the
owner's to say, and a snapshot of the factory takes it, with the Trash."
        ),
        "logs" | "retention" => format!(
            "{cli} logs [--keep DAYS | always] [--folder DIR | usual]
{cli} logs <run> [step]

With no run, how long the detail of runs' steps is kept and where it is
written; with `--keep` or `--folder`, that changed. Each is the owner's.

`--keep 30` has what runs' steps printed (their logs), and what the runner
said of each step as it went (`step.*`, `run.started`, `run.resumed`), let
go of once it is older than 30 days: at once, as the daemon starts, and once
an hour after that. What each run came to stays for ever: its row, how each
of its steps ended and what it gave, `run.finished` and `run.failed`; so do
records, their history, channels and what was said there, and measurements.
Nothing goes that something alive has not been handed yet (a handler that
hears those events, a `wait` for one, a comparison that counts one), and
nothing of a run under way. `{cli} run <id>` of a run whose detail went says
so. `--keep always`, as it is unsaid, keeps everything for ever.

`--folder /srv/factory-logs` has runs' steps write their output there from
then on, under `runs/`: it is made where it is not there, and refused where
it is not said whole or nothing can be written in it. What was written before
stays where it is and is read there; `--folder usual` goes back to the data
folder. The databases stay in the data folder. A snapshot of the factory
takes how long and not where (`help profile`), and holds no logs."
        ),
        "down" | "drain" => format!(
            "{cli} down                          stop the daemon now
{cli} down --drain [--within 10m]     stop it once what is in hand is done
{cli} down --at STEP [--of BLUEPRINT] stop it as a step begins (`help play`)

Plain `down` stops the daemon at once. What it runs is left running, and
`{cli} up` takes it up again: a step where it was, an agent's turn begun
again, or taken up where its program goes on without the daemon (`help
outlives`).

`--drain` stops it once what it has in hand is done. From the moment it is
asked the daemon begins nothing new: nothing is applied (an apply is refused,
saying so), no handler's run is begun (what it hears waits for the next start),
and no agent is given a turn that no run asked for (what is said to one in a
channel waits for the next start). It then waits for the turns in hand and
the runs under way to end, and stops. What waits for a person (a review, a
question, a held card) has nothing in hand and holds nothing up.

It waits ten minutes at most, or as long as `--within` says (`90s`, `1h`), and
then stops anyway, saying what was still in hand and what becomes of each:
a turn whose agent's program goes on without the daemon is taken up where it
is, and any other is begun again, or left for a person where whoever asked for
it said so. `{cli} status` says that it drains and what it waits for, and so
does the client. `down` asked meanwhile stops it at once."
        ),
        "play" | "handed" => format!(
            "{cli} up --profile NAME --handed          a factory to one side whose steps say what they were handed
{cli} --profile NAME apply <file|name>    play a blueprint in it
{cli} --profile NAME runs                 its runs, the newest first
{cli} --profile NAME logs <run> [step]    a run step by step
{cli} --profile NAME down --at STEP [--of BLUEPRINT]

To see what a blueprint does with what it is given, play it in a factory of
its own (`help profile`): what it does there reaches nothing of the one you work
in. Started with `--handed`, every step writes down what it was handed before
it does anything: its `with` as it came to once every `${{ }}` in it was worked
out, and for a step that calls or spawns another blueprint, which one, as
what, and the params it was given. `{cli} logs <run>` then says the run step
by step:

  greet (ok)
    # handed: {{\"greeting\":\"hello, Ada\"}}
    gave: {{\"greeting\":\"hello, Ada\"}}
  child (ok)
    # handed to hello as child: {{\"said\":\"hello, Ada\",\"who\":\"Ada\"}}
    gave: {{\"instance\":\"in_1m7gm1c0h511x8qr\"}}

What a step printed is between the two. The blueprint a step started has
runs of its own: `{cli} runs --instance <id>` lists them, and `{cli} instance
<id>` its children. Without
`--handed` the same command says what each step printed and gave. A value
longer than a line of a log holds is cut; a secret is never there, since a
secret is named in `env` and is not what a `with` comes to.

`down --at STEP` stops the daemon as a step of that name next begins, in
whichever instance, or only in an instance of the blueprint `--of` names.
The journal keeps that step's beginning and nothing said after it, which is
what a daemon that went at that instant would have left. `{cli} up` on the
same factory then goes on from there, and `runs` and `logs` say how: the
step taken up again (`step.started` says `resumed`), or refused because what
it does is not safe to do twice. A step that never begins stops nothing;
plain `down` stops the daemon as ever."
        ),
        "actions" | "action" => format!(
            "{cli} actions [name]

Lists what a blueprint can `do` on this device, each with what it does. With
a name, that one: what it takes, and under that what an argument says is
written inside it, what it gives back and what it needs of whoever applies.
`{cli} events --kinds [topic]` says the same of the events a handler can
hear, and `{cli} schema` prints the language whole, for an editor.

The client's Reference shows the same on one page, with the words of the
language: it is the line over the blueprints in the side panel, and a line
of Help. Pointing at a word in a blueprint's text there leads to it."
        ),
        "events" => format!(
            "{cli} events [--topics PATTERNS] [--limit N] [--follow]
{cli} events --kinds [topic]

Shows what has happened, newest last. `--topics` takes comma-separated topic
patterns, where `*` is one word and `**` any number: `step.*`, `board.**`.
`--follow` keeps going as things happen.

`--kinds` lists every kind of event this program emits, and with a topic what
that one carries: the names a handler may put under `for:` and read as
`event.data.<name>`. A blueprint is held to these when it is checked. What
blueprints say to each other with `emit` is not listed: what such an event
carries is what the `emit` step writes. The client's Reference lists the
same kinds, with who may hear each."
        ),
        "secret" | "secrets" => format!(
            "{cli} secret list [--where]
{cli} secret set <name> [--file FILE]
{cli} secret remove <name>
{cli} secret list | set <name> | remove <name> --project P --environment E
{cli} secret agents <name> VAR|--not --project P --environment E

A value a program needs and nothing else should see: the key of a payment
service, a password for sending mail. The factory keeps it under a name,
sealed with a key of this installation's, in a file only its owner can read,
in a folder that no copy of the factory carries.
`set` reads it from standard input or from a file, never from an argument, and
it is never shown again: `{cli} secret set stripe < key.txt`. No command and no
page reads one back; a value that is lost is set again. Setting and taking
away are the owner's; `list` shows the names. The client has the same under
Settings, Secrets.

`list --where` is the owner's: who set each, and what names it (the agents
handed it, the services started with it, the running instances whose texts
name it in an `env`), so that what would fail is seen before one is taken
away. A name something uses and nobody has set is listed there as not set.
`remove` says what still names what it took away.

With `--project P --environment E` it is one of that environment's own,
kept apart from the factory's and listed with the environment, not here.
Setting, taking away and marking one takes `environments:secrets`, which
the factory's owner holds without being given it and anybody else is given
as any scope is; seeing their names takes `secrets:use`. `agents` says the
variable an agent working under the environment is handed it as, and
`--not` hands it to none, which is how each begins. A copy of the factory
lists them and carries no value: each is set again there.

A blueprint names it where a program reads it, in the environment of the
blueprint or of a step, and lists `secrets` under `permissions`:

  steps:
    serve:
      do: service.start
      env:
        STRIPE_KEY: {{ secret: stripe }}
      with: {{ name: checkout, cmd: [./checkout] }}

An agent is handed one the same way, said of the agent and not in a blueprint:

  {cli} agent add builder --secret REMOTE_TOKEN=remote-bot     the variable its program reads, and the secret's name

(In a fleet it is the `secrets` cell of its row.) That is how an agent comes
by a key of another service's: behind its wall it has none of yours. What the
key lets it do there is that service's to say, so make one for the agent that
allows what its work needs and no more: a token for the one repository, a
deploy key. Its program can send what it holds anywhere, since a wall does not
hold the network. What the agent prints is written down without the value.
`--no-secrets` hands it none.

What the fleet itself does at a repository's remote (`fetch`, `push`) it
does as whoever runs the daemon, unless it is told a key to go by:

  {cli} apply fleet --project shop --param push=true --param key=shop-bot

The remote is then asked and sent to as that secret, a bot's token, and as
nobody else. It is given only to the remote's own address, which has to be an
https one. A package that acts at a hosting service takes a `key` the same way.

The value goes from its file into that program's environment as the program is
started, and is in no event, log, database or argument. What is kept of a
service is the name, read again each time it is started. A secret is not
something an expression can read, so nothing written in a blueprint can put
one anywhere else. What the program does with it is the program's: it can
send it anywhere it can reach.

What it prints is another matter. A program that prints what it was handed (its
environment, the key it was refused with) has that written down as
`[the secret `stripe`]` and never as the value: in a step's log and what the
step gives, in a service's log, in an error, and so in every event and message
made of them. It is taken out before anything reaches the disk. A value of
fewer than six letters is left as it is, being too short to tell from a word;
one a program prints changed (in base64, a letter at a time) is not known.",
        ),
        "emit" => format!(
            "{cli} emit <topic> [key=value]... [--data '{{\"key\": \"value\"}}']

Says an event of a blueprint's own topic, from outside: what a handler waiting
on that topic then does, it does. What it carries is the pairs given, or the
mapping after `--data`. A value that reads as a number, true or false is that.

Whoever says it holds a key for the topic. `{cli} key payments --scope
emit:shop.paid` makes a key that can say `shop.paid` and do nothing else, for a
script (`{env}=<the key> {cli} emit shop.paid order=o-17`) or for another
program: `POST /api/emit/shop.paid` with the key as a bearer token and the
mapping as JSON. A service that can only be given an address to call is given
`/in/<the key>/shop.paid`, which takes only a key that holds nothing but
`emit:<topic>`. A service that signs what it sends needs no key in its address
at all: `{cli} help caller`. What the system says of itself (`instance.failed`,
`board.item.moved`) cannot be said this way by anyone.

The event says who said it, in `event.source.actor`: `user:<id>` for a person
signed in, `key:<id>` for a key (the id is in the answer that made it), and
`caller:<name>` for a service that signed. Nothing a sender writes sets it.",
            env = brand::env_var("TOKEN"),
        ),
        "caller" | "callers" => format!(
            "{cli} caller list
{cli} caller set <name> --topic T --secret S --header H [--prefix P] [--encoding hex|base64]
       [--signed '{{time}}.{{body}}'] [--time-header H] [--time-prefix P] [--within 5m] [--key text|base64|hex]
{cli} caller remove <name>

A service outside that tells you when something has happened (a payment taken,
a build finished) calls an address, and signs what it sends with a secret the
two of you share. A caller is that service, described once. What it sends to
`/in/<name>` is said as the one topic named when the signature is one its
secret makes, and is refused when it is not. No key is in the address, so
reading the address lets nobody in. Describing one is the owner's.

The secret is set as any other is, and is named here, never given:

  {cli} secret set pay-signing < signing-secret.txt
  {cli} caller set payments --topic shop.paid --secret pay-signing --header X-Signature --prefix sha256=

Services differ only in where they put things, which their own pages say:

  the body, in hex after a word          --header X-Signature --prefix sha256=
  the body, in base64                    --header X-Hmac --encoding base64
  a time and the body, in one header     --header Pay-Signature --prefix v1= --time-prefix t=
  written `t=<time>,v1=<signature>`      --signed '{{time}}.{{body}}'
  an id, a time and the body, with the   --header Msg-Signature --prefix v1, --encoding base64 --key base64
  id and time in headers of their own    --time-header Msg-Timestamp --signed '{{header:msg-id}}.{{time}}.{{body}}'

`--signed` is what the service signed: `{{body}}` is what it sent, `{{time}}` when
it says it signed, `{{header:<name>}}` a header. `--key` is how the secret is
written, for a service whose secret is a key in base64. The signature is
HMAC-SHA256. A header that holds several signatures, as one does while a
service changes its secret, is taken when any of them is right.

A call that says when it was signed is refused once it is older than `--within`
(five minutes unless said), and one that was taken is said no second time
while it could still be sent again: it is answered as it was. A service that
signs no time can have a call sent again by anyone who saw it pass, so what a
handler does on such a call should be safe to do twice.

The event says who said it: `event.source.actor` is `caller:<name>`."
        ),
        "metrics" => format!(
            "{cli} metrics <name> [--since 1h] [--by label,label] [--label k=v]... [--project <id>]
{cli} metrics --labels [--since 1h] [--project <id>]

How often something happened, how often it failed, and how long it took.
Names include `step`, `run`, `http`, `agent.turn`, `service.health`,
`router.connection`, `store.request`, `snapshot`, any name a step gives itself with
`measure:` (such as `build`), and whatever a service or a blueprint measures of its
own (a line `::measure {{\"name\": \"checkout.done\"}}` in a service's output, a
`measure` step). `--by` splits the result: `{cli} metrics step --by action`,
`{cli} metrics http --by route,caller`. `--project` counts only what ran for
that project: `{cli} metrics run --project shop`. Requests to the daemon and
a device's own numbers belong to no project. A run that was cancelled, its
instance stopped or applied over, is counted and is not one that failed, and
neither is an agent's turn that somebody called off.

What a blueprint said with `labels:` is split by and narrowed to in the same
way: `{cli} metrics agent.turn --by variant --label experiment=cart-12` sets
the variants of one experiment side by side. `--labels` lists the labels
there are, with the values of each."
        ),
        "own" => format!(
            "{cli} own <blueprint> --project <project>
{cli} own <blueprint> --project <project> --say
{cli} own <blueprint> --project <project> --back [--force]

Makes a blueprint that ships the project's own: its text is copied into the
project, where a project's own is looked for (`blueprints/<name>`), and from
then that file is what runs there under the name. The copy is a fork of the
one that ships. It is how you change what a blueprint that ships does for one
project, without touching the rest; where telling it something else, or
changing a row of a table it makes, would do, do that first
(`{cli} explain <blueprint>` says which there are).

What is copied is the text what is running in the project goes on with, where
it is held to one, and otherwise the one that ships now; it says which version.

A file a blueprint that ships links (a prompt, `std/prompts/lane-round.md`) is
made the project's own the same way, under its own name
(`blueprints/std/prompts/lane-round.md`), which is where a project's own words
are read in place of what ships. Words say no version: `--say` says whether
they are another, and which blanks came or went.

What a fork was forked from is kept by the daemon as it is made: which
blueprint, the version, that text to the letter, who forked it and when. It
is kept for as long as the fork is, whatever version of this program is
installed by then, and goes with a copy of the factory and with a project
moved to a journal of its own. `{cli} explain <blueprint> --project <project>`
says it, as the fork's page does: what it was forked from, where it differs
from that, and how the one that ships has moved since. Nothing is merged: the
text is the project's to change, with the differences beside it. A fork made
by an earlier version of this program has nothing kept, and says only what
its text shows.

What follows from it being the project's:

  - It is a file like any other of the project's. What is running reads it as
    whoever applies or saves it vouches for it (`{cli} help apply`): change it
    in the client, or in an editor and then apply with `--take`.
  - It no longer moves when a new version of this program brings another under
    its name. `--say` says where the two then differ, by the version each says
    first, and then what the fork's summary says: where it differs from what
    it was forked from, and how the one that ships has moved since. Take over
    what you want of the new one by hand.
  - It is written where the factory keeps the project's own (`@blueprints/`),
    never in the project's folder, and is read there whether or not the
    project is trusted: only the factory writes there. In a project made of
    a folder that was already there, it is a file of the project's
    repository that does not stand in for a blueprint that ships until the
    project is trusted (`{cli} project trust`).

`--back` takes the file away, and the one that ships is what runs again once
what is running is applied. It is refused for a file that no longer says what
ships, with nothing taken away: `--back --force` takes it away all the same,
and what it said is then gone unless the project keeps it somewhere else.
What was kept of where it came from is forgotten with it."
        ),
        "texts" => format!(
            "{cli} texts
{cli} texts held | follow | usual [--lands in-place | again]
{cli} texts --lands in-place | again

How what is running comes by new blueprints is a setting of the factory, with
two parts. Alone, `{cli} texts` says both. Changing either is the owner's.

How texts are come by:

  held      What is running goes on with the blueprints it has, whatever version
            of this program is installed, until somebody applies it saying to
            take them as they now are (`{cli} apply ... --take`), which moves
            everything it reaches that ships, together. A project's own
            blueprints are run as whoever applied or saved them vouched for.
  follow    A blueprint that changes is taken as it changes, by everything
            running that reaches it, and nobody is asked: a file of a project's,
            and the blueprints of a checkout this program is run from. One that
            does not check is not taken: what runs it goes on with the last that
            did, and says so once, beside itself and in the channel its fleet
            reports in.
  usual     Nothing said, which is how a factory starts: what came with the
            program is held, and the blueprints of a checkout it is run from are
            followed, so a factory run from its checkout follows them unless it
            is told `held`. A project's own are followed only under `follow`.

`follow` gives something up, so it takes `shell` as taking a wall away does:
where texts are followed, whoever can write a project's blueprint files, an
agent among them, changes what runs with the authority of whoever applied it.
It vouches with what whoever turned it on held then and no more, and is said as
an event (`system.texts`). It is for where blueprints are being written.

How a text that is taken lands on what is running (`--lands`):

  in-place  It is carried over, or takes the text where it is, and nothing runs
            again that need not. Something whose steps would have to run again
            goes on, behind, until it is applied itself.
  again     Its steps run again, which for a fleet is beginning it again."
        ),
        "compare" | "comparisons" => format!(
            "{cli} compare [list] [--project P]
{cli} compare start <name> --of <blueprint> [--project P] [--per <param>] [--also <blueprint>]... [--asked]
        --way <way>[=<blueprint>[@<version>]]... [--share <way>=<n>]... [--with <way>.<param>=<value>]...
        [--prompt <way>.<file>=<other file>]... [--count cards | none | <name>=<topic>:<field>[:<field>+<field>[:<word>+<word>]]]...
        [--grows <way> --against <way> --holds <count>=<what it came to> [--at-least] --over <n> --step <n> --most <n> [--least <n>]]
{cli} compare show <name>
{cli} compare share <name> <way>=<share>... [--why <words>]
{cli} compare rule <name> --grows <way> --against <way> --holds <count>=<what it came to> [--at-least] --over <n> --step <n> --most <n> [--least <n>] | --none
{cli} compare end <name> [--goes <way>]

Has one blueprint started several ways, to see which does better. Nothing is
written in any blueprint: from when it is started, wherever a step in the
project calls or spawns `--of`, the daemon starts it one of the ways.

  --way a             a way. Alone, it is the work as it is written
  --way b=my-round    a way that starts another blueprint in its place: yours,
                      as its text is now (edit it later and this goes on with
                      the text it was started with; start another to try that)
  --with b.role=fast  what a way hands, over what the step hands
  --share a=3         its part of the things, against the others' (default 1)
  --per item          the param that says which thing a start is for. Each
                      thing goes one way and keeps to it: the way that has had
                      least of its share is given the next new thing. Without
                      it a thing is the place in the tree it is started at
  --way before=std/lane-round@3
                      a way held to a version: the text that calls itself
                      that, as the factory kept it (a running tree was
                      started with it, or can be put back on it). Refused
                      where that version is not kept, saying what is
  --prompt strict.std/prompts/review-seat.md=prompts/strict.md
                      a way that reads another file where one is linked
                      (what an agent is told), each by its name in the
                      library: two prompts for one step, nothing else changed
  --asked             its ways are named by the start: only a start whose
                      params say `compared: <name>` and `way: <one of its
                      ways>` is its, and goes that way whatever the shares.
                      Any number of these may be in force for one blueprint,
                      and a blueprint starts one itself (`compare.start`)
  --also std/review   another blueprint that is started for the same things:
                      started as written, and measured with the thing's way
  --count moved=board.item.moved:id:from+to
                      an event to count for each thing: what the count is
                      called, the event, the field of it that says which thing
                      it is about, and the fields that say what it came to (the
                      last word of the topic, where none is said). Each is
                      counted under the thing's way, with how long since the
                      thing was given its way. `--count cards` is the usual
                      ones for a board's cards (each move by where from and
                      where to, what a review came to, a question asked, work
                      left for a person), and is what `--per item` counts
                      where nothing is said. `--count none` counts nothing
  --count verdict=review.passed:card::passed
                      what of what it came to says the thing was accepted
                      (the last of several fields): each thing once. `show`
                      then says how many of each way's things were, over how
                      many it was given, and what it added up over those

Everything started a way is measured with the labels `compared=<name>` and
`way=<way>`. `show` is one screen to decide from: a column for each way (and
each text it ran) with how many things it was given, and under it what came
of them and how long after, what was chosen (a turn by its decision), what
was added up in all and for each thing (what turns cost, the tokens they read
and wrote), how many were accepted and what was added up for each of those
(all that was spent, the work that was not accepted with it), and what else
was measured. `{cli} metrics agent.turn --by
way,decision --label compared=<name>` splits one further. It counts; which
way is better is yours to say.

`share` says the shares again, with why: from then each way is given its part
of what comes next by them, whatever went before. A rule says them again
itself, as what is counted says: a gradual rollout, as a service's canary is.

  --grows slate --against usual --holds verdict=failed --over 20 --step 10 --most 50
                      once `slate` and `usual` have each had 20 things whose
                      `verdict` was counted since the shares were last set,
                      `slate` is given 10 more of every hundred, up to 50, where
                      its things came to `failed` no more often than
                      `usual`'s (`--at-least`: at least as often, for what is
                      wanted); where they did not, its share stays, or with
                      `--least 10` goes back a step, to 10 at least. Either
                      way what comes next is read afresh, and `show` says
                      each time the shares were said again and why. The rule
                      is yours: the daemon counts, and decides by it alone

`end` stops it. With `--goes <way>` everything is started that way from then
on, unmeasured, until you end it again with another way or none: that is also
how a blueprint of your own stands in for one that ships, and how you go back.
A comparison is in force for an instance only where whoever started it holds
what that instance runs with.

  {cli} compare start models --project shop --of std/lane-round --per item \\
      --way one --with one.role=first --way two --with two.role=second

`first` and `second` there are two roles of the project's `agents`, a row
for each, with its own model: a card whose way names a role no row has
stays in doing, and the channel says that no agent is available."
        ),
        // A factory's data and its copies, and who works in a factory and
        // where they do, are files of their own.
        _ => return crate::help_copies::topic(name).or_else(|| crate::help_who::topic(name)),
    })
}

/// `text` broken into lines no wider than `width`, each set in by `indent`.
fn wrapped(text: &str, indent: usize, width: usize) -> String {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if line.len() + 1 + word.len() <= width => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(format!("{}{word}", " ".repeat(indent))),
        }
    }
    lines.join("\n")
}
