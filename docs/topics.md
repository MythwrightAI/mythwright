<!-- public -->

# Topics

A channel is a conversation: what was said an hour ago scrolls away under
what is said now. A **topic** is the other kind of talk, a forum's: one
question or one piece of knowledge, made once, with a title, a post that
says what it is about, a summary of what is known now kept at its top,
and replies under it, each under what it answers. It can be found again
by its title, and where it is a question, it stands open until it is
answered or closed.

Use a channel to work something out together as it happens. Use a topic
for what somebody will look for later: *Which database for the shop*,
*Why the build is slow*, *How we name branches*.

## Where they are

**Topics** has a place of its own on the rail at the left, just after the
channels (Alt and 5 shows it). Its panel lists the topics of the
workspace the side panel is of, the latest written first, each with how
it stands where it is answered, closed or standing and its project's name
where it is a project's. Those that are closed are folded under **Closed**, which says
how many there are. The box at the panel's head finds a topic by its
title, by the first line of its summary, by how it stands and by its
project.

A topic is a workspace's or a project's, as a channel is: it is listed
with the workspace's things, or under its project. That is where it is
listed and nothing more (the next part says who may read it).

## Starting one

In the client, **New topic** at the head of the Topics panel asks for a
title, and where it is to be: the workspace the panel is of, or one of
that workspace's projects. **Make it** opens it, and the rest is written
on its page.

From a terminal:

```sh
mw topic make "Why the build is slow" --body "It takes minutes on every change." --status open
mw topic make "Which database for the shop" --project shop
mw topic list
```

A topic made with nowhere said is the usual workspace's own;
`--workspace <id>` names another, and `--project <name>` puts it in a
project. A topic is named afterwards by its title, whatever its capitals,
or by its id (`@-topics-3`, which `mw topic list` prints). A topic has no
status unless one is said: *Status*, below, says when to give one.

The first topic of a factory is made by whoever manages it: making it sets
the topics up, by applying `std/topics`, which takes `admin`. After that
anybody who may write records makes them. Somebody else who tries first
is told who can.

## Wikis: pages under a page

A topic may hold topics under it, to any depth: each is a **page**, and a
page with no page above it is a **wiki**. Every topic a factory had before
is a wiki of its own, with nothing under it. A page keeps everything a
topic has (its post, summary, replies, answer and revisions), and is
where its wiki is: a page under a project's wiki is the project's.

```sh
mw topic make "Handbook"
mw topic make "Deploying" --under "Handbook"
mw topic make "Rolling back" --under "Deploying"
mw topic move "Rolling back" --under "Handbook" --before "Deploying"
mw topic move "Rolling back" --top
mw topic show "Deploying"
```

`--under` names the page it goes under and `--top` makes it a wiki of its
own; `--before` names a page beside it that it goes before, and without
it it goes at the end. Every page under it goes with it. A page cannot go
under itself or a page under it, and a page moves among the wikis of its
own workspace: a whole wiki is put in another with `mw topic move <wiki>
--workspace W`. `mw topic show` says where a page is and the pages under
it. In this version the Topics panel still lists every page flat.

## Its page

A topic opens as a page of its own, and its address is its title
(`/topic/Why the build is slow`, as the address bar says it). At its head
are its title, how it stands where it has a status (open, answered, closed
or standing), where it is and how many replies it has. Whoever may change
it chooses its status there, or **Not tracked** for none.

Under that, a topic is one of two things, by how it stands and what it
holds:

- An **article**: a topic that is no tracked question (it is standing, or
  has no status) whose post says something, and every such page of a wiki
  under another page, written or not. It has three tabs under its
  head. **Article** is the page itself: its summary, its post (the
  article), and the pages under it, listed at its end. **Talk** is its
  replies, and says how many there are: the reply that answered it comes
  first while it stands answered, then a box that begins a new chain, then
  the replies as a tree. **History** is each revision of what it says.
- A **thread**: a question tracked as open, answered or closed, whatever
  its post says, since a question is talked out; and any other topic with
  no post. It has no tabs: under its head are its summary, the reply that
  answered it, a question's post, the pages under it and its replies.
  **History** at its head shows its history in their place, and pressed
  again, them.

