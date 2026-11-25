//! A button on a card: something its column offers, a row of the fleet's
//! `actions`, pressed by whoever its row says may, through one door.
//!
//! A row that holds keeps a card in its column until it is pressed: the
//! stage there begins nothing for it, it says so once each time it comes,
//! and it is taken up when the last of what holds it is pressed. Who may
//! press is asked of whoever presses, by every way there is (the route,
//! the bot, a blueprint's step), never of whoever applied the fleet. With
//! no row, everything is as it always was.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add, add_row, card, rig, shop};
use crate::lanes::{history, move_card, moved};

/// Presses `action` on `id` with `token`: what was answered, and how.
async fn press(rig: &Rig, token: &str, id: &str, action: &str, back: bool) -> (StatusCode, Value) {
    let body = Some(json!({ "action": action, "back": back }));
    rig.call(Method::POST, &format!("/api/board/items/{id}/press"), Some(token), body).await
}

/// Each time it was said that `id` is held, with what it waits for.
async fn helds(rig: &Rig, id: &str) -> Vec<Value> {
    let query = journal::EventQuery { topics: "board.item.held".into(), limit: 100, ..Default::default() };
    let all = rig.said(query).await;
    all.into_iter().filter(|event| event.data["id"] == id).map(|event| event.data["waits"].clone()).collect()
}

/// The credential an agent's program reads each time it asks the factory anything.
fn token_of(rig: &Rig, agent: &str) -> String {
    std::fs::read_to_string(rig.dir.join(format!("data/slots/{agent}/token"))).unwrap().trim().to_string()
}

#[tokio::test]
async fn a_fleet_starts_with_no_actions_and_with_none_a_card_made_ready_is_taken_up_as_it_always_was() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    let rows = rig.read("/api/collections/shop/actions/records").await;
    assert_eq!(rows["records"], json!([]), "the table is there, and empty");
    let id = add(&rig, "Lay the floor", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "review", 0).await;
    assert!(helds(&rig, &id).await.is_empty(), "nothing held it");
    assert_eq!(card(&rig, &id).await["pressed"], json!([]));
}

#[tokio::test]
async fn a_card_its_column_holds_waits_there_saying_so_once_and_is_taken_up_when_the_last_of_it_is_pressed() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    // Signed is a person's, for good; Cleared anybody's, each time the card comes.
    add_row(&rig, "actions", json!({ "title": "Signed", "column": "ready", "who": "person", "holds": true }))
        .await;
    add_row(&rig, "actions", json!({ "title": "Cleared", "column": "ready", "holds": true, "again": true }))
        .await;
    let id = add(&rig, "Lay the floor", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    history(&rig, &id, "it waits in ready for Signed, Cleared to be pressed").await;
    let held = rig.event("board.item.held", |event| event.data["id"] == "shop-1").await;
    assert_eq!(
        held.data["says"],
        "shop-1 waits in ready for Signed, which a person presses; and Cleared, which anyone who may change \
         it presses. It is taken up when each is pressed."
    );
    // Told to whoever the project's `tells` says, as what waits for a person: by the fleet's telling, which hears
    // that the card is held and tells in a run of its own, after the card has said it waits.
    rig.event("inbox.told", |event| event.data["about"] == "shop-1").await;
    let inbox = rig.read("/api/inbox").await;
    let lines = inbox["lines"].as_array().unwrap();
    assert!(
        lines.iter().any(|line| line["about"] == "shop-1" && line["why"] == "held"),
        "the card waits for a person: {inbox}"
    );

    // An agent may not press what a person presses; a key that may not
    // change cards presses nothing; and nothing came of either.
    let agent = token_of(&rig, "shop-worker-1");
    let (status, answer) = press(&rig, &agent, &id, "signed", false).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{answer}");
    assert!(answer.to_string().contains("an agent may not press it"), "{answer}");
    let reads = crate::held::key(&rig, &["board:read"]).await;
    let (status, answer) = press(&rig, &reads, &id, "signed", false).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{answer}");
    assert!(answer.to_string().contains("board:write"), "{answer}");
    // What it is not offered is said, with what it is.
    let (status, answer) = press(&rig, &rig.token, &id, "approved", false).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{answer}");
    assert!(answer.to_string().contains("it is offered Signed, Cleared"), "{answer}");
    assert_eq!(card(&rig, &id).await["status"], "ready");

    // Said due again while it is held, nothing more is said of it.
    let (status, signed) = press(&rig, &rig.token, &id, "signed", false).await;
    assert_eq!(status, StatusCode::OK, "{signed}");
    assert_eq!(
        (&signed["done"], &signed["released"], &signed["waits"]),
        (&json!(true), &json!(false), &json!(["Cleared"]))
    );
    history(&rig, &id, "Ada pressed Signed").await;
    // Pressed twice, it is pressed once; taken back, and pressed again.
    let (_, twice) = press(&rig, &rig.token, &id, "Signed", false).await;
    assert_eq!(twice["done"], false, "{twice}");
    let (_, back) = press(&rig, &rig.token, &id, "signed", true).await;
    assert_eq!(back["done"], true, "{back}");
    history(&rig, &id, "Ada took back Signed").await;
    press(&rig, &rig.token, &id, "signed", false).await;
    assert_eq!(card(&rig, &id).await["status"], "ready", "Cleared still holds it");

    // The last of it pressed, it is taken up and worked.
    let now = rig.mark();
    let (_, cleared) = press(&rig, &rig.token, &id, "cleared", false).await;
    assert_eq!(cleared["released"], true, "{cleared}");
    moved(&rig, &id, "doing", now).await;
    moved(&rig, &id, "review", now).await;
    assert_eq!(helds(&rig, &id).await, [json!(["Signed", "Cleared"])], "said once for the time it came");
    // Taken back once it has gone on, nothing is.
    let (status, answer) = press(&rig, &rig.token, &id, "cleared", true).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");

    // Sent back to ready: what is for good stands, what is for one coming
    // is to be pressed again.
    let now = rig.mark();
    move_card(&rig, &id, "ready", Some("Not yet.")).await;
    let again =
        rig.event("board.item.held", move |event| event.seq > now && event.data["id"] == "shop-1").await;
    assert_eq!(again.data["waits"], json!(["Cleared"]));
    assert_eq!(card(&rig, &id).await["status"], "ready");
    let now = rig.mark();
    press(&rig, &rig.token, &id, "cleared", false).await;
    moved(&rig, &id, "review", now).await;
}

