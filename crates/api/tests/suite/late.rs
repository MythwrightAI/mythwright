//! A card whose row was written with what is said of it never said, and
//! what becomes of it when the daemon is back.
//!
//! A program before this one wrote a move down and then said it, and a
//! daemon that went between the two left a row nobody had been told of;
//! a board kept as files or by a store still can, and so can a project
//! whose records are kept in the factory's journal while what is said of
//! it is kept in its own. The board holds its rows against what was said
//! when the daemon has started and says what was not, late. The tests
//! here plant the row as it would be found, and one holds the other
//! side: where a card's row and what is said of it are one journal's,
//! no stop and no failure to say leaves one.

use serde_json::json;

use crate::Rig;
use crate::fleets::{add, card, change, given, shop};
use crate::lanes::{move_card, moved};
use crate::restarted::{in_hand, rig};

/// Puts a card in a column and says nothing: what a journal from before
/// may hold. A program before this one wrote a move down and then said
/// it, and a daemon that went between the two left the row with nobody
/// told. A move is one write with its saying now, so no stop of this
/// program leaves that: the row is written here as it would be found.
pub(crate) async fn moved_and_not_said(rig: &Rig, id: &str, to: &str) {
    let collections = &rig.node().app.studio.collections;
    let _turn = collections.changing(id).await;
    let (board, mut record) = collections.record(id).await.unwrap();
    record.data.insert("status".into(), json!(to));
    collections.put(&board, record).await.unwrap();
}

/// The same, with what the move itself wrote on the row as it was made:
/// where the card came from, that a person moved it, what they said as
/// they did, and how many times it has been moved.
pub(crate) async fn moved_saying_and_not_said(rig: &Rig, id: &str, to: &str, said: &str) {
    let collections = &rig.node().app.studio.collections;
    let _turn = collections.changing(id).await;
    let (board, mut record) = collections.record(id).await.unwrap();
    let from = record.data["status"].as_str().unwrap_or_default().to_string();
    let kept: Vec<String> =
        serde_json::from_value(record.data.get("moved").cloned().unwrap_or_default()).unwrap_or_default();
    let count = studio::models::Moved::read(&kept).map_or(0, |moved| moved.count) + 1;
    let moved = studio::models::Moved { from, by: "user".into(), said: said.into(), count };
    record.data.insert("status".into(), json!(to));
    record.data.insert("moved".into(), json!(moved.kept()));
    collections.put(&board, record).await.unwrap();
}

/// Adds a card to a column and says nothing: what a journal from before
/// may hold, where a daemon went between writing a card down and saying
/// it. The board is the one `beside` is a card of.
pub(crate) async fn added_and_not_said(rig: &Rig, beside: &str, title: &str, to: &str) -> String {
    let collections = &rig.node().app.studio.collections;
    let (board, _) = collections.record(beside).await.unwrap();
    let data = kernel::fields(json!({ "title": title, "status": to }));
    collections.add(&board, data, 1000.0, &kernel::Principal::system()).await.unwrap().id
}

#[tokio::test]
async fn a_card_sent_back_as_the_daemon_went_with_nobody_told_is_given_a_round_when_it_is_back() {
    let (mut rig, mut held) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    let (id, _) = in_hand(&rig, &mut held).await;
    held.work();
    let reviewed = moved(&rig, &id, "review", 0).await;
    // Nothing is waited for between the card's coming to review and its
    // being sent back: the round that put it there may not be over. A
    // round begun again when the daemon is back finds a card that has
    // moved since it was told of it, and hands nothing on a second time
    // (`work.held`); this once waited the round out, since a round begun
    // again handed the card on as it had been doing and undid the move.
    // A person sends it back, and the daemon goes between writing that
    // down and saying it: the row says doing, and no blueprint heard.
    moved_and_not_said(&rig, &id, "doing").await;

    rig.restart().await;

    // Nobody says anything more. The stage that worked it looks at the
    // rows as the daemon starts, and the card goes round once more: what
    // was said as it was sent back went with the event, and is not known.
    // Waited for as something the daemon says, so that a wait that runs
    // out says what it said instead.
    let thread = format!("lane:{id}");
    rig.event("agent.turn.started", move |event| {
        event.seq > reviewed && event.data["thread"] == thread.as_str()
    })
    .await;
    held.until_asked("work", 2).await;
    moved(&rig, &id, "review", reviewed).await;
    let asked = given(&rig, "shop-worker-1").await;
    assert_eq!(asked.len(), 2, "{asked:?}");
    assert!(asked[1].contains("sent back with nothing said"), "{}", asked[1]);
}

