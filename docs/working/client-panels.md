# The web client: side panel and page heads

This file covers the client's side panel (Resources, Channels, Topics and the inbox panels, their
folds, search box and foot), workspaces as the client shows them, what a person arranges for
themselves, a project's outline, the right of a page's head with its gear, addresses as paths, and a
project's Repo and Git pages. Read it before changing anything in the side panel, a page head, or
how a thing is addressed. General client rules are in `client.md`; tables in `client-tables.md`;
checks in `client-checks.md`.

## What a person arranges, and where it is kept

- What follows a person is kept with their account: their themes and chosen theme (see `client.md`),
  and how they arranged the side panel (`people::arranged`: channel groups, each an id, a name and
  its channels in order; channels pinned to the top; and what is pinned at the top of Resources,
  each by the address that opens it). What is per window stays in the browser under the signed-in
  account: folds, open tabs per workspace, recently opened things.
- An arrangement is the caller's own and never taken from a request (a key's is its maker's). Read
  with `arranged:read`, changed with `arranged:write`, which people hold and agents and older keys
  do not. Checked where written (`people::arranged::checked`: ids one line and short, names
  non-empty and short, no duplicates, bounded counts, nothing beyond the three parts), case for case
  with the client (`ui/src/arrangedcases.json`).
- Each change is one write that emits `arranged.kept` (on `journal::HERE`, carrying it whole with a
  count `nth`, for `member:own:<actor>` only); a change that alters nothing is no write. Its table
  is the factory's own, kept in a copy with the accounts, moved with no project.
- **The daemon stores it and acts on none of it.** Nothing in the daemon may read a group or a pin
  to decide anything (who reads a channel, what a workspace lists, who is told what). A group is not
  something a channel has. An id naming something gone or unreadable is not refused (a channel may
  come back from the Trash); the client skips it.
- Client: `ui/src/arrangedkept.ts` reads and follows, over plain functions in `ui/src/arranged.ts`;
  changes go one at a time from the account's current arrangement (`arrange`, over `InTurn`); of two
  events, the later `nth` wins. The browser keeps a copy only to draw before reading, leaving out
  unlisted and gone channels (`copyOf`).
- What an older client kept in the browser (`client.channel.groups`, `client.side.lately` pins) is
  uploaded once, the first time the account's arrangement is read and only where the account has
  none (`/arranged/first`, so two windows at once keep the first), then removed from the browser. A
  page whose person may not change it changes it for that page only. `followed.mjs` checks it.
- A channel made from a group's menu (New channel…, for `channels:write`) uses the one channel
  dialog (`Ask` in `ui/src/app.tsx`, told the group by `madeInto` in `views/grouped.tsx`). Once made
  it is put at the group's end through the arrangement (`placed`), the group unfolds and it opens.
  One made in another workspace still goes in the group; messages about it are `madeSaid` in
  `ui/src/channelgroups.ts`.

## Pinned and Recents

- At the top of Resources: what is pinned, then what was recently opened (`Lately` in
  `ui/src/views/lately.tsx`). Pins are kept with the account; recents in the browser
  (`ui/src/latelykept.ts`, which hands both as one `Kept`). Both lists are functions of what was
  kept, what was read, and the panel's workspace (`ui/src/lately.ts`, with cases: `recentNow`,
  `pinnedNow`, `cleaned`, `heldStill`, `said`, `keptOf`).
- A thing is what opens as a tab, is of the factory's work, and is not always listed (`isThing`: a
  project page, board, table, library, mockups, files and a file, a blueprint, something running).
  Activity, Measurements, Members and the Trash are fixed lines, not things; channels, the start
  page, settings and package pages are not things either.
- A line is the thing's mark and its tab title (`tabTitle`), with whose it is beside it only where
  two lines read the same.
- A thing becomes latest when somebody opens or shows it (`onOpened`, told by `openTab` and
  `select`), never when tabs are restored or a tab shows because another closed. The latest eight
  are listed; more are kept, since the list is per workspace.
- What is pinned is not also in Recents. Pin from a project's row menu or from a line of either
  list. Remove from Recents (`unlisted`) changes nothing else.
- What is gone is not listed and is dropped from recents when a reading says so (a project
  forgotten, a table no longer declared, something running that names nothing). Pins are the
  account's and are skipped. One deleted from its own page is dropped at once (`forgot`,
  `forgetThing`). Something running is listed only once its project is known (`project` of `Known`
  in `ui/src/running.ts`).
- **The list holds still while the panel is hovered or has the keyboard** (`overPanel`,
  `heldStill`), so a pressed line does not jump from under the pointer. Only what can no longer be
  listed leaves meanwhile. A check reads either list with the pointer off the panel.
- Each line has its own container (`.lately-thing`), since checks find a project's tree by rows
  directly in the panel's list.
- Something new that opens as a tab is a case of `thingOf` and `projectOf` in `lately.ts`. Something
  new at the top of the panel that changes by itself holds still the same way. `lately.mjs` checks
  it.

## A project's outline

- How a project's things are arranged in the side panel is its **outline**, kept by the factory for
  everybody who reads the project (`kernel::outline` for the plain values, the `outline` crate for
  the keeper, `ui/src/outline.ts` for the client).
- A thing is one line the panel draws for a project, keyed by its search-id suffix (`board`,
  `table:<name>`, `library:<name>`, `mockups:<name>`, `environments`, `pipelines`, `blueprints`,
  `git`, `repo`, and `entry:<path>` reserved for a file). A folder is the outline's own: an id the
  factory gives (`f1`, never reused while kept), a name, and contents. It is on no disk.
- **It only says where a thing is shown.** Nothing a thing is known by changes, nothing moves on
  disk, and nothing in the daemon reads a folder to decide anything.
- It is kept in the factory's small durable state (`kernel::outline::NAMESPACE`, by project): taken
  into a copy (`studio::copy::KEPT`), left in place when a project moves to its own journal, kept
  with a project in the Trash, gone with one deleted for good; a project made anew under the same id
  starts with none. A project nobody arranged has none and is drawn as before (`laidOut`).