Every part is there only where it says something. The **summary**, what
is known now in a few lines, is drawn only once somebody writes one:
**Add summary** at the head adds it, and emptying it (or **Remove
summary**) takes it away again. **Write an article**, at the head of a
thread that is no tracked question, brings the two tabs and opens Article
to write it in; an article saved empty that is no page of a wiki is a
thread again. A question with no post offers **Add post** instead, which
writes it in the thread; a question taken off tracking (**Not tracked**,
or Standing) whose post says something is an article. Nothing else is
kept to say which a topic is.

A page made under a page (**+ New page**, or the `+` of its line in the
Topics panel) opens on Article, its title chosen to be written and its
post under it. A wiki made from the panel's head is a thread until
something is written in it: its first reply is its first post, and Write
an article makes it an article.

An address of a topic opens its article; one that ends in `#talk` opens
its talk, and `#talk/r-42` its talk at the reply said at 42, marked and
brought into view; `#history` opens its history, and `#history/3` its
third revision; `#s/rolling-back` opens its article at the heading
Rolling back. An inbox line of a reply opens the page so, at that
reply. Which tab shows is kept while the page is open elsewhere, and the
address bar says it.

### Contents and sections

An article with three headings or more has **Contents**: its headings
listed in the order they stand, each under the heading before it that
stands less far in, numbered (`1`, `1.2`). On a wide page they are a
column at the article's left that stays in view however far it is
scrolled (and scrolls by itself where it is longer than the window), the
section being read marked; on a narrow one, a box at its top, folded
until it is pressed, which folds again once an entry takes you to its
heading. Each entry is a link to its heading, reached by the keyboard
like any link. **Hide** puts the Contents away in every article for you,
kept with your account like the rest of Aa: a small **Contents** in their
place shows them again, and so does Contents under Aa. An article with
fewer headings looks as it did, and so does a question's post on its
thread, whatever headings it has.

Every heading of an article has an address: the topic's, then `#s/` and
the heading's words in small letters with hyphens between
(`#s/rolling-back`; a second heading called the same is `-2`, and words
in any script stay as they are). Pointed at, a heading shows a link mark
that copies its address. To whoever may change the topic it also shows
**Edit**, which opens the article to be written with the cursor at the
start of that section: the whole article is still the editor, so a
change that runs over into the next section is one change. **Save**, or
**Cancel**, returns to reading the article at the section the cursor
was in. On a touch screen, where nothing is pointed at, both are always
shown, faintly. A question's post has no such addresses: Edit at the head
opens it whole.

The summary and the post (an article, or a question's post on its
thread) are read, by whoever may change them as by anybody else, until
**Edit** at the head is pressed, which opens both to be written in place;
the summary also has a small Edit of its own beside it. One that says
nothing yet (a new page, an article begun by Write an article, a post begun
by Add post, a summary just added) is open to be written at once. What is
typed is kept with **Save**, which returns it to reading, or with Ctrl and
S, which leaves it open; until then the page says it is not saved, and
keeps it while another tab is shown, or another tab of the client.
**Cancel** leaves it, asking first where something typed would go.

**Aa** at the head of the page is how it reads for you: its text a size
larger or smaller, its width the usual measure (Standard) or the page's
(Wide), the client light, dark or as your system asks, whether a
long article's Contents are shown or hidden, and its **Typeface**: Sans,
the client's own letters, or Serif, a book's, for an article's text, its
headings and its title (a thread, its replies and the rest of the client
keep their letters). Each is kept with your account and is the same on a
library's documents; Settings has them under Appearance.

## History

Each change of the title, the post or the summary is kept as a revision of
the topic, whole, by whatever door it came through: the page, a table's
row, `mw record set`, a blueprint, an agent (`GET
/api/topics/<topic>/revisions`, oldest first, each saying who made it by
name and the blueprint it came through, where one made it). **History**
lists them newest first: who made each, when (how long ago, and the moment
whole on hover), what it changed (title, summary, article; on a thread,
whose post is no article, title, summary, post), and that a blueprint made
it where one did. Whoever may read the topic may read its
history.

A revision chosen shows the topic as it read then. Two chosen by their
boxes and **Compare**, or **Compare with the one before** on a revision,
show what changed between them, part by part: the words added marked in
one colour and those removed in another, struck through, each said as
added or removed to whoever hears the page. Lines are matched first, and
the words of each run of lines that changed after.

