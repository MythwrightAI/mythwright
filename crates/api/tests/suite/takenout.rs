//! A card moved out of doing while its agent is at it, and back, stopped
//! anywhere about the move and started again.
//!
//! The card is made ready and taken; while its agent's program is at it
//! somebody moves it back to the drafts, which stops its round and calls
//! off its agent's turn; it is made ready again and taken up where it was
//! left; its agent is let finish, and a person passes it. Stopped as the
//! move is said, as the round is stopped, as the turn is called off, or
//! anywhere else by hand (`SIMULATE=kinds`), it comes to the same: done,
//! with its agent asked once for each round of it.
//!
//! The card is moved only once the worker's program is there, and not as
//! its turn is said to begin: a turn is said to begin a moment before its
//! program is started, and a turn called off in that moment rightly starts
//! no program. Moved as the turn was said to begin, on a busy machine the
//! move came first, and the program was counted once where the play with
//! nothing in its way had it twice.
//!
//! Stopped from the move being said until the round's stop has called its
//! agent's turn off, or as the worker's first turn is queued or said to
//! begin, or as its program is there, the daemon begins that turn as it
//! starts, while the round asked for again finds its card moved, or is
//! stopped by the move before it has come to the turn, and ends without
//! asking for it. The stage lets go of what the round asked before the stop once the
//! round is over (`agent.let_go`), and the turn is called off and its
//! program ended, rather than its agent working the card for nobody and
//! the round of the card put back waiting behind it.

use std::path::PathBuf;

use kernel::Event;
use serde_json::json;

use crate::Rig;
use crate::fleets;
use crate::found::Grain;
use crate::lanes::move_card;
use crate::simulated::{moved, ready, shop_came_to, shop_with_a_board, tidied};
use crate::simulation::{BEGAN, Came, Place, Play, Scenario, simulate};

/// The worker's program waits here, once it is there, until the play lets
/// it work: so that the card can be moved away while it is at it.
const AT_WORK: &str = "at-work";

/// A node whose agents are stand-ins as the simulation's are, but that the
/// one who works waits a second time, at [`AT_WORK`], before it works.
async fn node(label: &str) -> (Rig, PathBuf) {
    let mut at = None;
    let rig = Rig::with(label, |config, dir| {
        let file = |name: &str| dir.join(name).to_string_lossy().into_owned();
        let (began, hold, ended, work) = (file("began"), file("hold"), file("ended"), file(AT_WORK));
        crate::hold(&dir.join(AT_WORK));
        let stands_in = |who: &str, does: &[&str]| {
            let first = ["read", "tell", &began, who, "held", &hold, who];
            crate::standin::argv(&[&first[..], does, &["append", &ended, who][..]].concat())
        };
        let writes = stands_in(
            "writer",
            &["held", &work, "writer", "write", "CHANGES-%:sum%.md", "changed", "say", "Done."],
        );
        let looks = stands_in("critic", &["say", "It does what the card asks.", "say", "DECISION: approve"]);
        config.adapters.push(("writer".into(), writes));
        config.adapters.push(("critic".into(), looks));
        at = Some(dir.to_path_buf());
    })
    .await;
    (Rig::owned(rig).await, at.unwrap())
}

/// Whether the stage's run for the card's round has ended.
fn round_over(event: &Event) -> bool {
    event.topic == "run.finished" && event.data["blueprint"] == "std/working" && event.data["flow"] == "on.go"
}

/// Whether the worker's turn has begun.
fn at_it(event: &Event) -> bool {
    event.topic == "agent.turn.started" && event.data["slot"] == "shop-worker-1"
}

/// Whether the worker's program is a place the play passed.
fn writer_there(place: &Place) -> bool {
    place.topic == BEGAN && place.who == "writer"
}

#[derive(Clone)]
struct TakenOut;

impl Scenario for TakenOut {
    const ACTS: usize = 5;

    async fn stage(&self) -> Play {
        let (rig, at) = node("api-takenout").await;
        let dir = shop_with_a_board(&rig, &[]).await;
        fleets::fleet(&rig, json!({ "workers": 1, "reviewers": 0 })).await;
        let mut play = Play::on(rig, &at).await;
        play.notes.insert("dir", dir.display().to_string());
        play.notes.insert("work", at.join(AT_WORK).display().to_string());
        play
    }

    /// Looking first, as a person does.
    async fn act(&self, nth: usize, play: &mut Play) {
        if nth == 0 {
            let id = ready(play, "Lay the floor").await;
            play.notes.insert("card", id);
            return;
        }
        let id = play.note("card").to_string();
        let column = fleets::card(&play.rig, &id).await["status"].clone();
        match (nth, column.as_str()) {
            (1, Some("doing")) => move_card(&play.rig, &id, "draft", Some("Not yet.")).await,
            (2, Some("draft")) => move_card(&play.rig, &id, "ready", Some("Now.")).await,
            // Its agent is let work, however often it is asked from here.
            (3, _) => crate::pass(&PathBuf::from(play.note("work")), 100),
            (4, Some("review")) => move_card(&play.rig, &id, "done", None).await,
            _ => {}
        }
    }

    fn heard(&self, nth: usize, play: &Play, event: &Event) -> bool {
        match nth {
            // Its program is there, and waits to be let work: what is
            // heard after it (or the lull) says so.
            0 => play.times(writer_there) >= 1,
            2 => at_it(event),
            1 => round_over(event),
            3 => moved(event, play.note("card"), "review"),
            _ => tidied(play) == 1,
        }
    }

    async fn came_to(&self, play: &Play) -> Came {
        shop_came_to(play, &["shop-worker-1"]).await
    }
}

/// Where the card is moved, where its round's stop is under way and where
/// its agent's turn ends; and where its worker's first turn is queued,
/// begins and its program is first there, the daemon begun again having it
/// waiting or in hand as the card is moved away.
fn about_the_move(place: &Place) -> bool {
    let turn = place.topic == "agent.turn.queued" || place.topic == "agent.turn.started";
    let first = place.nth == 1 && (turn || writer_there(place));
    first
        || place.topic == "board.item.moved"
        || (place.within("std/working") && place.flow == "on.away")
        || place.topic == "agent.turn.finished"
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_card_moved_out_of_doing_while_its_agent_is_at_it_comes_to_the_same_wherever_the_daemon_is_stopped()
{
    let grain = Grain::asked_or(Grain::Where(about_the_move));
    let found = simulate("a card taken out of its agent's hands and back", TakenOut, grain).await;
    found.holds();
    // With nothing in its way the worker's program was there before the
    // card was moved away, and was started again for the round after.
    let moved_away =
        |place: &Place| place.topic == "board.item.moved" && place.blueprint.is_empty() && place.nth == 2;
    let there = found.passed.iter().position(writer_there);
    let away = found.passed.iter().position(moved_away);
    assert!(
        there.zip(away).is_some_and(|(there, away)| there < away),
        "the worker's program was not there before the card was moved away"
    );
    assert_eq!(found.passed.iter().filter(|place| writer_there(place)).count(), 2);
}
