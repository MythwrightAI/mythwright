//! A feature: something wanted that takes several cards. It is a record of
//! a table of its own that is a board of its own, with a blueprint of its
//! own for each (`std/features`, `std/feature`): planned by an agent,
//! accepted by a person, and done when the cards its plan lists are.

use axum::http::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add, add_row, card, given, rig, says, shop};
use crate::lanes::moved;

const ARCHITECT: &str = "shop-architect-1";
const REVIEWER: &str = "shop-reviewer-1";

/// The shop's fleet with an architect in it, and the shop's features.
async fn featured(rig: &Rig) {
    featured_with(rig, json!({}), json!({})).await;
}

/// The same, the fleet's rows starting as `fleet` says and the features
/// given what `more` says beside the project.
async fn featured_with(rig: &Rig, fleet: Value, more: Value) {
    shop(rig, fleet).await;
    let architect =
        json!({ "title": "architect", "role": "architect", "adapter": "critic", "permission": "read" });
    add_row(rig, "agents", architect).await;
    let mut params = json!({ "project": "shop", "channel": "shop" });
    for (name, value) in more.as_object().unwrap() {
        params[name] = value.clone();
    }
    let asked = json!({ "blueprint": "std/features", "project": "shop", "params": params, "wait": true });
    let applied = rig.send("/api/apply", asked).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
}

/// Asks for a feature that gathers cards there already: its id.
async fn gather(rig: &Rig, title: &str, fields: Value) -> String {
    let mut all = json!({ "title": title, "status": "backlog" });
    for (name, value) in fields.as_object().unwrap() {
        all[name] = value.clone();
    }
    rig.send("/api/collections/shop/features/records", json!({ "fields": all })).await["id"]
        .as_str()
        .unwrap()
        .to_string()
}

/// The lines of a feature's history, oldest first.
async fn lines(rig: &Rig, id: &str) -> Vec<String> {
    let history = rig.read(&format!("/api/records/{id}/history")).await;
    history["history"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|line| line["said"].as_str())
        .map(str::to_string)
        .collect()
}

/// A plan that ends with these tickets.
fn plan(tickets: Value) -> String {
    format!("What there is to do, and in which order.\n\n```tickets\n{tickets}\n```")
}

/// Asks for a feature: its id.
async fn ask_for(rig: &Rig, title: &str, body: &str) -> String {
    let fields = json!({ "fields": { "title": title, "body": body, "status": "backlog" } });
    rig.send("/api/collections/shop/features/records", fields).await["id"].as_str().unwrap().to_string()
}

async fn feature(rig: &Rig, id: &str) -> Value {
    rig.read(&format!("/api/records/{id}")).await["data"].clone()
}

/// Waits for what is said of a feature under `topic`.
async fn said(rig: &Rig, topic: &str, id: &str) -> Value {
    let id = id.to_string();
    rig.event(topic, move |event| event.data["record"] == id.as_str()).await.data.clone()
}

