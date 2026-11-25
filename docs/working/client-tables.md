# The web client: tables, views and comments

This file covers how the client draws a project's tables: the table page and drawer, tables that
sit beside another, tables not written by hand, column groups, scope-list cells, order and band
views, windowed reading of large tables and live updates, column icons, column heads and their
menus, column widths, columns people add, comments on records and documents, comment counts, and
how what somebody may ask of the factory is shown. Read it before changing anything under
`ui/src/views/records.tsx`, `laid.tsx`, `windowed.tsx`, `headmenu.tsx`, `shaping.tsx` or the
document comment panel. General client rules are in `client.md`; the side panel and page head in
`client-panels.md`.

## A table in the client

- A table a blueprint declares is shown only as a table (`ui/src/views/records.tsx`): rows,
  cells, rows added and removed. It knows nothing of what its rows are about and must not come
  to.
- It is drawn the same in the drawer (`aside`, `openAside`). A table that sits beside another is
  opened there from that one's page, and one row of any table is opened into its form from
  wherever it is named (`openRow`). The drawer has no gear, so the table's head there has
  buttons for what a tab's page has as gear lines (Its tables, Layout).
- In the drawer, Escape is first for what is open inside the table (a cell being edited, what a
  head button opened, a column's menu, the icon panel) wherever the caret is, and closes the
  drawer only when nothing is open. The table takes that Escape before the frame (`opens`, at
  the window on key down; `firstHere` when the caret is in the table). Whatever opens in a table
  closes on an outside press and is a word of `OPEN`. It starts listening a moment after it is
  drawn; a check waits before Escape.
- The views of a table that is not a board are one thing (`ui/src/views/laid.tsx`): a page
  supplies its own look (a library's list, mockups' gallery) and is given the view tabs, saving
  and other layouts. A new page of a table uses it, never a second switcher.

## Tables that sit beside another

- A table may say it sits beside another table of its project, and which of its fields says what
  a row is about (`beside` and `about` of `collection.ensure`, `studio::shown`, kept in the
  table's row). It is how the table is shown and nothing else: it lets nobody in, keeps nobody
  out, and nothing in the daemon acts on it (no route, check or event asks what a table sits
  beside). Setting it takes what declaring the table takes (studio and API suites' `shown.rs`).
  `std/fleet` sets it on its three tables (`beside: board`; `stages` also `about: column`).
- Client logic: `ui/src/sits.ts` (which table one sits beside, trusted only where that table
  exists and following it does not loop; what a row is about: its field, else the row's name).
  Drawing: `views/sits.tsx`.
- Such a table is not listed in the side panel (`tablesOf` in the store). The page it sits
  beside lists it under Its tables with its row count, as gear lines (`TableActs`, `besideLines`
  in `ui/src/gear.ts`) or, in the drawer, a `Beside` button (`ViewsHead`), and opens it in the
  drawer, from where it can become a tab.
- Where that page lays records out in columns, a column's head shows its name and count, with
  one small mark where any row is about the column (`ColumnSetUp` in `views/columnset.tsx`, over
  `setUpOf` in `ui/src/columnset.ts`, on the board and in `ColumnsLayout` of `views/laid.tsx`),
  and a dot on it where a row holds the column's cards until pressed (read as the board's
  buttons are, `cardactions.ts`).
- Pressed (or Enter, Space), the mark opens a `RowMenu`: a heading per table (`tableCalled`),
  each row about the column with a few faint words under it (`under`: `oneOf`, plus its first
  two non-empty cells), and for tables whose rows describe what the factory's bot does with a
  column, that rule as a sentence (`stageSays`, `actionSays`; tables recognised by the fields
  they declare, `staged` and `pressed`, never by name; empty cells described as `fill` of the
  stages in `std/arrangement` fills them, and nothing claimed that the row does not say). Each
  opens its form. At the menu's foot is a line to add one row of each listed table about the
  column, for whoever may write records and holds what the table takes (`ColumnAsked`, drawn
  once in the frame).
- The client knows no stage, agent or segment for this and reads no table by name: something new
  that goes with a table declares `beside`. One exception: a fleet declared by an older version
  declares neither word, so Send back finds the rows that say where a card goes by what sits
  beside the board and is about its columns, falling back to the table called `stages` only
  where nothing does (`sendingBack` in `ui/src/sentback.ts`).
- Rows are read once for every place that draws them (`rowsOf` in `views/within.tsx`) and again
  when one is added, changed or removed.
- What sits beside something existing leaves the side panel, so a board, a library and mockups
  are refused `beside` (`Shown::fits`). `beside.mjs` checks all of it.

## Tables whose rows are not written by hand

- `by_hand: false` of `collection.ensure` (kept like `beside`; `studio::shown`; `true` is stored
  as nothing said) marks a table whose rows are written by commands and blueprints (a row per
  script run, simulation scenario, deploy). It is presentation only: the daemon refuses no
  write, and every door that writes a record works as before. Board, library and mockups are
  refused it.
- In the client it is one function (`offered` in `ui/src/byhand.ts`, with cases): no new row
  (head button, `+ New`, inline add); no cell or form field to change (cells are text, ticks are
  disabled, fields closed with a reason, no Save, no carrying a row to another column or day);
  no column changes (`useColumnsChanged` in `views/shaping.tsx`: no `+` after the last head, no
  rename, no Change type, Options, Insert, Duplicate or Delete); no CSV import. Everything about
  how somebody looks at it stays, and a row can be opened and moved to the Trash. Its head says
  so (`keptSaid`).
- Something new the client offers that writes a row or the table's shape asks `offered`.
  `byhand.mjs` checks it.

## Column groups

- `groups` of `collection.ensure` is a list of headings, each with its fields in order (kept
  like `beside`). It lives on the table, not on each field, because a blueprint's fields are a
  map that reaches the daemon in name order, so only a list can give the order. A field in two
  groups, or one the table lacks, is refused.
- Client: one function (`banded` in `ui/src/groups.ts`, with cases): the field a record is known
  by first, ungrouped fields as before, then each group in order; a group with nothing visible
  is no band. `TableLayout` draws a band of headings over the heads (two sticky rows, a divider
  where a group begins), and a row's form has a section per heading (`Whole` in
  `views/records.tsx`). A table without groups is drawn as before.
- `std/fleet` declares five groups for `agents`; `stages` declares none.

## Cells that are lists of scopes

- A cell holding a list of scopes is shown and edited by area. The client recognises it by what
  the field declares (`takes: each`, arriving as `takes_each`, with `unsaid`): `isScopes` in
  `ui/src/scopes.ts`, never by a field being called `holds`.
- In a table the cell is short (`briefly`: `the usual` where the row says nothing and the field
  says what that means, else the areas and a count). Pressed, it opens one panel over the cell
  with a line per area, the same table the Tools panel draws (`HoldsTable`), changed a line at a
  time, each change writing the whole list (`afterLine`); the panel stays until closed
  (`views/scopes.tsx`), is moved to be seen whole within the table's scroller, and scrolls
  inside itself if taller. A row's form has the same under the field's name.
- Nobody is offered what they do not hold (the option is shown disabled with a reason), and a
  row that already gives something the reader lacks says it is not theirs to change, since the
  daemon checks each scope a row gives against whoever changes any cell of it (`lacking`).
- An empty list means the row says nothing and so is given the usual set, not nothing: the last
  scope cannot be removed there, and the panel says so; returning to the usual is its own
  button. The Tools panel writes the same cell, and both follow the record's event. `beside.mjs`
  checks it.

## Which card an agent's turn is about

- Which card an agent's turn concerns is read from the conversation the turn belongs to, by one
  function (`heldIn` in `ui/src/heldin.ts`, with cases), using the conversation names the daemon
  uses (`spent::CONVERSATIONS`). So what an agent is said to be doing (`ui/src/doing.ts`),
  whether it is writing in a channel (`ui/src/typing.ts`), what a card cost and an agent's
  history agree.
- An agent the factory does not start has no turns: what it has in hand is the cards assigned to
  it that are in progress on its project's board (`inHand` in `ui/src/external.ts`). Something
  new an agent can have in hand is a case of `heldIn` with a conversation named for it.

## Order and band views

- Records by what comes first, and records in bands by what they name, are two things a view
  declares, for any table: `kind: order` with `by` (the field in which a record lists which
  others come before it) and `done` (the choice whose last value means done), and `group` for a
  board, an order or a table (`studio::View`, checked in `View::check`; `std/board` declares
  `order` beside `board`).
- Which column a record is in is one function of the rows (`ordered` in `ui/src/ordered.ts`,
  with cases for a chain, a diamond, a ring, a missing dependency, and something done in the
  middle); which band is another (`bands`). `ui/src/views/ordered.tsx` draws both, for the board
  and any table (`laid.tsx`); neither knows a card.
- A record is done at the last option of its choice and begun anywhere past the first. The head
  of "can be begun" counts those at the first option and says where the rest are.
- A card's `part_of` is text (a reference field would be refused because the board is made
  before any feature table). A band is named by the record of that id where the reader may read
  one (`useCalled`), else by the text.
- What is folded (a band, the done column) is the reader's, kept in the browser. Where no record
  says anything in the grouping field, no band is drawn.
- Where everything not done can be begun there is one step, and its cards flow in rows across
  the available width (`ordered-wide`); with several steps, each is a column. A card is never
  wider than its column: the column is a one-track grid told the column's width (`.cards`), and
  a card's marks wrap. `ordered.mjs` checks it by boxes, with labels on the cards.
- A view may be grouped by a field holding several values (a card's labels): the record appears
  in each value's band, drawn once per band but one record, so moving it in one band moves it in
  all.
- A view of any kind may be held to some records (`only`: for each field, the values a record
  must hold one of). The page shows those (`shown` in `ui/src/ordered.ts`), says so in the
  view's head with a count and one press to show all (`views/held.tsx`), and counts what is
  shown. The order is still worked out over all records, so one waiting on a hidden record names
  it. It is saved with the view, so a board held to a label is a tab of its own. Only fields
  where two records share a value are offered.
- Something new a view can do is a key of `View` and a line of `goes` in `ui/src/table.ts` (what
  carries over when the layout changes). The board's page is at its line limit; what it draws
  comes from separate files.

## Reading a table a window at a time

- A table laid out as a table, on its page and in the drawer, is read a window at a time
  (`ui/src/window.ts`, plain functions with cases; `rowsread.ts` reads; `rowsapi.ts` the two
  routes; `views/windowed.tsx` draws it and holds `TableLayout`).
- The daemon sorts by the reader's sort (`useSort`, sent with each reading and stored nowhere
  else), applies the view's hold, groups and counts (`rows/counted`: head and foot counts, band
  counts, figures). The page draws the rows on screen plus `OVER` either side, with spacers for
  the rest, paged past `CAP` pixels (`paged`).
