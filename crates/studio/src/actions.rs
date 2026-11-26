//! What blueprints can do with channels, the board, records and project files.
//!
//! Each action acts as the principal the instance runs as, so a blueprint
//! can post, move cards and write files exactly as far as whoever applied it
//! could, and no further.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionCx, ActionError, ActionResult, Registry};
use kernel::{BoxFut, ErrorKind, Fields};
use serde_json::{Value, json};

use crate::Studio;
use crate::board::{Changes, item_value};

pub(crate) fn add<F, Fut>(registry: &mut Registry, studio: &Studio, spec: ActionSpec, run: F)
where
    F: Fn(Studio, ActionCx, Fields) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ActionResult> + Send + 'static,
{
    let studio = studio.clone();
    registry.add_fn(spec, move |cx, args| -> BoxFut<'static, ActionResult> {
        Box::pin(run(studio.clone(), cx, args))
    });
}

pub(crate) fn text(args: &Fields, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::to_string).filter(|text| !text.is_empty())
}

fn strings(args: &Fields, key: &str) -> Option<Vec<String>> {
    args.get(key).and_then(Value::as_array).map(|items| items.iter().map(blueprint::expr::display).collect())
}

/// The project an action is about: the one named, else the instance's own.
pub(crate) fn project_of(cx: &ActionCx, args: &Fields) -> Result<String, ActionError> {
    text(args, "project")
        .or_else(|| cx.project().map(str::to_string))
        .ok_or_else(|| ActionError::new("which project? give `project`, or apply this blueprint within one"))
}

/// What `wrote` of an action that adds is.
const WROTE: &str = "the agent whose work it is added from, as an actor (`agent:<name>`), where that is known from what is written: its history says so beside this blueprint. Anything else, or nothing, says nobody";

/// The step this action is, for what it does to a card: the card's history
/// then says the blueprint did it, and not whoever applied the blueprint.
pub(crate) fn through(cx: &ActionCx) -> crate::board::Through {
    crate::board::Through {
        blueprint: cx.blueprint().to_string(),
        source: cx.source(),
        wrote: None,
        decided: None,
        had: None,
        doing: Some(cx.doing().to_string()),
        uncounted: false,
    }
}

/// The same, with whose work it was done from where the step says
/// (`wrote`): an agent, by its actor, and nothing else, so that what a
/// history says came of an agent's work is said of an agent.
pub(crate) fn through_wrote(cx: &ActionCx, args: &Fields) -> crate::board::Through {
    let wrote =
        text(args, "wrote").filter(|wrote| wrote.strip_prefix("agent:").is_some_and(|name| !name.is_empty()));
    crate::board::Through { wrote, ..through(cx) }
}

pub(crate) fn register(registry: &mut Registry, studio: &Studio) {
    kept(registry, studio);
    outside(registry, studio);
}

