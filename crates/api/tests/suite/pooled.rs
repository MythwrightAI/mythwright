//! A fleet works as many cards at once as it says, and the rest wait their
//! turn: where they are, saying that they wait and what for.
//!
//! The agent here does not finish until the test lets it, so that a card is
//! still being worked when the next becomes ready.

use std::path::PathBuf;

use serde_json::{Value, json};

use crate::lanes::{WORKER, history, move_card, moved, shop};
use crate::{Rig, soon};

/// A node whose agent reads its prompt and then waits at a hold until
/// the test lets it go; and the hold. Where there is a hold for the card
/// it was given (`gate-<card>`, beside the other), it waits at that one:
/// two agents at work at once are then let go one at a time.
async fn rig() -> (Rig, PathBuf) {
    let mut gate = None;
    let rig = Rig::with("api-pooled", |config, dir| {
        let hold = dir.join("gate");
        crate::hold(&hold);
        // Which card it was given is in what it is asked: `Card <id>: ...`.
        let waits = format!("{0}-%:picked%|{0}", hold.display());
        let does = ["read", "pick", "Card ", ":", "held", &waits, "%:picked%"];
        let changes = ["append", "CHANGES.md", "changed", "say", "I changed a file."];
        config.adapters.push(("writer".into(), crate::standin::argv(&[&does[..], &changes[..]].concat())));
        gate = Some(hold);
    })
    .await;
    (Rig::owned(rig).await, gate.unwrap())
}

/// Lets the agent that is waiting at the hold finish.
async fn let_go(gate: &std::path::Path) {
    crate::let_go("an agent to be waiting at the gate", gate).await;
}

async fn add(rig: &Rig, title: &str) -> String {
    let card = rig.send("/api/board/shop/items", json!({ "title": title })).await;
    card["id"].as_str().unwrap().to_string()
}

async fn card(rig: &Rig, id: &str) -> Value {
    let board = rig.read("/api/board/shop").await;
    board["items"].as_array().unwrap().iter().find(|card| card["id"] == id).cloned().unwrap()
}

/// Where a card is, and what it says it waits for.
async fn stands(rig: &Rig, id: &str) -> (String, String) {
    let card = card(rig, id).await;
    let text = |value: &Value| value.as_str().unwrap_or_default().to_string();
    (text(&card["status"]), text(&card["more"]["waiting"]))
}

/// Waits until a card has said that it waits, after `after` in the journal.
async fn waits(rig: &Rig, id: &str, after: u64) -> u64 {
    let card = id.to_string();
    let said = rig
        .event("board.item.updated", move |event| {
            event.seq > after
                && event.data["id"] == card.as_str()
                && event.data["changed"] == json!(["waiting"])
        })
        .await;
    said.seq
}

/// How many turns an agent has been given.
async fn given(rig: &Rig, agent: &str) -> usize {
    rig.read(&format!("/api/agents/{agent}/turns")).await["turns"].as_array().unwrap().len()
}

/// How many turns the fleet's one worker has been given.
async fn turns(rig: &Rig) -> usize {
    given(rig, WORKER).await
}

/// The pool of the stage that works cards, by the name it answers to.
const POOL: &str = "shop/ready";

/// How a pool keeps a card that asked for a place by its name.
fn named(id: &str) -> String {
    format!("key:{id}")
}

/// Who holds a place in that pool and who waits for one.
///
/// How many wait is not what a fleet says of its cards. A card two things
/// found due (its stage's look at its column, and the saying of its move)
/// asked twice, and is kept among those waiting, for its next turn, while
/// it holds its place: so who holds one is held to, and of those waiting
/// only that a card which is to wait is among them.
async fn pool(rig: &Rig) -> (Vec<String>, Vec<String>) {
    pool_of(rig, POOL).await
}

