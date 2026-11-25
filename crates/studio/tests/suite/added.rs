//! A table's columns as the people who look at it change them
//! (`studio::added`): one added, called something else, retyped, its
//! options changed and put in order, and one taken away; what a
//! declaration over them comes to; and who may.

use kernel::ErrorKind;
use serde_json::{Map, Value, json};
use studio::added::{Change, New, Place, heading};
use studio::collections::Spec;
use studio::{Collection, Collections, Record, View};

use crate::{Rig, ada, agent};

fn data(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap()
}

fn words(all: &[&str]) -> Vec<String> {
    all.iter().map(|one| one.to_string()).collect()
}

const FIELDS: &str = r#"{
    "title": { "type": "text", "required": true, "icon": "tag" },
    "size": { "type": "number" },
    "stage": { "type": "choice", "of": ["new", "old", "gone"] }
}"#;

/// Declares `tasks` in the project `shop`, with these fields.
async fn tasks(collections: &Collections, fields: Value, store: &str) -> kernel::Result<Collection> {
    let views = json!({ "rows": { "kind": "table" } });
    let spec = Spec::read(&fields, &views, Some(store)).unwrap();
    collections.ensure("shop", "tasks", spec, None, &ada()).await
}

async fn shop(store: &str) -> Rig {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    tasks(&rig.studio.collections, serde_json::from_str(FIELDS).unwrap(), store).await.unwrap();
    rig
}

fn column(label: &str, kind: &str, of: &[&str]) -> New {
    New { label: label.into(), kind: kind.into(), of: words(of), ..New::default() }
}

async fn add(collections: &Collections, value: Value) -> Record {
    let table = collections.get("shop", "tasks").await.unwrap();
    collections.add(&table, data(value), 1.0, &ada()).await.unwrap()
}

/// The names of a table's fields, in the order its row keeps them.
fn names(table: &Collection) -> Vec<&str> {
    table.fields.iter().map(|field| field.name.as_str()).collect()
}

async fn held(collections: &Collections, id: &str, field: &str) -> Value {
    let table = collections.get("shop", "tasks").await.unwrap();
    let records = collections.records(&table).await.unwrap();
    records.iter().find(|record| record.id == id).unwrap().data.get(field).cloned().unwrap_or(Value::Null)
}

#[tokio::test]
async fn a_column_somebody_adds_is_the_tables_and_holds_no_authority() {
    let rig = shop("local").await;
    let collections = &rig.studio.collections;
    let basket = add(collections, json!({ "title": "Basket", "size": 3 })).await;

    let (table, called) =
        collections.add_column("shop", "tasks", column("Story Points", "number", &[]), &ada()).await.unwrap();
    assert_eq!(called, "story_points");
    assert_eq!(names(&table), ["size", "stage", "title", "story_points"]);
    let field = table.field("story_points").unwrap();
    // It is marked as somebody's, called what they wrote, and says nothing that gives or guards anything.
    assert!(field.added);
    assert_eq!(
        (field.label_chosen.as_deref(), heading(field).as_str()),
        (Some("Story Points"), "Story Points")
    );
    assert!(!field.required && !field.quiet && !field.hidden && !field.takes_each);
    assert!(field.takes.is_empty() && field.takes_of.is_empty() && field.unsaid.is_empty());
    // Called what its name reads as, it keeps no label.
    let (table, called) =
        collections.add_column("shop", "tasks", column("Due date", "date", &[]), &ada()).await.unwrap();
    assert_eq!(called, "due_date");
    assert_eq!(table.field("due_date").unwrap().label_chosen, None);
    // Everybody is told, in the write that kept it.
    let told: Vec<Value> =
        rig.events("collection.fields").await.into_iter().map(|event| event.data).collect();
    assert_eq!(
        told,
        [
            json!({ "project": "shop", "name": "tasks", "field": "story_points", "did": "added" }),
            json!({ "project": "shop", "name": "tasks", "field": "due_date", "did": "added" }),
        ]
    );

    // A record from before reads with nothing in it, and is written with something there.
    assert_eq!(held(collections, &basket.id, "story_points").await, Value::Null);
    let mut pointed = collections.record(&basket.id).await.unwrap().1;
    pointed.data.insert("story_points".into(), json!(5));
    collections.put(&table, pointed).await.unwrap();
    assert_eq!(held(collections, &basket.id, "story_points").await, json!(5));
    assert_eq!(held(collections, &basket.id, "size").await, json!(3));

    // What it may hold: a kind with no authority in it, said as that kind needs.
    for (wrong, why) in [
        (column("Who", "actor", &[]), "a column somebody adds holds one of"),
        (column("Odd", "sprocket", &[]), "`sprocket` is not one of them"),
        (column("Mood", "choice", &[]), "is a choice and needs what can be chosen"),
        (column("Sprint", "record", &[]), "needs the one table whose records it can name"),
        (column("Sprint", "record", &["sprints"]), "has no collection called `sprints`"),
        (column("", "text", &[]), "a column is called by a word or a few"),
    ] {
        let refused = collections.add_column("shop", "tasks", wrong, &ada()).await.unwrap_err();
        assert!(refused.message.contains(why), "{}", refused.message);
    }
    // A choice offers what was written, each a word joined by dashes, once.
    let mood = column("Mood", "choice", &["Fine", "in a hurry", "fine", " "]);
    let (table, _) = collections.add_column("shop", "tasks", mood, &ada()).await.unwrap();
    assert_eq!(table.field("mood").unwrap().of, ["fine", "in-a-hurry"]);
    // And one names the records of a table there is: this one, say.
    let (table, _) = collections
        .add_column("shop", "tasks", column("Blocks", "records", &["Tasks"]), &ada())
        .await
        .unwrap();
    assert_eq!(table.field("blocks").unwrap().of, ["tasks"]);

    // No two columns read alike, whatever each is named; and a name that
    // is taken, or is a record's own, is numbered and never taken over.
    for alike in ["Size", "story points", "TITLE"] {
        let refused =
            collections.add_column("shop", "tasks", column(alike, "text", &[]), &ada()).await.unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Conflict, "{alike}");
        assert!(refused.message.contains("has a column called"), "{}", refused.message);
    }
    let (table, called) =
        collections.add_column("shop", "tasks", column("Size!", "text", &[]), &ada()).await.unwrap();
    assert_eq!((called.as_str(), heading(table.field("size_2").unwrap()).as_str()), ("size_2", "Size!"));
    let (table, called) =
        collections.add_column("shop", "tasks", column("Created", "date", &[]), &ada()).await.unwrap();
    assert_eq!(
        (called.as_str(), heading(table.field("created_2").unwrap()).as_str()),
        ("created_2", "Created")
    );
    let (_, called) =
        collections.add_column("shop", "tasks", column("日本", "text", &[]), &ada()).await.unwrap();
    assert_eq!(called, "column");
}

