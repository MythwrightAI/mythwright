# Agents

This file covers agents: how one is set up and named, how its turns are
given and answered, the wall its program runs behind and the home it is shown,
the wrapper that starts every agent program and the programs that outlive the
daemon, how git is used by the daemon and by agents, packages installed as
copies, keys handed to agents, agents in kept (private) channels, external
agents, removing an agent, the Members page's Agents tab, the built-in assistant, and
draining the daemon. Read it before changing anything in `crates/agents`,
`crates/extensions`, `kernel::git`, `places`, or the client's agent views.

The test programs the suites start in place of a real agent's program are
the stand-in program (`crates/hosting/standin`).

## Setting an agent up

- Every agent the factory starts is brought in by `std/agent`, whose params
  are its slot's settings (`docs/architecture.md`, *An agent is set up by a
  blueprint*). A role's agents each have one instance under the role; one
  added by hand has one keyed by its name (`api::byhand`, which the routes
  that add and change one call, `POST /api/channels/{id}/agents` among them).
- What brings an agent up is read from what the instances hold and nothing
  else (`Agents::bringing`, `Brought`): an instance keyed by the agent's name
  is its own, and what is above it counts it. Something new that reads whose
  an agent is calls `bringing`, never the instance that holds it alone (for a
  role's agent that is the role's blueprint, not the role).
- Something new that adds an agent by hand, or changes one, goes through
  `byhand`, so that a project's fork of `std/agent` sets it up. A setting new
  to a slot is a param of `std/agent` (with its `description`), a line of
  `byhand::params_of` and `said`, and an argument of `agent.ensure`.
- An agent's name is unique in the whole factory (`slots.name`), so the fleet
  names each after its project and row (`shop-worker-1`), and work asked of a
  role never goes to another project's agent (`Agents::available_apart`).
  Anything new that brings agents up for a project names them for it.
- The bot's name is its own: a new agent is refused it where a slot is made
  (`Agents::ensure`), and the bot is refused an agent's name
  (`/api/system/speaker`). An agent that had the name before the bot existed
  is kept: `ensure` refuses only a slot that does not yet exist. Nothing may
  tell the two apart by name; an actor says its kind (`agent:`, `bot:`).
- An agent never changes its own slot, even within what it holds: a slot is
  set by a person or another agent (`own::not_its_own` in `crates/agents/src/own.rs`).
- A slot made before walls existed has none until told (the column's
  default; `ensure` gives a new one a wall). Do not wall them in a migration:
  their programs were never run behind one.

### Empty cells

- A cell emptied in the client is kept as empty text; one never filled is
  null. What reads a row of a table a person edits takes both as "nothing
  said": `rows.read` does that once (`said` in `kernel::rows`, through
  `studio::rows`). A blueprint that reads such a row itself writes
  `(x ?? '') != ''`; `??` alone lets an emptied cell through.
- Where a row says nothing of a cell and `fill` names no default, the value
  handed on is empty, and empty is what the agent gets: a row of `agents`
  whose `model` is emptied puts its agents on their provider's usual model,
  and one whose `told` is emptied tells them nothing of its own beside what
  the fleet adds (`std/arrangement` hands `told` with `also`, never as `""`).
  `agent.ensure` takes a model or instructions given as `""` as none. There
  are no standard role instructions to fall back on, only those the fleet's
  first rows are written with (`first_agents` of `std/fleet`). A value to
  keep when a cell is emptied is a line of `fill`
  (`crates/api/tests/suite/edited.rs` checks both).

## Roles and turns

- The application knows no role. A slot has a `role`, a word, and
  `Agents::available` can be asked for one of a role; that is all. What a
  worker or a reviewer is, is the instructions `std/role` gives its slots and
  the blueprints that give the role work. Nothing in Rust branches on a
  role's name.
- An agent's turn with no agent named goes to whoever holds the conversation
  it names (`Agents::holding`), and a conversation not yet held goes to an
  agent with nothing in hand (`Agents::available`). Nothing keeps which agent
  has a card: the thread `lane:<card>` is what knows.
- Every turn is queued through `enqueue_chosen`; something new that gives an
  agent work goes through it, and something new that chooses among agents
  leaves out those a channel holds alone (`alone_in`, below) and external
  agents (`Slot::external`).

### Answers and posts

