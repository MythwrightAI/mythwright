//! A card that has things of its project's own done where it is reviewed,
//! stopped anywhere and started again.
//!
//! The scenario is a card reviewed and merged (`simulated.rs`) in a
//! project whose `segments` has three rows for the column it is reviewed
//! in: a command run where the card was worked before anybody looks at
//! it, one run once it is approved and before its work is merged, and one
//! after the merge. What is new to be stopped in is the stage's run that
//! has the list gone through, the going through itself and each thing it
//! calls with its command under way, the review in session between them,
//! the run that hears what the review came to, and the merge. Wherever the
//! daemon goes, the card is checked, looked at once, merged and done.

use kernel::Event;
use serde_json::json;

use crate::fleets;
use crate::found::Grain;
use crate::observed::node_with_stand_ins;
use crate::simulated::{card_called, in_hand, moved, ready, shop_came_to, shop_with_a_board, tidied};
use crate::simulation::{Came, Place, Play, Scenario, simulate};

/// A card is worked, the project's command is run on its work, an agent
/// reviews and approves it, and it is merged.
#[derive(Clone)]
struct Checked;

impl Scenario for Checked {
    const ACTS: usize = 1;

    async fn stage(&self) -> Play {
        let (rig, at) = node_with_stand_ins("api-segmented").await;
        let dir = shop_with_a_board(&rig, &[]).await;
        fleets::fleet(&rig, json!({ "workers": 1, "reviewers": 1 })).await;
        // Whoever works leaves a file of changes in the card's place,
        // which is where the command is run: it passes there.
        let row = json!({
            "title": "Tests", "stage": "review", "order": 5, "does": "std/run",
            "command": crate::standin::line(&["any", "CHANGES-"]),
            "record_to": "test-runs",
        });
        fleets::add_row(&rig, "segments", row).await;
        let approved = json!({
            "title": "Licences", "stage": "review", "order": 15, "does": "std/run",
            "command": crate::standin::line(&["any", "CHANGES-"]),
        });
        fleets::add_row(&rig, "segments", approved).await;
        let merged = json!({
            "title": "Announce", "stage": "review", "order": 25, "does": "std/run", "command": "exit 0",
        });
        fleets::add_row(&rig, "segments", merged).await;
        handed_rows(&rig, 3).await;
        let mut play = Play::on(rig, &at).await;
        play.notes.insert("dir", dir.display().to_string());
        play
    }

    async fn act(&self, _: usize, play: &mut Play) {
        let id = ready(play, "Add a cart").await;
        play.notes.insert("card", id);
    }

    fn heard(&self, _: usize, play: &Play, _: &Event) -> bool {
        tidied(play) == 1
    }

    async fn came_to(&self, play: &Play) -> Came {
        // Each run of the first command that ended is kept, and it passed
        // each time: one run, or two where the daemon went between its
        // being kept and the card going on, since one done again after a
        // stop is another run.
        // A table no run was ever kept in is not there: no run kept, said
        // as what the play came to and not as a failed read.
        let (_, runs) = play.rig.get("/api/collections/shop/test-runs/records").await;
        let runs = runs["records"].as_array().cloned().unwrap_or_default();
        let kept = (1..=2).contains(&runs.len()) && runs.iter().all(|run| run["data"]["result"] == "passed");
        let came = shop_came_to(play, &["shop-worker-1", "shop-reviewer-1"]).await;
        came.with("its runs kept, each passed", json!(kept))
    }
}

