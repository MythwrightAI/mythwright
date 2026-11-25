# Workspaces, channels and the factory's bot

This file covers workspaces as the daemon keeps them, channels and who is in
them, kept (private) channels, archiving, renaming, a channel's details and
managers, discussions between a channel's agents, the Channel tab of the
Tools panel and what runs in a channel, the factory's bot (what it posts, its
commands, its ceiling, what it did), attachments of channels and of records,
a channel's documents, removing a channel into the Trash, and the one way
members are added. Read it before changing `crates/talk`, `crates/workspaces`,
the bot, or the client's channel views. How agents behave in channels is in
`docs/working/agents.md`; the inbox and conversations are in
`docs/working/inbox.md`.

## Workspaces

- A workspace is grouping only (`studio::workspaces`): a name, its projects,
  and the channels that are its own. It lets nobody in and keeps nobody out;
  the one thing it bounds is who a card may be assigned to. No route, event or
  check may ask which workspace something is in before deciding who may read
  it (`crates/api/tests/suite/spaces.rs`). Per-workspace or per-project
  permissions are separate work.
- Placement is kept in tables of its own, not on `Project` or `Channel`. A
  channel belongs to a project or to a workspace, never both and never
  neither, set by one write (`Workspaces::place_channel`); that, a project
  being forgotten and the factory being settled are the only things that
  change a channel's project after creation. Moving a project's own channel
  is refused while something of that project runs and posts there
  (`posts_in`).
- A row may name something gone: it is skipped when read, and whatever
  removes a channel reports it (`forgot_channel`). What keeps something of a
  channel by its project follows `channel.placed` (the search index rewrites
  messages under the new project, `place_said_in`).
- A kept channel is left out of what a workspace lists for somebody who does
  not read it (`Workspaces::shown_to`); that is the channel's rule.
- The side panel's two lists are pure functions of what was read and of the
  panel's workspace (`listed` and `gathered` in `ui/src/workspaces.ts`;
  reading and following is `ui/src/spaces.ts`).

### Every project and channel has a place

- Once somebody owns the factory it always has a workspace, every project is
  in exactly one, and every channel belongs to a project or a workspace.
  Nothing offers, lists or reports a project in none or a channel of nothing.
- The first workspace is made with the factory's owner and named after them
  (its id is a slug of that name and never changes), or `Unnamed workspace`
  (`unnamed-workspace`) for an owner created at first start, until named.
- A project goes where whoever adds it says, otherwise into the usual
  workspace, the earliest that still exists (`Workspaces::usual`). It can be
  moved but never taken out of all (`place_project` with nothing, the route's
  `null` and `mw workspace take` are refused, saying to `put` it elsewhere).
- The last workspace is never removed (the sentence says to rename it). One
  that holds projects or channels is removed only with a destination
  (`remove` with `to`, `--to`): each is moved in the removing write and
  emitted (`project.placed`, `channel.placed`). The caller is checked for each
  of its channels as `place_channel` would (`Channels::may_write`); a channel
  kept from them is not named.
- A channel made for a project belongs to it; one made for none belongs to
  the workspace named, else the usual one, in the write that makes its row.
  `talk` knows no workspace, so it is told how to place a channel at creation
  (`Channels::makes_with`, set by `Workspaces::open`; `Channels::asked_in`), as
  `Projects` is (`Projects::makes_with`, `crate::placed`). Every channel is made
  through `Channels::make`. Saying a channel belongs to nothing is refused
  (`Belongs::Nothing`, `mw channel belongs --nothing`,
  `workspaces::to_nothing`).
- A forgotten project leaves its channels to its workspace, in the forgetting
  write (`forget_project`).
- The rule is enforced in one place, `Workspaces::settle` (via
  `Studio::settle`): it makes a missing workspace, puts unplaced projects and
  ownerless channels in the usual one, and emits each as the system; with
  nothing to do it writes nothing. It runs when the factory's owner registers
  and when a node starts (which covers data from earlier versions, copies, and
  interrupted work), nowhere else.
