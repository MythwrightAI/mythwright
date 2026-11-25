//! A card whose command, run before anybody looks at it, goes on past the
//! time its row says (`warn_after` of `std/run`), stopped
//! anywhere and started again.
//!
//! The scenario is a card worked in a shop whose `segments` runs a
//! command for it before its review, and says that the command may run
//! twenty minutes before somebody is told. The command waits to be let
//! go; the clock is moved on past its time while it waits, and then it is
//! let go and passes. Wherever the daemon goes, the card is reviewed,
//! merged and done, and whoever applied the fleet has one line of the
//! command's running long, told once: a command begun again after a stop
//! that runs past its time again tells nobody a second time.

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

/// How many times the command has begun: it says so, a line each.
fn begun(at: &Path) -> usize {
    std::fs::read_to_string(at.join("began-check")).unwrap_or_default().lines().count()
}

/// Waits until the command has begun more times than `seen`.
async fn begun_after(at: &Path, seen: usize) {
    while begun(at) <= seen {
        tokio::task::yield_now().await;
    }
}

/// A factory on a clock the play moves, whose stand-ins each say their
/// program is there and wait to be let go, as the simulation's do.
async fn node() -> (Rig, PathBuf, ManualClock) {
    let clock = ManualClock::new();
    let moved = clock.clone();
    let mut at = None;
    let rig = Rig::with("api-slowing", |config, dir| {
        config.clock = Some(Arc::new(moved));
        let file = |name: &str| dir.join(name).to_string_lossy().into_owned();
        let (began, hold, ended) = (file("began"), file("hold"), file("ended"));
        let stands_in = |who: &str, does: &[&str]| {
            let first = ["read", "tell", &began, who, "held", &hold, who];
            crate::standin::argv(&[&first[..], does, &["append", &ended, who][..]].concat())
        };
        let writes = ["write", "CHANGES-%:sum%.md", "changed", "say", "I changed a file."];
        config.adapters.push(("writer".into(), stands_in("writer", &writes)));
        let looks = ["say", "It does what the card asks.", "say", "DECISION: approve"];
        config.adapters.push(("critic".into(), stands_in("critic", &looks)));
        at = Some(dir.to_path_buf());
    })
    .await;
    (Rig::owned(rig).await, at.unwrap(), clock)
}

#[derive(Clone)]
struct Slowing;

impl Scenario for Slowing {
    const ACTS: usize = 3;

    async fn stage(&self) -> Play {
        let (rig, at, clock) = node().await;
        let dir = shop_with_a_board(&rig, &[]).await;
        fleets::fleet(&rig, json!({ "workers": 1, "reviewers": 1 })).await;
        rig.event("record.created", |event| event.data["collection"] == "tells").await;
        // The command says it began and waits to be let go, at a hold of
        // its own: the play lets go the stand-ins of agents by itself.
        crate::hold(&at.join("hold-check"));
        let file = |name: &str| at.join(name).to_string_lossy().into_owned();
        let command = crate::standin::line(&[
            "append",
            &file("began-check"),
            "check",
            "held",
            &file("hold-check"),
            "check",
        ]);
        let row = json!({
            "title": "Tests", "stage": "review", "order": 5, "does": "std/run", "command": command,
            "warn_after": "20m",
        });
        fleets::add_row(&rig, "segments", row).await;
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
            // Past its time, once a command has begun. A command begun by a
            // daemon on its way out may say so after the one begun again
            // when it is back, which then waits out its time from after
            // the clock was moved: so it is moved again each time another
            // begins, until the card is said to be slow.
            1 => {
                let at = PathBuf::from(play.note("at"));
                let clock = clocks().lock().unwrap()[play.note("dir")].clone();
                // Any said from here on: one may come of the clock moved for
                // a command that was not the last to say it began.
                let (mut seen, mark) = (0, play.rig.mark());
                loop {
                    crate::soon("a command to have begun", begun_after(&at, seen)).await;
                    seen = begun(&at);
                    clock.advance(PAST);
                    tokio::select! {
                        _ = play.rig.event("card.slow", move |event| event.seq > mark) => break,
                        () = begun_after(&at, seen) => {}
                    }
                }
            }
            _ => crate::pass(&Path::new(play.note("at")).join("hold-check"), 8),
        }
    }

    fn heard(&self, nth: usize, play: &Play, _: &Event) -> bool {
        match nth {
            0 => begun(Path::new(play.note("at"))) > 0,
            1 => {
                play.times(|place| {
                    place.topic == "run.finished" && place.within("std/told") && place.flow == "on.slow"
                }) > 0
            }
            _ => tidied(play) == 1,
        }
    }

    async fn came_to(&self, play: &Play) -> Came {
        let found = play.rig.read("/api/inbox?all=true").await;
        let lines = found["lines"].as_array().into_iter().flatten();
        let told: Vec<_> = lines.map(|line| json!([line["why"], line["about"], line["count"]])).collect();
        shop_came_to(play, &["shop-worker-1", "shop-reviewer-1"]).await.with("told", json!(told))
    }
}

/// Where the command's running long is under way: what runs it, what it
/// says of the card, and its telling.
fn in_the_telling(place: &Place) -> bool {
    place.within("std/run")
        || place.topic == "card.slow"
        || (place.within("std/told") && place.flow == "on.slow")
        || place.topic == "inbox.told"
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_command_that_runs_past_its_time_is_told_once_and_its_card_goes_on_wherever_the_daemon_is_stopped()
{
    let grain = Grain::asked_or(Grain::Where(in_the_telling));
    let found = simulate("a command that runs past its time", Slowing, grain).await;
    found.holds();
}
