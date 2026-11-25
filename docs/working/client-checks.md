# Checking the web client

This file says how the client in `ui/` is checked: the unit tests and build check, the
headless-browser walkthrough and the scripted checks in `ui/scripts/`, and what each check
script covers. Read it before changing anything a user sees, to know which checks to run. The
rules the checks lean on (waiting for the drawer, making an order rather than waiting longer)
are in `client.md`.

## The two kinds of check

- `cd ui && npm run check` runs the type check, the unit tests (`*.test.ts` beside the code,
  which hold the plain functions to their cases) and a build with `--held` (which fails where a
  bundled package is missing from `THIRD-PARTY-NOTICES.md`).
- There is no screen on the build machine, so the client is also driven in a headless browser
  and the pictures it saves are read. Each check script starts a fresh daemon of its own, drives
  one or more pages, takes pictures and stops the daemon however it ends.
- `cd ui && npm run walkthrough` runs the first-run story (`scripts/walkthrough.mjs`), about
  forty pictures in `target/shots`.
- Every other check is run the same way, from `ui/`, against the debug build:

  ```sh
  DAEMON=../target/debug/factory node scripts/<name>.mjs ../target/shots
  ```

- A check is of the client built from the files beside it: build the client (`cd ui && npm run
  build`) and the daemon first, or the check fails at once saying so (`ui/scripts/built.ts`).

## Environment

- `BROWSER`, `BROWSER_LIBS` and `BROWSER_FLAGS` are described at the top of
  `ui/scripts/walkthrough.mjs`: `BROWSER` is a Chromium binary (default: the newest headless
  shell Playwright downloaded); `BROWSER_LIBS` lists directories of shared libraries the browser
  needs where they are not installed system-wide; `BROWSER_FLAGS` adds browser flags such as
  `--disable-gpu`. `SCHEME=light` takes the walkthrough's pictures in the light theme.
- `LOOK="size=largest density=compact"` has a check wear a look (see "Looks" in `client.md`):
  its measures are put on every page the check's browser opens, as each document begins
  (`ui/scripts/looked.ts`, read through `where.ts`). Every check is run both plainly and with
  that look before a change that moves where things stand is called done.
- Checks run as the desktop window unless they say otherwise (`ui/scripts/windowed.ts`);
  `inplace.mjs` checks the plain browser.
- `TRACE=<file>` on `tools.mjs` keeps the browser's trace of the edge drag.
- `ROWS=<n>` on `bigtable.mjs` sets how many rows it makes (ten thousand by default).
- The desktop window itself cannot be photographed here. Its top bar is drawn by the client, so
  `window.mjs` shows it; whether the window moves and resizes when asked needs a person at a
  screen.

## Shared helpers (not checks)

- `where.ts` (`saysWhere`: reads the address the daemon prints), `started.ts` (each daemon gets
  settings of its own), `built.ts`, `windowed.ts`, `looked.ts`, `ends.mjs` (`endsWithThis`: the
  daemon is stopped however the check exits, and leftover processes fail it).
- `drawer.mjs` (`drawerAtRest`, `toolsAtRest`, `sideAtRest`, `watchLeaving`), `entering.mjs`
  (`beforeEntering`), `gear.mjs` (`gearLine`, `tablesRead`), `testing.mjs` (`pressTesting`),
  `folded.mjs` (`unfoldAll`, `unfoldChannels`, `unfold`), `adding.mjs` (`openAdding`), `lit.mjs`
  (`litAs`: the browser asked for dark or light, and the page followed before anything is read).
- Box-checking helpers: `crowded.mjs` (nothing on a page overlaps or leaves the window),
  `filled.mjs` (a page of tables fills its tab evenly), `apart.mjs` (the Members page's
  Agents tab and Settings by their boxes), `heads.mjs` (table heads keep their words whole), `joined.mjs` (each
  side panel is one tree), `legended.mjs` (the band under a graph covers no tile), `offered.mjs`
  (what floats beside a composer), `besidedrawer.mjs` (a blueprint page laid out beside an open
  drawer), `fleettook.mjs` (waits for a fleet to take a table change).
- `publishing.ts`, `generations.ts`, `notices.ts` and `iconset.ts` are parts of the build, not
  checks.

## The checks

Each line: the script, then what it covers. "Runs X" means the script runs another script as
part of itself; that one is not run alone.

### First run, daemon and window

- `walkthrough.mjs`: the first-run story, about forty pictures, including an instance's History
  with what it was applied with.
