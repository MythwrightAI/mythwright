//! Where a project's own file under the name of a text that ships stands
//! there: in use, not yet (written outside the factory since what is
//! running was applied), or passed over (the project is not trusted).
//! Worked out from what the runner keeps and kept nowhere, and said in
//! the summary, in what `mw status` lists and in the answer to a save.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// Something of the project's own that stays running and reaches the
/// rule that ships, by naming it as what a param is unless said.
const HOLDER: &str = "\
blueprint: holder
params:
  rule: { type: string, default: std/dispatch }
steps:
  one: { do: noop }
on:
  poked:
    event: holder.poke
    do: noop
";

/// The rule with something else said than what ships says.
fn ours(rig: &Rig) -> String {
    let ships = rig.node().app.engine.library().shipped("std/dispatch", None).unwrap();
    let ours =
        ships.replace("params.agents[0].name ?? ''", "params.agents[1].name ?? params.agents[0].name ?? ''");
    assert_ne!(ours, ships);
    ours
}

const FILE: &str = "@blueprints/std/dispatch.bp";
/// The same, in a folder of a project's repository that it said is read.
const REPO: &str = "blueprints/std/dispatch.bp";

/// Every own file of every project and where it stands, as `mw status` reads it.
async fn listed(rig: &Rig) -> Vec<Value> {
    rig.read("/api/blueprints/stands").await["own"].as_array().cloned().unwrap_or_default()
}

fn lines(summary: &Value, kind: &str) -> Vec<String> {
    let parts = summary["parts"].as_array().cloned().unwrap_or_default();
    let part = parts.iter().find(|part| part["kind"] == kind).cloned().unwrap_or_default();
    part["lines"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|line| line["text"].as_str().map(str::to_string))
        .collect()
}

async fn save(rig: &Rig, project: &str, path: &str, text: &str) -> Value {
    let body = json!({ "path": path, "content": text });
    let at = format!("/api/projects/{project}/file");
    let (status, answer) = rig.call(Method::PUT, &at, Some(&rig.token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    answer
}

#[tokio::test]
async fn a_file_written_outside_the_factory_is_said_to_be_not_in_use_until_it_is_saved_here() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let applied = rig.send("/api/apply", json!({ "source": HOLDER, "project": "shop", "wait": true })).await;
    let id = applied["instance"].as_str().expect("what it started").to_string();
    assert!(listed(&rig).await.is_empty(), "nothing of its own yet");

    // Written by an editor where the factory keeps it: there, and what is running goes on with what ships.
    let folder = rig.placed("shop");
    std::fs::create_dir_all(folder.join("std")).unwrap();
    std::fs::write(folder.join("std/dispatch.bp"), ours(&rig)).unwrap();
    let found = listed(&rig).await;
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        (&found[0]["project"], &found[0]["file"], &found[0]["in_use"]),
        (&json!("shop"), &json!(FILE), &json!(false))
    );
    let said = found[0]["said"].as_str().unwrap();
    assert!(
        said.starts_with(
            "This project's own `@blueprints/std/dispatch.bp`, for `std/dispatch`, is not in use yet"
        ),
        "{said}"
    );
    assert!(said.contains("since `holder` was applied"), "{said}");
    assert!(said.contains(&format!(" apply --instance {id} --take`")), "{said}");

    // The summary of what ships says it first, and so does the file's own.
    let ships = rig.node().app.engine.library().shipped("std/dispatch", None).unwrap();
    let body = json!({ "source": ships, "project": "shop", "ships": true });
    let default = rig.send("/api/blueprints/summary", body).await["summary"].clone();
    assert_eq!(lines(&default, "is")[0], said, "{default}");
    let named =
        rig.send("/api/blueprints/summary", json!({ "name": "std/dispatch", "project": "shop" })).await;
    let is = lines(&named["summary"], "is");
    assert!(is[0].starts_with("Forked from `std/dispatch`"), "{is:?}");
    assert_eq!(is[1], said, "{is:?}");

    // Saved through the factory by whoever applied it: in use, and the save says so.
    let answer = save(&rig, "shop", FILE, &ours(&rig)).await;
    assert_eq!(answer["stands"]["in_use"], true, "{answer}");
    assert_eq!(
        answer["stands"]["said"],
        "`@blueprints/std/dispatch.bp` is in use: it is what runs in this project under `std/dispatch`, in place \
         of the one that ships.",
    );
    let found = listed(&rig).await;
    assert_eq!(found[0]["in_use"], true, "{found:?}");
    let named =
        rig.send("/api/blueprints/summary", json!({ "name": "std/dispatch", "project": "shop" })).await;
    assert!(!lines(&named["summary"], "is").iter().any(|line| line.contains("not in use")), "{named}");

    // A file of the project's that is no name that ships says nothing of where it stands.
    let body = json!({ "path": "notes/plain.md", "content": "words" });
    let (_, answer) = rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
    assert!(answer.get("stands").is_none(), "{answer}");
}

