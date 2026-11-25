//! What a told line shows beside its words that is read for whoever
//! reads it (`kernel::parts`, `told::read`): rows of a project's table,
//! chosen by their ids or by what a cell says, with the columns named;
//! the runs of an instance, of what ran under a key and of what ran
//! under an environment; and measurements, each its latest value beside
//! the median of its last few. Each part says the tab of the line's page
//! it is in, and each is read as the reader may read it: a key that holds
//! less is shown none of what it may not read, and told why there.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::kept::key;

/// A table of runs a script keeps, and a table kept private.
const TABLES: &str = r#"blueprint: kept-runs
permissions: [records, projects:write]
steps:
  runs:
    do: collection.ensure
    with:
      name: test-runs
      by_hand: false
      fields:
        title: { type: text, required: true }
        result: { type: choice, of: [passed, failed] }
        seconds: { type: number }
        commit: { type: text }
        printed: { type: text }
  notes:
    do: collection.ensure
    with: { name: notes, reads: "private:notes", fields: { title: { type: text, required: true } } }
"#;

/// What is deployed under an environment.
const DEPLOY: &str = "\
blueprint: deploy
steps:
  loaded: { do: emit, with: { topic: shop.loaded } }
";

/// A text that tells whoever applied it, with these parts.
const TOLD: &str = r#"blueprint: toldtabs
permissions: [inbox]
params:
  parts: { type: list }
steps:
  told:
    do: inbox.tell
    with: { to: applier, says: "The preview is ready to try.", why: preview, parts: "${params.parts}" }
"#;

async fn added(rig: &Rig, table: &str, fields: Value) -> String {
    let row = rig.send(&format!("/api/collections/shop/{table}/records"), json!({ "fields": fields })).await;
    row["id"].as_str().unwrap_or_else(|| panic!("{row}")).to_string()
}

async fn applied(rig: &Rig, body: Value) -> Value {
    let applied = rig.send("/api/apply", body).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    applied
}