- `firstrun.mjs`: a fresh daemon with no registration, `mw open` and its link, a password set on
  the Profile.
- `canary.mjs`: what is running during a rollout under Activity, and the measurements after.
- `devices.mjs`: two devices; a step the hub sent listed under Activity; the device in Settings.
- `drained.mjs`: a daemon asked to stop once work in hand is done, shown on desk and phone
  pages.
- `reconnected.mjs`: a run that ended while the page's event stream was cut, read again once the
  stream reports it fell behind.
- `update.mjs`: the client replaced under an open page across builds; a client that did not load
  whole retried.
- `window.mjs`: the top bar the client draws in the desktop app.
- `profiles.mjs`: which factory (profile) a page is in, in the tab title, sign-in, phone bar and
  Settings.
- `across.mjs`: a factory started from a copy says so across the top of every page.
- `phone.mjs`: every screen by touch at 390 wide, the most used again at 412.
- `readme.mjs`: makes the README's pictures in `docs/readme/` from its blueprint examples.

### Tabs, panes and the side panel

- `panes.mjs`: tabs split into groups, resized, joined and remembered; a crowded tab row
  scrolled sideways.
- `inplace.mjs`: the client in a plain browser: one thing in place, history entries,
  Back/Forward, new browser tabs.
- `tabbed.mjs`: each workspace's own set of open tabs, switched, restored, and forgotten on
  sign-out.
- `returned.mjs`: every kind of tab shown again exactly as left (scroll, drawer, unfolded
  parts); `TRIAL=1`, `ONLY=board`.
- `spaces.mjs`: workspaces in the side panel foot, moving projects and channels, renaming and
  removing; runs `movedto.mjs`, `spacepart.mjs`.
- `narrowed.mjs`: Activity, Measurements and Agents narrowed to the panel's workspace or one
  project.
- `lists.mjs`: the side panel's blueprints in two levels, a project's own, the add-project
  dialog, Repo lines; runs `listed.mjs`.
- `folding.mjs`: folds of the Resources and Channels panels, Collapse all and Expand all; uses
  `joined.mjs`.
- `lately.mjs`: Pinned and Recents at the top of Resources, holding still under the pointer.
- `sought.mjs`: the search box of each side panel, narrowing without asking the daemon, and
  opening search everywhere.
- `searched.mjs`: the search of everything, opened by Ctrl+Shift+F as by Ctrl+K, in the middle
  across; a topic, a reply (opening its talk at it), a row, a document (at its heading), a
  channel and a person each found and opened; arrows and Enter; narrowing by kind; phone width.
- `foot.mjs`: the side panel foot: Help's menu, What's new, the workspace menu, Settings, Log
  out.
- `followed.mjs`: the side panel arrangement kept with the account, seen from another browser.
- `following.mjs`: following a wiki with its pages from its head (This page, This page and the pages under it), a page under it changed as one line of the follower's inbox, a board followed from its head, Settings → Following set to Daily and Unfollow, at a phone's width.
- `groups.mjs`: channel groups: made, filled by menu and drag, pinned, folded, removed and
  restored.
- `sideslid.mjs`: the side panel sliding away and back by every way of folding it, laid out once
  each way.
- `outlined.mjs`: a project's things in folders of its outline, as made from a terminal.
- `arranged-tree.mjs`: the outline arranged from side panel menus: folders, Move to…, rename, the
  board shown as Roadmap (panel, head, tab, kept, cleared),
  forget.
- `outlinedrag.mjs`: outline lines carried to another place by pointer and by Alt plus arrows.
- `tablemade.mjs`: a table made from the side panel's New… menu, in a project or a folder.
- `members.mjs`: the Members line under Factory: people and agents, each line's menu, direct
  messages; the Members page its name opens, People and Agents tabs, the line still folding,
  `/agents` landing on Agents.
- `about.mjs`: the About page from the rail's mark: its three tabs and what a member is shown.

### Page heads

- `head.mjs`: the right of a page's head on every kind of page, the gear, pin, address, CSV
  export/import; runs `headgear.mjs`, `headaddress.mjs`.
- `trusted.mjs`: trusting a project made from an existing folder, from its page's gear.
- `features.mjs`: a project's features board, and features given from the board's gear.
- `blueprinted.mjs`: where a project keeps its own blueprints, chosen from its page's gear; one
  of its own and its fork opened by their name under the project (`/shop/blueprints/<name>`).

### Blueprints and runs

- `blueprint.mjs`: writing a blueprint, applying in the drawer, a run's steps and outputs, graph
  tiles; uses `legended.mjs`, `besidedrawer.mjs`.
