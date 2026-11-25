//! A run that waits for a person to let it go on (`approval.ask`,
//! `told::goes`), through the routes: who is told
//! and who may decide (whoever started it only where
//! the environment's rule names them, never an agent), the run going on
//! once it is approved and failing with why once it is declined, a key
//! that holds less refused, and an ask that waited its time written as
//! expired with nobody able to approve it after.

use std::sync::Arc;
use std::time::Duration;

use kernel::ManualClock;
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::held::said;
use crate::kept::{Person, key, person};

/// A text that builds, waits for somebody to let it go on, and then ships,
/// saying who let it.
const SHIP: &str = "\
blueprint: ship
steps:
  built:
    do: emit
    with: { topic: shop.built }
  approve:
    needs: [built]
    do: approval.ask
    with: { says: Ship the build?, links: [\"https://example.com/preview\"] }
  shipped:
    needs: [approve]
    do: emit
    with: { topic: shop.shipped, by: \"${steps.approve.out.approved_by_name}\" }
";

/// The same, given a minute to be let go on.
const QUICK: &str = "\
blueprint: quick
steps:
  approve:
    do: approval.ask
    timeout: 1m
    with: { says: Go on within the minute? }
";

async fn ticking() -> (Rig, ManualClock) {
    let clock = ManualClock::new();
    let kept = clock.clone();
    let rig = Rig::with("api-goahead", move |config, _| config.clock = Some(Arc::new(kept))).await;
    (Rig::owned(rig).await, clock)
}

/// Applies `source` for Shop as `token`, under `environment` where said:
/// the run of its steps.
async fn applied(rig: &Rig, token: &str, source: &str, environment: Option<&str>) -> String {
    let mut body = json!({ "source": source, "project": "shop" });
    if let Some(environment) = environment {
        body["environment"] = json!(environment);
    }
    let (status, applied) = rig.call(Method::POST, "/api/apply", Some(token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    applied["run"].as_str().unwrap().to_string()
}

/// The ask `run` made, once it is told.
async fn asked(rig: &Rig, run: &str) -> Arc<kernel::Event> {
    rig.event("project.approval.asked", |event| event.data["run"] == json!(run)).await
}

/// The line of `token`'s inbox that tells of `ask`, where there is one.
async fn line_of(rig: &Rig, token: &str, ask: &str) -> Option<Value> {
    let (_, inbox) = rig.call(Method::GET, "/api/inbox", Some(token), None).await;
    let lines = inbox["lines"].as_array().cloned().unwrap_or_default();
    lines.into_iter().find(|line| line["why"] == json!(format!("asked {ask}")))
}

async fn decide(rig: &Rig, token: &str, ask: &str, how: &str, body: Value) -> (StatusCode, Value) {
    rig.call(Method::POST, &format!("/api/asked/{ask}/{how}"), Some(token), Some(body)).await
}

async fn ended(rig: &Rig, run: &str) -> Value {
    rig.event("run.finished", |event| event.source.run.as_deref() == Some(run)).await;
    rig.read(&format!("/api/runs/{run}")).await
}

#[tokio::test]
async fn a_run_waits_for_somebody_who_did_not_start_it_and_goes_on_once_they_let_it() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let grace: Person = person(&rig, "Grace").await;

    let run = applied(&rig, &rig.token, SHIP, None).await;
    let told = asked(&rig, &run).await;
    let ask = told.data["ask"].as_str().unwrap().to_string();
    assert_eq!(told.data["project"], "shop");
    // Told to Grace, who may let it go on, and not to Ada, who started it.
    assert_eq!(told.data["to"], json!([grace.actor]), "{told:?}");
    // The line is owed by the ask's row and told a moment after it is said.
    rig.event("inbox.told", |event| {
        event.data["owner"] == json!(grace.actor) && event.data["about"] == json!(run)
    })
    .await;
    assert!(line_of(&rig, &rig.token, &ask).await.is_none(), "whoever started it is not asked");
    let line = line_of(&rig, &grace.token, &ask).await.expect("Grace is told");
    assert!(
        line["says"].as_str().unwrap().contains("Ada's run of ship waits for your go-ahead: Ship the build?")
    );
    // About the run that waits, whose page the line opens.
    assert_eq!((&line["about"], &line["place"]), (&json!(run), &json!(run)), "{line}");
    let (_, parts) = rig
        .call(
            Method::GET,
            &format!("/api/inbox/{}/parts", line["id"].as_str().unwrap()),
            Some(&grace.token),
            None,
        )
        .await;
    let go = &parts["parts"][0];
    assert_eq!(
        (&go["kind"], &go["go"]["ask"], &go["go"]["may"]),
        (&json!("go"), &json!(ask), &json!(true)),
        "{parts}"
    );
    assert_eq!(go["go"]["links"], json!(["https://example.com/preview"]));
    assert_eq!(go["go"]["run"], json!(run));

    // The run's page reads the same, as whoever reads it.
    let (_, of_run) = rig.call(Method::GET, &format!("/api/asked?run={run}"), Some(&grace.token), None).await;
    assert_eq!(of_run["asked"][0]["ask"], json!(ask), "{of_run}");
    // Ada is shown it, and may not let her own run go on.
    let shown = rig.read(&format!("/api/asked/{ask}")).await;
    assert_eq!(shown["may"], json!(false), "{shown}");
    assert!(shown["why_not"].as_str().unwrap().contains("you started this run"), "{shown}");
    let (status, refused) = decide(&rig, &rig.token, &ask, "approve", json!({})).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    // Nor does a key of Grace's that holds less.
    let narrow = key(&rig, &grace.token, &["projects", "blueprints:read", "inbox"]).await;
    let (status, refused) = decide(&rig, &narrow, &ask, "approve", json!({})).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("blueprints:run"), "{refused}");
    assert_eq!(rig.read(&format!("/api/runs/{run}")).await["status"], "running");

    // Grace lets it go on, and the run ships saying so.
    let (status, approved) = decide(&rig, &grace.token, &ask, "approve", json!({})).await;
    assert_eq!((status, &approved["status"]), (StatusCode::OK, &json!("approved")), "{approved}");
    let finished = ended(&rig, &run).await;
    assert_eq!(finished["status"], "ok", "{finished}");
    let shipped = rig.event("shop.shipped", |event| event.source.run.as_deref() == Some(&run)).await;
    assert_eq!(shipped.data["by"], "Grace");
    // Pressed again, it is as it was decided.
    let (status, again) = decide(&rig, &grace.token, &ask, "decline", json!({ "why": "late" })).await;
    assert_eq!((status, &again["status"]), (StatusCode::OK, &json!("approved")), "{again}");
}

#[tokio::test]
async fn under_an_environment_that_says_who_approves_only_they_decide_its_starter_among_them_and_a_decline_fails_the_step_with_why()
 {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let grace = person(&rig, "Grace").await;
    let base = "/api/projects/shop/environments";
    rig.send(base, json!({ "name": "prod" })).await;
    let rules = format!("{base}/prod/rules");

    // Saying who approves takes what changing its secrets takes, and names somebody here.
    let (status, _) =
        rig.call(Method::PUT, &rules, Some(&grace.token), Some(json!({ "approve": ["Grace"] }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, refused) = rig
        .call(Method::PUT, &rules, Some(&rig.token), Some(json!({ "approve": ["nobody@example.com"] })))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    let (status, set) = rig
        .call(Method::PUT, &rules, Some(&rig.token), Some(json!({ "approve": ["ada@example.com"] })))
        .await;
    assert_eq!(status, StatusCode::OK, "{set}");
    let ada = set["environment"]["rules"]["approve"][0].as_str().unwrap().to_string();
    assert!(ada.starts_with("user:"), "{set}");
    assert_eq!(set["changed"]["rules"], json!(true));

    // Ada's own run under prod is hers to let go on, since the rule names her, and nobody else's.
    let run = applied(&rig, &rig.token, SHIP, Some("prod")).await;
    let told = asked(&rig, &run).await;
    let ask = told.data["ask"].as_str().unwrap().to_string();
    assert_eq!((&told.data["to"], &told.data["environment"]), (&json!([ada]), &json!("prod")), "{told:?}");
    assert!(line_of(&rig, &grace.token, &ask).await.is_none(), "Grace is not one the rule names");
    let (status, refused) = decide(&rig, &grace.token, &ask, "approve", json!({})).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("approved by those its rule names"), "{refused}");
    assert_eq!(rig.read(&format!("/api/asked/{ask}")).await["may"], json!(true));

    // Declined with why: the step fails with it, and nothing ships.
    let (status, declined) =
        decide(&rig, &rig.token, &ask, "decline", json!({ "why": "not on a Friday" })).await;
    assert_eq!((status, &declined["status"]), (StatusCode::OK, &json!("declined")), "{declined}");
    let finished = ended(&rig, &run).await;
    assert_eq!(finished["status"], "failed", "{finished}");
    let step =
        finished["steps"].as_array().unwrap().iter().find(|step| step["step"] == "approve").unwrap().clone();
    assert!(step["error"].as_str().unwrap_or_default().contains("not on a Friday"), "{step}");
    assert_eq!(rig.count_of("shop.shipped").await, 0);
}

#[tokio::test]
async fn an_ask_that_waited_as_long_as_its_step_is_given_is_expired_and_nobody_approves_it_after() {
    let (rig, clock) = ticking().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let grace = person(&rig, "Grace").await;
    let run = applied(&rig, &rig.token, QUICK, None).await;
    let ask = asked(&rig, &run).await.data["ask"].as_str().unwrap().to_string();
    clock.advance(Duration::from_secs(61));
    let finished = ended(&rig, &run).await;
    assert_eq!(finished["status"], "failed", "{finished}");
    let shown = rig.read(&format!("/api/asked/{ask}")).await;
    assert_eq!(shown["status"], "expired", "{shown}");
    let (status, still) = decide(&rig, &grace.token, &ask, "approve", json!({})).await;
    assert_eq!((status, &still["status"]), (StatusCode::OK, &json!("expired")), "{still}");
}