- What is read is kept by the tab, drawn at once when the tab returns and read again behind. The
  row at the top of the screen stays put as rows above it are measured, read or recounted
  (`shifted`).
- **What changes underneath is drawn where it is** (`ui/src/rowslive.ts`, with cases): ids heard
  in a moment are fetched at once (`ids=`); a row on screen is redrawn as it now is, marked
  where it would now sort elsewhere, struck where the view no longer shows it; added rows are
  counted where the view is sorted and appended where it is not (`appended`, numbered after
  every row so nothing on screen moves). A line over the rows says so, with Show, which reads
  again around the top row (`around`) and keeps it in place (`placed`).
- Nothing moves the rows but that press, the reader's own changes included (`mine`; rows they
  removed from the view are removed, `dropped`; a CSV import is read again). A window answered
  after a row it holds was heard of fetches that row again (`lately`, `Arriving` by arrival
  order).
- Search and Ctrl+G go-to-row are in the table's head (`views/rowsfind.tsx`). A table declared
  again is re-read whole shortly after.
- **Whatever still needs every row reads them only while the table has at most `WHOLE` rows**
  (`rowsread.ts`): other layouts, the column menu's hold options, a row naming another row of
  its own table. Past it nothing reads the whole table, other layouts say so, and CSV export and
  import read every record only when pressed (`every` of `TableActs`).
