//! How a project's things are arranged in the side panel, over HTTP
//! (`outline`): changed one operation at a time by whoever may write the
//! project's files, read as whoever reads it may see it, and taken into
//! a copy of the factory; and things put in a folder of it by a blueprint
//! (`outline.put`), the same done twice.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

const OUTLINE: &str = "/api/projects/shop/outline";

/// Two tables of Shop's: its runs, and a diary kept private.
const TABLES: &str = r#"
blueprint: tables
permissions: [projects]
steps:
  runs:
    do: collection.ensure
    with:
      name: runs
      fields: { title: { type: text, required: true } }
  diary:
    do: collection.ensure
    with:
      name: diary
      reads: private:journal
      fields: { title: { type: text, required: true } }
"#;

async fn shop(rig: &Rig) {
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let applied = rig.send("/api/apply", json!({ "source": TABLES, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
}

async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "helper", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

/// The owner's Insights, with both tables in it.
async fn arranged(rig: &Rig) {
    let made =
        rig.send(OUTLINE, json!({ "op": "make", "name": "Insights", "seen": ["table:runs", "repo"] })).await;
    assert_eq!(made["done"], json!({ "made": "f1" }), "{made}");
    let put = json!({ "op": "put", "nodes": ["table:runs", "table:diary"], "into": "f1" });
    let put = rig.send(OUTLINE, put).await;
    assert_eq!((put["changed"].clone(), put["nth"].clone()), (json!(true), json!(2)), "{put}");
}

#[tokio::test]
async fn an_outline_is_read_as_whoever_asks_may_see_it_and_changed_only_by_whoever_may_write_the_projects_files()
 {
    let rig = Rig::new().await;
    shop(&rig).await;
    assert_eq!(rig.read(OUTLINE).await["outline"], Value::Null, "nobody arranged it");
    arranged(&rig).await;
    assert_eq!(
        rig.read(OUTLINE).await["outline"]["items"],
        json!([
            { "thing": "repo" },
            { "folder": "f1", "name": "Insights", "items": [{ "thing": "table:runs" }, { "thing": "table:diary" }] },
        ])
    );

    // A key that holds what an agent usually holds reads it without the diary, and the folder all the same.
    let helper = key(&rig, kernel::principal::AGENT_SCOPES).await;
    let (status, read) = rig.call(Method::GET, OUTLINE, Some(&helper), None).await;
    assert_eq!(status, StatusCode::OK, "{read}");
    assert_eq!(read["outline"]["items"][1]["items"], json!([{ "thing": "table:runs" }]));
    // It may not place the diary, and is told so as of something that is not there.
    let body = json!({ "op": "put", "nodes": ["table:diary"] });
    let (status, refused) = rig.call(Method::POST, OUTLINE, Some(&helper), Some(body)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    // What it does change leaves the diary where it is.
    let body = json!({ "op": "rename", "folder": "f1", "name": "Runs" });
    let (status, renamed) = rig.call(Method::POST, OUTLINE, Some(&helper), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{renamed}");
    assert_eq!(rig.read(OUTLINE).await["outline"]["items"][1]["items"][1], json!({ "thing": "table:diary" }));

    // A key that only reads projects reads it, and changes nothing.
    let reader = key(&rig, &["projects:read"]).await;
    let (status, _) = rig.call(Method::GET, OUTLINE, Some(&reader), None).await;
    assert_eq!(status, StatusCode::OK);
    let body = json!({ "op": "make", "name": "Mine" });
    let (status, refused) = rig.call(Method::POST, OUTLINE, Some(&reader), Some(body)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");

    // What is refused is said in its sentence, and nothing is written.
    let (status, refused) = rig.post(OUTLINE, json!({ "op": "make", "name": "runs" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(
        refused["error"]["message"].as_str().unwrap().contains("Two folders here are called runs"),
        "{refused}"
    );
    assert_eq!(rig.read(OUTLINE).await["outline"]["nth"], 3);
}

#[tokio::test]
async fn how_a_projects_things_are_arranged_goes_into_a_copy_of_the_factory() {
    let shared = crate::snapshots::Shared::new();
    let prod = shared.factory("prod").await;
    shop(&prod).await;
    arranged(&prod).await;
    prod.send(OUTLINE, json!({ "op": "name", "thing": "repo", "name": "Code" })).await;
    let to = shared.snapshot("first");
    crate::snapshots::taken(&prod, &to).await;
    let copy = shared.copy("staging", &to).await;
    assert_eq!(copy.read(OUTLINE).await["outline"], prod.read(OUTLINE).await["outline"]);
    assert_eq!(copy.read(OUTLINE).await["outline"]["items"][0], json!({ "thing": "repo", "name": "Code" }));
}

#[tokio::test]
async fn a_project_in_the_trash_keeps_its_outline_and_is_given_it_back_and_one_deleted_for_good_takes_it_with_it()
 {
    let rig = Rig::new().await;
    shop(&rig).await;
    arranged(&rig).await;
    let before = rig.read(OUTLINE).await["outline"].clone();
    let trashed = rig.call(Method::DELETE, "/api/projects/shop", Some(&rig.token), None).await.1;
    let trash = trashed["trash"].as_str().unwrap().to_string();
    rig.send(&format!("/api/trash/{trash}/restore"), json!({})).await;
    assert_eq!(rig.read(OUTLINE).await["outline"], before, "given back with the project");

    let trashed = rig.call(Method::DELETE, "/api/projects/shop", Some(&rig.token), None).await.1;
    let trash = trashed["trash"].as_str().unwrap().to_string();
    let (status, gone) =
        rig.call(Method::DELETE, &format!("/api/trash/{trash}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    let kept = rig.node().app.journal.kv_get(kernel::outline::NAMESPACE, "shop").await.unwrap();
    assert_eq!(kept, None, "deleted for good, its outline goes with it");
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    assert_eq!(rig.read(OUTLINE).await["outline"], Value::Null, "made anew, nobody has arranged it");
}

/// Two libraries of Shop's: its guides, and days kept private.
const LIBRARIES: &str = r#"
blueprint: libraries
permissions: [projects]
steps:
  guides:
    do: collection.ensure
    with:
      name: guides
      store: documents
      at: notes/guides
      fields: { title: { type: text, required: true }, body: { type: text }, path: { type: text } }
  days:
    do: collection.ensure
    with:
      name: days
      store: documents
      at: notes/days
      reads: private:journal
      fields: { title: { type: text, required: true }, body: { type: text }, path: { type: text } }
"#;

#[tokio::test]
async fn a_thing_is_shown_as_what_somebody_named_it_kept_for_everybody_who_may_read_it_and_cleared() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let body = json!({ "source": LIBRARIES, "project": "shop", "key": "libraries", "wait": true });
    let applied = rig.send("/api/apply", body).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");

    // Named from a page that drew the guides, the days and the repo: kept where the page drew them.
    let seen = json!(["library:guides", "library:days", "repo"]);
    let named = rig
        .send(OUTLINE, json!({ "op": "name", "thing": "library:guides", "name": " Handbook ", "seen": seen }))
        .await;
    assert_eq!((named["changed"].clone(), named["nth"].clone()), (json!(true), json!(1)), "{named}");
    rig.send(OUTLINE, json!({ "op": "name", "thing": "library:days", "name": "Plans for May" })).await;
    assert_eq!(
        rig.read(OUTLINE).await["outline"]["items"],
        json!([
            { "thing": "library:guides", "name": "Handbook" },
            { "thing": "library:days", "name": "Plans for May" },
            { "thing": "repo" },
        ])
    );
    // Nothing anything else reads it by changes: the library is still `guides`.
    let listed = rig.read("/api/collections?project=shop").await;
    let names: Vec<&str> =
        listed["collections"].as_array().unwrap().iter().filter_map(|one| one["name"].as_str()).collect();
    assert!(names.contains(&"guides") && names.contains(&"days"), "{listed}");

    // A key that may not read the days is handed the guides' name and never the days'.
    let helper = key(&rig, kernel::principal::AGENT_SCOPES).await;
    let (status, read) = rig.call(Method::GET, OUTLINE, Some(&helper), None).await;
    assert_eq!(status, StatusCode::OK, "{read}");
    assert_eq!(
        read["outline"]["items"],
        json!([{ "thing": "library:guides", "name": "Handbook" }, { "thing": "repo" }])
    );
    assert!(!read.to_string().contains("Plans for May"), "{read}");
    // Nor may it name them, told so as of something that is not there; what it may read it names.
    let body = json!({ "op": "name", "thing": "library:days", "name": "Mine" });
    let (status, refused) = rig.call(Method::POST, OUTLINE, Some(&helper), Some(body)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    let body = json!({ "op": "name", "thing": "repo", "name": "Code" });
    let (status, answered) = rig.call(Method::POST, OUTLINE, Some(&helper), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{answered}");
    assert!(!answered.to_string().contains("Plans for May"), "{answered}");
    // A key that only reads projects names nothing.
    let reader = key(&rig, &["projects:read"]).await;
    let body = json!({ "op": "name", "thing": "repo", "name": "Mine" });
    let (status, _) = rig.call(Method::POST, OUTLINE, Some(&reader), Some(body)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // A table is called something else by its title, and is not named here.
    let (status, refused) =
        rig.post(OUTLINE, json!({ "op": "name", "thing": "table:runs", "name": "Runs" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(refused["error"]["message"].as_str().unwrap().contains("rename the table itself"), "{refused}");

    // Cleared, by nothing or by only spaces, it is shown as it always was, and said once each.
    let cleared = rig.send(OUTLINE, json!({ "op": "name", "thing": "library:guides", "name": null })).await;
    assert_eq!(cleared["outline"]["items"][0], json!({ "thing": "library:guides" }), "{cleared}");
    rig.send(OUTLINE, json!({ "op": "name", "thing": "repo", "name": "   " })).await;
    let again = rig.send(OUTLINE, json!({ "op": "name", "thing": "repo" })).await;
    assert_eq!(again["changed"], false, "{again}");
    let outline = rig.read(OUTLINE).await["outline"].clone();
    assert_eq!(outline["items"][2], json!({ "thing": "repo" }), "{outline}");
    assert_eq!(outline["nth"], 5, "each change written once and nothing for what changed nothing: {outline}");
    let events = rig.read("/api/events/recent?topics=project.arranged&limit=50").await;
    let said = events["events"].as_array().map_or(0, |all| all.len());
    assert_eq!(said, 5, "{events}");
}

/// The runs put in Insights/Runs of whichever project it is applied for.
const SHOWN: &str = r#"
blueprint: shown
permissions: [projects]
steps:
  shown:
    do: outline.put
    with: { things: ["table:runs"], folder: Insights/Runs }
"#;

#[tokio::test]
async fn a_blueprint_puts_things_in_a_folder_of_the_outline_making_it_and_the_same_done_twice_is_nothing() {
    let rig = Rig::new().await;
    shop(&rig).await;
    // Applied twice, under two keys, so that its step is done twice.
    for key in ["shown", "again"] {
        let body = json!({ "source": SHOWN, "project": "shop", "key": key, "wait": true });
        let applied = rig.send("/api/apply", body).await;
        assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    }
    let outline = rig.read("/api/projects/shop/outline").await["outline"].clone();
    assert_eq!(
        outline["items"],
        json!([{ "folder": "f1", "name": "Insights", "items": [
            { "folder": "f2", "name": "Runs", "items": [{ "thing": "table:runs" }] }
        ] }])
    );
    assert_eq!(outline["nth"], 1, "applied again, nothing was written: {outline}");
}
