//! Two cards where the stage that works them takes the next only once the
//! last is merged (`next_after: merged`), stopped anywhere about the
//! second waiting behind the first and started again.
//!
//! The first is made ready and taken; the second is made ready while the
//! first is in hand, and waits behind it, asking for no place; the first
//! is reviewed, merged and done, and the second is then due again, worked
//! from what the first left, reviewed and merged. Stopped as the second
//! says it waits, as its turn is weighed against the first, as the stage
//! looks at its column again when the first goes on, or as any program is
//! first there, it comes to the same: each card's branch cut from the
//! project's branch with the other's work in it where it came after it,
//! and each merged.

use std::path::{Path, PathBuf};

use kernel::Event;
use serde_json::{Value, json};

use crate::found::Grain;
use crate::lanes::asking;
use crate::simulated::{moved, ready, shop, shop_came_to, tidied};
use crate::simulation::{BEGAN, Came, Place, Play, Scenario, simulate};

#[derive(Clone)]
struct MergedFirst;

impl Scenario for MergedFirst {
    const ACTS: usize = 2;

    async fn stage(&self) -> Play {
        shop(1, 1, &[], &[("stages", "work", json!({ "next_after": "merged" }))]).await
    }

    /// The second is made ready once the first is in hand, so that it is
    /// the second that waits.
    async fn act(&self, nth: usize, play: &mut Play) {
        if nth == 0 {
            let first = ready(play, "Lay the floor").await;
            play.notes.insert("first", first);
            return;
        }
        let second = ready(play, "Put the shelves up").await;
        play.notes.insert("second", second);
    }

    fn heard(&self, nth: usize, play: &Play, event: &Event) -> bool {
        match nth {
            0 => moved(event, play.note("first"), "doing"),
            _ => tidied(play) == 2,
        }
    }

    async fn came_to(&self, play: &Play) -> Came {
        // Each card was cut from the project's branch with the one before it
        // in it, which the play that is never stopped comes to (`ordered.rs`
        // holds that much without stopping).
        let cut = cut(&PathBuf::from(play.note("dir")));
        shop_came_to(play, &["shop-worker-1", "shop-reviewer-1"]).await.with("cut from the last", cut)
    }
}

/// For each merge on the project's branch, whether the branch it took in
/// was cut from what it was merged onto: so the work of the card merged
/// before it was there for whoever worked it.
fn cut(dir: &Path) -> Value {
    let git = |args: &[&str]| {
        let ran = asking(dir).args(args).output().unwrap();
        (ran.status.success(), String::from_utf8_lossy(&ran.stdout).trim().to_string())
    };
    let merges = git(&["log", "--merges", "--format=%P"]).1;
    let each: Vec<bool> = merges
        .lines()
        .map(|parents| {
            let parents: Vec<&str> = parents.split(' ').collect();
            git(&["merge-base", "--is-ancestor", parents[0], parents[1]]).0
        })
        .collect();
    json!(each)
}

/// Where the second has said it waits, where its turn has been weighed
/// against the card ahead of it as it was given a place, where the stage
/// begins to look at its column again as the first goes on and has looked,
/// where the second is found free, and where each program is first there.
/// Fewer than every place about the wait: each play is two cards worked
/// and merged one after the other, and as many plays side by side as the
/// wait has places did not all come to rest in time on a crowded machine.
fn about_the_wait(place: &Place) -> bool {
    let working = place.within("std/working") && place.topic != "step.started";
    let said =
        working && place.flow == "on.begun" && place.step == "behind" && place.topic == "step.finished";
    let freed = working && place.flow == "on.begun" && place.step == "ahead" && place.nth == 3;
    let weighed = working && place.flow == "on.go" && place.step == "ahead";
    let looked = working && place.flow == "on.unmerged" && place.nth == 1 && place.topic != "run.finished";
    said || freed || weighed || looked || (place.topic == BEGAN && place.nth == 1)
}

#[tokio::test]
async fn two_cards_where_the_next_is_taken_once_the_last_is_merged_are_worked_one_from_the_other_wherever_the_daemon_is_stopped()
 {
    let grain = Grain::asked_or(Grain::Where(about_the_wait));
    let found = simulate("a card waiting behind one not merged", MergedFirst, grain).await;
    found.holds();
    assert!(found.tried.len() >= 6, "only {} places were played", found.tried.len());
}
