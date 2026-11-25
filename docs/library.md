<!-- public -->

# A library

A library is a project's documents: the Markdown files of one folder,
listed, written in, commented on, searched, and read as pages. It is a
table like a board, but its records are not kept in a database or in
files the factory made. They are the files already in the folder.

## Making a library

```sh
mw apply std/docs --project shop                       # the folder `docs`
mw apply std/docs --project shop --param folder=notes  # another folder
mw apply std/docs --project shop --param folder=.      # the whole project
```

The project then has **Docs** beside its board in the side panel.

A project added by hand gets `std/docs` applied for it, unless its
library was turned off (`--without docs`, or the switch in the dialog
that adds a project). Any other project has no library until you apply
it. Stopping the instance does not touch any file.

`std/docs` is a few lines around the action `collection.ensure`, which
any blueprint can use directly:

```yaml
steps:
  handbook:
    do: collection.ensure
    with:
      name: handbook
      store: documents          # the Markdown files of a folder, as they are
      at: handbook              # the folder; unless said, the one named like the table
      fields:
        title: { type: text, required: true }
        body: { type: text }
        path: { type: text }
        owner: { type: actor }
      history:
        record.created: "{actor} wrote it"
        record.updated: "{actor} changed its {changed}"
        record.commented: "{actor} said: {body}"
        record.removed: "{actor} took it away"
```

## What a document is

- **Any file ending in `.md`**, in the folder or in any folder under it.
  Folders that are never listed (`target`, `node_modules`, `.git` and the
  like) are skipped, and a link that leads out of the folder is not
  followed.
- **Its title** is the heading it begins with (`# Installing it`). A file
  with no heading at the top is called by its file name.
- **Its body** is the rest of the file.
- **Its `path`** is where it is in the library: `guides/install.md`.
- **Any other field its table declares** (`tags`, in `std/docs`) is kept
  in a settings block at the top of the file: between two lines of three
  dashes, one field per line.

  ```markdown
  ---
  tags: ["how-to", "first steps"]
  ---

  # Installing it
  ```

  A document has the block only when something is in it. Lines in the
  block that are not fields of the table are left exactly as they are.
  The block is read leniently: `tags: [how-to, first steps]` is the same
  list.

Nothing is added to a file to make it a document. A file written in an
editor, by an agent, or arriving with a change to the project is in the
library the next time the folder is read.

## A document's id

A document's id is where it is: `<project>-<library>:<path>`, with the
path's `.md` dropped and `:` between its parts. `guides/install.md` in
Shop's `docs` library is `shop-docs:guides:install`. So it is the same
document on every device with the same files, and after a restart.

**A moved file is a different document.** Moves are not followed: what
was said on the document, and its history, stay with the old id. To move
a document, move the file; you cannot change a document's `path` field.

## Writing and changing documents