/// Who holds a place in the pool of this name and who waits for one.
async fn pool_of(rig: &Rig, name: &str) -> (Vec<String>, Vec<String>) {
    let all = rig.read("/api/instances?live=true&project=shop").await;
    let mut state = None;
    for one in all["instances"].as_array().unwrap().iter().filter(|one| one["blueprint"] == "std/pool") {
        let one =
            rig.read(&format!("/api/instances/{}", one["id"].as_str().unwrap())).await["instance"].clone();
        if one["params"]["name"] == name {
            state = Some(one["state"].clone());
        }
    }
    let state = state.unwrap_or_else(|| panic!("there is no pool called {name}"));
    let names = |list: &Value| -> Vec<String> {
        list.as_array().unwrap().iter().map(|name| name.as_str().unwrap().to_string()).collect()
    };
    (names(&state["held"]), names(&state["waiting"]))
}

/// Waits until nothing holds a place in that pool and nothing waits for one.
async fn pool_is_empty(rig: &Rig) {
    pool_of_is_empty(rig, POOL).await;
}

/// The same, of the pool of this name.
async fn pool_of_is_empty(rig: &Rig, name: &str) {
    let mut changed = rig.follow_after(rig.mark(), "instance.state");
    soon("the pool to be empty", async {
        while pool_of(rig, name).await != (Vec::new(), Vec::new()) {
            changed.next().await.unwrap();
        }
    })
    .await;
}

#[tokio::test]
async fn a_fleet_works_one_card_at_a_time_and_the_next_waits_in_ready_and_says_what_for() {
    let (rig, gate) = rig().await;
    shop(&rig, json!({})).await;
    let (first, second) = (add(&rig, "Add a cart").await, add(&rig, "Add a till").await);

    // The first is taken at once, and its agent is at work.
    move_card(&rig, &first, "ready", None).await;
    moved(&rig, &first, "doing", 0).await;
    rig.event("agent.turn.started", |_| true).await;

    // The second becomes ready while that goes on. It is not taken: it
    // stays where it is, and says that it waits its turn and why.
    move_card(&rig, &second, "ready", None).await;
    waits(&rig, &second, 0).await;
    assert_eq!(
        stands(&rig, &second).await,
        ("ready".to_string(), "one card is worked at a time here".to_string())
    );
    assert_eq!(turns(&rig).await, 1, "its agent has been given nothing of it");
    let (held, waiting) = pool(&rig).await;
    assert_eq!(held, [named(&first)], "the first has the one place");
    assert!(waiting.contains(&named(&second)), "the second waits for it: {waiting:?}");
    history(&rig, &second, "it waits its turn: one card is worked at a time here").await;

    // The first goes to review, and the second's turn has come: it is
    // taken, says so, and waits for nothing.
    let now = rig.mark();
    let_go(&gate).await;
    moved(&rig, &first, "review", now).await;
    moved(&rig, &second, "doing", now).await;
    waits(&rig, &second, now).await;
    assert_eq!(stands(&rig, &second).await, ("doing".to_string(), String::new()));
    let_go(&gate).await;
    moved(&rig, &second, "review", now).await;

    let said = history(&rig, &second, "conductor moved it from doing to review").await;
    // As far as its coming to review: the fleet has nobody in the role that
    // reviews, so that it waits for a person may be said a moment after it.
    let came = said.iter().position(|line| line == "conductor moved it from doing to review").unwrap();
    let journey: Vec<&str> =
        said[..=came].iter().map(String::as_str).filter(|line| !line.starts_with("Ada ")).collect();
    assert_eq!(
        journey,
        [
            "it waits its turn: one card is worked at a time here",
            "its turn came",
            "conductor moved it from ready to doing",
            "shop-worker-1 worked on it",
            "conductor moved it from doing to review",
        ]
    );
    assert_eq!(turns(&rig).await, 2);
    // Both rounds are over, and nothing holds a place or waits for one.
    pool_is_empty(&rig).await;
}

