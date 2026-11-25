//! A card reviewed by a council of three, stopped anywhere a seat is and
//! started again.
//!
//! The scenario is a card worked, reviewed and merged (`simulated.rs`)
//! where the stage that reviews says three look and all three have to
//! approve. Those who look are one step of the review (`agent.task` with
//! `seats`), which keeps which turn is whose, and the review is kept: a
//! daemon stopped as a seat is asked, as its turn begins, with its
//! program there, or once it has answered while the others still look,
//! has the review wait for the same seats when it is back, each agent
//! given the turn it had.
//!
//! And a council laid by a row of `segments` into the stage that works,
//! between the work and the keeping (`std/turn` with `seats`): a round
//! keeps nothing, so a daemon stopped while two look has the round begun
//! again from the first of its list, the work given the turn it had and
//! each seat the turn it had, wherever a seat was.

use kernel::Event;
use serde_json::json;

use crate::fleets;
use crate::found::Grain;
use crate::observed::node_with_stand_ins;
use crate::simulated::{ready, shop, shop_came_to, shop_with_a_board, tidied};
use crate::simulation::{BEGAN, Came, Place, Play, Scenario, simulate};

/// A card is worked, three agents review it side by side and all three
/// have to approve it, and it is merged: a council, which is one step of
/// its review (`agent.task` with `seats`).
#[derive(Clone)]
struct ByACouncil;

/// Those who review in it.
const COUNCIL: [&str; 3] = ["shop-reviewer-1", "shop-reviewer-2", "shop-reviewer-3"];

impl Scenario for ByACouncil {
    const ACTS: usize = 1;

    async fn stage(&self) -> Play {
        let all = ("stages", "review", json!({ "seats": 3, "needed": 3 }));
        shop(1, 3, &[], std::slice::from_ref(&all)).await
    }

    async fn act(&self, _: usize, play: &mut Play) {
        let id = ready(play, "Add a cart").await;
        play.notes.insert("card", id);
    }

    fn heard(&self, _: usize, play: &Play, _: &Event) -> bool {
        tidied(play) == 1
    }

    async fn came_to(&self, play: &Play) -> Came {
        let agents = [&["shop-worker-1"][..], &COUNCIL[..]].concat();
        let mut came = shop_came_to(play, &agents).await;
        // Three look, so their program is counted by the seat: started
        // three times is once for each of them, and begun again after a
        // stop is once more for each that was at work.
        let critics = "the program of the critic";
        if let Some(started) = came.asked.get_mut(critics) {
            *started = started.div_ceil(3);
        }
        came
    }
}

/// Where one of a council's seats is asked, before the review has kept
/// which turns it waits for; at the start of its turn; has its program
/// there; or has answered while others still look.
fn a_seat(place: &Place) -> bool {
    let turn = ["agent.turn.queued", "agent.turn.started", "agent.turn.finished"];
    (turn.contains(&place.topic.as_str()) && COUNCIL.contains(&place.who.as_str()))
        || (place.topic == BEGAN && place.who == "critic")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_card_three_review_side_by_side_is_merged_all_the_same_wherever_a_seat_is_when_the_daemon_is_stopped()
 {
    let found =
        simulate("a card reviewed by a council of three", ByACouncil, Grain::asked_or(Grain::Where(a_seat)))
            .await;
    found.holds();
    // Each seat as it is asked, as its turn begins, with its program
    // there, and once it has answered.
    assert!(found.tried.len() >= 12 || std::env::var("SIMULATE").is_ok(), "{}", found.tried.len());
}

/// A card is worked, two agents look at the work at once before it is
/// kept and both approve, one reviews it, and it is merged: a council
/// that a row lays into the work stage's list.
#[derive(Clone)]
struct ByACouncilAtWork;

/// Those who look in it, who review after it too.
const AT_WORK: [&str; 2] = ["shop-reviewer-1", "shop-reviewer-2"];

impl Scenario for ByACouncilAtWork {
    const ACTS: usize = 1;

    async fn stage(&self) -> Play {
        let (rig, at) = node_with_stand_ins("api-seated").await;
        let dir = shop_with_a_board(&rig, &[]).await;
        fleets::fleet(&rig, json!({ "workers": 1, "reviewers": 2, "seats": 1 })).await;
        let row = json!({
            "title": "Second look", "stage": "work", "order": 15, "does": "std/turn", "role": "reviewer",
            "seats": 2, "needed": 2, "ends": ["passed=on failed=back"],
        });
        fleets::add_row(&rig, "segments", row).await;
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
        let agents = [&["shop-worker-1"][..], &AT_WORK[..]].concat();
        let mut came = shop_came_to(play, &agents).await;
        // Two look at the work and one reviews it: started three times,
        // and begun again after a stop once more for each that was at work.
        let critics = "the program of the critic";
        if let Some(started) = came.asked.get_mut(critics) {
            *started = started.div_ceil(3);
        }
        came
    }
}

/// Where a seat of the council at work begins: its turn queued and begun,
/// and its program there.
fn a_seat_begins(place: &Place) -> bool {
    let turn = ["agent.turn.queued", "agent.turn.started"];
    (turn.contains(&place.topic.as_str()) && AT_WORK.contains(&place.who.as_str()))
        || (place.topic == BEGAN && place.who == "critic")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_card_two_look_at_at_once_where_it_is_worked_is_kept_reviewed_and_merged_wherever_a_seat_begins_when_the_daemon_is_stopped()
 {
    let grain = Grain::asked_or(Grain::Where(a_seat_begins));
    let found = simulate("a card two look at where it is worked", ByACouncilAtWork, grain).await;
    found.holds();
    // Each of the two seats and the review's as it is queued, as it
    // begins and with its program there.
    assert!(found.tried.len() >= 6 || std::env::var("SIMULATE").is_ok(), "{}", found.tried.len());
}