- `project.placed` is on the list the factory's journal keeps for good
  (`journal::HERE`, `Writing::say_here`), whatever journal keeps the project's
  other events, so `project.created` and `project.placed` are in different
  journals and nothing may rely on their order (studio suite
  `workspaces.rs`).
- Something new that makes a project goes through `Projects`, a channel
  through `Channels::make`, and something that takes either out of where it is
  puts it somewhere in the same write. `settled.rs` (API suite) starts on an
  earlier version's file and checks everything ends up placed, emitted once.

## Membership and kept (private) channels

- Whoever posts in a channel is a member, decided in the write that stores
  the message (`comes_in` in `talk::joined`, called by `Channels::saying`). A
  person joins an open channel by posting; an agent must already be in it
  (`docs/working/agents.md`). A channel's row, members and their events are
  one write (`Channels::make`, `join`, `leave`). New posting goes through
  `Channels::saying`.
- A channel may be kept to its members (`talk::kept`: `channels.kept`; its
  display name is `title`, and its `name` is its own id).
- Who reads one is one expression, `reads` in `crates/talk/src/kept.rs`: the
  channels' scope and membership, or the scope and what the factory's owner
  holds (`private`). A key acts as its maker with its own scopes, so it is a
  member through its maker only where the key holds `channels:read`.
- Three doors differ only in the scope checked first: `Channels::may_read`,
  `may_write`, and `may_find` (no channel scope; for callers that check their
  own: removing a channel, bringing an agent in, `agent.present`). A kept
  channel answers a non-reader exactly as a missing one (`kept::missing`,
  `no_message`, `no_attachment`).
- A door returns a `Reader` for that one channel. A `Channel` with members, a
  `Message` and an `Attachment` come only from methods that take a `&Reader`
  or that pass a door themselves (`post_at`, `readable`, `attached`, `remove`,
  `ensure`); a talk suite test reads the source for public methods that break
  this. Daemon-internal reads use `Channels::daemon(why)`, each listed with its
  reason (`READS_AS_THE_DAEMON` in `crates/api/tests/suite/kept/listed.rs`), as
  is every place the node reads messages (`READS_WHAT_WAS_SAID`).
- A route calls a door before anything else. Lists by id for somebody leave
  out `Channels::kept_from` (`Workspaces::shown_to`, `Search::readable`). A
  thread, and the message something answers, are looked up only in the
  channel asked for.
- Every event of a kept channel takes `Channel::takes` (`member:<id>`), and
  who is told is decided at delivery from what `Channels::open` installed
  (`Journal::members_from`), which cannot wait on the database. So membership
  is also in memory (`kept::Memory`), filled from the tables on open, changed
  before the write for somebody leaving and after it for somebody joining, one
  change at a time (`Memory::changing`), restored if the write fails. Whoever
  is removed from every channel in another crate's write is released first
  (`Channels::leaving`, whose `Left` `leave_all` takes). An event that must
  reach somebody who just left, or everyone in a removed channel, names them
  (`member:<id>;<actor>`: `channel.left`, `channel.removed`, `channel.joined`,
  `channel.created`).
- New channel events use `Source::taking(channel.takes(..))`; new reads take a
  `&Reader`; new membership changes update memory on the right side of the
  write, holding the turn.

### The door

- One door (`talk::door`; `agents::door` for the agents in it; routes in
  `crates/api/src/routes/kept.rs`; `mw channel create --kept`, `keep`, `open`,
  `add`, `leave`, `alone` in `crates/cli/src/channels.rs`).
- *Made*: one write with its maker, the people and bots asked for
  (`Asked::members`) and its place; agents asked for are checked first
  (`Agents::could_be_in`) and brought in after (`bring_in`).