/// Waits until everything of the fleet has come up: nothing that is alive
/// in the project is still starting. A fleet's apply having answered is
/// not that: its stages may still be at their steps, and one stopped then
/// is taken up in the middle of them and never hears that the daemon
/// started, having not been listening when that was said.
async fn come_up(rig: &Rig) {
    let mut active = rig.follow("instance.active");
    crate::soon("everything of the fleet to have come up", async {
        loop {
            let live = rig.read("/api/instances?live=true&project=shop").await;
            let live = live["instances"].as_array().cloned().unwrap_or_default();
            if !live.is_empty() && live.iter().all(|one| one["status"] != "starting") {
                return;
            }
            active.next().await.unwrap();
        }
    })
    .await;
}

/// Stops the daemon and starts it again, and waits until it has looked for
/// what was written and never said: where the journal had got to before.
/// The stage that works looks when it hears that the daemon started,
/// which it hears where it was up before the daemon stopped.
async fn looked_again(rig: &mut Rig) -> u64 {
    come_up(rig).await;
    let before = rig.mark();
    rig.restart().await;
    rig.event("run.finished", move |event| event.seq > before && event.data["blueprint"] == "std/owed").await;
    before
}

#[tokio::test]
async fn a_card_in_doing_that_no_round_has_had_is_left_alone_when_the_daemon_is_started_again() {
    let (mut rig, held) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    // A person took a card in hand themselves, and that was said: it was
    // never a round's.
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "doing", None).await;

    let started = looked_again(&mut rig).await;

    // The daemon has looked at what was written and never said, found
    // nothing of this card, and nobody is asked. Its being finished is
    // waited for, as what is done for the card after anything the look
    // would have begun.
    move_card(&rig, &id, "done", None).await;
    rig.event("run.finished", move |event| event.seq > started && event.data["flow"] == "on.done").await;
    assert_eq!(card(&rig, &id).await["status"], "done");
    assert_eq!((held.asked("work"), given(&rig, "shop-worker-1").await.len()), (0, 0));
}

#[tokio::test]
async fn a_card_made_ready_as_the_daemon_went_with_nobody_told_is_worked_when_it_is_back_and_told_once() {
    let (mut rig, mut held) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    let id = add(&rig, "Add a cart", &[]).await;
    // A person makes it ready, and the daemon goes between writing that
    // down and saying it: the row says ready, and no stage heard.
    moved_and_not_said(&rig, &id, "ready").await;

    rig.restart().await;

    // Nobody says anything more. The move is said when the daemon is
    // back, late, and the stage of that column takes the card up.
    let its = id.clone();
    let late = rig
        .event("board.item.moved", move |event| {
            event.data["id"] == its.as_str() && event.data["to"] == "ready"
        })
        .await;
    assert_eq!((late.data["from"].clone(), late.data["late"].clone()), (json!("draft"), json!(true)));
    held.until_asked("work", 1).await;

    // Started again with the card in hand, it is not said a second time.
    rig.restart().await;
    held.work();
    moved(&rig, &id, "review", late.seq).await;
    let said = rig.said(journal::EventQuery {
        topics: "board.item.moved".into(),
        limit: 1000,
        ..Default::default()
    });
    let to_ready = said.await.iter().filter(|event| event.data["to"] == "ready").count();
    assert_eq!(to_ready, 1);
    assert_eq!(given(&rig, "shop-worker-1").await.len(), 1);
}

#[tokio::test]
async fn a_card_put_in_review_as_the_daemon_went_with_nobody_told_is_reviewed_when_it_is_back() {
    let (mut rig, mut held) = rig().await;
    shop(&rig, json!({})).await;
    let id = add(&rig, "Add a cart", &[]).await;
    moved_and_not_said(&rig, &id, "review").await;

    rig.restart().await;

    // The stage that reviews is told when the daemon is back, and whoever
    // reviews is asked: the card does not wait there for ever.
    held.until_asked("look", 1).await;
    held.look();
    moved(&rig, &id, "done", 0).await;
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), 1);
}