#[tokio::test]
async fn a_fleet_told_to_start_nothing_says_why_on_the_card_and_in_the_channel_and_goes_on_when_told() {
    let (rig, gate) = rig().await;
    shop(&rig, json!({})).await;
    let id = add(&rig, "Add a cart").await;

    // Nothing is being worked, and still nothing new is to begin.
    rig.send("/api/emit/pool.pause", json!({ "pool": POOL, "why": "the bill is being looked at" })).await;
    rig.event("pool.paused", |_| true).await;
    move_card(&rig, &id, "ready", None).await;
    waits(&rig, &id, 0).await;
    assert_eq!(
        stands(&rig, &id).await,
        (
            "ready".to_string(),
            "no work is being started here for now: the bill is being looked at".to_string()
        )
    );
    let said = rig
        .event("channel.message", |event| {
            event.data["body"].as_str().is_some_and(|body| body.starts_with("No new work is being started"))
        })
        .await;
    assert_eq!(
        said.data["body"],
        "No new work is being started for now: the bill is being looked at. What is under way goes on."
    );
    assert_eq!(turns(&rig).await, 0);

    // Told to go on, it takes the card that waited.
    let now = rig.mark();
    rig.send("/api/emit/pool.resume", json!({ "pool": POOL })).await;
    moved(&rig, &id, "doing", now).await;
    rig.event("channel.message", |event| {
        event.data["body"] == "Cards in ready are worked 1 at a time from now."
    })
    .await;
    let_go(&gate).await;
    moved(&rig, &id, "review", now).await;
    // That it waited is taken off the card as its turn comes, by another
    // handler than the one that moves it: it is waited for, and not read
    // in the instant after the move. A stage that is coming up says a
    // card is due more than once, and the card is asked for again while
    // it waits; where that asking was what the pool gave the place for,
    // as it went on, the card was not said to have waited and went on
    // saying that it did.
    waits(&rig, &id, now).await;
    assert_eq!(stands(&rig, &id).await, ("review".to_string(), String::new()));
}

#[tokio::test]
async fn a_card_taken_out_of_the_way_while_it_waited_is_not_worked_until_it_is_made_ready_again() {
    let (rig, gate) = rig().await;
    shop(&rig, json!({})).await;
    let (first, second) = (add(&rig, "Add a cart").await, add(&rig, "Add a till").await);
    move_card(&rig, &first, "ready", None).await;
    rig.event("agent.turn.started", |_| true).await;
    move_card(&rig, &second, "ready", None).await;
    waits(&rig, &second, 0).await;

    // Whoever owns the board thinks better of the second, before its turn.
    move_card(&rig, &second, "draft", None).await;
    let now = rig.mark();
    let_go(&gate).await;
    moved(&rig, &first, "review", now).await;
    // Its turn comes, and it is left where it was put: its place is given
    // back, and no agent is given it.
    pool_is_empty(&rig).await;
    let turn_given_back = rig.mark();
    assert_eq!(stands(&rig, &second).await.0, "draft");
    assert_eq!(turns(&rig).await, 1);

    // Made ready again, it is taken up as a card is the first time, and worked.
    move_card(&rig, &second, "ready", None).await;
    moved(&rig, &second, "doing", turn_given_back).await;
    let_go(&gate).await;
    moved(&rig, &second, "review", turn_given_back).await;
    assert_eq!(turns(&rig).await, 2);
}

/// A pipe for the agent that is given this card to wait at, and the pipe.
fn gate_of(gate: &std::path::Path, id: &str) -> PathBuf {
    let pipe = PathBuf::from(format!("{}-{id}", gate.display()));
    if !pipe.exists() {
        crate::hold(&pipe);
    }
    pipe
}

#[tokio::test]
async fn a_fleet_that_says_two_at_once_takes_two_and_has_the_third_wait_and_takes_it_when_given_more_room() {
    let (rig, gate) = rig().await;
    // Two who work: two cards are worked at once. Each agent waits at a
    // pipe of its card's own, so that they are let go one at a time.
    shop(&rig, json!({ "workers": 2 })).await;
    let cards = [add(&rig, "One").await, add(&rig, "Two").await, add(&rig, "Three").await];
    for id in &cards {
        gate_of(&gate, id);
    }
    for id in &cards[..2] {
        move_card(&rig, id, "ready", None).await;
        moved(&rig, id, "doing", 0).await;
    }
    move_card(&rig, &cards[2], "ready", None).await;
    waits(&rig, &cards[2], 0).await;
    assert_eq!(
        stands(&rig, &cards[2]).await,
        ("ready".to_string(), "2 cards are worked at a time here".to_string())
    );
    let (held, waiting) = pool(&rig).await;
    let held: std::collections::BTreeSet<String> = held.into_iter().collect();
    assert_eq!(held, [named(&cards[0]), named(&cards[1])].into(), "the first two have the two places");
    assert!(waiting.contains(&named(&cards[2])), "the third waits for one: {waiting:?}");

    // The row says three who work: the pool the stage keeps has three
    // places, and the card that waited is taken.
    let now = rig.mark();
    crate::fleets::change(&rig, "agents", "worker", json!({ "count": 3 })).await;
    moved(&rig, &cards[2], "doing", now).await;
    assert_eq!(pool(&rig).await.0.len(), 3);

    for id in &cards {
        let_go(&gate_of(&gate, id)).await;
        moved(&rig, id, "review", now).await;
    }
}

