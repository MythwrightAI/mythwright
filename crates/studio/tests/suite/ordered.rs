//! A view that lays a collection's records out by what comes first, and
//! one that gathers them by what a field names: what a blueprint and a
//! person say for either, kept with the collection, and refused in a
//! sentence where it is written wrongly.

use serde_json::json;
use studio::collections::Spec;

use crate::{Rig, ada};

fn view(kind: &str, by: Option<&str>, done: Option<&str>, group: Option<&str>) -> studio::View {
    studio::View {
        name: "steps".into(),
        kind: kind.into(),
        by: by.map(str::to_string),
        saved: false,
        hides: None,
        group: group.map(str::to_string),
        done: done.map(str::to_string),
        only: None,
        shown: Default::default(),
    }
}

/// A view held to records that hold one of `any` in `field`.
fn held(kind: &str, by: Option<&str>, field: &str, any: &[&str]) -> studio::View {
    let only = [(field.to_string(), any.iter().map(|one| one.to_string()).collect())].into();
    studio::View { only: Some(only), ..view(kind, by, None, None) }
}

#[test]
fn a_blueprint_says_an_order_by_the_field_that_says_what_comes_before_and_what_its_records_are_gathered_by() {
    let fields = json!({
        "title": { "type": "text" },
        "status": { "type": "choice", "of": ["now", "later", "over"] },
        "after": { "type": "list" },
        "of": { "type": "text" },
    });
    let ways = json!({ "steps": { "kind": "order", "by": "after", "done": "status", "group": "of" } });
    let spec = Spec::read(&fields, &ways, None).unwrap();
    assert_eq!(spec.views, [view("order", Some("after"), Some("status"), Some("of"))]);
    // And what a view is held to: the things a record is to hold one of in a field, a list or one alone.
    let ways =
        json!({ "steps": { "kind": "board", "by": "status", "only": { "after": ["a", 2], "of": "shop" } } });
    let spec = Spec::read(&fields, &ways, None).unwrap();
    assert_eq!(
        serde_json::to_value(&spec.views[0].only).unwrap(),
        json!({ "after": ["a", "2"], "of": ["shop"] })
    );
    let nothing = json!({ "steps": { "kind": "table", "only": {} } });
    assert_eq!(Spec::read(&fields, &nothing, None).unwrap().views[0].only, None);
    for wrong in
        [json!("labels"), json!(["labels"]), json!({ "of": { "is": "shop" } }), json!({ "of": [["shop"]] })]
    {
        let said = Spec::read(&fields, &json!({ "steps": { "kind": "table", "only": wrong } }), None)
            .unwrap_err()
            .to_string();
        assert!(
            said.contains(
                "the view `steps` says `only`, which is a map from a field to what a record is to hold there"
            ),
            "{said}"
        );
    }
    // One that says neither says neither, and is written down so.
    let plain = Spec::read(&fields, &json!({ "steps": { "kind": "order", "by": "after" } }), None).unwrap();
    assert_eq!(
        serde_json::to_value(&plain.views[0]).unwrap(),
        json!({ "name": "steps", "kind": "order", "by": "after", "saved": false })
    );
}

