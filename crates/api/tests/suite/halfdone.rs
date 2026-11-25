//! What git was in the middle of when it was cut off, met by the action
//! that comes to it next: put right where it is a card's own, and left
//! where it may be somebody's.

use std::path::Path;

use serde_json::{Value, json};

use crate::lanes::{git, rig, shop};
use crate::{Rig, soon};

/// Asks one thing of a place of the project's, and how that went.
async fn asked(rig: &Rig, action: &str, with: &str) -> Value {
    let source = format!(
        "blueprint: errand\npermissions: [shell]\nsteps:\n  go:\n    do: {action}\n    with: {with}\n"
    );
    let went = rig.send("/api/apply", json!({ "project": "shop", "source": source, "wait": true })).await;
    assert_eq!(went["finished"]["status"], "ok", "{action}: {went}");
    went
}

/// A card's place with something changed in it and kept.
async fn worked(rig: &Rig, dir: &Path, item: &str) {
    asked(rig, "place.make", &format!("{{ item: {item} }}")).await;
    std::fs::write(dir.join(".lanes").join(item).join(format!("{item}.md")), "changed\n").unwrap();
    let with =
        format!("{{ item: {item}, dir: .lanes/{item}, by: shop-worker-1, message: \"{item}: its work\" }}");
    asked(rig, "place.keep", &with).await;
}

#[tokio::test]
async fn a_place_git_was_cut_off_making_is_taken_away_and_made_whole_and_one_somebody_locked_is_theirs() {
    let rig = rig().await;
    let (dir, _) = shop(&rig, json!({})).await;
    // As git leaves one it was stopped in the middle of making: known to
    // the repository, locked in git's own word for one it is still
    // making, and with some of its files.
    let making = ["worktree", "add", "--quiet", "--lock", "--reason", "initializing", "-b"];
    git(&dir, &[&making[..], &["lane/shop-1", ".lanes/shop-1"]].concat());
    let place = dir.join(".lanes/shop-1");
    for entry in std::fs::read_dir(&place)
        .unwrap()
        .flatten()
        .filter(|entry| entry.path().is_file() && entry.file_name() != ".git")
    {
        std::fs::remove_file(entry.path()).unwrap();
    }
    assert_ne!(git(&place, &["status", "--porcelain"]), "");

    asked(&rig, "place.make", "{ item: shop-1 }").await;
    assert!(!git(&dir, &["worktree", "list", "--porcelain"]).contains("locked"));
    assert_eq!(git(&place, &["status", "--porcelain"]), "");
    assert_eq!(git(&place, &["rev-parse", "--abbrev-ref", "HEAD"]), "lane/shop-1");

    // Locked by somebody for a reason of their own, it is as they left it.
    git(
        &dir,
        &["worktree", "add", "--quiet", "--lock", "--reason", "mine", "-b", "lane/shop-2", ".lanes/shop-2"],
    );
    std::fs::write(dir.join(".lanes/shop-2/NOTES.md"), "theirs\n").unwrap();
    asked(&rig, "place.make", "{ item: shop-2 }").await;
    assert!(git(&dir, &["worktree", "list", "--porcelain"]).contains("locked mine"));
    assert_eq!(git(&dir.join(".lanes/shop-2"), &["status", "--porcelain"]), "?? NOTES.md");
}

#[tokio::test]
async fn a_lock_left_on_the_index_of_a_cards_own_place_is_cleared_and_what_was_changed_there_is_kept() {
    let rig = rig().await;
    let (dir, _) = shop(&rig, json!({})).await;
    asked(&rig, "place.make", "{ item: shop-1 }").await;
    let place = dir.join(".lanes/shop-1");
    // A git that was committing here was cut off, and left its lock.
    let its = git(&place, &["rev-parse", "--absolute-git-dir"]);
    std::fs::write(Path::new(&its).join("index.lock"), "").unwrap();
    std::fs::write(place.join("CHANGES.md"), "changed\n").unwrap();

    let with = "{ item: shop-1, dir: .lanes/shop-1, by: shop-worker-1, message: \"shop-1: Add a cart\" }";
    asked(&rig, "place.keep", with).await;
    assert_eq!(git(&dir, &["log", "--format=%s", "-1", "lane/shop-1"]), "shop-1: Add a cart");
    assert_eq!(git(&place, &["status", "--porcelain"]), "");
}

