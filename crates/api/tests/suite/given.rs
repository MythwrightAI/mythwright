//! Who a card, or a seat at its review, is given to, where several of a
//! role could each take it: what the stage's row says (`picks`), a rank
//! an agents row says, and the agent a card itself names. And what comes
//! of it counted, for whoever wants a figure: what each agent chose, and
//! what each stage came to.
//!
//! Nothing here is a rule of the daemon's about workers or reviewers: the
//! stage's row is handed down to where an agent is picked, and the rest is
//! `Agents::available_apart`.

use axum::http::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add, add_row, change, given, rig, says, shop};
use crate::lanes::{move_card, moved};

/// A card added, made ready and worked: its id, once it is in review.
async fn worked(rig: &Rig, title: &str) -> String {
    let id = add(rig, title, &[]).await;
    move_card(rig, &id, "ready", None).await;
    moved(rig, &id, "review", 0).await;
    id
}

/// How many cards each of the two who work was given.
async fn each(rig: &Rig) -> (usize, usize) {
    (given(rig, "shop-worker-1").await.len(), given(rig, "shop-worker-2").await.len())
}

/// Says who a card is given to, as someone would on the board.
async fn give(rig: &Rig, id: &str, to: &str) {
    let (status, answer) = rig
        .call(
            Method::PATCH,
            &format!("/api/board/items/{id}"),
            Some(&rig.token),
            Some(json!({ "assignee": to })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
}

/// `(the labels a series is split by, how many were measured)`, for each
/// series that says every one of `by`.
async fn counted(rig: &Rig, name: &str, by: &[&str]) -> Vec<(Value, u64)> {
    let answer = rig.read(&format!("/api/metrics?name={name}&by={}&labels=project:shop", by.join(","))).await;
    let series = answer["series"].as_array().unwrap();
    let mut all: Vec<(Value, u64)> = series
        .iter()
        .filter(|one| by.iter().all(|label| one["labels"][label].is_string()))
        .map(|one| (one["labels"].clone(), one["total"]["count"].as_u64().unwrap()))
        .collect();
    all.sort_by_key(|(labels, _)| labels.to_string());
    all
}

#[tokio::test]
async fn a_card_goes_to_the_agent_brought_up_first_until_the_stages_row_says_each_in_turn() {
    let (rig, _at) = rig().await;
    // Two who work, and nobody to review: a card that was worked waits in review.
    shop(&rig, json!({ "workers": 2, "reviewers": 0 })).await;
    // Nothing said of how they are chosen between: one after another, each
    // card finds both with nothing in hand, and the first of them takes it.
    worked(&rig, "Add a cart").await;
    worked(&rig, "Add a till").await;
    assert_eq!(each(&rig).await, (2, 0));

    // Each in turn, said in the stage's row while the fleet runs: the one
    // that was never given a card, then the one given one longest ago.
    change(&rig, "stages", "work", json!({ "picks": "turn" })).await;
    worked(&rig, "Add a shelf").await;
    assert_eq!(each(&rig).await, (2, 1));
    worked(&rig, "Add a door").await;
    assert_eq!(each(&rig).await, (3, 1));
    worked(&rig, "Add a sign").await;
    assert_eq!(each(&rig).await, (3, 2));
}

#[tokio::test]
async fn a_review_that_picks_by_rank_seats_whoever_its_row_ranks_highest() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({ "reviewers": 2 })).await;
    // A third who reviews, a row of its own that is ranked above the
    // others; and one of the three to a card, the highest ranked.
    let senior = json!({ "title": "senior", "role": "reviewer", "adapter": "critic", "permission": "read", "rank": 5 });
    add_row(&rig, "agents", senior).await;
    change(&rig, "stages", "review", json!({ "seats": 1, "picks": "rank" })).await;
    let by = |id: String| async {
        let passed = rig.event("review.passed", move |event| event.data["card"] == id.as_str()).await;
        passed.data["by"].as_str().unwrap().to_string()
    };
    assert_eq!(by(worked(&rig, "Add a cart").await).await, "shop-senior-1");

    // Ranked below them from now, it is not the one: of the others, who
    // are ranked alike, it is the one brought up first.
    change(&rig, "agents", "senior", json!({ "rank": -1 })).await;
    assert_eq!(by(worked(&rig, "Add a till").await).await, "shop-reviewer-1");
    // And with nothing said of how they are chosen between, rank is not read at all.
    change(&rig, "agents", "senior", json!({ "rank": 9 })).await;
    change(&rig, "stages", "review", json!({ "picks": null })).await;
    assert_eq!(by(worked(&rig, "Add a shelf").await).await, "shop-reviewer-1");
}

