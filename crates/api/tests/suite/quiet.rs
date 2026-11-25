//! Which records of a step are held until its run next writes, for every
//! action there is: this program's, the language's own and what each
//! package that ships describes, installed here or not
//! (`conductor::holds`, `RunCx::hush` in `crates/conductor/src/run.rs`).
//! One that only decides has its beginning and its ending held: stopped
//! before they are written, it is done from its start as though it were
//! new, which only something that says, writes and reaches nothing may
//! be. One that is the same done twice has its ending alone held, and its
//! beginning written as every step's is: stopped before its ending is
//! written, it is done again told so, which is all being the same done
//! twice ever asked of it (`channel.post` looks for its message then, and
//! `emit` for its event), so the rule asks nothing more of any of them and
//! none of them is let out of it. The plays of an action that leaves
//! something and looks for it when told hold that it is left once
//! wherever the conductor is stopped (the runner's suite's `left.rs`).

use std::path::{Path, PathBuf};

use blueprint::ActionSpec;
use conductor::{Quiet, holds};

use crate::Rig;

fn top() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every action there is, each once.
async fn every() -> Vec<ActionSpec> {
    let rig = Rig::new().await;
    let mut actions = rig.node().app.engine.registry().all_specs();
    actions.extend(blueprint::intrinsics());
    let (packages, _) = agents::extension::load_all(&top().join("extensions"));
    for package in packages.values() {
        for action in package.actions.values() {
            if !actions.iter().any(|known| known.name == action.spec.name) {
                actions.push(action.spec.clone());
            }
        }
    }
    actions.sort_by(|one, other| one.name.cmp(&other.name));
    actions.dedup_by(|one, other| one.name == other.name);
    actions
}

#[tokio::test]
async fn a_step_has_its_ending_held_where_its_action_is_the_same_done_twice_and_its_beginning_too_only_where_it_decides()
 {
    let actions = every().await;
    let of = |quiet: Quiet| -> Vec<&str> {
        actions.iter().filter(|action| holds(action) == quiet).map(|action| action.name.as_str()).collect()
    };
    let (whole, ending, not) = (of(Quiet::Whole), of(Quiet::Ending), of(Quiet::Not));
    println!("beginning and ending held, {}: {}", whole.len(), whole.join(" "));
    println!("ending held, {}: {}", ending.len(), ending.join(" "));
    println!("written as each is, {}: {}", not.len(), not.join(" "));
    let mut wrong = Vec::new();
    for action in &actions {
        let meant = match (action.decides, action.idempotent) {
            (true, _) => Quiet::Whole,
            (false, true) => Quiet::Ending,
            (false, false) => Quiet::Not,
        };
        if holds(action) != meant {
            wrong.push(format!(
                "`{}` has {:?} held, where it says it is {meant:?}",
                action.name,
                holds(action)
            ));
        }
        // One whose beginning is held is done from its start as though it
        // were new: it may make nothing and run nothing, and has to be
        // safe to do again where it was not held (beside another at work).
        if action.decides && (action.makes.is_some() || !action.runs.is_empty() || !action.idempotent) {
            wrong.push(format!(
                "`{}` says it only decides, and makes something, runs a command or is not the same done twice",
                action.name
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    // Those that leave something and look for it when told they are being
    // done again are never held from their beginning: a stop after what
    // they leave would then find no record, and leave it a second time.
    for leaves in ["channel.post", "emit", "stop", "inbox.tell", "record.comment"] {
        if let Some(action) = actions.iter().find(|action| action.name == leaves) {
            assert_ne!(holds(action), Quiet::Whole, "`{leaves}` leaves something behind");
        }
    }
    assert!(!ending.is_empty() && !whole.is_empty());
}