#[tokio::test]
async fn what_somebody_added_and_chose_stands_through_a_declaration_and_a_declared_name_takes_over() {
    for store in ["local", "files"] {
        let rig = shop(store).await;
        let collections = &rig.studio.collections;
        let fields = || serde_json::from_str::<Value>(FIELDS).unwrap();
        collections.add_column("shop", "tasks", column("Notes", "text", &[]), &ada()).await.unwrap();
        collections
            .add_column("shop", "tasks", column("Mood", "choice", &["fine", "rushed"]), &ada())
            .await
            .unwrap();
        let label = |field: &'static str, label: Option<&'static str>| async move {
            let change = Change::Label(label.map(str::to_string));
            collections.change_column("shop", "tasks", field, change, &ada()).await
        };
        label("size", Some("How big")).await.unwrap();
        label("notes", Some("Remarks")).await.unwrap();
        collections.choose_icon("shop", "tasks", "notes", Some("star"), &ada()).await.unwrap();
        collections.choose_icon("shop", "tasks", "size", Some("ruler"), &ada()).await.unwrap();
        let order = Change::Options(words(&["gone", "new", "old"]));
        collections.change_column("shop", "tasks", "stage", order, &ada()).await.unwrap();
        // A view of somebody's that is laid out by their column.
        let by_mood =
            View { name: "Moods".into(), kind: "board".into(), by: Some("mood".into()), ..View::default() };
        collections.save_view("shop", "tasks", by_mood, &ada()).await.unwrap();
        let basket =
            add(collections, json!({ "title": "Basket", "size": 3, "notes": "fragile", "mood": "fine" }))
                .await;

        // Declared again as it was: every column of theirs, with what was
        // chosen of each and what the rows hold; the view they laid out by
        // one; and the order they put a declared choice in.
        let again = tasks(collections, fields(), store).await.unwrap();
        assert_eq!(names(&again), ["size", "stage", "title", "notes", "mood"], "{store}");
        let notes = again.field("notes").unwrap();
        assert!(notes.added, "{store}");
        assert_eq!(
            (notes.label_chosen.as_deref(), notes.icon_chosen.as_deref()),
            (Some("Remarks"), Some("star"))
        );
        assert_eq!(again.field("mood").unwrap().of, ["fine", "rushed"]);
        let size = again.field("size").unwrap();
        assert_eq!(
            (size.added, size.label_chosen.as_deref(), size.icon_chosen.as_deref()),
            (false, Some("How big"), Some("ruler"))
        );
        assert_eq!(again.field("stage").unwrap().of, ["gone", "new", "old"], "{store}");
        assert!(
            again.views.iter().any(|view| view.name == "Moods" && view.saved),
            "{store}: {:?}",
            again.views
        );
        assert_eq!(held(collections, &basket.id, "notes").await, json!("fragile"), "{store}");
        assert_eq!(held(collections, &basket.id, "mood").await, json!("fine"), "{store}");
        // What a row holds of their choice does not stop the table being declared, as often as it is.
        tasks(collections, fields(), store).await.unwrap();

        // The declaration comes to offer another thing: it stands after the one it follows there.
        let mut wider = fields();
        wider["stage"]["of"] = json!(["new", "fresh", "old", "gone"]);
        let wider = tasks(collections, wider, store).await.unwrap();
        assert_eq!(wider.field("stage").unwrap().of, ["gone", "new", "fresh", "old"], "{store}");

        // A declaration that drops a field takes what was chosen of it away:
        // one that comes back under the name is a new one.
        let mut without = fields();
        without.as_object_mut().unwrap().remove("size");
        let dropped = tasks(collections, without, store).await.unwrap();
        assert_eq!(names(&dropped), ["stage", "title", "notes", "mood"], "{store}");
        let back = tasks(collections, fields(), store).await.unwrap();
        let size = back.field("size").unwrap();
        assert_eq!((size.label_chosen.as_deref(), size.icon_chosen.as_deref()), (None, None), "{store}");

        // A declaration that says a field of a name somebody's column has:
        // refused as any change of kind is while a row holds what does not fit,
        let mut taken = fields();
        taken["notes"] = json!({ "type": "number" });
        let refused = tasks(collections, taken, store).await.unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Conflict, "{store}");
        assert!(
            refused.message.starts_with("`notes` was a `text` and is now declared a `number`"),
            "{}",
            refused.message
        );
        assert!(collections.get("shop", "tasks").await.unwrap().field("notes").unwrap().added, "{store}");
        // and where the rows fit, the declared one stands from then on, with
        // what they hold and what was chosen of the column by its name.
        let mut taken = fields();
        taken["notes"] = json!({ "type": "text", "icon": "pen" });
        let declared = tasks(collections, taken, store).await.unwrap();
        assert_eq!(names(&declared), ["notes", "size", "stage", "title", "mood"], "{store}");
        let notes = declared.field("notes").unwrap();
        assert!(!notes.added, "{store}");
        assert_eq!(
            (notes.icon.as_deref(), notes.icon_chosen.as_deref(), notes.label_chosen.as_deref()),
            (Some("pen"), Some("star"), Some("Remarks"))
        );
        assert_eq!(held(collections, &basket.id, "notes").await, json!("fragile"), "{store}");
        // It is the declaration's now: not theirs to take away, and gone when the declaration drops it.
        let refused = collections.remove_column("shop", "tasks", "notes", &ada()).await.unwrap_err();
        assert!(refused.message.contains("is declared with the table"), "{}", refused.message);
        let last = tasks(collections, fields(), store).await.unwrap();
        assert_eq!(names(&last), ["size", "stage", "title", "mood"], "{store}");
    }
}

