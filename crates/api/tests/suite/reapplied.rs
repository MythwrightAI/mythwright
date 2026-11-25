//! A fleet applied again in the moment a changed row of its tables is
//! being taken, with a card in hand and another in review.
//!
//! A changed row is taken by a handler of the fleet's, which calls the
//! arrangement and waits for it; the arrangement has every role, pool
//! and stage of the fleet under it. An apply over the fleet with
//! something else said has the fleet's steps run again, which cancels
//! that handler's run in its call. The arrangement was there before
//! that call and is not the call's to stop (`child::start` in the
//! runner), so nothing of the fleet is stopped: its steps, run again,
//! call the arrangement themselves, and the agents that were at work go
//! on with what they had.
//!
//! It was not always so. A call that was cut short stopped what it had
//! called, whoever had made it, and the arrangement went with every
//! role, pool and stage under it and each agent at its work. What came
//! of that was the machine's to say, and this play, crowded, came to
//! each: the fleet started anew with its cards given again; the apply
//! refused as cancelled, its own call having come to an arrangement
//! that was being stopped, and the fleet left with nothing under it;
//! the card in hand left in `doing` for good, its round having heard
//! that its agent was stopped before the stage that asked for the round
//! was; and the channel told that something in the fleet failed. So the
//! play is held against the same play with no apply in it, whole.
//!
//! The order is made by holding, not by load. The node reads a role
//! that is the one that ships with one step more, which waits where its
//! row says the rank 7 and says so once it is listening: a row given
//! that rank is being taken, by a handler in its call, for as long as
//! the play says nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use kernel::Source;
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::fleets::{add, params, row, shop};
use crate::lanes::{move_card, moved};
use crate::observed::{alive, asked, cards, repository, said};
use crate::{Listening, Rig};

/// The role as it ships.
const ROLE: &str = include_str!("../../../../blueprints/std/role.bp");
pub(crate) const WORKER: &str = "shop-worker-1";
pub(crate) const REVIEWER: &str = "shop-reviewer-1";

/// The role as it ships with one step more, beside its own: where its
/// row says the rank 7 it waits to be told to go on (`shop.go`), and
/// says that it waits once it is listening (`shop.held`). The role's
/// steps are called by the arrangement, and the arrangement by the
/// fleet's handler for a changed row.
fn held() -> String {
    let steps = "\nsteps:\n";
    let waits = "  held:\n    when: ${ params.rank == 7 }\n    do: wait\n    with:\n      event: shop.go\n      \
                 say: { topic: shop.held }\n\n";
    assert_eq!(ROLE.matches(steps).count(), 1, "the role that ships has its steps under one heading");
    ROLE.replacen(steps, &format!("{steps}{waits}"), 1)
}

/// A node that reads the role that can be held in place of the one it
/// ships, with the stand-ins of [`stand_ins`].
async fn rig() -> (Rig, PathBuf) {
    stand_ins("api-reapplied", Some(held())).await
}

/// A node whose stand-ins say where the play listens (`began`) that
/// their program is there, by the name of their agent, and then wait at
/// the hold called after it; reading `role` in place of the role it
/// ships, where one is given.
pub(crate) async fn stand_ins(label: &str, role: Option<String>) -> (Rig, PathBuf) {
    let mut at = None;
    let rig = Rig::with(label, |config, dir| {
        if let Some(role) = role {
            let std = dir.join("held").join("std");
            std::fs::create_dir_all(&std).unwrap();
            std::fs::write(std.join(brand::blueprint_file("role")), role).unwrap();
            config.blueprints_dir = Some(dir.join("held"));
        }
        let file = |name: &str| dir.join(name).to_string_lossy().into_owned();
        let name = format!("%{}%", brand::env_var("SLOT"));
        let (began, hold) = (file("began"), file(&format!("hold-{name}")));
        let stands_in = |does: &[&str]| {
            let waits = ["swallow", "tell", &began, &name, "held", &hold, &name];
            crate::standin::argv(&[&waits[..], does].concat())
        };
        let writes = stands_in(&["append", "CHANGES.md", "changed", "say", "I changed a file."]);
        let looks = stands_in(&["say", "It does what the card asks.", "say", "DECISION: approve"]);
        config.adapters.push(("writer".into(), writes));
        config.adapters.push(("critic".into(), looks));
        at = Some(dir.to_path_buf());
    })
    .await;
    (Rig::owned(rig).await, at.unwrap())
}

