# Notion

Keep the cards of a project that are **linked** with a page of a
[Notion](https://www.notion.so) database in step with it, a ticket at a
time. It is the GitHub package's `github/issues` done for a Notion
database, and works the same way: read *Issues: tickets kept in step* in
`extensions/github/README.md` beside this one for the whole of it, and
this page for where Notion differs. How such a package is written is in
*Keeping a tracker in step* in `docs/extensions.md`.

The package is called `notion-pages`, not after the service alone: the
application's own client says in its notes that it follows how Notion
lays things out, and a package's name may be written nowhere outside its
folder but in Markdown.

```sh
mw secret set notion-key < integration-secret.txt
mw extension add notion-pages
mw apply notion-pages/pages --project shop --param database=0123456789abcdef0123456789abcdef --param key=notion-key
mw record add shop links card=shop-12                                      # a page is made for the card
mw record add shop links page=89abcdef-0123-4567-89ab-cdef01234567          # a card is made for the page
```

A database's page is a ticket of four of its properties, each said by
its name where it is applied:

| Of the page | Of the card | Param | Unless said |
|---|---|---|---|
| its title | its title | `title_property` | `Name` |
| a text property | its body | `body_property` | `Description` |
| a checkbox, ticked | being in the board's last column | `done_property` | `Done` |
| a text property | the card a page was made for, by which it is found again | `mark_property` | `Card` |

Each has to be in the database, of that kind. What is written on the page
itself, under its properties, is not a card's body and is not read: it is
blocks, which this does not turn into text (*What is not built*, below).

Of a linked ticket the title and the body cross both ways as `takes_in`
says; a card moved to the last column has its page ticked done and one
moved out of it has the tick taken off, and a page ticked or unticked
moves its card to the last column or the first (`closed` and `reopened`
say others). What is said on the page is said on the card once, with who
said it, and never as public; what is said on the card stays here unless
whoever says it marks it public and holds `extensions:notion-pages`, and
is then said on the page once.

A page is linked by a row of `links`, a card by a label (`out_label`) as
on GitHub, and a page by a tick in a checkbox of the database
(`in_property`, `Factory` or any name): a page ticked there has a card made
for it the next time Notion is asked what changed.

## The program, and why

Notion has no command line of its own. Its actions run **curl** (8.3 or
later), asking Notion's web API, at the version written with each request
(`Notion-Version: 2022-06-28`), so that a version Notion brings later
changes nothing here until this package is changed for it. The
integration's secret is handed to curl in the environment (`NOTION_KEY`,
from the secret `key` names), and curl writes it into the request's
header itself (`--variable`, `--expand-header`, which came with 8.3): it
is in no argument, and what curl prints is written down with the secret's
name in its place.

## What it needs

- curl 8.3 or later on the daemon's device.
- An [internal integration](https://www.notion.so/my-integrations) of the
  workspace, with leave to read, update and insert content and to read and
  insert comments, and its secret kept by the factory (`key`).
- The database shared with the integration (*Connections*, in the
  database's menu), and its id: the 32 letters and digits in its address.
- The four properties above in the database.

Applying it says at once if the secret is not taken or the integration
may not see the database.

## Who can write what an agent works from

Only the people of the workspace, and guests somebody shares the
database with, can change one of its pages or say something on it. Notion
says nothing this can read of which of those somebody is, so whose
changes are taken is everyone's, unless said, or nobody's:

| `takes_in` | A change of a linked page's title or body | What is said on the page |
|---|---|---|
| `everyone`, unless said | Comes into the card. | Is said on the card, and may answer what an agent asked on it. |
| `nobody` | Is not taken and not written over. | Is said on the card, and answers nothing. |

`trusted` is refused, saying why. Whoever shares a database with guests
who are not to steer the fleet's agents says `nobody`.

## Hearing what changed in Notion

**On a timer**, where Notion cannot reach this factory (`--param
timer=true`): it asks once a minute which ten pages of the database were
edited last, and reads each of those that is linked. Notion says when a
page was edited to the minute, so a page is not passed over for having
been read in the same minute: each is read again as it is.
`mw emit notion.look project=shop` has it ask at once; it looks of its
own accord when it is applied and each time the daemon comes back.

**As it happens**, where Notion can reach the factory at an address: a
webhook subscription of the integration's (its settings, *Webhooks*) for
page and comment events, sent to `https://<where this factory answers>/in/notion`,
and a caller that takes what it sends only where it is signed with the
subscription's verification token:

```sh
mw secret set notion-signing < verification-token.txt
mw caller set notion --topic notion.called --secret notion-signing --header X-Notion-Signature --prefix sha256=
```

Notion says it signs what it sends so, an HMAC-SHA256 of the body in
`X-Notion-Signature` after `sha256=`; that has not been seen against
the real service. Of what a call says only which page it is about and
which database that is in are taken, and the page is read again as it is.
The database's id is held against the one this was told as it is written:
say it with its dashes or without as the call writes it.

| Param | |
|---|---|
| `database` | The id of the database. Needed. |
| `key` | The secret of the integration. Needed. |
| `title_property`, `body_property`, `done_property`, `mark_property` | The properties, as above. |
| `in_property` | A checkbox that links a page when it is ticked. Left out, only a row of `links` links a page. |
| `channel` | Where it says what it did or could not do. |
| `out_label` | A label that links a card. |
| `closed`, `reopened` | The columns of a card whose page is ticked done, and of one whose page is not. |
| `prefer` | Which side is taken where both changed: `here`, unless `there` is said. |
| `takes_in` | As above: `everyone`, unless `nobody` is said. |
| `timer` | `true` to ask Notion once a minute what changed. |

It needs `extensions:notion-pages`, `board`, `records`, `projects`,
`channels:write`, `blueprints`, `emit` and `secrets`.

## What is not held, or not built

- **What is written on a page under its properties** is not a card's body
  and is not read or written: Notion keeps it as blocks, which a
  package's expressions cannot turn into text. A card's body is a text
  property of the database.
- **A text longer than 2000 letters** is refused, in a sentence, where it
  would go to Notion: Notion takes a text property and a comment in pieces
  of at most that, and this sends one piece.
- **What is said from here** carries a mark at its end, `[said <n>]`,
  which Notion shows: Notion has nothing to hide it in.
- **Who said something** is the name Notion resolves for the comment
  (`display_name`), read from what its API answers and not yet seen against
  the real service.
- **A page moved to the trash** in Notion is not said to be anything: its
  card stays as it is.
- **Features are not built**: a feature here is no page there; a
  feature's plan comes to cards, each linked as any card is.
- **A daemon stopped as a page is being made** is held as Jira's is: the
  page is looked for by the card in its mark when it is back, and made
  only where it is not there; one Notion made in the instant before the
  daemon went and does not yet list may be made twice.
- **The package has not been run against the real Notion.** What curl
  prints was written into the stand-in from Notion's documentation of its
  API. The first trial is to apply it to a database of a workspace with
  an integration of its own, link one card and one page, and see that each
  comes across whole.

## The examples

`examples/` holds a stand-in for curl asking Notion (`stand-in.sh`) and
cases, played by the application's tests against a daemon that can find
the stand-in under the program's name and nothing else.