#[tokio::test]
async fn a_column_is_called_something_else_by_whoever_may_save_a_view_and_shaped_by_whoever_holds_the_scope()
{
    let rig = shop("local").await;
    let collections = &rig.studio.collections;
    collections.add_column("shop", "tasks", column("Notes", "text", &[]), &ada()).await.unwrap();
    // An agent holds the whole of `records` and not the scope; somebody who only reads holds neither.
    let worker = agent("worker", kernel::principal::AGENT_SCOPES);
    let reader = agent("reader", &["projects:read", "records:read", "board:read"]);
    let shaper = agent("shaper", &["records", "columns"]);

    // What a column is called, and the order of what a choice offers, are
    // how the table is looked at: whoever may save a view.
    let called = Change::Label(Some("Remarks".into()));
    let table = collections.change_column("shop", "tasks", "notes", called.clone(), &worker).await.unwrap();
    assert_eq!(heading(table.field("notes").unwrap()), "Remarks");
    let table = collections
        .change_column("shop", "tasks", "title", Change::Label(Some("Task".into())), &worker)
        .await
        .unwrap();
    assert_eq!(
        (heading(table.field("title").unwrap()).as_str(), table.field("title").unwrap().name.as_str()),
        ("Task", "title")
    );
    let order = Change::Options(words(&["old", "new", "gone"]));
    let table = collections.change_column("shop", "tasks", "stage", order.clone(), &worker).await.unwrap();
    assert_eq!(table.field("stage").unwrap().of, ["old", "new", "gone"]);
    for change in [called.clone(), order.clone()] {
        let field = if matches!(change, Change::Label(_)) { "notes" } else { "stage" };
        let refused = collections.change_column("shop", "tasks", field, change, &reader).await.unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Denied);
    }
    // No two columns are called alike, and a label taken away is the name again.
    let refused = collections
        .change_column("shop", "tasks", "size", Change::Label(Some("remarks".into())), &ada())
        .await
        .unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    let table =
        collections.change_column("shop", "tasks", "notes", Change::Label(None), &worker).await.unwrap();
    assert_eq!(table.field("notes").unwrap().label_chosen, None);
    let table = collections
        .change_column("shop", "tasks", "title", Change::Label(Some("Title".into())), &worker)
        .await
        .unwrap();
    assert_eq!(
        table.field("title").unwrap().label_chosen,
        None,
        "called what its name reads as, it is called nothing else"
    );

    // What columns there are, what one holds and what it offers take the scope.
    let needs = "is not allowed to do this (needs `columns`)";
    let refused =
        collections.add_column("shop", "tasks", column("More", "text", &[]), &worker).await.unwrap_err();
    assert!(refused.kind == ErrorKind::Denied && refused.message.contains(needs), "{}", refused.message);
    for change in [
        Change::Kind { kind: "number".into(), of: Vec::new() },
        Change::Kind { kind: "choice".into(), of: words(&["a"]) },
    ] {
        let refused = collections.change_column("shop", "tasks", "notes", change, &worker).await.unwrap_err();
        assert!(refused.kind == ErrorKind::Denied && refused.message.contains(needs), "{}", refused.message);
    }
    let refused = collections.remove_column("shop", "tasks", "notes", &worker).await.unwrap_err();
    assert!(refused.kind == ErrorKind::Denied && refused.message.contains(needs), "{}", refused.message);
    // The scope alone is not being able to change the table.
    let only = agent("only", &["columns", "records:read"]);
    let refused =
        collections.add_column("shop", "tasks", column("More", "text", &[]), &only).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied);
    let table = collections.get("shop", "tasks").await.unwrap();
    assert_eq!(names(&table), ["size", "stage", "title", "notes"]);

    // Whoever holds it does all three, of a column somebody added;
    collections.add_column("shop", "tasks", column("More", "text", &[]), &shaper).await.unwrap();
    let number = Change::Kind { kind: "number".into(), of: Vec::new() };
    collections.change_column("shop", "tasks", "more", number.clone(), &shaper).await.unwrap();
    collections.remove_column("shop", "tasks", "more", &shaper).await.unwrap();
    // and of what is declared nobody does, whatever they hold: it is read
    // by what declared it, and is changed where the table is declared.
    for field in ["size", "title", "stage"] {
        let refused =
            collections.change_column("shop", "tasks", field, number.clone(), &ada()).await.unwrap_err();
        assert!(
            refused.kind == ErrorKind::Conflict && refused.message.contains("is declared with the table"),
            "{}",
            refused.message
        );
        let refused = collections.remove_column("shop", "tasks", field, &ada()).await.unwrap_err();
        assert!(refused.message.contains("is declared with the table"), "{}", refused.message);
    }
    let other = Change::Options(words(&["new", "old", "gone", "more"]));
    let refused = collections.change_column("shop", "tasks", "stage", other, &ada()).await.unwrap_err();
    assert!(
        refused.message.contains("What it offers is said where the table is declared"),
        "{}",
        refused.message
    );
    let renamed = Change::Renamed { from: "new".into(), to: "fresh".into() };
    assert!(collections.change_column("shop", "tasks", "stage", renamed, &ada()).await.is_err());
    assert_eq!(
        collections.change_column("shop", "tasks", "nothing", called, &ada()).await.unwrap_err().kind,
        ErrorKind::NotFound
    );

    // A table kept from whoever asks is not there for them, in the words of one that is not.
    let private = Spec::read(&json!({ "title": { "type": "text" } }), &json!(null), None).unwrap();
    let private = private.read_by(Some("private:journal")).unwrap();
    collections.ensure("shop", "moods", private, None, &ada()).await.unwrap();
    let kept =
        collections.add_column("shop", "moods", column("More", "text", &[]), &shaper).await.unwrap_err();
    let missing =
        collections.add_column("shop", "nothing", column("More", "text", &[]), &shaper).await.unwrap_err();
    assert_eq!((kept.kind, kept.message.replace("moods", "nothing")), (ErrorKind::NotFound, missing.message));
    // A library's records are people's files: no column is written into them.
    let docs = Spec::read(&json!({ "title": { "type": "text" } }), &json!(null), Some("documents")).unwrap();
    collections.ensure("shop", "docs", docs, None, &ada()).await.unwrap();
    let refused =
        collections.add_column("shop", "docs", column("More", "text", &[]), &ada()).await.unwrap_err();
    assert!(refused.message.contains("are files of the project as people wrote them"), "{}", refused.message);
}