- An agent's answer is posted where it was asked, by the daemon
  (`turns.rs`), as the agent and on the leave of whoever asked it there:
  `Channels::post_answer` in a channel, `Records::answer` in a topic. The
  agent's own scopes are not asked (one with no `channels:write` or
  `records:write` still answers), and it is given nothing: what it posts with
  its own credential is asked of it as ever. An answer that cannot be said
  (too long, a kept channel it is not in) is not dropped: the factory says
  there why ("`<agent>` answered, and its answer could not be said here: …"). What the agent posts for itself with the factory's command (a
  page it made) is its program's doing, and lands in the right place for two
  reasons, both needed: the agent is told (`heard.rs`: which thread, the
  command that posts a file, `--thread` and `--top`, that a file is attached
  rather than named by its path on the device), and what it posts in that
  channel while its turn there is in hand, naming no thread, is put in the
  thread the turn was asked in (`Agents::asked_under`, called by the route
  that takes a message, for agents only). `--top` (`top` to the route) is the
  channel itself.
- An agent with nothing in hand there, a person, the bot and a blueprint's
  own posts land where they say. Something new an agent is given to post
  with calls `asked_under` the same way. Tests: `threaded.rs` (API suite),
  `threads.rs` (command line suite, with the stand-in program running the
  real binary).
- A post made while a turn is in hand names the model that wrote it, as the
  answer does (`Agents::model_in_hand`: what its program last reported, else
  what the slot runs on). The client shows a model by its provider's name
  for it wherever it knows that name (`modelOf` in `views/who.tsx`, over
  `ui/src/modelsaid.ts`: the slot's name for the model, which `/api/agents`
  hands with `model_reported`; else the provider's listing of the id; else
  the id), both in a message's head and in deciding whether it groups with
  the one before (`ui/scripts/modelled.mjs`). The page and the answer are
  still two messages.


### Agents in channels

- Whoever posts in a channel is a member, decided in the write that stores
  the message (`comes_in` in `talk::joined`, via `Channels::saying`). An agent
  posts only in a channel it is in, else it is refused with how to bring it in
  (`not_in`), including its own posts with the factory's command.
