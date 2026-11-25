# The web client: general rules

This file covers the web client in `ui/` as a whole: how it is built while pages are open, how
it draws times and themes, how tabs work in a browser and in the desktop window, how a view
keeps what it showed, how it reads and follows the daemon, the drawer and the panels that slide,
forms, and the rules the client's checks lean on. Read it before changing anything in `ui/src/`
that is not a single page. The side panel and page heads are in `client-panels.md`, tables in
`client-tables.md`, and the checks themselves in `client-checks.md`.

## Building while pages are open

- People have the client open while it is rebuilt. Its build (`ui/build.mjs`) writes the files
  itself, keeps the builds before it (those replaced within a week, twenty at most:
  `ui/scripts/generations.ts`), and replaces `index.html` last. Left to itself, esbuild deletes
  its previous output on every rebuild, under the pages still using it.
- An install keeps the builds the installed copy served by the same rule (`xtask::generations`;
  a test checks the two figures agree), so a page left open survives several installs.
- A build writes and tidies holding `dist.lock` beside `dist/` (`ui/scripts/publishing.ts`), so
  two builds at once in one checkout do not delete each other's fresh files.
- A build piped into something that stops reading early (`npm run build | head -1`) is stopped
  with it, and the previous build is served. Let it finish.
- What the page opens with (`data-opens` in `ui/index.html`) is loaded once more by the page
  itself where it does not arrive, because a browser drops every request in flight when the
  system reports a network change (`net::ERR_NETWORK_CHANGED`), loopback included. Where the
  retry fails too, the page says so (`couldNotStart`, which `main.tsx` also calls where the
  daemon could not be reached as the page opened). Requests made later are not retried; one
  dropped is reported as any failure is. `update.mjs` checks it by dropping what the page opens
  with.

## Global names and editors

- Class names in the client are global. A new class that is also a layout word (`row`, `column`,
  `panel`) picks up the styles already written for that word. Pick a specific name.
- The code editor reports its text and is handed it back a moment later. Handing it back must
  never insert it again: by then more may have been typed (`ui/src/editor/echo.ts`).
- The blueprint editor's list of suggestions ignores Enter in its first moments, so that ending
  a line as it appears does not choose from it. A dialog likewise starts listening for Escape a
  moment after it is drawn. A check waits before pressing either.
- A check that drives the block editor must not press a key in the same instant as a click: the
  editor learns where a click landed a moment later, as the browser reports it.
- Reading a file the daemon follows is itself reported as an event on that file.
  `hosting::Watch` drops those; anything that woke on them would read for ever.

## Finding in a text

- Finding is the code editor's, wherever that editor is: a blueprint's text, an earlier text in
  its History, a file opened as code, a document's source (`ui/src/editor/find.tsx`). The block
  editor a document is written in has none. The searching is the library's; the bar is drawn
  here.
- **The bar moves no line.** The panel slot the library uses is taken out of the flow
  (`finder.css`), and the bar lies over the text's top right, placed from boxes (`placed` in
  `ui/src/editor/finding.ts`): clear of the line numbers, and beside a drawer that lies over the
  text. It sits under whatever lies over the page (`z-index` in `finder.css`). Only a match in
  the first lines gets room above the first line (`makeRoom`), which goes with the bar.
  Something new drawn over the editor is laid over it the same way.
- Nothing of the search is in an editor until its bar is first asked for: what marks matches is
  added by the first Ctrl+F.
- Who gets a key: in the text or the bar, the editor (its keymap; the bar hands what it does not
  answer to the `search-panel` scope, which is how Ctrl+S saves from there). A read-only text
  cannot hold the keyboard, so the page listens for Ctrl+F, F3, Ctrl+G and Escape while an
  editor's view is on it, and `answers` decides which are the editor's: none in a typing field,
  under a dialog, or in another pane; Escape only where nothing has the keyboard and no drawer
  is open. With no editor showing, the key is the browser's.
- Escape puts one thing away: in the bar, the bar; in the text, a suggestion first, then the
  bar; with neither, the drawer. Each press is stopped where it was used.
- The bar opens with a one-line selection, never with a match the search selected itself
  (`atMatch`), so reopened it holds the expression as written.
- Something new on the editor's page that takes Escape or Ctrl+F is a case of `answers`, with
  its line in `finding.test.ts`.
- What is searched for is kept by the tab (`editor/sought.ts`, by the tab's key, which a view's
  box exposes as `data-tab`) and goes when the tab closes. A bar button with nothing to do says
  so and stays (`aria-disabled`).
- A check that goes back a match presses `Control+Shift+G` (the capital letter): asked for the
  small letter, the editor reads it as the key without Shift. `ui/scripts/find.mjs` checks all
  of it.

## Things that float, and drags

- What floats over the page (a menu, a card about someone) must not push the page about, and
  must not vanish as the pointer crosses to it.
- Nor may what contains it clip it or lie over it. Anything that can be drawn inside a panel
  that clips, or under a row that overlaps, is put on the page's body and placed by the boxes of
  what it is beside (`Floating` in `views/offered.tsx`, over `offeredAt` in
  `ui/src/mentions.ts`): the names a composer offers after `@`, its tools' menus, the names
  offered on a record. `offered.mjs` (run by `carried.mjs` and `toolscard.mjs`) checks it.
- Nothing appears or moves under the pointer when a drag begins. Where something can be dropped
  is there before the drag. A channel leaves its group by being dropped on the Channels panel's
  head (outlined while one is over it), or among the channels in no group, listed above the
  groups (`outOfGroup` in `views/grouped.tsx`), or by No group in its menu.