- *An open one kept* (`Agents::keep_channel` over `Channels::keep`), by a
  member (`door::may_keep`): its name becomes its title and its name its id.
  Memory is filled before the write; `channel.kept` (taking nothing) tells
  non-members to stop showing it. Refused while an agent in it could not be in
  a kept channel and while a turn of it waits or is in hand. Ended turns are
  marked as the channel's (`turns.kept`) before the write, and agents' program
  memories of it are forgotten. A message is emitted as its channel is in the
  write that stores it (`door::as_it_is`).
- *Opened again* (`Agents::open_channel` over `Channels::open_again`): only by
  the factory's owner (a member is told so first; an owner key needs `admin`).
  It takes a shared name (another if taken); memory forgets it after the
  write; refused while a turn is unfinished.
- *Leaving*: a person by `Channels::step_out`, except the last person of a
  kept channel; an agent is removed by a member who may change agents
  (`Agents::leave_channel`); nobody joins by themselves.
- *Asked nowhere else* (`Agents::hold_alone` over `Channels::hold_alone`):
  held in memory before the write, released after; refused while an agent in
  it is in another kept channel or has unfinished work elsewhere.
- All of it holds the membership turn, plus the turn for giving agents work
  (`Inner::choosing`) where turns are involved. Something new that changes
  whether a channel is kept takes both and has its line in
  `crates/api/tests/suite/kept_door.rs`.
- `channel.ensure` takes `kept` (and `alone`) and is idempotent: an open
  channel is found by name, a kept one as the first kept channel of the same
  project with that title that has whoever applied the blueprint in it
  (`Channels::ensure_as`). It changes nothing of an existing channel, and
  asking for a kept one where an open one has the name is refused.
- Not enforced (in the README): what a member's own blueprint emits of what
  it heard, the disk and search index, what was said while open, and agents'
  leftovers.

### Kept channels in the client

- Shown as Private, always by title (`calledOf`, `hashed` in
  `ui/src/kept.ts`), with a lock (`signs.lock`) where an open channel has `#`.
- Offers are pure functions (`mayKeep`, `mayOpen`, `mayLeave`, `lastPerson`,
  `mayAdd`), drawn only where they do something; refusals show the daemon's
  sentence (`views/keptdoor.tsx`).
- The page re-reads channels on `channel.kept`, `channel.joined` and
  `channel.left`; a channel no longer listed is forgotten as a removed one
  (`gone`, `forgetChannel`). New code that names a channel uses `hashed`; new
  code that keeps something of a channel is told by `onChannelRemoved`.
  Checked by `ui/scripts/kept.mjs`.

## Archived channels

- `talk::archived`: read, listed and found as before, written by nobody,
  restorable. Stored on its row (`channels.archived`); archive and restore are
  each one write (`channel.archived`, `channel.unarchived`).
- Nothing is posted in one: every message goes through `Channels::saying`,
  which reads the row first (`archived::refused` says how to restore). The bot
  is checked before joining by posting (`post_speaker`). Decisions made
  without the database (`enqueue_chosen`, `dispatch`) use
  `Channels::is_archived`, kept in memory by id and name.
- Nothing in one waits unread: its inbox lines open it but are not counted or
  raised (`unread_of`, `tell_in`; client `count`, `channelsWait`).
- Archiving changes no access. Whoever may write may archive; refused while
  something running posts there (`posts_in`) or a turn of it waits or is in
  hand (`Agents::archive_channel`, under the turn for giving agents work).
- In the client it is under a folded Archived part at the foot of the
  workspace's channels (`views/archivedpart.tsx`); an archived conversation is
  under the inbox's Archived part (`asListed`). Archiving is not removal: it is
  still listed by `/api/channels`, so `onChannelRemoved` is not called and its
  groups, pins and folds stay. Checked by `ui/scripts/archived.mjs`.

## Renaming