#[tokio::test]
async fn told_no_agent_a_card_is_given_to_one_with_nothing_in_hand_and_every_round_after_is_that_agents() {
    let (rig, gate) = rig().await;
    // Two who work, and no card says which of them it is for.
    shop(&rig, json!({ "workers": 2 })).await;
    let (first, second) = (add(&rig, "Add a cart").await, add(&rig, "Add a till").await);
    for id in [&first, &second] {
        gate_of(&gate, id);
    }
    let asked = async |rig: &Rig| (given(rig, "shop-worker-1").await, given(rig, "shop-worker-2").await);
    move_card(&rig, &first, "ready", None).await;
    moved(&rig, &first, "doing", 0).await;
    rig.event("agent.turn.started", |_| true).await;
    // One agent has the first card in hand: the second card goes to the
    // other, and does not wait behind it.
    move_card(&rig, &second, "ready", None).await;
    moved(&rig, &second, "doing", 0).await;
    rig.count("agent.turn.started", 2).await;
    assert_eq!(asked(&rig).await, (1, 1));

    // Both finish.
    let now = rig.mark();
    let_go(&gate_of(&gate, &first)).await;
    let_go(&gate_of(&gate, &second)).await;
    moved(&rig, &first, "review", now).await;
    let reviewed = moved(&rig, &second, "review", now).await;
    crate::lanes::at_rest(&rig, reviewed).await;

    // The second card is sent back. Both agents are free, and the first
    // would be whichever comes first: it goes to the one that worked it,
    // in whose conversation it was worked.
    let now = rig.mark();
    move_card(&rig, &second, "doing", Some("The total is wrong.")).await;
    rig.event("agent.turn.started", move |event| event.seq > now).await;
    assert_eq!(asked(&rig).await, (1, 2));
    let_go(&gate_of(&gate, &second)).await;
    moved(&rig, &second, "review", now).await;
}