- `summary.mjs`: a blueprint's Summary in plain words, folds, data model; runs `madeof.mjs` and
  `summarised.mjs`.
- `previewed.mjs`: Preview under Testing: each part played frame by frame against the daemon's
  graph; uses `legended.mjs`.
- `within.mjs`: what the editor explains and offers inside a step's arguments.
- `forked.mjs`: forking a shipped blueprint, the fork's Summary, going back; uses
  `summarised.mjs`.
- `reference.mjs`: the Reference page: keywords, actions, events, search, addresses, the
  editor's card link.
- `scenarios.mjs`: a Reference word shown working with Run it in a sandbox in the drawer.
- `behind.mjs`: an instance behind the current text, brought up to date; a shipped blueprint
  that changed, taken at the top with one press, which brings what runs under it along.
- `noted.mjs`: an instance's note column on Activity, warnings and failures, and the drawer.
- `compared.mjs`: comparing ways of starting a blueprint, the columns of measurements, ending a
  comparison.
- `find.mjs`: Ctrl+F in the code editor: the bar, matches, keys, replacing, Escape order.
- `scrolled.mjs`: a blueprint's text scrolled fast is numbered and coloured on every frame.
- `sandboxed.mjs`: a sandbox test from a blueprint's page: acts, stand-ins answered, runs
  compared, nothing in the factory.
- `sidebyside.mjs`: sandbox tests of several texts side by side, Apply, Compare on real work,
  Fork.
- `pipelines.mjs`: a run's own page as a pipeline run, and a project's Pipelines page with Run.
- `testruns.mjs`: a segment's command runs recorded into a table named by `record_to`.

### Tables, board and records

- `records.mjs`: a card naming a record of another table; a tick cell's width.
- `icons.mjs`: choosing a table column's icon, kept for everybody.
- `headed.mjs`: a column head's one button and menu: hold, gather, figures, freeze, hide, move,
  sort; runs `widths.mjs`.
- `widths.mjs`: dragging a column's width edge; saved once, per view, at the usual text size.
- `columned.mjs`: columns added, renamed, retyped, copied and removed by people; choice options
  edited.
- `byhand.mjs`: a table whose rows are not written by hand offers no editing.
- `bigtable.mjs`: a table of many rows read a window at a time, sorted, searched, live-updated.
- `beside.mjs`: tables that sit beside a board, opened from the gear, column marks and their
  menus.
- `arranged.mjs`: a fleet's agents and stages tables opened as tabs and edited.
- `ordered.mjs`: a board's cards by what comes first, bands by feature and label, labels'
  colours, a label picked in a card's drawer from those already on the board's cards.
- `nested.mjs`: notes by period, each table naming the one above; private days.
- `slid.mjs`: a card moved from a terminal slides to its column; when nothing slides.
- `review.mjs`: a card sent back from review; who said each thing with their picture; the drawer
  edge.
- `cardcost.mjs`: what a card cost by round, in its drawer.
- `chained.mjs`: a card waiting on people in turn, each told once by the fleet.
- `pressed.mjs`: a button a column offers on a card, holding it until pressed.
- `assigned.mjs`: whom a card is assigned to, said the same everywhere it is drawn.
- `attached.mjs`: files attached to a card or row, in their Attachments tab, and the Trash.
- `gitwork.mjs`: a package's table about cards shown in a card's Details.
- `contested.mjs`: a card's A/B Testing tab in its drawer, two roles, a judge, a reviewer.

### Documents, library and files

- `writing.mjs`: writing a document as blocks, and seeing it as a page.
- `handled.mjs`: the handle beside a document block: `+`, grip, menu, carry, keyboard.
- `library.mjs`: a project's library, its layouts, and comments in the panel beside a document.
- `selected.mjs`: comments on selected words of a document, marks, margin cards.
- `site.mjs`: a library written out as pages and opened from the disk, with contents beside each
  page.
- `mockups.mjs`: a project's mockups gallery and what a mockup's script can reach.
- `files.mjs`: browsing a project's Repo, and making files and folders.
- `pathlinks.mjs`: a project's file named in somebody's words becoming a link.
- `copied.mjs`: code blocks in messages and their Copy button.

### Channels, inbox and topics

- `channel.mjs`: a channel with a person, agent and bot; messages, threads, pastes; runs
  `attaching.mjs`, `participants.mjs`, `failedrun.mjs`, `updates.mjs`, `topic.mjs`.