- An edge that is dragged tells nobody until it is let go (`ui/src/held.ts`, used by
  `views/edge.tsx` and `views/sideedge.tsx`). While held, the width goes straight onto the
  layout as a CSS property set once a frame; whatever keeps the width is told at the end.
- Whatever is beside a panel must not redraw for a width that is still changing. The channel
  counts its drawings (`ui/src/drawn.ts`) and `tools.mjs` checks there are none while the edge
  is held. Something new laid out by a dragged width reads it from a CSS property, not from a
  signal set on every move. A property set at every step of a drag is not inherited
  (`--context-width` is registered with `@property` in `style.css`), or every message has its
  style recomputed.
- Each list is kept where it was scrolled while an edge moves (`keepPlace`); a new list that
  scrolls beside an edge must be one of those it looks for.

## Time

- Every time the client draws goes through `ui/src/clock.ts` (`clock` and `when` of `util.ts`
  are it, in the setting the page wears, `worn`): in the reader's zone, on a 12- or 24-hour
  clock, with the zone's short name as `Intl` gives it that day.
- The setting is kept with the account (`people::clocked`, scopes `clock:read` and
  `clock:write`, event `clock.kept` to that account alone; client `ui/src/clockkept.ts`),
  checked case for case with the client against `ui/src/clockcases.json`. The client checks a
  zone against what its browser knows before it sends it.
- The daemon keeps a zone as text and writes no time in one. It knows a zone in one place only:
  the time of day a handler fires at (`at:`, `reader::at`, still `blueprint::at`, using `jiff`),
  which states its zone (`02:00 Europe/Zagreb`) and is refused without one. Nothing else may
  look a zone up, a person's included.
- Something new that draws a time calls `clock` or `when`, never `toLocaleTimeString`, the hours
  of a `Date`, or a zone written out. A test of a drawn time sets the zone first (`worn.value`).

## Tabs: in a browser and in the desktop window

- **In a browser, the browser's tabs are the tabs** (`ui/src/inplace.ts`, with its cases): one
  thing shows at a time, in place. Only the desktop window keeps tabs of the client's own, split
  into groups, one set per workspace. Which one the page is in is what the window says as it
  loads it (`window.desktop` with `window.ipc`, `desktop.ts`); `inPlace` is that and nothing
  else.
- **The seam is `openTab`** in the store. In the window it opens a tab; in a browser it shows
  the thing in place (`showInPlace`). What was left is kept by its key among the last
  `KEPT_BEHIND` shown (`parked`: what it read, its scroll, its drawer, its sandbox, through
  `Returned` and `drawerkept.ts`); older ones are let go as a closed tab is, except half-written
  text (`keepDraft`).
- A thing opened by somebody gets its own browser history entry (`placeKeeper.opening`, which
  `address.ts` answers with `pushState`, before the thing shows). What the browser came to by
  itself (Back, Forward, an address, a reload) is opened inside `arriving`, which adds no entry,
  and is shown as it was left.
- Every thing has an address: the factory's pages by word (`/settings`, `/activity`,
  `/measurements`, `/trash`, `/secrets`, `/releases`, `/profile`), the Members page at its
  Agents tab by the Agents page's old word (`/agents`) and at People as `/#/members` (no
  project is refused that word: adding one to `kernel::paths::RESERVED` is a daemon change), a channel
  (`/channels/<id>`), a package's page (`/extensions/<name>`), and the start page `/`
  (`addressOf`, `tabAt`). Project things are covered in `client-panels.md`.
- A press in place that would leave unsaved changes asks first (`leaving`, drawn by `Leaving` in
  `workbench.tsx`: Stay, Discard them, Save and go), as closing a tab does. Back asks nothing;
  the page asks before unload (`beforeunload`).
- A side panel line that opens a thing says which (`data-names`, `names`). Ctrl or Cmd click, or
  a middle click (`opensApart`), opens it in a new browser tab at its address (`openApart`, in
  `address.ts`), as does "Open in a new tab" in a line's menu (`views/newtab.tsx`).
- In a browser there is no row of tabs and no split; `tabsof.ts` settles the workspace and keeps
  nothing. A bar over what shows (`PlaceBar`) appears only while the side panel is folded away,
  and always on a phone (`barShown`). The browser tab's title says what shows (`titleShowing`).
- Something new that opens a thing goes through `openTab`. A new kind of tab gets an address
  (`addressOf`, `tabAt`, a case each), a title (`tabTitle`), and is let go of in `letGoOf` for
  anything kept for it by its key.
- The checks run as the window unless they say otherwise (`ui/scripts/windowed.ts`, read through
  `where.ts`), since they press tabs; `inplace.mjs` checks the browser's way.

### Tab sets per workspace (desktop window)

- Which tabs are open, how they are split, which shows in each group, and which group was last
  used form one set per workspace, kept in the browser under the signed-in account by workspace
  id (`ui/src/tabsof.ts` reads and writes them; `ui/src/tabskept.ts` holds the plain functions
  with their cases). No route is told which tabs are open or which workspace is looked at.
- Whatever makes the side panel show another workspace, the set on the page is kept under the
  old workspace and the other's is shown (`switched`), or the start page alone. One effect over
  the panel's workspace does it (`held`), so new code that changes the workspace need do
  nothing.
- **A switch is not a close.** `closeTab` is not called, so nothing kept by a tab's key is let
  go, nobody is asked about unsaved changes, `onOpened` is not told, and recents do not change.
  The set goes on the page in one change (`showTabs`, through `arrange`). The drawer is put away
  and kept for its tab.