async fn change(rig: &Rig, id: &str, fields: Value) {
    let (status, answer) = rig
        .call(
            Method::PATCH,
            &format!("/api/records/{id}"),
            Some(&rig.token),
            Some(json!({ "fields": fields })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
}

const PLAN: &str = "First what a cart holds, then the page that shows one.\n\n```tickets\n[\n  { \"title\": \"The cart's model\", \"body\": \"What a cart holds.\" },\n  { \"title\": \"The cart's page\", \"after\": [1] }\n]\n```";

#[tokio::test]
async fn a_feature_is_planned_by_an_agent_accepted_by_a_person_and_done_when_the_cards_its_plan_lists_are() {
    let (rig, at) = rig().await;
    featured(&rig).await;
    // Asked for, it is put to an agent of the role that plans, which says
    // how it would be built and which pieces of work that comes to.
    says(&at, ARCHITECT, PLAN, "propose");
    let id = ask_for(&rig, "A shopping cart", "People want to buy more than one thing.").await;
    assert_eq!(said(&rig, "feature.planned", &id).await["agent"], ARCHITECT);
    let planned = feature(&rig, &id).await;
    assert_eq!(planned["status"], "planned", "{planned}");
    assert!(planned["plan"].as_str().unwrap().contains("```tickets"), "{planned}");
    let told = given(&rig, ARCHITECT).await;
    assert!(
        told[0].starts_with("This feature is yours to plan.") && told[0].contains("A shopping cart"),
        "{told:?}"
    );
    // Nothing is on the board for it yet: nobody has accepted the plan.
    assert_eq!(rig.read("/api/board/shop").await["items"], json!([]));

    // Accepted by being moved on, it comes to a card for each ticket: each
    // a piece of it, the second waiting on the first, and both ready.
    change(&rig, &id, json!({ "status": "building" })).await;
    let accepted = said(&rig, "feature.accepted", &id).await;
    assert_eq!((accepted["count"].as_u64(), accepted["cards"].as_str()), (Some(2), Some("shop-1, shop-2")));
    let (first, second) = (card(&rig, "shop-1").await, card(&rig, "shop-2").await);
    assert_eq!((&first["title"], &second["title"]), (&json!("The cart's model"), &json!("The cart's page")));
    assert_eq!(second["more"]["after"], json!(["shop-1"]), "{second}");
    assert_eq!(first["more"]["part_of"], id.as_str(), "{first}");
    // Each was added by the feature's blueprint from what its agent wrote,
    // which is the factory's bot's doing, and its history says so, not
    // whoever applied the features.
    let wrote = format!("conductor added it to draft from what {ARCHITECT} wrote");
    assert_eq!(crate::wrote::first(&rig, "shop-1").await.0, wrote);

    // They are worked and looked over as any card is, in the order the
    // plan said; and when the last of them is done, so is the feature.
    said(&rig, "feature.done", &id).await;
    assert_eq!(feature(&rig, &id).await["status"], "done");
    assert_eq!(
        (card(&rig, "shop-1").await["status"].clone(), card(&rig, "shop-2").await["status"].clone()),
        (json!("done"), json!("done"))
    );
    let history = rig.read(&format!("/api/records/{id}/history")).await;
    let lines: Vec<&str> =
        history["history"].as_array().unwrap().iter().filter_map(|line| line["said"].as_str()).collect();
    assert!(lines.iter().any(|line| line.ends_with("said how it would be built")), "{lines:?}");
    assert!(lines.iter().any(|line| line.contains("came to 2 card(s): shop-1, shop-2")), "{lines:?}");
    assert!(lines.contains(&"every card of it is done"), "{lines:?}");
}

#[tokio::test]
async fn an_agent_that_cannot_tell_what_a_feature_wants_asks_and_a_plan_with_no_tickets_is_one_card() {
    let (rig, at) = rig().await;
    featured(&rig).await;
    says(&at, ARCHITECT, "Which shop is it for?", "ask");
    let id = ask_for(&rig, "A shopping cart", "").await;
    assert_eq!(said(&rig, "feature.asked", &id).await["question"], "Which shop is it for?");
    assert_eq!(feature(&rig, &id).await["question"], "Which shop is it for?");
    assert_eq!(feature(&rig, &id).await["status"], "backlog");

    // What somebody says on the feature is put to the same agent.
    says(&at, ARCHITECT, "One piece: the cart, model and page together.", "propose");
    let words = json!({ "body": "The one on the high street." });
    rig.send(&format!("/api/records/{id}/comments"), words).await;
    said(&rig, "feature.planned", &id).await;
    let told = given(&rig, ARCHITECT).await;
    assert!(
        told[1].contains("Somebody said this on the feature:") && told[1].contains("the high street"),
        "{told:?}"
    );
    let planned = feature(&rig, &id).await;
    assert_eq!((&planned["status"], &planned["question"]), (&json!("planned"), &json!("")), "{planned}");

    // Whoever applied the fleet was told of each as it waited for a
    // person, in their inbox: that the feature's agent asked, and that
    // its plan waits to be accepted.
    for why in ["asked", "plan"] {
        let about = id.clone();
        rig.event("inbox.told", move |event| {
            event.data["about"] == about.as_str() && event.data["count"] == 1
        })
        .await;
        crate::tells::heard(&rig, if why == "asked" { "on.wondered" } else { "on.planned" }, 0).await;
    }
    let asked = crate::tells::lines(&rig, &rig.token, "asked").await;
    assert_eq!(asked, [(format!("{id}: {ARCHITECT} asks: Which shop is it for?"), 1)]);
    let waits = crate::tells::lines(&rig, &rig.token, "plan").await;
    let accept = "Move it on to accept it, or say on it what should be otherwise.";
    assert_eq!(
        waits,
        [(format!("{id}: {ARCHITECT} has said how it would be done, and the plan waits. {accept}"), 1)]
    );

    // Somebody puts its plan in their own words, which are then theirs and
    // not its agent's.
    // Its handlers take turns, so the change is heard before the move.
    let ours = "One piece: the cart, model and page, together and soon.";
    change(&rig, &id, json!({ "plan": ours })).await;
    // Accepted, a plan that lists no tickets is one piece of work: a card
    // that is the feature, carrying its plan for whoever works it.
    change(&rig, &id, json!({ "status": "building" })).await;
    let accepted = said(&rig, "feature.accepted", &id).await;
    assert_eq!(accepted["count"], 1, "{accepted}");
    let whole = card(&rig, "shop-1").await;
    assert_eq!(whole["title"], "A shopping cart");
    assert_eq!(whole["more"]["plan"], ours, "{whole}");
    assert_eq!(crate::wrote::first(&rig, "shop-1").await.0, "conductor added it to draft");
    assert_eq!(whole["more"]["part_of"], id.as_str());
}

#[tokio::test]
async fn cards_there_already_are_put_in_order_by_a_plan_a_person_accepts_and_worked_in_that_order() {
    let (rig, at) = rig().await;
    featured_with(&rig, json!({ "workers": 2 }), json!({})).await;
    // Three cards written up before anybody put them in order.
    let (till, shelves, opening) = (
        add(&rig, "The till", &[]).await,
        add(&rig, "The shelves", &[]).await,
        add(&rig, "The opening", &[]).await,
    );
    // Whoever plans puts them in order and says one thing more is needed:
    // the till and the shelves side by side, the opening after both, and
    // a party after the opening.
    let tickets = json!([
        { "card": till }, { "card": shelves }, { "card": opening, "after": [1, 2] },
        { "title": "The party", "body": "Invite the street.", "after": [3] },
    ]);
    says(&at, ARCHITECT, &plan(tickets), "propose");
    let id = gather(&rig, "Open the shop", json!({ "gathers": [till, shelves, opening] })).await;
    assert_eq!(said(&rig, "feature.planned", &id).await["agent"], ARCHITECT);
    // Its agent was shown the cards it gathers, each as it stands.
    let told = given(&rig, ARCHITECT).await;
    assert!(told[0].contains("It gathers these cards, which are on the board already."), "{told:?}");
    for card in
        ["shop-1, in draft: The till", "shop-2, in draft: The shelves", "shop-3, in draft: The opening"]
    {
        assert!(told[0].contains(card), "{card}: {told:?}");
    }
    // Nothing is put in order on the agent's word: the cards are as they were.
    let before = card(&rig, &opening).await;
    assert_eq!((&before["status"], &before["more"]["after"]), (&json!("draft"), &json!([])), "{before}");

    // Those who review wait at a gate, so that what is worked stays where
    // it is looked over until the test says.
    let gate = at.join(format!("hold-{REVIEWER}"));
    crate::hold(&gate);
    let from = rig.mark();
    change(&rig, &id, json!({ "status": "building" })).await;
    let accepted = said(&rig, "feature.accepted", &id).await;
    assert_eq!(
        (accepted["count"].as_u64(), accepted["cards"].as_str()),
        (Some(4), Some("shop-1, shop-2, shop-3, shop-4")),
        "{accepted}"
    );
    // Each card says what it waits on and that it is a piece of this: the
    // three that were there, and the one the plan added.
    let party = card(&rig, "shop-4").await;
    assert_eq!(
        (&party["title"], &party["more"]["after"]),
        (&json!("The party"), &json!(["shop-3"])),
        "{party}"
    );
    assert_eq!(card(&rig, &opening).await["more"]["after"], json!(["shop-1", "shop-2"]));
    for one in ["shop-1", "shop-2", "shop-3", "shop-4"] {
        assert_eq!(card(&rig, one).await["more"]["part_of"], id.as_str(), "{one}");
    }
    assert_eq!(feature(&rig, &id).await["cards"], json!(["shop-1", "shop-2", "shop-3", "shop-4"]));

    // The two that wait on nothing of each other are worked side by side:
    // both come to be looked over while nothing has been passed, and the
    // one that waits on them has not been begun.
    moved(&rig, &till, "review", from).await;
    moved(&rig, &shelves, "review", from).await;
    let waits = card(&rig, &opening).await;
    assert_eq!(waits["status"], "ready", "{waits}");
    // Passed, one after the other; then the one that waited on both is
    // worked, and after it the one the plan added; and the feature is done
    // when the last of them is. Whoever reviews is let go once for each
    // card, and the card's being done is waited for before the next: a
    // word written down the gate while the one before is still leaving
    // it is heard by nobody.
    let mut passed = from;
    for _ in 0..4 {
        crate::let_go("somebody who reviews to be waiting", &gate).await;
        passed = rig
            .event("board.item.moved", move |event| event.seq > passed && event.data["to"] == "done")
            .await
            .seq;
    }
    said(&rig, "feature.done", &id).await;
    for one in ["shop-1", "shop-2", "shop-3", "shop-4"] {
        assert_eq!(card(&rig, one).await["status"], "done", "{one}");
    }
    // The first time each was taken up, and when each was done.
    let (till_done, shelves_done) =
        (moved(&rig, &till, "done", from).await, moved(&rig, &shelves, "done", from).await);
    let (opening_begun, opening_done) =
        (moved(&rig, &opening, "doing", from).await, moved(&rig, &opening, "done", from).await);
    let party_begun = moved(&rig, "shop-4", "doing", from).await;
    assert!(opening_begun > till_done && opening_begun > shelves_done, "the opening waited on both");
    assert!(party_begun > opening_done, "the party waited on the opening");
    let history = lines(&rig, &id).await;
    assert!(
        history.iter().any(|line| line.contains("came to 4 card(s): shop-1, shop-2, shop-3, shop-4")),
        "{history:?}"
    );
}

#[tokio::test]
async fn a_plan_that_does_not_fit_the_board_is_refused_whole_and_taken_once_it_is_put_right() {
    let (rig, at) = rig().await;
    featured(&rig).await;
    let (till, shelves, sign) = (
        add(&rig, "The till", &[]).await,
        add(&rig, "The shelves", &[]).await,
        add(&rig, "The sign", &[]).await,
    );
    // A plan that takes up a card it was not asked to, and has two cards
    // each wait on the other.
    let wrong = json!([
        { "card": till, "after": [2] }, { "card": shelves, "after": [1] }, { "card": sign },
        { "title": "The party" },
    ]);
    says(&at, ARCHITECT, &plan(wrong), "propose");
    let id = gather(&rig, "Open the shop", json!({ "gathers": [till, shelves] })).await;
    said(&rig, "feature.planned", &id).await;
    change(&rig, &id, json!({ "status": "building" })).await;
    let refused = said(&rig, "feature.unaccepted", &id).await;
    let why = refused["why"].as_str().unwrap();
    assert!(
        why.contains("Ticket 3 is shop-3, which is not one of those this was asked to put in order (shop-1 and shop-2).")
            && why.contains("Shop-1 would wait on itself: shop-1 waits on shop-2, which waits on shop-1."),
        "{why}"
    );
    // Nothing of it was written: no card was added, none says what it
    // waits on or that it is a piece of anything, and the feature is back
    // where it waits with a plan, saying why in its history.
    let board = rig.read("/api/board/shop").await;
    assert_eq!(board["items"].as_array().unwrap().len(), 3, "{board}");
    for one in [&till, &shelves, &sign] {
        let card = card(&rig, one).await;
        assert_eq!((&card["more"]["after"], &card["status"]), (&json!([]), &json!("draft")), "{card}");
        assert!(card["more"]["part_of"].as_str().unwrap_or_default().is_empty(), "{card}");
    }
    let id_ = id.clone();
    rig.event("record.updated", move |event| {
        event.data["id"] == id_.as_str()
            && event.source.instance.is_some()
            && event.data["changed"] == json!(["status"])
    })
    .await;
    assert_eq!(feature(&rig, &id).await["status"], "planned");
    let history = lines(&rig, &id).await;
    assert!(
        history.iter().any(
            |line| line.starts_with("it was moved on, and its plan was not accepted: Ticket 3 is shop-3")
        ),
        "{history:?}"
    );
    assert!(crate::lanes::said(&rig).await.contains("was moved on, and its plan was not accepted"));

    // Somebody says what should be otherwise, its agent plans again, and
    // moved on again it is accepted.
    let right =
        json!([{ "card": till }, { "card": shelves, "after": [1] }, { "title": "The party", "after": [2] }]);
    says(&at, ARCHITECT, &plan(right), "propose");
    let from = rig.mark();
    rig.send(&format!("/api/records/{id}/comments"), json!({ "body": "The sign is not part of this." }))
        .await;
    let id_ = id.clone();
    rig.event("feature.planned", move |event| event.seq > from && event.data["record"] == id_.as_str()).await;
    change(&rig, &id, json!({ "status": "building" })).await;
    let accepted = said(&rig, "feature.accepted", &id).await;
    assert_eq!(accepted["cards"], "shop-1, shop-2, shop-4", "{accepted}");
    assert_eq!(card(&rig, &shelves).await["more"]["after"], json!(["shop-1"]));
    assert!(card(&rig, &sign).await["more"]["part_of"].as_str().unwrap_or_default().is_empty());
}

#[tokio::test]
async fn a_label_gathers_every_card_that_carries_it_when_the_feature_is_first_planned() {
    let (rig, at) = rig().await;
    featured(&rig).await;
    let labelled = |title: &'static str, labels: Value| {
        let rig = &rig;
        async move {
            let card = json!({ "title": title, "labels": labels });
            rig.send("/api/board/shop/items", card).await["id"].as_str().unwrap().to_string()
        }
    };
    let till = labelled("The till", json!(["opening"])).await;
    let shelves = labelled("The shelves", json!(["opening", "wood"])).await;
    let sign = labelled("The sign", json!(["later"])).await;
    // The plan leaves the cards as they stand, which is a plan too: they
    // are gathered, and nothing more is said of their order.
    says(&at, ARCHITECT, "They can all be done at once.", "propose");
    let id = gather(&rig, "Open the shop", json!({ "labelled": "opening" })).await;
    let gathered = said(&rig, "feature.gathered", &id).await;
    assert_eq!(
        (gathered["count"].as_u64(), gathered["cards"].as_str()),
        (Some(2), Some("shop-1 and shop-2"))
    );
    said(&rig, "feature.planned", &id).await;
    assert_eq!(feature(&rig, &id).await["gathers"], json!([till, shelves]));
    let told = given(&rig, ARCHITECT).await;
    assert!(
        told[0].contains("The till") && told[0].contains("The shelves") && !told[0].contains("The sign"),
        "{told:?}"
    );
    // A card given the label after that is not one of them: the group is
    // what it was when the plan was made.
    let late = labelled("The awning", json!(["opening"])).await;
    change(&rig, &id, json!({ "status": "building" })).await;
    let accepted = said(&rig, "feature.accepted", &id).await;
    assert_eq!((accepted["count"].as_u64(), accepted["cards"].as_str()), (Some(2), Some("shop-1, shop-2")));
    assert_eq!(card(&rig, &till).await["more"]["part_of"], id.as_str());
    for one in [&sign, &late] {
        assert!(card(&rig, one).await["more"]["part_of"].as_str().unwrap_or_default().is_empty(), "{one}");
    }
    assert!(lines(&rig, &id).await.iter().any(|line| line == "it gathers 2 card(s): shop-1 and shop-2"));
}

/// The credential an agent's program reads each time it asks the factory anything.
fn token_of(rig: &Rig, agent: &str) -> String {
    std::fs::read_to_string(rig.dir.join(format!("data/slots/{agent}/token"))).unwrap().trim().to_string()
}

async fn move_on_as(rig: &Rig, token: &str, id: &str) -> StatusCode {
    let fields = json!({ "fields": { "status": "building" } });
    rig.call(Method::PATCH, &format!("/api/records/{id}"), Some(token), Some(fields)).await.0
}

#[tokio::test]
async fn a_plan_is_a_persons_to_accept_who_holds_what_changing_a_card_takes_unless_the_features_say_anyone() {
    let (rig, at) = rig().await;
    featured(&rig).await;
    let till = add(&rig, "The till", &[]).await;
    says(&at, ARCHITECT, &plan(json!([{ "card": till }, { "title": "The party", "after": [1] }])), "propose");
    let id = gather(&rig, "Open the shop", json!({ "gathers": [till] })).await;
    said(&rig, "feature.planned", &id).await;

    // Its own agent holds what changing a record takes, and moves it on:
    // that accepts nothing. Nor does another agent's move.
    for agent in [ARCHITECT, REVIEWER] {
        let from = rig.mark();
        assert_eq!(move_on_as(&rig, &token_of(&rig, agent), &id).await, StatusCode::OK);
        let id_ = id.clone();
        let refused = rig
            .event("feature.unaccepted", move |event| {
                event.seq > from && event.data["record"] == id_.as_str()
            })
            .await;
        assert_eq!(
            refused.data["why"], "an agent moved it on, and its plan is a person's to accept.",
            "{agent}"
        );
        let id_ = id.clone();
        rig.event("record.updated", move |event| {
            event.seq > from && event.data["id"] == id_.as_str() && event.source.instance.is_some()
        })
        .await;
        assert_eq!(feature(&rig, &id).await["status"], "planned", "{agent}");
    }
    // Somebody with a key that may change records and not cards: the same,
    // saying what they lack. Asked of the key and not of whoever made it.
    let narrow = crate::held::key(&rig, &["projects", "channels", "records", "board:read"]).await;
    let from = rig.mark();
    assert_eq!(move_on_as(&rig, &narrow, &id).await, StatusCode::OK);
    let id_ = id.clone();
    let refused = rig
        .event("feature.unaccepted", move |event| event.seq > from && event.data["record"] == id_.as_str())
        .await;
    assert!(
        refused.data["why"].as_str().unwrap().starts_with("whoever moved it on does not hold `board:write`"),
        "{}",
        refused.data
    );
    let id_ = id.clone();
    rig.event("record.updated", move |event| {
        event.seq > from && event.data["id"] == id_.as_str() && event.source.instance.is_some()
    })
    .await;
    // Nothing was added and nothing changed for any of them.
    assert_eq!(rig.read("/api/board/shop").await["items"].as_array().unwrap().len(), 1);
    assert!(card(&rig, &till).await["more"]["part_of"].as_str().unwrap_or_default().is_empty());
    // A person with a key that may change cards accepts it.
    let wide = crate::held::key(&rig, &["projects", "channels", "records", "board"]).await;
    assert_eq!(move_on_as(&rig, &wide, &id).await, StatusCode::OK);
    assert_eq!(said(&rig, "feature.accepted", &id).await["cards"], "shop-1, shop-2");
}

#[tokio::test]
async fn where_the_features_say_anyone_accepts_an_agent_that_may_change_cards_does() {
    let (rig, at) = rig().await;
    featured_with(&rig, json!({}), json!({ "accepts": "anyone" })).await;
    says(&at, ARCHITECT, &plan(json!([{ "title": "The till" }])), "propose");
    let id = ask_for(&rig, "Open the shop", "").await;
    said(&rig, "feature.planned", &id).await;
    assert_eq!(move_on_as(&rig, &token_of(&rig, REVIEWER), &id).await, StatusCode::OK);
    assert_eq!(said(&rig, "feature.accepted", &id).await["count"], 1);
}

#[tokio::test]
async fn a_feature_whose_cards_were_all_done_before_it_was_accepted_is_done_then() {
    let (rig, at) = rig().await;
    featured(&rig).await;
    let till = add(&rig, "The till", &[]).await;
    crate::lanes::move_card(&rig, &till, "done", None).await;
    says(&at, ARCHITECT, "It is done already.", "propose");
    let id = gather(&rig, "Open the shop", json!({ "gathers": [till] })).await;
    said(&rig, "feature.planned", &id).await;
    change(&rig, &id, json!({ "status": "building" })).await;
    said(&rig, "feature.done", &id).await;
    assert_eq!(feature(&rig, &id).await["status"], "done");
    assert_eq!(card(&rig, &till).await["status"], "done");
}

/// Moves a planned feature on in a daemon that takes nothing more once
/// it has said the first event `at` holds of, and gives when the journal
/// has shut there.
///
/// It goes by `Journal::shut()` and by nothing sooner. This was told
/// that the place had been come to from inside `at`, which is asked on
/// the database's thread a moment before the journal has shut: where the
/// test ran in that moment it stopped the node while the journal still
/// said it had not shut, the node asked it to write what was measured,
/// the database was held an instant later, and the stop was never
/// answered. Seen once in a gate held to four processors, and ten
/// times in sixty-four with six or eight of these at once on four.
///
/// And whoever asks anything of a journal that has shut may never be
/// answered, so the answer to the moving on is waited for only until
/// then. This request's route has written all it writes before anything
/// it sets off is said, and is answered even where the journal shuts at
/// its own event (tried); it is held to the rule all the same.
async fn moved_on_until(rig: &Rig, id: &str, place: &str, at: fn(&kernel::Event) -> bool) {
    let journal = rig.node().app.journal.clone();
    journal.shut_after(at);
    tokio::select! {
        () = change(rig, id, json!({ "status": "building" })) => {}
        () = journal.shut() => {}
    }
    crate::soon(&format!("the daemon to come to where it is stopped, {place}"), journal.shut()).await;
}

#[tokio::test]
async fn a_plan_being_taken_up_when_the_daemon_stops_comes_to_the_same_cards_once_it_is_back() {
    // Where the daemon stops: as the first new card is added, as a card
    // that was there is being told what it waits on, as the cards are
    // being made ready, and when the feature has just said it was accepted.
    type Place = fn(&kernel::Event) -> bool;
    let places: [(&str, Place); 4] = [
        ("as a card is added", |event| {
            event.topic == "board.item.created" && event.data["title"] == "The party"
        }),
        ("as a card there already is changed", |event| {
            event.topic == "board.item.updated"
                && event.data["id"] == "shop-2"
                && event.data["through"] == "std/feature"
        }),
        ("as a card is made ready", |event| {
            event.topic == "board.item.moved" && event.data["id"] == "shop-1" && event.data["to"] == "ready"
        }),
        ("when it has said it was accepted", |event| event.topic == "feature.accepted"),
    ];
    for (place, at) in places {
        let (mut rig, script) = rig().await;
        // Nobody works: what is held is what the plan comes to.
        featured_with(&rig, json!({ "workers": 0, "reviewers": 0 }), json!({})).await;
        let (till, shelves) = (add(&rig, "The till", &[]).await, add(&rig, "The shelves", &[]).await);
        let tickets = json!([
            { "card": till }, { "card": shelves, "after": [1] },
            { "title": "The party", "after": [2] }, { "title": "The bunting", "after": [3] },
        ]);
        says(&script, ARCHITECT, &plan(tickets), "propose");
        let id = gather(&rig, "Open the shop", json!({ "gathers": [till, shelves] })).await;
        said(&rig, "feature.planned", &id).await;

        let from = rig.mark();
        moved_on_until(&rig, &id, place, at).await;
        rig.restart().await;

        // Back, it finishes what it was in the middle of, or takes the
        // plan up again where that could not be gone on with.
        rig.event("run.finished", move |event| {
            event.seq > from
                && event.data["blueprint"] == "std/feature"
                && event.data["flow"] == "on.taken"
                && event.data["status"] == "ok"
        })
        .await;
        said(&rig, "feature.accepted", &id).await;
        let board = rig.read("/api/board/shop").await;
        let mut titles: Vec<&str> =
            board["items"].as_array().unwrap().iter().map(|card| card["title"].as_str().unwrap()).collect();
        titles.sort_unstable();
        assert_eq!(
            titles,
            ["The bunting", "The party", "The shelves", "The till"],
            "{place}: each card once"
        );
        let by_title = |title: &str| {
            board["items"].as_array().unwrap().iter().find(|card| card["title"] == title).cloned().unwrap()
        };
        let (party, bunting) = (by_title("The party"), by_title("The bunting"));
        assert_eq!(by_title("The till")["more"]["after"], json!([]), "{place}");
        assert_eq!(by_title("The shelves")["more"]["after"], json!([till]), "{place}");
        assert_eq!(party["more"]["after"], json!([shelves]), "{place}");
        assert_eq!(bunting["more"]["after"], json!([party["id"]]), "{place}");
        for card in board["items"].as_array().unwrap() {
            assert_eq!(card["more"]["part_of"], id.as_str(), "{place}: {card}");
        }
        let cards = feature(&rig, &id).await["cards"].clone();
        assert_eq!(cards, json!([till, shelves, party["id"], bunting["id"]]), "{place}");
        // It says it was accepted once, wherever the daemon stopped.
        let accepted =
            lines(&rig, &id).await.iter().filter(|line| line.starts_with("it was accepted")).count();
        assert_eq!(accepted, 1, "{place}: {:?}", lines(&rig, &id).await);
        assert_eq!(feature(&rig, &id).await["status"], "building", "{place}");
    }
}