- **It is changed by operations, never written back whole** (`make`, `put`, `rename`, `remove`,
  `restore`, `name`, `forget`; `POST /api/projects/{id}/outline`, `mw project outline`), by whoever holds
  `projects:write`, each one write emitting `project.arranged` (on `journal::HERE`), which carries
  no outline.
- It is read as the reader may see it (unreadable tables, libraries and mockups removed, folders
  kept), so writing back what somebody saw would lose the rest. Naming something the caller may not
  read is refused in the same words as something missing.
- Each operation carries `seen`: things drawn at the top that the outline does not name, in drawn
  order, so a position given by what was seen is the one meant. A terminal sends none.
- What an outline must satisfy (`checked`) and what each change does (`apply`) are one rule in Rust
  and in the client, checked against `ui/src/outlinecases.json` (kernel suite and
  `outline.test.ts`). Something new is a case there first. Unknown node keys are skipped and kept.
- Client: `ProjectTree` (`views/projecttree.tsx`) draws it, read when a project is listed and again
  on `project.arranged` (`ui/src/outlined.ts`); unnamed things are drawn after it in the usual
  order. Folders fold like everything else (`outlineFold`, in Expand all; `outlineFoldsGone`), and
  the panel's box finds a thing by its folder's name (`projectItem` built from `laidOut`). Something
  new a project lists is a key of `keysOf` and a case of `Thing`. `outlined.mjs` checks it.
- **Menus** (`views/outline.tsx`, for `projects:write` only): a project's row offers New… (Folder…,
  a dialog that refuses a name a sibling folder has; Table…), Rename…, and, where arranged, Forget
  the arrangement… behind a question. A folder line offers New…, Rename…, Move to… and Remove folder
  (no question; its contents take its place; Undo calls `restore`). Other lines offer Move to…
  (`placesFor`: the top and each folder by depth, the current place marked; a folder is not offered
  itself or its contents; `indent` draws depth), and Rename… above it: a table's sets its title
  (`called.ts`), anything else's its shown name (below).
- **A thing may be shown as something else** (`name` of a thing node, set and cleared by the `name`
  operation; `mw project outline <id> --name board --to Roadmap`): for everybody reading the project,
  in the side panel (`usualOf` where unnamed), the page head and the tab title (`headWord` in
  `views/thingwas.tsx`), the project page's tiles and gear, and the panel's box. The page head shows
  what the thing is faintly beside it (`ThingWas`, `whatIs`: `Board · table board`). Nothing a
  blueprint, the CLI or the API reads by name changes: the key and the table are as they were. A
  table is refused (it has its title, so one name for people, not two). Blank clears it; Rename…
  saves the usual word as a clear. It is kept and hidden with its thing, so who may not read a thing
  never learns its name, and naming one they may not read is refused as missing. Something new that
  draws a project's thing by name takes it from `thingShown` (`ui/src/outlined.ts`).
