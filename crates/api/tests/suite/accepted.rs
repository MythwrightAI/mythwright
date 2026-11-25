//! What each contender of a card's A/B test cost per card accepted,
//! beside how often its work was: two contenders, one dear and one cheap,
//! each run by a stand-in that says what its turn cost, a judge that
//! picks one, and a reviewer who sends the first card's work back once
//! and then passes it, and passes the second's: the winner's work again
//! is counted under its way, and is no more cards given it. Laid out by
//! the daemon (`compare show`'s `beside`) for a comparison the tests
//! share and for a test's own, which counts what comes of its card until
//! its review passes and is then ended, having counted that.

use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add, card, shop};
use crate::lanes::move_card;

/// What a stand-in says of a turn, as an agent's program says it: a
/// line of JSON, with what the turn cost and the tokens it read and wrote.
fn turn(text: &str, cost: f64) -> String {
    json!({ "text": text, "cost": cost, "in": 1000, "out": 100 }).to_string()
}

/// A package for a product there is not, whose program is the stand-in
/// doing `words`, read as an agent's program that says what it took.
fn package(name: &str, words: &[&str]) -> String {
    let args: Vec<String> = words.iter().map(|word| json!(word).to_string()).collect();
    format!(
        "extension: {name}\ntitle: {name}\ndescription: Not a real agent.\nagent:\n  program: {program}\n  \
         args:\n    - [{args}]\n  input: ${{ turn.prompt }}\n  output:\n    - when: ${{ event.text != null }}\n      \
         reply: ${{ event.text }}\n      cost_usd: ${{ event.cost }}\n      input_tokens: ${{ event.in }}\n      \
         output_tokens: ${{ event.out }}\n",
        program = json!(crate::STANDIN),
        args = args.join(", "),
    )
}

/// What a turn of each contender costs.
const DEAR: f64 = 1.25;
const CHEAP: f64 = 0.125;

/// A factory with the fleet that ships in `shop`, its A/B tests given:
/// two who contend (`dear` and `cheap`), each changing a file and saying
/// what it cost; a judge who always picks A; and a reviewer who sends
/// the first work it looks at back, and passes everything after.
async fn factory() -> Rig {
    let rig = Rig::owned(Rig::with("api-accepted", |_, _| {}).await).await;
    let looked = rig.dir.join("looked").to_string_lossy().into_owned();
    let works = |cost: f64| turn("I changed a file.", cost);
    let (dear, cheap) = (works(DEAR), works(CHEAP));
    let picks = turn("A does what the card asks; B does not.\nDECISION: A", 0.01);
    let approves = turn("It does what the card asks.\nDECISION: approve", 0.01);
    let rejects = turn("The total is wrong.\nDECISION: send_back", 0.01);
    let once =
        ["swallow", "there", &looked, "(", "say", &approves, ")", "write", &looked, "once", "say", &rejects];
    for source in [
        package("dear", &["swallow", "append", "CHANGES.md", "changed", "say", &dear]),
        package("cheap", &["swallow", "append", "CHANGES.md", "changed", "say", &cheap]),
        package("picker", &["swallow", "say", &picks]),
        package("looker", &once),
    ] {
        rig.send("/api/extensions", json!({ "source": source })).await;
    }
    let rows = json!([
        { "title": "worker", "role": "worker", "count": 1, "adapter": "cheap" },
        { "title": "dear", "role": "dear", "count": 1, "adapter": "dear" },
        { "title": "cheap", "role": "cheap", "count": 1, "adapter": "cheap" },
        { "title": "reviewer", "role": "reviewer", "count": 1, "adapter": "looker", "permission": "read" },
        { "title": "judge", "role": "judge", "count": 1, "adapter": "picker", "permission": "read" },
    ]);
    shop(&rig, json!({ "agents": rows })).await;
    let ab =
        json!({ "blueprint": "std/ab", "project": "shop", "params": { "project": "shop" }, "wait": true });
    assert_eq!(rig.send("/api/apply", ab).await["finished"]["status"], "ok");
    rig
}

/// A card with a test set up as `fields` say, worked through to done:
/// the test's row once it is.
async fn contested(rig: &Rig, title: &str, fields: Value) -> Value {
    let id = add(rig, title, &[]).await;
    let mut fields = fields;
    fields["card"] = json!(id);
    let before = rig.mark();
    let test = rig.send("/api/collections/shop/contests/records", json!({ "fields": fields })).await;
    let test = test["id"].as_str().unwrap().to_string();
    rig.event("fleet.arranged", move |event| event.seq > before).await;
    move_card(rig, &id, "ready", None).await;
    let card_id = id.clone();
    rig.event("card.left,board.item.moved", move |event| {
        let of = event.data["card"] == card_id.as_str() || event.data["id"] == card_id.as_str();
        of && (event.topic == "card.left" || event.data["to"] == "done")
    })
    .await;
    assert_eq!(card(rig, &id).await["status"], "done", "{}", card(rig, &id).await);
    rig.read(&format!("/api/records/{test}")).await
}

