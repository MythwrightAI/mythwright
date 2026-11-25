//! A card that waits on somebody, climbing its project's chain, stopped
//! anywhere in the climb and started again (`std/chain`).
//!
//! The scenario is a card worked in a shop where nobody reviews: it
//! comes to review and waits for a person. The chain has two rungs:
//! whoever applied the fleet is told at once, and after four hours an
//! agent of the role `lead` looks at the card, and sends it back to be
//! worked. Worked again, it has been sent back as often as its stage
//! allows (once), and is left for a person. Wherever the daemon goes,
//! the card ends left in review, and whoever applied the fleet has one
//! line of its waiting, told twice (by `tells` and by the first rung, and
//! not again however often the climb is taken up), and one of its being
//! left.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use kernel::{Clock, Event, ManualClock};
use serde_json::json;

use crate::found::Grain;
use crate::simulated::{ready, shop_came_to, shop_with_a_board};
use crate::simulation::{Came, Place, Play, Scenario, simulate};
use crate::{Rig, fleets};

/// How long the first rung says the card may wait at it.
const FOUR_HOURS: Duration = Duration::from_secs(4 * 3600);

/// The clock of each play's factory, by its project's folder: plays are made
/// several at a time, each with a clock of its own.
fn clocks() -> &'static Mutex<HashMap<String, ManualClock>> {
    static CLOCKS: OnceLock<Mutex<HashMap<String, ManualClock>>> = OnceLock::new();
    CLOCKS.get_or_init(Default::default)
}

/// A factory on a clock the play moves, whose stand-ins each say their
/// program is there and wait to be let go, as the simulation's do: the
/// worker changes a file, the reviewer approves, and the lead sends a card
/// back.
async fn node() -> (Rig, PathBuf, ManualClock) {
    let clock = ManualClock::new();
    let moved = clock.clone();
    let mut at = None;
    let rig = Rig::with("api-climbed", |config, dir| {
        config.clock = Some(Arc::new(moved));
        let file = |name: &str| dir.join(name).to_string_lossy().into_owned();
        let (began, hold, ended) = (file("began"), file("hold"), file("ended"));
        let stands_in = |who: &str, does: &[&str]| {
            let first = ["read", "tell", &began, who, "held", &hold, who];
            crate::standin::argv(&[&first[..], does, &["append", &ended, who][..]].concat())
        };
        let writes = ["write", "CHANGES-%:sum%.md", "changed", "say", "I changed a file."];
        config.adapters.push(("writer".into(), stands_in("writer", &writes)));
        config.adapters.push(("critic".into(), stands_in("critic", &["say", "DECISION: approve"])));
        let back = ["say", "It cannot be checked here without leave.", "say", "DECISION: back"];
        config.adapters.push(("looker".into(), stands_in("looker", &back)));
        at = Some(dir.to_path_buf());
    })
    .await;
    (Rig::owned(rig).await, at.unwrap(), clock)
}

/// A card is made ready, worked and comes to wait in review; whoever
/// applied the fleet is told; four hours on the lead sends it back; and,
/// worked again, it is left for a person.
#[derive(Clone)]
struct Climbed;

impl Scenario for Climbed {
    const ACTS: usize = 2;

    async fn stage(&self) -> Play {
        let (rig, at, clock) = node().await;
        let dir = shop_with_a_board(&rig, &[]).await;
        let agents = json!([
            { "title": "worker", "role": "worker", "count": 1, "adapter": "writer" },
            { "title": "reviewer", "role": "reviewer", "count": 0, "adapter": "critic", "permission": "read" },
            { "title": "lead", "role": "lead", "count": 1, "adapter": "looker", "permission": "read" },
        ]);
        fleets::fleet(&rig, json!({ "agents": agents, "rounds": 1 })).await;
        rig.event("record.created", |event| event.data["collection"] == "tells").await;
        for rung in [
            json!({ "title": "owner", "waits": ["review"], "step": 1, "who": "applier", "after": "4h" }),
            json!({ "title": "lead", "waits": ["review"], "step": 2, "who": "agent:shop-lead-1", "does": "look" }),
        ] {
            let before = rig.mark();
            rig.send("/api/collections/shop/chain/records", json!({ "fields": rung })).await;
            rig.event("run.finished", move |event| {
                event.seq > before
                    && event.data["blueprint"] == "std/chain"
                    && event.data["flow"] == "on.changed"
            })
            .await;
        }
        let mut play = Play::on(rig, &at).await;
        play.notes.insert("dir", dir.display().to_string());
        clocks().lock().unwrap().insert(dir.display().to_string(), clock);
        play
    }

    async fn act(&self, nth: usize, play: &mut Play) {
        match nth {
            0 => {
                let id = ready(play, "Add a lamp").await;
                play.notes.insert("card", id);
            }
            // Four hours on, once the rung's time is being waited out: that
            // it is, is said and not always kept (`step.waiting`), so it is
            // waited for here and is no place. Done again after a stop, it
            // finds what was said before the stop, and the wait taken up
            // wakes at the time it kept, which has gone by.
            _ => {
                let clock = clocks().lock().unwrap()[play.note("dir")].clone();
                let until = clock.now_ms() + FOUR_HOURS.as_millis() as i64;
                play.rig
                    .event("step.waiting", move |event| event.data["until"].as_i64() == Some(until))
                    .await;
                clock.advance(FOUR_HOURS);
            }
        }
    }

    fn heard(&self, nth: usize, play: &Play, _: &Event) -> bool {
        let over = |blueprint: &'static str, flow: &'static str| {
            play.times(|place| place.topic == "run.finished" && place.within(blueprint) && place.flow == flow)
                > 0
        };
        match nth {
            // The first rung was told, and what waits out its time is there.
            0 => {
                over("std/told", "on.waited")
                    && play.times(|place| place.topic == "instance.started" && place.within("std/later")) > 0
            }
            // The lead sent it back, it was worked again, and it is left.
            _ => {
                over("std/chain", "on.due")
                    && play.times(|place| place.topic == "card.left") > 0
                    && over("std/told", "on.left")
            }
        }
    }

    async fn came_to(&self, play: &Play) -> Came {
        let card = fleets::card(&play.rig, play.note("card")).await;
        shop_came_to(play, &["shop-worker-1", "shop-lead-1"])
            .await
            .with("told", told(play).await)
            .with("where the card is on the chain", card["more"]["rung"].clone())
    }
}

/// Every line of the owner's box: what it is there for, what it is about
/// and how often it was told.
async fn told(play: &Play) -> serde_json::Value {
    let found = play.rig.read("/api/inbox?all=true").await;
    let lines = found["lines"].as_array().into_iter().flatten();
    lines.map(|line| json!([line["why"], line["about"], line["count"]])).collect()
}

/// Where the climb is under way: anything the chain, what it calls and
/// what waits out its time says, and a line being told.
fn in_the_climb(place: &Place) -> bool {
    place.within("std/chain")
        || place.within("std/chain-wait")
        || place.within("std/later")
        || place.topic == "inbox.told"
        || place.topic == "later.over"
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_card_that_climbs_its_chain_comes_to_the_same_and_each_rung_is_told_once_wherever_the_daemon_is_stopped()
 {
    let grain = Grain::asked_or(Grain::Where(in_the_climb));
    let found = simulate("a card that climbs its chain", Climbed, grain).await;
    found.holds();
}