#[tokio::test]
async fn a_column_is_made_another_kind_where_every_row_fits_and_every_row_is_still_written() {
    let rig = shop("local").await;
    let collections = &rig.studio.collections;
    collections.add_column("shop", "tasks", column("Notes", "text", &[]), &ada()).await.unwrap();
    let basket = add(collections, json!({ "title": "Basket", "notes": "2026-11-30" })).await;
    let checkout = add(collections, json!({ "title": "Checkout", "notes": "soon" })).await;
    // A cell that was emptied is kept as empty text, and says nothing.
    let returns = add(collections, json!({ "title": "Returns", "notes": "" })).await;
    let kind = |kind: &'static str, of: &'static [&'static str]| async move {
        let change = Change::Kind { kind: kind.into(), of: words(of) };
        collections.change_column("shop", "tasks", "notes", change, &ada()).await
    };

    let refused = kind("date", &[]).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert_eq!(
        refused.message,
        "`notes` holds a `text` and 1 record(s) of `tasks` say something there that a `date` does not take: \
         change or empty those first, and then make it one"
    );
    assert_eq!(collections.get("shop", "tasks").await.unwrap().field("notes").unwrap().kind, "text");
    // The one that did not fit is changed, and then it is a date.
    let mut dated = collections.record(&checkout.id).await.unwrap().1;
    dated.data.insert("notes".into(), json!("2026-12-01"));
    let table = collections.get("shop", "tasks").await.unwrap();
    collections.put(&table, dated).await.unwrap();
    let table = kind("date", &[]).await.unwrap();
    assert_eq!(table.field("notes").unwrap().kind, "date");
    // Every row still reads, and is still written whole: the one that was emptied too.
    for id in [&basket.id, &checkout.id, &returns.id] {
        let mut row = collections.record(id).await.unwrap().1;
        row.data.insert("size".into(), json!(1));
        collections.put(&table, row).await.unwrap_or_else(|refused| panic!("{id}: {}", refused.message));
    }
    assert_eq!(held(collections, &basket.id, "notes").await, json!("2026-11-30"));
    assert_eq!(held(collections, &returns.id, "notes").await, Value::Null);

    // A choice of what the rows say, and not of anything else.
    assert!(kind("choice", &["2026-11-30"]).await.is_err());
    let table = kind("choice", &["2026-11-30", "2026-12-01", "later"]).await.unwrap();
    assert_eq!(table.field("notes").unwrap().of, ["2026-11-30", "2026-12-01", "later"]);
    // A view laid out by it goes with what it was laid out by, as where a
    // declaration takes a choice away; and never the table's last view.
    let by = View { name: "Lanes".into(), kind: "board".into(), by: Some("notes".into()), ..View::default() };
    collections.save_view("shop", "tasks", by.clone(), &ada()).await.unwrap();
    let table = kind("text", &[]).await.unwrap();
    assert_eq!(table.views.iter().map(|view| view.name.as_str()).collect::<Vec<_>>(), ["rows"]);
    kind("choice", &["2026-11-30", "2026-12-01"]).await.unwrap();
    collections.save_view("shop", "tasks", by, &ada()).await.unwrap();
    collections.remove_view("shop", "tasks", "rows", &ada()).await.unwrap();
    let refused = kind("text", &[]).await.unwrap_err();
    assert!(refused.message.contains("is the only view the table has"), "{}", refused.message);
    let refused = collections.remove_column("shop", "tasks", "notes", &ada()).await.unwrap_err();
    assert!(refused.message.contains("is the only view the table has"), "{}", refused.message);
    assert_eq!(collections.get("shop", "tasks").await.unwrap().field("notes").unwrap().kind, "choice");
    let told: Vec<String> = rig
        .events("collection.fields")
        .await
        .iter()
        .map(|event| event.data["did"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(told, ["added", "retyped", "retyped", "retyped", "retyped"]);
}

#[tokio::test]
async fn what_a_choice_somebody_made_offers_is_added_put_in_order_called_something_else_and_taken_away() {
    for store in ["local", "files"] {
        let rig = shop(store).await;
        let collections = &rig.studio.collections;
        collections
            .add_column("shop", "tasks", column("Mood", "choice", &["fine", "rushed"]), &ada())
            .await
            .unwrap();
        let basket = add(collections, json!({ "title": "Basket", "mood": "rushed" })).await;
        let offer = |of: &'static [&'static str]| async move {
            collections.change_column("shop", "tasks", "mood", Change::Options(words(of)), &ada()).await
        };
        let rename = |from: &'static str, to: &'static str| async move {
            let change = Change::Renamed { from: from.into(), to: to.into() };
            collections.change_column("shop", "tasks", "mood", change, &ada()).await
        };
        // Added to, and put in another order.
        assert_eq!(
            offer(&["calm", "fine", "rushed"]).await.unwrap().field("mood").unwrap().of,
            ["calm", "fine", "rushed"],
            "{store}"
        );
        assert_eq!(
            offer(&["rushed", "calm", "fine"]).await.unwrap().field("mood").unwrap().of,
            ["rushed", "calm", "fine"],
            "{store}"
        );
        // One that a row still chooses stays until the row says something else.
        let refused = offer(&["calm", "fine"]).await.unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Conflict, "{store}");
        assert_eq!(
            refused.message,
            "`rushed` is still the `mood` of 1 record(s) of `tasks`; change them before taking it away"
        );
        assert_eq!(
            offer(&["rushed", "fine"]).await.unwrap().field("mood").unwrap().of,
            ["rushed", "fine"],
            "{store}"
        );
        assert!(offer(&[]).await.is_err(), "{store}: a choice offers something");

        // Called something else: where the rows are this database's, those
        // that chose it say the new word in the same write, and nothing is
        // said of each; a view held to it is held to the new word.
        let held_to = View {
            name: "Rushed".into(),
            kind: "table".into(),
            only: Some([("mood".to_string(), words(&["rushed", "fine"]))].into()),
            ..View::default()
        };
        collections.save_view("shop", "tasks", held_to, &ada()).await.unwrap();
        for (from, to) in [("nothing", "x"), ("rushed", "fine"), ("rushed", " ")] {
            assert!(rename(from, to).await.is_err(), "{store}: {from} to {to}");
        }
        let before = rig.events("record.*").await.len();
        if store == "local" {
            let table = rename("rushed", "In a Hurry").await.unwrap();
            assert_eq!(table.field("mood").unwrap().of, ["in-a-hurry", "fine"]);
            assert_eq!(held(collections, &basket.id, "mood").await, json!("in-a-hurry"));
            let view = table.views.iter().find(|view| view.name == "Rushed").unwrap();
            assert_eq!(view.only.as_ref().unwrap()["mood"], ["in-a-hurry", "fine"]);
            assert_eq!(rig.events("record.*").await.len(), before);
            // The row is written whole afterwards, as any row is.
            let mut row = collections.record(&basket.id).await.unwrap().1;
            row.data.insert("size".into(), json!(2));
            collections.put(&table, row).await.unwrap();
        } else {
            // Kept as files, each row would be a write of its own: it is said how to get there.
            let refused = rename("rushed", "hurried").await.unwrap_err();
            assert!(refused.message.contains("1 record(s) of `tasks` chose `rushed`"), "{}", refused.message);
            // One no row chose is called something else wherever the rows are.
            assert_eq!(rename("fine", "calm").await.unwrap().field("mood").unwrap().of, ["rushed", "calm"]);
        }
        // An option that goes is taken out of what a view is held to.
        let (gone, stays) = if store == "local" { ("fine", "in-a-hurry") } else { ("calm", "rushed") };
        let change = Change::Options(words(&[stays]));
        let table = collections.change_column("shop", "tasks", "mood", change, &ada()).await.unwrap();
        let view = table.views.iter().find(|view| view.name == "Rushed").unwrap();
        assert_eq!(view.only.as_ref().unwrap()["mood"], [stays], "{store}: {gone} went");
    }
}

