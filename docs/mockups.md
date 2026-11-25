<!-- public -->

# Mockups

A project's mockups: pages made to try an idea with. A mockup is a folder
of plain web files in the project, shown in the client small and live
among the others, opened to be looked at properly at the width of a phone,
a tablet or the room there is, alone or beside another, and commented on. An
agent can be asked for one.

Nothing is added to a page for it to be a mockup, and nothing but the
project holds it: it is files, kept and carried as the project's files are.

## Giving a project its mockups

A project has none until a blueprint makes a folder of it theirs.
`std/mockups` does:

```sh
mw apply std/mockups --project shop
mw apply std/mockups --project shop --param folder=design/mockups
```

The folder is `mockups` unless another is said. Under the project, beside
its Board and its Docs, there is then **Mockups**.

The blueprint is this much, and a project that wants other stages, or
more kept about each mockup, declares its own the same way:

```yaml
steps:
  mockups:
    do: collection.ensure
    with:
      name: mockups
      store: mockups            # the folders of a folder that each hold a page
      at: "${params.folder}"
      fields:
        title: { type: text, required: true }
        path: { type: text }
        of: { type: record, of: mockups }
        status: { type: choice, of: [draft, review, chosen, dropped] }
        notes: { type: text }
```

## What a mockup is

A folder in the mockups' folder that holds a file called `index.html`:

```
mockups/
  pricing/
    index.html        the page
    style.css         whatever the page uses is beside it,
    app.js            and in folders under it
    art/mark.svg
    mockup.yaml       what is kept about it, where anything is
```

It is a mockup as soon as its page is there, whoever wrote it: by hand, in
the client's editor, by an agent, or arriving with a change to the
project. A folder with no `index.html` is not one, and neither is a file
by itself or a folder whose name begins with a dot.

It is known by where it is: `shop-mockups:pricing` is `mockups/pricing/`.
So it is the same mockup on every device that has the same files, and a
folder that is moved or renamed is another mockup, with a history of its
own.

What is kept about one beyond its files is in `mockup.yaml`, a line a
thing. The file is there only for a mockup that has something in it, and
lines that are nobody's field are left as they are:

```yaml
title: Pricing, bold
of: pricing
status: review
notes: The middle plan stands out more.
```

- **`title`**: what it is called. Where it says nothing, it is called
  what its page's `<title>` says, or what its folder is called.
- **`of`**: the mockup this is a variant of, by that one's folder.
- **`status`**: how far it has got. One that says nothing is at the
  first of the stages there are (`draft`).
- **`notes`**: anything noted of it.

## Variants

A variant is a mockup like any other that says which one it is another
way of doing (`of: pricing`). In the gallery it is set beside what it
varies, and a variant of a variant with them.

Made from the client (**Variant**, on one that is open) or by
`record.add` with `of`, it begins as a copy of everything in the other's
folder, in a folder of its own, to be changed. A folder that is there is
never written over. Made by hand or by an agent, it is a folder with a
page and the one line in `mockup.yaml`.

## In the client

- **The gallery**: every mockup as it is now, drawn small, with what it is
  called and how far it has got; variants beside what they vary. A file
  of one changed by anything is drawn again by itself.
- **One opened**: at the width of a phone (390), a tablet (820) or the
  room there is; **Beside** puts another next to it at the same width;
  how far it has got is changed there; **Variant** makes one of it;
  **Edit** opens its page in the editor; **Comments** shows what has
  been said on it, with a line to add a comment, and after that what has
  happened to it. A mockup's own page is its author's, and nothing is
  drawn into it: its comments are beside it and never on it.
- **New mockup** begins one from a plain page.

What is made, changed and said through the factory is a mockup's history
(`{actor} made it`, `{actor} changed its status`, `{actor} said: ...`).
What another program does to its files is not: nobody saw who did it. A
mockup can be watched from a channel as anything with a history can
(`std/watch`).

From a terminal, or a blueprint, a mockup is a record like any other:

```sh
mw record list shop mockups
mw record add shop mockups title="Checkout"
mw record set shop-mockups:checkout status=review
mw record comment shop-mockups:checkout The button is too small.
```

## Asking an agent for one

Every agent is told where a mockup goes and what makes one. Asked in the
project's channel to make a mockup of a pricing page, it writes
`mockups/pricing/index.html` and what the page uses beside it, and the
gallery shows it. Asked for another way of doing it, it writes a second
folder whose `mockup.yaml` says `of: pricing`.

## Its own address

`/mockups/<id>` (`/mockups/shop-mockups:pricing`) is a mockup by itself,
for whoever is signed in to the client in that browser and may read the
project's records: on a phone, say, at the address the daemon is reached
at there. **Its address**, on one that is open, opens it.

## What a mockup can do, and what it cannot

A mockup's page is somebody's, or an agent's, and its scripts run: a
prototype is no use otherwise. So it is shown in a place of its own, in
the client's frame and at its own address alike.

- **It is served through a way in.** Whoever may read the project's
  records is given an address that holds a name nobody could guess
  (`/mockups/in/<name>/`). That name lets its holder read the files of
  that one mockup's folder, for an hour, and nothing else. It is no
  credential: it is not a session, opens no route of the API, and what it
  reads it cannot change. A daemon that starts again has none; whoever
  shows a mockup asks again.
- **It belongs to nowhere.** Everything served there is sent with a
  policy that makes the browser treat it as a stranger to this daemon: it
  has no cookies and no storage, so it is never given the session; what
  it asks of the daemon's API is refused before it is sent; and it may
  fetch only from its own way in, so it can call nothing else either.
  It cannot reach the page around it.
- **It reads only its own folder.** Whatever is asked for is held inside
  it: nothing that goes up, nothing from the top of the disk, and no link
  in the folder that leads out of it.

What works in one, then: scripts, styles, pictures, fonts and data of its
own folder; forms and dialogs (a form is sent nowhere); links between its
own pages. What does not: storage in the browser (`localStorage`,
cookies), anything fetched from another site, and anything of the daemon.

Whoever holds a way in's address can read that mockup's files until it
runs out. It is told to nobody the page sends its reader on to, but it is
in the browser's history: on a machine that is shared, close what was
opened.

## Environments

Nothing here is particular to one. A factory serves the mockups of its
own projects, to those signed in to it; a mockup is tried in another
environment by being there, which is done as files are: through the
project's repository. Carrying one from an environment to another by
itself is not built.

## What is not built

Measuring two variants against each other with visitors; publishing a
mockup to the web; carrying one between environments; a visual editor;
following a renamed folder; taking one away from the client (it is a
record: `mw record remove`).