- Each change goes through `changeOutline` (`ui/src/outlined.ts`), one at a time per project, drawn
  at once by the same `apply`, sent with `seenOf`. A line with a menu is in a `.nav-line`, so a
  check finds a line's button by `:scope > button.nav-item` or `:scope > .nav-line >
  button.nav-item`. `arranged-tree.mjs` checks the menus.
- **A blueprint places a thing in a folder** with `outline.put` (as whoever applied it,
  `projects:write`): `things` into the folder `folder` names by path (`Insights/Runs`,
  case-insensitive), creating missing folders at the end (`kernel::outline::placed_in`); idempotent;
  one write (`Outlines::put_in`). Something new that arranges from a blueprint goes through it.
- **Carrying a line** (`views/outlinedrag.tsx`, handed to the tree as `useCarried`): every placeable
  line says what it is (`data-place` on its own row), and only its own line is grabbed, never a Repo
  file or a blueprint under Blueprints. Where it lands is one function of line boxes measured at the
  start and the pointer (`landing`); a keyboard step (Alt and arrows) is another (`stepped`); each
  lands by one `put` through `moveTo`. Nothing moves during a carry; a folded folder is landed in at
  its end without unfolding.
- **The carry belongs to a tree, not to the outline** (`views/treedrag.tsx`, `useTreeCarry`, over
  `ui/src/treedrag.ts` with cases: `landing`, `stepped`, `steppedRows`). It reads lines from the
  page (`data-place` on a line's own row, `data-outline` on a line that holds lines) and is handed
  what a landing does (`land`), and optionally `step` and `allows`. The Topics panel uses it for a
  wiki's pages (`views/wikitree.tsx`). Something else drawn as a tree whose lines are carried uses
  it the same way. `outlinedrag.mjs` checks it.
- **A table is made from the side panel** with New… then Table… on a project or folder line
  (`views/tablemade.tsx`, over `ui/src/tablemade.ts`), offered to whoever may write the project and
  holds `blueprints:run`. It applies `std/table` for the project as the caller under key
  `table-<name>` and puts the table in the folder asked; a stop between the two leaves it at the end
  of the list.
- A name the project already has a table of is refused before asking, by the daemon's slug rule
  (`slugOf`, `reader::expr::slug`), and again by the daemon: `std/table` passes `anew` to
  `collection.ensure`, which refuses a table something else declared. Something new that makes a
  table for somebody passes `anew`. Where the project runs under environments, the environment is
  asked. API suite `tablemade.rs` and `tablemade.mjs` check it.

## Display names of projects and tables

- What everything else knows a project or table by never changes. A project's display name
  (`Project::name`; `id` is identity) is changed by `projects:write` (`Projects::rename`, `PATCH
  /api/projects/{id}` with `name`, `mw project rename`, Rename… on its row and page gear); refused
  if empty, over 80 letters, or another project's name in any case; one write emitting
  `project.renamed` (on `journal::HERE`), followed by every page (`store.ts`).
- A table may declare a display title (`title` of `collection.ensure`, kept in `studio::shown`).
  Whoever holds `projects:write` and may change the table may override it for everybody
  (`title_chosen`, `Collections::title`, `PUT`/`DELETE /api/collections/{project}/{name}/title`, `mw
  record title … |--usual`, Rename…). The override stands over what is declared and is carried when
  the table is declared again (`added::carry`); it emits `collection.titled`, taking what reading
  the table takes. Its name stays what `rows.read`, its address and `record_to` use.
- The client shows a table by `tableCalled` (`ui/src/called.ts`, its refusal rules matched with the
  daemon's). A board, library, mockups, Repo and Environments keep their fixed words and cannot be
  renamed.
- A table may also declare `description` (drawn at the foot of its head), and a field may say where
  it is among the fields a record is called by (`calls`), which `knownBy` uses before `title`. A row
  count uses the table's display name's last word, pluralised (`rowWords` in `ui/src/sits.ts`).

## Folds and Collapse/Expand all

- Everything the Resources panel lists folds. Parts are under headings in a fixed order: Pinned,
  Recents, Projects, Factory, Blueprints (`SECTIONS`; `Section` in `ui/src/views/navtree.tsx`).
  Pinned and Recents are drawn only when non-empty.
- In a browser that kept nothing, Projects, Factory and Blueprints start unfolded; everything under
  them, and Pinned and Recents, start folded.
- The browser keeps only what was turned from how a fold starts (`turned`, `client.turned`; names in
  `ui/src/folds.ts`: `SECTIONS`, `projectFold`). For the three parts that start unfolded, the name
  records that it was folded (`section:<part>:folded`, `STARTS_UNFOLDED`, `startsUnfolded`); for
  everything else, that it was unfolded. Code that unfolds a part for somebody hands it to
  `unfoldedAll`, never adds its name. Old names that no longer mean anything are dropped as they are
  read (`turnedNow`); a fold that is taken away is removed too.
- A project's name opens its page and unfolds it (`openedByName`); pressing it again does not fold
  it (its arrow does). Repo, a blueprint with parts and Members under Factory open their page only. Opening a channel
  unfolds nothing.
- At the panel's head, Collapse all forgets every fold of that panel (`foldedAll`; the other panels'
  folds are kept) and Expand all unfolds every fold of what was read (`resourcesFolds`,
  `blueprintFolds` in `ui/src/blueprintlist.ts`; drawn by `views/folding.tsx`): the parts, each
  listed project, each project's blueprints, the factory's blueprints with families and parts, and a
  project's files only where its folder was already looked at on this page (`hasLooked`). It asks
  the daemon for nothing. The factory's blueprint list is read when the panel first shows
  (`useBlueprintsRead`).
- The panel's search box holds open a part where it finds something (`held`); emptied, folds are as
  they were.
- **The Channels panel folds the same way** (`CHANNELS` in `folds.ts`, `views/grouped.tsx`): its
  branches (pinned, each group, each project's channels, archived) start folded, as does each
  channel's own fold; under a channel its people, agents and bots start unfolded. A folded branch
  shows how many unread wait under it; the archived branch shows how many it holds. Collapse all
  folds every branch and channel; Expand all unfolds every listed branch and channel with its
  members (`channelsUnfolded`, of `channelsDrawn`).
- A group just made is unfolded so a channel dropped in is visible; a missing group's fold is
  dropped once groups are read (`groupFoldsGone`), since group ids are reused. Folds are per window,
  not per account.
- **A check unfolds as a person would**: `unfoldAll` in `ui/scripts/folded.mjs` presses Expand all
  (twice, once more after the blueprints list), called by every check's `side(page, 'Resources')`;
  `unfoldChannels` likewise for Channels; `unfold` presses one arrow only if it is folded. Never
  press a fold's arrow to open it without checking it is closed. `folding.mjs` checks both panels.
- **Each panel is one tree**: what is under a heading hangs from the heading's arrow (`Section`
  draws it as `.tree`, with `under` for extra behaviour, as for Recents). All levels use one drawing
  of the lines in `folding.css` (`.tree`): every row's mark sits `--mark-in` from the row's left, a
  line runs down from its middle and across at `--row-half`, and each level is the same step in,
  shrinking when the panel is dragged narrow (`--mark-in`, `--tree-in`, from `--side-width` on
  `.sidebar`). So what is under a heading is `.nav > .tree > …`; a check finds a project by `:scope
  > .tree > [data-project]`. Something new that folds draws its mark where the others do.
  `joined.mjs` checks it.

## Workspaces in the side panel

- The side panel shows one workspace at a time, the reader's own choice. It is shown and chosen in a
  strip at the foot of both workspace panels (`SpaceFoot` in `ui/src/views/chosen.tsx`, one choice
  for both, fixed while lists scroll). There is no "everything" choice, and the workspace is no row
  among the projects.
- Its projects are listed directly (`listed` in `ui/src/workspaces.ts`); its channels are its own
  under no heading, then its projects' under each project (`gathered`). What belongs to another
  workspace is left out and counted.
- A project no workspace names (its placement not yet read) is listed under the usual workspace,
  where the daemon puts it: one function, `spaceOf`, with `usualOf` for which is usual (what the
  daemon says, else the first made). A channel uses `spaceOfChannel`: its project's workspace, else
  the one it belongs to, else the usual one; `choiceOf`, `holdsChannel`, `belongsTo` and `spaceNear`
  go through it.
- Until workspaces are read, lists are held to nothing. Which factory this is shows in the tab
  title, the phone's top bar and Settings, never on a panel head (`views/where.tsx`).
- The choice is kept in the browser under the signed-in account by workspace id
  (`ui/src/chosen.ts`). **No route is told which workspace somebody is looking at.** What a list
  shows is a function of what was read and the choice (`Held`: `holdsProject`, `choiceOf`,
  `holdsChannel`; `standing`; `unseen` and `waitingIn` for unread counts; cases in
  `workspaces.test.ts`).
- If the chosen workspace is gone, the panel takes the workspace of the tab showing in the last-used
  group (`spaceNear`), else the usual one. On load with nothing chosen, see `beginAt` in
  `client.md`. The result is kept once everything is read (`spacesRead`, `entered` in the store).
  Whatever chooses a workspace the page may not have read yet reads first (`readWorkspaces`).
- Choosing switches the open tab set (see `client.md`) and changes nothing else: no fold, group or
  pin; opening something of another workspace does not move the panel. Pinned or grouped channels
  follow the same rule (`arrangedOf` in `ui/src/channelgroups.ts`); a group with all its channels
  hidden is not drawn.
- Unread counts in channels the panel leaves out are shown on the strip by their own button
  (`OutOfSight`); pressing it shows the workspace with the most (`mostWaiting`). Each workspace in
  the menu shows its count too.
- The foot's menu is `RowMenu` (`views/rowmenu.tsx`, from `views/grouped.tsx`, handed `up`, `wide`,
  `label`, `from`): floats, opens upward, takes the keyboard when opened from a focusable control
  (arrows, Home, End; Escape gives it back), and is closed by pressing the strip again. It lists
  what existed when opened, so a check waits for a new workspace to be read before looking for it.
- The menu reads top down (`offered` in `views/chosen.tsx`): the current workspace with its project
  count, who is signed in, the workspaces, New workspace (`forSpace` in `views/spaces.tsx`), then
  set apart Settings (opens Settings at the workspace's part: `settingsPart`), Invite members only
  for somebody who may invite (`can_invite`), and Log out (`signOut`).
- Settings has a part for the panel's workspace (`views/spacesettings.tsx`, first after the
  profile): its name is a field saved on Enter or blur, sent once and only if changed, with the
  daemon's refusal shown under it, following terminal renames while not being edited. Removal is at
  the very foot of Settings (`SpaceRemoval`, Remove this workspace), only while another workspace
  exists. Both need `projects:write`; others see the name as text. The removal question
  (`RemoveSpace`) asks where the contents go (other workspaces, the usual one first), always sends
  that, and keeps its button closed until the workspace's name is typed exactly (`removeClosed`,
  with cases). That confirmation is the client's only: `mw workspace remove` and the route take an
  id.
- **Moving something from its row is one line, `Move to…`.** It opens a submenu: for a project each
  workspace (current marked) and New workspace, never "none"; for a channel (`belongsItem` in
  `views/spaces.tsx`) each workspace and then, under a heading, each project. Lines and questions
  are plain functions with cases (`ui/src/moveto.ts`: `projectPlaces`, `channelPlaces`,
  `projectQuestion`, `channelQuestion`). Choosing asks a question (`MoveTo`) that names the thing
  and the destination and says nobody is let in or kept out; the daemon's refusal is shown in it.
  Needs `projects:write` (project) or `channels:write` (channel). Channel groups are the reader's
  arrangement, not moves, and stay as menu lines.
- **A submenu is a capability of any `RowMenu`** (`sub` of `Offered`): it says so (`aria-haspopup`,
  an end mark), opens on hover, press, tap, Enter or Right (the last two move focus into it); Left
  and Escape close it and return focus, leaving the menu open (that Escape is heard before anything
  else). Placed by `besideAt` (`ui/src/submenu.ts`, with cases): right of the menu if room, else
  left, first line level with its opener, moved up to fit, and over the menu where neither side
  fits. It floats, moves nothing, is placed before it is drawn, and survives the pointer crossing
  other lines on its way (`towards`). `movedto.mjs` (run by `spaces.mjs`) checks it.
- A new side panel list of projects or channels is handed `held` and counts what it leaves out with
  `unseen`. Something added from the panel is offered the workspace showing: the channel dialog asks
  the workspace above the name (never "none"), caret in the name; the project dialog asks none and
  makes it in the panel's workspace, said in its head (`madeIn` in `views/newproject.tsx`); a
  terminal can say one (`project create --workspace`).
- The factory's blueprints are not held to the workspace; the Factory's lines are always listed.
  Tabs that no longer exist are dropped from what a browser kept (`TAKEN_OUT` in
  `ui/src/tabskept.ts`, `tabsOf` in `ui/src/context.ts`).

## Activity, Measurements and Agents narrowed

- These three pages (the agents being the Members page's Agents tab, whose choice is in the page's
  head while that tab shows) are of the panel's workspace, and each head has the same choice within it
  (`Narrow` in `ui/src/views/narrow.tsx`): the whole workspace by name (the default), each of its
  projects under a heading, and last, set apart, All workspaces (everything in the factory,
  including what was applied for no project and the factory's own log and figures). A project of
  another workspace is no line; choose that workspace at the foot instead.
- Each page is of the workspace whose tab set it is in; if it is the showing tab in both, it follows
  the panel.
- The choice is the reader's, kept in the browser per workspace (`ui/src/narrowed.ts`,
  `client.factory.shown:<actor>`). Plain functions with cases in `ui/src/narrowing.ts`: `shownBy`,
  `settled`, `cleaned`, `lines`, `fromBefore` (an older single-project choice is read once as its
  project's workspace's choice, then removed).
- Until workspaces are read, the pages read nothing (`waiting`); if they cannot be read
  (`spacesAsked`), everything is shown. All workspaces is a line only where it shows more (`more` of
  `Narrow`; `leftOut` in `ui/src/agentbands.ts`); with one line, nothing is drawn.
- `narrowTo(project)` in the store (from a project's page or gear) chooses the project and moves the
  panel to its workspace if needed: the one place opening something moves the panel. It narrows
  first and then opens, so the page opens in the right tab set; keep that order.
- **Nothing in the daemon learns which workspace somebody is looking at, and no route takes a
  workspace.** Pages ask by projects. Agents are read whole and held to the page's projects
  (`banded`; the factory's own only under All workspaces). Activity asks with `project` repeated
  (`?project=a&project=b`, `named` in `crates/api/src/routes/work.rs`); measurements with the label
  repeated (`labels=project:a,project:b`, `carries` in `crates/journal/src/metrics.rs`). A workspace
  with no project asks for nothing. Permissions are as for one project (API suite `several.rs`,
  journal suite `labelled.rs`).
- A late answer for a previous choice is not drawn (`asked`); measurements mark whose they are
  (`data-of`), which checks wait for. Something new on these pages read by project is handed the
  project list (`only` of `shown`). A check about what is nobody's chooses All workspaces first.
  `narrowed.mjs` checks it.

## The foot and Help

- The foot (`SpaceFoot`) is there before workspaces are read, because Help is at its right
  (`ui/src/views/help.tsx`): a round button whose menu holds a support link, what to say in a
  report, and what is new.
- The support address is written by whoever builds the product in `brand.toml` (`support`, the one
  key that may be empty: `OPTIONAL` in the `brand` crate's build script, `brand::SUPPORT`, sent as
  `support` of `/api/meta`). Left empty, as in this repository, no such line is drawn and no address
  of anybody's appears in the application. What may be drawn is decided by `supportLink`
  (`ui/src/help.ts`): a web page or a mail address, never anything a browser would run; opened in a
  browser tab, nothing fetched by the page.
- What to say in a report (`whatToSay`: program and version, client build, daemon's system from
  `/api/fleet`, browser) goes on the clipboard only.
- What is new is `docs/releases.md`, built into the program (`api::releases`, `GET /api/releases`),
  so installs have it offline. A release is a `## ` heading; the menu's list is read from the
  headings. A test fails if the newest heading is not this program's version
  (`crates/api/tests/suite/releases.rs`). The notes open as a page in a tab (`views/releases.tsx`,
  made like a document page via `studio::pages`, drawn by `WholePage`, which scrolls the requested
  release into view: `shows`).