- An agent is in a channel to listen, or only to answer what it is asked
  there (`Member::listens`, `channel_members.listens`). Only a listening agent
  is the channel's one agent, to whom messages naming nobody go (`heard.rs`),
  and only listening ones count in `agent.present`. Everybody brought in by a
  door listens (`join`, `bring`, `std/agent`'s `joins`); never the reverse.
- An agent comes in to answer (`Channels::bring_asked`) four ways: a fleet
  brings each agent into the channel it reports in (`reports` of
  `std/arrangement`, through `std/role` and `std/agent` to `agent.ensure`,
  `Agents::reports_in`; a kept channel only where allowed); a step asking it to
  report in an open channel (`ask` in `agents::actions`); a person naming it in
  an open channel (`dispatch`); and its own answer, in the write that posts it
  (`Place::answers`, from `say_answer`), so no answer is lost. A fleet an
  earlier version started brings nobody in until applied again; meanwhile each
  agent is brought in as first asked.
- A stopped agent leaves the channels it was in only to answer (`put_out`);
  one brought in to listen stays; a removed one is in none (`leave_all`).
  Something new that has an agent report in a channel calls `bring_asked`
  first. Tests: `members.rs` (talk), `posting.rs` (API),
  `ui/scripts/reported.mjs`.

## Walls, homes and what an agent is told

- An agent's program starts behind a wall unless its slot says otherwise
  (`agents::walled`, enforced by `hosting::wall`). A wall that cannot be held
  fails the turn: never start the program in the open instead.
- New code that starts a program for an agent goes through `adapter::command`.
  Anything new an agent's program is shown is declared in `Agents::behind`
  only, including folders it is told it is not shown (`hidden` of
  `agent.task`, other work on the same item): hidden by the wall where there is
  one, reported as shown anyway (`unhidden`) where there is none.
- The application adds nothing behind a wall by a program's name: what a
  program keeps in its home is its package's to declare (`agent.wall`); what a
  project's work needs, the agent's row's.
- Every turn behind a wall runs in the agent's own home
  (`<data>/slots/<name>/home`). A sign-in is carried as a value (key, token)
  or by the package declaring which of its program's files is the sign-in
  (`agent.wall.signin`, `walled::Needs`); only that is shown from the home of
  whoever runs the daemon, in place, never copied. Only an agent whose sign-in
  cannot be told apart (`kept::Theirs`, also used by `unparted`) is shown what
  its program keeps whole. Never name a program's file in a crate for this.
- What an agent is told about itself (`Agents::preamble`) is written before
  each turn into the file its package names as standing instructions
  (`agent.wall.instructions`), only where that file is in the agent's own home
  (`Behind::instructions`); the turn is then handed one line ahead of the
  prompt (`Agents::briefed`). Elsewhere the preamble precedes the prompt. New
  things an agent is told go into `preamble`.
- Part of it is what its slot allows, in words (`crates/agents/src/allowed.rs`):
  its scopes by area (`kernel::principal::areas`), what its program may do on
  the device (wall, how many extra places but never which, the web), what it
  signs in with (kind and name, never a value), and that only others set its
  slot. Built from the slot each turn. A brief no turn owns any more is removed
  by the comment it begins with.
- A program's remembered conversation is kept with the kind of home it was
  held in (`home` of `agent.session`, `remembered` in `turns.rs`) and carried
  on only in that kind.
- What an agent's program is said to see is what the running daemon enforces,
  never only what the slot asks (`agents::allowed::seen`, cases in `seen.rs`;
  `sees` in `/api/agents` and the add answer, `mw agent list`, `ui/src/sees.ts`).
- Test rigs hold no walls (`Walls::Open`). A test of a wall says how it is
  held (a stand-in that records what it was told to show, or the system,
  saying so where unsupported). A client check whose stand-in reads or writes
  outside its project starts the daemon with `--walls off`.

### What takes `shell`

- Removing a wall or showing a program more of the device takes `shell`
  (`Agents::wall_of`; `takes` on those cells of a fleet's row, for `off` alone
  on `wall`). New ways of setting `wall`, `reads` or `changes` ask the same.
- So does turning on a program outliving the daemon (`Agents::outlives_of`;
  `takes: { on: shell }`), and letting it use the web unasked
  (`Agents::web_of`, the slot's `web`), since a wall does not hold the network.
- An agent's program is told by its package that the factory's own command
  needs no permission prompt (`turn.cli`, `turn.cli_path`); that command is
  checked by the slot's key at the factory. Nothing else is allowed on the
  application's say-so; `web` is a word the package maps to its tools. A
  package declares its start command for each kind of wall
  (`examples/starts.json`), checked by a test.

### Repositories and keys

- Behind a wall a repository is shown as far as the slot's `git` says
  (`hosting::repository`: `off`, `read`, `commit`, `write`). Whatever it says,
  what git would run or obey stays read-only (`config`, `hooks`,
  `config.worktree`, a worktree's `commondir`, `gitdir` and `.git` file),
  because the daemon's next `git` runs with no wall. New such files go in
  `as_it_is` or `part_as_it_is`. `std/lane-round` still commits what an agent
  left uncommitted. A merge an agent's work needs is begun for it outside the wall
  and ended by that commit (`place.merge` with `bring`, `fleet.md`).
- An agent gets another service's key by being handed a secret the factory
  keeps (`agents::keys`, the slot's `secrets`), read at turn start and
  scrubbed from output (`Keys::unsaid`). The application never says what such
  a key allows, and nothing shows an agent the keys of whoever runs the
  daemon. Environment values marked for agents are handed the same way
  (`Agents::keys_for`; `docs/working/projects.md`).

## The wrapper, and programs that outlive the daemon

- Every agent's program is started by its wrapper on every system
  (`agents::running` over `hosting::kept`), and its answer is read from the
  wrapper's files. The wrapper is outside the wall.
- `outlives` of a slot decides only what a daemon stop does, read when the
  daemon stops (from `running::Outliving`, kept in memory, updated in
  `write_slot` and `remove`). As the daemon halts, a program whose slot does
  not outlive it is ended with its group and marked beside its files
  (`Process::cut`, `.cut`), so the next daemon does not take its leftovers for
  an answer (`running::there_to_take_up`); one that outlives it is taken up.
  After a kill, the next daemon ends a still-running program whose slot does
  not outlive the daemon and restarts (or leaves) the turn; a program that
  finished by itself is taken up with its answer.
- Two cases still use pipes and end with the daemon: a turn handed something
  in no variable, and a program started with no daemon (`Turn::hosting`). New
  code that lets go of a turn's program unstopped goes through `Running`'s
  `drop`; new code writing `outlives` updates `Outliving`.
- A program that outlives the daemon is found again by its wrapper's files
  under the turn id, and by process id only together with start time. The
  files are in the daemon's `run` folder (not copied, readable only by whoever
  runs the daemon), written already scrubbed by the wrapper
  (`hosting::wrapped`, told by `Kept::unsay`). A value that cannot be scrubbed
  means the turn is not kept.
- Files are removed only after the turn's end is stored
  (`hosting::kept::clean`, in `Agents::conclude` after `end`); everything
  before that forgets the group but leaves the files (`Process::forget`).
  Checked by `restarted.rs`.
- The wrapper waits to be told to start: the daemon records the wrapper's start
  time and only then signals it down a private pipe (`kept::Process::started`,
  `wraps`); a daemon killed between leaves a wrapper that starts nothing. New
  wrapper work goes before that wait.
- Whether such a program can be taken up is decided in one place
  (`kept::Process::is_there`): alive first, then how it ended.

## Git

- Git is called in one place, `kernel::git::git`: no hook runs, no program of
  the repository is asked what changed, nobody is prompted. No shipped
  blueprint calls git; lanes use `places` actions (`place.make`,
  `place.keep`, `place.send`, `place.merge`, `place.clear`, `place.changed`,
  `place.take`, `place.follows`). A test fails on a shipped blueprint, or Rust
  outside `kernel::git`, that calls git another way. New git needs are an
  action over `kernel::git`.
- What a blueprint does step by step is a Rust action; steps that only branch
  on a command's answer, or an expression that builds a command line, are a
  missing action.
- Who commits is the repository's setting, then whoever runs the daemon,
  never the daemon's environment: the four author and committer variables
  (`kernel::git::WHO`) are removed where git is called. Whose work it is goes
  in `--author`. An agent with no wall sees the daemon's environment as is;
  behind a wall that lets it commit it is named itself. Each test suite starts
  its own git in one place (`lanes::asking`, `asking` in `keyed.rs`, `git` in
  `repositories.rs`), which removes the same four (checked by `walled.rs`).
- Two gits in one repository do not wait for each other. So: the daemon takes
  no optional locks (`GIT_OPTIONAL_LOCKS=0`, set in `kernel::git::git`);
  writes take the repository's turn for their whole action (`Place::turn`:
  making and clearing a place, keeping, merging; never remote calls); a
  refused lock is retried for a while (`kernel::git::LOCK_PATIENCE`,
  `Place::ask`) then fails naming whose lock it may be; a failed merge is
  aborted. New git writes take the turn and call through `ask`.
- A stopping daemon asks git nothing more and waits up to ten seconds for
  what is running (`Agents::settle`, from `Node::stop`). What git left half
  done is repaired by the next action, only where it is the factory's own:
  `place.make` removes the item's worktree left locked `initializing`,
  `place.keep` clears an index lock in the item's own place, `place.merge`
  aborts and redoes a merge of the item's branch. A lock in the project's own
  folder is never cleared. New repository changes say what they leave when cut
  off, and whose it is.
- `place.gather` merges in a place detached from its branch and moves the
  branch once, at the end, so a failed merge or a stop leaves the branch
  where it was; called again it first restores what a stop left.
  `place.forward` moves only by fast-forward. Checked by `gathered.rs` and
  `landing.rs` (API suite).

## Packages installed as copies

- A package is installed as a copy, and an upgraded installation keeps using
  the copy, so new things a package declares are not seen until its owner
  takes the one that came with the program. Never replace an installed package
  unasked (`came.rs`, `updated.rs` check a start leaves it byte for byte).
- What is reported is that the two differ and where (`Agents::behind_by`, over
  `agents::came::differs`): `mw extension list`, `mw status`, the package's
  line and page in the client (`ui/src/came.ts`, `views/came.tsx`), and refusal
  messages (`package_note`, `unparted`). Nothing can tell a changed copy from
  an older shipped one, so nothing says newer or older.
- Taking the one that came is explicit (`Agents::take_what_came`,
  `POST /api/extensions/<name>/update`, `mw extension update <name>|--all`,
  Update in the client): installed through `add_extension` (`admin`, the old
  copy kept once as `extension.was.yaml`, `extension.added`, `settle_kept`);
  an identical one writes nothing; one not installed is refused.

## Agents in kept (private) channels

Five rules keep an agent from being a way out of a kept channel
(`crates/agents/src/kept.rs`). They apply to agents the factory starts; an
external agent is in a kept channel as a person is.

- **Its key opens the channel only while a turn asked there is in hand.** A
  turn belongs to a kept channel where its conversation does
  (`Agents::kept_of`), recorded at queue time and never changed
  (`turns.kept`). In-hand turns are remembered (`kept::Hand`), which is the
  agent's half of the read rule, installed into `talk`
  (`Channels::asked_with`). The memory is taken just before the write that
  begins a turn (`Hand::taking`, in `together::begin`) and released before the
  write that ends it, so it may briefly say less than the rows, never more.
  What the factory's owner holds reads nothing for an agent. A turn is
  `answered` while its answer is posted, which counts as in hand.
- **What is emitted of such a turn goes to members only.** `agent.turn.*`
  and `agent.output` take membership (`TurnInfo::takes`). A step continues a
  channel's conversation only as somebody who reads it (`Agents::may_go_on`,
  in `agent.task` and `agent.consult`). Turns leave the daemon through
  `Agents::turns_for`, which drops the words of a turn whose kept channel the
  reader cannot read (`kept_from`); a message's words are handed separately
  (`asked`), read with the reader's own leave (`Channels::said_in`).
- **An agent with no wall is in no kept channel.** `Agents::brought` calls
  `may_be_in` first; agents enter only through `Channels::bring`; one told to
  work with no wall leaves each kept channel, announced by the bot
  (`out_of_kept`); a factory started with no walls gives no turn of one
  (`may_ask`). Tests of agents in kept channels hold a wall (`kept.rs`,
  `kept_agents.rs`).
- **Its program has a home for each kept channel** (`Agents::home_for`,
  `place_for`: `<data>/slots/<name>/kept/<channel>/home`); other turns have
  `kept/` hidden.
- **An agent whose sign-in cannot be told apart is in no kept channel**
  (`Agents::unparted`, evaluated each time with reason and way out: it signs in
  as whoever runs the daemon with no declared `signin`, the daemon's
  environment names a folder for it, or it may be handed a factory folder). It
  is refused at `may_be_in`, gets no kept turn (`may_ask`), and leaves kept
  channels when it becomes so (`out_of_kept`), re-evaluated by
  `Agents::brought` and by `settle_kept` (package taken, sign-in added, agents
  started). New things an agent's standing depends on call `settle_kept`.
- A channel may say its agents are asked nowhere else (`channels.alone`,
  `Channels::create_kept_alone`, `alone_in`): checked where turns are queued
  (`may_ask`) and candidates listed (`could_take`), and such an agent joins no
  second kept channel (`Memory::not_brought`). Refusals name no channel.
- What an agent kept of a channel goes when it leaves (`Agents::leave_channel`,
  or the channel is opened again), when the channel is removed (`forget_kept`,
  from `forget_channel`), with the agent, and after an in-hand turn ends
  (`left_behind`). New per-conversation agent state is forgotten in
  `forget_place`.
- The three shipped packages declare `wall.signin` (`docs/extensions.md`
  says what is unverified). An older installed copy declares nothing until
  updated, and its agents are refused kept channels meanwhile. A fleet whose
  worker is in a kept channel keeps working (`kept_agents.rs`); only `alone`
  stops it.

## External agents

- An external agent (`crates/agents/src/external.rs`) is a program somebody
  runs themselves that works through the factory's command under its own
  name: a slot like any (listed, in channels, given cards, `agent:<name>` with
  its slot's scopes), for which nothing is ever started (no program, wall,
  home, token file or turn).
- It is external because its slot's `adapter` is `agents::EXTERNAL` (no column
  or migration). No package or `--command` may use that name, and
  `add_adapter` adds nothing for it. `ensure` refuses converting between
  external and started (`said_of_external`).