#[tokio::test]
async fn what_holds_nothing_is_pressed_and_kept_and_a_scope_its_row_names_is_asked_of_whoever_presses() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    let row = json!({ "title": "Looked at", "column": "review", "scope": "admin", "says": "looked it over" });
    add_row(&rig, "actions", row).await;
    let id = add(&rig, "Lay the floor", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "review", 0).await;
    // Offered in review, and holding nothing there, it holds nobody up.
    assert!(helds(&rig, &id).await.is_empty());
    let boards = crate::held::key(&rig, &["board", "records"]).await;
    let (status, answer) = press(&rig, &boards, &id, "looked-at", false).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{answer}");
    assert!(answer.to_string().contains("whoever holds `admin`"), "{answer}");
    let (status, pressed) = press(&rig, &rig.token, &id, "looked-at", false).await;
    assert_eq!((status, &pressed["released"]), (StatusCode::OK, &json!(false)), "{pressed}");
    history(&rig, &id, "Ada looked it over").await;
    let kept = card(&rig, &id).await["pressed"].clone();
    assert!(kept[0].as_str().unwrap().starts_with("pressed looked-at "), "{kept}");
    // What was said of it is the board's to say, and says who and which time.
    let said = rig.event("board.item.pressed", |event| event.data["action"] == "looked-at").await;
    assert_eq!((&said.data["status"], &said.data["back"]), (&json!("review"), &json!(false)));
    assert!(said.data["by"].as_str().unwrap().starts_with("user:"), "{}", said.data);
    assert_eq!(said.data["coming"], 3, "moved three times: to ready, to doing and to review");
}