/// What works something out, and what reads and writes the tables the
/// studio keeps in its journal: the actions a sandbox test of a blueprint
/// is given as they are, over a journal of the sandbox's own
/// (`Studio::register_kept`).
pub(crate) fn kept(registry: &mut Registry, studio: &Studio) {
    crate::rows::register(registry, studio);
    crate::first::register(registry, studio);
    talk::answered::register(registry);
    crate::cards::register(registry, studio);
    crate::pressed::register(registry, studio);
    crate::said::register(registry, studio);
    crate::questioned::register(registry, studio);
    crate::change::register(registry, studio);
    talk::register(registry, &studio.channels);

    add(
        registry,
        studio,
        ActionSpec::new("board.add", "Add a card to a project's board.")
.example(kernel::asked::BOARD_ADD)
            .making("a card", "title")
            .required("title", T::String, "what the work is")
            .arg("project", T::String, "which board (default: this instance's project)")
            .arg("body", T::String, "details")
            .arg("status", T::String, "the column to put it in (default: the first)")
            .arg("labels", T::List, "labels")
            .arg("fields", T::Object, "what else its board keeps of it, each field by its name: the ones the board's blueprint declared beyond a card's own (`sprint: shop-sprints-3`)")
            .arg("wrote", T::String, WROTE)
            .capability("board:write")
            .output("id", "the card's id")
            // The same done twice: a step done again because the daemon
            // stopped in the middle of it finds the card it added, by
            // what was said of it (`crate::left`).
            .idempotent(),
        |studio, cx, args| async move {
            let project = project_of(&cx, &args)?;
            if let Some(id) = studio.left(&cx, (&project, Some(crate::board::BOARD)), "board.item.created").await? {
                return Ok(match studio.board.get(&id).await {
                    Ok(card) => item_value(&card),
                    Err(_) => json!({ "id": id }),
                });
            }
            let card = crate::board::New {
                title: text(&args, "title").unwrap_or_default(),
                body: text(&args, "body").unwrap_or_default(),
                status: text(&args, "status"),
                labels: strings(&args, "labels").unwrap_or_default(),
                more: args.get("fields").and_then(Value::as_object).cloned().unwrap_or_default(),
            };
            let through = through_wrote(&cx, &args);
            Ok(item_value(&studio.board.add_through(&project, card, cx.principal(), &through).await?))
        },
    );

    add(
        registry,
        studio,
        ActionSpec::new(
            "collection.ensure",
            "Declare a collection of records for a project: what they are made of, how they are looked at, and where they are kept.",
        )
.example(kernel::asked::COLLECTION_ENSURE)
        .making("a table", "name")
        // A table that sits beside another says something of it, and is
        // what a summary offers to change the rows of.
        .setting("beside")
        // What it is made of, by the words this action reads, which a summary says each table by.
        .modelled(blueprint::Modelled {
            parts: "fields",
            kind: ("type", "text"),
            of: "of",
            naming: &["record", "records"],
            required: "required",
            takes: ("takes", "each", "whoever writes a row holds each scope it then says, which the row gives"),
            unsaid: "unsaid",
            suggests: "suggests",
            beside: ("beside", "about"),
            by_hand: "by_hand",
        })
        .required("name", T::String, "the collection's name; a project's board is the one called `board`")
        .required("fields", T::Object, "what its records are made of: each field by its name, with what it holds")
        .within("fields", kernel::inside::FIELDS)
        .arg("views", T::Object, "the ways it starts laid out: each by its name, with how its records are laid out and what that goes by")
        .within("views", kernel::inside::VIEWS)
        .arg("store", T::String, "where the records are kept: `local`, this installation's database (the default), `files`, one file a record in the project, `service`, a store service another blueprint hosts, `documents`, the Markdown files of a folder of the project as they are, or `mockups`, the folders of a folder of the project that each hold a page")
        .arg("at", T::String, "for `store: service`, the address the store service answers at: what the blueprint that hosts it gives out as `url`; for `store: documents` or `mockups`, the folder of the project they are in (the one called what the collection is, unless said)")
        .arg("model", T::String, "the model its records are, where they are one this program itself reads: `card`, for a board. A field of the model that is declared must be what the model says; one left out that a record can do without is there all the same")
        .arg("history", T::Any, "which events are kept as the history of its records: a list of topics or groups of them (`board.item.*`), or a map from each to the sentence said of it, with what the event says of itself in braces (`{actor} moved it from {from} to {to}`)")
        .arg("takes", T::String, "a scope changing its records takes beside what changing any record does, such as `agents:write` for a table a fleet is arranged by; whoever declares it, or declares it again, must hold it")
        .arg("reads", T::String, "a scope reading its records takes beside what reading any record does, such as `private:notes` for a table that is kept from whoever was not given that: its records, the files it keeps them in and what is said of them are then theirs alone who hold it; whoever declares it, or declares it again, must hold it")
        .arg("steers", T::String, "another collection of the project whose records these steer, such as `agents` for the table that says what a fleet's agents are given to do: what those records do, and for whom, goes by what a record here says, so one is added or changed here only by whoever could write each of those as it stands (what its cells take, and each scope a list of its own gives); declaring it again without this takes `admin`")
        .arg("beside", T::String, "another table of the project this one sits beside, such as `board` for the tables a fleet is arranged by: it is then reached from that table's page and is not listed by itself among what the project has. It is how the table is shown and nothing else: it lets nobody in and keeps nobody out")
        .arg("about", T::String, "with `beside`: the field of this table that says what of the other a row is about, such as `column` for a row that says what is done with the cards of a column, which is then said at that column's head; a row that leaves the field empty is about what it is called")
        .arg("groups", T::Any, "its columns in groups: a list of headings in the order they are drawn, each a mapping from the heading to the fields under it in order, as in `[{ \"Who they are\": [role, count] }]`. A field in no group is drawn before the groups; one named twice, or that the collection has not got, is refused. It is how the table is shown and nothing else")
        .arg("by_hand", T::Bool, "`false` for a table whose rows are written only by the commands and blueprints that keep it, such as one a script adds a row to for each run: the client then offers no new row, no cell to change and no column to add, call something else or take away, and its rows are read there, sorted, held, gathered and taken away as any are. Nothing else is refused: a command and a blueprint write its records as they write any table's. Said only of a table of plain records; it is how the table is shown and nothing else")
        .arg("title", T::String, "what it is called for people, beside its name, such as `Test runs` for `test-runs`: its name is still what everything else knows it by. Whoever may change it may call it something else, which is kept when it is declared again; a board, a library and mockups keep their words")
        .arg("description", T::String, "what it is for, in one sentence, said under its head wherever it is shown")
        .arg("anew", T::Bool, "`true` for a table that is only to be made: where the project has a table of its name that something else declared (another blueprint, a script, a person), it is refused and nothing of that one is changed; declared again by the same instance it is the same done twice")
        .arg("project", T::String, "which project (default: this instance's)")
        .capability("projects:write")
        .output("name", "the collection's name")
        .output("store", "where it is kept")
        .output("at", "for one kept by a store service, its address")
        .output("records", "how many records it has")
        .idempotent(),
        |studio, cx, args| async move {
            let project = project_of(&cx, &args)?;
            let name = text(&args, "name").unwrap_or_default();
            let fields = args.get("fields").cloned().unwrap_or(Value::Null);
            let views = args.get("views").cloned().unwrap_or(Value::Null);
            let spec = crate::collections::Spec::read(&fields, &views, text(&args, "store").as_deref())?
                .kept_at(text(&args, "at").as_deref())
                .modelled(text(&args, "model").as_deref())
                .taking(text(&args, "takes").as_deref())?
                .read_by(text(&args, "reads").as_deref())?
                .steering(text(&args, "steers").as_deref())
                .sitting(text(&args, "beside").as_deref(), text(&args, "about").as_deref())?
                .grouped(args.get("groups").unwrap_or(&Value::Null))?
                .by_hand(args.get("by_hand").unwrap_or(&Value::Null))?
                .titled(text(&args, "title").as_deref())?
                .described(text(&args, "description").as_deref())?
                .with_history(args.get("history").unwrap_or(&Value::Null))?
                .anew(args.get("anew").and_then(Value::as_bool).unwrap_or(false))
                .declared_by(cx.blueprint());
            let collection =
                studio.collections.ensure(&project, &name, spec, Some(cx.instance()), cx.principal()).await?;
            // The instance whose blueprint said so is the one that has it.
            cx.hold(&format!("collection:{}", collection.name)).await?;
            let records = studio.collections.count(&collection).await?;
            Ok(json!({
                "project": project, "name": collection.name, "store": collection.store.as_str(),
                "at": collection.at, "records": records,
            }))
        },
    );

    add(
        registry,
        studio,
        ActionSpec::new("board.move", "Move a card to another column.")
.example(kernel::asked::BOARD_MOVE)
            .required("id", T::String, "the card")
            .required("to", T::String, "the column")
            .arg("note", T::String, "what is said as it is moved, such as why it is sent back from review; kept in the card's history")
            .arg("decided", T::String, "the agent whose decision the move is (`agent:<name>`), said with it and in the card's history; it gives nobody anything")
            .arg("had", T::Int, "how many times the card had been moved when whoever moves it looked (`moved` of the card, its count): where it has been moved since, it is left where it is and nothing is said")
            .arg("uncounted", T::Bool, "it is sent back for something other than a verdict on its work, such as to be brought up to date: said with the move, and not counted among the times it was sent back")
            .capability("board:write")
            .output("status", "the column it is now in")
            .idempotent(),
        |studio, cx, args| async move {
            let (id, to) = (text(&args, "id").unwrap_or_default(), text(&args, "to").unwrap_or_default());
            let decided = text(&args, "decided").filter(|by| by.strip_prefix("agent:").is_some_and(|name| !name.is_empty()));
            let had = args.get("had").and_then(Value::as_u64);
            let uncounted = args.get("uncounted").and_then(Value::as_bool).unwrap_or(false);
            let (note, through) = (text(&args, "note"), crate::board::Through { decided, had, uncounted, ..through(&cx) });
            let moved = studio.board.move_through(&id, &to, None, note.as_deref(), cx.principal(), Some(&through));
            Ok(item_value(&moved.await?))
        },
    );

    add(
        registry,
        studio,
        ActionSpec::new(
            "board.settle",
            "Say of each card that was moved or added with nobody told that it was: for when the daemon has \
             started, and once for each start (asked again, nothing is said).",
        )
        .arg("project", T::String, "which project's board (default: this instance's)")
        .capability("board:write")
        .output("said", "the cards it was said of, each with its `id`, where it was last said to be (`from`; empty for one \
             never said to be added) and where it is (`to`)")
        .idempotent(),
        |studio, cx, args| async move {
            let project = project_of(&cx, &args)?;
            Ok(json!({ "said": studio.board.settle(&project).await? }))
        },
    );

    add(
        registry,
        studio,
        ActionSpec::new(
            "board.update",
            "Change a card's title, details, assignee, labels or the day it is wanted by.",
        )
.example(kernel::asked::BOARD_UPDATE)
        .required("id", T::String, "the card")
        .arg("title", T::String, "a new title")
        .arg("body", T::String, "new details")
        .arg("assignee", T::String, "who is doing it, e.g. `agent:builder`; empty to clear")
        .arg("labels", T::List, "new labels")
        .arg("due", T::String, "the day it is wanted by, e.g. `2026-11-30`; empty to clear")
        .arg("fields", T::Object, "what else its board keeps of it, each field by its name: the ones the board's blueprint declared beyond a card's own, such as the record it names (`sprint: shop-sprints-3`); empty to clear one")
        .capability("board:write")
        .idempotent(),
        |studio, cx, args| async move {
            let changes = Changes {
                more: args.get("fields").and_then(Value::as_object).cloned().unwrap_or_default(),
                title: text(&args, "title"),
                body: args.get("body").and_then(Value::as_str).map(str::to_string),
                assignee: args
                    .get("assignee")
                    .and_then(Value::as_str)
                    .map(|who| (!who.is_empty()).then(|| who.to_string())),
                labels: strings(&args, "labels"),
                due: args
                    .get("due")
                    .and_then(Value::as_str)
                    .map(|day| (!day.is_empty()).then(|| day.to_string())),
            };
            let id = text(&args, "id").unwrap_or_default();
            let through = through(&cx);
            Ok(item_value(&studio.board.update_through(&id, changes, cx.principal(), Some(&through)).await?))
        },
    );

    add(
        registry,
        studio,
        ActionSpec::new("board.list", "The cards on a project's board.")
.example(kernel::asked::BOARD_LIST)
            .arg("project", T::String, "which project (default: this instance's)")
            .arg("status", T::String, "only the cards in this column")
            .arg("except", T::String, "all but the cards in this column")
            .arg(
                "ids",
                T::List,
                "only these cards, in this order, each found by its id without the rest of the board being read; \
                 one that names no card of the project is left out",
            )
            .capability("board:read")
            .output("items", "the cards, each with its `id`, `title` and `status`, in the order of the board")
            .output("ids", "their ids, in the same order")
            .output("count", "how many there are")
            .output("columns", "the board's columns, left to right, whatever was asked for")
            .output("first", "the first of its columns, where cards begin")
            .output("last", "the last of its columns, where they end")
            .idempotent(),
        |studio, cx, args| async move {
            let project = project_of(&cx, &args)?;
            let (only, except) = (text(&args, "status"), text(&args, "except"));
            let cards = match args.get("ids").and_then(Value::as_array) {
                Some(ids) => {
                    let mut named = Vec::new();
                    for id in ids.iter().map(blueprint::expr::display) {
                        match studio.board.get(&id).await {
                            Ok(card) if card.project == project => named.push(card),
                            // What an id names may be gone, or be another project's.
                            Ok(_) => {}
                            Err(gone) if gone.kind == ErrorKind::NotFound => {}
                            Err(other) => return Err(other.into()),
                        }
                    }
                    named
                }
                // One column is read by itself, by the index of a board's
                // records by their column: a stage looks at its own.
                None => match (&only, &except) {
                    (Some(column), None) => studio.board.column(&project, column).await?,
                    _ => studio.board.list(&project).await?,
                },
            };
            let items: Vec<Value> = cards
                .iter()
                .filter(|item| only.as_deref().is_none_or(|column| item.status == column))
                .filter(|item| except.as_deref().is_none_or(|column| item.status != column))
                .map(|item| json!({ "id": item.id, "title": item.title, "status": item.status }))
                .collect();
            let ids: Vec<&Value> = items.iter().map(|item| &item["id"]).collect();
            let columns = studio.board.columns(&project).await?;
            let (first, last) = (columns.first(), columns.last());
            Ok(json!({
                "count": items.len(), "ids": ids, "items": items, "columns": columns, "first": first, "last": last,
            }))
        },
    );

    add(
        registry,
        studio,
        ActionSpec::new("board.get", "Read a card.")
.example(kernel::asked::BOARD_GET)
            .deciding()
            .required("id", T::String, "the card")
            .capability("board:read")
            .output("id", "the card's id")
            .output("title", "its title")
            .output("body", "its details")
            .output("status", "its column")
            .output("assignee", "who it is given to, as `user:<id>` or `agent:<slot>`, if anyone")
            .output("labels", "its labels")
            .output("due", "the day it is wanted by, if one was said")
            .output(
                "moved",
                "how it came to be where it is, as its row says: the column it was moved from, the kind of actor \
                 that moved it, what was said as it was moved, and how many times it has been moved; empty for a \
                 card never moved",
            )
            .output("pressed", "what was pressed on it of what its column offers, as its row keeps it; empty where nothing was")
            .output("more", "what else its board keeps of it: the fields its blueprint declared, by name")
            .output(
                "ranked",
                "where each of its choices stands among what its board offers, by name, as a number: the first as \
                 many as there are and the last 1, by the order they stand in now, with nothing chosen where the \
                 word nothing stands for does (`unsaid`), else 0; and each of its numbers as it is. What a pool \
                 that takes cards by priority goes by",
            )
            .idempotent(),
        |studio, _cx, args| async move {
            let (board, record) = studio.board.card(&text(&args, "id").unwrap_or_default()).await?;
            let mut ranked = serde_json::Map::new();
            for field in board.fields.iter().filter(|field| field.kind == "choice" || field.kind == "number") {
                let held = record.data.get(&field.name).cloned().unwrap_or(Value::Null);
                let rank = match field.kind.as_str() {
                    "choice" => json!(kernel::ranked::rank(&held, &field.of, field.unsaid.first().map(String::as_str))),
                    _ => held,
                };
                ranked.insert(field.name.clone(), rank);
            }
            let mut card = item_value(&crate::board::item(record));
            card["ranked"] = Value::Object(ranked);
            Ok(card)
        },
    );

    add(
        registry,
        studio,
        ActionSpec::new(
            "record.add",
            "Add a record to a collection a blueprint declared: a sprint, a release, whatever the project keeps.",
        )
.example(kernel::asked::RECORD_ADD)
        .making("a row of the table", "collection")
        .required("collection", T::String, "the collection's name, as `collection.ensure` declared it")
        .required("fields", T::Object, "what it holds, each field by its name (`title: November`)")
        .arg("project", T::String, "which project (default: this instance's)")
        .arg("wrote", T::String, WROTE)
        .arg("declaring", T::Object, "the fields a table of this name is declared with where the project has none, as `collection.ensure` takes them: of one that is there only the fields it has are written, and it keeps its own. Either way it is a table that says nothing of what writing or reading it takes, of what its rows steer, or of a model")
        .within("declaring", kernel::inside::FIELDS)
        .arg("by_hand", T::Bool, "with `declaring`: `false` where the table made is one whose rows are written only by the commands and blueprints that keep it, as `collection.ensure` takes it; said of a table that is there already it changes nothing")
        .capability("records:write")
        .output("id", "the record's id, such as `shop-sprints-3`")
        .output("data", "what it holds, each field by its name")
        // The same done twice, as `board.add` is.
        .idempotent(),
        |studio, cx, args| async move {
            let project = project_of(&cx, &args)?;
            let collection = text(&args, "collection").unwrap_or_default();
            let table = blueprint::expr::slug(&collection);
            if let Some(id) = studio.left(&cx, (&project, Some(&table)), "record.created").await? {
                return Ok(match studio.collections.record(&id).await {
                    Ok((_, record)) => json!(record),
                    Err(_) => json!({ "id": id, "project": project, "collection": table }),
                });
            }
            let fields = args.get("fields").and_then(Value::as_object).cloned().unwrap_or_default();
            let through = through_wrote(&cx, &args);
            if let Some(declaring) = args.get("declaring").filter(|said| !said.is_null()) {
                let at = (project.as_str(), collection.as_str());
                let declaring = (declaring, args.get("by_hand").unwrap_or(&Value::Null));
                let added = studio.records.add_declaring(at, fields, declaring, cx.instance(), cx.principal(), Some(&through));
                return Ok(json!(added.await?));
            }
            let added = studio.records.add(&project, &collection, fields, cx.principal(), Some(&through));
            Ok(json!(added.await?))
        },
    );

    add(
        registry,
        studio,
        ActionSpec::new(
            "record.keep",
            "Keep a record under a number of your choosing, another record's own: added where the table has none of it, and changed to these fields where it has, so that one row stands for one thing and is found again by its id alone.",
        )
        .example(kernel::asked::RECORD_KEEP)
        .required("collection", T::String, "the collection's name, as `collection.ensure` declared it")
        .required("number", T::Int, "the number it is kept under, such as a card's own: the record is `<project>-<collection>-<number>`")
        .required("fields", T::Object, "what it holds, each field by its name")
        .arg("project", T::String, "which project (default: this instance's)")
        .capability("records:write")
        .output("id", "the record's id")
        .output("data", "what it now holds")
        .idempotent(),
        |studio, cx, args| async move {
            let project = project_of(&cx, &args)?;
            let collection = text(&args, "collection").unwrap_or_default();
            let number = args.get("number").and_then(Value::as_i64).unwrap_or(0);
            if number < 1 {
                return Err(kernel::Error::invalid(format!("a record is kept under a number of one or more; it is {number}")).into());
            }
            let fields = args.get("fields").and_then(Value::as_object).cloned().unwrap_or_default();
            let through = through(&cx);
            let kept = studio.records.keep((&project, &collection, number), fields, cx.principal(), Some(&through));
            Ok(json!(kept.await?))
        },
    );

    add(
        registry,
        studio,
        ActionSpec::new("record.update", "Change some of a record's fields, leaving the others as they are.")
            .example(kernel::asked::RECORD_UPDATE)
            .required("id", T::String, "the record")
            .required("fields", T::Object, "the fields to change, each by its name; empty to clear one")
            .capability("records:write")
            .output("data", "what it now holds")
            .idempotent(),
        |studio, cx, args| async move {
            let id = text(&args, "id").unwrap_or_default();
            let fields = args.get("fields").and_then(Value::as_object).cloned().unwrap_or_default();
            let through = through(&cx);
            Ok(json!(studio.records.update(&id, fields, cx.principal(), Some(&through)).await?))
        },
    );

    add(
        registry,
        studio,
        // It asks for nothing here: what it needs depends on the record
        // (`board:write` for a card, `records:write` for any other), and is
        // asked for when it is done.
        ActionSpec::new(
            "record.comment",
            "Say something on a record, a card included; it is kept in the record's history and changes nothing of it. Needs `board:write` for a card, `records:write` for any other.",
        )
.example(kernel::asked::RECORD_COMMENT)
        .required("id", T::String, "the record")
        .required("body", T::String, "what to say")
        .arg("public", T::Bool, "mark it public: it is private unless this says so, and what is private is told to nobody outside the factory by whatever keeps the record somewhere else as well")
        .arg("anchor", T::Object, "the words of a document it is about, where it is about a few of them and not the whole document")
        .within("anchor", kernel::anchor::WITHIN)
        .output("id", "the record")
        .output("collection", "the collection it is a record of")
        // The same done twice, as `board.add` is: what is said is only
        // said, so what was said is all there is to find.
        .idempotent(),
        |studio, cx, args| async move {
            let (id, body) = (text(&args, "id").unwrap_or_default(), text(&args, "body").unwrap_or_default());
            if cx.again() {
                let (_, record) = studio.collections.record(&id).await?;
                if studio.left(&cx, (&record.project, None), crate::records::COMMENTED).await?.is_some() {
                    return Ok(json!(record));
                }
            }
            let public = args.get("public").and_then(Value::as_bool).unwrap_or(false);
            let anchor = kernel::anchor::read(args.get("anchor"))?;
            let through = through(&cx);
            Ok(json!(studio.records.comment_on(&id, &body, public, anchor.as_ref(), cx.principal(), Some(&through)).await?))
        },
    );

    add(
        registry,
        studio,
        // What an instance does for somebody else it does with its own
        // authority; this is how its blueprint asks what that somebody
        // holds themselves, before it does. It tells nobody what they
        // could not do: whoever asks holds the scope asked about.
        ActionSpec::new(
            "actor.holds",
            "Whether whoever an event says did it holds a scope now. Whoever asks has to hold that scope themselves.",
        )
.example(kernel::asked::ACTOR_HOLDS)
        .arg("actor", T::String, "who, as an event's `source.actor` says them: `user:<id>`, `key:<id>` or `agent:<slot>`; nobody, if empty")
        .required("scope", T::String, "the scope, as `area` or `area:verb`")
        .arg("scopes", T::List, "more scopes asked with it, every one of which they are to hold: what something they ask for runs with, as a blueprint's `permissions` say it")
        .arg("who", T::String, "of whom it is asked: `person`, and an agent is answered as not holding it, whatever it holds, for something that is a person's to do; or `anyone` (the usual), and it goes by what they hold alone")
        .output("holds", "whether they hold it now; false for an actor that is nobody's any more, or never was anybody, and for an agent where it was asked of a person")
        .output("known", "whether there is such an actor now")
        .output("name", "what they are called, where there is such an actor; empty where there is none")
        .output("kind", "what kind of actor it is, as the actor says before its colon: `user`, `key`, `agent`, or whatever else did it (`system`, `caller`); empty for nobody")
        .output("refused", "why they do not hold it, as a word: `agent`, asked of a person and it is an agent; `scope`, they do not hold it, or are nobody; empty where they hold it")
        .output("lacks", "the scopes asked that they do not hold, in the order they were asked; empty where they hold them all")
        .idempotent(),
        |studio, cx, args| async move {
            let (actor, scope) = (text(&args, "actor").unwrap_or_default(), text(&args, "scope").unwrap_or_default());
            let mut asked = vec![scope];
            for more in args.get("scopes").and_then(Value::as_array).into_iter().flatten() {
                match more.as_str() {
                    Some(more) if !asked.iter().any(|one| one == more) => asked.push(more.to_string()),
                    Some(_) => {}
                    None => return Err(ActionError::new(format!("`scopes` is a list of scopes, and {more} is none"))),
                }
            }
            for scope in &asked {
                cx.principal().require(scope)?;
            }
            let persons = match text(&args, "who").as_deref() {
                None | Some("" | "anyone") => false,
                Some("person") => true,
                Some(other) => {
                    return Err(ActionError::new(format!(
                        "`who` is `person` or `anyone`, and what is said is `{other}`"
                    )));
                }
            };
            let holder = studio.identity.holder(&actor).await?;
            let kind = actor.split_once(':').map_or(actor.as_str(), |(kind, _)| kind);
            // Asked of a person, an agent holds nothing here whatever its
            // slot was given; everybody else is asked what they hold, and
            // whatever is nobody (the factory, a caller) holds nothing.
            let may = !persons || kind != "agent";
            let lacks: Vec<&String> = match (&holder, may) {
                (Some(holder), true) => asked.iter().filter(|scope| !holder.can(scope)).collect(),
                _ => asked.iter().collect(),
            };
            let holds = lacks.is_empty();
            let refused = match (holds, may) {
                (true, _) => "",
                (false, false) => "agent",
                (false, true) => "scope",
            };
            Ok(json!({
                "holds": holds,
                "known": holder.is_some(),
                "name": holder.as_ref().map(|holder| holder.name.clone()).unwrap_or_default(),
                "kind": kind,
                "refused": refused,
                "lacks": lacks,
            }))
        },
    );

    add(
        registry,
        studio,
        ActionSpec::new("record.get", "Read a record.")
            .example(kernel::asked::RECORD_GET)
            .deciding()
            .required("id", T::String, "the record")
            .capability("records:read")
            .output("collection", "the collection it is a record of")
            .output("data", "what it holds, each field by its name")
            .idempotent(),
        |studio, cx, args| async move {
            Ok(json!(studio.records.get(&text(&args, "id").unwrap_or_default(), cx.principal()).await?))
        },
    );

    add(
        registry,
        studio,
        ActionSpec::new("record.list", "The records of a collection.")
            .example(kernel::asked::RECORD_LIST)
            .required("collection", T::String, "the collection's name")
            .arg("project", T::String, "which project (default: this instance's)")
            .capability("records:read")
            .output("items", "the records, each with its `id` and its `data`, in the order they were made")
            .output("count", "how many there are")
            .idempotent(),
        |studio, cx, args| async move {
            let project = project_of(&cx, &args)?;
            let collection = text(&args, "collection").unwrap_or_default();
            let items = studio.records.list(&project, &collection, cx.principal()).await?;
            Ok(json!({ "count": items.len(), "items": items }))
        },
    );

    add(
        registry,
        studio,
        ActionSpec::new(
            "record.remove",
            "Take a record away. What named it goes on saying what it named, and its history is kept.",
        )
        .example(kernel::asked::RECORD_REMOVE)
        .required("id", T::String, "the record")
        .capability("records:write"),
        |studio, cx, args| async move {
            let through = through(&cx);
            studio
                .records
                .remove(&text(&args, "id").unwrap_or_default(), cx.principal(), Some(&through))
                .await?;
            Ok(json!({}))
        },
    );
}