#[tokio::test]
async fn an_order_and_a_band_are_kept_with_the_collection_and_refused_where_they_name_the_wrong_thing() {
    let rig = Rig::new().await;
    let project = "shop".to_string();
    rig.board(&project).await;
    let collections = &rig.studio.collections;
    let fields = json!({
        "title": { "type": "text", "required": true },
        "status": { "type": "choice", "of": ["now", "later", "over"] },
        "after": { "type": "list" },
        "of": { "type": "text" },
        "who": { "type": "actor" },
        "due": { "type": "date" },
        "labels": { "type": "list" },
    });
    let spec = Spec::read(&fields, &json!(null), None).unwrap();
    collections.ensure(&project, "tasks", spec, None, &ada()).await.unwrap();
    let ada = ada();
    let save = |view: studio::View| collections.save_view(&project, "tasks", view, &ada);
    let refused = |view: studio::View| async { save(view).await.unwrap_err().message };

    // By what a task waits on, done by its choice, a band for what it is of.
    let kept = save(view("order", Some("after"), Some("status"), Some("of"))).await.unwrap();
    let steps = kept.views.iter().find(|one| one.name == "steps").unwrap();
    assert_eq!(
        (steps.by.as_deref(), steps.done.as_deref(), steps.group.as_deref(), steps.saved),
        (Some("after"), Some("status"), Some("of"), true)
    );
    // An order with nothing said of which is done, or of bands, is one all the same.
    save(view("order", Some("after"), None, None)).await.unwrap();
    // And a board is gathered by who has each.
    save(view("board", Some("status"), None, Some("who"))).await.unwrap();

    // An order goes by a field that can say which records come before.
    for by in [None, Some("title"), Some("status"), Some("nowhere")] {
        let said = refused(view("order", by, None, None)).await;
        assert!(
            said.contains(
                "the view `steps` is an order and needs `by`: the field in which a record says which others come before it"
            ),
            "{said}"
        );
    }
    // What says a record is done is a choice, and is an order's to say.
    let wrongly = [
        ("order", Some("after"), "title"),
        ("order", Some("after"), "nowhere"),
        ("board", Some("status"), "status"),
    ];
    for (kind, by, done) in wrongly {
        let said = refused(view(kind, by, Some(done), None)).await;
        let wanted = format!(
            "says `done: {done}`, which is for an order and names the choice field whose last value says a record is done"
        );
        assert!(said.contains(&wanted), "{said}");
    }
    // Records are gathered by a field that names something, in a board, an order or a table.
    let wrongly =
        [("order", Some("after"), "due"), ("board", Some("status"), "nowhere"), ("gallery", None, "who")];
    for (kind, by, group) in wrongly {
        let said = refused(view(kind, by, None, Some(group))).await;
        let wanted = format!(
            "says `group: {group}`, which is for a board, an order or a table and names the field its records are gathered by: a `record`, `records`, `choice`, `actor`, `text` or `list`"
        );
        assert!(said.contains(&wanted), "{said}");
    }
    // A table's rows are gathered under a line for each thing the field says.
    save(view("table", None, None, Some("who"))).await.unwrap();
    // By a field that holds several things too: a record is then in the band of each.
    save(view("order", Some("after"), None, Some("labels"))).await.unwrap();
    save(view("board", Some("status"), None, Some("labels"))).await.unwrap();

    // A view is held to records that hold one of some things in a field, whatever way it is laid out.
    for kind in [("board", Some("status")), ("order", Some("after")), ("table", None), ("gallery", None)] {
        let kept = save(held(kind.0, kind.1, "labels", &["urgent", "soon"])).await.unwrap();
        let steps = kept.views.iter().find(|one| one.name == "steps").unwrap();
        assert_eq!(
            serde_json::to_value(&steps.only).unwrap(),
            json!({ "labels": ["urgent", "soon"] }),
            "{}",
            kind.0
        );
    }
    save(held("table", None, "status", &["now"])).await.unwrap();
    // To a field that names something, and to at least one thing there.
    for field in ["due", "nowhere"] {
        let said = refused(held("table", None, field, &["x"])).await;
        let wanted = format!(
            "says `only` by `{field}`, which is not a field its records can be held to: a `record`, `records`, `choice`, `actor`, `text` or `list`"
        );
        assert!(said.contains(&wanted), "{said}");
    }
    for any in [&[][..], &[" "][..]] {
        let said = refused(held("table", None, "labels", any)).await;
        assert!(
            said.contains(
                "says `only` by `labels` and nothing a record is to hold there; say at least one thing"
            ),
            "{said}"
        );
    }
    // Nothing that was refused was kept.
    let now = collections.get(&project, "tasks").await.unwrap();
    let steps = now.views.iter().find(|one| one.name == "steps").unwrap();
    assert_eq!(serde_json::to_value(&steps.only).unwrap(), json!({ "status": ["now"] }));
}