- `recalled.mjs`: Up in a channel's box brings back what the reader sent, like a terminal's
  history.
- `modelled.mjs`: which model wrote an agent's messages, by the provider's name for it.
- `carried.mjs`: who is writing and which agent is working, above the box; runs `offered.mjs`,
  `threadstep.mjs`.
- `tools.mjs`: the Tools panel beside a channel, its edge dragged fast; runs `toolsslid.mjs`,
  `toolscard.mjs`.
- `docsed.mjs`: the documents a channel keeps to hand, in the Docs tab.
- `posted.mjs`: a project file posted in a channel as that file, and a copy added to a project.
- `kept.mjs`: a private channel: made, members added and leaving, made private and opened again.
- `archived.mjs`: an archived channel: listed apart, read-only, unarchived.
- `discussed.mjs`: a channel's agents talking things over in a thread (Discussions…).
- `runsin.mjs`: what runs in a channel, its settings form and Stop….
- `renamed.mjs`: a channel renamed, seen on another page.
- `details.mjs`: a channel's Details, Members and Settings, and its managers.
- `reported.mjs`: whoever says something in a channel is a member; an agent outside is refused.
- `botdone.mjs`: what the factory's bot did lately, and what it may ask.
- `inbox.mjs`: the inbox place on the rail, its lines, counts and reading.
- `told.mjs`: what a fleet tells whoever applied it, a told line's page, links in its words.
- `reviewed.mjs`: a told line's parts: cards with Pass, Fail and Discuss; runs `toldtabs.mjs`.
- `toldtabs.mjs`: a told line's page in tabs: runs, rows, measurements.
- `asked.mjs`: a change an agent asks for and may not make, approved or declined.
- `goahead.mjs`: a run waiting for a person to let it go on, approved or declined.
- `tagged.mjs`: somebody named with @ on a card, a record or in a topic's reply, told in their inbox; a name drawn as a mention, with who it is on hover and focus; a page somebody follows, changed, as one line of their inbox.
- `conversed.mjs`: a conversation to people and agents, in the inbox, opened as a channel.
- `topics.mjs`: the Topics panel on the rail, its list, search, folds, New topic; a status chosen and taken away at a page's head; a thread, a summary added and emptied, Write an article bringing the tabs.
- `wikis.mjs`: the Topics panel as a tree of wikis: pages made, renamed, moved, trashed; a status shown and cleared; a page made opening on Article, its reply counted on Talk.
- `wikidrag.mjs`: a wiki's pages carried to another place in its tree.
- `topicpage.mjs`: a topic as a page: a tracked question as a thread (its post over its replies, read until Edit, no tabs; Add post; answered, closed, a reply's address in view) and an untracked topic as an article (Article and Talk, summary, post, replies on Talk, sub-pages; tabs by keyboard and by address, a reply's address).
- `wikipage.mjs`: a wiki page and the addresses that open it; an inbox line opening Talk at a reply.
- `cited.mjs`: links between pages and sources: `[[` in the post's editor and a reply's box (on Talk, the topic being an article), the link followed, Linked from on the page linked to, a link kept through a rename and a move, footnotes as raised numbers with their source on hover and focus, References at the article's end, a link to a page that is not there drawn without its words; a link's card on hover and from the keyboard (title, wiki, summary or article read once, Escape), a reply link's card, none for a page one cannot open, the card on a phone's width and a tap that opens with no card. Run it with `SCHEME=light` too.
- `boxes.mjs`: an article's boxes: an info box with its picture, a series of three pages and a note at the top, read on a wide window (the boxes at the text's right by their boxes, the note over both in italics, the post drawing none of them again, the page it is on marked in the series) and on a phone (the note, then the boxes, above the text); an article with four headings, a note and boxes, by its boxes at 1440 (Contents left, text, boxes right, none over another), 1100 (Contents folded above, boxes beside the text), 900 and a phone's width (note, boxes, folded Contents, text), its Contents listing none of what a box says; a series' link followed to its page, which says it is linked from the first and has its series at its end; a broken info box as its code with its line; for whoever may write, after Edit, the boxes beside the editor and `/info` putting in a starter; nothing fetched elsewhere. Run it with `LOOK="size=largest density=compact"` and `SCHEME=light` too.
- `pictures.mjs`: pictures in a topic's post and replies (on Talk, the topic being an article): pasted, captioned, Side by its box on a wide window and across on a phone, an SVG, a view, a `mermaid` block drawn as an SVG inside the column (drawn again in the other light, Code showing its source, a wide one scrolling sideways in its box at a phone's width) and a broken one as its code with the error line under it, nothing fetched elsewhere. Run it with `SCHEME=light` too.
- `reading.mjs`: a long article read as a paper, at 1440 and a phone's width: a table (headings, rules, right-aligned figures, a pipe and bold in cells, a footnote marker, its link to a page followed), five diagrams (a wide flowchart, a state diagram laid sideways, a gantt of eight weeks, a pie, a mind map of seven branches) each inside its column with its words at 10.5px or more, the gantt scrolling in its box with the edge that has more marked and opened whole by Open and by pressing it, the pie's slices told apart and from the page, the mind map's box fitted to it; Contents in view after 3000px, the section marked; Linked from with the three pages of a series gathered into one folded line that unfolds; Edit and Save after a letter typed and taken back keeping the post byte for byte, the table in the editor as its text; Aa's Typeface Serif on the article's words and headings, kept after a reload and on the phone, Sans again; nothing over anything else, nothing fetched elsewhere. Run it with `LOOK` and `SCHEME=light` too.
- `fullscreen.mjs`: a view (a page that counts) as a figure in a topic's article, Open larger and Full screen under it beside its caption; opened larger in a dialog, whose Full screen Escape leaves with the dialog staying; Full screen from the article, left by Exit full screen and by Escape: the client's box in full screen (`document.fullscreenElement`), its frame and the page inside the viewport's size, the page's background behind it, Exit full screen small in the corner, the frame's sandbox `allow-scripts` and no `allow`, and the page the same one throughout (a number it drew once is unchanged, its count went on), back as tall as it was; a diagram's Full screen filling the screen, pressing it opening nothing, and back at its size; a channel's view and the tools panel's the same. Run it with `LOOK` and `SCHEME=light` too.
- `history.mjs`: a topic read until Edit, its typing kept through Talk and back and Cancel asking, Save; History's revisions, two compared with the words added marked, one restored, `#history` and `#history/<n>`; a question's thread with its post read and Edit at its head, its summary by its small Edit and its history in its place, calling its post Post; a reader with no Edit or Restore. Run it with `SCHEME=light` and a `LOOK` too.
- `contents.mjs`: a long topic's Contents beside the article (nested, numbered, sticky, the section read marked) and as a folded box on a phone; an entry by click and keyboard, `#s/<slug>` opened and reloaded, a heading's link mark copying its address; Edit beside a heading with the cursor at that section, Save reading it again there; Hide by keyboard kept after a reload; fewer than three headings as before; a reader with no Edit. Run it with `SCHEME=light` and a `LOOK` too.

