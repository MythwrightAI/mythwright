//! A project's outline as the factory keeps it: each change one write
//! with what is said of it, read and changed as whoever asks may see it,
//! and gone with its project.

use kernel::outline::{NAMESPACE, Op};
use kernel::principal::MEMBER_SCOPES;
use kernel::{ErrorKind, Principal};
use serde_json::{Value, json};
use studio::collections::Spec;

use crate::{Rig, ada, person};

fn op(said: Value) -> Op {
    Op::read(&said).unwrap()
}

fn items(rig_outline: Option<kernel::outline::Outline>) -> Value {
    rig_outline.map_or(Value::Null, |outline| outline.to_value()["items"].clone())
}

/// Shop, with a table of runs anybody of the project reads and one kept private.
async fn shop() -> Rig {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let fields = json!({ "title": { "type": "text", "required": true } });
    for (name, reads) in [("runs", None), ("diary", Some("private:journal"))] {
        let mut spec = Spec::read(&fields, &json!(null), None).unwrap();
        spec.reads = reads.map(str::to_string);
        rig.studio.collections.ensure("shop", name, spec, None, &ada()).await.unwrap();
    }
    rig
}

/// A member, who reads the project and not the private table.
fn grace() -> Principal {
    person("Grace", MEMBER_SCOPES, None)
}

#[tokio::test]
async fn each_change_is_one_write_with_what_is_said_of_it_and_one_that_comes_to_nothing_is_none() {
    let rig = shop().await;
    assert_eq!(rig.outlines.read("shop", &ada()).await.unwrap(), None, "nobody arranged it");
    let made =
        rig.outlines.change("shop", op(json!({ "op": "make", "name": "Insights" })), vec![], &ada()).await;
    let made = made.unwrap();
    assert_eq!((made.nth, made.done.clone()), (1, json!({ "made": "f1" })));
    let put = op(json!({ "op": "put", "nodes": ["table:runs"], "into": "f1" }));
    let put = rig.outlines.change("shop", put.clone(), vec![], &ada()).await.unwrap();
    assert_eq!(put.nth, 2);
    assert_eq!(
        items(rig.outlines.read("shop", &ada()).await.unwrap()),
        json!([{ "folder": "f1", "name": "Insights", "items": [{ "thing": "table:runs" }] }])
    );
    let said = rig.events("project.arranged").await;
    assert_eq!(said.len(), 2);
    assert_eq!(said[1].data, json!({ "project": "shop", "nth": 2, "by": "user:ada", "forgot": false }));
    assert!(said[1].data.get("items").is_none(), "what is said carries no outline");

    // Put where it is already, nothing is written and nothing is said.
    let again = op(json!({ "op": "put", "nodes": ["table:runs"], "into": "f1" }));
    let again = rig.outlines.change("shop", again, vec!["board".into()], &ada()).await.unwrap();
    assert_eq!((again.changed, again.nth), (false, 2));
    assert_eq!(rig.events("project.arranged").await.len(), 2);

    // Forgotten, the panel is as it was with none.
    let forgot = rig.outlines.change("shop", Op::Forget, vec![], &ada()).await.unwrap();
    assert_eq!((forgot.outline, forgot.nth), (None, 3));
    assert_eq!(rig.journal.kv_get(NAMESPACE, "shop").await.unwrap(), None);
    assert_eq!(rig.events("project.arranged").await[2].data["forgot"], true);
}

#[tokio::test]
async fn it_is_changed_only_by_whoever_may_write_the_projects_files_and_read_by_whoever_may_read_it() {
    let rig = shop().await;
    let make = op(json!({ "op": "make", "name": "Insights" }));
    let reader = person("Lin", &["projects:read", "records:read"], None);
    let refused = rig.outlines.change("shop", make.clone(), vec![], &reader).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied, "{refused:?}");
    // A key that holds less than its maker is refused as it holds.
    let key = person("Ada", &["projects:read"], Some("key_1"));
    let refused = rig.outlines.change("shop", make.clone(), vec![], &key).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied);
    let nobody = person("Nobody", &["records:read"], None);
    assert_eq!(rig.outlines.read("shop", &nobody).await.unwrap_err().kind, ErrorKind::Denied);
    assert!(rig.events("project.arranged").await.is_empty());
    // A member may: an agent of the project, credited as itself, too.
    let agent = kernel::Principal {
        kind: kernel::PrincipalKind::Agent,
        ..person("shop-worker-1", &["projects"], None)
    };
    rig.outlines.change("shop", make, vec![], &agent).await.unwrap();
    assert_eq!(rig.events("project.arranged").await[0].data["by"], "agent:shop-worker-1");
    assert!(rig.outlines.read("shop", &reader).await.unwrap().is_some());
}