#[tokio::test]
async fn a_merge_of_a_cards_own_branch_that_was_cut_off_is_called_off_and_made_again_and_anybody_elses_is_left()
 {
    let rig = rig().await;
    let (dir, _) = shop(&rig, json!({})).await;
    worked(&rig, &dir, "shop-1").await;
    // The merge of its branch was cut off before it was committed.
    git(&dir, &["merge", "--quiet", "--no-ff", "--no-commit", "lane/shop-1"]);
    assert!(dir.join(".git/MERGE_HEAD").exists());

    asked(&rig, "place.merge", "{ item: shop-1, message: \"shop-1: Add a cart\" }").await;
    assert!(!dir.join(".git/MERGE_HEAD").exists());
    assert_eq!(git(&dir, &["log", "--format=%s", "-1"]), "shop-1: Add a cart");
    assert_eq!(git(&dir, &["status", "--porcelain", "--untracked-files=no"]), "");

    // A merge of anything else under way there is somebody's, and is left.
    worked(&rig, &dir, "shop-2").await;
    worked(&rig, &dir, "shop-3").await;
    git(&dir, &["merge", "--quiet", "--no-ff", "--no-commit", "lane/shop-3"]);
    let theirs = git(&dir, &["rev-parse", "MERGE_HEAD"]);
    asked(&rig, "place.merge", "{ item: shop-2, message: \"shop-2: its work\" }").await;
    assert_eq!(git(&dir, &["rev-parse", "MERGE_HEAD"]), theirs);
    assert_eq!(git(&dir, &["log", "--format=%s", "-1"]), "shop-1: Add a cart");
}

/// Asks one thing of a place and does not wait for it: the run.
async fn asking(rig: &Rig, name: &str, action: &str, with: &str) -> String {
    let source = format!(
        "blueprint: errand-{name}\npermissions: [shell]\nsteps:\n  go:\n    do: {action}\n    with: {with}\n"
    );
    let went = rig.send("/api/apply", json!({ "project": "shop", "source": source })).await;
    went["run"].as_str().unwrap_or_else(|| panic!("{action}: {went}")).to_string()
}

/// Waits until the step of such a run has said `words` in its log.
async fn logged(rig: &Rig, run: &str, words: &str) {
    let (logs, id) = (rig.node().app.engine.logs().clone(), format!("{run}.go"));
    soon(&format!("the step to say {words:?}"), async {
        while !logs.read("runs", &id, 0, 1000).iter().any(|line| line.text.contains(words)) {
            tokio::task::yield_now().await;
        }
    })
    .await;
}

/// What the step of such a run gave, once the run has ended well.
async fn gave(rig: &Rig, run: &str) -> Value {
    let run = soon("the run to end", rig.node().app.engine.wait_run(run)).await.unwrap();
    assert_eq!(run.status, conductor::RunStatus::Ok, "{run:?}");
    run.steps[0].output.clone()
}

#[tokio::test]
async fn a_merge_refused_a_lock_by_somebody_elses_git_waits_for_it_and_the_daemons_own_gits_take_turns() {
    let rig = rig().await;
    let (dir, _) = shop(&rig, json!({})).await;
    worked(&rig, &dir, "shop-1").await;
    asked(&rig, "place.make", "{ item: shop-2 }").await;
    std::fs::write(dir.join(".lanes/shop-2/shop-2.md"), "changed\n").unwrap();
    // Somebody's own git is at work in the project's folder, and has the
    // lock on its index, as a `git status` has for a moment.
    let theirs = dir.join(".git/index.lock");
    std::fs::write(&theirs, "").unwrap();

    // One card's merge is refused the lock, and waits for it.
    let merge = "{ item: shop-1, message: \"shop-1: its work\" }";
    let merging = asking(&rig, "merge", "place.merge", merge).await;
    logged(&rig, &merging, "this waits for it and asks again").await;
    // What was left for another card is to be kept meanwhile, in the same
    // repository: the daemon's own git, which waits its turn there.
    let keep = "{ item: shop-2, dir: .lanes/shop-2, by: shop-worker-1, message: \"shop-2: its work\" }";
    let keeping = asking(&rig, "keep", "place.keep", keep).await;
    logged(&rig, &keeping, "this waits its turn").await;
    assert_eq!(git(&dir, &["log", "--format=%s", "-1", "lane/shop-2"]), "first");

    // Their git is done, and both are: the merge first, as it was asked.
    std::fs::remove_file(&theirs).unwrap();
    assert_eq!(gave(&rig, &merging).await["came"], "merged");
    assert_eq!(gave(&rig, &keeping).await["committed"], true);
    assert_eq!(git(&dir, &["log", "--format=%s", "-1"]), "shop-1: its work");
    assert_eq!(git(&dir, &["log", "--format=%s", "-1", "lane/shop-2"]), "shop-2: its work");
}