#[tokio::test]
async fn a_file_of_a_project_that_is_not_trusted_is_said_to_be_passed_over_with_the_command_that_trusts_it() {
    let rig = Rig::new().await;
    let folder = rig.dir.join("taken-up");
    std::fs::create_dir_all(folder.join("blueprints/std")).unwrap();
    std::fs::write(folder.join(REPO), ours(&rig)).unwrap();
    rig.send("/api/projects", json!({ "name": "Yard", "path": folder })).await;
    // Read as its own only where it says that folder of its repository is read.
    assert!(listed(&rig).await.is_empty());
    rig.send("/api/projects/yard/blueprints", json!({ "folder": "blueprints" })).await;
    let found = listed(&rig).await;
    assert_eq!(found.len(), 1, "{found:?}");
    let said = found[0]["said"].as_str().unwrap();
    assert!(said.contains("is passed over, and the one that ships is what runs here"), "{said}");
    assert!(said.ends_with(&format!("`{} project trust yard`.", brand::CLI)), "{said}");
    // Its own summary says that first, and nothing of a fork.
    let body = json!({ "source": ours(&rig), "project": "yard", "path": REPO });
    let own = rig.send("/api/blueprints/summary", body).await["summary"].clone();
    assert_eq!(lines(&own, "is")[0], said, "{own}");
    let kinds: Vec<&str> =
        own["parts"].as_array().unwrap().iter().filter_map(|part| part["kind"].as_str()).collect();
    assert!(!kinds.contains(&"differs") && !kinds.contains(&"back"), "{kinds:?}");
    // And saved through the factory it is still passed over, which the save says.
    let answer = save(&rig, "yard", REPO, &ours(&rig)).await;
    assert_eq!(
        (&answer["stands"]["in_use"], &answer["stands"]["said"]),
        (&json!(false), &json!(said)),
        "{answer}"
    );
    // Trusted, it is what runs.
    rig.send("/api/projects/yard/trust", json!({ "trusted": true })).await;
    assert_eq!(listed(&rig).await[0]["in_use"], true);
    // Kept where the factory keeps them, it is never passed over: what is written there was written through the factory.
    rig.send("/api/projects/yard/trust", json!({ "trusted": false })).await;
    rig.call(Method::DELETE, &format!("/api/projects/yard/file?path={REPO}"), Some(&rig.token), None).await;
    save(&rig, "yard", FILE, &ours(&rig)).await;
    let found = listed(&rig).await;
    assert_eq!((&found[0]["file"], &found[0]["in_use"]), (&json!(FILE), &json!(true)), "{found:?}");
}

#[tokio::test]
async fn whoever_may_not_read_projects_is_told_nothing_of_where_their_files_stand() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let narrow = rig.send("/api/credentials", json!({ "name": "narrow", "scopes": ["channels"] })).await;
    let token = narrow["token"].as_str().unwrap();
    let (status, answer) = rig.call(Method::GET, "/api/blueprints/stands", Some(token), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{answer}");
    assert!(answer.to_string().contains("projects:read"), "{answer}");
}