#[tokio::test]
async fn a_table_kept_private_is_out_of_what_somebody_who_may_not_read_it_is_handed_and_their_change_leaves_it_where_it_was()
 {
    let rig = shop().await;
    let made =
        rig.outlines.change("shop", op(json!({ "op": "make", "name": "Insights" })), vec![], &ada()).await;
    assert_eq!(made.unwrap().done["made"], "f1");
    let put = op(json!({ "op": "put", "nodes": ["table:diary", "table:runs"], "into": "f1" }));
    rig.outlines.change("shop", put, vec![], &ada()).await.unwrap();

    // Grace sees the folder and what she may read in it.
    let shown = json!([{ "folder": "f1", "name": "Insights", "items": [{ "thing": "table:runs" }] }]);
    assert_eq!(items(rig.outlines.read("shop", &grace()).await.unwrap()), shown);

    // She may not place it, nor anything that is not there, and is told the same of both.
    let hidden = op(json!({ "op": "put", "nodes": ["table:diary"] }));
    let refused = rig.outlines.change("shop", hidden, vec![], &grace()).await.unwrap_err();
    let missing = op(json!({ "op": "put", "nodes": ["table:nothing"] }));
    let none = rig.outlines.change("shop", missing, vec![], &grace()).await.unwrap_err();
    assert_eq!((refused.kind, none.kind), (ErrorKind::NotFound, ErrorKind::NotFound));
    assert_eq!(refused.message.replace("diary", "nothing"), none.message);
    // What she says she saw is held to what she may see: the private table is not named by it.
    let renamed = op(json!({ "op": "rename", "folder": "f1", "name": "Runs" }));
    rig.outlines.change("shop", renamed, vec!["table:diary".into(), "repo".into()], &grace()).await.unwrap();

    // Removing the folder, she is not told what was in it that she may not read, and it stays in the outline.
    let removed =
        rig.outlines.change("shop", op(json!({ "op": "remove", "folder": "f1" })), vec![], &grace()).await;
    assert_eq!(removed.unwrap().done["removed"]["items"], json!(["table:runs"]));
    assert_eq!(
        items(rig.outlines.read("shop", &ada()).await.unwrap()),
        json!([{ "thing": "table:diary" }, { "thing": "table:runs" }, { "thing": "repo" }])
    );
}

#[tokio::test]
async fn a_project_made_anew_begins_with_no_outline_and_one_deleted_for_good_takes_its_with_it() {
    let rig = shop().await;
    rig.outlines
        .change("shop", op(json!({ "op": "make", "name": "Insights" })), vec![], &ada())
        .await
        .unwrap();
    // Kept in the Trash with it.
    rig.studio.projects.forget("shop", &ada()).await.unwrap();
    assert!(rig.journal.kv_get(NAMESPACE, "shop").await.unwrap().is_some());
    // What a stop between leaves behind under its id is nobody's once one is made anew.
    rig.journal
        .kv_set(NAMESPACE, "riverside", &json!({ "items": [{ "folder": "f1", "name": "Old" }] }))
        .await
        .unwrap();
    rig.studio.projects.create("Riverside", None, &ada()).await.unwrap();
    assert_eq!(rig.outlines.read("riverside", &ada()).await.unwrap(), None);

    // Deleted at once, where nothing is kept in the Trash, it goes in the write that forgets the project.
    rig.outlines
        .change("riverside", op(json!({ "op": "make", "name": "Old" })), vec![], &ada())
        .await
        .unwrap();
    rig.journal.kv_set(studio::trash::KEEPS, "factory", &json!({ "days": 0 })).await.unwrap();
    rig.studio.projects.forget("riverside", &ada()).await.unwrap();
    assert_eq!(rig.journal.kv_get(NAMESPACE, "riverside").await.unwrap(), None);
}

#[tokio::test]
async fn things_put_in_a_folder_by_its_path_make_it_once_and_are_put_as_whoever_asks_may_see() {
    let rig = shop().await;
    let path = |said: &str| said.split('/').map(str::to_string).collect::<Vec<_>>();
    let put =
        rig.outlines.put_in("shop", path("Insights/Runs"), vec!["table:runs".into()], &ada()).await.unwrap();
    assert_eq!((put.changed, put.nth, put.done.clone()), (true, 1, json!({ "folder": "f2" })));
    // Asked again, nothing is written and nothing is said.
    let again =
        rig.outlines.put_in("shop", path("insights/runs"), vec!["table:runs".into()], &ada()).await.unwrap();
    assert_eq!((again.changed, again.nth), (false, 1));
    assert_eq!(rig.events("project.arranged").await.len(), 1);
    // A member may not put what they may not read, told as of something that is not there; nor may a key that only reads.
    let refused = rig.outlines.put_in("shop", path("Insights"), vec!["table:diary".into()], &grace()).await;
    assert_eq!(refused.unwrap_err().kind, ErrorKind::NotFound);
    let reader = person("Rhea", &["projects:read"], Some("k1"));
    let refused = rig.outlines.put_in("shop", path("Insights"), vec!["table:runs".into()], &reader).await;
    assert_eq!(refused.unwrap_err().kind, ErrorKind::Denied);
}