### Agents, people and settings

- `providers.mjs`: choosing what an agent runs on, and its model.
- `agents.mjs`: the Members page's Agents tab: bands, permissions, history in the drawer, removal; runs
  `agentmenu.mjs`.
- `agentedit.mjs`: an agent's whole setup from Edit… in the drawer; uses `agentmenu.mjs`.
- `external.mjs`: an agent the factory does not start: invited, joined, given cards; runs
  `addmember.mjs`.
- `given.mjs`: a person given a scope beyond their role, from Settings.
- `peoplegroups.mjs`: Settings' Groups: made, members, scopes, renamed, removed.
- `keyrevoked.mjs`: keys revoked and rolled from Settings, and what they applied.
- `secrets.mjs`: the factory's secrets page: set, named, replaced, removed.
- `logs.mjs`: how long step detail is kept and where it is written, in Settings.
- `clocked.mjs`: a person's time zone and 12/24-hour clock, kept with the account.
- `appearance.mjs`: Appearance at a topic's and a document's head, by the keyboard: Wide widens the
  column by its box, Larger and Dark applied at once and kept with the account and in another
  browser; untouched, the topic is drawn pixel for pixel as without the reading column; Settings;
  no overlap or sideways scroll at three widths and a phone's.
- `themed.mjs`: themes kept with the account, light versions, looks, files; runs `picked.mjs`.
- `trashed.mjs`: the Trash: things deleted, restored and deleted for good.
- `updated.mjs`: an installed package that differs from the shipped one, and Update.
- `settingup.mjs`: the add-project dialog as a table of what it starts with, and saved setups.

### Environments and git

- `projectenvs.mjs`: a project's environments page and drawer, variables, secrets, rules.
- `flags.mjs`: a project's feature flags per environment, including one waiting for approval.
- `deployed.mjs`: an environment run from a branch in its drawer, and its runs.
- `gitpage.mjs`: a project's Git page and its tabs, including a package's parts.
