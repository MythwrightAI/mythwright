# Security: scopes, keys, secrets and who may see what

This file covers how the factory decides who may do and see what:
scopes and principals, sessions and sign-in, keys, people and groups,
secrets, private tables and what events tell, acting as a deputy, the
factory's bot, devices and links, and what the HTTP layer checks before a
route runs. Read it before you add a route, an action that writes
records, an event, anything that handles a secret, or anything that runs
with more authority than whoever asked for it.

## Principles

- Nothing leaves the machine unasked: no telemetry, no update checks, no
  addresses of ours in the code. The `offline` gate runs the whole
  product with the network cut off and fails if one packet heads outside.
  What a user points it at (an agent's model, a hub, a command in a
  blueprint) is theirs.
- Nobody gets by a deputy what they do not hold. Wherever an instance
  with authority acts on what somebody else may write, what the writer
  holds is checked:
  - a table a blueprint acts on says `takes` (`studio::declared`, and
    `Collection::may_change` wherever a record is changed);
  - an event is emitted with `emit:<topic>`, from a step as from outside;
  - a handler that counts what its own children emit says `scope: own`;
  - giving an agent work takes holding what it holds
    (`agents::actions::pick`);
  - a blueprint that acts for whoever did something else calls
    `actor.holds` on the event's `source.actor` (`std/pass-by-hand` before
    it merges for whoever moved a card).
  Something new of this kind is checked the same way and has a test that
  tries it with a key with fewer scopes (`crates/api/tests/suite/held.rs`).
- Every described event says who may hear it, with
  `blueprint::events::heard` at the end of its crate's `events.rs`; one
  left out is heard by `admin` only.

## Sign-in and sessions

- A factory started by `serve` (and so `up` and the desktop window) with
  `Config::makes_owner` makes its owner account on first start, before it
  answers anybody, where there is no account at all, in one statement
  (`Identity::make_owner`, `people::opened`, `api::opened`). It is named
  `Unnamed` (`kernel::UNNAMED`) with no email and no password; its
  workspace is `Unnamed workspace` (`workspaces::called_after`, made by
  `Studio::settle`).
- Nothing of the machine or the user running it is taken for a name:
  nothing reads `USER` or the like for one, and nothing new may.
- When the factory's owner is first named (`PUT /api/me/name`, used by
  `mw register` and the Profile page), their workspace, if still called
  `Unnamed workspace`, is renamed after them (`Workspaces::follow_owner`,
  one write emitting `workspace.renamed`). Its id stays
  `unnamed-workspace`: workspace ids never change, because the client
  keys tabs and choices by them.
- The session the first start makes is handed to the program that started
  the daemon (`Node::made`), which keeps it as the command line's session
  (`client::keep_made`) only where the home has no session for that
  address or has one for that same factory, and before it reports that it
  is running.
- A node not told `makes_owner` registers its first account from this
  machine; that is only a test's node. `Identity::register` takes whether
  the request came from this machine; only the route knows, from the
  connection.
- A link signs somebody in once, within a minute (`LINK_MS`). Its secret
  is kept by hash in process memory only (`opened::Links`, bounded,
  nothing written, nothing emitted). It is made:
  - for whoever asks for themselves with everything they hold
    (`POST /api/auth/once`: a person's session, or a key covering every
    scope its account holds; never an agent or a device, by the one rule
    `Identity::whole`, which setting a password and a name also uses);
  - or for the factory's owner, for whoever proves they can read the data
    folder: `/auth/once/asked` gives a challenge once, `/auth/once/proved`
    takes what the installation's secret derives from it
    (`link::own::Own::proves`). `mw open`, `mw login --here`, and
    `mw register` by the first-start owner do this from a terminal that
    holds no session.
  The link is traded by `POST /api/auth/link`, which takes no session. The
  page reads it from `#once=` in the address and removes it before
  anything else reads it (`ui/src/opened.ts`). Something new that signs
  somebody in without a password goes through a link, never through a
  route that makes or names an account by where the request came from.
