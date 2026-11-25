//! Cards that wait on other cards, with nothing remembered but what their
//! rows say (`std/cards`): a card that was asked whether it may begin says
//! what it waits for, which is also what says it was asked, and is looked
//! at again when a card it names is finished or taken away, when what it
//! waits on is said again, and when its board starts.

use kernel::Source;
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add, card};
use crate::lanes::move_card;

/// A `shop` project with a board, and so with what looks after its cards.
async fn board(rig: &Rig) {
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let board = json!({ "blueprint": "std/board", "project": "shop", "wait": true });
    assert_eq!(rig.send("/api/apply", board).await["finished"]["status"], "ok");
}

/// Says `topic` of a card, as whatever would begin it does, and which
/// event that was.
async fn say(rig: &Rig, topic: &str, id: &str) -> u64 {
    let data = json!({ "card": id, "project": "shop" });
    rig.node().app.journal.emit(topic, Source::default(), data).await.unwrap().seq
}

/// Waits until the run that the event numbered `seq` set off has ended:
/// what is heard of one card is done one thing at a time, so everything
/// heard of that card before it is over too.
async fn over(rig: &Rig, seq: u64) {
    let mut said = rig.follow("run.finished");
    crate::soon(&format!("the run event {seq} set off to end"), async {
        loop {
            let runs = rig.node().app.engine.recent_runs(500, None).await.unwrap();
            if runs.iter().any(|run| run.event["seq"] == seq && run.finished.is_some()) {
                return;
            }
            said.next().await.unwrap();
        }
    })
    .await;
}

/// Each event of `topic` there has been about `id`.
async fn said_of(rig: &Rig, topic: &str, id: &str) -> Vec<Value> {
    let events = rig.read(&format!("/api/events/recent?topics={topic}&limit=500")).await;
    let about = |event: &&Value| event["data"]["card"] == id;
    events["events"].as_array().unwrap().iter().filter(about).cloned().collect()
}

#[tokio::test]
async fn two_cards_finished_in_one_moment_free_the_card_that_names_both_once_and_one_nobody_asked_is_left_as_it_is()
 {
    let rig = Rig::new().await;
    board(&rig).await;
    let (floor, walls) = (add(&rig, "Lay the floor", &[]).await, add(&rig, "Paint the walls", &[]).await);
    let shelves = add(&rig, "Put the shelves up", &[floor.as_str(), walls.as_str()]).await;
    // Names the floor too, and nobody has asked whether it may begin.
    let rug = add(&rig, "Roll the rug out", &[floor.as_str()]).await;

    // Asked, the shelves have to wait, and say what for.
    say(&rig, "after.asked", &shelves).await;
    let waits = format!("after {floor}, {walls}, not done yet");
    let told = rig.event("card.waiting", |_| true).await;
    assert_eq!(told.data, json!({ "card": shelves, "project": "shop", "what": waits }));
    assert_eq!(card(&rig, &shelves).await["more"]["waiting"], waits.as_str());

    // Both finished at once.
    tokio::join!(move_card(&rig, &floor, "done", None), move_card(&rig, &walls, "done", None));
    rig.event("after.clear", |_| true).await;
    // And everything heard of the shelves is over: one more look, which
    // finds nothing to say, ends after whatever came before it.
    over(&rig, say(&rig, "after.again", &shelves).await).await;

    // Told once that nothing is left, said to be free once, and it says nothing now.
    assert_eq!(said_of(&rig, "after.clear", &shelves).await.len(), 1);
    assert_eq!(said_of(&rig, "card.freed", &shelves).await.len(), 1);
    assert_eq!(card(&rig, &shelves).await["more"]["waiting"], "");
    // The rug names the floor and was never asked: nothing was written on
    // it, and nobody was told anything of it, by the floor being finished
    // or by being looked at.
    over(&rig, say(&rig, "after.again", &rug).await).await;
    assert!(card(&rig, &rug).await["more"]["waiting"].as_str().unwrap_or_default().is_empty());
    for topic in ["after.clear", "card.waiting", "card.freed"] {
        assert!(said_of(&rig, topic, &rug).await.is_empty(), "{topic} of the rug");
    }
    // Asked now, with nothing left, it is told at once.
    say(&rig, "after.asked", &rug).await;
    let rug_too = rug.clone();
    rig.event("after.clear", move |event| event.data["card"] == rug_too.as_str()).await;
}

