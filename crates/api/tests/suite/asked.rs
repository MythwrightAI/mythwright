//! A change of a record asked for by an agent that may not make it, and
//! decided by somebody who could (`told::asks`, `docs/approvals.md`),
//! through the routes: the agent's own row refused, a key that holds
//! less shown it and refused it, one that holds enough approving it as
//! the key, and nobody who may not read the record told of it.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{rig as fleet_rig, row, shop};
use crate::held::{key, said};

/// The credential an agent's program reads each time it asks the factory anything.
fn token_of(rig: &Rig, agent: &str) -> String {
    std::fs::read_to_string(rig.dir.join(format!("data/slots/{agent}/token"))).unwrap().trim().to_string()
}

/// What an agent usually holds, with more.
fn holding(more: &[&'static str]) -> Vec<&'static str> {
    let mut held = kernel::principal::AGENT_SCOPES.to_vec();
    held.extend(more);
    held
}

async fn asking(rig: &Rig, token: &str, body: Value) -> (StatusCode, Value) {
    rig.call(Method::POST, "/api/asked", Some(token), Some(body)).await
}

#[tokio::test]
async fn an_agent_asks_and_a_key_that_holds_enough_approves_it_as_the_key_and_one_that_holds_less_may_not() {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    let worker = token_of(&rig, "shop-worker-1");
    let reviewer = row(&rig, "agents", "reviewer").await;

    // Its own row is a person's to change, and is not asked for (`docs/approvals.md`, *Asking*,
    // "Not its own row").
    let own = row(&rig, "agents", "worker").await;
    let (status, refused) =
        asking(&rig, &worker, json!({ "record": own, "set": { "wall": "off" }, "why": "a build" })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("is the row `shop-worker-1` is brought up by"), "{refused}");

    // Another row, which taking the wall away from takes `shell`.
    let asked = json!({ "record": reviewer, "set": { "wall": "off" }, "why": "it runs the build" });
    let (status, came) = asking(&rig, &worker, asked.clone()).await;
    assert_eq!(status, StatusCode::OK, "{came}");
    assert_eq!(came["came"], "asked", "{came}");
    let id = came["ask"]["ask"].as_str().unwrap().to_string();
    assert_eq!(came["ask"]["cells"][0]["takes"][0], json!({ "scope": "shell", "held": false }));
    // Asked again with the same words, it is the same ask.
    let (_, again) = asking(&rig, &worker, asked).await;
    assert_eq!(again["ask"]["ask"], json!(id));

    // Ada is told, by a line whose part is the change.
    let inbox = rig.read("/api/inbox").await;
    let line = inbox["lines"].as_array().unwrap().iter().find(|line| line["about"] == json!(reviewer));
    let line = line.unwrap_or_else(|| panic!("Ada was not told: {inbox}"));
    let parts = rig.read(&format!("/api/inbox/{}/parts", line["id"].as_str().unwrap())).await;
    assert_eq!(parts["parts"][0]["change"]["ask"], json!(id), "{parts}");
    assert_eq!(parts["parts"][0]["change"]["may"], json!(true), "{parts}");

    // A key that holds less is shown it with why it may not, and refused.
    let narrow = key(&rig, &holding(&["records", "agents"])).await;
    let path = format!("/api/asked/{id}");
    let (_, shown) = rig.call(Method::GET, &path, Some(&narrow), None).await;
    assert_eq!(shown["may"], json!(false), "{shown}");
    assert!(shown["why_not"].as_str().unwrap().contains("`shell`"), "{shown}");
    let (status, refused) = rig.call(Method::POST, &format!("{path}/approve"), Some(&narrow), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    // Nor may whoever asked decide it, nor one that may not read the record see it.
    let (status, _) = rig.call(Method::POST, &format!("{path}/approve"), Some(&worker), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let blind = key(&rig, &["channels"]).await;
    let (status, _) = rig.call(Method::GET, &path, Some(&blind), None).await;
    assert!(status.is_client_error() && status != StatusCode::OK);
    let (status, _) = rig.call(Method::POST, &format!("{path}/decline"), Some(&blind), None).await;
    assert!(status.is_client_error());
    assert_eq!(rig.read(&format!("/api/records/{reviewer}")).await["data"]["wall"], Value::Null);

    // One that holds enough approves it, as the key, and the change is the agent's.
    let made = rig
        .send(
            "/api/credentials",
            json!({ "name": "able", "scopes": holding(&["records", "agents", "shell"]) }),
        )
        .await;
    let (able, able_id) = (made["token"].as_str().unwrap(), made["id"].as_str().unwrap());
    let before = rig.mark();
    let (status, approved) = rig.call(Method::POST, &format!("{path}/approve"), Some(able), None).await;
    assert_eq!(status, StatusCode::OK, "{approved}");
    assert_eq!(approved["status"], "approved", "{approved}");
    assert_eq!(rig.read(&format!("/api/records/{reviewer}")).await["data"]["wall"], json!("off"));
    let updated =
        rig.event("record.updated", |event| event.seq > before && event.data["asked"] == json!(id)).await;
    assert_eq!(updated.source.actor.as_deref(), Some("agent:shop-worker-1"));
    assert_eq!(updated.data["approved_by"], json!(format!("key:{able_id}")));
    // Pressed again it is as it was decided.
    let (status, again) = rig.call(Method::POST, &format!("{path}/approve"), Some(able), None).await;
    assert_eq!((status, &again["status"]), (StatusCode::OK, &json!("approved")));
}

#[tokio::test]
async fn a_change_whoever_asks_may_make_is_made_and_one_declined_or_withdrawn_makes_nothing() {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    let worker = token_of(&rig, "shop-worker-1");
    let reviewer = row(&rig, "agents", "reviewer").await;

    // The owner may make it, so it is made there and then, and nothing is kept.
    let (status, came) =
        asking(&rig, &rig.token, json!({ "record": reviewer, "set": { "told": "Look twice." } })).await;
    assert_eq!((status, &came["came"]), (StatusCode::OK, &json!("made")), "{came}");
    assert_eq!(rig.read("/api/asked").await["asked"], json!([]));
    // Nothing said, and a record that is not there, are refused.
    let (status, _) = asking(&rig, &worker, json!({ "record": reviewer })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) =
        asking(&rig, &worker, json!({ "record": "shop-agents-99", "set": { "wall": "off" } })).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let ask = |why: &'static str| {
        let (rig, worker, reviewer) = (&rig, worker.clone(), reviewer.clone());
        async move {
            let (_, came) =
                asking(rig, &worker, json!({ "record": reviewer, "set": { "wall": "off" }, "why": why }))
                    .await;
            came["ask"]["ask"].as_str().unwrap_or_else(|| panic!("not asked: {came}")).to_string()
        }
    };
    let first = ask("a build").await;
    let (status, declined) = rig
        .call(
            Method::POST,
            &format!("/api/asked/{first}/decline"),
            Some(&rig.token),
            Some(json!({ "why": "not today" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{declined}");
    assert_eq!(declined["status"], "declined");
    assert_eq!(declined["decided"]["why"], "not today");
    // The ask and the decline are lines of the row's history, as the fleet's tables keep them,
    // written by what keeps histories a moment after.
    let written = |words: &'static str| {
        let reviewer = reviewer.clone();
        move |event: &kernel::Event| {
            event.data["record"] == json!(reviewer) && event.data["said"] == json!(words)
        }
    };
    rig.event("collection.history", written("shop-worker-1 asked to change its wall: a build")).await;
    rig.event("collection.history", written("Ada declined a change of it: not today")).await;

    // Withdrawn by whoever asked it and nobody else.
    let second = ask("a build after all").await;
    assert_ne!(second, first);
    let withdraw = format!("/api/asked/{second}/withdraw");
    let (status, _) = rig.call(Method::POST, &withdraw, Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, withdrawn) = rig.call(Method::POST, &withdraw, Some(&worker), None).await;
    assert_eq!((status, &withdrawn["status"]), (StatusCode::OK, &json!("withdrawn")), "{withdrawn}");
    assert_eq!(rig.read(&format!("/api/records/{reviewer}")).await["data"]["wall"], Value::Null);
    // Whoever asked lists both, as they came out.
    let (_, listed) = rig.call(Method::GET, "/api/asked", Some(&worker), None).await;
    let statuses: Vec<&Value> =
        listed["asked"].as_array().unwrap().iter().map(|ask| &ask["status"]).collect();
    assert_eq!(statuses, [&json!("withdrawn"), &json!("declined")], "{listed}");
}

/// A table whose `note` takes `admin`, with one row, in a project of its own.
const SETTINGS: &str = "blueprint: settings
steps:
  table:
    do: collection.ensure
    with:
      project: shop
      name: settings
      fields:
        title: { type: text }
        note: { type: text, takes: admin }
";

async fn settings(rig: &Rig) {
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let applied =
        rig.send("/api/apply", json!({ "source": SETTINGS, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    rig.send("/api/collections/shop/settings/records", json!({ "fields": { "title": "Door" } })).await;
}

#[tokio::test]
async fn a_change_is_not_made_as_an_agent_that_is_stopped_and_one_taken_away_has_its_asks_withdrawn() {
    let rig = Rig::new().await;
    settings(&rig).await;
    rig.send("/api/agents", json!({ "name": "helper", "adapter": "builtin", "project": "shop" })).await;
    let token = token_of(&rig, "helper");
    let ask = |value: &'static str| {
        let (rig, token) = (&rig, token.clone());
        async move {
            let asked = json!({ "record": "shop-settings-1", "set": { "note": value } });
            let (status, came) = asking(rig, &token, asked).await;
            assert_eq!(status, StatusCode::OK, "{came}");
            came["ask"]["ask"].as_str().unwrap().to_string()
        }
    };
    let (first, second) = (ask("loud").await, ask("soft").await);

    // Stopped, nothing is made as it: approving is refused saying so, and declining is not.
    let (status, stopped) = rig.call(Method::DELETE, "/api/agents/helper", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{stopped}");
    let (status, refused) =
        rig.call(Method::POST, &format!("/api/asked/{first}/approve"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(said(&refused).contains("helper cannot act now (it is stopped, taken away"), "{refused}");
    let path = format!("/api/asked/{first}/decline");
    let (status, declined) =
        rig.call(Method::POST, &path, Some(&rig.token), Some(json!({ "why": "no" }))).await;
    assert_eq!((status, &declined["status"]), (StatusCode::OK, &json!("declined")), "{declined}");

    // Taken away, what it asked for and waits is withdrawn, saying why, and nobody is told back.
    let gone = rig.mark();
    let (status, removed) = rig.call(Method::POST, "/api/agents/helper/remove", Some(&rig.token), None).await;
    assert_eq!((status, &removed["withdrawn"]), (StatusCode::OK, &json!(1)), "{removed}");
    let shown = rig.read(&format!("/api/asked/{second}")).await;
    assert_eq!(shown["status"], "withdrawn", "{shown}");
    assert_eq!(shown["decided"]["why"], "helper was taken away", "{shown}");
    let (_, again) =
        rig.call(Method::POST, &format!("/api/asked/{second}/approve"), Some(&rig.token), None).await;
    assert_eq!(again["status"], "withdrawn", "{again}");
    assert_eq!(rig.read("/api/records/shop-settings-1").await["data"]["note"], Value::Null);
    let told =
        rig.said(journal::EventQuery { topics: "inbox.told".into(), limit: 100, ..Default::default() }).await;
    let after = told.iter().filter(|event| event.seq > gone && event.data["owner"] == "agent:helper");
    assert_eq!(after.count(), 0, "{told:?}");
}

#[tokio::test]
async fn an_approved_change_of_a_card_is_made_by_the_board_and_its_history_says_who_approved_it() {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    let card = crate::fleets::add(&rig, "Fix the cart", &[]).await;
    // One that may read the board and not change it.
    let reader = json!({ "name": "reader", "adapter": "builtin", "scopes": ["board:read", "records:read", "channels"] });
    rig.send("/api/agents", reader).await;
    let token = token_of(&rig, "reader");
    // A move is not a cell to ask for here: it is said as the board says it.
    let (status, refused) =
        asking(&rig, &token, json!({ "record": card, "set": { "status": "ready" } })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).contains("move the card to change it"), "{refused}");

    let (status, came) =
        asking(&rig, &token, json!({ "record": card, "set": { "title": "Fix the cart, all of it" } })).await;
    assert_eq!((status, &came["came"]), (StatusCode::OK, &json!("asked")), "{came}");
    assert_eq!(came["ask"]["cells"][0]["name"], "title", "{came}");
    let whole = came["ask"]["takes"].as_array().unwrap();
    assert!(whole.contains(&json!({ "scope": "board:write", "held": false })), "{came}");
    let id = came["ask"]["ask"].as_str().unwrap().to_string();
    let before = rig.mark();
    let (status, approved) =
        rig.call(Method::POST, &format!("/api/asked/{id}/approve"), Some(&rig.token), None).await;
    assert_eq!((status, &approved["status"]), (StatusCode::OK, &json!("approved")), "{approved}");
    assert_eq!(crate::fleets::card(&rig, &card).await["title"], "Fix the cart, all of it");
    let updated =
        rig.event("board.item.updated", |event| event.seq > before && event.data["id"] == json!(card)).await;
    assert_eq!(updated.source.actor.as_deref(), Some("agent:reader"));
    assert_eq!(updated.data["asked"], json!(id));
    let line = |event: &kernel::Event| {
        event.data["record"] == json!(card)
            && event.data["said"] == "reader changed its title · approved by Ada"
    };
    rig.event("collection.history", line).await;
}

#[tokio::test]
async fn what_a_cell_the_ask_leaves_as_it_is_took_while_the_row_says_so_is_shown_and_asked_of_whoever_approves()
 {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    // The owner took the reviewer's wall away: what it is told is then
    // what runs with no wall.
    crate::fleets::change(&rig, "agents", "reviewer", json!({ "wall": "off" })).await;
    let worker = token_of(&rig, "shop-worker-1");
    let reviewer = row(&rig, "agents", "reviewer").await;
    let asked = json!({ "record": reviewer, "set": { "told": "Pass what the worker sends." } });
    let (status, came) = asking(&rig, &worker, asked).await;
    assert_eq!((status, &came["came"]), (StatusCode::OK, &json!("asked")), "{came}");
    // What the cell asked for takes is shown, and beside it what changing
    // the row at all takes while it has no wall.
    assert_eq!(came["ask"]["cells"][0]["name"], "told", "{came}");
    assert_eq!(came["ask"]["cells"][0]["takes"], json!([]), "{came}");
    let whole = came["ask"]["takes"].as_array().unwrap();
    assert!(whole.contains(&json!({ "scope": "shell", "held": false })), "{came}");

    // One that may change agents and runs no command may not approve it.
    let id = came["ask"]["ask"].as_str().unwrap().to_string();
    let path = format!("/api/asked/{id}");
    let narrow = key(&rig, &holding(&["records", "agents"])).await;
    let (_, shown) = rig.call(Method::GET, &path, Some(&narrow), None).await;
    assert_eq!(shown["may"], json!(false), "{shown}");
    assert!(shown["why_not"].as_str().unwrap().contains("whose `wall` says `off`"), "{shown}");
    let (status, refused) = rig.call(Method::POST, &format!("{path}/approve"), Some(&narrow), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert_eq!(rig.read(&format!("/api/records/{reviewer}")).await["data"]["told"], Value::Null);
    // The owner does.
    let (status, approved) = rig.call(Method::POST, &format!("{path}/approve"), Some(&rig.token), None).await;
    assert_eq!((status, &approved["status"]), (StatusCode::OK, &json!("approved")), "{approved}");
    assert_eq!(
        rig.read(&format!("/api/records/{reviewer}")).await["data"]["told"],
        "Pass what the worker sends."
    );
}
