//! Sandbox tests side by side: one set of acts played against several
//! texts of a blueprint, each in a sandbox of its own, and what each came
//! to laid beside the others as a comparison's columns are.
#![cfg(target_os = "linux")]

use reqwest::Method;
use serde_json::{Value, json};

use crate::Rig;
use crate::sandboxed::{differing, rows};

/// Asks once, and keeps who rang a bell.
const ONCE: &str = "blueprint: asks\nversion: 1\npermissions: [agents]\nsteps:\n  ask: { do: agent.task, with: { prompt: \"Which way?\", role: worker } }\nstate:\n  rung: nobody\non:\n  rung:\n    event: bell.rung\n    steps:\n      note: { do: set, with: { rung: \"${event.data.by}\" } }\n";

/// The same, asking twice.
const TWICE: &str = "blueprint: asks\nversion: 2\npermissions: [agents]\nsteps:\n  ask: { do: agent.task, with: { prompt: \"Which way?\", role: worker } }\n  again: { do: agent.task, with: { prompt: \"Are you sure?\", role: worker } }\nstate:\n  rung: nobody\non:\n  rung:\n    event: bell.rung\n    steps:\n      note: { do: set, with: { rung: \"${event.data.by}\" } }\n";

/// The row of `beside` called `what`, by its column, as counts: nothing
/// where that column has none.
fn counts(beside: &Value, what: &str) -> Vec<Option<u64>> {
    let row = beside["rows"].as_array().unwrap().iter().find(|row| row["what"] == what);
    let row = row.unwrap_or_else(|| panic!("no row `{what}`: {beside}"));
    row["by"].as_array().unwrap().iter().map(|cell| cell["count"].as_u64()).collect()
}

#[tokio::test]
async fn one_set_of_acts_is_played_against_two_texts_each_in_a_sandbox_of_its_own_and_counted_side_by_side() {
    let rig = Rig::new().await;
    // At rest first: what the factory does by itself as it starts (the
    // search index's first pass, with its cursor) is no sandbox's, and
    // was still being written as its owner was registered, once that
    // was no longer a hash slow enough for it to be done by then.
    crate::simulation::rested(&rig).await;
    let before = rows(&rig).await;

    let played = rig
        .send(
            "/api/sandboxes/beside",
            json!({
                "texts": [{ "called": "once", "source": ONCE }, { "called": "twice", "source": TWICE }],
                "acts": [
                    { "answer": { "asked": 1, "usual": true } },
                    { "answer": { "asked": 2, "gives": { "reply": "yes", "available": true } } },
                    { "say": { "topic": "bell.rung", "data": { "by": "ada" } } },
                ],
            }),
        )
        .await;

    assert_eq!(played["ok"], true, "{played}");
    assert!(played["said"].as_str().unwrap().starts_with("This was a sandbox test"), "{played}");
    let columns = played["sandboxes"].as_array().unwrap();
    assert_eq!(columns.len(), 2);
    // In the order they were asked for, each with a sandbox of its own and its own text.
    assert_eq!((&columns[0]["called"], &columns[1]["called"]), (&json!("once"), &json!("twice")));
    assert_ne!(columns[0]["sandbox"], columns[1]["sandbox"]);
    for column in columns {
        let shown = &column["shown"];
        assert!(shown["said"].as_str().unwrap().contains("passing here is not passing for real"), "{shown}");
        assert_eq!(shown["instances"][0]["state"]["rung"], "ada", "{shown}");
        assert_eq!(shown["waiting"], json!([]), "{shown}");
        assert_eq!(column["refused"], Value::Null);
    }
    // The text that asks once had no second stand-in to answer: that is said, and passed over.
    assert_eq!(columns[0]["skipped"].as_array().unwrap().len(), 1, "{}", columns[0]);
    assert!(columns[0]["skipped"][0].as_str().unwrap().contains("2nd"), "{}", columns[0]);
    assert_eq!(columns[1]["skipped"], json!([]));
    // Answered as usual with what a stand-in for it usually answers, which each sandbox knows.
    assert_eq!(columns[1]["shown"]["asked"][0]["answered"], columns[1]["shown"]["asked"][0]["usual"]);
    assert_eq!(columns[1]["shown"]["asked"][1]["answered"]["reply"], "yes", "{}", columns[1]);

    // Laid out as a comparison's columns are: a column a text, one thing given each.
    let beside = &played["beside"];
    let laid = beside["columns"].as_array().unwrap();
    assert_eq!(laid.len(), 2);
    assert_eq!((&laid[0]["way"], &laid[1]["way"]), (&json!("once"), &json!("twice")));
    assert_eq!((&laid[0]["version"], &laid[1]["version"]), (&json!("1"), &json!("2")));
    assert_eq!((&laid[0]["given"], &laid[1]["given"]), (&json!(1), &json!(1)));
    assert_ne!(laid[0]["text"], laid[1]["text"]);
    assert_eq!(counts(beside, "agent.task stood in"), vec![Some(1), Some(2)]);
    assert_eq!(counts(beside, "bell.rung"), vec![Some(1), Some(1)]);
    // What ran is measured as it was, beside each other.
    let steps = counts(beside, "step");
    assert!(steps[1] > steps[0], "the text that asks twice ran more steps: {beside}");
    // Counts, by kind, in the kinds a comparison has; nothing is marked as better.
    for row in beside["rows"].as_array().unwrap() {
        assert!(["came", "chose", "spent", "took"].contains(&row["kind"].as_str().unwrap()), "{row}");
        assert_eq!(row["by"].as_array().unwrap().len(), 2, "{row}");
    }
    let said = beside.to_string();
    for word in ["better", "best", "winner", "score"] {
        assert!(!said.contains(word), "`{word}` is in what is laid out: {said}");
    }

    // Nothing of either is in the factory in use: no agent was asked, no row was written.
    assert_eq!(rig.read("/api/agents").await["agents"], json!([]));
    assert!(differing(&before, &rows(&rig).await).is_empty(), "{:?}", differing(&before, &rows(&rig).await));
    assert_eq!(rig.node().app.sandboxes.count(), 2);

    // Each is kept as any sandbox is until the page lets go, and then ended.
    for column in columns {
        let at = format!("/api/sandboxes/{}", column["sandbox"].as_str().unwrap());
        assert_eq!(rig.get(&at).await.0, 200);
        let (status, _) = rig.call(Method::DELETE, &at, Some(&rig.token), None).await;
        assert_eq!(status, 200);
        assert_eq!(rig.get(&at).await.0, 404);
    }
    assert_eq!(rig.node().app.sandboxes.count(), 0);
    assert!(!rig.dir.join("data").join("sandboxes").exists(), "nothing of them is left on the disk");
}