- One place: `talk::renamed`, `Channels::rename` via `Agents::rename_channel`
  (`PATCH /api/channels/{id}` with `name`, `mw channel rename`, Rename… in the
  row's menu). An open channel changes its name (refused if taken); a kept
  channel or conversation changes its title.
- Row and `channel.renamed` are one write, checked against the row as found.
  Who may: its managers. Refused for an archived channel; while something
  running finds it by name (`named_by_name` in `routes/renamed.rs`, beside
  `posts_in`), since the next `channel.ensure` would recreate the old name;
  and for an open one while a turn of it waits or is in hand.
- Ended turns naming an open channel by name are rewritten to its id first.
  Everything else keeps a channel by id and follows the event where it shows
  the name (search index via `place_said_in`; the client re-reads). `#name` in
  old messages stays. Something new that keeps a channel by name follows
  `channel.renamed`, or uses the id. Checked by `ui/scripts/renamed.mjs`.

## Details and managers

- Managers change a channel's details (`talk::managed`): its name, its
  `topic` (what it is for; one line, `MAX_TOPIC` letters,
  `channel.described`), and its managers (`channel.managers`).
- The rule is one pure function (`managed::manages_as`, checked case for case
  with the client's `manages` in `ui/src/managed.ts` against
  `ui/src/managedcases.json`): whoever may write in channels and holds `admin`
  or manages the channel.
- Managers are people in it (`channel_members.manages`), so leaving ends it:
  the maker (in the creating write) and whoever a manager names
  (`Channels::manage`, `POST /api/channels/{id}/managers`,
  `mw channel manager <channel> <who> [--not]`). Every person in a
  conversation manages it. Agents and bots manage nothing.
- The last manager cannot be removed, but may leave; such a channel, or one
  from before managers existed, is managed by whoever manages the factory.
- Checked beside the channel's door (`Channels::writable`), never instead.
  `PATCH /api/channels/{id}` takes `name`, `topic` or both; `mw channel
  describe` sets the topic. New changes to what a channel is call
  `managed::may_manage`.
- Client: Details in the Channel tab (`views/channeldetails.tsx`), editable
  for whoever `mayChangeDetails`; `ManagerLine` on a member's page; Rename…
  only for managers (`mayRename`). Checked by `ui/scripts/details.mjs` and
  `managed.rs` (talk and API suites).

## The Channel tab of the Tools panel

- What is set on a channel is under the Tools panel's Channel tab (`MEMBERS`
  in `ui/src/context.ts`, stored as `channel`; an older `members` maps to it,
  `tabNow`), reading Details, Members, Settings and Running here
  (`ChannelSettings` in `views/channelset.tsx`): private or open, leaving,
  archiving, a conversation opened as a channel, Discussions…. Channel and
  conversation heads have no gear. Each settings line is the same one the
  row's menu offers, built by the same function (`doorItems`, `archiveItems`,
  `conversationItems`). Something new set on a channel is a line there.
- Members are read about in one place, the Tools panel's member list
  (`showMember`, `Members` in `ui/src/views/context.tsx`; `listed` in
  `ui/src/members.ts`): people, agents and bots, each a card
  (`MemberCard`), then the one pressed. Which blueprint had the bot post is
  shown there, never in the channel. A model on an agent's card is the
  provider's name, else the chosen name, else the reported id made readable
  (`readable` in `ui/src/modelsaid.ts`).

### What runs in a channel

- Whether an instance posts in a channel is read from a param or state word
  called `channel` (`posts_in` in `crates/api/src/routes/talk.rs`). A
  blueprint that names its channel otherwise is not found.
- Running here (`RunsIn` in `views/runsin.tsx`, over `ui/src/runsin.ts`) lists
  each live instance that posts there, as the daemon reports
  (`GET /api/channels/{id}/running`, `posts_in` over every live instance), to
  whoever reads the channel and holds `blueprints:read`. For whoever may apply,
  an instance with no parent has Settings… (a form from its current blueprint
  in the drawer via `Applying`, the channel param kept, re-applied through
  `/api/apply` with its blueprint, key, project, environment and labels) and
  Stop… behind a confirmation. It is re-read on apply, change or end
  (`heardOf`); nothing polls.