- Every place that gives agents work leaves it out or refuses by name
  (`external::not_given`); new ones check `Slot::external`:
  `Agents::could_take`, `pick` in `actions.rs` (with `if_available` it reports
  `external`, so a round leaves the card in hand for it: `external` of
  `work.worked`), `enqueue_chosen`, `dispatch` (a message naming it starts
  nothing and goes to its inbox where it may read the channel),
  `agent.ensure`, `agent.present`, `try_program_in`. A card given to it is
  left in hand for it.
- Its credential is stored only as a hash. It joins with an invitation
  (`crates/people/src/enrolled.rs`, in `kv` by the secret's hash, not copied):
  `Agents::invite` by whoever may add agents and holds its scopes
  (`external::may_give`), single use, one day by default, fourteen at most;
  `Agents::join` at `POST /api/agent/join` (no session) gives one answer for
  every kind of invalid invitation. The credential is made before the
  invitation is consumed.
- Its scopes are changed on the credential (`Identity::rescope`, wherever an
  external slot's scopes are written), never by replacing it. Stopping
  (`put_out`) and removing withdraw credential and invitation; a new
  invitation re-admits it and replaces a lost token. It gives its own up with
  `Agents::unjoin` (`POST /api/agent/leave`). A copy keeps the slot only.
  Events (`agent.slot.invited`, `.joined`, `.unjoined`) carry no secrets.
- The command line keeps its token in the user's settings, one file per agent
  name and factory address (`crates/cli/src/joined.rs`); commands run as it by
  `--as` or `AGENT`; a credential flag, `TOKEN_FILE` and `TOKEN` take
  precedence.
- Client (`ui/src/external.ts`, `views/external.tsx`): invited from the Agents
  tab of Members or a channel's add dialog, invitation shown once; its line says
  `external`; what it is doing is the cards given to it that are in hand
  (`inHand`); nothing about a program is shown. Checked by
  `ui/scripts/external.mjs`. Not built: messages naming it waiting unread.

## Removing an agent

- One place: `Agents::remove` (`crates/agents/src/removed.rs`;
  `POST /api/agents/{name}/remove`, `GET` to preview; `mw agent remove`).
  `DELETE /api/agents/{name}` only stops one.
- Only a person with `agents:write` and `admin`; never an agent, nor the
  daemon, the bot or an action. Blocked, with a sentence saying what to do, by
  something running that holds it (`agent:<name>` in an instance's `holds`,
  which is what marks a fleet's agent) and by work in hand. Removal cancels no
  work.
- What goes: the slot; its credential and token file (withdrawn before the
  write); in one write with `agent.slot.removed` and `channel.left`
  (`Channels::leave_all`), its memberships, inbox (`Inbox::forget_owner`),
  remembered sessions (`agent.session`) and last model; then its folder
  (`slots/<name>`) and in-memory state. What stays names it as text:
  messages, history, turns, measurements, assigned cards.
- A freed name has no heirs: a new slot for a name with turns records the last
  turn before it (`since`), and everything that finds turns by name honours it
  (`holding`, `beside`, `filed::own`, `asked_before`, `turns`). New code
  reading turns by name honours `since`; new per-agent state is removed in
  `remove`. A test that plants an old turn plants its slot too. Not checked:
  what reads a name as "whoever has it now" (assignee, old `actor.holds`,
  measurement labels).
- Client: `Remove…` in the agent's menu (`mayTakeAway`, `views/takenaway.tsx`,
  the daemon asked first); a removed agent is shown as gone (`Who`).

## The Members page's Agents tab

- The second tab of the Members page (`ui/src/views/members.tsx`, the tab
  itself `views/agents.tsx`), opened by the Members line's name under Factory
  in the Resources panel; it was a page of its own, and `/agents` and a tab kept
  as `agents` open this tab (`tabAt`, `BECAME` in `ui/src/tabskept.ts`). The
  first tab is People (`PeopleTab`): whoever has an account here, each line with
  the Members line's menu (`offeredFor`) and Invite someone as Settings has it
  (`views/invite.tsx`). Narrowed like Activity and Measurements (`Narrow`, in
  the page's head while the Agents tab shows: the workspace, a project, or All
  workspaces). Bands (`banded` in `ui/src/agentbands.ts`):
  projects in panel order, the factory's own last and only under All
  workspaces, stopped agents last. A check reading every agent picks All
  workspaces (`agentsPage` in `ui/scripts/apart.mjs`).
- What an agent is doing is `doingNow` (`ui/src/doing.ts`), from turn events
  read once with `Arriving` and followed after; a kept channel's turn shows
  the agent idle to outsiders.
- Permissions are a summary (`brieflyHeld`) and, pressed, the Tools panel's
  table, read-only.
- Every action is in the menu at the end of the line (`agentActs` in
  `views/agentacts.tsx`, shared with the Members line): Edit… and History
  first, the rest only where allowed. New things shown of an agent fit both
  layouts (`agentlist.css`, by the panel's width).
- **Edit…** opens the whole setup in the drawer (`views/agentedit.tsx` over
  `ui/src/agentedit.ts`, chosen by `setupOf`): an agent added by hand gets the
  add dialog's fields (`ProgramFields`, `WhereFields`) plus reach, sign-ins,
  keys, scopes and its fork's extra params, saved as one apply
  (`POST /api/agents` with `params`); a fleet's agent gets its row's form
  (found by `held_by`, never a table name); an external agent its scopes and
  project in one `PATCH`. Closed parts follow what the daemon would refuse
  (`closedOf`). Checked by `ui/scripts/agentedit.mjs`.
- **Add or invite an agent** is one dialog (`AddOrInviteAgent`): run here or
  from outside (`waysOffered` in `ui/src/agentadded.ts`: applying `std/agent`
  takes `agents:write` and `blueprints:run`; inviting `agents:write`). Run here
  uses the channel dialog's `AddAgent`, asks a project of the workspace and an
  environment where needed, and goes to `POST /api/agents` (new agents only).
- **Move to project…** (`mayMove`, `views/agentmoved.tsx`) for an agent
  nothing running holds, by whoever may change agents and holds its scopes:
  `PATCH /api/agents/<name>` with `project` (`Agents::move_to`,
  `mw agent move`). It keeps its state, scopes and credential, then works
  under no environment until one is set. Refused at once with work in hand.
- **History** (`views/agenthistory.tsx`) is the latest turns (`LATEST`,
  fifty), shown via `ui/src/agenthistory.ts` (`totals`, `endedAs` where a
  cancelled turn is no failure, `lengths` relative to the largest, `byDay`),
  in the theme's dimmer text colour. A turn handed without words
  (`kept_from`) opens to nothing. A line opens on the asking words where the
  daemon hands them separately (`asked`), then the answer, the whole prompt
  folded below. The client never extracts words from a prompt: new prompts
  that embed somebody's words hand them separately. Checked by
  `ui/scripts/agents.mjs`.

## The built-in assistant

- An agent on the built-in assistant is told the one command that gives it a
  real program (`agents::real`): a fleet's agent by its row
  (`record set shop-agents-2 adapter=<package>`), one added by hand by adding
  it again with `--adapter`. A fleet up before a package was installed keeps
  the assistant until its row says otherwise.
- Whose an agent is comes from what is running (`agent:<name>` in `holds`),
  where the step that asked for work fails (`answered` in `agents::actions`).
  The record is found from the suffixes of the holding instance's key, only if
  it is a record of the agent's project whose table has the cell. New hints
  toward "a real agent" use `real::how`. Checked by `fleets.rs`, `written.rs`.

## Draining the daemon

- `mw down --drain [--within T]` (`/api/system/stop` with `drain`,
  `api::drain`) stops the daemon once work in hand is done. From then nothing
  new begins: `Engine::apply` and `Engine::put_back` refuse; a handler reaching
  an event waits with its place unmoved (`Engine::hold_new_work`, in
  `Handler::start`); a turn no run asked for (`kind` `message`) is not begun
  (`agents::drained`). A turn a running run asks for is begun.
- It waits for turns and runs to end, re-checked only on `run.finished` and
  `agent.turn.finished`, under a bound (ten minutes default, a day at most; a
  row of the timers table in `docs/architecture.md`), then stops anyway,
  reporting what was cut (`system.stopping` with `cut`,
  `/api/system/drain?wait=true`). A turn on pipes (`piped`) ends with the
  daemon.
- `mw status` and every client page report it (`draining` of `/api/meta`,
  `system.draining`, `system.stopping`; `ui/src/drained.ts`,
  `views/draining.tsx`).
- New code that begins work by itself calls `Engine::holds_new_work` first.
  Tests: `drained.rs` (command line), `signins.rs` (agents),
  `ui/scripts/drained.mjs`.