- A set belongs to the workspace it was opened in, not to what its tabs show: something of
  another workspace opened from a link, mention, search or address is a tab here and moves
  nothing.
- What a tab keeps is keyed by the tab's key, so a thing open in two sets shares it.
- **What forgets for a closed tab still forgets.** A kept set is shown as things stand
  (`standingSet`: tabs of a forgotten project or an unlisted channel are dropped with what was
  kept for them). Whoever removes a thing takes its tab out of every kept set at once
  (`forgetTab`: a removed channel through `onChannelRemoved`; a file deleted from its own page).
  A removed workspace's set is forgotten; a renamed one keeps it. Signing out forgets every set.
- **Order on load.** The workspace is settled first with no tab on the page (`beginAt` in
  `ui/src/chosen.ts`): the chosen one if it still exists, else the workspace of the tab that was
  showing in the single set kept by older clients (`client.tabs`), else the usual one. Only then
  are tabs put back (`first`); an old single set becomes that workspace's and is removed from
  where it was. The frame has not "entered" until the workspaces were read or failed
  (`spacesAsked` in `ui/src/spaces.ts`, waited for by `tabsKeeper.back`, which `enter` in the
  store waits for).
- **But the frame is drawn before it has entered.** Anything opened in that moment is opened
  among the tabs put back (`among` in `tabskept.ts`), never thrown away. Something new that sets
  the tabs while they may already have been touched passes what is there to `among`. A check
  makes this order with `beforeEntering` in `ui/scripts/entering.mjs`.
- Where the workspaces cannot be read, or nobody owns the factory yet, there is one set, which
  becomes the first workspace's later. A failed reading changes nothing.
- The store keeps no tabs itself; it says who does (`tabsKeeper`). Something new kept for a tab
  is keyed by the tab and let go in `closeTab` and in `lose` (`tabsof.ts`); something new that
  removes a thing and closes its tab calls `forgetTab`. `ui/scripts/tabbed.mjs` checks all of
  it.

## A tab shown again is as it was left

- A view is on the page only while its tab shows. Hidden views are not kept alive: the checks
  rely on only the showing view being there, and a hidden view would keep following events.
  Typed text that must outlive that is kept in the store (`keepDraft`).
- What a view showed is kept by the tab's key (`ui/src/returned.ts`; one scope per open tab, let
  go in `closeTab`, in `lose` of `tabsof.ts`, and on sign-out). The frame draws every view and
  the drawer's body inside `Returned` (`views/returned.tsx`).
- A view keeps what it read with `useKept` instead of `useState`: it is drawn at once from what
  it last had and **always read again behind**, since it heard nothing while away. Kept data is
  never taken for the current state. Values held only between drawings use `useKeptRef`. Which
  sub-view showed and what was unfolded are kept the same way.
- Scroll positions are kept by `Returned`, found again by what each part is and how many of its
  kind come before it, and put back once each part is long enough, held for a few frames while
  what is above it lays out. A part the view scrolled itself (a channel at its newest message)
  is left there; the reader scrolling or pressing ends the restore (`putBack`, with cases), as
  does a view that says `scrollsItself`. The code editor keeps its own (`data-scrolls-itself`,
  `scrolled`, `scrollTo`).
- Something new a view reads is kept with `useKept` under a name unique in the tab. Something
  that is for one thing (a run's steps, one workspace's figures) keeps which thing with it and
  is not shown for another.
- Something read in an effect that must be read as the view goes is read in a layout effect's
  cleanup; a plain effect's cleanup runs after the page is drawn without it.
- `ui/scripts/returned.mjs` checks each kind of tab, frame by frame on the way back.

## Reading once and following events

- What is read once and then followed by events can miss what was emitted between the daemon
  taking the reading and the reading arriving. Keep what arrives meanwhile (`Arriving` in
  `ui/src/arrived.ts`) and merge by id (`withArrived`). A count has no ids, so it is settled
  from the list it counts (`recount` in `ui/src/said.ts`, where channel messages are kept).
- What was read of something while it was still going is never kept as what it came to:
  `views/blueprint.tsx` keeps only what a run reported after it ended.
- An apply is answered after its steps ran, and by then the graph has drawn them from events.
  Whatever the reader chose meanwhile stays showing (`shownOnceApplied` in `ui/src/applied.ts`).
  Something new done when an answer arrives checks what was done while it was on its way.
- Read first and assign afterwards: `{ ...signal.value, [key]: await … }` takes its copy before
  the wait.

## Changes made in turn