- The dot on Help shows while the newest release is not the one this browser last opened the menu at
  (`ui/src/releases.ts`, kept per account).
- Both foot menus are `RowMenu`, which takes non-pressable lines (`Said`: headings, who is signed
  in; skipped by the keyboard), a drawing before a line (`mark`, with room kept so names align), and
  a style (`look`). Lines are found by position, not by name (a workspace may be called Settings). A
  line is as wide as its menu and its words wrap (`foot.css`).
- Drawings are a few strokes each in `views/signs.tsx`, never from the column icon set
  (`icons:set`), which is loaded only where a column icon is drawn. A workspace is drawn by its
  first letter (`Face`, `letter`); a picture would be everybody's and so the daemon's to keep.
- Something new in either menu that leads outside the daemon is an address from `brand.toml`, or it
  is not drawn. `foot.mjs` checks it.

## Rows and heads of the side panel

- A row is as wide as the panel: its name shrinks (`.nav-row .nav-item` in `style.css`), and what
  follows the name (not-trusted mark, row menu) stays whole inside the row.
- A panel's head shows what the panel is (Channels, Topics, Resources, Inbox, Extensions), whole at
  the usual width beside every head button; dragged narrow, the title and the search button's words
  give way before anything pressable leaves the panel. Head buttons are one group at its right
  (`.brand-acts`), which on a phone goes under the title. Something new at a panel's head goes in
  that group.