#[tokio::test]
async fn a_column_taken_away_goes_with_what_the_rows_held_and_one_added_begins_empty() {
    for store in ["local", "files"] {
        let rig = shop(store).await;
        let collections = &rig.studio.collections;
        collections.add_column("shop", "tasks", column("Notes", "text", &[]), &ada()).await.unwrap();
        let basket = add(collections, json!({ "title": "Basket", "notes": "fragile", "size": 3 })).await;
        // What views said of it goes with it.
        let view = View {
            name: "Mine".into(),
            kind: "table".into(),
            group: Some("notes".into()),
            hides: Some(words(&["notes", "size"])),
            only: Some([("notes".to_string(), words(&["fragile"]))].into()),
            shown: kernel::columns::Columns {
                order: Some(words(&["notes", "size"])),
                freezes: Some("notes".into()),
                figures: Some([("notes".to_string(), "filled".to_string())].into()),
                widths: Some([("notes".to_string(), 240), ("size".to_string(), 90)].into()),
            },
            ..View::default()
        };
        collections.save_view("shop", "tasks", view, &ada()).await.unwrap();

        let table = collections.remove_column("shop", "tasks", "notes", &ada()).await.unwrap();
        assert_eq!(names(&table), ["size", "stage", "title"], "{store}");
        let mine = table.views.iter().find(|view| view.name == "Mine").unwrap();
        assert_eq!(
            (mine.group.as_deref(), mine.only.as_ref(), mine.shown.freezes.as_deref()),
            (None, None, None),
            "{store}"
        );
        assert_eq!(
            (mine.hides.as_deref(), mine.shown.order.as_deref()),
            (Some(&words(&["size"])[..]), Some(&words(&["size"])[..]))
        );
        assert_eq!(mine.shown.figures.as_ref().map(|figures| figures.len()), Some(0), "{store}");
        // How wide it was goes, and how wide the others are stays.
        assert_eq!(mine.shown.widths, Some([("size".to_string(), 90)].into()), "{store}");
        // No row is refused for what it held: each is read and written as before.
        let mut row = collections.record(&basket.id).await.unwrap().1;
        assert!(!row.data.contains_key("notes"), "{store}: {:?}", row.data);
        row.data.insert("size".into(), json!(4));
        collections.put(&table, row).await.unwrap();
        let again = collections.remove_column("shop", "tasks", "notes", &ada()).await.unwrap_err();
        assert_eq!(again.kind, ErrorKind::NotFound, "{store}");

        // A column of the name added again is a new one, and begins empty.
        let other = add(collections, json!({ "title": "Checkout" })).await;
        if store == "files" {
            // A file not written since still says it, where people keep it: said, and nothing guessed.
            let path = rig.dir.join("projects/shop/tasks").join(format!("{}.md", other.number));
            let text = std::fs::read_to_string(&path).unwrap();
            std::fs::write(&path, text.replacen("---\n", "---\nnotes: \"left\"\n", 1)).unwrap();
            let refused = collections
                .add_column("shop", "tasks", column("Notes", "number", &[]), &ada())
                .await
                .unwrap_err();
            assert_eq!(refused.kind, ErrorKind::Conflict);
            assert!(
                refused.message.starts_with("1 record(s) of `tasks` already say something under `notes`"),
                "{}",
                refused.message
            );
            std::fs::write(&path, text).unwrap();
        }
        let (table, called) =
            collections.add_column("shop", "tasks", column("Notes", "number", &[]), &ada()).await.unwrap();
        assert_eq!(called, "notes");
        for id in [&basket.id, &other.id] {
            assert_eq!(held(collections, id, "notes").await, Value::Null, "{store}");
            let mut row = collections.record(id).await.unwrap().1;
            row.data.insert("notes".into(), json!(7));
            collections.put(&table, row).await.unwrap();
            assert_eq!(held(collections, id, "notes").await, json!(7), "{store}");
        }
    }
}