- A daemon's start prints no link when nobody is at a terminal (a log
  would keep it) and opens a browser only at a terminal (`IsTerminal`) or
  with `--open`, so no test or check opens one. Every daemon a client
  check starts gets settings of its own (`ui/scripts/started.ts`), and a
  rig that starts the real program sets `XDG_CONFIG_HOME` as `run_in`
  does. After it prints which program it is, the daemon prints nothing
  until it stops: a rig stops reading there, and writing to a pipe nobody
  reads would end the daemon.
- Somebody a service outside vouches for is signed in to the account of
  that issuer and subject, found by those two and nothing else
  (`Identity::sign_in_external`, `crates/people/src/outside.rs`; stored as
  the pair, `theirs`). An email in the statement finds nobody. An address
  an account has that is not already this issuer's subject refuses the
  sign-in with that account unchanged; two issuers with one subject are
  two accounts. No route calls this yet; the route that does says which
  issuers are trusted. Linking an outside identity to an existing account
  needs that account's own consent and is not built.

## Scopes, people and groups

- A person may be given a scope beyond their role (`people::given`,
  `accounts.given`), one at a time, by whoever holds it and `admin`:
  nobody gives what they do not hold, and the factory's owner, who holds
  every scope, is given nothing. A given scope is one the system checks
  (`kernel::principal::named`: a line of `SCOPES`, an area of them, or
  `private:`, `emit:` and `extensions:` with a word), never `*`. It is
  added where a session's scopes are worked out (`Account::principal`),
  so it holds from the next request.
- Taking a given scope away also removes it from the keys that person
  made, in the same write, and whatever was told something because of it
  checks again (`Identity::withdrawn`); what was done with it stays done.
  Each change is one write emitting `identity.given` or `identity.taken`.
  Doors: `PUT`/`DELETE /api/accounts/{id}/given/{scope}`,
  `mw give <email> <scope> [--not]`, and Settings' People
  (`views/given.tsx`, `ui/src/given.ts`), which offers only what the
  daemon would allow (`holds` of `/api/accounts`).
- A group (`people::groups`) is people and agents (`user:<account>`,
  `agent:<name>`, never another group) with scopes its members hold
  beside their own. What a member holds is worked out wherever everything
  somebody holds is, never written into anything else: a session's
  (`Identity::principal_of`, used by `known` and `holder`), an agent
  credential's (`grouped_agent`, so a changed group reaches an agent at
  its next request), and inside a write as that write finds the groups
  (`groups::held_through`, used by `people::held`, `people::named`,
  `given`). `Account::principal` is the role plus given scopes only;
  something new that asks what a person holds now calls `principal_of`.
- A key holds what it was made with and is not widened by a group. Who
  holds less through a group change (taken out, the group narrowed or
  removed) has their keys narrowed in the same write
  (`given::narrow_keys`) and `withdrawn` emitted.
- Groups are changed by `admin`, nobody giving through one what they do
  not hold, and somebody is put in a group only by whoever holds all it
  holds. An agent never changes a group it is in or its own membership
  (`not_its_own`). Groups are held in memory for synchronous checks
  (`Groups`, `Identity::grouped`), changed one at a time and set before
  each write to the union of before and after, and after it to the new
  value (`change_or`), so memory never says more than the rows. Events
  are `identity.group.*`; membership is heard as `/people` is read
  (`channels:read`), what a group holds by `admin`. An agent removed
  leaves every group in the same write (`groups::forget_member`, from
  `Agents::remove`).
- The factory's owner holds `private`; a member does not. A test that
  keeps a table private and reads it as somebody else gives that key the
  scope.
- An agent holds no `blueprints:run` unless its slot says so
  (`AGENT_SCOPES`). A test that has an agent apply gives it `blueprints`.
- An agent's answer to what it was asked in a channel or a topic is said
  by the daemon as the agent, on the leave of whoever asked it there
  (`Channels::post_answer`, `Records::answer`): its own scopes are not
  asked and nothing is given to it. Nothing an agent does with its own
  credential reaches either; something new said for an agent goes through
  them only where somebody who could say it there asked for it.