- A check finds a panel by its `aria-label` on its `nav`. `spaces.mjs` and `profiles.mjs` check
  heads by boxes.
- On a phone, controls in rows and headings are finger-sized and do not extend past the room around
  them (`.nav h3 button`, `.nav-row.inner` in `phone.css`); `sought.mjs` checks every panel at 390
  and 412 wide.

## The panel search box

- Each panel that lists things (Resources, Channels, Topics, inbox) has one box under its head
  (`PanelSeek` in `ui/src/views/panelseek.tsx`) that narrows everything the panel lists: in
  Resources pinned and recents, the Factory lines and Members, the Reference, the factory's
  blueprints (`sought`), and each project with its contents and files as far as read; in Channels
  each channel by name (`calledOf`, `hashed`), with groups, pinned and project branches; in Topics
  each topic by title, summary's first line, status and project (`topicsFound`); in the inbox each
  line by sender, place and words.
- **It narrows what the panel has read and asks the daemon nothing**; a fold it holds open reads
  nothing either (`onOpen` of `Fold` is not called while `held`).
- Matching is one plain function (`narrowed` in `ui/src/panelsought.ts`, with cases) over a tree
  each panel builds (`projectItem`, `keptItems`, `channelItems`; inbox: `linesFound`), using
  `wordsOf` and `says` from `blueprintlist.ts`: every typed word must appear in the thing or in the
  name of a container that carries its name (`shop board` finds Shop's board; Repo or Blueprints
  carry nothing).
- A thing that matches itself shows as left, folds and all; a container that matches only through
  its contents shows only those, held open (`held`, arrow inert), and returns to its folds when the
  box is emptied (by hand or Escape, which is then the box's alone).
- Views read results through `Sought` (a context, in Resources) or compute them (`GroupedChannels`,
  `InboxSide`) and ask `shows` and `holds` by the ids `panelsought.ts` gives (`ofProject`, `ofKept`,
  `ofChannels`). Something new a panel lists is a case of its tree with an id, or a search hides it.
- Empty headings are not drawn; where nothing matches, one line says so (`NothingFound`). What was
  typed is kept per panel and per account, in memory only: back after switching panels, gone on
  reload.
- Files under one project's Repo are drawn two hundred at a time with a line that draws the next
  (`filesBounded`, `MOST_FILES`, `drawnFirst`, `moreSaid`). The narrowing applies after a held-still
  list is held.
- **It is the panel's only box, and searching everything starts from it.** While empty it shows the
  search shortcut keys at its right (pressing them opens search empty; `seekHint`; not under a
  finger), in room it always keeps (`.panel-seek.keyed`). While something is typed it offers one
  line, Search everything for "…" (`everywhere`, words whole up to `MOST_SAID` letters, wrapping),
  which, Enter, and Ctrl+K in the box open search with the text (`searchEverything`, taken up by
  `paletteWith`). Ctrl+K elsewhere opens it empty. That line belongs to the box, not the list.
  Something new that searches more than the panel read starts from the box, never a second box.
  `sought.mjs` checks it.

## The right of a page's head

- The right of a page's head is one component on every page of a thing (`PageActs` in
  `ui/src/views/pagehead.tsx`): when it was last changed, Follow where the thing can be followed
  (`FollowButton` in `views/followbutton.tsx`, for the tab `followableOf` in `following.ts` names:
  a board, a table, a library, a project), what shares it, its address, its pin, and
  a gear. The title and the words beside it are one item (`Titled`, `.head-title` in `head.css`),
  centred on the head's line, the small words on the title's baseline.
