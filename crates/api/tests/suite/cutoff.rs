//! A card whose agent's turn runs past the time its stage's row gives one
//! (`lasts`), stopped anywhere and started again.
//!
//! The scenario is a card worked in a shop whose stage that works says
//! how a card is finished there and that a turn may take twenty minutes.
//! The worker's program begins and goes on until it is stopped; the clock
//! is moved past its time while it is at work, its program is stopped,
//! and asked once what it had done it says the first step is done and the
//! second waits only on what is checked later. Wherever the daemon goes,
//! the card is reviewed, merged and done, and its agent is never asked to
//! work it again: a turn begun again after a stop is held to the time the
//! first was given.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use kernel::{Event, ManualClock};
use serde_json::json;

use crate::found::Grain;
use crate::simulated::{ready, shop_came_to, shop_with_a_board, tidied};
use crate::simulation::{Came, Place, Play, Scenario, simulate};
use crate::{Rig, fleets};

/// Past the twenty minutes the row says.
const PAST: Duration = Duration::from_secs(21 * 60);

/// The clock of each play's factory, by its project's folder.
fn clocks() -> &'static Mutex<HashMap<String, ManualClock>> {
    static CLOCKS: OnceLock<Mutex<HashMap<String, ManualClock>>> = OnceLock::new();
    CLOCKS.get_or_init(Default::default)
}

/// How many times the worker's program has begun to work: it says so, a
/// line each.
fn begun(at: &Path) -> usize {
    std::fs::read_to_string(at.join("began-work")).unwrap_or_default().lines().count()
}

/// Whether it was asked once what it did, its time having run out.
fn asked_after(at: &Path) -> bool {
    at.join("asked-after").exists()
}

/// Waits until it has begun more times than `seen`.
async fn begun_after(at: &Path, seen: usize) {
    while begun(at) <= seen {
        tokio::task::yield_now().await;
    }
}

/// A factory on a clock the play moves, whose stand-ins each say their
/// program is there and wait to be let go, as the simulation's do. The
/// worker, asked what it did once its time ran out, says so; asked to
/// work, it says it began and goes on until it is stopped.
async fn node() -> (Rig, PathBuf, ManualClock) {
    let clock = ManualClock::new();
    let moved = clock.clone();
    let mut at = None;
    let rig = Rig::with("api-cutoff", |config, dir| {
        config.clock = Some(Arc::new(moved));
        let file = |name: &str| dir.join(name).to_string_lossy().into_owned();
        let (began, hold, ended, work) = (file("began"), file("hold"), file("ended"), file("began-work"));
        let asked = file("asked-after");
        let first = |who: &'static str| ["tell", &began, who, "held", &hold, who].map(str::to_string);
        // Asked what it did once its time ran out, it says so first: that
        // is what the play waits for, while it waits to be let go.
        let mut writes: Vec<String> =
            ["read", "where", "ran out", "(", "append", &asked, "writer"].map(str::to_string).to_vec();
        writes.extend(first("writer"));
        let said = ["The cart is made.", "STEP 1: done", "STEP 2: later", "DECISION: done"];
        writes.extend(said.iter().flat_map(|line| ["say".to_string(), line.to_string()]));
        writes.extend(["append", &ended, "writer", ")"].map(str::to_string));
        writes.extend(first("writer"));
        writes.extend(["append", &work, "writer", "idle"].map(str::to_string));
        let writes: Vec<&str> = writes.iter().map(String::as_str).collect();
        config.adapters.push(("writer".into(), crate::standin::argv(&writes)));
        let mut looks = vec!["read".to_string()];
        looks.extend(first("critic"));
        looks.extend(
            ["say", "It does what the card asks.", "say", "DECISION: approve", "append", &ended, "critic"]
                .map(str::to_string),
        );
        let looks: Vec<&str> = looks.iter().map(String::as_str).collect();
        config.adapters.push(("critic".into(), crate::standin::argv(&looks)));
        at = Some(dir.to_path_buf());
    })
    .await;
    (Rig::owned(rig).await, at.unwrap(), clock)
}

#[derive(Clone)]
struct Cutoff;

impl Scenario for Cutoff {
    const ACTS: usize = 3;

    async fn stage(&self) -> Play {
        let (rig, at, clock) = node().await;
        let dir = shop_with_a_board(&rig, &[]).await;
        fleets::fleet(&rig, json!({ "workers": 1, "reviewers": 1 })).await;
        let row = json!({
            "finish": ["its tests pass", "the docs it changes say what is new"],
            "later": ["the whole gate, run when the work lands"],
            "lasts": "20m",
        });
        fleets::change(&rig, "stages", "work", row).await;
        let mut play = Play::on(rig, &at).await;
        play.notes.insert("dir", dir.display().to_string());
        play.notes.insert("at", at.display().to_string());
        clocks().lock().unwrap().insert(dir.display().to_string(), clock);
        play
    }

    async fn act(&self, nth: usize, play: &mut Play) {
        match nth {
            0 => {
                let id = ready(play, "Add a cart").await;
                play.notes.insert("card", id);
            }
            // Past its time, once its program is at work. One begun again
            // by a daemon that came back is held to the same time, and
            // the clock is moved on again each time another begins,
            // until its agent is asked what it did.
            1 => {
                let at = PathBuf::from(play.note("at"));
                let clock = clocks().lock().unwrap()[play.note("dir")].clone();
                // Done again after a stop, it moves the clock on only for a
                // program that began since it last did: moved again for one
                // it had moved it for, the ask after it would be cut too.
                let mut seen: usize = play.notes.get("seen").map_or(0, |seen| seen.parse().unwrap());
                loop {
                    let asked = async {
                        while !asked_after(&at) {
                            tokio::task::yield_now().await;
                        }
                    };
                    tokio::select! {
                        () = asked => break,
                        () = begun_after(&at, seen) => {}
                    }
                    seen = begun(&at);
                    clock.advance(PAST);
                    play.notes.insert("seen", seen.to_string());
                }
            }
            _ => {}
        }
    }

    fn heard(&self, nth: usize, play: &Play, _: &Event) -> bool {
        match nth {
            0 => begun(Path::new(play.note("at"))) > 0,
            1 => asked_after(Path::new(play.note("at"))),
            _ => tidied(play) == 1,
        }
    }

    async fn came_to(&self, play: &Play) -> Came {
        shop_came_to(play, &["shop-worker-1", "shop-reviewer-1"]).await
    }
}

/// Where the turn and what comes of its time are under way.
fn in_the_turn(place: &Place) -> bool {
    place.within("std/turn") || place.topic.starts_with("agent.turn")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_turn_that_runs_past_its_time_is_asked_once_what_it_did_and_its_card_goes_on_wherever_the_daemon_is_stopped()
 {
    let grain = Grain::asked_or(Grain::Where(in_the_turn));
    let found = simulate("a turn that runs past its time", Cutoff, grain).await;
    found.holds();
}