- A change to something the daemon keeps whole is computed from the thing as it is when the
  change is made, never as the page drew it, and changes to one thing are made one at a time
  (`InTurn` in `ui/src/inturn.ts`; for a table's views, `Change` and `changed` in
  `ui/src/table.ts`, used by the board's page and `useViews`).
- So a change is a function of the thing, run when the previous change has been answered, and
  handed what that answer returned.
- A reading asked before a change was answered and arriving after it shows the old state and is
  not kept (`InTurn::read`); it is read again.
- Something new that saves a whole from switches that each change a part of it does the same.

## Running things and step output

- The id of something running (`in_…`) is drawn by one component wherever the client writes one
  (`Running` and `WithRunning` in `ui/src/views/running.tsx`, over `ui/src/running.ts`): the id
  with its blueprint and key, both pressed to open it; inside something already pressable, plain
  words (`plain`). What an id refers to never changes, so it is read once and kept; one the
  daemon says is missing is shown as gone. A tab of something running is titled the same way
  (`tabTitle`).
- Something new that shows what a step gave, or names an instance, passes the text to
  `WithRunning` or the id to `Running`, never writes the id itself. A word that only starts like
  an id (`in_place`) is not one (`parts`, with cases).
- A step's output is shown from what the daemon sent with the run (`Step.output`), as a line
  (`brief` in `ui/src/gave.ts`) and whole on a press. Nothing more is fetched to show it: output
  is stored with secrets already removed, and a second way of reading it would need the same
  care. The output column appears only where some step has output.
- A run the reader may not be told the trigger of arrives without step output and its logs are
  refused (`kept_from`); the page says so above the steps (`keptFrom` in `ui/src/gave.ts`) and
  offers no log.

## Pages, code blocks and path links

- A page shown whole (`WholePage` in `ui/src/views/file.tsx`) is in a frame that allows
  `allow-same-origin` and nothing else, so the client can measure it. It must never also allow
  scripts: the two together would make the document's content a script of the client's own.
- A block of code in somebody's words is drawn in one place (`CodeBlock` in
  `ui/src/views/codeblock.tsx`) with a Copy button at its top right. Where blocks are is one
  function of the text (`partsOf` in `ui/src/codeblocks.ts`, with cases): fenced, or indented
  four spaces or a tab after an empty line. Under a list item, code is four further in than the
  item's words; a line only as far in as those words continues the item, which `Prose` draws
  inside the item.
- `Prose` calls it for every message, thread, turn and note; anything that draws somebody's
  words another way passes them to `WithCode`. The block keeps a column at its right as wide as
  the button, so the button covers no text and nothing moves; the button's label is drawn from
  `data-word`, so copied or selected text does not include it. A document in its own frame has
  none.