/// What reaches outside the tables: a project's files, written and read,
/// and a folder taken up as a project. An action added here is one that
/// a sandbox test is given a stand-in for and never the thing itself; one that only keeps rows is added in [`kept`].
fn outside(registry: &mut Registry, studio: &Studio) {
    crate::attaching::register(registry, studio);
    add(
        registry,
        studio,
        ActionSpec::new(
            "library.export",
            "Write a library out as web pages that anything can serve: a page a document, with what the pages use beside them.",
        )
.example(kernel::asked::LIBRARY_EXPORT)
        .making("the web pages of the library", "collection")
        .required("collection", T::String, "the library's name")
        .arg("project", T::String, "which project (default: this instance's)")
        .arg("to", T::String, "the folder of the project it is written to, replaced as a whole (default: `site`)")
        .arg(
            "publish",
            T::String,
            "which documents: `marked`, the ones whose own settings say `public: true` (default), or `all`",
        )
        .capability("projects:write")
        .output("to", "the folder it was written to, from the top of the project")
        .output("from", "the library's own folder, from the top of the project")
        .output("pages", "how many pages: a document each, and a front page for each folder")
        .output("files", "how many other files: the style sheet, and what the pages use")
        .output("changed", "whether anything was written; what is already as it would be written is left alone")
        .output(
            "left_out",
            "each link or picture written as its words and not followed: the `page` it is in, the `link`, and `why`",
        )
        .idempotent(),
        |studio, cx, args| async move {
            let project = project_of(&cx, &args)?;
            let done = crate::site::export(
                &studio,
                &project,
                &text(&args, "collection").unwrap_or_default(),
                text(&args, "to").as_deref(),
                text(&args, "publish").as_deref(),
                cx.principal(),
                cx.source(),
            )
            .await?;
            Ok(serde_json::to_value(done).unwrap_or_default())
        },
    );

    add(
        registry,
        studio,
        ActionSpec::new("project.create", "Create a project, or adopt an existing directory as one.")
            .example(kernel::asked::PROJECT_CREATE)
            .making("a project", "name")
            .required("name", T::String, "the project's name")
            .arg("path", T::String, "an existing directory to adopt (absolute)")
            .capability("projects:write")
            .output("id", "the project's id")
            .output("path", "where its files are"),
        |studio, cx, args| async move {
            let path = text(&args, "path").map(std::path::PathBuf::from);
            let project = studio
                .projects
                .create(&text(&args, "name").unwrap_or_default(), path, cx.principal())
                .await?;
            Ok(json!({ "id": project.id, "path": project.path, "brownfield": project.brownfield }))
        },
    );

    add(
        registry,
        studio,
        ActionSpec::new("fs.write", "Write a file in a project.")
            .example(kernel::asked::FS_WRITE)
            .making("a file of the project", "path")
            .required("path", T::String, "where, relative to the project")
            .required("content", T::String, "what to write")
            .arg("project", T::String, "which project (default: this instance's)")
            .capability("projects:write")
            .output("path", "the file written")
            .idempotent(),
        |studio, cx, args| async move {
            let project = project_of(&cx, &args)?;
            let content = args.get("content").and_then(Value::as_str).unwrap_or_default();
            let entry = studio
                .projects
                .write(&project, &text(&args, "path").unwrap_or_default(), content, cx.principal())
                .await?;
            Ok(json!({ "path": entry.path, "size": entry.size }))
        },
    );

    add(
        registry,
        studio,
        ActionSpec::new("fs.read", "Read a file in a project.")
            .example(kernel::asked::FS_READ)
            .required("path", T::String, "which file, relative to the project")
            .arg("project", T::String, "which project (default: this instance's)")
            .arg(
                "missing_ok",
                T::Bool,
                "return `exists: false` instead of failing when there is no such file",
            )
            .capability("projects:read")
            .output("content", "the file's text")
            .output("exists", "whether it was there")
            .idempotent(),
        |studio, cx, args| async move {
            let project = project_of(&cx, &args)?;
            let path = text(&args, "path").unwrap_or_default();
            studio.projects.may_read_path(&project, &path, cx.principal()).await?;
            match studio.projects.read(&project, &path).await {
                Ok(content) => Ok(json!({ "exists": true, "content": content })),
                Err(err)
                    if err.kind == kernel::ErrorKind::NotFound
                        && args.get("missing_ok") == Some(&Value::Bool(true)) =>
                {
                    Ok(json!({ "exists": false, "content": "" }))
                }
                Err(err) => Err(err.into()),
            }
        },
    );

    add(
        registry,
        studio,
        ActionSpec::new("project.scan", "Notice files that changed outside the system, e.g. after a build.")
            .example(kernel::asked::PROJECT_SCAN)
            .arg("project", T::String, "which project (default: this instance's)")
            .capability("projects:read")
            .output("changed", "how many files were added, modified or removed")
            .idempotent(),
        |studio, cx, args| async move {
            let changes = studio.projects.scan(&project_of(&cx, &args)?).await?;
            Ok(json!({ "changed": changes.len() }))
        },
    );
}