## Discussions

- A channel's agents discuss in threads where the channel enables it
  (`std/discuss`, its params the settings; `docs/discussions.md`). Every
  decision is `kernel::discussed`: whether a message begins one, belongs to
  one or neither (`takes`), whose turn and whether over (`next`), what is
  posted as a stretch ends (`ending`), what an agent is told of the others
  (`brief`).
- The keeper is `talk::discussed` (`discussing` per channel, one row per
  discussion), each change one write with `channel.discussion.*` (on
  `journal::HERE`, scoped as the channel), held in memory.
- Dispatch is the one door (`Agents::handed` in
  `crates/agents/src/discussing.rs`): a message that begins a discussion or
  is in its thread gives no agent its own turn; with a lead, a person naming
  no agent goes to the lead.
- Turns are `agent.task` from `std/discussion-turn` (keeps nothing, calls
  itself, bounded by the cap) with `thread:<root>`, shown the thread at turn
  start (never baked into the prompt). A stretch is `std/discussion`, kept,
  whose ending posts once (`says` of `discussion.end`).
- A message's kind (a plan, something to discuss) is a mark it is posted with
  (`kernel::discussed::MARKS`, `messages.mark`), never read from its words; an
  agent's answer is marked by the word its turn ends with where the channel
  takes plans (`Agents::plans_in`).
- Outside discussions, agents and bots answering one another start at most
  thirty turns an hour per channel, counted across restarts, then wait for a
  person, who is told (`Agents::may_start`).
- A person ends a discussion without answering (`Channels::end_discussion`,
  one write; the route then stops the stretch).
- New settings are a param of `std/discuss` and a field of `Settings`; new
  decisions are cases of `kernel::discussed`.

## The factory's bot

### What blueprints post

- Blueprints post through `channel.post` (`crates/studio/src/actions.rs`): as
  the bot named by `as:`, else as the factory's one bot (`studio::speaker`,
  `Channels::post_speaker`), with the step kept on the message (`said_by`). No
  blueprint names the speaker; one that needs the bot's name gets it from
  `channel.ensure` (`speaker`). The client shows such a message as the bot's
  only, opening only `detail` (a full error). Rust's own posts also use
  `post_speaker`; nothing new is posted as `system`. The bot's name is in the
  journal's `kv` (`speaker`), taken into a copy.
- `channel.post` is idempotent: a re-run step finds its message by run, step
  and execution (`Channels::said_already`, `SaidBy.doing`; `ActionCx::doing`
  distinguishes each try, hook entry and task element). Message, attachments
  and event are one write (`Channels::insert`); a message an earlier version
  wrote but never emitted is announced when found.
- Something new an action leaves behind records which step left it and looks
  for that when `ActionCx::again` says it is a re-run; only then may it
  declare `.idempotent()`.
- `record.add`, `board.add` and `record.comment` are found the same way
  (`crates/studio/src/left.rs`): their events carry `doing` (from
  `Through::doing`), and a re-run reads that topic for its run in the
  project's journal (`Journal::said_of`) from a little before its try began
  (`ActionCx::began_at`, `Journal::place_before`). Tables kept as files, by a
  store service or as documents are refused instead (`Studio::left`). Checked
  by `twice.rs`. New record-adding steps emit `doing` (`left::mark`) and look
  with `Studio::left`.
- A failure is reported once, by its innermost reason (`reason` on
  `run.failed`, `instance.failed` and a step; the full chain is
  `kernel::why`). `instance.failed` says which `call` waits on it
  (`called_by`) and whether it may fail (`let_be`), so the fleet's failure
  handlers (`trouble`, `let_be`, `slipped` in `std/fleet`) skip what a step
  will report anyway.

### Who the bot credits on records

