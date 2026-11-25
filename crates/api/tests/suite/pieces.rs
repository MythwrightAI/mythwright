//! Several pieces of work on one item, each in a place of its own, and
//! one of them made the item's: places under a key of their own
//! (`<item>--<way>`), cut from what the item had so far, held against it,
//! given to the item, and the rest kept as branches or thrown away.

use std::path::Path;

use serde_json::{Value, json};

use crate::Rig;
use crate::lanes::{git, rig, shop};

/// Asks one thing of a place of the project's: what it gave, or why it failed.
async fn asked(rig: &Rig, action: &str, with: &str) -> Result<Value, String> {
    let source = format!(
        "blueprint: errand\npermissions: [shell]\nsteps:\n  go:\n    do: {action}\n    with: {with}\noutputs:\n  \
         out: ${{steps.go.out}}\n"
    );
    let went = rig.send("/api/apply", json!({ "project": "shop", "source": source, "wait": true })).await;
    match went["finished"]["status"].as_str() {
        Some("ok") => Ok(went["finished"]["outputs"]["out"].clone()),
        _ => Err(went["finished"]["error"].as_str().map_or_else(|| went.to_string(), str::to_string)),
    }
}

/// `item`'s place made (cut from `from`'s branch where one is said), a
/// file written in it, and kept as a commit that says `what`.
async fn worked(rig: &Rig, dir: &Path, item: &str, from: Option<&str>, what: &str) {
    let from = from.map(|from| format!(", from: {from}")).unwrap_or_default();
    asked(rig, "place.make", &format!("{{ item: \"{item}\"{from} }}")).await.unwrap();
    std::fs::write(dir.join(".lanes").join(item).join(format!("{what}.md")), format!("{what}\n")).unwrap();
    let with = format!("{{ item: \"{item}\", dir: \".lanes/{item}\", by: worker, message: \"{what}\" }}");
    asked(rig, "place.keep", &with).await.unwrap();
}

fn log(dir: &Path, branch: &str) -> String {
    git(dir, &["log", "--format=%s", branch])
}

fn branches(dir: &Path) -> String {
    git(dir, &["branch", "--format=%(refname:short)"])
}

#[tokio::test]
async fn pieces_of_work_on_one_item_begin_from_what_it_had_and_the_one_it_takes_becomes_its_own_place() {
    let rig = rig().await;
    let (dir, _) = shop(&rig, json!({})).await;
    worked(&rig, &dir, "shop-1", None, "round-one").await;

    // Each is cut from what the item had so far, and not from the project's branch.
    worked(&rig, &dir, "shop-1--a", Some("shop-1"), "way-a").await;
    worked(&rig, &dir, "shop-1--b", Some("shop-1"), "way-b").await;
    assert_eq!(log(&dir, "lane/shop-1--a"), "way-a\nround-one\nfirst");
    assert_eq!(log(&dir, "lane/shop-1--b"), "way-b\nround-one\nfirst");
    // An item with no branch to cut from is cut from the project's, as one with no `from` is.
    worked(&rig, &dir, "shop-9--a", Some("shop-9"), "alone").await;
    assert_eq!(log(&dir, "lane/shop-9--a"), "alone\nfirst");

    // What one did beside what the item had, as a patch as long as is asked.
    let changed =
        asked(&rig, "place.changed", "{ item: \"shop-1--a\", against: shop-1, patch: 65536 }").await;
    let changed = changed.unwrap();
    let patch = changed["patch"].as_str().unwrap();
    assert!(patch.contains("+way-a") && !patch.contains("round-one"), "{patch}");
    assert_eq!(changed["cut"], false);
    let short = asked(&rig, "place.changed", "{ item: \"shop-1--a\", against: shop-1, patch: 40 }").await;
    let short = short.unwrap();
    assert_eq!(short["cut"], true);
    assert!(short["patch"].as_str().unwrap().len() <= 40, "{short}");
    // Against the project's branch where nothing is said, and no patch unless one is asked for.
    let whole = asked(&rig, "place.changed", "{ item: \"shop-1--a\" }").await.unwrap();
    assert!(whole["changed"].as_str().unwrap().contains("round-one.md"), "{whole}");
    assert_eq!(whole["patch"], "");

    // Taken: the item's place is that work's, on the item's own branch, and the other's is gone.
    let taken = asked(&rig, "place.take", "{ item: shop-1, from: \"shop-1--a\" }").await.unwrap();
    assert_eq!((taken["came"].as_str(), taken["dir"].as_str()), (Some("taken"), Some(".lanes/shop-1")));
    assert_eq!(git(&dir.join(".lanes/shop-1"), &["rev-parse", "--abbrev-ref", "HEAD"]), "lane/shop-1");
    assert_eq!(log(&dir, "lane/shop-1"), "way-a\nround-one\nfirst");
    assert!(dir.join(".lanes/shop-1/way-a.md").exists() && !dir.join(".lanes/shop-1--a").exists());
    assert!(!branches(&dir).lines().any(|one| one == "lane/shop-1--a"));
    // Asked again, it is there.
    let again = asked(&rig, "place.take", "{ item: shop-1, from: \"shop-1--a\" }").await.unwrap();
    assert_eq!(again["came"], "there");

    // The other kept as a branch: its worktree goes and its work stays.
    let kept = asked(&rig, "place.clear", "{ item: \"shop-1--b\" }").await.unwrap();
    assert_eq!(kept["came"], "branch_kept");
    assert!(!dir.join(".lanes/shop-1--b").exists());
    assert_eq!(log(&dir, "lane/shop-1--b"), "way-b\nround-one\nfirst");

    // Or thrown away: the worktree with what was never committed in it, and the branch.
    worked(&rig, &dir, "shop-1--c", Some("shop-1"), "way-c").await;
    std::fs::write(dir.join(".lanes/shop-1--c/loose.md"), "never committed\n").unwrap();
    let refused = asked(&rig, "place.clear", "{ item: \"shop-1--c\" }").await.unwrap();
    assert_eq!(refused["came"], "place_kept");
    let thrown = asked(&rig, "place.clear", "{ item: \"shop-1--c\", discard: true }").await.unwrap();
    assert_eq!(thrown["came"], "cleared");
    assert!(!dir.join(".lanes/shop-1--c").exists());
    assert!(!branches(&dir).lines().any(|one| one == "lane/shop-1--c"));
}