- The board's table has all its cards in hand and draws them all.
- A check that reads rows after a sort or hold waits for them (`sortedAs` in `headed.mjs`).
  `bigtable.mjs` checks all of it.

## Column icons

- A column's icon is the field's, for everybody, and is a name: declared (`icon` of a field,
  `studio::icons`) or chosen by somebody looking at the table (`icon_chosen`, via `PUT`/`DELETE
  /api/collections/<project>/<table>/fields/<field>/icon`, which needs what saving a view needs
  and answers a hidden table as missing). Never per person or per view; nothing of it is in the
  browser.
- A chosen icon stands over the declared one and lives in the table's row with its fields (so
  copies and moves take it), carried by field name when the table is declared again
  (`added::carry`, inside `Collections::keep`'s write from the row as it then is). A removed
  field takes it away. The row's parts are written separately: an icon change writes the fields,
  a view save the views, each in its own transaction (API suite `icons.rs`). It emits
  `collection.icon`, followed by every open page (`ui/src/store.ts`).
- The daemon knows no icon set: a name is a short word (`icons::name`). The drawing is the
  client's (`drawingOf` in `ui/src/icons.ts`: chosen over declared; an unknown name, or one
  whose set has not loaded, is drawn as the field kind's mark).
- The set is `lucide-static`, a build-only dependency, compiled in and never fetched
  (`icons:set`, a module `ui/build.mjs` makes; `ui/scripts/iconset.ts` decides what is built).
  It is a separate chunk loaded only when a picker opens or a field naming an icon is drawn
  (`wantIcons`). Icons whose name or search words spell a package, its program, or a line of
  `extensions/reserved` are left out (read where the `providers` gate reads them; the build
  fails if it cannot read them). The set's notice is kept as a comment in the built chunk.
- A column's mark is always drawn by `FieldMark` (`ui/src/views/mark.tsx`): in a table's head
  and on the first line of the head's menu. Whoever may save a view gets it as a button on that
  menu line (`IconPick` in `views/iconpicker.tsx`, `mayChoose`), which opens the icon panel
  under the head; others see the icon only. The panel is fixed on the page's body where `placed`
  puts it, and draws a few hundred icons at a time. `icons.mjs` checks it.

## Column heads and their menu

- A column has one thing to press, its whole head (`ColumnHead` in `ui/src/views/headmenu.tsx`):
  pressed (or Enter, Space) it opens the column's menu; held and dragged sideways past a few
  pixels it carries the column. The menu is one frameless panel on the page's body, placed under
  the head by `placed`, and is a word of what Escape is first for in the drawer (`.headmenu`).
- Which lines it has is one function (`lines` in `ui/src/columns.ts`, with cases), with only
  lines that work today. Lines that change the columns themselves are in the next section and
  drawn by `views/shaping.tsx`.
- **Sorting is the reader's own**, not part of a view: everybody has it, it saves nothing, and
  it is kept in the browser per view (`useSort`, `client.sort.<project>.<table>.<view>`).
- Every other line changes the view, which is the table's for everybody, and is offered only to
  whoever may save one (`mayChoose`): hold the view to values of the column (`only`;
  `holdableBy` in `ui/src/ordered.ts`; no line for numbers or dates), group rows by it (`group`;
  `useBands`, `BandHead` in `views/ordered.tsx`), a figure at its foot (`figures`), freeze up to
  it (`freezes`), hide it (`hides`), move it (`columns`). Each is a `Change` computed from the
  view as it is and saved in turn (`InTurn`).
- The four column keys of a view are plain values checked in `kernel::columns` (`Columns`,
  flattened into `studio::View` as `shown`): `columns` (order; unknown names skipped, never
  refused), `freezes`, `figures` (column to one of `FIGURES`, checked against the column's kind
  by `fits`), and `widths` (column to width). `freezes` and `figures` are refused when a view is
  declared or saved (`View::shows`), never when kept, so a saved view outlives a column it
  names. A width is ignored when read if not whole or outside `NARROWEST` (60) and `WIDEST`
  (2000). All three apply to tables only. A removed column takes its width with it (`forget` in
  `studio::added`).
- Client functions: `banded` keeps columns in view order inside each group, never across groups;
  `carried`, `stepped`, `landing`, `frozen` and `staying` in `ui/src/columns.ts` decide drops
  and sticky columns; `ui/src/figures.ts` computes figures, counting and never scoring (empty
  text and empty lists count as nothing; the mean of no numbers is no figure). `goes` in
  `ui/src/table.ts` carries order to table, list and gallery, and freezing, figures and widths
  to tables only.
- **Frozen columns** use `position: sticky`, offset by the widths before them as the browser
  reports them (`useStaying` in `views/frozen.ts`, a `ResizeObserver`, never a timer), measured
  from the scroller's edge (`--side`, `--foot` on `.grid-scroll`). Only as many stay as leave
  `ROOM_LEFT` for the rest (none at phone width). Where the frozen edge falls inside a group,
  its heading splits into two cells (`headings`). Freezing is by column name, so carrying a
  column across the edge joins or leaves the frozen set.
- **Carrying a column** moves nothing until let go: the head is drawn fainter, a line shows
  where it would land (`.grid-drop`, placed each frame from positions measured at the start),
  and nobody is told until release; releasing where it started, or Escape, tells nobody. A click
  after a carry opens nothing (`carriedNow`). A finger scrolls instead; the menu's Move left and
  Move right, and keyboard moves in the layout's column list (`listed`, `steps`), do the same.
- Every table page follows `collection.views` (`ui/src/store.ts`), so a view saved on one page
  is drawn on another. A reading of a project's tables overtaken by a local change is not kept
  and is read again (`tablesChangedHere`).
- `ui/src/api.ts` and `views/table.tsx` are near their line limit: view tabs and the layout
  panel are in `views/layouts.tsx`, and something new a head does goes in its own file.
  `headed.mjs` checks all of it.

## Column widths

- **A width belongs to the view, for everybody, and is stored at the usual text size**: whole
  pixels the page draws times `--size` (`--cw` on the column's cells, `.sized` in
  `ui/src/widths.css`). A column the view says nothing of fits its content; a group heading
  spans its columns.
- It is dragged by an edge at the right of the head (`WidthEdge` in `views/widths.tsx`,
  functions in `ui/src/widths.ts` with cases), shown only to whoever may save a view and only
  where there is a pointer (`hover: none` draws none). The edge is not part of the head's
  button: a press on it never opens the menu or carries the column.
- While held it behaves like any dragged edge (`held.ts`): nobody is told, and the width is
  applied each frame by one CSS rule on the column's cells (`heldRule`, a `style` element and a
  mark Preact does not own, so a redraw in between loses nothing). Released, it is saved once as
  a `Change` and the rule stays until the save is answered. Releasing where it started, or
  Escape, saves nothing; a double press clears the width; arrow keys on the edge change it by
  `STEP`, each step saved.