#[tokio::test]
async fn a_column_is_added_where_it_is_put_in_a_view_and_copied_without_what_its_rows_hold() {
    let rig = shop("local").await;
    let collections = &rig.studio.collections;
    let basket = add(collections, json!({ "title": "Basket", "stage": "old" })).await;
    // Put before a column of the blueprint's view: the view is somebody's from
    // then on, and says what it leaves out as one of theirs does.
    let place = Place {
        view: "rows".into(),
        order: words(&["title", "stage", "size", "ghost"]),
        before: Some("size".into()),
        ..Place::default()
    };
    let new = New { place: Some(place), ..column("Notes", "text", &[]) };
    let (table, _) = collections.add_column("shop", "tasks", new, &ada()).await.unwrap();
    let rows = table.views.iter().find(|view| view.name == "rows").unwrap();
    assert_eq!(rows.shown.order.as_deref(), Some(&words(&["title", "stage", "notes", "size"])[..]));
    assert!(rows.saved && rows.hides.is_some());
    // Both are told, of one write.
    let told = rig.topics("collection.*").await;
    assert_eq!(&told[told.len() - 2..], ["collection.fields", "collection.views"]);
    // A view that is not there is no place, and the column is added all the same.
    let nowhere = Place { view: "gone".into(), ..Place::default() };
    let new = New { place: Some(nowhere), ..column("Weight", "number", &[]) };
    let (table, _) = collections.add_column("shop", "tasks", new, &ada()).await.unwrap();
    assert!(table.field("weight").is_some());
    // A table that has no view is shown by one of its own name until somebody saves one:
    // a column put somewhere in it keeps that view, as saving it would; one that could
    // not be laid out so is no view, and the column is added all the same.
    let bare =
        Spec::read(&json!({ "title": { "type": "text" }, "size": { "type": "number" } }), &json!(null), None);
    collections.ensure("shop", "bare", bare.unwrap(), None, &ada()).await.unwrap();
    let odd = Place { view: "bare".into(), kind: "board".into(), ..Place::default() };
    let (table, _) = collections
        .add_column("shop", "bare", New { place: Some(odd), ..column("Odd", "text", &[]) }, &ada())
        .await
        .unwrap();
    assert!(table.views.is_empty(), "{:?}", table.views);
    let first = Place {
        view: "bare".into(),
        kind: "table".into(),
        order: words(&["title", "size", "odd"]),
        before: Some("size".into()),
    };
    let (table, _) = collections
        .add_column("shop", "bare", New { place: Some(first), ..column("Notes", "text", &[]) }, &ada())
        .await
        .unwrap();
    assert_eq!(table.views.len(), 1);
    let view = &table.views[0];
    assert_eq!((view.name.as_str(), view.kind.as_str(), view.saved), ("bare", "table", true));
    assert_eq!(view.shown.order.as_deref(), Some(&words(&["title", "notes", "size", "odd"])[..]));

    // A copy holds the same kind of thing and offers the same, and none of what the rows hold;
    // of a declared column too, with nothing of what that one guards or is needed for.
    let copy = New { like: Some("stage".into()), ..New::default() };
    let (table, called) = collections.add_column("shop", "tasks", copy.clone(), &ada()).await.unwrap();
    let field = table.field(&called).unwrap();
    assert_eq!(
        (called.as_str(), heading(field).as_str(), field.kind.as_str()),
        ("stage_copy", "Stage copy", "choice")
    );
    assert_eq!(
        (field.of.clone(), field.added, field.required),
        (words(&["new", "old", "gone"]), true, false)
    );
    assert_eq!(held(collections, &basket.id, "stage_copy").await, Value::Null);
    let (_, called) = collections.add_column("shop", "tasks", copy, &ada()).await.unwrap();
    assert_eq!(called, "stage_copy_2");
    let title = New { like: Some("title".into()), label: "Subtitle".into(), ..New::default() };
    let (table, called) = collections.add_column("shop", "tasks", title, &ada()).await.unwrap();
    assert!(!table.field(&called).unwrap().required, "what a record must have is its table's to say");
    let missing = New { like: Some("nothing".into()), ..New::default() };
    assert_eq!(
        collections.add_column("shop", "tasks", missing, &ada()).await.unwrap_err().kind,
        ErrorKind::NotFound
    );
}