- An agent never changes its own slot, even within what it holds
  (`agents::own`). Its own route refuses it before anything is read
  (`own::not_its_own` in `ensure`, `give`, `move_to` and an invitation),
  and a row of a table whose rows give agents what they hold (a field with
  `takes: each`) is refused to the agent that row brings up
  (`Collections::not_its_own`, checked in `Records::update` and `remove`,
  answered from what `own::install` registered, with the engine held
  weakly through `conductor::Downgraded`). Something new that changes a
  slot, or writes such a row, checks the same; tests in the API suite's
  `slotted.rs`.

## Keys

- A key made with no scopes holds its maker's. A test that wants a
  member's key asks for `MEMBER_SCOPES`.
- A withdrawn key stops what it applied (`people::keys`, then
  `Engine::applied_with` and `stop_withdrawn`) unless told to leave it
  (`leave`; `?leave=true`, `--leave`), and withdraws every key made with
  it, all the way down, in the same write (`made_with`). A rolled key
  keeps its id and scopes, so what it applied goes on.
- What runs with a key's authority is found by `Principal::key` and
  nothing else: something new that keeps a key's authority for later
  keeps the key's id in its principal, or a withdrawal will not find it.
- Who may withdraw or roll a key is `people::keys::theirs` (the key's
  maker, `admin`, or a key rolling itself), not the route.
- `actor.holds` answers for the actor it is given, so the actor must say
  who acted with what. A card moved, added or changed with a key names
  the key (`api::state::Said`, `Board::move_said`, `Board::add_said`,
  `update_said`; `by` on `board.item.*`), and so does a comment on a
  record (`Records::comment`, `source.actor` and `by`), because a key may
  hold less than its account. This holds for the board's routes, the
  command line with a key's token, and the bot acting for a message sent
  with a key (`Said` uses `Principal::key`). Removing a card, and
  anything else a key does to a non-card record, still names the account.
  A blueprint step names whoever applied it; an agent's own credential is
  not a key and names the agent. Before a blueprint calls `actor.holds`
  on another kind of event, the route that emits it must name the key
  too.

## Events from outside and callers

- An event emitted from outside (`api::routes::outside`) needs a key with
  `emit:<topic>`, and never one of the system's own topics. A key in an
  address (`/in/<key>/<topic>`) is accepted only if emitting events is
  all it holds: addresses end up in logs.
- A caller (`/in/<name>`, `people::callers`, offered as `api::callers`)
  is let in by its signature and nothing else, checked over the bytes as
  they arrived, before anything is parsed, with the `hmac` crate. A
  caller description whose signature would not cover the body, or that
  reads an unsigned time, is refused when it is configured
  (`Caller::refused`); never loosen the check itself.
- Who emitted an event is `source.actor` (`user:`, `key:<id>`,
  `caller:<name>`), set by the route and never by what was sent. A key is
  named by its id, since its name is chosen by its maker.

## Fields, rows and deputies

- Naming a record takes being able to read it (`Collections::may_name`,
  checked before `pointed` wherever a record is added or changed),
  because whatever reads the row follows the reference with its own
  authority. It is checked on what the write names, never on what a row
  already named, and answers the same whether or not the record exists.
  A new path that writes a record's fields checks it beside `may_set`
  (API suite `scoped.rs`).