**Restore this version**, on any revision but the latest, to whoever may
change the topic, asks once more and sets the title, the post and the
summary back to what that revision said, through the same door any change
goes through: so the restore is a revision of its own, which History says
restored the earlier one, and nothing is lost by it.

## Tables

A table is written in the post, or in a reply, as its lines: a line of
its headings with pipes between them, a line of dashes under it, and a
line for each row.

```
| Practice | Minutes a day | Studies |
|:---------|:-------------:|--------:|
| [Breath](topic:@-topics-4) | 10 | 12[^1] |
| Body scan | 20–45 | 7 |
```

It is drawn as a paper's tables are: a rule over it, one under its
headings and one under its last row, thin rules between rows, none between
columns. A colon at the left of a column's dashes sets its words at the
left, at both ends in the middle, and at the right alone at the right,
as figures are, lined up. A cell holds what a line holds: marks, links,
links to pages and footnotes' markers; a pipe that is a cell's own is
written `\|`. A row with fewer cells than the headings has empty ones,
and the table ends at an empty line or a line with no pipe. One wider than
the column scrolls sideways in its own box, a shade at the edge that has
more, never the page. Tables are drawn wherever words are: a post, a
reply, a channel's message, an agent's answer.

After **Edit**, a table is a block of its own holding its lines as they
were written, edited as text and saved as it was: nothing else of the
post is changed by opening it and saving it. An editor of tables by their
cells is not in this version.

## Pictures and diagrams

A picture goes in a topic's post or a reply by pasting it, dropping it
on the text, or choosing it: with **Picture or page** in the post's `/`
menu, or the paperclip under a reply's box. It is attached to the topic
and written into the text on a line of its own:

```
![How a sale goes](attachment:ratt_1m88s7raa1k5p5we "side")
```

What is in the brackets is its caption, said under it. Chosen in the
post, a picture has a small bar over it: **Side** has it stand at the
side of the column with the words going round it (on a wide screen; on
one as narrow as a phone it stands across), **Across** has it stand
across the column, and the box beside them is its caption. Written by
hand, `"side"` after the address does the same, and nothing there means
across.

- PNG, JPEG, GIF, WebP and SVG are drawn, at most as wide as the column;
  pressed, a picture opens whole in a tab of its own.
- **A `mermaid` block of code is drawn as a diagram**, in the post, a
  reply or an agent's answer alike: write the block between ```` ```mermaid ````
  and ```` ``` ````. It is made smaller to fit the column only while its
  words still read (at about 11px); one that would be smaller still is
  drawn where its words read and scrolls sideways in its box, a shade at
  the edge that has more. Its box is fitted to what it draws, with no empty
  band and nothing cut off; a gantt is drawn as wide wherever it is read.
  It is in the page's light or dark and the page's own colours: a pie's
  slices and a mind map's branches are eight hues told apart from each
  other and from the page, the same in light and dark. Pressed, or by
  **Open** under it, it opens whole over the page, as large as it was
  drawn (Escape puts it away), **Full screen** gives it the whole screen,
  as large as the screen holds it whole (Exit full screen or Escape puts
  it back), and **Code** shows what it was drawn from.
  One that cannot be read is shown as its code with a faint line saying
  what is wrong. Nothing in it runs and nothing is fetched to draw it. The
  ELK layout (`layout: elk`) is not in this version: such a diagram says so.
- **An SVG is drawn as a picture** too: draw a diagram in any program that
  writes SVG and put it in as one. Nothing in an SVG runs.
- An attached page (`.html`) put in the same way is drawn in place as a
  view, in the box and with the rules a channel's views have ([Views](views.md)).
  Under it, beside its caption, **Open larger** shows it in a dialog over
  the page and **Full screen** gives it the whole screen; it goes on as it
  was in each, and back in the text, since it is never loaded again.
- A picture is the topic's, so whoever may read the topic may see it and
  nobody else. Its address is the attachment's id, which no revision and
  no new title changes. One that was removed, or that is not the reader's
  to see, is drawn as a line saying so.
- A picture at an address somewhere else (`https://…`) is never fetched,
  since the factory sends nothing anywhere unasked: it is drawn as a link
  with its caption.