#[tokio::test]
async fn a_place_taken_half_way_is_finished_and_nothing_only_the_item_has_is_lost() {
    let rig = rig().await;
    let (dir, _) = shop(&rig, json!({})).await;

    // Stopped with the item's own worktree taken away and the other's not yet moved.
    worked(&rig, &dir, "shop-1", None, "one").await;
    worked(&rig, &dir, "shop-1--a", Some("shop-1"), "one-a").await;
    git(&dir, &["worktree", "remove", ".lanes/shop-1"]);
    let taken = asked(&rig, "place.take", "{ item: shop-1, from: \"shop-1--a\" }").await.unwrap();
    assert_eq!(taken["came"], "taken");
    assert_eq!(log(&dir, "lane/shop-1"), "one-a\none\nfirst");

    // Stopped with the other's worktree moved and its branch not yet renamed.
    worked(&rig, &dir, "shop-2", None, "two").await;
    worked(&rig, &dir, "shop-2--a", Some("shop-2"), "two-a").await;
    git(&dir, &["worktree", "remove", ".lanes/shop-2"]);
    git(&dir, &["worktree", "move", ".lanes/shop-2--a", ".lanes/shop-2"]);
    let taken = asked(&rig, "place.take", "{ item: shop-2, from: \"shop-2--a\" }").await.unwrap();
    assert_eq!(taken["came"], "taken");
    assert_eq!(git(&dir.join(".lanes/shop-2"), &["rev-parse", "--abbrev-ref", "HEAD"]), "lane/shop-2");
    assert_eq!(log(&dir, "lane/shop-2"), "two-a\ntwo\nfirst");

    // Work the other's place has no branch of its worktree for: given one on the branch.
    worked(&rig, &dir, "shop-3--a", None, "three-a").await;
    git(&dir, &["worktree", "remove", ".lanes/shop-3--a"]);
    let taken = asked(&rig, "place.take", "{ item: shop-3, from: \"shop-3--a\" }").await.unwrap();
    assert_eq!(taken["came"], "taken");
    assert!(dir.join(".lanes/shop-3/three-a.md").exists());

    // What was never committed in the item's own place is not thrown away.
    worked(&rig, &dir, "shop-4", None, "four").await;
    worked(&rig, &dir, "shop-4--a", Some("shop-4"), "four-a").await;
    std::fs::write(dir.join(".lanes/shop-4/loose.md"), "only here\n").unwrap();
    let why = asked(&rig, "place.take", "{ item: shop-4, from: \"shop-4--a\" }").await.unwrap_err();
    assert!(why.contains("never committed"), "{why}");
    assert!(dir.join(".lanes/shop-4/loose.md").exists() && dir.join(".lanes/shop-4--a").exists());

    // Nor work on the item's branch that the other's has not.
    worked(&rig, &dir, "shop-5", None, "five").await;
    worked(&rig, &dir, "shop-5--a", None, "five-a").await;
    let why = asked(&rig, "place.take", "{ item: shop-5, from: \"shop-5--a\" }").await.unwrap_err();
    assert!(why.contains("has work that lane/shop-5--a has not"), "{why}");
    assert_eq!(log(&dir, "lane/shop-5"), "five\nfirst");

    // A place is taken from another's, and one that has none is said to have none.
    let why = asked(&rig, "place.take", "{ item: shop-6, from: shop-6 }").await.unwrap_err();
    assert!(why.contains("the item itself"), "{why}");
    let why = asked(&rig, "place.take", "{ item: shop-6, from: \"shop-6--a\" }").await.unwrap_err();
    assert!(why.contains("has no place to give"), "{why}");
    let why = asked(&rig, "place.make", "{ item: shop-6, from: ../elsewhere }").await.unwrap_err();
    assert!(why.contains("`from` names another piece of work"), "{why}");
}

#[tokio::test]
async fn no_place_is_taken_in_a_project_that_is_no_repository_and_that_is_said_as_it_is() {
    let rig = rig().await;
    let (dir, _) = shop(&rig, json!({})).await;
    // A repository with no history (one with none at all is found as the checkout the tests are in):
    // every piece of work on an item is worked in the project itself.
    std::fs::remove_dir_all(dir.join(".git")).unwrap();
    git(&dir, &["init", "--quiet", "."]);
    let made = asked(&rig, "place.make", "{ item: \"shop-1--a\", from: shop-1 }").await.unwrap();
    assert_eq!(made["came"], "none", "{made}");
    let refused = asked(&rig, "place.take", "{ item: shop-1, from: \"shop-1--a\" }").await.unwrap_err();
    assert!(refused.contains("the project is no repository with history"), "{refused}");
    assert!(!refused.contains("there is no branch"), "{refused}");
}
