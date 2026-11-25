//! A card's A/B test, stopped anywhere and started again.
//!
//! The scenario is a card worked two ways at once and judged
//! (`std/contest`, laid into where the card is worked by `std/ab`): two
//! contenders, each a role of the fleet, each working the card in a place
//! of its own; one judge picking one by its letter; the winner's work made
//! the card's, reviewed and merged as any card's; the loser's kept as its
//! branch. Everything a test does keeps nothing as it goes and is the same
//! done twice, so a daemon stopped anywhere in it has the round begun
//! again, which finds what was done done: each contender's place, its
//! agent's turn by its asking, what it committed, the judge's turn, the
//! verdicts, the winner's place taken.
//!
//! And as a shadow test: the same two, one of them the incumbent whose
//! work is the card's whatever is judged, the judge taking the other, so
//! that what is kept is not what the judge took.

use std::path::PathBuf;

use kernel::Event;
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets;
use crate::found::Grain;
use crate::simulated::{card_called, shop_came_to, shop_with_a_board, tidied};
use crate::simulation::{BEGAN, Came, Place, Play, Scenario, simulate};

/// Those who work in it: the two contenders, the judge and the reviewer.
const AGENTS: [&str; 4] = ["shop-otter-1", "shop-slate-1", "shop-judge-1", "shop-reviewer-1"];

/// A factory whose stand-ins say their program is there, wait to be let
/// go, do their work and say they have ended: one that writes, one that
/// approves (or, where `sends_back`, sends the work back the first time
/// it looks), and one that judges and picks `picks`.
async fn node(sends_back: bool, picks: &str) -> (Rig, PathBuf) {
    let mut at = None;
    let rig = Rig::with("api-contesting", |config, dir| {
        let file = |name: &str| dir.join(name).to_string_lossy().into_owned();
        let (began, hold, ended) = (file("began"), file("hold"), file("ended"));
        let stands_in = |who: &str, does: &[&str]| {
            let first = ["read", "tell", &began, who, "held", &hold, who];
            crate::standin::argv(&[&first[..], does, &["append", &ended, who][..]].concat())
        };
        let writes = ["write", "CHANGES-%:sum%.md", "changed", "say", "I changed a file."];
        config.adapters.push(("writer".into(), stands_in("writer", &writes)));
        let approves = ["say", "It does what the card asks.", "say", "DECISION: approve"];
        let looked = file("looked");
        let once = [
            &["there", &looked, "("][..],
            &approves[..],
            &["append", &ended, "critic", ")", "write", &looked, "once"][..],
            &["say", "The total is wrong.", "say", "DECISION: send_back"][..],
        ]
        .concat();
        config
            .adapters
            .push(("critic".into(), stands_in("critic", if sends_back { &once } else { &approves })));
        let decision = format!("DECISION: {picks}");
        let picks = ["say", "It does what the card asks.", "say", &decision];
        config.adapters.push(("judge".into(), stands_in("judge", &picks)));
        at = Some(dir.to_path_buf());
    })
    .await;
    (Rig::owned(rig).await, at.unwrap())
}

/// A card set up to be worked by two contenders and judged, then made
/// ready; the winner's work reviewed and merged.
#[derive(Clone)]
struct Contested {
    /// The winner's work is sent back from its review once.
    sends_back: bool,
    /// A shadow test whose incumbent is `otter`, judged by their ways: the judge takes `slate`.
    shadow: bool,
}

impl Scenario for Contested {
    const ACTS: usize = 2;

    async fn stage(&self) -> Play {
        let (rig, at) = node(self.sends_back, if self.shadow { "slate" } else { "A" }).await;
        let dir = shop_with_a_board(&rig, &[]).await;
        let agents = json!([
            { "title": "worker", "role": "worker", "count": 1, "adapter": "writer" },
            { "title": "otter", "role": "otter", "count": 1, "adapter": "writer" },
            { "title": "slate", "role": "slate", "count": 1, "adapter": "writer" },
            { "title": "reviewer", "role": "reviewer", "count": 1, "adapter": "critic", "permission": "read" },
            { "title": "judge", "role": "judge", "count": 1, "adapter": "judge", "permission": "read" },
        ]);
        fleets::fleet(&rig, json!({ "agents": agents })).await;
        let ab = json!({ "blueprint": "std/ab", "project": "shop", "params": { "project": "shop" }, "wait": true });
        let given = rig.send("/api/apply", ab).await;
        assert_eq!(given["finished"]["status"], "ok", "{given}");
        let mut play = Play::on(rig, &at).await;
        play.notes.insert("dir", dir.display().to_string());
        play
    }

    async fn act(&self, nth: usize, play: &mut Play) {
        if nth == 0 {
            // A card, and a test of it, each unless it is there.
            let id = match card_called(play, "Add a cart").await {
                Some(card) => card["id"].as_str().unwrap().to_string(),
                None => fleets::add(&play.rig, "Add a cart", &[]).await,
            };
            if tests(&play.rig, &id).await.is_empty() {
                let mut test = json!({ "card": id, "contenders": ["otter", "slate"], "judges": "judge" });
                if self.shadow {
                    test["shadow"] = json!("otter");
                    test["known_by"] = json!("ways");
                }
                let test = json!({ "fields": test });
                play.rig.send("/api/collections/shop/contests/records", test).await;
            }
            play.notes.insert("card", id);
        } else {
            let id = play.note("card").to_string();
            if fleets::card(&play.rig, &id).await["status"] == "draft" {
                crate::lanes::move_card(&play.rig, &id, "ready", None).await;
            }
        }
    }

