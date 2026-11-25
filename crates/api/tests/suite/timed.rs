//! How long each card took: a project given its table of that from its
//! board's page, the cards done before filled in once from their
//! histories, a card worked by hand written as it comes to the last
//! column and written again when it comes there again, and nothing kept
//! twice by asking again. What is kept is counted, never scored.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use kernel::ManualClock;
use serde_json::{Value, json};

use crate::Rig;
use crate::lanes::move_card;

const MINUTE: Duration = Duration::from_secs(60);

/// A node on a clock the test moves, with a project that has a board and no fleet.
async fn shop() -> (Rig, ManualClock) {
    let clock = ManualClock::new();
    let moved = clock.clone();
    let rig = Rig::with("api-timed", move |config, _| config.clock = Some(Arc::new(moved))).await;
    let rig = Rig::owned(rig).await;
    rig.send("/api/projects", json!({ "name": "Shop", "set_up": ["board"] })).await;
    (rig, clock)
}

async fn add(rig: &Rig, title: &str) -> String {
    rig.send("/api/board/shop/items", json!({ "title": title })).await["id"].as_str().unwrap().to_string()
}

/// Moves a card after so many minutes on the clock.
async fn after(rig: &Rig, clock: &ManualClock, minutes: u32, id: &str, column: &str) {
    clock.advance(MINUTE * minutes);
    move_card(rig, id, column, None).await;
}