From a terminal, a file is attached with `mw record attach <topic> <file>`,
which prints its id, and the line above is written into the text with
`mw record set` or a reply.

## Links and sources

A page that rests on another page, or on something outside, says where it
came from: it links the pages it names, and gives its sources as
footnotes, gathered at the end of its article.

**A link to another page** is typed as `[[` and the start of its title,
in the post or in a reply's box: the pages are offered by title, each
with the wiki it is in, as `@` offers people, and the one chosen is
written as a link to its id, with its title as the words:

```
Many frogs died of [Chytrid fungus](topic:@-topics-49).
See [what Ada said](topic:@-topics-49#817039).
```

The second leads to one of its replies, by the number `mw topic show`
prints. The words are the writer's, as any link's are; where the link
leads is the page's id, so a new title or a move to another wiki changes
nothing of where it leads (pointed at, a link says the page's title as it
is now). Pressed, it opens the page, or the page at the reply. A link to a
page that was removed, or that the reader may not read, is drawn as *a
page you cannot open*, and the words written for it are not shown.

Pointed at for a moment, or reached by the keyboard, a link shows a card
of where it leads, as reading sites do: the page's title, its wiki and the
page above it, the first lines of its summary (or of its article, where it
has none) as plain text, and how many replies it has, with **Open**; a
link to a reply shows who said it and its first lines. Escape puts the
card away, and on a touch screen a tap opens the page with no card. The
card says nothing of a page the reader may not read: a link to one shows
none, and a link to one in what the card quotes is said as a page they
cannot open. A page is read once for its card while the client runs, and
again only after something is said of it.

**Linked from**, at the end of a page's article, lists the pages that link
to it, each with the wiki it is in, each opening it; it says nothing where
none do. Every page of a series carries the same series box, so the pages
whose only link is such a box are gathered at its end into one folded
line for each series (*3 pages of the series Meditation*), which unfolds to
them; a page that links from its words or another box is listed as any
other. It is read from what the pages say now (their posts and
summaries and the boxes in them, never what is written as code), so a link written, changed or
taken out is there at once.

**A source** is a footnote: a marker after what it supports, and a line of
its own, anywhere in the post (at its end, usually), that says what it is:

```
Chytrid fungus killed whole species of frogs.[^1]

[^1]: Ada Lovelace, *Notes*, Taylor, 1843. https://example.org/notes (checked 2026-11-25)
```

A marker is drawn as a small raised number, numbered in the order the
markers are first used, whatever their labels; pointed at or reached by
the keyboard, it shows its source, and pressed it goes to it. The end of
the article gets **References**: the post's sources, numbered, each with
a way back to where it is used. An address in a source is a link, and is
never fetched. In a reply, its sources are listed under it.

**Notes**, **See also** and **External links** are headings the writer
adds to the post where a page needs them, as an encyclopedia's are: a
heading and a list under it. Nothing draws them by itself.

An agent asked in a topic is always asked to do the same: to give its
sources for what it states as a fact as footnotes in this form, and to
link the pages it names. No topic turns that off, since that would need a
setting kept for it.

## Boxes and notes

An article can have the parts an encyclopedia's pages have besides their
text: a box of key facts at its side, a box of the pages that go
together with it, and a note at its top that points elsewhere. Each is a
block of code in a language of its own, written in the post like any
other, so it is text that an agent can write and that every revision
keeps. In the post's `/` menu, **Info box**, **Series box** and **Note at
the top** put one in to be filled in, and `[[` inside one puts in a link
to a page.

**An info box** is a title, a picture where it has one, and a fact a line:

````
```infobox
title: Frog
picture: attachment:ratt_1m88s7raa1k5p5we
caption: A common frog
Order: Anura
Species: 7,600+
Range: every continent but [Antarctica](topic:@-topics-7)
```
````

`title:` is needed; `picture:` is a picture attached to the topic, as
in a picture's line above, and `caption:` is said under it. Every other
line is `Key: value`: what comes after the first `: ` is the value, which
may hold links to pages and plain links (never fetched). `title`,
`picture` and `caption` are the box's own only as written here, in small
letters: `Title: …` is a fact.

**A series box** is a title and a link to a page a line, grouped under
`## ` lines where there are groups:

