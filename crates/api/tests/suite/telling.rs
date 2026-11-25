//! A card whose agent asks, with whoever applied the fleet told of it,
//! stopped anywhere in the telling and started again.
//!
//! The scenario is a card made ready in a shop whose one worker asks in
//! place of working. The fleet attaches what tells of what waits for a
//! person (`std/told`), which hears the question, reads who is told
//! what, asks whom that comes to and says the question on the card,
//! naming them, which is what puts a line in their inbox. What is new
//! to be stopped in is that run: as it begins, at each of its steps, as
//! the question is said, as the line is told, and as it ends. Wherever
//! the daemon goes, the card says its question once and whoever applied
//! the fleet has one line of it, told once.
//!
//! And a card its round leaves for a person: a command of the project's
//! that never passes sends it back as often as it may, and the run that
//! asked for the round says that the card is left (`card.left`), which
//! is heard and told the same way. Stopped as that is said, or anywhere
//! in the telling, the leaving is said once and told once.

use kernel::Event;
use serde_json::json;

use crate::fleets;
use crate::found::Grain;
use crate::observed::{node_whose_writer, node_with_stand_ins};
use crate::simulated::{ready, shop_came_to, shop_with_a_board};
use crate::simulation::{Came, Place, Play, Scenario, simulate};

/// A card is made ready, its agent asks, the question is put on the
/// card, and whoever applied the fleet is told.
#[derive(Clone)]
struct Asked;

impl Scenario for Asked {
    const ACTS: usize = 1;

    async fn stage(&self) -> Play {
        let asks = ["say", "Which total: with tax, or without?", "say", "DECISION: ask"];
        let (rig, at) = node_whose_writer("api-telling", &asks).await;
        let dir = shop_with_a_board(&rig, &[]).await;
        fleets::fleet(&rig, json!({ "workers": 1, "reviewers": 0 })).await;
        // Who is told what is there before the play begins: the table's
        // first row is added by what tells, beside the fleet's own steps.
        rig.event("record.created", |event| event.data["collection"] == "tells").await;
        let mut play = Play::on(rig, &at).await;
        play.notes.insert("dir", dir.display().to_string());
        play
    }

    async fn act(&self, _: usize, play: &mut Play) {
        let id = ready(play, "Add a cart").await;
        play.notes.insert("card", id);
    }

    fn heard(&self, _: usize, play: &Play, _: &Event) -> bool {
        // The question is on the card, and what tells has done with it.
        let over = |blueprint: &'static str| {
            play.times(|place| {
                place.topic == "run.finished" && place.within(blueprint) && place.flow == "on.asked"
            }) > 0
        };
        over("std/cards") && over("std/told")
    }

    async fn came_to(&self, play: &Play) -> Came {
        shop_came_to(play, &["shop-worker-1"]).await.with("told", told(play).await)
    }
}

/// Where a telling is under way: anything what tells says of the run
/// that hears the question, and the line being told.
fn in_the_telling(place: &Place) -> bool {
    place.within("std/told") || place.topic == "inbox.told"
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_card_that_asks_tells_whoever_applied_its_fleet_once_wherever_the_daemon_is_stopped_in_the_telling()
{
    let grain = Grain::asked_or(Grain::Where(in_the_telling));
    let found = simulate("a card that asks, with somebody told", Asked, grain).await;
    found.holds();
}

/// Every line of the owner's box: what it says, why, what opens it, how
/// often it was told and the words said on a record that it is of.
async fn told(play: &Play) -> serde_json::Value {
    let found = play.rig.read("/api/inbox?all=true").await;
    let lines = found["lines"].as_array().into_iter().flatten();
    lines
        .map(|line| {
            json!([line["says"], line["why"], line["place"], line["count"], line["on_record"]["body"]])
        })
        .collect()
}

/// A card is made ready and worked, a command of the project's does not
/// pass, the card is given again and the command does not pass again:
/// the card is left for a person, and whoever applied the fleet is told.
#[derive(Clone)]
struct Left;

impl Scenario for Left {
    const ACTS: usize = 1;

    async fn stage(&self) -> Play {
        let (rig, at) = node_with_stand_ins("api-telling-left").await;
        let dir = shop_with_a_board(&rig, &[]).await;
        fleets::fleet(&rig, json!({ "workers": 1, "reviewers": 0 })).await;
        let strict = json!({
            "title": "Strict", "stage": "work", "order": 15, "does": "std/run", "command": "exit 1", "rounds": 1,
        });
        fleets::add_row(&rig, "segments", strict).await;
        rig.event("record.created", |event| event.data["collection"] == "tells").await;
        let mut play = Play::on(rig, &at).await;
        play.notes.insert("dir", dir.display().to_string());
        play
    }

    async fn act(&self, _: usize, play: &mut Play) {
        let id = ready(play, "Add a cart").await;
        play.notes.insert("card", id);
    }

    fn heard(&self, _: usize, play: &Play, _: &Event) -> bool {
        // The leaving was said, what tells has done with it, and the
        // run that asked for the card's last round is over.
        let said = play.times(|place| place.topic == "card.left") > 0;
        let told = play.times(|place| {
            place.topic == "run.finished" && place.within("std/told") && place.flow == "on.left"
        }) > 0;
        let over = play.times(|place| {
            place.topic == "run.finished" && place.within("std/working") && place.flow == "on.go"
        }) > 1;
        said && told && over
    }

    async fn came_to(&self, play: &Play) -> Came {
        shop_came_to(play, &["shop-worker-1"]).await.with("told", told(play).await)
    }
}

/// Where a leaving is said and told: the step that says it, the saying,
/// anything what tells says of its run, and the line being told.
fn in_the_leaving(place: &Place) -> bool {
    in_the_telling(place)
        || place.topic == "card.left"
        || (place.within("std/working") && place.step == "left")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_card_its_round_leaves_is_said_to_be_left_and_told_once_wherever_the_daemon_is_stopped_in_that() {
    let grain = Grain::asked_or(Grain::Where(in_the_leaving));
    let found = simulate("a card its round leaves, with somebody told", Left, grain).await;
    found.holds();
}