- In the phone layout widths are not applied. `Heads.change` resolves when the save is answered
  (`laid.tsx`, `board.tsx`). `widths.mjs` (run by `headed.mjs`) checks it.

## Columns people add

- A table's columns belong to the people who look at it as well as to its blueprint
  (`studio::added`, plain values in `kernel::added`, which is unrelated to `kernel::columns`, a
  view's settings). Three things are kept on a `Field`, in the table's row: `added` (a person
  added it: the table's, not any declaration's), `label_chosen` (a display label for any
  column), and `of_chosen` (a chosen order for a declared choice's options).
- **A column's name is identity and never changes**: it is the key in every record, file, view,
  `groups`, `about`, a blueprint's `rows.read` and `${…}`, and a model. Renaming a column sets
  its label, shown over the name in headings only (`heading` in `ui/src/table.ts`,
  `added::heading`); the menu says what blueprints and files call it. The name is made once from
  the first label (`kernel::added::name`: lowercase letters, digits and `_`, starting with a
  letter), numbered where taken or reserved (`RESERVED`); no two headings of a table are the
  same.
- A declared field is not changed here: its kind, options and existence are the declaration's
  (`theirs` refuses, saying so). People may relabel it, choose its icon and order its options.
- A column somebody adds never gives or guards anything: no `takes`, `takes_of` or `takes_each`,
  never `required`, `quiet` or `hidden`, and one of `kernel::added::KINDS` (which excludes
  `actor`); a `record` or `records` column names a table the adder may read. A test checks that
  the field `add_column` makes declares none of these.
- Changing which columns exist (add, retype, remove, a choice's options) takes its own scope,
  `columns`, beside what saving a view takes. Members hold it; agents do not unless their row
  says so. A label and option order take what saving a view takes. A hidden table answers as
  missing; libraries and mockups are refused.
- **When the table is declared again** (`added::beside`, called in `Collections::ensure` before
  checks; `added::carry`, inside `Collections::keep`'s write): added columns stay after declared
  ones with their values; chosen icons, labels and orders follow by field name; a field the
  declaration drops loses what was chosen of it; a declared field with an added column's name
  takes over, with values where `refit` passes and refused where it does not. An option the
  declaration adds to a person-ordered choice goes after the one it follows
  (`kernel::added::ordered`). `Board::make` hands back what it read as a declaration, and
  `beside` strips what is people's and restores it from the row, so nothing becomes declared by
  being read back. Something new kept on a field that is people's is carried in `carry` by field
  name; something new that builds a `Spec` from a table's fields needs nothing, since `ensure`
  calls `beside`.
- **Each change is one write that emits its event** (`reshape`, through `Journal::write` of the
  journal that keeps the table: `collection.fields` with `did`; `collection.views` where a view
  changed; `board.columns` for a board's column order). The row is read, changed and written in
  that write, fields and views separately, so concurrent view saves, icon choices and
  declarations lose nothing (studio and API suites' `added.rs`).
- What a change must check of the records (whether each fits a new kind, `fields::misfits`;
  whether an option to remove is still chosen) is checked before the write, and the write is
  refused if the column changed since (`moved_on`). So the page submits a box once, not on Enter
  and again on blur.
- Rows kept in the journal's database change in the same write: a new column starts empty, a
  removed one is removed from rows (`json_remove`), a renamed option is rewritten (`json_set`)
  with no `record.updated` per row. Rows kept as files or by a store service are not rewritten:
  a renamed option is reported with how to proceed, and a column is not added under a name the
  files already use. Retyping is accepted only if every row fits; nothing is converted. An
  emptied cell is empty text, which every field takes as nothing (`fit` in `fields.rs`).
- A view outlasts a column (`forget` removes what views said of a removed column; `tidy` removes
  a view the fields no longer allow, never the table's last). A column added from the page
  carries its position in the view it was added in, in the same write (`Place`). An added column
  is in no group.
- **A board's columns and its `status` options are one order**, arranged from the board's page
  (`Board::set_columns`) or the column's menu, kept as `of_chosen`; applying the board's
  blueprint again no longer resets it. Columns added there are the board's own (`of_added`,
  computed by `kernel::added::added`); declaring again or taking a new `std/board` version keeps
  them in place with their cards, removes only what a declaration offered, and inserts new
  declared columns after the one they follow (`kernel::added::offered`). Every way of adding a
  board column goes through that door (`mw board columns`, the route, the board's page; API
  suite `kept_columns.rs`).
- A fleet's three tables accept added columns like any table (`segments` reads only the cells it
  names; `rows.read` hands a row whole); API suite `added.rs` works a card through after the
  fleet is applied again.
- Client: `ui/src/added.ts` (plain functions with cases: `shapes` for which menu lines,
  `mayShape` for who may, `placing` for where an inserted column goes, `filling` for what
  filling a column takes, `whose`), `ui/src/shaped.ts` (the requests), and `views/shaping.tsx`
  (drawn inside the column menu, frameless). Every open page follows `collection.fields` and
  re-reads rows, since renaming an option or removing a column emits no per-row event.
  `columned.mjs` checks it.

## Comments on records and documents

- What is said on a record is a comment; what happened to it is its history. The daemon keeps
  one thing, the history, and a comment is a history line that is something said (`comments` in
  `ui/src/journey.ts`, with `happened` for the rest; every line is in exactly one).
- **Each line is drawn with whoever it is of**: their picture beside their name, the same face
  as in a channel, both one button that opens where they are read about (`readAbout`: the Tools
  panel beside the showing channel, else the drawer). Before what happened, the picture alone.
  Who that is is one function (`speakerOf` in `ui/src/journey.ts`, with cases): who said or
  asked it, the factory's bot where a blueprint's step did, an agent by name, nothing pressable
  for an unknown name. An asker that is a record (an A/B test asking on its card, `recordOf`) is
  drawn as the bot speaking for it, with the record named after and pressable. The picture's
  letters come from CSS (`data-letters`), so copied text is just the words.
- **A document's comments are in a panel beside its text**, of the Tools panel's kind, opened by
  the speech-bubble button that is the last of its head's row, after the gear (`tool` of
  `PageActs`, handed by `FileActs`): `DocTools` in `ui/src/views/doctools.tsx`, over
  `ui/src/doctools.ts` with cases, in the `file` tab. Three tabs in order: Comments (every
  comment, oldest first, each on words saying what it is on and whether those words are still
  there, with the line to add one); Details (path, library, size, last change and by whom,
  public mark, comment count, link to history); History (as `Lines` draws it; the gear's History
  opens it too).
- Whether it is open, which tab shows and which mark's comments are open are kept by the
  document's tab (`useKept`). It slides like the Tools panel (`useSlid` with `DOC_SLIDES`,
  `.doc-with` in `drawer.css`), reports rest (`docToolsAtRest` in `ui/scripts/drawer.mjs`), is
  widened by its edge (`Edge`, its width kept separately), leaves the text the larger share of a
  narrow tab, and takes the screen on a phone with a way back at its head. Anything that asks
  for a document's comments by name (the mark on its line in the library, the `comments` hint of
  an address, a block's Comment) opens it on Comments with the keyboard in the input
  (`askComments` in `views/file.tsx`).
- Which record a file is, is one function of the project's tables as read (`documentAt` in
  `ui/src/library.ts`, with cases): the first listed library whose folder holds the file,
  matching the daemon (`Collections::document_at`). A file that is no document gets none of
  this. What is being typed is kept in the store under the record (`say:<record>`).
  `library.mjs` checks the panel.
- A thing with no client page of its own (a mockup, whose page is its author's scripts) has
  comments in the drawer before its history (`Commented`); nothing is drawn into a mockup's
  page. A new place that shows what was said on a record uses one of these, and says Comment and
  Comments. A card's drawer keeps its own words and is where the `public` mark is offered; a
  document's comments offer none.

## Comments on a document's words

- A comment may be on a few words, and **nothing of it is kept in the file**, which other
  programs write. What it is about is kept with the comment (`anchor` of `record.commented`,
  checked and bounded where written, `kernel::anchor`; `Records::comment_on`) and found again by
  searching the current text (`found` in `ui/src/anchors.ts`: the words with context under the
  same headings, then the words under the same headings, then anywhere, else not found; cases in
  `ui/src/anchorcases.json`).
- The block editor and the source read a document as the same blocks of plain text
  (`blocksOfDoc` in `write/selected.ts`, `blocksOfMarkdown`), each character mapped to its
  source position, so marks are placed there and no position is ever stored.
- A comment whose words are not found is listed in the Comments tab saying what it was about,
  never dropped (`saidOn` in `ui/src/anchored.ts`). The published page draws no marks; every
  comment is listed in the panel.
- A mark's comments open beside the text, at the words' height (`marginOf`), and at the head of
  the panel's Comments tab only where the page cannot make room (`aside` of `useThreads`). The
  page makes room (`roomy`, `margined` in `write/writer.tsx`): where the centred column leaves
  too little to its right, it shifts right as far as leaves `--room` (checked against `ROOM`)
  and narrows only as needed. **It does so before anything is pressed**, as long as the document
  has comments on words, so a pressed mark stays under the pointer and closing a card moves
  nothing. The open mark is drawn as open in both editors.
- Finding is the client's alone; if the daemon ever needs it, it is written there against the
  same cases. Code that changes a document's text needs nothing for comments: they follow the
  words. `selected.mjs` checks it.

## Block handles

- A top-level block of a document written as blocks has a handle (`ui/src/write/handle.ts`;
  decisions in `write/handled.ts`, with cases: which block the pointer is at, where a carried
  block lands, what Turn into keeps, which menu lines, how a block comment begins).
- Hovered, a block shows a `+` and a grip in the margin to its left. They float in the editor's
  scroll box and are not part of the document, so nothing moves as they appear or as the pointer
  crosses to them; they exist hidden before anything is pressed. A narrow margin shows only the
  grip; one with no room shows neither.
- `+` adds an empty line below with the `/` menu open. Dragging the grip (pointer events, so
  touch works) moves the block: it fades, a line shows where it lands, Escape cancels.
- Pressing the grip (or Enter) opens the client's `RowMenu` (through `writer.tsx`): Turn into
  (the `/` menu's text blocks, current one marked), Duplicate, Move up and Move down where
  possible, Comment where the document takes comments (starts a comment in the side panel that
  names the block by its first words: `commentBegun` in `views/comments.tsx`), and Delete.
  Ctrl+/ opens the same menu for the block with the cursor; Escape returns focus to the grip,
  then the text.
- Turn into loses no words: a multi-line block turned into a one-line kind becomes one block per
  line; lists keep their items. Every action is a normal document edit, undone and saved like
  typing; no route or event of its own. Typing hides the handle until the pointer moves. On a
  phone the page keeps room on the left for a finger-sized grip for the cursor's block, and
  there is no `+`.
- A comment belongs to the document, not the block; naming the block in its words is the only
  tie. `handled.mjs` checks it.

## Comment counts

- How many comments each record of a table has is one request per table (`GET
  /api/collections/{project}/{name}/commented`, `Records::commented`, `studio::commented`): one
  statement over the histories, limited to that table's records' lines (which sit together
  because a table's record ids share a prefix; ownership is still checked by
  `Collection::owns`). Nothing is stored for it, so copies and moves need nothing. It is visible
  only to whoever may read the table's records (`crates/api/tests/suite/commented.rs`). A list
  never fetches each record's history to count.
- The client reads it once per list and again when a history line that is something said is
  written for one of the table's records (`useCommented`, `commentedOn` in
  `ui/src/commented.ts`): on `collection.history`, emitted in the write that keeps the line,
  never on `record.commented`, which arrives first. A count has no ids, so it is re-read, never
  incremented, and of two readings in flight the last requested wins (`Latest`).
- The comment mark has its own room in every line and is only hidden where there are none, so
  nothing moves when it appears; without a pointer (`hover: none`) every mark is shown.

## What somebody may ask, shown by area

- What a person or agent may ask of the factory is shown by area (`ui/src/holds.ts`,
  `views/holds.tsx`): a line per area, what is held there in words, and what is not held. The
  areas and their actions are the daemon's (`kernel::principal::areas`, from the one list every
  scope is checked against, sent as `scopes` of `/api/meta`); the client lists none itself. A
  new scope is a line of `kernel::principal::SCOPES` and appears with nothing else done.
- For an agent that is a row of a fleet (found by what the fleet calls it, `rowOf`), a changed
  line writes the whole list into the row's `holds`, and the fleet follows the row; the page
  never changes such an agent's slot directly.
- An agent in no row (added by hand) is changed there too, by whoever may change agents
  (`mayChange`), sending only what it is to hold (`PATCH /api/agents/{name}`, `Agents::give`): a
  stopped agent stays stopped. Nobody gives what they do not hold (the one check in
  `Agents::ensure`).
- A changed agent holds the new list from its next request, even mid-turn: its credential is
  replaced, and its program reads the current one from its file on each request (`TOKEN_FILE`),
  never caching it.
- The sentence under the table says how it is changed and by whom (`saidApart`), and never names
  a flag without what follows it.