/// Lets the stand-in of `who` go on, which is waiting at its hold or on
/// its way there.
pub(crate) async fn let_go(at: &Path, who: &str) {
    crate::let_go("a stand-in to be waiting at its hold", &at.join(format!("hold-{who}"))).await;
}

/// Waits until the program of `who` has said it is there.
pub(crate) async fn began(there: &mut Listening, who: &str) {
    crate::soon(&format!("the program of {who} to be there"), async {
        while there.next_line().await.unwrap().as_deref() != Some(who) {}
    })
    .await;
}

/// Tells whatever role is waiting to go on.
async fn go_on(rig: &Rig) {
    rig.node().app.journal.emit("shop.go", Source::default(), json!({ "project": "shop" })).await.unwrap();
}

/// Lets each stand-in go as its program says it is there, until so many
/// cards have been done since `from` and their places tidied away. A
/// play that never comes to that says where its cards are and what was
/// said last.
pub(crate) async fn played_out(rig: &Rig, there: &mut Listening, at: &Path, from: u64, done: usize) {
    let mut ended = rig.follow_after(from, "run.finished");
    let tidied = async {
        let mut times = 0;
        while times < done {
            if ended.next().await.unwrap().data["flow"] == "on.tidied" {
                times += 1;
            }
        }
    };
    tokio::pin!(tidied);
    let out = async {
        loop {
            tokio::select! {
                () = &mut tidied => break,
                line = there.next_line() => let_go(at, &line.unwrap().unwrap()).await,
            }
        }
    };
    if tokio::time::timeout(crate::PATIENCE, out).await.is_err() {
        let board = cards(rig, "shop").await;
        panic!(
            "the cards were not all done: {board}\nwhat happened last, the latest first:\n{}",
            rig.lately().await
        );
    }
}

/// What a play came to.
#[derive(Debug, PartialEq)]
struct Came {
    /// Where each card is, what is on each branch, what the channel was
    /// told and what is alive.
    same: BTreeMap<&'static str, Value>,
    /// How often each agent was asked.
    asked: BTreeMap<String, u64>,
    /// What was stopped from the row being changed until the fleet had
    /// taken it, by blueprint, each as often as it was.
    stopped: BTreeMap<String, usize>,
    /// Whose turn was called off in the same while.
    called_off: BTreeSet<String>,
    /// What failed in it: the blueprint of each run that said so.
    failed: Vec<String>,
}