/// The rows of `kind` of what `name` lays out, once one of them says
/// something of every column; and the columns.
async fn laid(rig: &Rig, name: &str, kind: &str) -> (Vec<Value>, Vec<Value>) {
    let name = name.to_string();
    crate::soon(&format!("what {name} counts of what was accepted"), async {
        loop {
            let shown = rig.read(&format!("/api/comparisons/{name}")).await;
            let beside = &shown["beside"];
            let rows: Vec<Value> = beside["rows"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|row| row["kind"] == kind)
                .cloned()
                .collect();
            let counted = rows.iter().any(|row| {
                row["by"]
                    .as_array()
                    .is_some_and(|by| by.iter().any(|cell| cell["count"] == 1 || cell["accepted"] == 1))
            });
            if counted {
                return (beside["columns"].as_array().cloned().unwrap_or_default(), rows);
            }
            tokio::task::yield_now().await;
        }
    })
    .await
}

/// Where the column of `way` is among `columns`.
fn at(columns: &[Value], way: &str) -> usize {
    columns
        .iter()
        .position(|column| column["way"] == way)
        .unwrap_or_else(|| panic!("no column {way}: {columns:?}"))
}

/// What a way's one turn cost.
fn cost(way: &str) -> f64 {
    if way == "dear" { DEAR } else { CHEAP }
}

/// Holds what is laid out of `name`: the winner's one thing accepted of
/// the one it was given, the loser's none; and what each spent, the
/// winner's (`turns` of its agent's, its work done again among them) over
/// the one accepted.
async fn holds(rig: &Rig, name: &str, winner: &str, turns: f64) {
    let loser = if winner == "dear" { "cheap" } else { "dear" };
    let (columns, accepted) = laid(rig, name, "accepted").await;
    let (won, lost) = (at(&columns, winner), at(&columns, loser));
    assert_eq!(accepted.len(), 1, "{accepted:?}");
    let by = &accepted[0]["by"];
    assert_eq!(by[won], json!({ "count": 1, "given": 1, "rate": 1.0 }), "{by}");
    assert_eq!(by[lost], json!({ "count": 0, "given": 1, "rate": 0.0 }), "{by}");
    let (_, per) = laid(rig, name, "per_accepted").await;
    let spent = per.iter().find(|row| row["what"] == "agent.cost_usd").unwrap_or_else(|| panic!("{per:?}"));
    assert_eq!(spent["unit"], "usd");
    let figure = |cell: &Value, key: &str| cell[key].as_f64().unwrap_or_else(|| panic!("{key} of {cell}"));
    let (won, lost) = (&spent["by"][won], &spent["by"][lost]);
    assert!((figure(won, "sum") - turns * cost(winner)).abs() < 1e-9, "{spent}");
    assert!((figure(won, "per_accepted") - turns * cost(winner)).abs() < 1e-9, "{spent}");
    assert!((figure(lost, "sum") - cost(loser)).abs() < 1e-9, "{spent}");
    assert_eq!(lost["accepted"], 0, "{spent}");
    assert!(lost.get("per_accepted").is_none(), "{spent}");
}

#[tokio::test]
async fn what_each_contender_cost_per_card_accepted_is_laid_beside_how_often_its_work_was() {
    let rig = factory().await;

    // Counted under a comparison the tests share: its card sent back once and worked again by the
    // winner, which is the winner's work, and no second card given it.
    let shared = json!({ "contenders": ["dear", "cheap"], "judges": "judge", "compared": "pricing" });
    let test = contested(&rig, "Add a cart", shared).await;
    assert_eq!(test["data"]["state"], "came", "{test}");
    let winner = test["data"]["winner"].as_str().unwrap().to_string();
    holds(&rig, "pricing", &winner, 2.0).await;
    let shown = rig.read("/api/comparisons/pricing").await;
    assert!(shown["comparison"]["ended"].is_null(), "a shared one goes on: {shown}");
    let said = shown["beside"].to_string();
    assert!(!["better", "best", "score"].iter().any(|word| said.contains(word)), "{said}");

    // Counted under the test's own, which goes on counting what comes of the card after it is judged,
    // and is ended once its review has passed, with that counted.
    let own =
        contested(&rig, "Add a till", json!({ "contenders": ["dear", "cheap"], "judges": "judge" })).await;
    let name = format!("ab-{}", own["id"].as_str().unwrap());
    holds(&rig, &name, own["data"]["winner"].as_str().unwrap(), 1.0).await;
    crate::soon("the test's own comparison to be ended", async {
        while rig.read(&format!("/api/comparisons/{name}")).await["comparison"]["ended"].is_null() {
            tokio::task::yield_now().await;
        }
    })
    .await;
}