    fn heard(&self, nth: usize, play: &Play, event: &Event) -> bool {
        match nth {
            // The test laid into the card's list, and the fleet has it.
            0 => event.topic == "fleet.arranged",
            _ => tidied(play) == 1,
        }
    }

    async fn came_to(&self, play: &Play) -> Came {
        // The fleet's worker is counted exactly: never asked, a card sent back after its test being the winner's.
        let mut came = shop_came_to(play, &[&AGENTS[..], &["shop-worker-1"][..]].concat()).await;
        // Two write at once: started twice is once for each, and begun
        // again after a stop once more for each that was at work.
        if let Some(started) = came.asked.get_mut("the program of the writer") {
            *started = started.div_ceil(2);
        }
        let id = play.note("card").to_string();
        let test = tests(&play.rig, &id).await;
        let row = &test[0]["data"];
        let said = |row: &Value, keys: &[&str]| -> Value {
            keys.iter()
                .map(|key| (key.to_string(), row[*key].clone()))
                .collect::<serde_json::Map<_, _>>()
                .into()
        };
        let all = play.rig.read("/api/collections/shop/entries/records").await;
        let mut entries: Vec<Value> = all["records"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|entry| {
                said(&entry["data"], &["letter", "way", "state", "verdict", "agent", "key", "incumbent"])
            })
            .collect();
        entries.sort_by_key(|entry| entry["letter"].to_string());
        let segments = play.rig.read("/api/collections/shop/segments/records").await;
        let laid: Vec<Value> = segments["records"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|row| said(&row["data"], &["does", "with", "order", "item"]))
            .collect();
        came.with("test", said(row, &["state", "winner", "agent"]))
            .with("entries", json!(entries))
            .with("laid", json!(laid))
    }
}

/// The card's tests.
async fn tests(rig: &Rig, card: &str) -> Vec<Value> {
    let all = rig.read("/api/collections/shop/contests/records").await;
    all["records"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|row| row["data"]["card"] == card)
        .cloned()
        .collect()
}

/// Where a contender or the judge is given its turn, its program is there,
/// or the test waits for what it started.
fn in_the_test(place: &Place) -> bool {
    let turn = ["agent.turn.queued", "agent.turn.started", "agent.turn.finished"];
    let ours = AGENTS[..3].contains(&place.who.as_str());
    (turn.contains(&place.topic.as_str()) && ours)
        || (place.topic == BEGAN && ["writer", "judge"].contains(&place.who.as_str()))
        || (place.topic == "step.waiting" && place.blueprint.starts_with("std/conten"))
        || (place.topic == "step.waiting" && place.blueprint.starts_with("std/judge"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_card_worked_two_ways_at_once_and_judged_is_merged_all_the_same_wherever_the_daemon_is_stopped() {
    let found = simulate(
        "a card's A/B test",
        Contested { sends_back: false, shadow: false },
        Grain::asked_or(Grain::Where(in_the_test)),
    )
    .await;
    found.holds();
    // Each contender and the judge as each is asked, begins, has its
    // program there and answers, and the test as it waits.
    assert!(found.tried.len() >= 12 || std::env::var("SIMULATE").is_ok(), "{}", found.tried.len());
}

/// Where the test comes to its winner and has the card's work handed over,
/// the card written down as the winner's in its comparison; and where that
/// comparison is ended once the card's review has passed.
fn handing_over(place: &Place) -> bool {
    let test =
        place.within("std/contended") && ["went", "row", "handed", "said"].contains(&place.step.as_str());
    test || place.within("std/ab") && ["on.handed", "on.accepted"].contains(&place.flow.as_str())
        || place.within("std/contest") && ["next", "handed", "again"].contains(&place.step.as_str())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_card_sent_back_after_its_test_is_the_winners_agents_wherever_the_daemon_was_stopped_as_it_was_handed_over()
 {
    let found = simulate(
        "a card's A/B test, sent back once",
        Contested { sends_back: true, shadow: false },
        Grain::asked_or(Grain::Where(handing_over)),
    )
    .await;
    found.holds();
    assert!(found.tried.len() >= 4 || std::env::var("SIMULATE").is_ok(), "{}", found.tried.len());
}

/// Where what is kept is decided and done: the judge's word taken in, the
/// verdicts written, the incumbent's place taken and the challenger's
/// kept, and the test saying what it came to.
fn kept_whatever_was_judged(place: &Place) -> bool {
    let steps = ["came", "verdicts", "take", "kept", "given", "row", "said"];
    (place.within("std/contended") && steps.contains(&place.step.as_str()))
        || (place.topic == "agent.turn.finished" && place.who == "shop-judge-1")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_shadow_tests_incumbent_is_merged_and_the_challenger_the_judge_took_kept_apart_wherever_the_daemon_is_stopped()
 {
    let found = simulate(
        "a card's shadow test",
        Contested { sends_back: false, shadow: true },
        Grain::asked_or(Grain::Where(kept_whatever_was_judged)),
    )
    .await;
    found.holds();
    assert!(found.tried.len() >= 6 || std::env::var("SIMULATE").is_ok(), "{}", found.tried.len());
}