/// The play: a card in review with its reviewer at work and held, a
/// card in hand with its worker at work and held, a row of the fleet's
/// agents changed, and, while the handler that takes it is in its call
/// of the arrangement, the fleet applied again with something else said
/// (`applies`), or not. Both cards are then let run to their end.
async fn play(applies: bool) -> Came {
    let (rig, at) = rig().await;
    let mut there = Listening::at(&at.join("began"));
    for who in [WORKER, REVIEWER] {
        crate::hold(&at.join(format!("hold-{who}")));
    }
    let dir = shop(&rig, json!({})).await;

    // A card in review, its reviewer's program there and held.
    let first = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &first, "ready", None).await;
    began(&mut there, WORKER).await;
    let_go(&at, WORKER).await;
    moved(&rig, &first, "review", 0).await;
    began(&mut there, REVIEWER).await;
    // And a card in hand, its worker's program there and held.
    let second = add(&rig, "Add a till", &[]).await;
    move_card(&rig, &second, "ready", None).await;
    began(&mut there, WORKER).await;

    // A row is changed, and the handler that takes it is in its call of
    // the arrangement, which waits for the role.
    let changed = rig.mark();
    let id = row(&rig, "agents", "worker").await;
    let rank = json!({ "fields": { "rank": 7 } });
    let (status, answer) =
        rig.call(Method::PATCH, &format!("/api/records/{id}"), Some(&rig.token), Some(rank)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let waits = rig.event("shop.held", move |event| event.seq > changed).await.seq;

    if applies {
        // The fleet is applied again with something else said: its steps
        // run again, and the handler is cancelled in its call. Only then
        // is the role told to go on, and the apply is answered when the
        // fleet's steps have called the arrangement themselves.
        let other = params(json!({ "rounds": 3 }));
        let request = json!({ "blueprint": "std/fleet", "project": "shop", "params": other, "wait": true });
        let apply = rig.send("/api/apply", request);
        let cut = async {
            rig.event("run.finished", move |event| {
                event.seq > waits && event.data["flow"] == "on.changed" && event.data["status"] == "cancelled"
            })
            .await;
            go_on(&rig).await;
        };
        let (applied, ()) = tokio::join!(apply, cut);
        assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    } else {
        go_on(&rig).await;
        crate::taken::by_fleet(&rig, changed).await;
    }
    let arranged = rig.mark();
    // Nothing stopped either of them: they are where they were.
    for who in [WORKER, REVIEWER] {
        let_go(&at, who).await;
    }
    played_out(&rig, &mut there, &at, changed, 2).await;
    crate::simulation::rested(&rig).await;

    let during = |topics: &str| journal::EventQuery {
        topics: topics.into(),
        after: changed,
        before: arranged,
        limit: 10_000,
        ..Default::default()
    };
    let mut stopped = BTreeMap::new();
    for event in rig.said(during("instance.stopped")).await {
        *stopped.entry(event.data["blueprint"].as_str().unwrap().to_string()).or_insert(0) += 1;
    }
    let ended = rig.said(during("agent.turn.finished")).await;
    let called_off = ended
        .iter()
        .filter(|event| event.data["called_off"] == true)
        .map(|event| event.data["slot"].as_str().unwrap().to_string())
        .collect();
    let failed = rig.said(during("run.failed")).await;
    let same = BTreeMap::from([
        ("cards", cards(&rig, "shop").await),
        ("repository", repository(&dir)),
        ("said in the channel", said(&rig, "shop").await),
        ("alive", alive(&rig, Some("shop")).await),
    ]);
    Came {
        same,
        asked: asked(&rig, &[WORKER, REVIEWER]).await,
        stopped,
        called_off,
        failed: failed.iter().map(|event| event.data["blueprint"].as_str().unwrap().to_string()).collect(),
    }
}

#[tokio::test]
async fn a_fleet_applied_again_as_a_changed_row_is_taken_stops_nothing_and_its_cards_go_on_as_they_were() {
    let (applied, usual) = tokio::join!(play(true), play(false));

    // Nothing is stopped for the handler having been cut in its call, no
    // agent is stopped at its work, and nothing fails.
    assert_eq!(applied.stopped, BTreeMap::new());
    assert_eq!(applied.called_off, BTreeSet::new());
    assert_eq!(applied.failed, [] as [&str; 0]);
    // So each agent is asked once for each card, as with no apply.
    assert_eq!((applied.asked[WORKER], applied.asked[REVIEWER]), (2, 2));

    // And everything is as it is where the fleet was not applied over:
    // both cards done and merged, none sent back, the same alive, and
    // the channel told nothing more.
    assert_eq!(applied, usual);
    let done = json!({ "in": "done", "sent back": 0 });
    assert_eq!(usual.same["cards"], json!({ "shop-1": done, "shop-2": done }));
}