- A blueprint step's change to a record is shown as the bot's (`through` on
  `board.item.created` and `record.created`; `board::told_as`,
  `Board::add_through`), by the bot's current name, never as whoever applied
  it, who stays the event's actor and `by`. The blueprint is shown after the
  line (` · via std/lane-round`: `viaOf` in `ui/src/journey.ts`).
- Wording is `kernel::worded` (`as_called`), applied when a line is written
  and when read (`Collections::history`); the client uses the name it read
  (`saidNow`, `whoNow`, `addedBy`).
- A step's own `emit` carries no `through`; the history keeper finds the
  blueprint from the emitting instance (`studio::through`,
  `Engine::running_in_hand`, `runs::blueprint_kept`). Checked by `lanes.rs`
  and the studio suite's `stepped.rs`.
- `wrote` (an agent's actor only) names whose work a step adds a record from
  (`kernel::worded::from_what`, `fromWhat`); it grants nothing, and shipped
  blueprints pass only what is known. Checked by `wrote.rs`, `featured.rs`.

### What the bot did lately

- Its page under the Tools panel's members (`views/botdone.tsx` over
  `ui/src/botdone.ts`) lists history lines blueprint steps wrote
  (`Collections::done_through`) and what it posted under its current name
  (`Channels::said_by_the_bot`), newest first by day, served by
  `GET /api/system/speaker/history` (`routes/bot.rs`), filtered by what the
  caller may read. Nothing is stored for it. Checked by `botdone.rs` and
  `ui/scripts/botdone.mjs`.

### Commands

- A command calls the daemon's own route as the poster, authenticated as when
  they posted (`talk::bidden`, routes from `api::commands` with `Within`).
  Never infer identity from a message's name. A new command is a line of
  `Command::read` and a request, never a direct call of what the route calls;
  path words go through `part`. Tested in `commanded.rs`.
- Accepted commands are one list (`talk::commanded::TAKES`, also
  `api::commands::TAKES`), which is the help, the client's completion
  (`/api/system/speaker`, `taking` in `ui/src/mentions.ts`) and the parser.
  Each line is written exactly as `mw help` writes the command and is the
  grammar (`talk::taken`); of the lines matching the most words, the first
  that parses wins (`Command::read`, cases in `taken.rs`). `bot.rs` (command
  line suite) checks lines against help and flags; `commanded.rs` checks each
  is accepted.
- Board and agent commands are in `talk::bidden`, the rest in `talk`'s
  `asking`; each route's scopes are a line of `REACHES`. Excluded: workspaces,
  projects, accounts, keys, scopes, secrets, sign-in, the factory itself,
  profiles, copies, extensions. Nothing tells a channel what its members may
  not read (inbox counted not read out, kept channels only from inside,
  private tables not listed, search finds no messages).
- In a project's channel the bot is that project's (`within` in
  `talk::bidden`); the route checks whether the caller may act at all before
  whose the thing is, so a refusal never reveals that something exists.
- In the composer Tab inserts an offered command and Enter always sends.
- A message naming a bot of the channel and no agent is the bot's
  (`Agents::dispatch`). A bot's `@` of an agent gives it a turn, so
  `greeting` in `std/fleet` names agents without `@`. The bot does not hear
  itself.

### The bot's ceiling

- The bot holds nothing itself: a command runs as the poster narrowed to the
  ceiling (`kernel::Principal::within`, in `answer` of `talk::bidden`). Unset,
  the ceiling is everything its commands reach (`commands::REACHES`, handed as
  `reaches` beside `holds` by `/api/system/speaker`).
- Stored with its name (`kv` `speaker`/`holds`), changed by
  `PUT /api/system/speaker/holds` (`Speaker::hold`) by `admin`, giving only
  what they hold, one write with `channel.speaker_held`. A refusal caused by
  the ceiling names it (`beyond`); others keep the route's words.