#[tokio::test]
async fn a_boards_columns_arranged_and_its_status_put_in_order_are_one_order_that_a_declaration_keeps() {
    let rig = Rig::new().await;
    rig.board("Shop").await;
    let (collections, board) = (&rig.studio.collections, &rig.studio.board);
    let usual = || studio::board::spec(&words(studio::board::DEFAULT_COLUMNS), studio::Store::Local);
    assert_eq!(board.columns("shop").await.unwrap(), studio::board::DEFAULT_COLUMNS);

    // Put in order from the column's own menu: the board's columns are in that order, and it is said as that.
    let order = words(&["done", "review", "doing", "ready", "draft"]);
    collections
        .change_column("shop", "board", "status", Change::Options(order.clone()), &ada())
        .await
        .unwrap();
    assert_eq!(board.columns("shop").await.unwrap(), order);
    let told = rig.events("board.columns").await;
    assert_eq!(told.last().unwrap().data, json!({ "project": "shop", "columns": order }));
    // The board's blueprint applied again says its own order, and theirs stands.
    collections.ensure("shop", "board", usual(), None, &ada()).await.unwrap();
    assert_eq!(board.columns("shop").await.unwrap(), order);

    // Arranged from the board's page, with a column more: the same order, kept the same way.
    let arranged = words(&["draft", "ready", "testing", "doing", "review", "done"]);
    board.set_columns("shop", arranged.clone(), &ada()).await.unwrap();
    assert_eq!(board.columns("shop").await.unwrap(), arranged);
    let status = collections.get("shop", "board").await.unwrap().field("status").unwrap().clone();
    assert_eq!((status.of_chosen, status.added), (arranged.clone(), false));
    assert_eq!(status.of_added, Some(words(&["testing"])));
    // A column somebody added is theirs, as a column of a table is: said
    // again without it, the board keeps it where it was, and keeps it
    // through another arranging and a declaration after that.
    collections.ensure("shop", "board", usual(), None, &ada()).await.unwrap();
    assert_eq!(board.columns("shop").await.unwrap(), arranged);
    // A declaration that offers one more puts it after the one it follows
    // there; one that offers one less takes away only what was its.
    let more = || {
        studio::board::spec(
            &words(&["draft", "ready", "doing", "review", "staging", "done"]),
            studio::Store::Local,
        )
    };
    collections.ensure("shop", "board", more(), None, &ada()).await.unwrap();
    assert_eq!(
        board.columns("shop").await.unwrap(),
        words(&["draft", "ready", "testing", "doing", "review", "staging", "done"])
    );
    let less = || studio::board::spec(&words(&["draft", "ready", "doing", "done"]), studio::Store::Local);
    collections.ensure("shop", "board", less(), None, &ada()).await.unwrap();
    assert_eq!(board.columns("shop").await.unwrap(), words(&["draft", "ready", "testing", "doing", "done"]));
    // Arranged without it, it goes: arranging says the whole of the board's columns.
    board.set_columns("shop", words(studio::board::DEFAULT_COLUMNS), &ada()).await.unwrap();
    collections.ensure("shop", "board", usual(), None, &ada()).await.unwrap();
    assert_eq!(board.columns("shop").await.unwrap(), words(studio::board::DEFAULT_COLUMNS));
    collections
        .change_column("shop", "board", "status", Change::Options(order.clone()), &ada())
        .await
        .unwrap();
    collections.ensure("shop", "board", usual(), None, &ada()).await.unwrap();
    assert_eq!(board.columns("shop").await.unwrap(), order);

    // A card's own fields are the model's, which this program reads: none is somebody's to
    // reshape, and a column somebody adds to a board is kept through a card being moved.
    for field in ["title", "status", "assignee", "moved"] {
        let number = Change::Kind { kind: "number".into(), of: Vec::new() };
        let refused = collections.change_column("shop", "board", field, number, &ada()).await.unwrap_err();
        assert!(refused.message.contains("is declared with the table"), "{}", refused.message);
    }
    let refused = collections.remove_column("shop", "board", "status", &ada()).await.unwrap_err();
    assert!(refused.message.contains("a `card` has it, and this program reads it"), "{}", refused.message);
    collections.add_column("shop", "board", column("Estimate", "number", &[]), &ada()).await.unwrap();
    let card = board.add("shop", "Basket", "", None, Vec::new(), &ada()).await.unwrap();
    let sized = studio::Changes { more: data(json!({ "estimate": 3 })), ..studio::Changes::default() };
    board.update(&card.id, sized, &ada()).await.unwrap();
    let moved = board.move_to(&card.id, "done", None, &ada()).await.unwrap();
    assert_eq!(moved.more.get("estimate"), Some(&json!(3)));
    board.set_columns("shop", arranged, &ada()).await.unwrap();
    assert_eq!(board.get(&card.id).await.unwrap().more.get("estimate"), Some(&json!(3)));
    assert!(collections.get("shop", "board").await.unwrap().field("estimate").unwrap().added);
}