- A page says what it is (`tab`) and hands the rest; nothing is written per kind of page.
  Table-backed pages (table, board, library, mockups) hand it through `TableActs`
  (`views/tablehead.tsx`), a file through `FileActs` (`views/filehead.tsx`).
- **A line is drawn only where pressing it does something**: no Share where nothing shares the thing
  (a private table says so beside when it changed), no gear line that does nothing. New items are
  lines the page hands (`menu`, `share`, `more`). A panel a page opens beside itself is one extra
  button, its `tool`, last in the row and lit while open (a document's comments, details and
  history).
- The group ends the head's first line (`.head-break`); the page's own head buttons go on the next
  line at the right (a `grow` after the break), so what they open hangs from their right edge. A
  page with nothing after it says `last` (the board, whose view tabs are their own line). Not drawn
  in the drawer, nor on non-things (a channel, Activity, Measurements, Members, Settings); a
  blueprint page and something running have none yet.
- **The gear is the one thing to press for what else is done with a page.** A new action on a page
  is a gear line before it is a button. Tables that sit beside a table are lines under a
  non-pressable heading (Its tables), each with its side panel mark and its row count as `hint`,
  opened in the drawer; `TableActs` adds them for every table-backed page. Board actions follow
  (`tools` of `TableActs`: add card, add column, arrange columns, Layout); the board's head has no
  Tools or Its tables buttons.
- Which lines exist and their words are plain functions with cases (`besideLines`, `boardLines` in
  `ui/src/gear.ts`): a line's explanation is its `hint` where it fits at the menu's width, else a
  `note` under it. They stand under the gear's own lines and over "last changed", which stays last.