#[tokio::test]
async fn a_card_given_to_an_agent_is_worked_by_that_one_whoever_would_have_been_picked() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({ "workers": 2, "reviewers": 0 })).await;
    // Both have nothing in hand, and the first would take it: the card says the second.
    let id = add(&rig, "Add a cart", &[]).await;
    give(&rig, &id, "agent:shop-worker-2").await;
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "review", 0).await;
    assert_eq!(each(&rig).await, (0, 1));

    // A card given to a person is anybody's of the role, as one given to nobody is.
    let theirs = add(&rig, "Add a till", &[]).await;
    let me = rig.read("/api/me").await["actor"].as_str().unwrap().to_string();
    give(&rig, &theirs, &me).await;
    move_card(&rig, &theirs, "ready", None).await;
    moved(&rig, &theirs, "review", 0).await;
    assert_eq!(each(&rig).await, (1, 1));

    // One given to an agent that is not there is given to nobody else, and that is said.
    let nobodys = add(&rig, "Add a shelf", &[]).await;
    give(&rig, &nobodys, "agent:shop-worker-7").await;
    move_card(&rig, &nobodys, "ready", None).await;
    let said = format!("{nobodys}: shop-worker-7, whom the card is given to, is not an agent that is up");
    rig.event("channel.message", move |event| {
        event.data["body"].as_str().unwrap_or_default().starts_with(&said)
    })
    .await;
    assert_eq!(each(&rig).await, (1, 1));
}

#[tokio::test]
async fn who_a_card_goes_to_is_a_rule_that_is_asked_and_a_project_may_have_its_own() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({ "workers": 2, "reviewers": 0 })).await;
    // As it ships the rule says the first of those who could take it,
    // which is who the stage's row puts first.
    worked(&rig, "Add a cart").await;
    assert_eq!(each(&rig).await, (1, 0));

    // The project's own, saved by its owner: cards about the till are the
    // second worker's, and every other card goes as the row says.
    let rule = "blueprint: std/dispatch\nparams:\n  about: { type: any, default: {} }\n  agents: { type: list, default: [] }\noutputs:\n  take: \"${ contains(lower(params.about.title ?? ''), 'till') ? 'shop-worker-2' : params.agents[0].name }\"\n";
    let saved =
        json!({ "path": format!("@blueprints/{}", brand::blueprint_file("std/dispatch")), "content": rule });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(saved)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    worked(&rig, "Add a till").await;
    assert_eq!(each(&rig).await, (1, 1));
    worked(&rig, "Add a shelf").await;
    assert_eq!(each(&rig).await, (2, 1));

    // A rule that names somebody who could not take the work has said
    // nothing: the card goes as the row says, and its step's log says why.
    let wrong = rule.replace("shop-worker-2", "shop-worker-9");
    let saved =
        json!({ "path": format!("@blueprints/{}", brand::blueprint_file("std/dispatch")), "content": wrong });
    rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(saved)).await;
    worked(&rig, "Mend the till").await;
    assert_eq!(each(&rig).await, (3, 1));
}

#[tokio::test]
async fn what_each_who_looked_chose_and_what_the_stage_came_to_are_counted() {
    let (rig, at) = rig().await;
    shop(&rig, json!({})).await;
    // The one who reviews finds something the first time it is shown the card.
    says(&at, "shop-reviewer-1", "Nothing tests it.", "send_back");
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    // In doing when it is taken, and again when it is sent back.
    let taken = moved(&rig, &id, "doing", 0).await;
    let back = moved(&rig, &id, "doing", taken).await;
    // Then it finds nothing, and the card passes.
    std::fs::remove_file(at.join("says-shop-reviewer-1")).unwrap();
    moved(&rig, &id, "done", back).await;

    // What it chose each time, in the words it was given to choose
    // between: how often an agent approves is asked for, and is nobody's
    // opinion of what a score is.
    assert_eq!(
        counted(&rig, "agent.turn", &["slot", "decision"]).await,
        [
            (json!({ "decision": "approve", "slot": "shop-reviewer-1" }), 1),
            (json!({ "decision": "send_back", "slot": "shop-reviewer-1" }), 1),
        ]
    );
    // And what the stage came to, each time it was shown a card.
    assert_eq!(
        counted(&rig, "review.verdict", &["stage", "verdict"]).await,
        [
            (json!({ "stage": "review", "verdict": "failed" }), 1),
            (json!({ "stage": "review", "verdict": "passed" }), 1),
        ]
    );
}