In the client, the library's page lists the documents by folder. A
document opens where any file does: as blocks to write in, or as the page
it would be published as. **New document** makes a file named after its
title, in the folder you choose, with the title as its heading. A
document with comments has a comment mark at the end of its line saying
how many; the mark opens the document with its comments showing (see
[Comments and history](#comments-and-history)). **Aa** at the head of a
document is how it reads for you (its text's size, Standard or Wide, and
light or dark), kept with your account, as on a topic's page.

From a blueprint or the terminal, a document is a record like any other:

```sh
mw record add shop docs title="Release notes" path=guides/releases
mw record set shop-docs:guides:releases tags=news,first
mw record comment shop-docs:guides:releases "Is this the place for 0.2?"
mw record show shop-docs:guides:releases
mw record remove shop-docs:guides:releases
```

`record.add` names the file after the title when no `path` is given, and
never writes over a document that is already there.

## Other views of a library

The library's page lists documents by folder. Anyone who reads it can
save other views of the same documents beside that, one tab each, as on a
board: a table (a row per document, its fields edited in their cells), a
gallery, or a calendar (by the day each was made, or by a date field the
library declares). A saved view belongs to the library, so the next
person to open the page has it too; which view you are looking at is
kept in your browser. The same holds for any table a blueprint declares,
and for a project's mockups.

### Opening a row into what names it

In a table or a list, a row can be opened into the records of other
tables that name it. For example, a document of `quarters` whose settings
say `year: life-years:2026` is listed under 2026 in the table of years,
and opens in turn into the months that name it.

Nothing needs declaring for this but the fields that name another record
(`year: { type: record, of: years }`). It goes as deep as records name
each other. A field may name several records
(`spans: { type: records, of: years }`, written
`spans: ["life-years:2025", "life-years:2026"]` in a document's
settings); such a record is listed under each of them.

Under an opened row, **+ New** writes a record that names it: a quarter
of that year, which is a file placed where its title says.

`examples/blueprints/period-notes.bp` lays out a person's notes this way:
years, quarters, months, weeks and days, a library each.

## Kept private

A library may say who reads it, with `reads: private:journal` on its
`collection.ensure` ([blueprints.md](blueprints.md), *Permissions*). Its
documents are then read only by whoever holds that scope: the factory's
owner, who holds every scope, and anyone given it. Someone who may only
read the project cannot read them in any way: not in the library, not as
files in its folder, not as pages, not by searching, and not from behind
the wall of an agent working in the project. To let an agent read them, give it
the scope in its row (`holds`), or with `scopes` when it is added.

This does not hide them from anything that reads the disk as the user
who runs the daemon: a program started with no wall, or a blueprint's own
command.

## Comments and history

Writing, changing and removing a document through the factory, and
anything said on it, each add a line to its history, worded by the
blueprint as a card's is. That includes saving in the client's editor:
the change is the document's, and whoever saved it is named.

**Changes made by another program are not history lines**, because
nobody saw who made them. The change is in the library at once and the
search finds it; whatever keeps the project's history (git, for example)
has the rest.

### The comments panel

In the client, a document's comments are in a panel beside its text on
its own page. The speech bubble at the end of the page's head, just after
the gear, opens it and shows how many comments there are. The panel has
three tabs:

- **Comments**: every comment, with who said it, when, and the words,
  newest last. Under them is **Add a comment** (Enter sends; Shift+Enter
  starts a new line). A comment marked public says so here, as its
  history line does. The page cannot mark a comment public; to post a
  public one, use the terminal or a blueprint
  (`mw record comment <document> --public <words...>`). The comments are
  the history lines that are something said.
- **Details**: what the document is: where it is, its library, its size,
  when it was changed and by whom, whether it is marked public, and how
  many comments it has.
- **History**: everything that happened to it, in order, comments
  included. **History** in the gear of the page's head opens it too.

Drag the panel's edge to widen it. It stays open, on the tab you left it
on, when you come back to the document's tab. On a phone it takes the
whole screen.

In the library's list, the mark at the end of a document's line says how
many comments it has, and stays current as comments are made. On a
document with none, the mark shows only while its line is pointed at or
the mark has keyboard focus. Either way it opens the document with the
panel on its comments and the cursor ready to add one. The counts are
read once for the whole library
(`GET /api/collections/<project>/<library>/commented`), not per document.

### Comments on a few words

A comment can be about a few words of a document. Select words within
one block in **Write**; when you let go, a small bar with **Comment**
appears over them, and what you then say is about those words.

Other programs write the file too, so nothing of the comment is kept in
it. What the comment is about is kept with the comment, in its history
line (`anchor` beside the words), and sent with its event
(`record.commented`). The anchor holds the quoted words, a few words
either side, and the headings over the block. Each part is checked when
written and has a limit: 400 characters quoted, 40 either side, six
headings. From a terminal:

```sh
mw record comment shop-docs:guides:install --on "the second step" --before "Then run " --under Installing --under "On Linux" Which one is that?
```

The page finds the words again by searching the text as it is now, in
this order (`ui/src/anchors.ts`, with its cases):

1. the words with their neighbours, under the same headings;
2. the words alone, under the same headings;
3. the words anywhere, choosing the place whose neighbours are most like
   the kept ones;
4. otherwise, not found.

Found words are marked in **Write** and **Source** alike, by colour only:
pointing at a mark moves nothing. Pressing a mark opens its comments,
with a line to add another on the same words, and the mark is drawn as
the open one while they show. They open beside the text, at the height
of the words.

When the page is too narrow for that with its column centred, it makes
room: while the document has comments on its words, its column sits
further left (and narrower only where it must), so a pressed mark already
has its room, stays under the pointer, and nothing moves when the
comments are put away. When even that would leave too narrow a column (a
narrow tab, a phone), they open at the top of **Comments** in the panel.

Every comment on words is listed in the panel too, saying what it is on.
A comment whose words are no longer found says what it was about and that
those words are gone. It is never dropped. **Page**, the document as it
is published, draws no marks: every comment is listed beside it, each
saying what it is on.

A Markdown file that belongs to no library has none of this; its page is
like any file's. Which document a file is depends only on where it is,
as above.

Comments are not part of the document's file, and none of them is
written out when the library is written out as pages.

### Records, cards and watching

Because a document is a record, anything that works on records works on
it. A card can name one (a `record` field with `of: docs`), and
`std/watch` reports in a channel what happens to one document, or to all
of them:

```sh
mw apply std/watch --project shop --param collection=docs --param channel=general
```

## Noticing file changes

Every library's folder is watched: the operating system reports changes,
and nothing polls (see [architecture.md](architecture.md), *Being told,
not looking*). When something changes, the project's files are scanned
once, as after a run, and the change is reported as usual
(`project.file.changed`). The search and every open page follow that
event.

When the system cannot watch (its limit on watched folders is used up),
a change is found at the next scan of the project's files: after a run,
or with `mw project scan`.

## Reading a library as pages

The daemon serves every library as web pages:

```
http://127.0.0.1:7700/library/<project>/<library>/
```

- **The front page** is the folder's `README.md` or `index.md` if it has
  one; otherwise a list of what is in it, by folder. Each folder inside
  the library has a front page the same way.
- **The contents are beside every page**: the library's folders and
  documents, with the current page marked. They sit at the page's left
  where there is room, and under it on a phone, with **Contents** at the
  top of the page linking there. They are links and nothing else. A
  folder is shown by its front page's title, or else its name.
- **Order** comes from a list in the head of a folder's front page: its
  documents (by file name, with or without `.md`) and folders (by name,
  with or without a trailing `/`):

  ```
  ---
  order: [install, usage, more/]
  ---
  ```

  The list can also be written one item per line under `order:`, each
  starting with `-`. Anything not in the list comes after it: documents
  first, then folders, each by title. Names in the list that do not exist
  are skipped. A folder with no front page is ordered by title
  throughout, and its contents list follows the same order. The rule is
  one function, `kernel::contents`, with its cases.
- **Each document** is the page that publishing would make of it (the
  same code as the **Page** view), so `guides/install.md` is at
  `/library/shop/docs/guides/install.md`. Links between documents, and
  pictures beside them, work as written in the files.
- **Nothing in a page runs.** A document may contain anything, so its
  page is sent with a policy that allows no script and fetches nothing
  but pictures from the library.
- **Only for whoever may read the project's records**: someone signed
  in, or a credential that holds `records:read`. None of it is public.
- **Kept inside the folder.** A path containing `..`, a path from the
  top of the disk, and a link in the folder that leads out of it are all
  refused.

In the client, the library's page opens this with **Share**, then
**Read as pages**; a document's page opens its own page with **Share**,
then **Open as a page**. Both menus can copy the address too.

## Writing a library out as pages

The same pages can be written out as plain files, for any web server:

```sh
mw apply std/site --project shop                        # the documents that say they are public
mw apply std/site --project shop --param publish=all    # every document
mw apply std/site --project shop --param to=public      # into `public`, not `site`
```

`std/site` writes the library out once, into a folder of the project
(`site`, unless `to` says another), and again whenever a library file is
added, changed or removed, for as long as it stays applied. Your own
blueprint can do the same with one step, `library.export`.

In the client, **Share** on the library's page has **Write it out as
pages**, which applies `std/site` with nothing said. It is offered to
whoever may apply a blueprint. From then on **Share** says where the
library is written and opens the instance that writes it.

### What gets written

- **Only published documents.** A document says it is published in its
  settings block:

  ```
  ---
  public: true
  ---
  ```

  Nothing else of the library appears in the output. `publish: all`
  writes every document. A library made by `std/docs` keeps `public` as
  a field, so it can be set like any other. In the client it is the
  **Public** switch under **Share** on the document's page, which the
  daemon writes into the file. The switch is closed while the page has
  unsaved changes, since it would write the file under them.
- **The files**: `index.html`, the front page; a page per document
  (`guides/install.md` becomes `guides/install.html`); an `index.html` for
  each folder with a written document (its `index.md` or `README.md`, or
  else a list of its contents); the pictures and files those pages use;
  and one style sheet, `page.css`. Each page is the page the daemon
  serves, made by the same code, with the contents beside it in the same
  order (listing only what is written).
- **Every link leads to a file that exists.** Links are written relative
  to the page, so the folder works at any address, under any path, and
  opened straight from disk. A link to an unpublished document, to a
  missing file, to somewhere outside the library, or from the top of an
  address is written as its plain words, and each one is listed in the
  step's output (`left_out`: the page, the link, and why). Links to
  other sites are kept as they are.
- **A picture or file is copied only when a written page uses it**, and
  only if it is a kind that is shown or kept and never run: pictures,
  PDF, and plain data (`txt`, `csv`, `json`, `toml`, `yaml`). An `svg`
  that contains script is left out. A picture on another site is not
  fetched; it is written as its plain words.
- **Nothing in the output runs, and nothing is fetched from elsewhere.**
  Each page says so itself, so a browser holds it to that whatever
  serves it. Raw HTML in a document stays in its page as written, but a
  link or picture written that way is not followed or copied, and script
  written that way does not run.
- **The folder is written whole or not at all.** It is built beside its
  final place and moved there when done, so whatever serves it sees
  either the old pages or the new ones. A page that has not changed
  keeps its old write time.
- **Only an export's own folder is replaced.** An export lists what it
  wrote in the folder (`.exported`). A folder that holds anything not on
  that list is left exactly as it is, and the export fails, naming the
  file: move it out, or choose another folder.
- **Beside the library, inside the project.** The folder is given from
  the top of the project. A folder outside the project, inside the
  library's own folder, or around it is refused. So a library that is
  the whole project (`folder=.`) cannot be written out until it has a
  folder of its own.
- **Who may**: whoever may read the library and write the project's files
  (`records:read` and `projects:write`).

### Putting it on a host

The output is a folder of files and nothing more, so it goes wherever
static files go: copy it to a web server's folder, give it to a service
that serves a folder, or keep it on a branch or repository that one
serves. It works under any path of an address with nothing to configure.
Hosting it is not something the application does. If the folder is
inside a repository and should not be part of its history, add it to
`.gitignore`, or write it into a folder that already is.

When several files change at once (a change to the project bringing
many documents), the library is written out once per file, one after
another. The first does the work; the rest find the folder already as it
would be written and leave it.

## Not built

- Hosting the output, and a list of what would be published before it
  is. See [roadmap.md](roadmap.md), *Publishing*.
- A project's own page template: every page is drawn one way.
- Searching within the output; a site map or a feed.
- A check of the output against names that must not appear in it.
- Following a moved file.
- A library kept anywhere but in a project's files.
- Listing a library without reading every document in it: a very large
  library is read whole each time it is listed.
