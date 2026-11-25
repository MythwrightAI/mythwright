//! What of each way's things was accepted, as what the comparison counts
//! says, beside what the way spent: a thing written down as one of a
//! way's with nothing started for it (`compare.went`), counted as
//! accepted once however often that is said, laid out over how many the
//! way was given and under what it spent; and what was said of a thing
//! before its comparison ended counted under it, though whoever counts
//! had not come to it as it ended; and work started later for a thing
//! written down as a way's (`also`, by the param `also_per` names) counted
//! under that way, and not as one more thing given it.

use comparisons::{ACCEPTED, CAME, Compare, Counted};
use conductor::Apply;
use journal::MetricQuery;
use kernel::Source;
use serde_json::{Value, json};

use crate::asked::{ask, asked, asks, tone};
use crate::compared::child;
use crate::{Rig, active, handled, user};

/// Writes a thing down as one of a way's, as a blueprint does.
const WENT: &str = "\
blueprint: went
permissions: [blueprints:run]
params: { thing: '', way: '' }
steps:
  went: { do: compare.went, with: { name: ab, thing: \"${ params.thing }\", way: \"${ params.way }\" } }
outputs:
  written: ${ steps.went.out.written }
";

/// What it came to, written down as `key` asks: whether it was written
/// now, or why it was refused.
async fn went(rig: &Rig, key: &str, thing: &str, way: &str) -> Result<bool, String> {
    let apply =
        Apply { key: Some(key.to_string()), ..rig.request(WENT, json!({ "thing": thing, "way": way })) };
    let applied = rig.engine.apply(apply).await.map_err(|why| why.message)?;
    let run = applied.run.expect("its steps are run");
    let ran = crate::soon("it to be written down", rig.engine.wait_run(&run)).await.unwrap();
    match ran.outputs["written"].as_bool() {
        Some(written) => Ok(written),
        None => Err(ran.error.unwrap_or_default()),
    }
}

/// A review of a thing passing (`approved`) or not, as the comparison counts it.
fn reviewed() -> Counted {
    Counted {
        name: "reviewed".into(),
        topic: "test.reviewed".into(),
        thing: "id".into(),
        by: vec!["came".into()],
        accepts: vec!["approved".into()],
        once: Vec::new(),
    }
}

/// The comparison `ab`, its ways loud and quiet named by the start, each
/// thing told apart by its id; and one thing given each way.
async fn given(rig: &Rig) {
    asks(rig).await;
    let ab = Compare {
        per: Some("id".into()),
        counts: vec![reviewed()],
        ..asked("ab", vec![tone("loud", 1), tone("quiet", 1)])
    };
    rig.compared.compare(ab, &user()).await.unwrap();
    ask(rig, &[("e1", "ab", "loud"), ("e2", "ab", "quiet")], 0).await;
}

async fn say(rig: &Rig, id: &str, came: &str) {
    rig.journal.emit("test.reviewed", Source::default(), json!({ "id": id, "came": came })).await.unwrap();
}

/// How many things of each way of `ab` were counted as accepted.
async fn accepted(rig: &Rig) -> Vec<(String, u64)> {
    rig.journal.flush_metrics().await.unwrap();
    let query = MetricQuery {
        name: CAME.into(),
        group_by: vec!["way".into()],
        labels: vec![("compared".into(), "ab".into()), ("counted".into(), ACCEPTED.into())],
        ..Default::default()
    };
    let mut all: Vec<(String, u64)> = rig
        .journal
        .query_metrics(query)
        .await
        .unwrap()
        .into_iter()
        .map(|one| (one.labels["way"].clone(), one.total.count))
        .collect();
    all.sort();
    all
}

async fn until_accepted(rig: &Rig, wanted: &[(&str, u64)]) {
    let wanted: Vec<(String, u64)> = wanted.iter().map(|(way, count)| (way.to_string(), *count)).collect();
    crate::soon("what was accepted to be counted", async {
        while accepted(rig).await != wanted {
            tokio::task::yield_now().await;
        }
    })
    .await;
}

/// Everything measured under `ab`, laid beside each other as `compare show` has it.
async fn beside(rig: &Rig) -> Value {
    let one = rig.compared.comparison("ab").await.unwrap();
    rig.journal.flush_metrics().await.unwrap();
    let query = MetricQuery {
        name: "*".into(),
        group_by: comparisons::beside::BY.map(str::to_string).to_vec(),
        labels: vec![("compared".into(), "ab".into())],
        ..Default::default()
    };
    comparisons::beside::beside(&one, &rig.journal.query_metrics(query).await.unwrap())
}

#[tokio::test]
async fn a_thing_written_down_as_a_ways_is_counted_under_it_and_accepted_once_however_often_that_is_said() {
    let rig = Rig::new().await;
    given(&rig).await;
    // The card that e1's work went on with is written down as loud's: once, and then answered as it is.
    assert_eq!(went(&rig, "w1", "card-1", "loud").await, Ok(true));
    assert_eq!(went(&rig, "w2", "card-1", "loud").await, Ok(false));
    // Nothing was given for it: loud and quiet were each given one thing.
    let one = rig.compared.comparison("ab").await.unwrap();
    assert_eq!(one.ways.iter().map(|way| way.went).collect::<Vec<_>>(), [1, 1]);

    // Its review sent back, then passed twice over: accepted once. Of e2, never.
    say(&rig, "card-1", "sent_back").await;
    say(&rig, "card-1", "approved").await;
    say(&rig, "card-1", "approved").await;
    say(&rig, "e2", "sent_back").await;
    until_accepted(&rig, &[("loud", 1)]).await;

    // What each spent, as an agent's turns measure it under their way.
    for (way, cost) in [("loud", 0.9), ("loud", 0.3), ("quiet", 0.2)] {
        rig.journal.measure("agent.cost_usd", &[("compared", "ab"), ("way", way)], cost, true);
    }
    let laid = beside(&rig).await;
    let rows = laid["rows"].as_array().unwrap();
    let kinds: Vec<&str> = rows.iter().map(|row| row["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["came", "came", "spent", "accepted", "per_accepted", "took", "took"], "{laid}");
    let row = |kind: &str| rows.iter().find(|row| row["kind"] == kind).unwrap();
    // How many of each way's things were accepted, over how many it was given.
    assert_eq!(row("accepted")["what"], "accepted");
    assert_eq!(
        row("accepted")["by"],
        json!([{ "count": 1, "given": 1, "rate": 1.0 }, { "count": 0, "given": 1, "rate": 0.0 }])
    );
    // What each spent, over how many were accepted: none, for a way none of whose work was.
    let per = row("per_accepted");
    assert_eq!((per["what"].as_str(), per["unit"].as_str()), (Some("agent.cost_usd"), Some("usd")));
    let loud = &per["by"][0];
    assert!((loud["sum"].as_f64().unwrap() - 1.2).abs() < 1e-9, "{per}");
    assert!((loud["per_accepted"].as_f64().unwrap() - 1.2).abs() < 1e-9, "{per}");
    assert_eq!(loud["accepted"], 1);
    assert_eq!(per["by"][1]["accepted"], 0);
    assert!(per["by"][1].get("per_accepted").is_none(), "{per}");
    // What is said of each way never orders or names one as the better.
    let said = laid.to_string();
    assert!(!["better", "best", "score", "winner"].iter().any(|word| said.contains(word)), "{said}");

    // A thing that went one way is not written down as another's; nor in a comparison that has ended.
    let other = went(&rig, "w3", "e1", "quiet").await.unwrap_err();
    assert!(other.contains("went the way `loud` of `ab`"), "{other}");
    rig.compared.end_comparison("ab", None).await.unwrap();
    let ended = went(&rig, "w4", "card-2", "loud").await.unwrap_err();
    assert!(ended.contains("still going"), "{ended}");
}

#[tokio::test]
async fn a_comparison_that_says_nothing_is_accepted_lays_out_no_such_rows() {
    let rig = Rig::new().await;
    asks(&rig).await;
    let ab = Compare {
        per: Some("id".into()),
        counts: vec![Counted { accepts: Vec::new(), ..reviewed() }],
        ..asked("ab", vec![tone("loud", 1)])
    };
    rig.compared.compare(ab, &user()).await.unwrap();
    ask(&rig, &[("e1", "ab", "loud")], 0).await;
    say(&rig, "e1", "approved").await;
    rig.journal.measure("agent.cost_usd", &[("compared", "ab"), ("way", "loud")], 0.5, true);
    crate::soon("what came of it to be counted", async {
        loop {
            let laid = beside(&rig).await;
            let kinds: Vec<String> = laid["rows"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| row["kind"].as_str().unwrap().to_string())
                .collect();
            if kinds.contains(&"came".to_string()) {
                assert_eq!(kinds, ["came", "spent", "took", "took"], "{laid}");
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(accepted(&rig).await.is_empty());
}

#[tokio::test]
async fn what_was_said_before_a_comparison_ended_is_counted_under_it_though_nobody_had_counted_it_yet() {
    let mut rig = Rig::new().await;
    given(&rig).await;
    // Nobody counts while the conductor is stopped: what is said then is
    // after where counting had come to.
    rig.stopped().await;
    say(&rig, "e1", "approved").await;
    // Ended before anybody has counted it, by a conductor not yet taken up.
    rig.compared.end_comparison("ab", None).await.unwrap();
    until_accepted(&rig, &[("loud", 1)]).await;
    crate::soon("it to be owed nothing more", async {
        while rig.compared.comparison("ab").await.unwrap().until.is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    // And nothing said after it ended is.
    say(&rig, "e2", "approved").await;
    rig.engine.resume().await.unwrap();
    crate::soon("the counting to be over", async {
        while rig.journal.kv_get("compared.counted", "cursor").await.unwrap().is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert_eq!(accepted(&rig).await, [("loud".to_string(), 1)]);
}

/// The work done on a thing again, by the param `item`: as a card sent
/// back is worked again, named by no comparison.
const REWORK: &str = "\
blueprint: rework
params: { item: '' }
steps:
  claim: { do: test.count, with: { key: rework } }
";

/// What starts it for each thing it is asked for.
const REDO: &str = "\
blueprint: redo
on:
  redo:
    event: test.redo
    steps:
      done:
        call: rework
        as: \"${ event.data.key }\"
        with: { item: \"${ event.data.item }\" }
";

#[tokio::test]
async fn work_on_a_thing_written_down_as_a_ways_is_counted_under_that_way_and_not_as_given() {
    let rig = Rig::new().await;
    asks(&rig).await;
    rig.library.put("rework", REWORK);
    rig.library.put("redo", REDO);
    let redo = rig.apply(REDO, json!({})).await;
    active(&rig, &redo.instance).await;
    let ab = Compare {
        per: Some("id".into()),
        also: vec!["rework".into()],
        also_per: Some("item".into()),
        counts: vec![reviewed()],
        ..asked("ab", vec![tone("loud", 1), tone("quiet", 1)])
    };
    rig.compared.compare(ab, &user()).await.unwrap();
    ask(&rig, &[("e1", "ab", "loud"), ("e2", "ab", "quiet")], 0).await;
    assert_eq!(went(&rig, "w1", "card-1", "loud").await, Ok(true));

    // The card written down as loud's worked again, and one nobody wrote down.
    for (key, item) in [("r1", "card-1"), ("r2", "card-9")] {
        rig.journal.emit("test.redo", Source::default(), json!({ "key": key, "item": item })).await.unwrap();
    }
    handled(&rig, "on.redo", 2).await;
    assert_eq!(rig.shared.count("rework"), 2);
    let under = |labels: &std::collections::BTreeMap<String, String>| {
        (labels.get("compared").cloned(), labels.get("way").cloned())
    };
    assert_eq!(under(&child(&rig, "r1").await.labels), (Some("ab".into()), Some("loud".into())));
    assert_eq!(under(&child(&rig, "r2").await.labels), (None, None));
    // Counted under loud, and no more of its things given: what it spent
    // is spent on the one thing it was given.
    let one = rig.compared.comparison("ab").await.unwrap();
    assert_eq!(one.ways.iter().map(|way| way.went).collect::<Vec<_>>(), [1, 1]);
    assert_eq!(one.also_per.as_deref(), Some("item"));

    // Asked again with another param, it is refused: what was counted by
    // one would be counted apart from what was counted by the other.
    let again = Compare {
        per: Some("id".into()),
        also: vec!["rework".into()],
        also_per: Some("card".into()),
        ..asked("ab", vec![tone("loud", 1), tone("quiet", 1)])
    };
    assert!(rig.compared.compare(again, &user()).await.is_err());
}
