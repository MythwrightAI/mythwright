<!-- public -->

# Example: a team wiki kept current by agents

Say your team keeps a handbook for a service called `shop`: how it is
deployed, how to roll a deploy back, how branches are named. It is right
the week it is written and drifts after that, because nobody's job is to
read every merge and ask which page it makes wrong. This is how Mythwright
keeps it current: the handbook is a wiki of topics, each merge has an agent
asked to bring the pages it touches up to date, and a reader who wants
something added says so in a reply, which becomes a card on the board.

Each part is a blueprint or a row of a table, and you can read and change
every one of them.

## 1. The wiki

A topic is a page with a title, a post, a summary of what is known now kept
at its top, and replies under it ([topics.md](topics.md)). A topic may hold
topics under it, to any depth, and one with nothing above it is a wiki.
Make the handbook a wiki of the project's:

```sh
mw topic make "Handbook" --project shop --body "How we build, ship and run the shop."
mw topic make "Deploying" --under "Handbook" --body "How the shop is deployed."
mw topic make "Rolling back" --under "Deploying"
mw topic make "Naming branches" --under "Handbook"
mw topic show "Handbook"
```

In the client it is under **Topics** on the rail at the left, and each page
opens as a page of its own, its address its title. The post and the summary
are written in place, as a document is; each change is kept as a revision
of the page. A page may be marked `standing`, kept up to date and never
answered, which is what most handbook pages are:

```sh
mw topic list
mw record set @-topics-2 status=standing
```

A topic is read by whoever may read records in the factory: write nothing
in the handbook that is not for everybody here.

## 2. Somebody to keep it

Add a role to the project's fleet whose agents keep the handbook and do
nothing else. It is a row of the project's `agents`:

```sh
mw record add shop agents title=writer role=writer count=1 permission=read told="You keep the team's handbook, a wiki of topics, true to the code. You change pages and nothing else."
```

`permission=read` lets its program read the project's files and change
none of them; it changes pages with the factory's own command, which is
held by what its agent may ask of the factory. No stage gives this role
cards, so it is asked only by what is below.

## 3. Ask it after each merge

A project's pipelines are its own blueprints under `pipelines/`, and the
table `triggers` says what runs each (`mw project create shop --with
pipelines` gives a new project that table). This one is run when `main`
moves: it reads what the merge changed and asks the writer to bring the
pages up to date.

```yaml
blueprint: pipelines/wiki
description: Ask the writer to bring the handbook up to date with what was merged.
permissions: [shell, channels:write, agents:write]

params:
  branch: { type: string, description: "The branch that moved." }
  commit: { type: string, description: "Where it moved to." }

steps:
  changed:
    description: Read what the merge changed.
    do: shell
    with: { cmd: [git, show, --stat, --format=%s, "${params.commit}"] }

  channel:
    do: channel.ensure
    with: { name: "${instance.project}", project: "${instance.project}" }

  update:
    description: Ask the writer to update the pages that describe it.
    do: agent.task
    with:
      role: writer
      channel: "${steps.channel.out.id}"
      prompt: |
        ${params.branch} moved to ${params.commit}, which changed:

        ${steps.changed.out.stdout}

        Read the pages of the Handbook wiki (`mw topic show Handbook`, and each
        page under it) and bring up to date any that this makes wrong, with
        `mw record set <page> body=...`. Change nothing else. Say which pages
        you changed and why, or that none needed it.
```

Save it, and add the row that runs it:

```sh
mw check wiki.bp
mw file write shop @blueprints/pipelines/wiki.bp --from wiki.bp
mw record add shop triggers title=wiki pipeline=pipelines/wiki on=branch branch=main
```

A row that says `on=branch` has the project's branches followed: when
`main` moves in the project's repository, by a merge of the fleet's or
a commit of anybody's, the pipeline is run and handed the branch and the commit it moved to. The
writer says in the project's channel which pages it changed and why, and
each change of a page is kept as a revision, so one you disagree with is
seen, and can be put back.

Which pages a merge touches is the writer's to work out from what changed
and what the pages say. To make that easier, keep one page for each part
of the service, and say at the top of its post which folders it is about.

## 4. Replies that ask for something become cards

Anybody reading a page can reply under it. A reply, on any topic, that
begins with `request:` is somebody asking for the page, or the service, to change: a
small blueprint of the project's own hears it and adds a card.

```yaml
blueprint: wiki-requests
description: Add a card for each reply on a wiki page that begins with "request:".
permissions: [board:write]

on:
  asked:
    event: record.commented
    for: { collection: topics }
    when: ${ starts_with(lower(event.data.body), "request:") }
    do: board.add
    with:
      title: "${event.data.body}"
      body: "Asked on the wiki page ${event.data.id}."
      labels: [wiki]
```

```sh
mw file write shop @blueprints/wiki-requests.bp --from wiki-requests.bp
mw apply wiki-requests --project shop
mw topic reply "Rolling back" "request: say how long a rollback takes"
mw board shop
```

The card lands in the board's first column, labelled `wiki`, and names the
page it was asked on. From there it is a card like any other: put it in
`ready` and the fleet works it, or answer it yourself. A reply that says
something without `request:` is only a reply.

The word is the team's to choose, and so is what a reply has to say to be
a card: the `when` line is yours to change.

## 5. Readers, and agents, in the pages

- A reply to a page that names an agent with `@` asks that agent: it is
  given the page and the replies above, and answers under the reply that
  asked. `@shop-writer-1 is this page still right after Tuesday's change?`
  is a question the writer can answer.
- A reply can be marked as what answered a page, and a page closed or
  opened again (`mw topic answer`, `mw record set @-topics-2
  status=closed`).
- `mw topic move` reorders the tree: a page goes under another, before
  one beside it, or to the top as a wiki of its own.

## What it does not do

- It does not read your code and decide what the handbook should say. The
  writer agent does, with a real agent program behind it: out of the box
  agents run on a small built-in assistant that cannot do this work
  ([extensions.md](extensions.md) says how to add one).
- It does not keep any page from anybody who may read records.
- Publishing a wiki to the web is planned, as a hosted service. In this
  version a wiki is read in the factory.