- A field may say what setting it takes (`takes`, `fields::may_set`) or
  what setting one of its values takes (`takes: { on: shell }`,
  `Field::takes_of`, for a choice or a flag). Either is one scope or a
  list, all of which are checked (`studio::Takes`), and a refusal names
  every scope it takes and which are missing (`Takes::refused`). A value
  that is two things at once (a command or a secret's name) says both. A
  new path that writes fields checks it with the values. Re-declaring a
  table with less asked of a field takes each scope that was asked
  (`fields::still_guarded`).
- A list of scopes that something with more authority then gives to
  somebody says `takes: each` (`Field::takes_each`; `holds` of a fleet's
  `agents`). Whoever adds the record, or changes any cell of it, must
  hold every scope the list then says (`kernel::principal::covers`),
  checked against the whole record as written, because other cells (what
  an agent is told, how many there are) change what the scopes are used
  for. This matches `Agents::ensure`: an agent never has more than
  whoever changes it. A row that leaves the list unsaid is held to the
  table's `unsaid` (`AGENT_SCOPES` in `std/fleet`). Removing the word or
  widening `unsaid` on re-declaration takes `admin`. Never check one
  table's name in Rust instead.
- A table whose rows say what is done with another table's rows says
  `steers` (`Collections::steered`; `stages` steers `agents`). A row of
  it is added or changed only by whoever could write each row of the
  steered table as it stands (`fields::may_set` of each), at every way a
  record is written (`Collections::add`, `Records::update`, a card's
  change and move under the card's turn). Removing a row is not checked;
  re-declaring so it steers nothing takes `admin`. Tests: `scoped.rs`,
  the studio suite's `steered.rs`, `snapshots.rs`.
- A cell of an agent row that gives its program more says what that value
  takes (`permission: full` says `takes: { full: shell }`).
- A cell whose value becomes a param of a blueprint run as whoever
  applied the fleet is guarded at least as the param's own cell:
  `with` of `segments` takes `shell` and `secrets:use`, and may never set
  a cell of the row's own or what its stage says of itself (`fitted` in
  `deciding::segments`). An old `with` row is checked the next time it is
  written and refused when its card comes if it says what is not its own
  (`crates/api/tests/suite/steering.rs`). A cell that only names the
  blueprint a stage runs (`does`, `review`, `proposal`) takes what the
  row takes: a project's own blueprint runs only as it was vouched for.
- A field may record who last wrote another field (`writer_of: <field>`,
  `fields::stamp`), written by the factory at the doors every write of a
  record's cells goes through (`Collections::add_numbered`,
  `Records::checked`, `Board::change`), as an actor (a key as itself),
  never by the writer; a change of it alone is refused. It lets something
  with more authority check whether the writer may have it act (the
  environment a triggers row runs its pipeline under). Such a field is
  `quiet` and `hidden`. Something new that writes a record's cells goes
  through one of those doors.
- A project's own blueprint runs for a running instance only as it was
  vouched for (`crates/texts/src/vouched.rs`): by whoever applied the
  instance or something above it, or saved the file through the daemon
  holding what the instance runs with. A test that changes a project's
  blueprint under a running instance saves it through
  `PUT /api/projects/{id}/file` as the factory's owner or applies again
  with `take`; a file written with `std::fs` is not taken up. Something new
  that has an instance load a text by name calls as that instance
  (`Engine::apply_within`, or `apply_from` with the step), never
  `Engine::apply`, which is somebody asking and vouches for the files as
  they stand.
- A project's own file in its repository under a shipped name is used
  only when the project is trusted (`Library::passed_over`); one in the
  factory's place for its own (`@blueprints/`) always is.

## What a blueprint's step does: the factory's bot

- What a blueprint's step does to a record is shown as the factory's
  bot, never as whoever applied it, who stays the event's actor and `by`:
  who holds what is unchanged. `board.add`, `record.add` and `rows.start`
  set `through` (`board::told_as`, `Board::add_through`), and the history
  line shows the bot's current name with the blueprint after it
  (`· via std/lane-round`; `viaOf` in `ui/src/journey.ts`). Wording is
  `kernel::worded::as_called`, applied when a line is written and again
  when a history is read.
- An event a step emits itself (`emit`) carries no `through`; the history
  keeper finds the blueprint from the emitting instance (`studio::through`,
  `Engine::running_in_hand`, else `runs::blueprint_kept`) without waiting.
  Checked by `lanes.rs` and the studio suite's `stepped.rs`.
- Where a step is told whose work it records (`wrote`, an agent's actor
  only), the event and line name the agent (`kernel::worded::from_what`).
  `wrote` grants nothing; shipped blueprints pass only what is known, never
  a guess. Tests: `wrote.rs`, `featured.rs`.