/// The row of how long `card` took, once it is what `wanted` says of it.
async fn row(rig: &Rig, card: &str, wanted: impl Fn(&Value) -> bool) -> Value {
    crate::soon(&format!("the row of {card}"), async {
        loop {
            let (_, read) = rig.get("/api/collections/shop/timed/records").await;
            let rows = read["records"].as_array().cloned().unwrap_or_default();
            let found = rows.iter().find(|row| row["data"]["card"] == card).map(|row| row["data"].clone());
            if let Some(found) = found.filter(|found| wanted(found)) {
                return found;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
}

async fn rows(rig: &Rig) -> usize {
    rig.read("/api/collections/shop/timed/records").await["records"].as_array().unwrap().len()
}

#[tokio::test]
async fn how_long_each_card_took_is_kept_from_its_history_as_it_comes_to_the_last_column() {
    let (rig, clock) = shop().await;
    // It is offered where the cards are, needing nothing but a board.
    let meta = rig.read("/api/meta").await;
    let offered =
        meta["project_starts"].as_array().unwrap().iter().find(|one| one["name"] == "timed").cloned();
    let offered = offered.expect("card times are offered");
    assert_eq!(
        (offered["offered"].clone(), offered["on"].clone()),
        (json!("board"), json!(false)),
        "{offered}"
    );

    // A card done before the project had it.
    let early = add(&rig, "Sell the first loaf").await;
    after(&rig, &clock, 30, &early, "doing").await;
    after(&rig, &clock, 45, &early, "done").await;

    let given = rig.send("/api/projects/shop/starts", json!({ "names": ["timed"] })).await;
    assert!(given["left"].is_null(), "{given}");
    // Filled in once, from its history.
    let kept = row(&rig, &early, |row| row["total"] == json!(75.0)).await;
    assert_eq!(kept["in_backlog"], json!(30.0), "{kept}");
    assert_eq!(kept["in_doing"], json!(45.0), "{kept}");
    assert_eq!((kept["back"].clone(), kept["reached"].clone()), (json!(0), json!(1)), "{kept}");
    assert_eq!(kept["title"], "Sell the first loaf", "{kept}");
    assert_eq!(kept["whole"], true, "{kept}");

    // A card worked by hand, sent back once, done.
    let card = add(&rig, "Bake the bread").await;
    after(&rig, &clock, 10, &card, "ready").await;
    after(&rig, &clock, 5, &card, "doing").await;
    after(&rig, &clock, 20, &card, "review").await;
    after(&rig, &clock, 5, &card, "doing").await;
    after(&rig, &clock, 10, &card, "review").await;
    assert_eq!(rows(&rig).await, 1, "nothing is kept of a card before it is done");
    after(&rig, &clock, 15, &card, "done").await;
    let done = row(&rig, &card, |row| row["reached"] == json!(1)).await;
    let each =
        ["total", "in_backlog", "in_ready", "in_doing", "in_review", "back"].map(|key| done[key].clone());
    assert_eq!(each, [json!(65.0), json!(10.0), json!(5.0), json!(30.0), json!(20.0), json!(1)], "{done}");
    assert_eq!(done["worked"], json!(["Ada"]), "{done}");

    // Taken out of done and back again: its row says so, and is still one.
    after(&rig, &clock, 60, &card, "review").await;
    after(&rig, &clock, 5, &card, "done").await;
    let again = row(&rig, &card, |row| row["reached"] == json!(2)).await;
    assert_eq!((again["total"].clone(), again["back"].clone()), (json!(130.0), json!(2)), "{again}");
    assert_eq!(rows(&rig).await, 2);

    // One added straight to the last column has its row as it is added.
    clock.advance(MINUTE * 3);
    let added =
        rig.send("/api/board/shop/items", json!({ "title": "Found in the oven", "status": "done" })).await;
    let added = added["id"].as_str().unwrap().to_string();
    let straight = row(&rig, &added, |row| row["reached"] == json!(1)).await;
    assert_eq!((straight["total"].clone(), straight["back"].clone()), (json!(0.0), json!(0)), "{straight}");

    // A column added to the board has its time kept from then on.
    let columns = json!({ "columns": ["backlog", "ready", "doing", "review", "staging", "done"] });
    rig.send("/api/board/shop/columns", columns).await;
    crate::soon("the table to keep a time for staging", async {
        loop {
            let tables = rig.read("/api/collections?project=shop").await.to_string();
            if tables.contains("in_staging") {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    let staged = add(&rig, "Wrap the loaves").await;
    after(&rig, &clock, 5, &staged, "staging").await;
    after(&rig, &clock, 7, &staged, "done").await;
    let kept = row(&rig, &staged, |row| row["reached"] == json!(1)).await;
    assert_eq!((kept["in_backlog"].clone(), kept["in_staging"].clone()), (json!(5.0), json!(7.0)), "{kept}");

    // Applied again for the project, the cards done are filled in again
    // and nothing is kept twice.
    let instance = given["timed"].as_str().unwrap().to_string();
    let applied =
        rig.send(&format!("/api/instances/{instance}/apply"), json!({ "force": true, "wait": true })).await;
    assert!(applied["finished"].is_object(), "its steps ran again: {applied}");
    assert_eq!(rows(&rig).await, 4);
    let still = row(&rig, &card, |_| true).await;
    assert_eq!(still, again);
}

/// Every journal's database counting its own steps from when it was
/// made, each thing its program does (`progress_handler`), as a card's
/// cost is counted (`costed.rs`), with what one transaction's beginning
/// and keeping take of them, so that what is counted is what the work
/// itself did.
///
/// A transaction is begun and kept for each commit, and played with
/// gathering forced (`ways gathered`) a commit keeps whatever was waiting
/// as it was begun, so how many there are is how the machine's work fell
/// and says nothing of what was asked: of the same two moves, one run
/// began 30 transactions and another 44, and counted with them a card
/// done beside 60 came to 1214 steps where beside 3 it came to 1151
/// (three at once on one processor). What beginning
/// and keeping one take is the same every time (eight steps), so those
/// are taken out of what is counted, each database's by how many it
/// began (`Db::begun`): what is left of the two moves came to 2597 to
/// 2603 and to 3698 or 3699 in every run, gathered or not.
struct Stepping {
    /// Each database, what it has counted, what one transaction takes of
    /// that and how many it had begun when it began to count.
    dbs: Vec<(journal::Db, Arc<AtomicU64>, u64, u64)>,
}

impl Stepping {
    async fn new(rig: &Rig) -> Stepping {
        let mut dbs = Vec::new();
        for db in rig.node().app.journal.dbs() {
            let counted = Arc::new(AtomicU64::new(0));
            let counting = counted.clone();
            let told = db.call(move |connection| {
                connection.progress_handler(
                    1,
                    Some(move || {
                        counting.fetch_add(1, Ordering::Relaxed);
                        false
                    }),
                )
            });
            told.await.unwrap();
            let from = db.begun();
            dbs.push((db, counted, 0, from));
        }
        crate::simulation::rested(rig).await;
        // A piece of work that does nothing is a transaction begun and
        // kept and nothing else, where nothing else was done beside it.
        for (db, counted, each, _) in &mut dbs {
            *each = crate::soon("a piece of work done by itself", async {
                loop {
                    let (steps, begun, done) = (counted.load(Ordering::Relaxed), db.begun(), db.done());
                    db.call(|_| Ok(())).await.unwrap();
                    if (db.begun() - begun, db.done() - done) == (1, 1) {
                        return counted.load(Ordering::Relaxed) - steps;
                    }
                }
            })
            .await;
        }
        Stepping { dbs }
    }

    /// The steps counted so far, of every database, without those that
    /// began and kept transactions.
    fn own(&self) -> u64 {
        let all = self.dbs.iter().map(|(db, counted, each, from)| {
            counted.load(Ordering::Relaxed).saturating_sub((db.begun() - from) * each)
        });
        all.sum()
    }
}

/// The steps from a move of `card` to `column` until the factory is at rest again.
async fn steps_of_a_move(rig: &Rig, stepping: &Stepping, card: &str, column: &str) -> u64 {
    crate::simulation::rested(rig).await;
    let before = stepping.own();
    move_card(rig, card, column, None).await;
    crate::simulation::rested(rig).await;
    stepping.own() - before
}

/// How many of the databases' own steps keeping a card's row costs, with
/// `done` cards done before it: what its coming to the last column costs
/// over and above another card's coming to a column before it, which
/// card times look at and pass by. (What a move costs by itself is the
/// board's, and is held by `costed.rs`.)
async fn steps_for_one_done(done: usize) -> u64 {
    let (rig, _clock) = shop().await;
    rig.send("/api/projects/shop/starts", json!({ "names": ["timed"] })).await;
    for at in 0..done {
        let before = rig.mark();
        let card = rig
            .send("/api/board/shop/items", json!({ "title": format!("Done {at}"), "status": "done" }))
            .await;
        let id = card["id"].as_str().unwrap().to_string();
        rig.event("run.finished", move |event| event.seq > before && event.data["flow"] == "on.came").await;
        row(&rig, &id, |_| true).await;
    }
    let (passed, kept) = (add(&rig, "Weigh the flour").await, add(&rig, "Bake the bread").await);
    for card in [&passed, &kept] {
        move_card(&rig, card, "doing", None).await;
    }
    let stepping = Stepping::new(&rig).await;
    let by = steps_of_a_move(&rig, &stepping, &passed, "review").await;
    let done = steps_of_a_move(&rig, &stepping, &kept, "done").await;
    let row = row(&rig, &kept, |row| row["reached"] == json!(1)).await;
    assert_eq!(row["card"], kept.as_str());
    done.saturating_sub(by)
}

#[tokio::test]
async fn a_card_done_costs_the_same_however_many_cards_are_done_before_it() {
    // Its row is kept under the card's own number and found by its id,
    // never by reading the table of every card done.
    let (few, many) = (steps_for_one_done(3).await, steps_for_one_done(60).await);
    println!("keeping a card's row: {few} steps beside 3 cards done, {many} beside 60");
    assert!(few > 0, "keeping a row costs something");
    assert!(
        many <= few + few / 20,
        "the databases' own steps for keeping one card's row: {few} beside 3 cards done and {many} beside 60"
    );
}