#[tokio::test(flavor = "multi_thread")]
async fn columns_added_beside_a_view_saved_an_icon_chosen_and_the_table_declared_are_all_kept() {
    let rig = shop("local").await;
    let collections = &rig.studio.collections;
    // Each is a part of the one row a table is kept as: written whole
    // from what was read a moment before, the last would take the others away.
    let who = ada();
    for round in 0..8 {
        let (icon, view) = (["star", "heart", "moon"][round % 3], ["Mine", "Yours"][round % 2]);
        let added = format!("Extra {round}");
        let saved = View { name: view.into(), kind: "list".into(), ..View::default() };
        let label = Change::Label(Some(format!("Stage {round}")));
        let all = tokio::join!(
            collections.add_column("shop", "tasks", column(&added, "text", &[]), &who),
            collections.save_view("shop", "tasks", saved, &who),
            collections.choose_icon("shop", "tasks", "size", Some(icon), &who),
            collections.change_column("shop", "tasks", "stage", label, &who),
            tasks(collections, serde_json::from_str(FIELDS).unwrap(), "local"),
        );
        let (added, viewed, chosen, called, declared) = all;
        for (what, refused) in [
            ("added", added.err()),
            ("saved", viewed.err()),
            ("chosen", chosen.err()),
            ("called", called.err()),
            ("declared", declared.err()),
        ] {
            assert!(refused.is_none(), "round {round}, {what}: {refused:?}");
        }
        let table = collections.get("shop", "tasks").await.unwrap();
        for before in 0..=round {
            assert!(
                table.field(&format!("extra_{before}")).is_some_and(|field| field.added),
                "round {round}: {:?}",
                names(&table)
            );
        }
        assert_eq!(table.field("size").unwrap().icon_chosen.as_deref(), Some(icon), "round {round}");
        assert_eq!(
            table.field("stage").unwrap().label_chosen,
            Some(format!("Stage {round}")),
            "round {round}"
        );
    }
}