#[tokio::test]
async fn the_bot_presses_as_whoever_asks_it_and_a_blueprints_step_as_whoever_applied_it() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    add_row(&rig, "actions", json!({ "title": "Signed", "column": "ready", "holds": true })).await;
    let first = add(&rig, "Lay the floor", &[]).await;
    move_card(&rig, &first, "ready", None).await;
    rig.event("board.item.held", |event| event.data["id"] == "shop-1").await;
    // In the project's channel, to the factory's bot.
    let now = rig.mark();
    let said = rig
        .send(
            "/api/channels/shop/messages",
            json!({ "body": format!("@conductor board press {first} signed") }),
        )
        .await;
    let asked = said["id"].as_str().unwrap().to_string();
    let answer = rig
        .event("channel.message", move |event| {
            event.data["reply_to"] == asked.as_str() && event.data["author"] == "bot:conductor"
        })
        .await;
    assert_eq!(answer.data["body"], format!("`{first}`: signed is pressed, and nothing holds it now."));
    moved(&rig, &first, "doing", now).await;

    // A blueprint's step, as whoever applied it: here a key that may not
    // change cards, which is refused, and then the owner.
    let second = add(&rig, "Put the shelves up", &[]).await;
    move_card(&rig, &second, "ready", None).await;
    let id = second.clone();
    rig.event("board.item.held", move |event| event.data["id"] == id.as_str()).await;
    let source = kernel::asked::BOARD_PRESS;
    let asked = json!({ "source": source, "project": "shop", "params": { "card": second }, "wait": true });
    let narrow = crate::held::key(&rig, &["blueprints", "board:read"]).await;
    let (status, refused) = rig.call(Method::POST, "/api/apply", Some(&narrow), Some(asked.clone())).await;
    assert!(status != StatusCode::OK || refused["finished"]["status"] != "ok", "{refused}");
    assert_eq!(card(&rig, &second).await["status"], "ready");
    let now = rig.mark();
    let applied = rig.send("/api/apply", asked).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    moved(&rig, &second, "doing", now).await;
    let pressed = rig.event("board.item.pressed", |event| event.data["id"] == "shop-2").await;
    assert_eq!(pressed.data["through"], "signed-when-it-passes", "{}", pressed.data);
}

#[tokio::test]
async fn a_held_card_is_held_still_when_the_daemon_is_back_and_is_said_to_be_once() {
    let (mut rig, _at) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    add_row(&rig, "actions", json!({ "title": "Signed", "column": "ready", "holds": true })).await;
    let id = add(&rig, "Lay the floor", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    rig.event("board.item.held", |event| event.data["id"] == "shop-1").await;
    rig.restart().await;
    // Its stage looks at its column when it is back, and finds it held.
    rig.send("/api/emit/stage.due", json!({ "card": id, "project": "shop", "stage": "ready", "note": "" }))
        .await;
    let now = rig.mark();
    press(&rig, &rig.token, &id, "signed", false).await;
    moved(&rig, &id, "review", now).await;
    assert_eq!(helds(&rig, &id).await.len(), 1, "said once, though looked at again");
}

/// What the working stage's look at a card came to, the first time it is
/// looked at after `after`: the word its row's `due` answered.
async fn looked(rig: &Rig, after: u64) -> String {
    let finished = rig
        .event("run.finished", move |event| {
            event.seq > after && event.data["blueprint"] == "std/working" && event.data["flow"] == "on.begun"
        })
        .await;
    let run = rig.read(&format!("/api/runs/{}", finished.source.run.as_deref().unwrap())).await;
    let due = run["steps"].as_array().unwrap().iter().find(|step| step["step"] == "due").unwrap();
    due["output"]["came"].as_str().unwrap_or_default().to_string()
}

#[tokio::test]
async fn a_card_deleted_and_given_back_has_what_was_pressed_on_it_and_is_held_by_the_rest_as_before() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    add_row(&rig, "actions", json!({ "title": "Signed", "column": "ready", "who": "person", "holds": true }))
        .await;
    add_row(&rig, "actions", json!({ "title": "Cleared", "column": "ready", "holds": true, "again": true }))
        .await;
    let id = add(&rig, "Lay the floor", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    rig.event("board.item.held", |event| event.data["id"] == "shop-1").await;
    press(&rig, &rig.token, &id, "signed", false).await;
    let kept = card(&rig, &id).await["pressed"].clone();

    // In the Trash it is on no board, and nothing is pressed on it there.
    let (_, deleted) =
        rig.call(Method::DELETE, &format!("/api/board/items/{id}"), Some(&rig.token), None).await;
    let trashed = deleted["trash"].as_str().unwrap().to_string();
    let (status, answer) = press(&rig, &rig.token, &id, "cleared", false).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{answer}");

    // Given back, its row is as it was: what was pressed for good, and
    // what was pressed for the time it came, which this still is, since
    // being given back is no move. So its stage, which hears it as added,
    // finds it held by what is left, and says nothing more of that.
    let now = rig.mark();
    rig.send(&format!("/api/trash/{trashed}/restore"), json!({})).await;
    assert_eq!(looked(&rig, now).await, "held");
    let back = card(&rig, &id).await;
    assert_eq!((&back["status"], &back["pressed"]), (&json!("ready"), &kept), "{back}");
    assert_eq!(
        helds(&rig, &id).await.len(),
        1,
        "said once for the time it came: {:?}",
        helds(&rig, &id).await
    );

    // The last of it pressed, it is taken up as any card is.
    let now = rig.mark();
    let (_, cleared) = press(&rig, &rig.token, &id, "cleared", false).await;
    assert_eq!(cleared["released"], true, "{cleared}");
    moved(&rig, &id, "review", now).await;
}