- What the bot did lately (`GET /api/system/speaker/history`,
  `routes/bot.rs`, `views/botdone.tsx`) is read from history lines a step
  wrote (`Collections::done_through`) and what the bot said under its
  current name (`Channels::said_by_the_bot`), each filtered by what the
  reader may read (`Collection::may_read`, `Channels::readable`); nothing
  is kept for it. Tests: `botdone.rs`, `kept.rs`.
- What the bot may do is a ceiling. It holds nothing of its own: a command
  runs as whoever sent the message, narrowed to the ceiling
  (`kernel::Principal::within`, in `answer` of `talk::bidden`). By default
  the ceiling is everything its commands reach (`commands::REACHES`). It
  is kept in the journal's `kv` (`speaker`/`holds`) and changed by
  `PUT /api/system/speaker/holds` (`Speaker::hold`) by `admin`, giving
  only what they hold, in one write emitting `channel.speaker_held`. A
  refusal for a scope the asker has and the ceiling lacks names the
  ceiling (`beyond`). What a blueprint says as the bot runs as whoever
  applied it and is not narrowed. A new route the bot calls adds its
  scopes to `REACHES`. Bots of `std/bot` take no commands and have no
  ceiling. Tests: `commanded.rs`, `ui/scripts/botdone.mjs`.

## Private tables and who is told events

- A table may be private (`reads`, `studio::closed`), enforced at every
  way of reading. Something new that hands out a record calls
  `Collection::may_read`; a project file, `Projects::may_read_path`
  (follows links; `is_kept_from` is for listings). A new place a table
  keeps records as files is a case of `Collection::folder`.
- An event about a private record or file is emitted with
  `Source::taking`, and whatever hands an event to somebody calls
  `App::may_hear` or `Engine::hears`, never the topic alone. What an event
  takes is decided in one place, `Journal::may_tell`
  (`crates/journal/src/told.rs`): a scope, or `member:<what>` for
  something kept to its members, which no scope holds, `*` included
  (`kernel::principal::covers`), answered by what was installed
  (`Journal::members_with`, or `members_from`); nobody is told where
  nothing was installed.
- A run keeps the whole event that triggered it and what each step
  returned, so a run leaves the daemon only through `api::handed`: whole
  to whoever may be told that event, otherwise without the event's data,
  outputs and failure reasons (`kept_from`), its step logs refused as for
  a missing run.