#[tokio::test]
async fn a_card_sent_back_is_counted_once_whatever_the_daemon_finds_of_it_when_it_starts() {
    let (mut rig, mut held) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    let (id, _) = in_hand(&rig, &mut held).await;
    held.work();
    let reviewed = moved(&rig, &id, "review", 0).await;
    rig.event("run.finished", move |event| event.seq > reviewed && event.data["flow"] == "on.go").await;
    // Nobody works here for now; and a person sends the card back, which
    // is said and counted. What the stage would do with it waits for
    // there to be somebody.
    change(&rig, "agents", "worker", json!({ "count": 0 })).await;
    move_card(&rig, &id, "doing", Some("The total is wrong.")).await;
    let its = id.clone();
    rig.event("board.item.updated", move |event| {
        event.data["id"] == its.as_str() && event.data["changed"] == json!(["rounds"])
    })
    .await;
    assert_eq!(card(&rig, &id).await["more"]["rounds"], 1);

    let started = looked_again(&mut rig).await;

    // The daemon has found a card in doing that a round has had and
    // nobody has now. Its move was said: it is not counted a second time.
    // The card's tidying is waited for, which takes its turn by the card
    // behind anything the look began for it.
    move_card(&rig, &id, "done", None).await;
    rig.event("run.finished", move |event| event.seq > started && event.data["flow"] == "on.tidied").await;
    assert_eq!(card(&rig, &id).await["more"]["rounds"], 1);
}

#[tokio::test]
async fn a_card_added_to_ready_as_the_daemon_went_with_nobody_told_is_worked_when_it_is_back() {
    let (mut rig, mut held) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    // A card of the board's, to find the board by.
    let other = add(&rig, "Sweep up", &[]).await;
    // One is added straight into the column that is worked, and the
    // daemon goes between writing it down and saying it: its row is
    // there, and nothing was ever said of it.
    let id = added_and_not_said(&rig, &other, "Add a cart", "ready").await;

    rig.restart().await;

    // It is said to have been added when the daemon is back, late, and
    // is from there a card added like any other: the stage takes it up.
    let its = id.clone();
    let late = rig.event("board.item.created", move |event| event.data["id"] == its.as_str()).await;
    assert_eq!((late.data["status"].clone(), late.data["late"].clone()), (json!("ready"), json!(true)));
    held.until_asked("work", 1).await;
    held.work();
    moved(&rig, &id, "review", late.seq).await;
    assert_eq!(given(&rig, "shop-worker-1").await.len(), 1);
}

/// What was said of cards moved to a column, late or not.
async fn said_moved_to(rig: &Rig, to: &str) -> Vec<serde_json::Value> {
    let query = journal::EventQuery { topics: "board.item.moved".into(), limit: 1000, ..Default::default() };
    let said = rig.said(query).await;
    said.into_iter().filter(|event| event.data["to"] == to).map(|event| event.data.clone()).collect()
}

#[tokio::test]
async fn a_move_nobody_was_told_of_is_said_however_long_the_daemon_was_away_and_a_board_is_looked_at_once_a_start()
 {
    let clock = kernel::ManualClock::new();
    let theirs = clock.clone();
    let rig = Rig::with("api-restarted-away", move |config, _| {
        config.clock = Some(std::sync::Arc::new(theirs));
    });
    let mut rig = Rig::owned(rig.await).await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    // A card made long before, moved with nobody told as the daemon goes.
    let id = add(&rig, "Add a cart", &[]).await;
    clock.advance(std::time::Duration::from_secs(86_400));
    moved_and_not_said(&rig, &id, "ready").await;
    rig.stop().await;
    // The daemon is away a day: whatever it says as it starts is of then,
    // and the row is of when it stopped.
    clock.advance(std::time::Duration::from_secs(86_400));
    rig.start().await;

    let look =
        json!({ "blueprint": "std/owed", "project": "shop", "params": { "project": "shop" }, "wait": true });
    let looked = rig.send("/api/apply", look.clone()).await;
    assert_eq!(looked["finished"]["status"], "ok", "{looked}");
    let said = said_moved_to(&rig, "ready").await;
    assert_eq!(said.len(), 1, "{said:?}");
    assert_eq!(
        (&said[0]["from"], &said[0]["late"], &said[0]["by"]),
        (&json!("draft"), &json!(true), &json!("system"))
    );

    // A board is looked at once for a start of the daemon. A row that
    // differs from what was said while the daemon is up is a move being
    // made, which says itself; it is looked for again at the next start.
    moved_and_not_said(&rig, &id, "doing").await;
    let looked = rig.send("/api/apply", look.clone()).await;
    assert_eq!(looked["finished"]["status"], "ok", "{looked}");
    assert!(said_moved_to(&rig, "doing").await.is_empty());
    rig.restart().await;
    let looked = rig.send("/api/apply", look).await;
    assert_eq!(looked["finished"]["status"], "ok", "{looked}");
    assert_eq!(said_moved_to(&rig, "doing").await.len(), 1);
    assert_eq!(said_moved_to(&rig, "ready").await.len(), 1);
}

