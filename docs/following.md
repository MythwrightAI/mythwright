<!-- public -->

# Following

Following something tells you in your inbox when it changes. You can
follow a page of a wiki (alone, or with every page under it), a project's
board, one of its tables, its library, a channel, or a whole project, and
say for each how often you are told: at once, once an hour, once a day,
or not at all.

## Following something

In the client, the head of each of those pages has a **Follow** button.
Press it to follow; press it again for how often, and to stop. On a page of
a wiki it offers **This page** and **This page and the pages under it**.
**Settings → Following** lists everything you follow, each with its kind
and how often, and **Unfollow**.

From the command line:

```sh
mw follow page:"Frogs: a field guide" --with-pages
mw follow board:shop --often hourly
mw follow table:shop/stock --often daily --at 08:30
mw follow library:shop/docs
mw follow channel:general --often off
mw follow project:shop
mw following
mw unfollow board:shop
```

`mw topic follow <page> --with-pages` follows a page the same way.
Following what you follow already changes only what you say, and keeps the
rest.

## What tells you

- **A page** tells of a reply on it, a change of its title, body or
  summary, of how it stands, and its being moved. Followed with its pages,
  the same of every page under it, and a page made under it, however long
  after you followed it.
- **A board, a table or a library** tells of a row (a card, a document)
  added, changed or taken away, and something said on one.
- **A channel** tells of each message.
- **A project** tells of all of those in its tables, its library and its
  channels, and of its being renamed or moved.

Nothing you do yourself is told you, by any key of yours either.

## How often

- **At once** (where nothing is said): one line in your inbox for the
  thing, raised by each change and saying what changed since you last read
  it ("Shop board: 3 cards changed"). A page's line says who changed it
  last and what that was, and opens the page there.
- **Hourly**: once the hour is over, one line for the thing that sums up
  everything that changed in it during the hour ("Frogs: a field guide: 6
  pages changed, 3 new replies, 1 page added"). An hour in which nothing
  changed is no line.
- **Daily**: the same once a day, at the time you say (09:00 where you say
  none), in the time zone your clock setting keeps (Settings, or the
  zone your browser said first), else UTC.
- **Off**: followed, and told nothing. It is there to turn back on.

A line of a digest, or of a board, table, library, channel or project
told at once, opens a list of the changes it told of, each with what it
changed and who.

A digest that is due while the factory is not running is told as it
starts again, once.

## Who may

You follow only what you may read: following anything else is refused.
You are told only what you may read when you are told: a change of a
table kept private from you, or said in a channel kept to members you are
not among, tells you nothing, and the lines you have of something you no
longer read are not shown while that lasts. An agent follows nothing: it is
told by being named.