- What a blueprint posts as the bot is not narrowed (stated on its page,
  `views/botholds.tsx`). A new command route adds its scopes to `REACHES`.
  Bots of `std/bot` take no commands and have no ceiling.

## Attachments

### A project's file posted in a channel

- A project file attached in a channel is posted as that file, not a copy:
  the poster sends its whole path (`from` of
  `POST /api/channels/{id}/attachments`; `mw post --attach` unless `--copy`);
  where it is a file of the channel's project (or of the deepest project
  holding it, for a channel of no project), readable by them
  (`Projects::may_read_path`), and the bytes match, the attachment names it by
  project and path (`api::routes::posted::recognised`,
  `Attachments::add_file`). Anything else is a copy.
- What it is now is read with the message, as the reader may read that file
  (`posted::seen`, `GET /api/attachments/{id}/seen`): `file.state` is `there`
  with its size, `gone`, or `kept`. Its bytes go through a project file's
  checks (`GET /api/attachments/{id}`, `/view`). `talk` knows no project files.
- An agent is told such an attachment as the file with the command that reads
  it (`heard.rs`), and a copy by `mw attachment <id> --out <file>`, never by a
  device or data-folder path.
- A copy becomes a project file (`POST /api/attachments/{id}/project`):
  `Projects::write_new`, then one write with `channel.attachment.placed`
  (`Channels::placed_file`), then the copy is removed; the message names the
  file from then on.
- Client: a chip (`views/attached.tsx` over `ui/src/posted.ts`) opening in the
  Tools panel's Docs tab (`showDoc`); Add to project… for whoever `mayPlace`;
  pages follow `channel.attachment.placed` (`placedFile`).
- New code handing out messages calls `posted::seen_all`; new code serving
  bytes calls `posted::bytes_of`. Checked by `posted.rs`, `threads.rs`,
  `ui/scripts/posted.mjs`.

### Attachments on records

- An attachment is readable by whoever may read what it is attached to. A
  record (a card included) carries attachments (`studio::attaching`), added by
  whoever may change it, read by whoever may read it, at every route:
  `POST`/`GET /api/records/{id}/attachments`, `DELETE …/{attachment}`,
  `GET /api/attachments/{id}` (a `ratt_` id is a record's,
  `attaching::is_of_a_record`), `mw record attach|attachments|detach`,
  `mw attachment`, and the action `record.attach` (a stand-in in a sandbox).
- Stored like a channel's: a file in the same data folder
  (`snapshot::ATTACHMENTS`; a copy takes it by the ids its databases list,
  `attaching::attached_ids`, `copy::attached`) and a row of
  `record_attachments` (`KEPT_APART`), each change one write with
  `record.attached`/`record.detached` scoped as the record, under the
  record's turn, at most 32 MB a file and `attaching::MOST` per record.
- Kept by record id: a record in the Trash keeps its attachments; deleted for
  good they go with it (`Collections::forget_attached`).
- A removed attachment goes into the Trash (`Records::detach`,
  `Kind::Attachment`): row moved in one write with `record.detached`, then the
  file moved; restore (`trash::attached::restore`) and delete
  (`attached::forget_left`) also check the old place. Without a Trash it goes
  at once.
- Deleting for good also removes attachments of a document or mockup by path
  (`forget_attached_at`, by `kernel::whole::within`) and of a project's records
  (`forget_project_attached`). New code that deletes a record, a record's file
  or a project for good calls one of these.
- An agent working a card is told its attachments in its brief (name, kind,
  size, the command that reads it; `Agents::attached_told`, only after
  `Collections::any_attached` finds any). `std/work`'s prompt asks it to
  attach evidence.
- A topic's pictures are its attachments, named in its text by id (`attachment:<id>`): read as
  the topic is, through revisions and a new title; an attached page is shown by
  `GET /api/attachments/{id}/view` as a channel's is (`attachment_view` takes a `ratt_` id too).
  Tested by `topicpictures.rs`.