#[tokio::test]
async fn a_card_is_not_added_or_moved_where_that_cannot_be_said_and_nothing_is_left_to_be_said_late() {
    let (mut rig, mut held) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    if !crate::one_write(&rig, "shop") {
        return;
    }
    let id = add(&rig, "Add a cart", &[]).await;
    // What is said of a card cannot be kept. A card's row and what is
    // said of it are one write: when they were two, the row was written
    // all the same, and a card stood where nobody had been told it was.
    // The journal that keeps the card, and so what is said of it.
    let journal = rig.node().app.journal.records_of(Some("shop"));
    let refuse = "CREATE TRIGGER unsaid BEFORE INSERT ON events
                  WHEN NEW.topic IN ('board.item.moved', 'board.item.created')
                  BEGIN SELECT RAISE(FAIL, 'database or disk is full'); END;";
    journal.db().call(move |connection| connection.execute_batch(refuse)).await.unwrap();
    let path = format!("/api/board/items/{id}");
    let asked = Some(json!({ "status": "ready", "note": "It is wanted this week." }));
    let (status, refused) = rig.call(reqwest::Method::PATCH, &path, Some(&rig.token), asked).await;
    assert_ne!(status, reqwest::StatusCode::OK, "{refused}");
    let (status, refused) = rig.post("/api/board/shop/items", json!({ "title": "Add a till" })).await;
    assert_ne!(status, reqwest::StatusCode::OK, "{refused}");
    journal.db().call(|connection| connection.execute_batch("DROP TRIGGER unsaid;")).await.unwrap();

    // Neither was written: the card is where it was, as it was, and
    // there is no other.
    let board = rig.read("/api/board/shop").await;
    let cards = board["items"].as_array().unwrap();
    assert_eq!(cards.len(), 1, "{board}");
    assert_eq!((cards[0]["id"].as_str(), cards[0]["status"].as_str()), (Some(id.as_str()), Some("draft")));
    assert!(cards[0]["fields"]["moved"].as_array().is_none_or(|moved| moved.is_empty()), "{board}");

    // So nothing is found when the daemon is back, and nothing said
    // late. Moved for good afterwards, it is said once, in the write
    // that moves it, and its worker is asked once.
    rig.restart().await;
    move_card(&rig, &id, "ready", None).await;
    held.until_asked("work", 1).await;
    let said = said_moved_to(&rig, "ready").await;
    assert_eq!(said.len(), 1, "{said:?}");
    assert_eq!((&said[0]["from"], &said[0]["late"]), (&json!("draft"), &json!(null)));
    let added = journal::EventQuery { topics: "board.item.created".into(), limit: 100, ..Default::default() };
    assert_eq!(rig.said(added).await.len(), 1);
    held.work();
    moved(&rig, &id, "review", 0).await;
}

#[tokio::test]
async fn what_was_said_as_a_card_was_moved_with_nobody_told_is_said_with_the_move_when_the_daemon_is_back() {
    let (mut rig, mut held) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    let id = add(&rig, "Add a cart", &[]).await;
    // A person made it ready and said why, and the saying of the move
    // was not kept: the row is written, with what was said, and nobody
    // was told.
    moved_saying_and_not_said(&rig, &id, "ready", "It is wanted this week.").await;

    rig.restart().await;

    // Said late, the move says what was said with it, and whoever works
    // the card is told.
    let its = id.clone();
    let late = rig
        .event("board.item.moved", move |event| {
            event.data["id"] == its.as_str() && event.data["to"] == "ready"
        })
        .await;
    assert_eq!(
        (late.data["late"].clone(), late.data["note"].clone()),
        (json!(true), json!("It is wanted this week."))
    );
    held.until_asked("work", 1).await;
    held.work();
    moved(&rig, &id, "review", late.seq).await;
    let told = given(&rig, "shop-worker-1").await;
    assert!(told[0].contains("It is wanted this week."), "{told:?}");
}