#[tokio::test]
async fn a_card_asked_for_again_while_it_is_in_hand_is_not_told_that_it_waits_its_turn_behind_itself() {
    let (rig, gate) = rig().await;
    shop(&rig, json!({})).await;
    let id = add(&rig, "Add a cart").await;
    move_card(&rig, &id, "ready", None).await;
    // It is in hand: its round has the one place, and its agent is at work.
    rig.event("agent.turn.started", |_| true).await;

    // Its turn is asked for a second time by its name, as it is where two
    // things each found it due (a stage's look at its column, and the
    // saying of its move). Here by a text of its own, so that it is
    // certain to be while the card holds its place.
    let asks = format!(
        "blueprint: again\nsteps:\n  ask:\n    do: emit\n    with: {{ topic: pool.asked, pool: {POOL}, as: {id}, about: {id} }}\n"
    );
    let applied = rig.send("/api/apply", json!({ "source": asks, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    // The pool has taken both askings in: the name holds its place and waits for its next.
    let mut kept = rig.follow("instance.state");
    soon("the pool to keep the name as holding and as waiting", async {
        while pool(&rig).await != (vec![named(&id)], vec![named(&id)]) {
            kept.next().await.unwrap();
        }
    })
    .await;

    // Its round ends, its place is given back and is its own again; the
    // turn nobody meant is given back in its turn, and the pool is empty.
    let_go(&gate).await;
    moved(&rig, &id, "review", 0).await;
    pool_is_empty(&rig).await;

    // Nothing was said of its waiting, at any point: not on the card, and not in its history.
    assert_eq!(stands(&rig, &id).await, ("review".to_string(), String::new()));
    let said = history(&rig, &id, "conductor moved it from doing to review").await;
    let of_waiting: Vec<&String> = said
        .iter()
        .filter(|line| line.contains("waits its turn") || line.contains("its turn came"))
        .collect();
    assert!(of_waiting.is_empty(), "{said:#?}");
    assert_eq!(rig.count_of("card.waiting").await + rig.count_of("pool.waiting").await, 0);
}

/// The stages a fleet starts with, the one that works saying `cells` besides.
fn working_says(cells: Value) -> Value {
    let mut stages = crate::fleets::params(json!({}))["stages"].clone();
    for (cell, value) in cells.as_object().unwrap() {
        stages[0][cell] = value.clone();
    }
    stages
}

/// A card added with this priority: a word of the board's, or nothing.
async fn add_ranked(rig: &Rig, title: &str, priority: Option<&str>) -> String {
    let card = rig
        .send("/api/board/shop/items", json!({ "title": title, "fields": { "priority": priority } }))
        .await;
    card["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn a_stage_that_takes_its_cards_by_priority_takes_the_highest_of_those_waiting_next_and_of_those_as_high_the_first()
 {
    let (rig, gate) = rig().await;
    shop(&rig, json!({ "stages": working_says(json!({ "next": "priority" })) })).await;
    let first = add_ranked(&rig, "Add a cart", Some("low")).await;
    let (low, high) = (
        add_ranked(&rig, "Add a till", Some("low")).await,
        add_ranked(&rig, "Fix the price", Some("urgent")).await,
    );
    let later = add_ranked(&rig, "Add a shelf", Some("urgent")).await;
    // Nothing said is `normal`: above the low one, below the urgent ones.
    let plain = add_ranked(&rig, "Add a basket", None).await;

    // The first is taken at once; the other four wait, the low one having
    // come first, the two urgent ones after it and the one that says nothing
    // last.
    move_card(&rig, &first, "ready", None).await;
    moved(&rig, &first, "doing", 0).await;
    rig.event("agent.turn.started", |_| true).await;
    for id in [&low, &high, &later, &plain] {
        move_card(&rig, id, "ready", None).await;
        waits(&rig, id, 0).await;
    }

    // Each place given back goes to the highest of those still waiting, and
    // of the two as high to the one that came first: told by which card was
    // taken while the others were still waiting in ready, and never by how
    // many rounds there were.
    let mut taken = Vec::new();
    let mut waiting = vec![low.clone(), high.clone(), later.clone(), plain.clone()];
    let mut now = rig.mark();
    while !waiting.is_empty() {
        let_go(&gate).await;
        let next =
            rig.event("board.item.moved", move |event| event.seq > now && event.data["to"] == "doing").await;
        let id = next.data["id"].as_str().unwrap().to_string();
        waiting.retain(|other| *other != id);
        for other in &waiting {
            assert_eq!(stands(&rig, other).await.0, "ready", "{other} still waits as {id} is taken");
        }
        taken.push(id);
        now = next.seq;
    }
    assert_eq!(taken, [high, later, plain, low]);
    let_go(&gate).await;
    pool_is_empty(&rig).await;
}

#[tokio::test]
async fn a_stage_that_reviews_its_cards_by_priority_begins_the_review_of_the_highest_of_those_waiting_next() {
    // One reviewer, who waits at a hold until it is let go: one card is
    // reviewed at a time, and what passes is done with nothing merged.
    let (rig, at) = crate::fleets::rig().await;
    let mut stages = crate::fleets::params(json!({}))["stages"].clone();
    stages[1]["next"] = json!("priority");
    stages[1]["merge"] = json!(false);
    crate::fleets::shop(&rig, json!({ "stages": stages })).await;
    let hold = at.join("hold");
    crate::hold(&hold);
    let first = add_ranked(&rig, "Add a cart", Some("low")).await;
    let (low, high) = (
        add_ranked(&rig, "Add a till", Some("low")).await,
        add_ranked(&rig, "Fix the price", Some("urgent")).await,
    );
    let later = add_ranked(&rig, "Add a shelf", Some("urgent")).await;
    // Nothing said is `normal`: above the low one, below the urgent ones.
    let plain = add_ranked(&rig, "Add a basket", None).await;

    // The first is put to review and its reviewer is at it; the other
    // four are put there after it and wait, the low one having come
    // first. A review asks its pool saying no priority of its own: the
    // pool reads the card's.
    move_card(&rig, &first, "review", None).await;
    rig.event("agent.turn.started", |_| true).await;
    for id in [&low, &high, &later, &plain] {
        move_card(&rig, id, "review", None).await;
        waits(&rig, id, 0).await;
    }

    // Each review that ends gives its place to the highest of those still
    // waiting, and of the two as high to the one that came first: told by
    // whose turn came (what its review says once it is given its place),
    // and never by how many reviews there were.
    let mut taken = Vec::new();
    let mut waiting = vec![low.clone(), high.clone(), later.clone(), plain.clone()];
    let mut now = rig.mark();
    while !waiting.is_empty() {
        crate::let_go("a reviewer to be waiting", &hold).await;
        let next = rig.event("card.taken", move |event| event.seq > now).await;
        let id = next.data["card"].as_str().unwrap().to_string();
        assert!(waiting.contains(&id), "{id} was taken, of {waiting:?}");
        waiting.retain(|other| *other != id);
        taken.push(id);
        now = next.seq;
    }
    assert_eq!(taken, [high, later, plain, low]);
    crate::let_go("a reviewer to be waiting", &hold).await;
    pool_of_is_empty(&rig, "shop/review").await;
}

/// The limit and the order of the pool of the stage that works, as it was applied.
async fn pool_told(rig: &Rig) -> (Value, Value) {
    let all = rig.read("/api/instances?live=true&project=shop").await;
    for one in all["instances"].as_array().unwrap().iter().filter(|one| one["blueprint"] == "std/pool") {
        let one =
            rig.read(&format!("/api/instances/{}", one["id"].as_str().unwrap())).await["instance"].clone();
        if one["params"]["name"] == POOL {
            return (one["params"]["limit"].clone(), one["params"]["order"].clone());
        }
    }
    panic!("the stage that works has no pool");
}

#[tokio::test]
async fn a_stage_held_to_one_card_at_a_time_takes_one_with_two_agents_and_never_more_than_its_agents_can_take()
 {
    let (rig, gate) = rig().await;
    shop(&rig, json!({ "workers": 2, "stages": working_says(json!({ "at_once": 1 })) })).await;
    assert_eq!(pool_told(&rig).await, (json!(1), json!("first")));
    let (first, second) = (add(&rig, "Add a cart").await, add(&rig, "Add a till").await);
    for id in [&first, &second] {
        gate_of(&gate, id);
    }

    // One agent has the first card, and the other has nothing in hand: the
    // second card waits all the same, saying why.
    move_card(&rig, &first, "ready", None).await;
    moved(&rig, &first, "doing", 0).await;
    rig.event("agent.turn.started", |_| true).await;
    move_card(&rig, &second, "ready", None).await;
    waits(&rig, &second, 0).await;
    assert_eq!(
        stands(&rig, &second).await,
        ("ready".to_string(), "one card is worked at a time here".to_string())
    );
    assert_eq!(pool(&rig).await.0, [named(&first)]);
    assert_eq!(given(&rig, "shop-worker-1").await + given(&rig, "shop-worker-2").await, 1);

    // Said to take more than its two agents can, it takes as many as they
    // can: the second card is taken, and the pool has two places.
    let now = rig.mark();
    crate::fleets::change(&rig, "stages", "work", json!({ "at_once": 5 })).await;
    moved(&rig, &second, "doing", now).await;
    assert_eq!(pool_told(&rig).await, (json!(2), json!("first")));
    for id in [&first, &second] {
        let_go(&gate_of(&gate, id)).await;
        moved(&rig, id, "review", now).await;
    }
}