````
```series
title: Amphibians
## Frogs and toads
[Frogs](topic:@-topics-5)
[Toads](topic:@-topics-6)
## Tailed
[Newts](topic:@-topics-8)
```
````

The page it is drawn on is marked among them. It stands at the side;
`place: end` puts it at the end of the article's text instead, where a
series shared by many pages usually goes. Write the same box in each page
of the series, and each marks itself.

**A note at the top** is a line or two (three at most):

````
```note
This page is about frogs as animals. For frogs in stories, see [Frogs in culture](topic:@-topics-9).
```
````

It is drawn in italics, further in than the text, over everything of the
article. Only the first note is drawn there; another is drawn where it is
written.

On a page's article the boxes stand at the right of its text while the
page has room for both side by side, and above its text, under the note,
where it has not (on a phone, or a page narrowed beside another). An
article with Contents too has them at its left, its text in the middle
and its boxes at its right where the page is wide; narrower, the
Contents fold above the text first, then the boxes go above it too,
under the note and over the folded Contents.
Anywhere else (a reply, a question's post) a box is drawn where it is
written. After **Edit**, whoever may change it sees the blocks in the editor
as their text, and the boxes beside it as last saved. A block that cannot be
read is shown as its code with a faint line under it saying what is
wrong. The links in boxes are links like any other: they lead where they
say and count in **Linked from**. An article with none of these looks as
it did.

## Replies

**Reply to the topic**, over the replies (on Talk, for an article),
begins a new chain; **Reply** under any reply answers that one, and is
drawn under it. Replies nest eight deep; one written to a reply at the
eighth level stands beside it. A chain longer than five starts folded
after its third reply, and says how many are under the fold.

```sh
mw topic reply "Why the build is slow" "Linking is most of it."
mw topic show "Why the build is slow"
mw topic reply "Why the build is slow" "Which linker?" --to 42
```

`mw topic show` prints the topic and each reply with the number it was
said at, further in for each level; `--to` takes that number.

## Status: answered, closed, opened again

Not every topic is a question. A wiki's page, or a thread that gathers what
is known, has no use for open or answered, and has **no status**: nothing
is shown beside it, its replies have no **Mark as the answer**, and an
agent asked in it is told none. That is how a topic is made where nothing
is said. Give it one where it is a question to be answered (`open`), or
something never answered and kept up to date (`standing`):

```sh
mw topic make "Which linker" --status open
mw record set @-topics-1 status=open
mw record set @-topics-1 status=
```

`status=` with nothing takes the status away again, as it empties any
field. A topic with no status is answered by nothing until it has one.

A reply is marked as what answered the topic with **Mark as the answer**
under it, and the topic is then answered; **Not the answer** takes that
back and opens it again. **Close**, at the head of its page, closes it,
and it is listed under Closed; **Open again** brings it back.

```sh
mw topic answer "Why the build is slow" 42
mw topic answer "Why the build is slow" --none
mw record set @-topics-1 status=closed
mw record set @-topics-1 status=open
```

A topic that is never answered and is kept up to date (how branches are
named, say) is `standing`; where every page is so, it is better left with
none. Its title, post, summary and status are
changed as any record's are (`mw record set <topic> summary=...`), from
any door alike.

## Moving and following

```sh
mw topic move "Why the build is slow" --project shop
mw topic follow "Why the build is slow"
mw topic follow "Handbook" --with-pages --often daily
mw topic follow "Why the build is slow" --not
```

Where a wiki is (`--workspace` or `--project`, which takes every page
under it along), the page a page is under, and who follows it, are
written by these commands (and the routes under them) and by nothing
else; a page under another is where its wiki is. A topic of a project
that is taken out of the factory goes to the usual workspace, and one of
a workspace that is taken away goes where that workspace's things went.

Whoever follows a page is told in their inbox when it changes: when
somebody replies on it, changes its title, body or summary, marks it
answered, closes it, opens it again, marks it standing or takes its
status away, or moves it
under another page, to the top, or (a wiki) to another workspace or
project. Putting it in another order among the pages beside it tells
nobody. Nobody is told of what they did themselves. A page has one line
in each follower's inbox, which each change raises again (unread, and
counted), saying who changed it last and what that did, and opening the
page, at the reply where that was a reply: ten quick saves are one line
counted ten times. Following a page follows that page alone, not the
pages under it nor the wiki above it, unless it is followed with its pages
(`--with-pages`, or **This page and the pages under it** on its Follow
button): then a change of any page under it, and a page made under it
later, raises its line, which opens the page that changed. Somebody who
follows a page and a page above it is told by the line of the nearest.
How often you are told (at once, hourly, daily or not at all), and
following boards, tables, libraries, channels and projects, are in
[Following](following.md). A follower who is named in a reply
is told by their line of the page, not named besides. Somebody who may
no longer read the topics is told nothing of them, and the lines they
have are not shown while that lasts.