/// Waits until every stage of the fleet has been handed `rows` rows of
/// `segments`. Adding a row waits for the fleet to say it was arranged,
/// which an arrangement a row before may say after the row is added and
/// without it; and the factory being at rest is a while of quiet, which a
/// crowded machine gives before the last arrangement has begun. Begun
/// with a stage that had not been handed a row yet, the card was reviewed
/// with nothing done before its review, and no run was kept. So the play begins with every stage holding them.
async fn handed_rows(rig: &crate::Rig, rows: usize) {
    let stages = ["std/working", "std/reviewing", "std/planning"];
    crate::soon("every stage to be handed the rows of segments", async {
        loop {
            let listed = rig.read("/api/instances?project=shop&live=true&limit=1000").await;
            let all = listed["instances"].as_array().cloned().unwrap_or_default();
            let of_stages: Vec<&serde_json::Value> = all
                .iter()
                .filter(|one| stages.contains(&one["blueprint"].as_str().unwrap_or_default()))
                .collect();
            let handed =
                |one: &&serde_json::Value| one["params"]["segments"].as_array().map_or(0, Vec::len) == rows;
            if !of_stages.is_empty() && of_stages.iter().all(handed) {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
}

/// Where the things done with a card in review are under way: the going
/// through, each thing it calls and the merge, as the steps of each begin.
/// Every other place of them, and the stage's own runs about them (the
/// one that has the list gone through, the one that hears what the review
/// came to), are played with every place, when that is asked for.
fn in_a_segment(place: &Place) -> bool {
    let done = ["std/run", "std/segments", "std/merge"].iter().any(|text| place.within(text));
    done && place.topic == "run.started"
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_card_stopped_in_a_segment_or_as_what_it_came_to_is_taken_up_is_checked_reviewed_and_merged_all_the_same()
 {
    let grain = Grain::asked_or(Grain::Where(in_a_segment));
    let found = simulate("a card checked before its review", Checked, grain).await;
    found.holds();
}

/// A card is added where a stage plans, with a command of the project's
/// run before anybody is asked how it would be done and another once it
/// has its plan; its stage then hands it on, and it is worked.
#[derive(Clone)]
struct PlannedAbout;

impl Scenario for PlannedAbout {
    const ACTS: usize = 1;

    async fn stage(&self) -> Play {
        let (rig, at) = node_with_stand_ins("api-segmented").await;
        let dir = shop_with_a_board(&rig, &[]).await;
        // The fleet's tables as they are first filled: one who plans and
        // one who works, and a stage for each. Nobody reviews: the play
        // is over when the card has been worked.
        let agents = json!([
            { "title": "worker", "role": "worker", "count": 1, "adapter": "writer" },
            { "title": "architect", "role": "architect", "adapter": "critic", "permission": "read" },
        ]);
        let stages = json!([
            { "title": "plan", "column": "draft", "does": "plan", "role": "architect", "pass": "ready" },
            { "title": "work", "column": "ready", "does": "work", "role": "worker", "pass": "review" },
        ]);
        fleets::fleet(&rig, json!({ "agents": agents, "stages": stages })).await;
        for (title, order) in [("Triage", 5), ("Lint", 15)] {
            let row = json!({ "title": title, "stage": "plan", "order": order, "does": "std/run", "command": "exit 0" });
            fleets::add_row(&rig, "segments", row).await;
        }
        handed_rows(&rig, 2).await;
        let mut play = Play::on(rig, &at).await;
        play.notes.insert("dir", dir.display().to_string());
        play
    }

    async fn act(&self, _: usize, play: &mut Play) {
        // Added where it is planned, as a person would who was not sure
        // their last click took.
        let id = match card_called(play, "Add a cart").await {
            Some(card) => card["id"].as_str().unwrap().to_string(),
            None => fleets::add(&play.rig, "Add a cart", &[]).await,
        };
        play.notes.insert("card", id);
    }

    fn heard(&self, _: usize, play: &Play, event: &Event) -> bool {
        moved(event, play.note("card"), "review")
    }

    async fn came_to(&self, play: &Play) -> Came {
        shop_came_to(play, &["shop-worker-1", "shop-architect-1"]).await
    }

    fn either(&self, place: &Place) -> Option<&'static str> {
        crate::either::a_proposal_ended(place)
    }
}

/// Where what is laid about a plan is gone through, before it and after
/// it, and where the card is then handed on: as the steps of each begin.
/// The things a going through calls are stopped in by the scenario of a
/// card checked before its review, and here with every place.
fn about_a_plan(place: &Place) -> bool {
    let done = ["std/segments", "std/onward"].iter().any(|text| place.within(text));
    done && place.topic == "run.started"
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_card_stopped_in_what_is_laid_about_its_plan_is_planned_once_handed_on_and_worked_all_the_same() {
    let grain = Grain::asked_or(Grain::Where(about_a_plan));
    let found = simulate("a card with things laid about its plan", PlannedAbout, grain).await;
    found.holds();
}

/// The same card, held to its architect being asked once as its worker is:
/// the plan's turn is one of `std/turn`, which keeps nothing, under the
/// round that the proposal calls for it.
#[derive(Clone)]
struct PlannedOnce;

impl Scenario for PlannedOnce {
    const ACTS: usize = 1;

    async fn stage(&self) -> Play {
        PlannedAbout.stage().await
    }

    async fn act(&self, nth: usize, play: &mut Play) {
        PlannedAbout.act(nth, play).await
    }

    fn heard(&self, nth: usize, play: &Play, event: &Event) -> bool {
        PlannedAbout.heard(nth, play, event)
    }

    async fn came_to(&self, play: &Play) -> Came {
        let mut came = PlannedAbout.came_to(play).await;
        let times = came.asked.remove("shop-architect-1");
        came.with("times shop-architect-1 was asked", json!(times))
    }

    fn either(&self, place: &Place) -> Option<&'static str> {
        PlannedAbout.either(place)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_card_whose_planner_or_worker_has_its_turn_in_hand_when_the_daemon_is_stopped_is_planned_and_worked_each_by_one_asking()
 {
    let found =
        simulate("a card planned and worked, held to one asking each", PlannedOnce, Grain::Where(in_hand))
            .await;
    found.holds();
    assert!(
        found.tried.len() >= 4,
        "each turn as it is said to begin and with its program there: {}",
        found.tried.len()
    );
}