#[tokio::test]
async fn a_turn_that_comes_for_a_card_sent_back_before_the_move_is_heard_tells_its_worker_why_and_the_move_said_late_asks_nobody_again()
 {
    let (mut rig, mut held) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    let (id, _) = in_hand(&rig, &mut held).await;
    held.work();
    let handed_on = moved(&rig, &id, "review", 0).await;
    rig.event("run.finished", move |event| event.seq > handed_on && event.data["flow"] == "on.go").await;

    // Somebody sent it back and said why, and the saying of the move was
    // not kept: the row is written, as the move wrote it, and nothing has
    // heard.
    moved_saying_and_not_said(&rig, &id, "doing", "The total is wrong.").await;

    // A turn comes for the card first, as a second does for a card that
    // was said to be due twice: the wrong way round, the turn before
    // anything has heard of the sending back. Its worker has the card.
    let turn = format!(
        "blueprint: a-turn\nsteps:\n  one:\n    do: emit\n    with: {{ topic: pool.granted, pool: shop/ready, \
         to: \"key:{id}\", as: {id}, waited: false }}\n"
    );
    let said = rig.send("/api/apply", json!({ "project": "shop", "source": turn, "wait": true })).await;
    assert_eq!(said["finished"]["status"], "ok", "{said}");
    let thread = format!("lane:{id}");
    let asked = |after: u64| {
        let thread = thread.clone();
        move |event: &kernel::Event| event.seq > after && event.data["thread"] == thread.as_str()
    };
    rig.event("agent.turn.started", asked(handed_on)).await;

    // The daemon is stopped and started with the card in its worker's
    // hands, and the move is said then, late: what hears it asks for the
    // card's turn, which finds a round already told of that move.
    let before = rig.mark();
    rig.restart().await;
    let its = id.clone();
    let late = rig
        .event("board.item.moved", move |event| {
            event.seq > before && event.data["id"] == its.as_str() && event.data["to"] == "doing"
        })
        .await;
    assert_eq!(
        (late.data["late"].clone(), late.data["note"].clone()),
        (json!(true), json!("The total is wrong."))
    );
    held.work();
    moved(&rig, &id, "review", late.seq).await;
    // The turn the late saying asked for has come and been given back.
    let again = late.seq;
    let turns = std::sync::atomic::AtomicUsize::new(0);
    rig.event("run.finished", move |event| {
        let go =
            event.seq > again && event.data["blueprint"] == "std/working" && event.data["flow"] == "on.go";
        go && turns.fetch_add(1, std::sync::atomic::Ordering::SeqCst) >= 1
    })
    .await;
    let told = given(&rig, "shop-worker-1").await;
    assert_eq!(told.len(), 2, "{told:?}");
    assert!(told[1].contains("The total is wrong."), "{}", told[1]);
}

/// The flow of `std/cards` that ends, after `mark`, as `flow`.
async fn cards_ran(rig: &Rig, mark: u64, flow: &'static str) -> u64 {
    let ran = rig.event("run.finished", move |event| {
        event.seq > mark && event.data["blueprint"] == "std/cards" && event.data["flow"] == flow
    });
    ran.await.seq
}

/// What looks after a board's cards hears that the daemon started only
/// where it was listening then; one stopped between beginning and
/// listening (a board just made) is taken up after that was said. It
/// looked only on hearing it, so a move written and never said in that
/// stop was looked for by nobody at that start, and at the next its row
/// was older than what a start looks at closely: it was never said. It
/// looks as it comes up too, once for a start whoever asks.
#[tokio::test]
async fn a_move_nobody_was_told_of_as_what_looks_after_a_boards_cards_was_coming_up_is_said_when_the_daemon_is_back()
 {
    coming_up("api-late-coming-up", "local").await;
}