- A project file named in somebody's words is a link that opens it as a tab (`pathPieces` and
  `asked` in `ui/src/pathlinked.ts`, with cases; drawn by `views/pathlink.tsx`), in code spans
  and code blocks too. It belongs to the project the words are of (`WordsOf`: a channel's; a
  record's by the project its id starts with, `OfRecord`), or to the one named before it with a
  colon (`shop:docs/x.md`).
- It is a link only where the page knows the file exists and the reader may read it: from the
  project trees the page has read, and otherwise by asking the daemon once per page (`GET
  /api/projects/{id}/file?path=…&head`, which returns the entry and nothing of its content,
  refused as reading would be: `ui/src/pathsknown.ts`), at most `MOST_ASKED` per piece of text.
  A path the page does not know stays plain text.
- Something new that draws somebody's words says whose project they are of with `WordsOf`.
  `ui/scripts/pathlinks.mjs` checks it.

## Kinds of block in somebody's words, and pictures

- What a text is drawn as is read in `ui/src/prose.ts`: a kind of block that is a line of its
  own is one function in `STANDING` there, and a mark inside a line one more alternative of
  `MARKS`. Each kind is drawn by its entry in `DRAWN` in `ui/src/ui.tsx`, which the types hold to
  every kind of `Block` there is: a new kind (a footnote, an info box) is added in those two
  places, with cases in `prose.test.ts` or a test file of its own.
- A table is the one kind that takes more than its line: `tableAt` (`ui/src/tables.ts`) reads it
  from its first line for both `prose.ts` and the block editor, so that what the page draws as a
  table the editor keeps as one. It is drawn by `views/prosetable.tsx`, ruled and in a box of its
  own that scrolls sideways (`useMoreShown`, `views/wider.tsx`, marks the edge that has more, as a
  diagram's box does). In the block editor it is a node of its own (`pipe_table` in
  `write/markdown.ts`), its text as written, edited as code is and written back unchanged; `[[`
  inside it puts a link in as text, as in a box. `cannotKeep` refuses only raw HTML.
- A block of code keeps what it is written in (`lang`, from `partsOf`). A renderer for a
  language is one entry in `RENDERERS` in `ui/src/views/codeblock.tsx`, chosen by `drawerOf`
  (only what was set up there: a block in `constructor` is its code), and handed the block as
  its code too, and the record the text is of (`Prose`'s `attached`). A language that is a
  diagram nothing draws says so under its code (`undrawnSaid`).
- An article's boxes are blocks of code in a language of their own (`infobox`, `series`,
  `note`), so that they are text that goes through the editor and every revision with nothing
  stored beside the post. `ui/src/boxes.ts` reads them (with cases): each read is a box or one
  line saying what is wrong. `views/boxes.tsx` draws them, and is their entry in `RENDERERS`:
  where they are written (a reply, a thread's post) a box is drawn in place, and one that cannot
  be read is its code with that line. On a topic's article, `boxedOf` says what is drawn apart:
  the first note over everything (`.topic-head-note`, in `Article`), the info boxes and series at
  the side (`sideBoxes`, handed by `TopicPage` as `Article`'s `boxes`), a series with `place:
  end` after the post; the post's `Prose` is told so by the `BoxesApart` context and leaves those
  blocks out (inside the `Headed` context too, so the headings it reads and their slugs are the
  article's own; a box's lines are drawn with no `Headed`, so nothing in a box is a heading of
  the article). They are drawn from what is saved, so whoever writes sees them beside the
  editor once Edit is pressed, which shows their code. Where the boxes and the Contents stand
  is decided by container queries on `.topic-article` (`boxes.css`, `contents.css`), not the
  window's width, since the side panel takes some of it: with both, the Contents are a column
  at the left only from 1100px (800px with no boxes), folded above from there down, and the
  boxes are at the right down to 700px and above the text (under the note, over the folded
  Contents) below it. The note is a row of the article's whole width, so nothing stands
  beside it. The daemon counts their links (`kernel::topics::BOXES`), case for case with
  `linkcases.json`. The `/` menu offers them where the editor links pages (`linking`), and `[[`
  inside one puts in the link as its text (`write/linked.ts`). `ui/scripts/boxes.mjs` checks it.
- A `mermaid` block is drawn by `views/diagram.tsx`. The library is a part of its own, imported
  with `import()` the first time a page draws a diagram (some 650 kB, and a chunk per kind of
  diagram), never at the start. It is set up before each diagram, one at a time, with
  `diagramSettings` (`ui/src/diagrams.ts`, with cases): `strict`, nothing drawn by itself, no
  bomb for an error, the page's letters, laid out by dagre (the library's own first choice is
  ELK), `default` or `dark` as `data-light` says, a gantt `GANTT_WIDTH` wide wherever it is drawn
  (the library would draw it as wide as the window), and `themeVariables` from the page's tokens
  as the browser computes them (`diagramColours`: boxes, lines and words the page's; a pie's
  slices and a mind map's branches the eight hues of `CATEGORICAL`, one set stepped for light and
  for dark, then the page's greys; words on them `ON_COLOUR`). It draws again on `themechange`.
  Its SVG goes in as the library hands it (cleaned by its DOMPurify); its `viewBox` is then
  fitted to what it draws (`trimmedBox`, measured once laid out, a gantt's today line left out)
  and its width decided by `diagramWidth` from that width, the column's and the middle size of
  its words (`typicalText`): fitted while they stay at `READABLE` (11px) or more, else drawn
  where they are that and scrolled in its box, drawn again as the column changes width (a
  `ResizeObserver`, no timer). The box is `role="img"`, named by `diagramName` (its `accTitle`,
  front-matter `title:` or a gantt's or pie's `title` line); pressed, or Open, shows a copy as
  large as it was drawn in a `Modal` on the page's body (the article is a container, which would
  hold anything fixed inside it); Code shows the `CodeBox`. One that fails is its code with
  `notDrawnSaid` under it. ELK (EPL/GPL, not accepted by `about.toml`) is replaced at build
  time by a module that says so (`withoutElk` in `ui/build.mjs`).
- A picture is `![caption](attachment:<id> "side")` (`ui/src/pictures.ts`, with cases): the id
  of an attachment of the record the text is of, which `Prose` is given as `attached` (a topic's
  post, summary, answer and replies). `views/pictured.tsx` draws it by what that record's
  attachments say it is (read once per record for the page, followed by `record.attached` and
  `record.detached`): a picture or an SVG as an `<img>`, a page as a `ViewFrame`, another file
  as a link to save. An address of anything else is a link and is never fetched; one that cannot
  be read is a line saying so.
- The block editor draws a picture with the same component (`write/pictured.ts`, a node view of
  the Markdown image node), and offers Side, Across and the caption in a bar over a picture
  chosen; it takes pictures pasted, dropped or chosen from the `/` menu only where it is given
  `picturing` (`views/picturein.tsx`: the record they are attached to). A reply's box takes them
  the same way (`usePicturesIn`). `ui/scripts/pictures.mjs` checks it.

## Themes

- A theme somebody makes is eight colours; its full list of colours is derived from them
  (`listOf` in `ui/src/themes.ts`), and a test checks that list against `themes.css`. A new
  theme colour is added to both and to `people::themes::COLOURS`.
- Whether a theme is readable is one rule in the daemon and the client (`unreadable`), checked
  sentence for sentence against `ui/src/themecases.json`, which also lists the shipped themes
  (`people::themes::SHIPPED`).
- A theme may have a light version (`light`, eight more colours), worn where the system asks for
  light. Each version must be readable on its own (`unreadableEither`,
  `people::themes::unreadable_with`). The page wears `wornOf(theme, prefersLight)`; `index.html`
  applies the light list before anything is drawn (`client.theme.light`, beside
  `client.theme.colours`). In the form the light colours start derived from the dark ones and
  follow them until one is changed (`lightFrom`, with cases). Something new drawn in a made
  theme's colours uses `wornOf`, never `colours` alone.
- Colours are chosen in the client's own picker, never the browser's (`views/colourpick.tsx`;
  logic in `ui/src/picking.ts`, with cases). It is opened from a swatch (a button exposing
  `data-colour`; checks use `colourOf` and `pickColour` in `ui/scripts/picked.mjs`), floats
  without moving anything, and scrolls inside itself with Done kept at its foot. It shows Was
  and Now side by side; an area and a hue slider by pointer and arrows (`stepped`, keeping the
  hue through grey: `hsvOf`); a typed `#rrggbb` (`typedColour`); the theme's other colours and
  the shipped ones (`picks`); and the readability sentence as the colour changes. Done or Enter
  keeps it; Escape or a press outside restores it. Escape is the picker's alone.
- Themes and the chosen theme are kept with the account (`people::themes`: tables `themes` and
  `themes_chosen`, the factory's own, kept in a copy, moved with no project). Every value is
  checked where written (`checked`: name non-empty, one line, at most 60 letters; colours as
  `#rrggbb`; readable text; each look setting one of its words; at most 40 themes per account).
- **Whose themes is never taken from a request**: they are the caller's (a key's are its
  maker's). Reading takes `themes:read`, changing `themes:write`; people hold both, agents and
  older keys do not. Somebody else's theme answers as missing, even to the factory's owner.
- Whoever may manage the factory (`admin` with `themes:write`) offers one of their own themes to
  everybody and withdraws offered ones. Each change is one write that emits `theme.*` (on
  `journal::HERE`) carrying the whole theme. Events about a personal theme take
  `member:own:<actor>` (`kernel::principal::own`).
- Client: `ui/src/themes.ts` (`heardTheme`, `takenUp`, `chosenOnce`, with cases);
  `ui/src/themed.ts` reads and follows. Themes an older client kept in the browser
  (`client.themes`) are uploaded once, the first time the account's themes are read, and then
  removed from the browser.
- Settings' Theme panel (`views/themes.tsx`) draws the shipped themes, offered ones with who
  offered them, and the reader's own, and only what the reader may do. Making one and reading
  one from a file are a line of their own (`.theme-actions`), never cards among the themes.
- Any theme can be saved as a file by the browser alone (`<name>.theme.json`: `version`, `name`,
  `colours`, and `light` and `look` where present; a `look` makes it version 2) and read back as
  the reader's own through the route that makes one (`ui/src/themefile.ts`, with cases). An
  unknown version is refused, saying which.
- Something new of a person's that should follow them to any browser is kept the same way, with
  what they hold checked and nothing taken from the request.

## Looks: letters, size, density, corners

- A theme may also set a look (`look`): `letters` (`serif`, `humanist`, `mono`), `size`
  (`smaller`, `larger`, `largest`), `density` (`compact`, `roomy`), `corners` (`square`,
  `round`), each optional, `usual` meaning nothing set. One list in both
  (`people::themes::LOOKS`, `LOOKS` in `ui/src/looks.ts`), one sentence for what is not a look
  (`unfit`).
- Each setting is one CSS measure (`lookOf`): letters are `--sans`, a stack of system fonts in
  `looks.ts` (`STACKS`); **nothing is fetched for them**. **Every font size in the style sheets
  is so many pixels times `--size`**, every corner so many times `--round`, row and control
  heights so many times `--tall` (`--size` and `--density` combined, in `themes.css`), and the
  spacing in most of a page goes by `--density`. `looks.test.ts` checks no font size or corner
  is written outside the look, except a graph's tiles.
- Write a new font size as `calc(<n>px * var(--size))`, a corner with `--round`, a row or
  control height with `--tall`.
- Under a finger and in the phone layout, nothing is shorter than usual whatever the look
  (`--tall` in `phone.css`). Something sized so its content fits on one line (the Tools panel's
  default width) grows with text size.
- Every font stack ends with the systems' emoji fonts (`EMOJI` in `ui/src/looks.ts`, `--sans`
  and `--mono` in `themes.css`, checked by `looks.test.ts`); a stack an older client stored is
  worn with them (`withEmoji`, `wearLook`).
- The page wears a look as it wears a made theme's colours (`wear`, `wearLook` in
  `ui/src/wearing.ts`), applies it before drawing from what the browser kept
  (`client.theme.look`, read by `index.html`), and in the form wears each setting as it is
  chosen, going back however the form is left.
- A check is told a look by one variable (`LOOK="size=largest density=compact"`, see
  `client-checks.md`). Where a layout breaks at a look, fix the layout, not the look.

## Appearance: what is read, and light or dark

- Beside a theme and its look, each person keeps how what they read looks (`people::appearance`,
  scopes `themes:read` and `themes:write`, event `appearance.kept` to them alone; client
  `ui/src/appearance.ts` with its cases, `appearancekept.ts`, `appearancecases.json` held by both):
  `text` (a look's sizes), `width` (`standard`, `wide`), `colour` (`automatic`, `light`, `dark`),
  `contents` (`shown`, `hidden`: a long topic's Contents beside it, or hidden until asked; a setting
  kept before it was a word reads as `shown`) and `typeface` (`sans`, `serif`: the letters of an
  article and a library's document; a setting kept before it was a word reads as `sans`).
- A page that is read wraps its reading column in the class `reading-column` (`READING`) and mounts
  `<Appearance />` (`views/appearance.tsx`) in its head, one line each: the topic page and a
  library's document do. Wide sets `--column` there to `--wide-column`; a text size sets `--size`
  (and `--tall`) there alone, from `--read-size` (`appearance.css`); Serif sets the letters of an
  article read (`.topic-page[data-shows="article"]`: its text, headings and title) and of a
  document's editor to `--read-face`, `STACKS.serif` (`readFace`, put on the page by `wear` and by
  `index.html` as the size is), what is pressed keeping the page's own. Standard, the usual size
  and Sans change nothing, so a page nobody touched reads as it did. Settings has the same five.
- Light is one mark on the page, `data-light`, put there by `wearing.ts` from the system and the
  choice (`lightWorn`), and before anything is drawn by `index.html` from what the browser kept
  (`client.appearance`). `themes.css` and anything else with a light colour go by `:root[data-light]`,
  never by `prefers-color-scheme`; the one exception is `page.css`, which a published page uses with
  no client. A view in its box is told the same (`viewframe.tsx`).

## Tags

- A word drawn as a tag (an option of a choice, an item of a list such as a card's label) is one
  component, `Tag` in `ui/src/views/tag.tsx`, in the colour its name maps to (`colour` in
  `ui/src/pills.ts`): one of eight, the same everywhere, stored nowhere. The eight are theme
  colours and two mixed from them (`.tag-1` to `.tag-8` in `style.css`), so made themes have
  them.
- `pills.test.ts` checks every theme, dark and light, for readable words on each tint and eight
  distinguishable colours. Something new that draws such a word draws a `Tag`, never its own
  badge. Colours chosen per label are not built; if built they belong in the field's
  declaration, not the browser.

## The drawer

- The drawer is one element, drawn by the frame (`ui/src/app.tsx`), which also owns its width
  (`ui/src/drawer.ts`, `views/draweredge.tsx`): the width is a CSS property of the workbench,
  never wider than it. Whatever opens a drawer says what is in it and nothing about width.
- It slides in from the right (`ui/src/drawer.css`): `transform` only, about a quarter of a
  second, by the style sheet's animation, no timer, no relayout of the page under it, not kept
  after it ends, not done where reduced motion is asked for. It stays the same element while
  open, so new contents do not slide in again: nothing may key it by its contents.
- Put away, it leaves the page at once. What slides out is a still copy
  (`ui/src/drawerleaves.ts`; `stillOf` and `goesOut` in `ui/src/still.ts`): same boxes and
  words, each field showing what it held (`holdsAsItStands`; never a password), inert, removed
  when the animation ends. It is not called `.drawer` (it is `.drawer-leaving`), and its content
  is in a closed shadow root, so no selector on the page finds anything inside it; only its box,
  with `data-of`. Something new that should be seen to go does the same: a still copy, never the
  live thing kept back.
- The workbench clips what is past its edge (`overflow: clip`) so nothing scrolls sideways while
  the drawer moves.
- The drawer reports when it is at rest (`data-rest`, set by `drawn` in `drawer.ts`). **A check
  finds the drawer by that** (`drawerAtRest`, `DRAWER_AT_REST` in `ui/scripts/drawer.mjs`); a
  check that asserts no drawer looks for any `.drawer`; never a pause instead.
- **The drawer belongs to the tab it was opened beside** (`ui/src/drawerkept.ts`, through
  `drawerKeeper`). Showing another tab puts it away and keeps it; showing the tab again brings
  it back as it was (contents, its tab's drawer scope `drawerScope`, scroll), with no slide
  (`back`, `.drawer-back`). Pressing the showing tab again, putting it away directly, or Open as
  a tab, keeps nothing (`keepsDrawer`).
- A drawer of something that can be removed says how to check (`gone` of `Drawer`, `goneBy`);
  brought back, it is checked, and discarded if gone.
- The slide's time and easing is one style-sheet measure (`--slide` in `drawer.css`).
- Something that asks for many things beside a text is asked in the drawer (`Compare` in
  `ui/src/views/compared.tsx`, `Applying` in `ui/src/views/applying.tsx`). Each field is a
  label, a box, and its help, with an explicit width (`.comparing-field`; a `<select>` otherwise
  grows to its longest option); rules in `ui/src/comparing.css`; buttons at the foot
  (`.comparing-foot`). Typed values are kept outside the view (`ui/src/waysasked.ts`,
  `ui/src/applyasked.ts`). Pressing the button acts on the text as it is then (`latest` in
  `ui/src/views/blueprint.tsx`).
- A dialog is never taller than the window; one with many fields scrolls, with its buttons kept
  at its foot (`.modal-foot`).

## The Tools panel and the side panel slide

- The Tools panel beside a channel comes and goes like the drawer (`ui/src/views/toolsslide.ts`,
  `drawer.css`), but takes room beside the channel. **The channel is laid out once on open and
  once on close, never while the panel moves**: opening takes the room at once and the panel
  slides in by `transform`; closing gives the room back at once and a still copy (`stillOf` with
  `whole`, `context-leaving`) slides out over it. While moving, the channel clips past its right
  edge (`overflow: clip` by `:has`).
- It slides in only when it comes to be beside a channel already on the page (`useSlid`); a
  channel drawn with its panel (tab shown again, reload) has it simply there. It reports rest
  with `data-rest`; a check that opens it, a thread, or a person there waits for `toolsAtRest` /
  `TOOLS_AT_REST` in `ui/scripts/drawer.mjs`. `toolsslid.mjs` (run by `tools.mjs`) checks it.
- The side panel slides the same way (`ui/src/views/sideslide.ts`), whatever folds or unfolds it
  (Ctrl+`\`, the head button and its stand-in, a rail place pressed again, a press beside it on
  a phone, a screen becoming narrow). All of those change `side`, and the frame calls
  `useSideSlid`, so new ways need do nothing. The workbench is laid out once each way: brought
  back, the panel slides in from behind the rail (`side-in`, `transform` plus `clip-path`);
  folded, a still copy (`side-leaving`, `stillOf` with `placed`) slides out.
- It slides only when it was folded and is brought back; choosing another rail place while it is
  out just swaps the panel. On a phone the rail and panel come in together (`side-in-phone`,
  `--side-span`; the copy is of both via `more` of `stillOf`) and the page beside them is dimmed
  (`scrim-in` on `.side-under`, `side-under-leaving`).
- It reports rest with `data-rest` on `.sidebar`; a check that folds or unfolds it waits for
  `sideSettled` / `sideAtRest` / `SIDE_AT_REST`. `sideslid.mjs` checks it.
- Nothing slides where reduced motion is asked for.

## Long lists and repeated messages

- **What is out of sight is not laid out** (`ui/src/inview.ts`): each message of a channel or
  thread except the newest fifty (`NEAR` in `views/channel.tsx`; the rest are `said-far`), and
  each line of the Tools panel's card list, uses `content-visibility: auto`, standing out of
  sight as tall as it was last laid out, kept by identity (`standsBy`, `useKeptSizes`), so a
  redrawn list has the same height and pixel positions.
- A list kept at its foot (a channel, a thread) is put back there whenever what comes into sight
  lays out at another height (`contentvisibilityautostatechange`, a `ResizeObserver` on visible
  items); a scroll the list made itself is never taken for the reader's (`putAt` in
  `views/channel.tsx`). The newest are always laid out because otherwise a burst of new messages
  drifts the list away from its foot.
- What lays out only in sight clips at its edges, so a message that is hovered, focused, or has
  something open over it is laid out normally (the rule in `style.css`). Something new drawn
  over a message from inside it is a case of that rule.
- Out-of-sight text is not rendered: a check reads an unscrolled message by `textContent`, never
  `innerText`.
- The Board tab of the Tools panel lists as many of a column's cards as fit and counts the rest
  (`firstOf` in `ui/src/context.ts`); "and N more" opens the board at that column.
- Messages are block elements spaced by margins (`.messages`), not flex boxes, for layout cost.
  `tools.mjs` drags the Tools edge over two thousand messages and three hundred cards and checks
  frame times; `TRACE=<file>` keeps the browser trace.
- **What a bot says again and again is folded** (`ui/src/folded.ts`, with cases): consecutive
  messages from the same step of the same running blueprint (`said_by`) are drawn as the latest
  with "N earlier updates" under it, kept unfolded by the tab (`channel.unfolded`). A message
  that was answered, or names somebody, is never folded away and ends the run; one a link asks
  for is never hidden. `updates.mjs` (run by `channel.mjs`) checks it.

## Cards that move

- **A card moving to another column slides there** (`ui/src/views/cardslide.ts`, over `moved` in
  `ui/src/slid.ts`), on the board and on any table laid out in columns (`ColumnsLayout`). What
  is drawn reduces to one word per arrangement (`arrangementOf`); boxes are noted before and
  read after, and each moved card slides by `transform` (`card-slid` in `drawer.css`). A card
  whose start is outside its column's clipped list flies over the page as a still copy
  (`card-flying`).
- Nothing slides under the pointer, during a carry, for the card let go by hand (`byHand`), for
  one out of sight, or with reduced motion. A card opts in with `data-slides`; new code that
  draws cards in columns sets it and calls `useCardSlide`. `slid.mjs` checks it.

## The card form and record forms

- A card opens into one form wherever it is opened (`CardForm` in `ui/src/views/cardform.tsx`):
  the drawer beside its board (`openCard` in `views/board.tsx`) and the Board tab of the Tools
  panel, which reads what the form needs of the board itself (`useAbout`). What happens after
  save, delete or send back is the opener's (`onDone`).
- **Each part is drawn as far as the reader may see and change it**: changing a card takes
  `board:write`, and a field that requires more asks that too. A part they may not change is
  shown inside a disabled `fieldset` (`.card-fields`), and Save, Delete, Say and Send back are
  not offered. The assignee is shown with `assignedTo`, never an account id.
- What is typed in the panel's form is kept by the tab in its own part of the scope (`Part`,
  `forgetPart` in `views/returned.tsx`, `partScope`) and let go when the form is done or Back is
  pressed. Something new a card's form shows goes in `CardForm`, and so in both places.
  `toolscard.mjs` (run by `tools.mjs`) checks the panel's form against the drawer's.
- A record form has no place for a field its table marks as kept for whatever looks after the
  record: one both `hidden` and `quiet` (`keptForItself` in `ui/src/table.ts`). It is told by
  what the table says, not by the field's name; the names listed there are for boards declared
  before tables could say so.

## The program's About page

- The program's own page opens from its mark at the foot of the rail, as a tab (`{ kind:
  'about', at? }`, `views/about.tsx` over `ui/src/about.ts`, with cases, and
  `ui/src/aboutshown.ts`). It has no keyboard shortcut. The mark is two filled strokes in the
  rail's colour (`signs.mark`); the program's name is what `/api/meta` says, never written in
  the client.
- It has tabs in its head, each known by the word after `#` (`/about#technical`,
  `/about#docs/<guide>`), the showing one kept by the tab: What is <name> (`docs/about.md`, a
  guide marked `<!-- public -->`, built in via `manual::guides::GUIDES`); Docs (the other public
  guides, read in place, links between guides followed); Technical (version, client build,
  system, data folder only to `admin`, installed packages and whether each is the shipped one to
  whoever reads agents, and the report text, shared with Help's menu via `reportText` /
  `copyWhatToSay` in `views/help.tsx`).
- All of it comes from the daemon as shipped (`api::routes::about`: `GET /api/about`, `GET
  /api/about/guides/<name>`), drawn by `WholePage` with nothing run and nothing fetched. New
  text for everybody goes in `docs/about.md` or a public guide; something technical is a line of
  `facts` with its case, shown only to whoever may read it. `ui/scripts/about.mjs` checks it.

## How checks of the client behave

- A check goes by what the page shows and nothing a busy machine does not keep. It waits for the
  thing it is about to read, not for its container. What it counts is counted so that a pause
  between two steps counts the same.
- Where what is checked is the order of two things, the check makes the order (for example
  `walkthrough.mjs` holds an apply's or a save's answer with `page.route` until the next thing
  is pressed). Never a longer wait. A check that fails now and then is run several at once, each
  with its own daemon, until it fails reliably.
- A check is of the client built from the files beside it, or it fails at once saying so: the
  build records what it was built from (`dist/built-from.json`), and `saysWhere` compares
  (`ui/scripts/built.ts`).
