//! A round that is a list of things done with a card, stopped anywhere
//! and started again.
//!
//! The scenario is a card reviewed and merged (`simulated.rs`) in a
//! project that sends its cards' branches to a remote and whose
//! `segments` has one row: a command run where the card was worked,
//! between the work and the keeping of it. So the round is four things,
//! each a blueprint that keeps nothing: the work, the command, the
//! keeping, the sending. What is new to be stopped in is each of those
//! and the moments between them. A round begun again where the daemon
//! went begins at the first of its list and finds done what was done:
//! its agent is asked once, the command is run again, what is still
//! uncommitted is committed, the branch is sent again.
//!
//! The rest of a list is gone through a thing at a time by a text that
//! calls itself (`std/through`), so a long list is a deep one: the second
//! scenario's round is seven things, five of them the project's commands,
//! on a stage that does eight at most, stopped wherever its fifth level
//! is. Begun again, the list is called from its first under the same
//! names, and comes to what it comes to unstopped.

use kernel::Event;
use serde_json::json;

use crate::fleets;
use crate::found::Grain;
use crate::lanes::git;
use crate::observed::node_with_stand_ins;
use crate::simulated::{ready, shop_came_to, shop_with_a_board, tidied};
use crate::simulation::{Came, Place, Play, Scenario, simulate};

/// A card is worked, the project's command is run on its work, the work
/// is kept and sent, an agent reviews and approves it, and it is merged.
#[derive(Clone)]
struct Tidied;

impl Scenario for Tidied {
    const ACTS: usize = 1;

    async fn stage(&self) -> Play {
        let (rig, at) = node_with_stand_ins("api-rounded").await;
        let dir = shop_with_a_board(&rig, &[]).await;
        let remote = rig.dir.join("remote.git");
        git(&rig.dir, &["init", "--quiet", "--bare", remote.to_str().unwrap()]);
        git(&dir, &["remote", "add", "origin", remote.to_str().unwrap()]);
        fleets::fleet(&rig, json!({ "workers": 1, "reviewers": 1, "push": true })).await;
        // Whoever works leaves a file of changes in the card's place,
        // which is where the command is run: it passes there, and what it
        // leaves is the same however often it is run.
        let row = json!({
            "title": "Tidy", "stage": "work", "order": 15, "does": "std/run",
            "command": crate::standin::line(&["any", "CHANGES-", "write", "TIDIED.md", "tidied"]),
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
        shop_came_to(play, &["shop-worker-1", "shop-reviewer-1"]).await
    }
}

/// Where one thing of a round has ended and the next is beginning:
/// between the work and the command, the command and the keeping, the
/// keeping and the sending; and as the last of them ends.
fn between_the_things_of_a_round(place: &Place) -> bool {
    let begins = ["std/run", "std/keep", "std/send"].iter().any(|one| place.within(one));
    (begins && place.topic == "instance.started")
        || (place.within("std/send") && place.topic == "instance.finished")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_card_stopped_between_the_things_of_its_round_is_worked_once_tidied_kept_sent_and_merged_all_the_same()
 {
    let grain = Grain::asked_or(Grain::Where(between_the_things_of_a_round));
    let found = simulate("a card whose round is four things", Tidied, grain).await;
    found.holds();
}

/// A card whose round is seven things: the work, five commands of the
/// project's, each leaving a file of its own, and the keeping of it all;
/// then reviewed, approved and merged.
#[derive(Clone)]
struct Deep;

impl Scenario for Deep {
    const ACTS: usize = 1;

    async fn stage(&self) -> Play {
        let (rig, at) = node_with_stand_ins("api-rounded-deep").await;
        let dir = shop_with_a_board(&rig, &[]).await;
        fleets::fleet(&rig, json!({ "workers": 1, "reviewers": 1 })).await;
        // More than a round does at most where nothing is said.
        fleets::change(&rig, "stages", "work", json!({ "most": 8 })).await;
        for nth in 1..=5 {
            let file = format!("TIDIED-{nth}.md");
            let row = json!({
                "title": format!("Tidy {nth}"), "stage": "work", "order": 10 + nth, "does": "std/run",
                "command": crate::standin::line(&["any", "CHANGES-", "write", &file, "tidied"]),
            });
            fleets::add_row(&rig, "segments", row).await;
        }
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
        shop_came_to(play, &["shop-worker-1", "shop-reviewer-1"]).await
    }
}

/// Wherever the fifth level of the list, or the thing it calls (the sixth
/// of the round, the fifth command), says something.
fn at_the_fifth_level(place: &Place) -> bool {
    (place.within("std/through") || place.within("std/run")) && place.nth == 5
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_card_stopped_five_deep_in_a_round_of_seven_things_is_worked_once_tidied_kept_and_merged_all_the_same()
 {
    let grain = Grain::asked_or(Grain::Where(at_the_fifth_level));
    let found = simulate("a card whose round is seven things", Deep, grain).await;
    found.holds();
}