- What a line opens that is more than a line (a view's layout) hangs under the gear (`hung` of
  `PageActs`, drawn in the break; `HungLayout` in `views/layouts.tsx`): it closes anything else open
  first (`pressOutside`), is no taller than the room below, and closes on an outside press, the
  gear, or Escape (then the panel's alone; it returns focus to the gear).
- **Something that captures a key stops listening in the commit that removes it**
  (`useLayoutEffect`), or a key pressed in that moment goes nowhere.
- A table's page keeps its Layout button beside its view tabs; a table in the drawer (no gear) keeps
  both its buttons (`aside` of `ViewsHead`). A check goes through the gear with
  `ui/scripts/gear.mjs` (`gearLine`, `tablesRead`); a gear opened before rows were read says `being
  read` until reopened.
- Share is on a library and on a document of one that keeps the mark (`views/shared.tsx`): the
  library as pages, written out by the shipped blueprint applied from there; a document's `public`
  switch, which writes the record and its file's head, closed while the page has unsaved changes.
  What Share offers is read when pressed and not followed.
- A file's gear: a copy beside it (`copyOf`, of what is saved, never over an existing file),
  download, its path, its history where it is a document, and delete behind a question
  (`projects:write`). Nothing moves or renames a file: a document is known by its path, and comments
  would be lost.
- **CSV.** A table and the board are written out and read back by the browser alone
  (`ui/src/csv.ts`, with cases; `views/csvdialog.tsx`): every record whatever the view, `id` then
  each declared field by name; lists and multi-record references joined by `; `; references as ids;
  nothing of a field kept for whatever looks after the record. Read in, a line whose `id` is a
  record changes only the cells that differ; a line without `id` is added; an `id` that names
  nothing is reported and skipped. So reading a file straight back changes nothing. What a file
  would do is shown first, then done a record at a time through the normal add and change routes, a
  refused line reported with its reason while the rest go on; nothing is rolled back. Reading in is
  offered to `mayChoose`. A new kind of cell is a case of `cellOf` and `valueOf`. Libraries and
  mockups have neither.
- The pin is the side panel's (`pinThing`, `isPinned` over `lately` in `ui/src/latelykept.ts`),
  never a second one.
- **When a thing was changed** is worked out from what its page read (`ui/src/edited.ts`, with
  cases): the later of the table's own write and its newest record's. Who, from the last line of
  that record's history, read once, then from `collection.history` events. A line names somebody
  only where it matches the moment the page read. A table with no history, a view saved, and a file
  another program wrote name nobody. On a phone it is shown in the gear's menu, not the head.
- Head menus are `RowMenu`: `Said` for text-only lines (`note` for a long sentence), `look: 'grave'`
  for deletes, a `mark` before every line. Three options exist only where handed: `hint` (faint
  words at a line's end), `on` (a switch that leaves the menu open; `closed` with a reason), and
  `fit` (keep a mixed-height menu on screen). A menu taller than the room under its opener stays
  under it and scrolls (`UNDER` in `views/rowmenu.tsx`). The gear and Share menus widen with
  `--size` and never exceed the window.
- Before the gear or Share opens, anything else open over the page is told as an outside press would
  tell it (`opens` in `pagehead.tsx`). The head's own drawings are from `views/signs.tsx`.
  `head.mjs` checks it all, with `headgear.mjs` and `headaddress.mjs`.

## A topic's page head

- A topic's page (`views/topicpage.tsx`) has no `PageActs` yet. Its head is the title (written in
  place), the status (`StatusPick`), where it is and how many replies, a `grow`, then its own
  buttons: Follow (`FollowButton`, offering This page and This page and the pages under it; a
  channel's head has it too, before its participants), then a control of how the page is shown
  (Appearance) first after the `grow`, then Edit (on Article, and on a thread with a post, to
  whoever may change it, while the post is read), Add summary, Write an article or Add post where
  `partsOf` offers them, then Close or Open again, and on a thread History last.
- An article's tabs, Article, Talk and History, are the head's last line (`Tabs`, `.view-tabs
  tabs-ruled`, `role="tablist"`, called "Article, talk and history"), as a Git page's are; each tab
  is a button reached by Tab, its panel a `role="tabpanel"` it controls. Talk counts the replies
  faintly, as a Git tab counts. A thread has none. The article's panel has a slot for a column of contents at its left and boxes at its right
  (`contents`, `boxes` of `Article`), each drawn only where handed.
- **Contents** (`views/topiccontents.tsx`, plain values in `ui/src/contents.ts` with
  `contents.test.ts`) fill the left slot once an article has `CONTENTS_FROM` (3) headings. Headings
  are what `prose.ts` reads (`#` to `###`); each has a slug, unique within the page, made the same
  way from the article read (`headingsOf`) and from the editor (`Writer.headings`), so that the two
  find the same heading. Beside the article where `.topic-article` (a size container) is at least
  800px wide, by a container query in `contents.css`, so the article is laid out once; the column
  is sticky in the view's frame (`.view`), which is what scrolls, so `.topic-page` drops its own
  overflow while it has Contents (`reading.mjs` holds it in view after 3000px). Narrower, a box above it, folded by the tab (`contents:open`). Hidden is `contents` of the reader's appearance, kept with the
  account (`people::appearance`), so it holds after a reload and in every article.
- `Prose` draws a heading in a `.prose-heading-line` with `data-section` and whatever `Headed` (a
  context of `ui.tsx`) hands beside it; only the article read provides it, so channels, replies and
  summaries draw headings as before. Beside each: a link mark that copies `sectionLink` (and takes
  the page there) and, to whoever may change it, Edit (`edit(slug)`), both shown on hover or focus
  (always on `hover: none`).
- Where the article is taken is the page's (`useSections`: `jump`, kept by the tab, and the number
  taken already), scrolled to once the heading is drawn read or in the editor (`sectionsIn`). It is
  said in the address bar as `#s/<slug>` (`Shown.section`) while Article shows, and forgotten when
  another tab shows. `DocField` takes the editor's handle (`onReady`), puts the cursor at a section
  (`toSection`), and on Save or Cancel says the section the cursor is in (`onBack`, from
  `sectionOf`), which the page takes the article to.

## Addresses are paths

- **A thing's address is a path**, with how it is shown after the `#`: `/shop/test-runs#newest`. One
  grammar of plain values with cases (`addressOf` and `tabAt` in `ui/src/tab.ts`, `tab.test.ts`).
- The path uses names people know: `/<project>` its page, and under it `board`, `repo` and
  `repo/<path>`, `git`, `environments`, `pipelines`, `blueprints/<name>`, `<table>` (a table,
  library or mockups by name; `tables/<table>` for one named like a reserved word) and
  `<library>/<path>` (a document). Not of a project: `run/<id>`, `instance/<id>`, `inbox/<line>`,
  `reference`, `reference/<entry>`, `blueprint/<name>`.
- `blueprints/<name>` opens the project's own blueprint of that name, its fork of one that ships
  among them, as the file it is (as its Blueprints line in the side panel opens it), and that
  file's tab says this address, wherever the file is kept (`@blueprints/` or a folder of its
  repository). Which name is whose own comes from what the page read of what each project uses
  (`/api/blueprints/used`, the `Owns` of `tab.ts`), read first if the project is not there yet. A
  name the project has nothing of is what ships under it, opened for the project. What ships,
  opened for a project whose fork stands in for it, is `/blueprint/<name>?project=<project>`.
- After `#`: a view by name (`ui/src/viewshown.ts`), a line (`L12`), a document's comments
  (`comments`), a card and a comment of it (`shop-17/c-482`), or a row of another table by id
  (`shop-test-runs-4`, valid only for that project and table), which opens the row's form over its
  table (`showRow` in `views/records.tsx`) and is copied from that form's head (`rowLink`).
  Of a topic: `talk`, `talk/r-<n>`, `history`, `history/<n>`, and `s/<slug>`, a heading of its
  article; under `s/` so that a heading called History or Talk is never taken for the tab, and
  only what is a slug already (`isSlug`) is one.
- Which kind a name is, and where a library keeps documents, come from the project's tables, read
  first if needed (`{ reads }`).
- **The address bar follows the showing tab** (`ui/src/address.ts`, `history.replaceState`), so Copy
  link and the bar agree; tabs with no address leave `/`. An address is opened as a tab once
  everything is read (`entered`) and again when the `#` part changes. A link on the page to one of
  this factory's addresses is opened the same way, never followed (`opens`, a document listener on
  `<a href>`); write such links with `hrefOf`. Opened while signed out, it waits and opens after
  sign-in.
- An address in the older form (`/#/table/shop/x`) still opens (`fromKept`) and the bar then shows
  the new form; pins are stored in the older form (`keptAs`).
- **No project may be named what the factory's own addresses start with**
  (`kernel::paths::RESERVED`: daemon routes, client asset folders, grammar words, and words kept for
  future factory pages): refused on create and rename with a reason, matched word for word with
  `RESERVED` in `tab.ts` by the client's test and by API suite `addresses.rs` (which also checks
  that every grammar path is answered with the page and no daemon path is taken for one:
  `missing_asset` in `crates/served/src/client.rs`). A project created before its name was reserved
  uses the older address form.