#[tokio::test]
async fn a_card_waiting_on_one_that_is_taken_away_or_whose_plan_is_changed_or_whose_board_starts_again_is_looked_at_again()
 {
    let rig = Rig::new().await;
    board(&rig).await;
    let (floor, walls) = (add(&rig, "Lay the floor", &[]).await, add(&rig, "Paint the walls", &[]).await);
    let shelves = add(&rig, "Put the shelves up", &[floor.as_str(), walls.as_str()]).await;
    say(&rig, "after.asked", &shelves).await;
    rig.event("card.waiting", |_| true).await;

    // One of the two is taken off the board: it waits on the other alone, and says so.
    let (status, _) =
        rig.call(reqwest::Method::DELETE, &format!("/api/board/items/{floor}"), Some(&rig.token), None).await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let fewer = format!("after {walls}, not done yet");
    let (id, what) = (shelves.clone(), fewer.clone());
    rig.event("card.waiting", move |event| {
        event.data["card"] == id.as_str() && event.data["what"] == what.as_str()
    })
    .await;
    assert_eq!(card(&rig, &shelves).await["more"]["waiting"], fewer.as_str());

    // The board is stopped and applied again while it waits: what looks
    // after its cards is another instance, which starts with nothing but
    // the rows. It finds the card that says it waits and looks at it
    // again, and nothing has changed.
    let before = rig.mark();
    let all = rig.read("/api/instances").await;
    let was =
        all["instances"].as_array().unwrap().iter().find(|one| one["blueprint"] == "std/board").unwrap();
    rig.send(&format!("/api/instances/{}/stop", was["id"].as_str().unwrap()), json!({})).await;
    let again = json!({ "blueprint": "std/board", "project": "shop", "wait": true });
    assert_eq!(rig.send("/api/apply", again).await["finished"]["status"], "ok");
    let id = shelves.clone();
    let looked =
        rig.event("after.again", move |event| event.seq > before && event.data["card"] == id.as_str()).await;
    over(&rig, rig.number(looked.seq)).await;
    assert_eq!(card(&rig, &shelves).await["more"]["waiting"], fewer.as_str());
    assert!(said_of(&rig, "after.clear", &shelves).await.is_empty());

    // What it waits on is said again, as nothing: it is free, and whoever asked is told.
    let fields = json!({ "fields": { "after": [] } });
    let (status, answer) = rig
        .call(reqwest::Method::PATCH, &format!("/api/board/items/{shelves}"), Some(&rig.token), Some(fields))
        .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{answer}");
    let id = shelves.clone();
    rig.event("after.clear", move |event| event.data["card"] == id.as_str()).await;
    assert_eq!(said_of(&rig, "card.freed", &shelves).await.len(), 1);
}

#[tokio::test]
async fn another_projects_pool_saying_that_something_waits_begins_nothing_for_this_board() {
    let rig = Rig::new().await;
    board(&rig).await;
    let cart = add(&rig, "Add a cart", &[]).await;
    let waiting = |project: &str, about: &str| json!({ "pool": format!("{project}/ready"), "project": project, "to": "key:x", "about": about, "limit": 1, "why": "" });
    let journal = &rig.node().app.journal;
    // Another project's pool, about something called as a card of this one
    // is; and a pool that says no project, as one from before does.
    let other = journal.emit("pool.waiting", Source::default(), waiting("depot", &cart)).await.unwrap().seq;
    let mut unsaid = waiting("shop", &cart);
    unsaid.as_object_mut().unwrap().remove("project");
    let none = journal.emit("pool.waiting", Source::default(), unsaid).await.unwrap().seq;
    // This project's own pool: the card says why it waits.
    let own = journal.emit("pool.waiting", Source::default(), waiting("shop", &cart)).await.unwrap().seq;
    over(&rig, own).await;
    assert_eq!(card(&rig, &cart).await["more"]["waiting"], "one card is worked at a time here");

    // Told by what set each run off: the other two set none off, here or anywhere.
    let runs = rig.node().app.engine.recent_runs(500, None).await.unwrap();
    let set_off: Vec<String> = runs
        .iter()
        .filter(|run| run.event["seq"] == other || run.event["seq"] == none)
        .map(|run| format!("{} of {}, by {}", run.flow, run.instance, run.event))
        .collect();
    assert!(set_off.is_empty(), "{set_off:#?}");
    assert_eq!(said_of(&rig, "card.waiting", &cart).await.len(), 1);

    // This project's pool, about something that is no card of its board:
    // a run is begun, since it may be one, and nothing comes of it.
    let stranger =
        journal.emit("pool.waiting", Source::default(), waiting("shop", "order-7")).await.unwrap().seq;
    over(&rig, stranger).await;
    assert!(said_of(&rig, "card.waiting", "order-7").await.is_empty());
}