## Who sees what

Topics are the factory's own, not one project's. Every topic is read by
whoever may read records, and written by whoever may write them: a
workspace or a project a topic is in lets nobody in and keeps nobody
out, and no topic can be kept from somebody who may read records yet.
Write nothing in a topic that is not for everybody here.

## An agent in a topic

An agent the factory starts is asked in a topic by being named with `@`
in a reply (`@scout which linker is faster?`). It is given the topic (its
title, how it stands where it has a status, where it is, the summary and
the post) and the chain of replies above the one that named it, and
nothing of any channel. It is asked to give sources as footnotes and to
link the pages it names (*Links and sources*, above).
Its answer is said as a reply under the one that asked it, as the agent's
own: the factory says it there for the agent, on the leave of whoever
asked, so an agent that may not change records itself still answers, and
is given nothing more by it. It is told that its answer is said for it,
and offered only the commands it may run itself: reading the topic whole
where it may read records, and replying elsewhere in it or making a page
under it where it may also change them. Where the agent could not answer, gave no
answer, or its answer could not be said (too long for a reply, say), the
factory says so under the reply that asked it instead.

Typing `@` in a reply box offers the people and agents who can be named;
a person named in a reply is told in their inbox, where they may read
records. A name of somebody is drawn underlined in the reply, and shows
who it is when pointed at or reached by the keyboard.

A person who replies in a chain an agent answered in, and names nobody,
is talking to the agent that spoke last there, as in a channel's thread.
What an agent says asks no other agent: two agents never go on with each
other in a topic. An agent follows no topic, and an agent the factory
does not start is not asked in one.

## From a program

The same as the commands, over HTTP, as whoever the request is signed in
as:

| | |
|---|---|
| `GET /api/topics` | Every topic, with how many replies each has. |
| `POST /api/topics` | Make one: `title`, and `body`, `status` (`open` or `standing`; none where not said), `workspace` or `project`. |
| `GET /api/topics/<topic>` | One whole, by its id or its title, with its replies. |
| `POST /api/topics/<topic>/replies` | Reply: `body`, and `parent`, the number of the reply it answers. |
| `POST /api/topics/<topic>/answer` | `reply`, the number of the reply that answered it, or `null` to open it again; refused where it has no status. |
| `POST /api/topics/<topic>/place` | Move it: `workspace` or `project`. |
| `POST /api/topics/<topic>/follow` | `follows`: `true` or `false`; `pages`, `often`, `at` as `POST /api/following` takes them. |
| `GET /api/following`, `POST /api/following` | Everything you follow; follow one thing (`kind`, `thing`, `follows`, `pages`, `often`, `at`). |
| `GET /api/inbox/<line>/changes` | The changes a line of something followed told of. |
| `GET /api/topics/<topic>/revisions` | Each revision of what it says, oldest first, with who made it (`by`, and `who` by name) and the blueprint it came through (`through`). |
| `GET /api/topics/<topic>/linked` | The pages that link to it: each its `id`, `title`, `wiki` (its `id` and `title`, or null for a wiki) and `series`: the title of the series box through which alone it links, or null where it links from anywhere else. |
| `PATCH /api/records/<id>` | Its title, body, summary or status, under `fields`, as any record's. |

What a blueprint hears of a topic is `topic.revised`, `topic.answered`,
`topic.closed`, `topic.opened`, `topic.standing`, `topic.untracked` (its
status taken away), `topic.placed` and `topic.followed`, beside the `record.*` events of its row
(`mw events --kinds topic.answered` says what one carries). `mw help
topic` says all of this in short.