- An address lets nobody in: a page shows what its reader may read, or says the thing is missing.
- Something new that opens as a tab and is a thing is a case of `addressOf`, `tabAt` and `thingOf`
  (`lately.ts`). Something new the daemon serves under its own path adds its first word to both
  reserved lists.

## A project's Repo

- A project is not assumed to be software. What it has is listed directly under it (`ProjectTree`:
  board, tables except those beside another, libraries, mockups, the blueprints it uses, and its
  files where its folder was given), with no word for what kind of project it is. Its folder is its
  Repo: a page of the folder as it is, saying nothing of source control (line `Shop repo`, `Its
  repo` on the project's page).
- Every project is one folder, and says whose it is (`folder` of `studio::Project`: `given` by
  whoever added it, or `own`, made by the factory). Worked out each time from where the folder is
  (`kernel::whole::within`, `Projects::folder_of`), never stored.
- The side panel lists Repo only for a `given` folder; files are still reachable from the project's
  page, by address, and from pins and recents. A daemon that does not report `folder` gets Repo for
  every project.
- New file and New folder are buttons in the Repo page's head and gear lines (made in the folder
  shown) and menu lines on the side panel's Repo and folder lines (made there)
  (`views/repomade.tsx`; logic in `ui/src/repomade.ts`, with cases). Needs `projects:write`. A file
  is written by `PUT /api/projects/{id}/file` with `new` (`Projects::write_new`); a folder by `POST
  /api/projects/{id}/folders` (`Projects::make_folder`, `mw file mkdir`); they emit
  `project.file.changed` and `project.folder.made`. Both refuse an existing name, and resolve paths
  as every project path does. A new Markdown file starts with a heading of its name.
- The add-project dialog asks for a folder under the name (`FolderField` in
  `ui/src/views/folder.tsx`) only of `admin` (taking up a folder exposes it to whoever reads the
  project), sent as `path`; the daemon's refusal is shown and the dialog stays.
- A check about a project's files makes its project with a given folder (`project create --path`);
  one passing through reaches them from the project's page (`lists.mjs` checks both). Checks find a
  thing by its project (`[data-project="shop"]`) and press folds by exact name.

## A project's Git page

- A project whose folder is a git repository of its own (`.git`, `studio::Project::repository`,
  checked when handed out, not stored) has a **Git** page (`{ kind: 'git' }`, `views/gitpage.tsx`
  over `ui/src/gitpage.ts`), opened from its page (Its Git) and by address. It is listed in the side
  panel, right before Repo, only for a `given` folder (`gitListed`): a factory-made folder is a
  repository only so work can branch it.
- **What git says is the application's.** Branch and upstream state, changes and staging, a branch's
  commits, branches and remotes are read by `places::seen` through `kernel::git::git` only
  (questions and parsers in `kernel::git`: `STATUS`, `COMMITS`, `REMOTES`, `status`, `commits`,
  `remotes`, with cases), served by `GET /api/projects/{id}/git` to whoever reads the project, and
  `mw project git`. A requested branch must be one listed, whole. Remote URLs are shown without
  anything before an `@` in a `://` URL (`git::shown`), where credentials go.
- **Git emits nothing as it changes a repository**, so the repository's folders are watched (`.git`,
  `refs` and below, by `hosting::Watch`: `places::seen::followed`, a task per project in
  `api::routes::git::follow`), fully only once somebody has looked at that repository since the
  daemon started (`places::seen::Looked`), since agents commit all day. A folder that is not a
  repository is watched for one being made. Changes emit `project.git.changed` (passed on, not
  kept).
- **A repository is followed once, whatever for**: the Git page and a pipeline's branch trigger
  (`branches.follow`) share one follower per project (`places::seen::For`, `places::seen::Heard`).
  Something new emitted as a repository changes is emitted by that follower; the API suite counts
  followers (`Watch::followers`, `gitpage.rs`).
- The page reads again on that event and on `project.file.changed`, one reading at a time
  (`OneAtATime`). A working-tree file written and not staged by another program is seen on the next
  other change, or by Refresh.
- **What a service keeps of the repository is a package's** (`repository:` in a manifest,
  `agents::shown`; see *What a package shows of a repository* in `docs/extensions.md`): which
  remotes it serves (a condition on `remote.name` and `remote.url`) and parts, each an action that
  takes no required input and gives `rows`, columns and the link field. A part runs as a step would,
  in the project's folder, as whoever runs the daemon; reading one needs `extensions:<package>`, and
  happens when its tab is first shown and on Refresh, never because the repository changed. A part
  that returns no `rows` is shown as unknown (with how the package is installed and signed in),
  never as empty.
- **The page is in tabs** (`ui/src/gittabs.ts`, with cases): Changes, Commits, Branches, Remotes,
  then one per package part, named as the package names it; a part the reader may not use is shown
  with the reason. A tab is known by a word (`changes`, the part's name, or `<package>.<name>` on a
  clash), kept by `useKept` and carried after `#` (`/<project>/git#pulls`); requested with
  `showGit(project, at)` (`ui/src/gitshown.ts`). Rows show only their declared columns; a link only
  if `http` or `https`.
- Nothing names a service in the application; a package's parts (for example the GitHub package's
  `pulls`, `issues-open`, `checks`) are its own. Something new shown of git is a `kernel::git`
  question with a case; of a service, a package part. Actions from the page are not built and would
  be package actions.
- **What a service knows of each piece of work is a table a package's blueprint keeps** (for example
  `github/work`): a row per card whose branch reached the remote, beside the board and about the
  card (`beside: board`, `about: card`, `by_hand: false`). Nothing in the application names it. A
  card shows its row in its Details (`views/cardrows.tsx`) and in the table's tab beside them, both
  over `cardtabs.ts` (`aboutCards`, `rowsAbout`, `cellsOf`). It is applied by hand. `gitpage.mjs`
  and `gitwork.mjs` check it.
