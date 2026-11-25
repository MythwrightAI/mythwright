//! Somebody named with `@` on a card, stopped anywhere in the telling and
//! started again.
//!
//! A comment on a card names a person; the keeper of histories writes
//! its line of the card's history, tells her in her box, and only then
//! keeps where it has read to (`studio::named`). What is new to be
//! stopped in is between those: as the comment is said, as its line of
//! history is written (before she is told), and as she is told (before
//! the place is kept). Wherever the daemon goes, it comes back with the
//! comment said once, its line written once, and one line in her box,
//! told once.

use kernel::Event;
use reqwest::Method;
use serde_json::{Value, json};

use crate::found::Grain;
use crate::kept::person;
use crate::observed::node_with_stand_ins;
use crate::simulated::shop_with_a_board;
use crate::simulation::{Came, Place, Play, Scenario, simulate};

/// What is said on the card, naming Nell.
const SAID: &str = "@nell the login fails on Safari";

/// The owner says something on a card that names Nell, who is told.
#[derive(Clone)]
struct Named;

impl Scenario for Named {
    const ACTS: usize = 1;

    async fn stage(&self) -> Play {
        let (rig, at) = node_with_stand_ins("api-tagging").await;
        rig.send("/api/tier", json!({ "tier": "premium" })).await;
        let nell = person(&rig, "Nell").await;
        shop_with_a_board(&rig, &[]).await;
        let card = rig.send("/api/board/shop/items", json!({ "title": "Fix the login" })).await;
        let mut play = Play::on(rig, &at).await;
        play.notes.insert("card", card["id"].as_str().unwrap().to_string());
        play.notes.insert("nell", nell.token);
        play.notes.insert("actor", nell.actor);
        play
    }

    /// Looking first, as a person does: what the daemon kept of a comment
    /// it went away in the middle of is in the card's history, and is not
    /// said again.
    async fn act(&self, _: usize, play: &mut Play) {
        let card = play.note("card").to_string();
        if said(play, &card).await.is_empty() {
            play.rig.send(&format!("/api/records/{card}/comments"), json!({ "body": SAID })).await;
        }
    }

    fn heard(&self, _: usize, play: &Play, event: &Event) -> bool {
        event.topic == "inbox.told"
            && event.data["owner"] == play.note("actor")
            && event.data["kind"] == "commented"
    }

    async fn came_to(&self, play: &Play) -> Came {
        let card = play.note("card").to_string();
        let (_, found) =
            play.rig.call(Method::GET, "/api/inbox?all=true", Some(play.note("nell")), None).await;
        let lines = found["lines"].as_array().into_iter().flatten();
        let boxed: Value = lines
            .filter(|line| line["kind"] == "commented")
            .map(|line| json!([line["place"], line["count"], line["on_record"]["body"], line["read"]]))
            .collect();
        Came::default().with("said", json!(said(play, &card).await)).with("boxed", boxed)
    }
}

/// The words of each thing said on the card, as its history keeps them.
async fn said(play: &Play, card: &str) -> Vec<Value> {
    let history = play.rig.read(&format!("/api/records/{card}/history")).await;
    let lines = history["history"].as_array().into_iter().flatten();
    lines
        .filter(|line| line["topic"] == "record.commented")
        .map(|line| line["data"]["body"].clone())
        .collect()
}

/// As the comment is said, as its line of history is written, and as
/// whoever it names is told.
fn in_the_telling(place: &Place) -> bool {
    matches!(place.topic.as_str(), "record.commented" | "collection.history" | "inbox.told")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn somebody_named_on_a_card_is_told_once_wherever_the_daemon_is_stopped_in_the_telling() {
    let found =
        simulate("somebody named on a card", Named, Grain::asked_or(Grain::Where(in_the_telling))).await;
    found.holds();
}
