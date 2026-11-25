//! A card its column holds until a person presses what the column offers
//! (a row of the fleet's `actions`), stopped anywhere about the press and
//! started again.
//!
//! The card is made ready, waits and says so once, is pressed, and is
//! worked and passed by a person. Stopped as it is said to be held, or
//! as it is pressed, or anywhere else by hand (`SIMULATE=kinds`), it says
//! once that it waits, is worked once and passed: a press the daemon
//! went away just after is found when it is back, since the press and
//! what is said of it are one write.

use kernel::Event;
use serde_json::json;

use crate::fleets;
use crate::found::Grain;
use crate::lanes::move_card;
use crate::observed::node_with_stand_ins;
use crate::simulated::{moved, ready, shop_came_to, shop_with_a_board, tidied};
use crate::simulation::{Came, Place, Play, Scenario, simulate};

/// A card made ready where its column holds it until a person presses
/// Signed: it waits, says so once, is pressed, and is worked and passed
/// by a person.
#[derive(Clone)]
struct PressedToGo;

impl Scenario for PressedToGo {
    const ACTS: usize = 3;

    async fn stage(&self) -> Play {
        let (rig, at) = node_with_stand_ins("api-simulated").await;
        let dir = shop_with_a_board(&rig, &[]).await;
        fleets::fleet(&rig, json!({ "workers": 1, "reviewers": 0 })).await;
        let signed = json!({ "title": "Signed", "column": "ready", "who": "person", "holds": true });
        fleets::add_row(&rig, "actions", signed).await;
        let mut play = Play::on(rig, &at).await;
        play.notes.insert("dir", dir.display().to_string());
        play
    }

    /// Looking first, as a person does: a card still in ready is pressed,
    /// and pressing what stands pressed presses nothing, so a press the
    /// daemon went away in the middle of is pressed again when it is back;
    /// one it kept may take the card up between the look and the press,
    /// which is then refused, and that is the card having gone on.
    async fn act(&self, nth: usize, play: &mut Play) {
        let id = match nth {
            0 => {
                let id = ready(play, "Lay the floor").await;
                play.notes.insert("card", id);
                return;
            }
            _ => play.note("card").to_string(),
        };
        let column = fleets::card(&play.rig, &id).await["status"].clone();
        match (nth, column.as_str()) {
            (1, Some("ready")) => {
                // Looked at in ready, the card may be taken up a moment later by a press the daemon kept
                // before it went away: pressed again then, it is refused for being in work, which is so.
                let path = format!("/api/board/items/{id}/press");
                let (status, answer) = play.rig.post(&path, json!({ "action": "signed" })).await;
                if status != reqwest::StatusCode::OK {
                    let now = fleets::card(&play.rig, &id).await["status"].clone();
                    assert_ne!(now, "ready", "POST {path}: {answer}");
                }
            }
            (2, Some("review")) => move_card(&play.rig, &id, "done", None).await,
            _ => {}
        }
    }

    fn heard(&self, nth: usize, play: &Play, event: &Event) -> bool {
        match nth {
            0 => event.topic == "board.item.held" && event.data["id"] == play.note("card"),
            1 => moved(event, play.note("card"), "review"),
            _ => tidied(play) == 1,
        }
    }

    async fn came_to(&self, play: &Play) -> Came {
        shop_came_to(play, &["shop-worker-1"]).await
    }
}

/// Where a card is said to be held, and where it is pressed.
fn held_or_pressed(place: &Place) -> bool {
    place.topic == "board.item.held" || place.topic == "board.item.pressed"
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_card_held_for_a_press_is_taken_up_once_pressed_wherever_the_daemon_is_stopped_about_the_press() {
    simulate("a card held until it is pressed", PressedToGo, Grain::Where(held_or_pressed)).await.holds();
}