/// The same where anybody can still come to it: a board kept as files is
/// written and then said, so a person's move written to its card's file
/// with the daemon gone before the saying is the move nobody was told of.
#[tokio::test]
async fn a_move_written_to_a_card_kept_as_a_file_and_never_said_as_its_board_was_coming_up_is_said_when_the_daemon_is_back()
 {
    coming_up("api-late-coming-up-files", "files").await;
}

/// A card of a board kept `store`'s way made ready by a person, the move
/// written where the card is kept and never said, with the daemon gone
/// as what looks after the cards has begun and is not yet listening; and
/// the daemon started twice. The move is said once, at the first start.
async fn coming_up(name: &str, store: &str) {
    let clock = kernel::ManualClock::new();
    let theirs = clock.clone();
    let rig = Rig::with(name, move |config, _| {
        config.clock = Some(std::sync::Arc::new(theirs));
    });
    let mut rig = Rig::owned(rig.await).await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let params = json!({ "columns": ["draft", "ready", "doing", "review", "done"], "store": store });
    let board = json!({ "blueprint": "std/board", "project": "shop", "params": params, "wait": true });
    let applied = rig.send("/api/apply", board).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let id = add(&rig, "Add a cart", &[]).await;
    // What looks after the board's cards is to come up again, as it does
    // with a board just made, and no stage works the board.
    let live = rig.read("/api/instances?live=true&project=shop").await;
    let live = live["instances"].as_array().cloned().unwrap_or_default();
    let cards =
        live.iter().find(|one| one["blueprint"] == "std/cards").expect("the board's cards are looked after");
    rig.send(&format!("/api/instances/{}/stop", cards["id"].as_str().unwrap()), json!({})).await;

    // A person makes the card ready and nobody is told: written where it
    // is kept, and the saying lost. Where the board is in the database
    // the two are one write now, so the row is planted as a journal from
    // before holds it; a board kept as files is moved by the route, with
    // what keeps what is said of the project refusing the saying, which
    // is a daemon that went between the file and the event.
    if store == "local" {
        moved_and_not_said(&rig, &id, "ready").await;
    } else {
        let said = rig.node().app.journal.of(Some("shop"));
        let refuse = "CREATE TRIGGER unsaid BEFORE INSERT ON events WHEN NEW.topic = 'board.item.moved'
                      BEGIN SELECT RAISE(FAIL, 'the daemon went'); END;";
        said.db().call(move |connection| connection.execute_batch(refuse)).await.unwrap();
        move_card(&rig, &id, "ready", None).await;
        said.db().call(|connection| connection.execute_batch("DROP TRIGGER unsaid;")).await.unwrap();
        assert_eq!(card(&rig, &id).await["status"], "ready");
        assert!(said_moved_to(&rig, "ready").await.is_empty());
    }
    let journal = rig.node().app.journal.clone();
    journal.shut_after(|event| event.topic == "instance.started" && event.data["blueprint"] == "std/cards");
    let apply = json!({ "blueprint": "std/cards", "project": "shop", "params": { "project": "shop" } });
    // Whoever asks anything of a journal that has shut may never be answered.
    tokio::select! {
        _ = rig.send("/api/apply", apply) => {}
        _ = journal.shut() => {}
    }
    journal.shut().await;

    // Back, it comes up, and that is when it looks: it was not listening
    // when the daemon said it had started.
    let first = rig.mark();
    rig.restart().await;
    cards_ran(&rig, first, "on.first").await;

    // The daemon is away for a while and is started again, well past the
    // minute about a stop that a start looks at closely: the move is not
    // looked for again, and was said once, the first time.
    clock.advance(std::time::Duration::from_secs(600));
    let second = rig.mark();
    rig.restart().await;
    cards_ran(&rig, second, "on.started").await;
    let query = journal::EventQuery { topics: "board.item.moved".into(), limit: 1000, ..Default::default() };
    let said: Vec<_> =
        rig.said(query).await.into_iter().filter(|event| event.data["to"] == "ready").collect();
    assert_eq!(said.len(), 1, "{said:?}");
    assert_eq!((&said[0].data["from"], &said[0].data["late"]), (&json!("draft"), &json!(true)));
    assert!(said[0].seq > first && said[0].seq < second, "said at the first start: {:?}", said[0]);
}