/// The parts of `token`'s line of the preview as `token` reads them now.
async fn parts(rig: &Rig, token: &str, line: &str) -> Vec<Value> {
    let (status, read) = rig.call(Method::GET, &format!("/api/inbox/{line}/parts"), Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{read}");
    read["parts"].as_array().unwrap().clone()
}

/// What each row a part lists says in `cell`, or that it is gone.
fn each(part: &Value, cell: &str) -> Vec<Value> {
    let rows = part["rows"].as_array().unwrap_or_else(|| panic!("{part}"));
    rows.iter()
        .map(|row| if row["gone"] == true { json!("gone") } else { row["cells"][cell].clone() })
        .collect()
}

#[tokio::test]
async fn a_told_lines_rows_runs_and_measures_are_read_as_whoever_reads_it_may_each_in_the_tab_it_says() {
    let rig = &Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    rig.send("/api/projects/shop/environments", json!({ "name": "prod" })).await;
    let under = |source: &str, key: &str, params: Value| json!({ "source": source, "project": "shop", "key": key, "environment": "prod", "params": params, "wait": true });
    applied(rig, under(TABLES, "tables", json!({}))).await;
    let mut runs = Vec::new();
    for (title, result, seconds, commit) in [
        ("r-1", "passed", 96, "a1"),
        ("r-2", "failed", 3700, "b2"),
        ("r-3", "passed", 12, "c3"),
        ("r-4", "passed", 40, "a1+"),
    ] {
        let printed = format!("{title} {}", "x".repeat(500));
        let fields = json!({ "title": title, "result": result, "seconds": seconds, "commit": commit, "printed": printed });
        runs.push(added(rig, "test-runs", fields).await);
    }
    let note = added(rig, "notes", json!({ "title": "Kept to some" })).await;
    applied(rig, under(DEPLOY, "deploy-prod", json!({}))).await;
    // Applied again as a deploy is, its steps run again.
    let mut again = under(DEPLOY, "deploy-prod", json!({}));
    again["force"] = json!(true);
    let deployed = applied(rig, again).await;
    let deploy = deployed["instance"].as_str().unwrap().to_string();

    // Told with a part of each kind, each in a tab or in none.
    let said = json!([
        { "kind": "rows", "table": "test-runs", "saying": { "commit": { "in": ["a1", "b2"] } },
          "columns": ["title", "result", "seconds", "lost"], "tab": "Insights" },
        { "kind": "rows", "table": "test-runs", "ids": [runs[2], "shop-test-runs-99", runs[0]], "tab": "Insights" },
        { "kind": "rows", "table": "notes", "ids": [note] },
        { "kind": "runs", "instance": deploy, "tab": "Deployment" },
        { "kind": "runs", "key": "deploy-prod", "tab": "Deployment" },
        { "kind": "runs", "environment": "prod", "tab": "Deployment" },
        { "kind": "measures", "measures": ["run"], "labels": { "blueprint": "deploy" }, "last": 5, "tab": "Insights" },
        { "kind": "links", "links": ["http://127.0.0.1:7760/"], "tab": "Links" },
    ]);
    applied(rig, under(TOLD, "told", json!({ "parts": said }))).await;
    let (lines, _) = crate::tells::inbox(rig, &rig.token).await;
    let line = lines.iter().find(|line| line["why"] == "preview").expect("a line of the preview");
    let line = line["id"].as_str().unwrap().to_string();
    let read = parts(rig, &rig.token, &line).await;
    let tabs: Vec<&Value> = read.iter().map(|part| &part["tab"]).collect();
    assert_eq!(
        tabs,
        [
            &json!("Insights"),
            &json!("Insights"),
            &Value::Null,
            &json!("Deployment"),
            &json!("Deployment"),
            &json!("Deployment"),
            &json!("Insights"),
            &json!("Links")
        ],
        "{read:?}"
    );

    // Rows chosen by what a cell says, in the table's order, with the
    // columns named that the table has, seconds said to be seconds.
    let chosen = &read[0];
    assert_eq!(each(chosen, "title"), [json!("r-1"), json!("r-2")], "{chosen}");
    assert_eq!(each(chosen, "seconds"), [json!(96), json!(3700)]);
    let columns: Vec<(&Value, &Value)> = chosen["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|column| (&column["name"], &column["seconds"]))
        .collect();
    assert_eq!(
        columns,
        [
            (&json!("title"), &json!(false)),
            (&json!("result"), &json!(false)),
            (&json!("seconds"), &json!(true))
        ]
    );
    assert_eq!(chosen["columns"][1]["of"], json!(["passed", "failed"]), "a choice says what it offers");
    assert_eq!((&chosen["unknown"], &chosen["more"]), (&json!(["lost"]), &json!(0)));
    // By ids, in their order, one that is no row said to be gone; the
    // table's own columns where it names none, a long cell cut.
    let by_ids = &read[1];
    assert_eq!(each(by_ids, "title"), [json!("r-3"), json!("gone"), json!("r-1")], "{by_ids}");
    let printed = by_ids["rows"][0]["cells"]["printed"].as_str().unwrap();
    assert!(printed.ends_with('…') && printed.chars().count() == kernel::parts::MOST_SAID + 1, "{printed}");
    // A private table is kept from whoever may not read it, the owner reading it.
    assert_eq!(each(&read[2], "title"), [json!("Kept to some")]);

    // The runs of the instance, of what ran under its key, and of what ran under prod.
    let of_instance = read[3]["runs"].as_array().unwrap_or_else(|| panic!("{}", read[3]));
    assert_eq!(of_instance.len(), 1, "{of_instance:?}");
    assert_eq!(
        (&of_instance[0]["called"], &of_instance[0]["status"]),
        (&json!("deploy as deploy-prod"), &json!("ok"))
    );
    assert!(of_instance[0]["finished"].as_i64() >= of_instance[0]["started"].as_i64());
    let of_key = read[4]["runs"].as_array().unwrap();
    assert_eq!(
        (of_key.len(), &of_key[0]["id"]),
        (2, &of_instance[0]["id"]),
        "each deploy's, the newest first"
    );
    let under_prod: Vec<&Value> =
        read[5]["runs"].as_array().unwrap().iter().map(|run| &run["called"]).collect();
    assert!(
        under_prod.contains(&&json!("deploy as deploy-prod"))
            && under_prod.contains(&&json!("kept-runs as tables")),
        "{under_prod:?}"
    );

    // What was measured of the deploy's runs: its latest beside the median of its last few.
    let measures = read[6]["measures"].as_array().unwrap_or_else(|| panic!("{}", read[6]));
    assert_eq!((&measures[0]["name"], &measures[0]["unit"]), (&json!("run"), &Value::Null), "{measures:?}");
    let measured = &measures[0]["measured"];
    assert!(
        measured["latest"].is_number()
            && measured["median"].is_number()
            && measured["of"].as_u64() >= Some(1),
        "{measured}"
    );
    assert_eq!(measures[0]["count"], 2, "both runs of the deploy, and none of anything else");

    // A key that holds its box and no more is shown none of them, each saying why.
    let blind = key(rig, &rig.token, &["inbox"]).await;
    let read = parts(rig, &blind, &line).await;
    for (at, needs) in
        [(0, "records:read"), (3, "blueprints:read"), (5, "blueprints:read"), (6, "fleet:read")]
    {
        assert!(read[at]["refused"].as_str().unwrap_or_default().contains(needs), "{at}: {}", read[at]);
        assert!(read[at]["rows"].is_null() && read[at]["runs"].is_null() && read[at]["measures"].is_null());
    }
    // One that may read records reads the table, and not the one kept private.
    let reader = key(rig, &rig.token, &["inbox", "records:read"]).await;
    let read = parts(rig, &reader, &line).await;
    assert_eq!(each(&read[0], "title"), [json!("r-1"), json!("r-2")]);
    assert!(read[2]["refused"].as_str().unwrap().contains("is kept private"), "{}", read[2]);
}
