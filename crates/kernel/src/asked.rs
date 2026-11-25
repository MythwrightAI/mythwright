//! What the actions of a project's tables, records, board and files are
//! shown by, in the Reference and the editor's card: each a whole
//! blueprint that asks the action, short and plain, held to checking with
//! no warning against every action there is (the API suite's
//! `examples.rs`). Plain rows here, which the studio's actions name, since
//! the studio is near its ceiling of lines.

/// `board.add`.
pub const BOARD_ADD: &str = "\
blueprint: card-for-a-failure
permissions: [board:write]

on:
  failed:
    event: run.failed
    do: board.add
    with: { title: \"${event.data.blueprint} failed\", body: \"${event.data.error}\", labels: [broken] }
";

/// `board.move`.
pub const BOARD_MOVE: &str = "\
blueprint: ready-when-asked
permissions: [board:write]

params:
  card: { type: string, description: The card to make ready. }

steps:
  ready:
    do: board.move
    with: { id: \"${params.card}\", to: ready, note: Asked for by the release. }
";

/// `board.press`.
pub const BOARD_PRESS: &str = "\
blueprint: signed-when-it-passes
permissions: [board:write]

params:
  card: { type: string, description: The card whose checks passed. }

steps:
  signed:
    do: board.press
    with: { id: \"${params.card}\", action: signed }
";

/// `board.update`.
pub const BOARD_UPDATE: &str = "\
blueprint: label-what-is-late
permissions: [board:write]

params:
  card: { type: string, description: The card that is late. }

steps:
  labelled:
    do: board.update
    with: { id: \"${params.card}\", labels: [late] }
";

/// `board.list`.
pub const BOARD_LIST: &str = "\
blueprint: what-is-ready
permissions: [board:read]

steps:
  ready:
    do: board.list
    with: { status: ready }

  said:
    do: log
    with: { message: \"Ready: ${steps.ready.out.count}\" }
";

/// `board.get`.
pub const BOARD_GET: &str = "\
blueprint: one-card-read
permissions: [board:read]

params:
  card: { type: string, description: The card to read. }

steps:
  card:
    do: board.get
    with: { id: \"${params.card}\" }

  said:
    do: log
    with: { message: \"${steps.card.out.title} is in ${steps.card.out.status}\" }
";

/// `collection.ensure`.
pub const COLLECTION_ENSURE: &str = "\
blueprint: visitors
permissions: [projects:write]

steps:
  table:
    do: collection.ensure
    with:
      name: visitors
      fields: { name: { type: text, required: true }, came: { type: date } }
";

/// `record.add`.
pub const RECORD_ADD: &str = "\
blueprint: visitor-kept
permissions: [records:write]

on:
  seen:
    event: visitor.seen
    outside: true
    do: record.add
    with: { collection: visitors, fields: { name: \"${event.data.name}\" } }
";

/// `record.keep`.
pub const RECORD_KEEP: &str = "\
blueprint: visitor-counted
permissions: [records:write]

on:
  seen:
    event: visitor.seen
    outside: true
    do: record.keep
    with:
      collection: visitors
      number: ${event.data.number}
      fields: { name: \"${event.data.name}\" }
";

/// `record.update`.
pub const RECORD_UPDATE: &str = "\
blueprint: visitor-back
permissions: [records:write]

params:
  visitor: { type: string, description: The record of who came back. }

steps:
  back:
    do: record.update
    with: { id: \"${params.visitor}\", fields: { came: \"2026-11-20\" } }
";

/// `record.get`.
pub const RECORD_SAID: &str = "\
blueprint: retold
permissions: [records:read, inbox:write]

params:
  note: { type: string, description: The record whose comments are passed on. }

steps:
  read:
    do: record.said
    with: { id: \"${params.note}\" }

  told:
    when: ${ steps.read.out.count > 0 }
    do: inbox.tell
    with: { to: applier, says: \"${steps.read.out.text}\" }
";
pub const RECORD_GET: &str = "\
blueprint: visitor-read
permissions: [records:read]

params:
  visitor: { type: string, description: The record to read. }

steps:
  read:
    do: record.get
    with: { id: \"${params.visitor}\" }

  said:
    do: log
    with: { message: \"${steps.read.out.data.name}\" }
";

/// `record.list`.
pub const RECORD_LIST: &str = "\
blueprint: visitors-counted
permissions: [records:read]

steps:
  listed:
    do: record.list
    with: { collection: visitors }

  said:
    do: log
    with: { message: \"${steps.listed.out.count} have come.\" }
";

/// `record.comment`.
pub const RECORD_COMMENT: &str = "\
blueprint: said-on-a-card

params:
  card: { type: string, description: The card to say something on. }

steps:
  said:
    do: record.comment
    with: { id: \"${params.card}\", body: The tests pass on this one. }
";

/// `record.ask`.
pub const RECORD_ASK: &str = "\
blueprint: asked-on-a-card

params:
  card: { type: string, description: The card to ask on. }
  question: { type: string, description: What is asked. }

steps:
  asked:
    do: record.ask
    with: { id: \"${params.card}\", says: \"${params.question}\", to: [applier] }
";

/// `record.attach`.
pub const RECORD_ATTACH: &str = "\
blueprint: shown-on-the-card
permissions: [board:write, projects:read]

params:
  card: { type: string, description: The card the picture shows something of. }

steps:
  shown:
    do: record.attach
    with: { id: \"${params.card}\", path: shots/after.png, name: after.png }
";

/// `record.remove`.
pub const RECORD_REMOVE: &str = "\
blueprint: visitor-forgotten
permissions: [records:write]

params:
  visitor: { type: string, description: The record to take away. }

steps:
  gone:
    do: record.remove
    with: { id: \"${params.visitor}\" }
";

/// `rows.read`.
pub const ROWS_READ: &str = "\
blueprint: open-orders
permissions: [records:read]

steps:
  open:
    do: rows.read
    with: { collection: orders, saying: { status: open } }

  said:
    when: ${ steps.open.out.those[0] != null }
    do: log
    with: { message: \"The first open order is ${ steps.open.out.those[0] }.\" }
";

/// `rows.start`.
pub const ROWS_START: &str = "\
blueprint: first-rooms
permissions: [records:write]

steps:
  rooms:
    do: rows.start
    with: { collection: rooms, rows: [{ name: kitchen }, { name: hall }] }
";

/// `project.create`.
pub const PROJECT_CREATE: &str = "\
blueprint: a-new-shop
permissions: [projects:write]

steps:
  shop:
    do: project.create
    with: { name: Shop }
";

/// `project.scan`.
pub const PROJECT_SCAN: &str = "\
blueprint: build-then-look
permissions: [shell, projects:read]

steps:
  build:
    do: shell
    with: { cmd: [make, docs] }

  looked:
    do: project.scan
";

/// `fs.write`.
pub const FS_WRITE: &str = "\
blueprint: notes-kept
permissions: [projects:write]

on:
  noted:
    event: note.said
    outside: true
    do: fs.write
    with: { path: notes/latest.md, content: \"${event.data.words}\" }
";

/// `fs.read`.
pub const FS_READ: &str = "\
blueprint: notes-read
permissions: [projects:read]

steps:
  read:
    do: fs.read
    with: { path: notes/latest.md, missing_ok: true }

  said:
    do: log
    with: { message: \"${steps.read.out.content}\" }
";

/// `library.export`.
pub const LIBRARY_EXPORT: &str = "\
blueprint: docs-as-pages
permissions: [projects:write]

steps:
  pages:
    do: library.export
    with: { collection: docs }
";

/// `actor.holds`.
pub const ACTOR_HOLDS: &str = "\
blueprint: done-by-whom
permissions: [board:write]

on:
  moved:
    event: board.item.moved
    when: ${ event.data.to == \"done\" }
    steps:
      may:
        do: actor.holds
        with: { actor: \"${event.source.actor}\", scope: board:write }

      said:
        when: ${ !steps.may.out.holds }
        do: log
        with: { message: \"${event.data.id} was moved by somebody who may no longer move cards.\" }
";