- What the runner emits about such a run (started, each step's start,
  end, failure, retry, skip, each log line, a failed hook, how it ended)
  takes what the triggering event took, through `RunCx::about`
  (`crates/conductor/src/run.rs`), read once from the stored event
  (`run::took`, `run::Fired`), so a run resumed after a restart, or ended
  by a conductor, emits the same. `RunCx::source` and `ActionCx::source`
  take nothing and are for what a blueprint has its run emit (`emit`, an
  action's own event, what a step starts). A step on another device uses
  `ActionCx::about`. Something new the runner emits about a step or run
  uses `about`.
- Tests read the source for readings of `takes`, for runs handed out
  past `api::handed`, and for runner sources that take nothing, each
  against a list with reasons (`crates/api/tests/suite/told.rs`); the
  runner suite's `handlers.rs` covers resumed and left runs. What a
  handler does with what it heard is its applier's to emit and is not
  checked.

## Secrets

- The installation's secret (`<data>/installation.secret`) is in no
  event, log, error or copy. A key is derived from it for one purpose
  named in the derivation (`Own::derive`); a new use is a new purpose.
  What the factory keeps secret on disk (a blueprint's secrets, agents'
  sign-ins) is sealed with `Own::at_rest` (`link::kept`) under the name it
  is kept as; anything new that keeps a value a program is handed seals
  it the same way.
- A secret's value is in one file (`hosting::secrets`) and in the
  environment of a program as it is started (`Hosting::hand_over`), and
  nowhere else: not `ActionCx::env`, not a service's spec (which keeps the
  name), not an argument, event or error. A blueprint names one only in
  `env`, as `{ secret: <name> }` (`blueprint::secrets`); the name may be an
  expression, and an empty name sets nothing. Something new that starts a
  program for a step reads `cx.secrets()` at the start and keeps nothing
  (a package's program too: `Hosting::dress_handed`). A step sent to
  another device carries names only (`RunCommand.secrets`); the device
  hands over what it keeps where it allows its hub (`secrets:use` and
  `shell`, checked in `Member::perform`).
- Output of a program handed a secret is written with the secret's name
  in its place, by one rule (`hosting::unsaid`), before it reaches disk.
  For a program whose output goes to files (`proc::Launch`, steps and
  services) the wrapper does it, reading the values from its own
  environment, never an argument (`Launch::handed`,
  `wrapped::Asked::unsay`). For output read over pipes (a package's
  program, git as a key) `Hosting::dress_handed` gives what to remove from
  each line. Something new that starts a program with something handed
  does one of the two and has a test that prints the value and searches
  every file. Where this program cannot say where it is, a program handed
  a secret is not started. A step handed a secret that leaves something
  holding its output is over only when that is.
- No route, command or page reads a secret back. Listed are the name, when
  it was set and by whom (`secret.set`), and for `admin` where it is named
  (`/api/secrets?named=true`), read from agents' rows, services' specs and
  live instances' texts (`names_in` in `crates/api/src/routes/secrets.rs`).
  A new place a secret can be named is added there.
- Git at a remote as a key (`key` of `std/fleet`, passed to
  `std/lane-round`) is told, in this order, that it has no credential
  helper (`-c credential.helper=`, which shuts out the daemon user's own)
  and then that this program is the helper (`cli::credential`), both only
  in `kernel::git::git_as_key`. The step names the key in `env`
  (`FACTORY_REMOTE_KEY: { secret: ... }`); the action hands it only to the
  command that talks to the remote (`Lane::ask_remote` in `places`), and
  the helper gives it only to that remote's address (`FACTORY_REMOTE`).
  Something new that talks to a remote goes through `ask_remote` and
  refuses a remote that takes no key (`kernel::git::takes_a_key`). Tested
  with the real binary (`crates/cli/tests/suite/keyed.rs`).

## HTTP and the client

- A request is answered only when addressed to an IP address,
  `localhost`, or a name the daemon was told (`crates/served/src/guard.rs`).
  A test that uses another name registers it first (`/api/system/names`).
- What a request sent beyond what its route accepts is read to its end
  and discarded before the refusal is sent (`crates/served/src/sent.rs`,
  around every route), so the client can read the `413`. A route that
  refuses for size answers `413`; something new that takes bytes declares
  its limit as the attach route does. Reading on stops at `READ_THROUGH`
  times what the route had read. Tested by a hand-written client
  (`crates/api/tests/suite/sent.rs`) that controls when each part is sent;
  write such tests that way, never with a client that sends the body at
  once.
- A browser's session counts only with the key its page keeps
  (`x-page-key`, `studio::Came`). A check that calls the daemon from inside
  the page sends it (`localStorage['page-key']`). What the browser fetches
  by itself and only reads (a picture, an attachment, the event stream, a
  library's pages and a mockup's address opened in a tab of their own)
  cannot send a header, and counts with what its address carries beside
  the cookie (`by`, `shown` in `ui/src/api.ts`): a new address of that kind
  is made with `shown` and added to `state::shown`, and only if it reads.
  The cookie alone reads nothing. A cookie is sent to every port of the
  machine.
- A page the browser opens at its address and then follows the links of
  (a library's) takes `api::through::Opened`, not `Auth`: what the address
  said beside the cookie comes with who asked, and every link the page is
  written with that leads somewhere on the daemon says it again
  (`kernel::addressed::saying`; `served::library` for a page served, never
  for one written out). Opened where it says nothing that counts (typed,
  kept from before), it is sent to the client's page (`/?to=...`), which
  adds what it shows and sends the browser on, signing in whoever is not
  first (`ui/src/senton.ts`, which sends on only to such pages). Anything
  other than a browser opening a page is told to sign in, as anywhere.
  Tested in `crates/api/tests/suite/alone.rs`.
- An SVG attached is handed over to be drawn (`image/svg+xml`, inline): an `<img>` runs nothing
  in it, and opened by itself it is held by the policy every attachment is sent with
  (`sandbox; default-src 'none'`). A picture in somebody's words at an address elsewhere is drawn
  as a link and never fetched (`ui/src/pictures.ts`).
- The client's page fetches only from its own daemon
  (`served::client::CLIENT_POLICY`); something from another address is
  fetched by the daemon and served, or not shown.
- A library page is somebody's document and is sent with a policy that
  lets nothing in it run (`served::library::PAGE_POLICY`). Anything else
  served from a project's files needs the same thought.
- A mockup's page runs scripts and is never served as one of the daemon's
  own pages: everything under a way in (`served::mockups`) is sent with
  `sandbox` in its policy and sources limited to that way in. A way in
  (`/mockups/in/<name>/`) is not a credential and must never become one:
  it reads one folder for a while, checked only by `Ways::leads`.
## Devices and links

- A device has no credential but the key it proves. Its link is a Noise
  handshake and sealed frames (`link::sealed`); nothing a device sends is
  taken in the clear and no route takes a device's word. The handshake and
  ciphers are the `snow` crate's (built without its `std` feature, which
  would bring in `ring` and C); nothing here is a cipher of its own. The
  curve and ciphers are built optimised in the dev profile (root
  `Cargo.toml`), which tests that make many handshakes rely on.
- What a link is made of is the `link` crate, standing on nothing else;
  `gateway::own` and `gateway::sealed` re-export it. A pair other than a
  device and its hub handshakes under its own words
  (`sealed::offer_for`, `hear_for`), never `DEVICE_AND_HUB`.
- A session counts once the device has spoken in it (`Door::proved`): the
  first handshake message can be replayed by anyone, so ending a session
  on a handshake alone would let a stranger end it.
- A first handshake message carries a number above every earlier one
  (`link::own::Said`); the hub refuses one that is not, before anything
  else. A test that handshakes by hand counts up from above the time and
  within what a clock reaches (`ByHand::at`, `Said::sensible`).
- The hub's door handles one device at a time, from finding it to keeping
  its session (`Door::hello` holds `one_at_a_time` throughout), so a
  device being removed cannot keep a session.
- Refusals at the hub's door are counted against their source; a member's
  new first message and an invitation the hub holds never are. A test
  that sends many refusals uses many addresses or expects `Shut::Later`
  after five.
- A member runs what its hub sends with no more than it allows
  (`Member::within`): the command's principal is narrowed before anything
  runs, and what it starts inherits that. Never pass `command.principal`
  on as it came.
- What a member reads before the hub has proved itself is bounded and
  steers nothing (`gateway::http`): a maximum size on every read, no
  redirects, at most a minute for a wait asked in the clear.
- `device.pick` (`gateway::place`) chooses by what each device last
  reported and counts what it picked against that device for two
  heartbeats (`FRESH`); it is not a reservation. The rule is
  `place::choose`, a function of figures, tested as one.
- A member's `retry_at` is set only while it waits; during an attempt it
  is none, and a test's clock stays still then (`Rig::until`). Asking for
  the link is given `SILENCE` on the journal's clock.
- Link tests go through a `Carrier` (the gateway suite's wire, the API
  suite's `Recorder`), which neither reads nor reorders what it carries.
  Nothing in this repository relays traffic between two other
  installations; that is a hosted service and is not here.

## Other rules

- Nothing is started under an instance that is stopping or gone
  (`Engine::apply_from`, `Store::insert_instance`): a stop cancels first
  and counts children after, and a child is written only where the
  cancel has not happened. A test that reads what an instance started
  waits for that to be emitted, not for the request to return.
- A page is made one way (`studio::pages`); served and exported pages
  differ only in where links lead (`pages::body_with`). An export replaces
  only a folder it wrote (`.exported` lists it); `studio::site::put` never
  deletes anything not on that list, and a test that writes into an
  exported folder expects the next export to be refused.
