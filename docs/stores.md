<!-- public -->

# Stores

A table's records are kept where its blueprint says: in this installation's
own database (`local`), as files in the project (`files`), or by a **store
service** (`service`). This page is about the third.

A store service is a process that keeps records and answers a small
protocol. A blueprint makes it and hosts it, the way a blueprint hosts
anything; the table is then declared with the address it answers at. Mythwright
knows no database by name. What is behind the address (a file, a server,
somebody's hosted database) is the business of the blueprint that made it.

`mw help stores` prints this page.

## Keeping a board in one

`std/store` makes a database and hosts it. `std/board` takes the address it
gives out. A project's own blueprint does both:

```yaml
blueprint: tracker
description: A board whose cards are kept in a database of the project's own.
permissions: [shell, fleet, blueprints, projects, board]

steps:
  database:
    call: std/store
    with: { name: cards }

  board:
    call: std/board
    with:
      store: service
      database: "${steps.database.out.url}"
      columns: [backlog, doing, done]

outputs:
  database: ${ steps.database.out.url }
```

```
mw apply blueprints/tracker.bp --project shop
```

The first step makes `.data/cards.db` in the project, if it is not there,
and hosts the program that serves it; the second declares the board and
says its cards are kept there. Applied again, both find what they made: the
database is left as it is, with the cards in it.

The board is used as any board is. In the client its page says, beside the
count of its cards, `kept in a database at 127.0.0.1:<port>`.

Or by hand, one after the other:

```
mw apply std/store --project shop                 # its outputs say "url":"http://127.0.0.1:<port>"
mw apply std/board --project shop --param store=service --param database=http://127.0.0.1:<port>
```

Any table is kept the same way. `collection.ensure` takes `store: service`
and `at: <address>`:

```yaml
steps:
  notes:
    do: collection.ensure
    with:
      name: notes
      store: service
      at: "${steps.database.out.url}"
      fields:
        title: { type: text, required: true }
```

## What is settled, and what is not

- **Where a table is kept is settled when it is made.** A board that is
  `local` is not moved to a service by declaring it again, or the other way
  round: that is refused, in a sentence that says so.
- **The address may be said again.** It is not where the table is kept but
  how the same store is reached, and a store that was stopped and started
  may answer somewhere else. Declaring the table again with another `at`
  changes the address and nothing else. No records are carried: if what
  answers at the new address is another database, the table is what that
  database holds. Declaring it again with no `at` leaves the address as it
  was.
- **The store has to be answering when the table is declared.** Applying
  `std/board` against an address where nothing answers fails there, and
  not later at the first card.
- **When the store stops answering**, reading or changing the table fails
  with a sentence that says where the store was expected and to start, or
  look at, the blueprint that hosts it. Nothing is kept anywhere else in
  the meantime, and the table works again when the store is back.
- **A record's history is not in the store.** What happened to a record
  (`history` in the table's blueprint) is kept in this installation's own
  database whatever the store, and so is what the table is: its fields and
  its views.
- **Stopping the blueprint that hosts a store stops the service**, and the
  database stays where it is. Nothing takes a database away.

## `std/store`

The store that ships. Its database is one SQLite file, and the program that
serves it is `mw` itself.

| Param | Meaning |
|---|---|
| `name` | The store's name (default `store`). With the project's, the name of its address on this device, `shop-store`, and of the service behind it: `shop-store-g1`, then `-g2` when it is replaced. |
| `file` | The database file, relative to the project. Left out, `.data/<name>.db`, a folder that is not listed among the project's files. |
| `port` | The port of its address. `0`, the default, picks a free one and keeps it until the store is stopped. |
| `ready_timeout` | How long it has to start answering (default `60s`). |

It gives out `url` (the address a table is kept at), `file`, `service` and
`port`. Its steps:

1. `create` runs `mw store init --db <file>`, which makes the file and its
   table if they are not there and leaves them alone if they are.
2. `host` calls `std/slot` with `mw store serve --db <file>` as its
   service, which listens at the port the service is given (`PORT`), and
   `GET /health` as its health check.
   So the store is started again if it dies, has an address that does not
   change, and when `std/store` is applied again with something changed the
   new one is started, waited for, and only then put in the old one's
   place.

The two commands are ordinary programs and can be run by hand:

```
mw store init --db FILE                        make the database, if it is not there
mw store serve --db FILE [--listen ADDRESS]    serve it; prints the address it answers at
```

With no `--listen`, `serve` listens on this device at the port named in
`PORT`, which is how a hosted service is told its port, and at a free one
when there is none.

Neither talks to the daemon or needs anyone to be signed in.

### How a blueprint runs `mw`

A blueprint cannot write `mw`: the command may be installed under that
name, or not on the `PATH` at all, or be a build in a checkout. So a
`shell` step or a service says `cli:` and the arguments, and what is run is
the program the daemon itself is, which is also the command line, with no
shell between:

```yaml
create:
  do: shell
  with: { cli: [store, init, --db, .data/cards.db] }
```

A script of your own finds the same program as `FACTORY_CLI` in its
environment (`"$FACTORY_CLI" store init --db cards.db`), which everything
a blueprint runs has.

It is the program and nothing more. No credential comes with it, because a
blueprint's commands are never handed the daemon's; a command that talks to
the daemon still has to sign in as someone.

## The protocol

HTTP, with JSON both ways. A store answers four things.

| Request | Answer |
|---|---|
| `GET /health` | `200` when it can serve. |
| `GET /records?project=<p>&collection=<c>` | `200` and `{ "records": [ ... ] }`: every record of that table. |
| `PUT /records/<id>` with a record as its body | `2xx`. The record is stored: new, or in place of the one with that id. |
| `DELETE /records/<id>` | `2xx`. The record is gone. Removing one that is not there is not an error. |

A record is exactly what `mw` keeps of one itself:

```json
{
  "id": "shop-3",
  "project": "shop",
  "collection": "board",
  "number": 3,
  "data": { "title": "Fix the queue", "status": "doing", "labels": ["bug"] },
  "position": 2.0,
  "created_by": "user:acct_1f0c",
  "created": 1794300000000,
  "updated": 1794300360000
}
```

`data` holds the table's fields and is the table's to define: a store keeps
it as it is given and does not look inside. `number` counts the records of
one table of one project from 1; `position` orders them where they are
shown; the two times are milliseconds since 1970. `id` is unique across
everything a store keeps, and is the same in the path and in the body.

A store that will not do something says why, as `{ "error": "<a sentence>" }`
under a status that fits: `400` for a request it cannot read, `404` for a
path it does not have, `405` for something it does not do, `500` or `503`
when what it keeps records in fails it.
The sentence is shown to whoever was using the table.

That is the whole of it, on purpose. A store keeps and returns; it does not
check a record against the table's fields, number records, or order them
(the listing is put in order of `number` by whoever asked). Those are done
by `mw` before a record is sent, in the same way whatever the store.

`mw` asks a store directly at its address, over plain HTTP, and gives it
ten seconds to answer. Each request is measured as `store.request`, by
`op` (`list`, `put`, `remove`), `project` and `collection`:
`mw metrics store.request --by op`.

## Another database

Nothing above is about SQLite except `std/store`. Another database is
another blueprint that gives out a `url`, and a program at that address
that speaks the protocol. Neither is built here for any other database; this
is how one would be.

For a database that is a server of its own, PostgreSQL say, the blueprint
has more steps and the same shape:

```yaml
blueprint: pg-store
description: Host a PostgreSQL server and a store service in front of it.
permissions: [shell, fleet, blueprints]

params:
  name: { type: string, default: pg }
  data: { type: string, default: .data/pg }
  port: { type: int, default: 5432 }

steps:
  init:
    description: The server's data directory, made once.
    do: shell
    with: { run: '[ -d "$DATA/base" ] || initdb -D "$DATA"', env: { DATA: "${params.data}" } }

  server:
    call: std/slot
    as: server
    with:
      name: "${params.name}-server"
      expose: false
      service:
        run: 'exec postgres -D "$DATA" -p "$PORT"'
        port: ${params.port}
        env: { DATA: "${params.data}" }
        health: { cmd: 'pg_isready -p "$PORT"' }

  schema:
    description: The database and the table the records go in.
    do: shell
    with: { run: 'psql -p "$PGPORT" -d postgres -f blueprints/records.sql', env: { PGPORT: "${params.port}" } }

  adapter:
    description: The program that speaks the store protocol and keeps the records in the server.
    call: std/slot
    as: adapter
    with:
      name: "${params.name}"
      service:
        run: 'exec ./bin/records-adapter --listen "127.0.0.1:$PORT"'
        env: { DATABASE_URL: "postgres://127.0.0.1:${params.port}/records" }
        health: { http: /health }

outputs:
  url: "http://${steps.adapter.out.address}"
```

The adapter is the only thing to write: a small HTTP server, in any
language, with the four answers above and three statements behind them (a
select by `project` and `collection`, an upsert by `id`, a delete by `id`).
`crates/served/src/store.rs` is the one for SQLite, in about two hundred
lines, and is the reference for it. A board kept there is `std/board` with
`store: service` and `database:` that blueprint's `url`, exactly as above.

A database that is somebody's hosted service needs no server step: only
the adapter, given where the service is and how to sign in to it.

## What is not built

- A store for any database but SQLite.
- Moving a table from one place to another: `local` to a service, or one
  store to another with its records.
- Reading the `url` of a store that some other blueprint started. A
  blueprint can use the outputs of what it called; the address of a store
  made elsewhere is given by hand, as a param.
- Signing in to a store. The protocol carries no credential: a store
  listens on this machine, and one that is reached over a network is to be
  put behind something that does.
- Being told by a store that its records changed. A record changed in the
  database by something else is seen the next time the table is read, and
  nothing says that it happened.
