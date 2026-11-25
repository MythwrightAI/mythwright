//! Where a card's branch comes from: cut in the project's folder as it
//! is, or, where the fleet says `fetch`, from what the repository's remote
//! has. Nothing is asked of a remote that was not said.

use std::path::{Path, PathBuf};

use serde_json::json;

use crate::Rig;
use crate::lanes::{git, rig, shop, worked};

/// Someone else's copy of the repository, beside the remote, on the
/// project's branch: what they commit and send is on the remote and not
/// in the project's folder.
fn elsewhere(rig: &Rig, remote: &Path, branch: &str) -> PathBuf {
    let other = rig.dir.join("elsewhere");
    git(&rig.dir, &["clone", "--quiet", remote.to_str().unwrap(), other.to_str().unwrap()]);
    git(&other, &["checkout", "--quiet", branch]);
    git(&other, &["config", "user.name", "Grace"]);
    git(&other, &["config", "user.email", "grace@example.com"]);
    other
}

/// A file committed there, on the branch that copy has checked out.
fn commit(dir: &Path, file: &str) {
    std::fs::write(dir.join(file), "something\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "--quiet", "-m", file]);
}

/// The project's branch sent to the remote, and what it is called.
fn sent(dir: &Path) -> String {
    let branch = git(dir, &["rev-parse", "--abbrev-ref", "HEAD"]);
    git(dir, &["push", "--quiet", "origin", &branch]);
    branch
}

#[tokio::test]
async fn a_lane_told_to_fetch_cuts_a_new_branch_from_what_the_remote_has() {
    let rig = rig().await;
    let (dir, remote) = shop(&rig, json!({ "fetch": true })).await;
    let branch = sent(&dir);
    // Someone adds to the project's branch on the remote. The folder here has not heard.
    let other = elsewhere(&rig, &remote, &branch);
    commit(&other, "NEWS.md");
    git(&other, &["push", "--quiet", "origin", &branch]);
    assert!(!dir.join("NEWS.md").exists());

    let id = worked(&rig, "Add a cart").await;
    // The card was worked from what the remote has, and the folder is as it was.
    let lane = dir.join(".lanes").join(&id);
    assert!(lane.join("NEWS.md").exists() && lane.join("CHANGES.md").exists());
    assert!(!dir.join("NEWS.md").exists(), "the project's own folder is not brought up to date by a lane");
    assert_eq!(git(&lane, &["log", "-2", "--format=%s"]), format!("{id}: Add a cart\nNEWS.md"));
}

#[tokio::test]
async fn a_lane_told_to_fetch_takes_up_the_branch_the_remote_already_has_for_its_card() {
    let rig = rig().await;
    let (dir, remote) = shop(&rig, json!({ "fetch": true, "push": true })).await;
    let branch = sent(&dir);
    // The card's branch was begun somewhere else, and is on the remote.
    let other = elsewhere(&rig, &remote, &branch);
    git(&other, &["checkout", "--quiet", "-b", "lane/shop-1"]);
    commit(&other, "BEGUN.md");
    git(&other, &["push", "--quiet", "origin", "lane/shop-1"]);

    let id = worked(&rig, "Add a cart").await;
    assert_eq!(id, "shop-1");
    // It was pulled down and gone on with, and what was added is sent back to it.
    let lane = dir.join(".lanes").join(&id);
    assert!(lane.join("BEGUN.md").exists() && lane.join("CHANGES.md").exists());
    assert_eq!(git(&lane, &["log", "-2", "--format=%s"]), "shop-1: Add a cart\nBEGUN.md");
    rig.event("card.pushed", |event| event.data["card"] == "shop-1").await;
    assert_eq!(git(&remote, &["log", "-2", "--format=%s", "lane/shop-1"]), "shop-1: Add a cart\nBEGUN.md");
}

#[tokio::test]
async fn where_the_folder_has_work_the_remote_has_not_a_new_branch_is_cut_from_the_folder() {
    let rig = rig().await;
    let (dir, _) = shop(&rig, json!({ "fetch": true })).await;
    sent(&dir);
    // Something is committed here and sent nowhere: a card that was merged, say.
    commit(&dir, "HERE.md");

    let id = worked(&rig, "Add a cart").await;
    let lane = dir.join(".lanes").join(&id);
    assert!(lane.join("HERE.md").exists(), "the card would have begun without what is here");
}

#[tokio::test]
async fn a_lane_that_was_not_told_to_fetch_asks_the_remote_nothing() {
    let rig = rig().await;
    let (dir, remote) = shop(&rig, json!({})).await;
    let branch = sent(&dir);
    let other = elsewhere(&rig, &remote, &branch);
    commit(&other, "NEWS.md");
    git(&other, &["push", "--quiet", "origin", &branch]);

    let id = worked(&rig, "Add a cart").await;
    // Cut from the folder as it is; and of the remote the repository knows
    // only what it was sent from here.
    assert!(!dir.join(".lanes").join(&id).join("NEWS.md").exists());
    let known = git(&dir, &["rev-parse", &format!("refs/remotes/origin/{branch}")]);
    assert_eq!(known, git(&dir, &["rev-parse", "HEAD"]), "nothing was fetched");
}

#[tokio::test]
async fn a_remote_that_cannot_be_asked_is_said_and_the_card_is_begun_from_what_is_here() {
    let rig = rig().await;
    let (dir, _) = shop(&rig, json!({ "fetch": true })).await;
    git(&dir, &["remote", "set-url", "origin", rig.dir.join("nowhere.git").to_str().unwrap()]);

    let id = worked(&rig, "Add a cart").await;
    assert!(dir.join(".lanes").join(&id).join("CHANGES.md").exists());
    let told = crate::lanes::said(&rig).await;
    assert!(told.contains(&format!("{id}: origin could not be asked what it has")), "{told}");
    assert!(told.contains("the card is begun from what is here"), "{told}");
}

#[tokio::test]
async fn a_round_handed_a_place_by_a_lane_that_made_it_itself_uses_that_place_and_makes_none() {
    // A lane opened before rounds made the card's place hands its own to
    // every round, and goes on working where it was.
    let rig = rig().await;
    let (dir, _) = shop(&rig, json!({})).await;
    // A card its lane has taken already: no other lane is opened for it here.
    let card = rig.send("/api/board/shop/items", json!({ "title": "Add a cart", "status": "doing" })).await;
    let id = card["id"].as_str().unwrap();
    git(&dir, &["worktree", "add", "--quiet", "-b", "old", "its-own"]);
    let round = json!({
        "blueprint": "std/lane-round", "project": "shop", "wait": true,
        "params": { "project": "shop", "item": id, "agent": crate::lanes::WORKER, "dir": "its-own" },
    });
    let applied = rig.send("/api/apply", round).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    assert!(dir.join("its-own/CHANGES.md").exists());
    assert!(!dir.join(".lanes").join(id).exists(), "a second place was made for the card");
}