#[tokio::test]
async fn what_ships_is_one_text_side_by_side_and_one_that_does_not_check_has_none_made() {
    let rig = Rig::new().await;
    let shipped =
        rig.read("/api/blueprints/source?name=std/board").await["source"].as_str().unwrap().to_string();

    let played = rig
        .send(
            "/api/sandboxes/beside",
            json!({
                "name": "std/board",
                "texts": [{ "called": "default", "ships": true }, { "called": "as it stands", "source": shipped }],
            }),
        )
        .await;

    assert_eq!(played["ok"], true, "{played}");
    let laid = played["beside"]["columns"].as_array().unwrap();
    // The same text, read from where it ships and as it was sent.
    assert_eq!(laid[0]["text"], laid[1]["text"], "{laid:?}");
    // What the runner measures of its own work is no row: how often it woke a handler is the machine's order, which
    // the same text played twice comes to one way or the other, and nothing of what came of the text.
    for row in played["beside"]["rows"].as_array().unwrap() {
        let measured = row["measured"].as_str().unwrap_or_default();
        assert!(
            !["handler.", "instance.", "run.", "step."].iter().any(|own| measured.starts_with(own)),
            "{row}"
        );
    }
    assert!(
        played["beside"]["rows"].as_array().unwrap().iter().any(|row| row["what"] == "step"),
        "what ran is laid out"
    );
    for column in played["sandboxes"].as_array().unwrap() {
        let tables = column["shown"]["tables"].as_array().unwrap();
        assert!(tables.iter().any(|table| table["name"] == "board"), "{column}");
        let at = format!("/api/sandboxes/{}", column["sandbox"].as_str().unwrap());
        rig.call(Method::DELETE, &at, Some(&rig.token), None).await;
    }

    // One that does not check: what is wrong is said with its column, and nothing is made.
    let wrong = rig
        .send(
            "/api/sandboxes/beside",
            json!({ "texts": [
                { "called": "once", "source": ONCE },
                { "called": "wrong", "source": "blueprint: asks\nsteps:\n  a: { do: no.such.action }\n" },
            ] }),
        )
        .await;
    assert_eq!(wrong["ok"], false, "{wrong}");
    assert_eq!(wrong["texts"][0]["called"], "wrong", "{wrong}");
    assert!(wrong["texts"][0]["diagnostics"].to_string().contains("no.such.action"), "{wrong}");
    assert_eq!(rig.node().app.sandboxes.count(), 0);

    // One text is no side by side, and two called the same have one heading for two columns.
    let (status, alone) =
        rig.post("/api/sandboxes/beside", json!({ "texts": [{ "called": "a", "source": ONCE }] })).await;
    assert_eq!(status, 400, "{alone}");
    let same = json!({ "texts": [{ "called": "a", "source": ONCE }, { "called": "a", "source": TWICE }] });
    assert_eq!(rig.post("/api/sandboxes/beside", same).await.0, 400);
    // And somebody who may not read blueprints is not shown any.
    assert_eq!(rig.node().app.sandboxes.count(), 0);
}
