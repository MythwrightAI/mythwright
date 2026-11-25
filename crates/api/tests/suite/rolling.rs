//! A gradual rollout stopped anywhere and started again.
//!
//! Two cards worked under a comparison that shares the work out between
//! the usual worker and a new setup (an agent of a role of its own), with
//! a rule that gives the new setup more of the cards once each has had one
//! card reviewed and its was sent back no more often. Both pass, so the
//! shares are said again once, by the rule. Stopped as a review is said
//! to have passed (what the rule reads), or as the shares are said
//! again, it comes to the same: what is counted is
//! read once for each thing however often whoever counts reads it again
//! after a stop, and the shares, what was read and the saying of it are
//! one write, so they are said again once and never twice.

use kernel::Event;
use serde_json::{Value, json};

use crate::fleets;
use crate::found::Grain;
use crate::observed::node_with_stand_ins;
use crate::simulated::{ready, shop_came_to, shop_with_a_board, tidied};
use crate::simulation::{Came, Place, Play, Scenario, simulate};

/// Two cards, one each way, worked and passed under a rule that grows the
/// new setup's share once both have been reviewed.
#[derive(Clone)]
struct RolledOut;

impl Scenario for RolledOut {
    const ACTS: usize = 2;

    async fn stage(&self) -> Play {
        let (rig, at) = node_with_stand_ins("api-rolling").await;
        let dir = shop_with_a_board(&rig, &[]).await;
        let agents = json!([
            { "title": "worker", "role": "worker", "count": 1, "adapter": "writer" },
            { "title": "slate", "role": "slate", "count": 1, "adapter": "writer" },
            { "title": "reviewer", "role": "reviewer", "count": 1, "adapter": "critic", "permission": "read" },
        ]);
        fleets::fleet(&rig, json!({ "agents": agents })).await;
        let asked = json!({
            "name": "rollout", "project": "shop", "of": "std/work", "per": "item", "counts": "cards",
            "ways": [{ "name": "usual" }, { "name": "slate", "with": { "agent": "shop-slate-1" } }],
            "rule": {
                "grows": "slate", "against": "usual", "count": "verdict", "came": "failed",
                "over": 1, "step": 25, "most": 75,
            },
        });
        rig.send("/api/comparisons", asked).await;
        let mut play = Play::on(rig, &at).await;
        play.notes.insert("dir", dir.display().to_string());
        play
    }

    /// Each card is made ready unless it is, as a person does who looks first.
    async fn act(&self, nth: usize, play: &mut Play) {
        let title = ["Add a cart", "Add a till"][nth];
        ready(play, title).await;
    }

    fn heard(&self, nth: usize, play: &Play, _event: &Event) -> bool {
        tidied(play) == nth + 1
    }

    async fn came_to(&self, play: &Play) -> Came {
        let came = shop_came_to(play, &["shop-worker-1", "shop-slate-1", "shop-reviewer-1"]).await;
        // What is counted is read beside the work, not by a run: what it
        // decides is waited for a while, and what there is then is what
        // the play came to. It decides once both cards have been reviewed,
        // and only then is there anything to wait for. A play is looked
        // at whenever it is quiet once the daemon was started again, a
        // card still being worked: waited for then, each look held the
        // play twenty seconds, and two such looks were all the time a
        // play has (`rules_are_waited_for_only_once_what_they_read_is_there`).
        if decided(play) {
            let read = async {
                loop {
                    let shown = play.rig.read("/api/comparisons/rollout").await;
                    if !shown["comparison"]["shared"].as_array().is_none_or(Vec::is_empty) {
                        return;
                    }
                    tokio::task::yield_now().await;
                }
            };
            let _ = tokio::time::timeout(std::time::Duration::from_secs(20), read).await;
        }
        let one = play.rig.read("/api/comparisons/rollout").await["comparison"].clone();
        let said = |shared: &Value| json!([shared["by"], shared["was"], shared["now"]]);
        let ways: Vec<Value> = one["ways"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|way| json!([way["name"], way["share"], way["went"]]))
            .collect();
        came.with("ways", json!(ways))
            .with(
                "shares said again",
                json!(one["shared"].as_array().into_iter().flatten().map(said).collect::<Vec<_>>()),
            )
            .with("rule done", one["rolling"]["done"].clone())
    }
}

/// Whether what the rule reads is all there: both cards reviewed.
fn decided(play: &Play) -> bool {
    play.times(|place| place.topic == "review.passed") >= RolledOut::ACTS
}

/// Where what the rule reads is said, and where the shares are said again.
fn read_or_said(place: &Place) -> bool {
    ["review.passed", "system.comparison.shared"].contains(&place.topic.as_str())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_rollouts_shares_are_said_again_once_by_its_rule_wherever_the_daemon_is_stopped() {
    let found = simulate("a gradual rollout", RolledOut, Grain::asked_or(Grain::Where(read_or_said))).await;
    found.holds();
    // Played through, the rule said the shares again: what each stopped play is held to.
    assert!(
        found.passed.iter().any(|place| place.topic == "system.comparison.shared"),
        "{:?}",
        found.passed.len()
    );
    assert!(found.tried.len() >= 3 || std::env::var("SIMULATE").is_ok(), "{}", found.tried.len());
}

/// The order the scenario failed by now and then, made: looked at while
/// a card is still to be worked, what a play came to is read at once. The
/// shares the rule says again are not there to be waited for until both
/// cards have been reviewed, and a play is looked at at every quiet
/// moment once the daemon was started again; waited for there, each look
/// took twenty seconds, two of them used up the forty a play has, and the
/// play was given up on with a stand-in's program waiting to be heard
/// (2 of 16 crowded, and more the busier the machine). Bounded at a
/// quarter of what such a look took, which reading the factory never nears.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rules_are_waited_for_only_once_what_they_read_is_there() {
    let mut play = RolledOut.stage().await;
    RolledOut.act(0, &mut play).await;
    let looked = tokio::time::timeout(std::time::Duration::from_secs(5), RolledOut.came_to(&play)).await;
    let came = looked.expect("what a play came to, read with a card still to be worked, is read at once");
    assert_eq!(came.same.get("shares said again"), Some(&json!([])), "{:?}", came.same);
}