/// A lock another git holds is that program's, held in the world's time:
/// a factory whose clock a test holds still goes on once it is let go of.
/// Waited for on the factory's clock, a merge that met a person's
/// `git status` (which holds the index's lock while it runs) waited for
/// ever there, where nothing moves the clock (the simulation's command
/// that runs past its time, which holds its clock still after moving it).
#[tokio::test]
async fn a_lock_another_git_let_go_of_is_waited_for_in_the_worlds_time_and_not_the_factorys_clock() {
    let held = kernel::ManualClock::new();
    let rig = Rig::owned(
        Rig::with("api-lane", move |config, _| {
            config.clock = Some(std::sync::Arc::new(held));
            let does = ["swallow", "append", "CHANGES.md", "changed", "say", "I changed a file."];
            config.adapters.push(("writer".into(), crate::standin::argv(&does)));
        })
        .await,
    )
    .await;
    let (dir, _) = shop(&rig, json!({})).await;
    worked(&rig, &dir, "shop-1").await;
    let theirs = dir.join(".git/index.lock");
    std::fs::write(&theirs, "").unwrap();
    let merging =
        asking(&rig, "merge", "place.merge", "{ item: shop-1, message: \"shop-1: its work\" }").await;
    logged(&rig, &merging, "this waits for it and asks again").await;
    // Their git is done, and nothing moves the factory's clock.
    std::fs::remove_file(&theirs).unwrap();
    assert_eq!(gave(&rig, &merging).await["came"], "merged");
    assert_eq!(git(&dir, &["log", "--format=%s", "-1"]), "shop-1: its work");
}

#[tokio::test]
async fn a_merge_that_fails_after_git_wrote_its_files_leaves_the_project_as_it_was_and_the_next_is_not_refused()
 {
    let rig = rig().await;
    let (dir, _) = shop(&rig, json!({})).await;
    worked(&rig, &dir, "shop-1").await;
    worked(&rig, &dir, "shop-2").await;
    let merge = |item: &str| format!("{{ item: {item}, message: \"{item}: its work\" }}");
    // Git cannot make the merge's commit: it is to sign it, with a program
    // that is not there. By then it has written the merged files into the
    // project's folder and its index, as it has when it is refused a lock
    // between writing them and committing.
    git(&dir, &["config", "commit.gpgsign", "true"]);
    git(&dir, &["config", "gpg.program", "/nowhere/gpg"]);
    let went = gave(&rig, &asking(&rig, "first", "place.merge", &merge("shop-1")).await).await;
    assert_eq!(went["came"], "stuck", "{went}");

    // The project is as it was: nothing put forward, nothing changed.
    assert_eq!(git(&dir, &["status", "--porcelain", "--untracked-files=no"]), "");
    assert!(!dir.join("shop-1.md").exists());
    assert_eq!(git(&dir, &["log", "--format=%s", "-1"]), "first");

    // Put right, the next card's merge is not refused for anything the
    // first left there, and the first is merged when it is asked again.
    git(&dir, &["config", "--unset", "commit.gpgsign"]);
    let next = gave(&rig, &asking(&rig, "next", "place.merge", &merge("shop-2")).await).await;
    assert_eq!(next["came"], "merged", "{next}");
    let again = gave(&rig, &asking(&rig, "again", "place.merge", &merge("shop-1")).await).await;
    assert_eq!(again["came"], "merged", "{again}");
}

#[tokio::test]
async fn what_somebody_put_forward_in_the_projects_folder_is_left_as_it_is_and_the_merge_says_so() {
    let rig = rig().await;
    let (dir, _) = shop(&rig, json!({})).await;
    worked(&rig, &dir, "shop-1").await;
    std::fs::write(dir.join("NOTES.md"), "theirs\n").unwrap();
    git(&dir, &["add", "NOTES.md"]);

    let merge = "{ item: shop-1, message: \"shop-1: its work\" }";
    let went = gave(&rig, &asking(&rig, "over", "place.merge", merge).await).await;
    assert_eq!(went["came"], "stuck", "{went}");
    assert!(went["why"].as_str().unwrap().contains("put forward in the project's folder"), "{went}");
    assert_eq!(git(&dir, &["status", "--porcelain", "--untracked-files=no"]), "A  NOTES.md");
}