- Client: an Attachments tab in a card's and a row's form (`RecordTabs`,
  `views/recordfiles.tsx` over `ui/src/recordfiles.ts`). Tests:
  `attaching.rs` (studio), `recordfiles.rs`, `snapshots.rs`, `records.rs`,
  `docsed.rs`, `ui/scripts/attached.mjs`.

## A channel's documents

- A channel's documents (`talk::docs`, table `channel_docs`) are library
  records by id or project files as `<project>/<path>`, shared by its
  members, taken in a copy and into the Trash with the channel (`HELD`).
  Reading is reading the channel; changing is writing there (refused while
  archived), one write each with `channel.docs`.
- What an entry names is resolved above `talk`, as the viewer may read it
  (`agents::docs`: `docs_seen`, `may_add_doc`): `there`, `gone`, or `kept`;
  never left out. An agent asked in the channel is told those it may read, by
  title, place and command, never contents (`Agents::docs_told`).
- Nothing restricts entries to the channel's workspace; the client offers its
  projects (`ui/src/channeldocs.ts`, `views/channeldocs.tsx`). New entry kinds
  are cases of `Doc` and `Agents::look_at`.
- The Docs tab is a default Tools tab (`DEFAULT_TABS`); a newly default tab is
  given once to readers with saved tab lists (`GIVEN_ONCE`, `keptOf`,
  `tabsOf`). Checked by `docsed.mjs`, `channel.mjs`.

## Removing a channel into the Trash

- `trash::Trash::delete_channel` (route, `mw channel remove`, client), or for
  good where nothing is kept (`Channels::remove`). Its row, members,
  messages, attachments and inbox lines are moved into its Trash row in one
  write (`Channels::trash` by `HELD` in `crates/talk/src/trashed.rs`, read by
  column name); then its files move, its `channel.message` events go
  (`Journal::forget_in_every`), and `channel.removed` (with `trash`) updates
  search and the client (`onChannelRemoved`). Agents forget what they were
  asked there (`Agents::forget_channel`).
- Restored (`Channels::put_back`) in one write (`channel.created` with
  `restored`; inbox lines told again), a kept one with the same members, an
  open one renamed `<name>-2` if taken, agents re-checked
  (`Agents::channel_back`). Deleted for good via `forget_trashed`.
- Seen in the Trash by channel readers and, for a kept one, by its members at
  removal (`Channels::would_read`); restored or deleted by the factory's
  owner. New stores of channel content are a line of `HELD`; `removed.rs`
  searches copies for the words.

## Adding members: the one way in

- The participants list a channel's head opens (`ui/src/views/participants.tsx`)
  ends in one line that adds somebody (`lineCalled` in
  `ui/src/memberadded.ts`), opening **Add or invite channel member**
  (`AddChannelMember`). The head has no other add button. Checks reach it via
  `openAdding` (`ui/scripts/adding.mjs`).
- The kind is chosen by radio cards: an agent (`views/addagent.tsx`), a person
  (an account not in it, or an invitation to register), or a bot (`std/bot`).
  Which kinds are offered is `kindsOffered`, by what the daemon would refuse;
  with none there is no line.
- For an agent it asks first whether it runs here or outside (`comingOf`,
  `waysOffered` in `ui/src/agentadded.ts`); an outside one is invited
  (`InviteFields`, `Invitation`), brought in at once, listed as invited until
  it joins. The existing-agent list is `AgentPick` (`views/agentpick.tsx`:
  `agentLine`, `agentsFound`, `lastLine`, `listPlaced`).
- What may be set of whoever is added is `settingsOf`
  (`ui/src/membersettings.ts`, `views/membersettings.tsx`): an agent's scopes
  by area, a person's extra scopes, nothing for a bot. New settings are entries
  there.
- An invitation to register names no channel (`invitedSaid`); a person joining
  a channel by themselves after registering is not built. Checked by
  `ui/scripts/addmember.mjs`.
