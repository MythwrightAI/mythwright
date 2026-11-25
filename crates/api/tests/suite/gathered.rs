//! Several pieces of work gathered into one place, and the project's branch
//! brought forward to it and sent (`place.gather`, `place.forward`): taken
//! whole or not at all, never over somebody's work, and the same done
//! twice; and a gathering cut off in each state git can be left in between
//! two of its writes, asked again, coming to the same commits.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::Rig;
use crate::lanes::git;

/// A `shop` project that is a repository with one commit on `main`, a
/// remote beside it, and a branch for each of `items`, each adding a file
/// of its own (or, where two say the same file, changing it each its way).
pub(crate) async fn shop(rig: &Rig, items: &[(&str, &str, &str)]) -> (PathBuf, PathBuf) {
    let project = rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let dir = PathBuf::from(project["path"].as_str().unwrap());
    let remote = rig.dir.join("remote.git");
    git(&rig.dir, &["init", "--quiet", "--bare", remote.to_str().unwrap()]);
    git(&dir, &["init", "--quiet", "--initial-branch", "main", "."]);
    git(&dir, &["config", "user.name", "Ada"]);
    git(&dir, &["config", "user.email", "ada@example.com"]);
    std::fs::write(dir.join("README.md"), "the shop\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "--quiet", "-m", "first"]);
    git(&dir, &["remote", "add", "origin", remote.to_str().unwrap()]);
    git(&dir, &["push", "--quiet", "origin", "main"]);
    for (item, file, says) in items {
        branched(&dir, item, file, says);
    }
    (dir, remote)
}

/// A branch `lane/<item>` cut from `main` with `file` saying `says`.
fn branched(dir: &Path, item: &str, file: &str, says: &str) {
    let branch = format!("lane/{item}");
    git(dir, &["branch", &branch, "main"]);
    let at = dir.join(".work");
    git(dir, &["worktree", "add", "--quiet", at.to_str().unwrap(), &branch]);
    std::fs::write(at.join(file), format!("{says}\n")).unwrap();
    git(&at, &["add", "-A"]);
    git(&at, &["commit", "--quiet", "-m", &format!("{item}: its work")]);
    git(dir, &["worktree", "remove", at.to_str().unwrap()]);
}

/// Asks one thing of the project's repository, and what its step gave.
async fn asked(rig: &Rig, action: &str, with: Value) -> Value {
    let step = asking(rig, action, with).await;
    assert_eq!(step["status"], "ok", "{action}: {step}");
    step["output"].clone()
}

/// The same, and the step as it ended, however that was.
async fn asking(rig: &Rig, action: &str, with: Value) -> Value {
    let source = format!(
        "blueprint: errand\npermissions: [shell]\nsteps:\n  go:\n    do: {action}\n    with: {with}\n"
    );
    let went = rig.send("/api/apply", json!({ "project": "shop", "source": source, "wait": true })).await;
    went["finished"]["steps"][0].clone()
}

/// What each commit on a branch's first parents says, the newest first.
pub(crate) fn subjects(dir: &Path, branch: &str) -> Vec<String> {
    git(dir, &["log", "--first-parent", "--format=%s", branch]).lines().map(str::to_string).collect()
}

/// What a stop left of a gathering, made by hand.
type Left<'a> = Box<dyn Fn() + 'a>;

const GATHERED: [&str; 4] = ["Gather lane/shop-3", "Gather lane/shop-2", "Gather lane/shop-1", "first"];

#[tokio::test]
async fn branches_are_gathered_one_after_another_and_the_projects_branch_brought_forward_to_them_and_sent() {
    let rig = Rig::new().await;
    let (dir, remote) = shop(
        &rig,
        &[("shop-1", "one.md", "one"), ("shop-2", "two.md", "two"), ("shop-3", "three.md", "three")],
    )
    .await;
    git(&dir, &["branch", "feature", "main"]);
    let with =
        json!({ "item": "train-1", "items": ["shop-1", "shop-2", "shop-9"], "branches": ["lane/shop-3"] });
    let gave = asked(&rig, "place.gather", with.clone()).await;
    assert_eq!(gave["came"], "gathered", "{gave}");
    assert_eq!(gave["merged"], json!(["lane/shop-1", "lane/shop-2", "lane/shop-3"]), "{gave}");
    assert_eq!(gave["nothing"], json!(["shop-9"]), "{gave}");
    assert_eq!(
        (gave["dir"].as_str(), gave["branch"].as_str()),
        (Some(".lanes/train-1"), Some("lane/train-1"))
    );
    let place = dir.join(".lanes/train-1");
    assert_eq!(git(&place, &["rev-parse", "--abbrev-ref", "HEAD"]), "lane/train-1");
    assert_eq!(subjects(&dir, "lane/train-1"), GATHERED);
    // Each merge of an item's branch says which item, as `place.merge` does.
    let said = git(&dir, &["log", "-1", "--format=%(trailers:key=Item,valueonly)", "lane/train-1~1"]);
    assert_eq!(said, "shop-2");
    // The project's branch is as it was.
    assert_eq!(subjects(&dir, "main"), ["first"]);

    // Asked again, it holds all of them already, and answers the same.
    let again = asked(&rig, "place.gather", with).await;
    assert_eq!((&again["came"], &again["merged"]), (&json!("gathered"), &json!([])), "{again}");
    assert_eq!(again["had"], json!(["lane/shop-1", "lane/shop-2", "lane/shop-3"]), "{again}");
    assert_eq!(subjects(&dir, "lane/train-1"), GATHERED);

    // Somebody's file in the project's folder that nothing brought touches
    // is kept as it was.
    std::fs::write(dir.join("NOTES.md"), "mine\n").unwrap();
    std::fs::write(dir.join("README.md"), "the shop, as I left it\n").unwrap();
    let forward = asked(&rig, "place.forward", json!({ "item": "train-1" })).await;
    assert_eq!((&forward["came"], &forward["sent"]), (&json!("forwarded"), &json!(false)), "{forward}");
    assert_eq!(git(&dir, &["rev-parse", "main"]), git(&dir, &["rev-parse", "lane/train-1"]));
    assert_eq!(std::fs::read_to_string(dir.join("three.md")).unwrap(), "three\n");
    assert_eq!(std::fs::read_to_string(dir.join("README.md")).unwrap(), "the shop, as I left it\n");
    assert_eq!(std::fs::read_to_string(dir.join("NOTES.md")).unwrap(), "mine\n");
    // Brought forward already, and now sent.
    let sent = asked(&rig, "place.forward", json!({ "item": "train-1", "remote": "origin" })).await;
    assert_eq!((&sent["came"], &sent["sent"], &sent["key"]), (&json!("forwarded"), &json!(true), &json!("")));
    assert_eq!(git(&remote, &["rev-parse", "main"]), git(&dir, &["rev-parse", "main"]));
    // A branch nobody has checked out is moved where it is, and sent under another name.
    let release = asked(
        &rig,
        "place.forward",
        json!({ "item": "train-1", "onto": "feature", "remote": "origin", "to": "release" }),
    )
    .await;
    assert_eq!(release["came"], "forwarded", "{release}");
    assert_eq!(git(&dir, &["rev-parse", "feature"]), git(&dir, &["rev-parse", "main"]));
    assert_eq!(git(&remote, &["rev-parse", "release"]), git(&dir, &["rev-parse", "main"]));
    assert_eq!(git(&dir, &["rev-parse", "--abbrev-ref", "HEAD"]), "main");
}

#[tokio::test]
async fn a_branch_that_does_not_go_with_what_was_gathered_before_it_leaves_nothing_of_the_asking() {
    let rig = Rig::new().await;
    let (dir, _) =
        shop(&rig, &[("shop-1", "same.md", "one way"), ("shop-2", "same.md", "the other way")]).await;
    let first = git(&dir, &["rev-parse", "main"]);
    let gave = asked(&rig, "place.gather", json!({ "item": "train-1", "items": ["shop-1", "shop-2"] })).await;
    assert_eq!((&gave["came"], &gave["with"]), (&json!("conflict"), &json!("lane/shop-2")), "{gave}");
    assert!(gave["said"].as_str().unwrap().contains("same.md"), "{gave}");
    assert_eq!(gave["merged"], json!([]), "{gave}");
    // The place is on its branch, as it was cut, with nothing under way.
    let place = dir.join(".lanes/train-1");
    assert_eq!(git(&dir, &["rev-parse", "lane/train-1"]), first);
    assert_eq!(git(&place, &["rev-parse", "--abbrev-ref", "HEAD"]), "lane/train-1");
    assert_eq!(git(&place, &["status", "--porcelain"]), "");
    assert!(!Path::new(&git(&place, &["rev-parse", "--absolute-git-dir"])).join("MERGE_HEAD").exists());
    // Without the one that did not go in, the rest are gathered.
    let gave = asked(&rig, "place.gather", json!({ "item": "train-1", "items": ["shop-1"] })).await;
    assert_eq!(gave["came"], "gathered", "{gave}");
    assert_eq!(subjects(&dir, "lane/train-1"), ["Gather lane/shop-1", "first"]);
    // A branch the repository has not got is refused, and so is an item that is no name.
    let missing = asking(&rig, "place.gather", json!({ "item": "train-1", "branches": ["nowhere"] })).await;
    assert!(missing["error"].as_str().unwrap().contains("no branch called nowhere"), "{missing}");
    let outside = asking(&rig, "place.gather", json!({ "item": "train-1", "items": ["../up"] })).await;
    assert!(outside["error"].as_str().unwrap().contains("cannot be the name"), "{outside}");
}

#[tokio::test]
async fn the_projects_branch_is_not_brought_forward_over_work_it_has_or_a_file_somebody_changed() {
    let rig = Rig::new().await;
    let (dir, _) = shop(&rig, &[("shop-1", "README.md", "changed by shop-1")]).await;
    asked(&rig, "place.gather", json!({ "item": "train-1", "items": ["shop-1"] })).await;
    // Somebody changed a file the gathered work changes, and never committed it.
    std::fs::write(dir.join("README.md"), "mine\n").unwrap();
    let main = git(&dir, &["rev-parse", "main"]);
    let stuck = asked(&rig, "place.forward", json!({ "item": "train-1" })).await;
    assert_eq!(stuck["came"], "stuck", "{stuck}");
    assert!(stuck["why"].as_str().unwrap().contains("README.md"), "{stuck}");
    assert_eq!(git(&dir, &["rev-parse", "main"]), main);
    assert_eq!(std::fs::read_to_string(dir.join("README.md")).unwrap(), "mine\n");
    // Put back, and something committed on the project's branch since: no
    // fast-forward, and nothing moved.
    git(&dir, &["checkout", "--quiet", "--", "README.md"]);
    std::fs::write(dir.join("LATER.md"), "later\n").unwrap();
    git(&dir, &["add", "LATER.md"]);
    git(&dir, &["commit", "--quiet", "-m", "later"]);
    let later = git(&dir, &["rev-parse", "main"]);
    let diverged = asked(&rig, "place.forward", json!({ "item": "train-1", "remote": "origin" })).await;
    assert_eq!((&diverged["came"], &diverged["sent"]), (&json!("diverged"), &json!(false)), "{diverged}");
    assert_eq!(git(&dir, &["rev-parse", "main"]), later);
    // Gathered again onto what is there now, it goes.
    let at = dir.join(".lanes/train-1");
    git(&dir, &["worktree", "remove", "--force", at.to_str().unwrap()]);
    git(&dir, &["branch", "--quiet", "-D", "lane/train-1"]);
    asked(&rig, "place.gather", json!({ "item": "train-1", "items": ["shop-1"] })).await;
    let forward = asked(&rig, "place.forward", json!({ "item": "train-1" })).await;
    assert_eq!(forward["came"], "forwarded", "{forward}");
    assert_eq!(subjects(&dir, "main"), ["Gather lane/shop-1", "later", "first"]);
}

#[tokio::test]
async fn a_gathering_cut_off_between_any_two_of_its_writes_and_asked_again_comes_to_the_same_commits() {
    let rig = Rig::new().await;
    let items = [("shop-1", "one.md", "one"), ("shop-2", "two.md", "two"), ("shop-3", "three.md", "three")];
    let (dir, _) = shop(&rig, &items).await;
    let with = json!({ "item": "train-1", "items": ["shop-1", "shop-2", "shop-3"] });
    asked(&rig, "place.gather", with.clone()).await;
    let tree = git(&dir, &["rev-parse", "lane/train-1^{tree}"]);
    let place = dir.join(".lanes/train-1");
    let at = place.to_str().unwrap().to_string();
    // What the action itself does, as git leaves it at each write.
    let gather = |n: usize| {
        for (item, _, _) in &items[..n] {
            let (message, trailer) = (format!("Gather lane/{item}"), format!("Item: {item}"));
            let branch = format!("lane/{item}");
            git(
                &place,
                &["merge", "--quiet", "--no-ff", "--no-edit", "-m", &message, "-m", &trailer, &branch],
            );
        }
    };
    let made = || git(&dir, &["worktree", "add", "--quiet", "-b", "lane/train-1", &at, "main"]);
    let states: Vec<(&str, Left<'_>)> = vec![
        ("nothing made", Box::new(|| {})),
        (
            "its place half made",
            Box::new(|| {
                let making = ["worktree", "add", "--quiet", "--lock", "--reason", "initializing", "-b"];
                git(&dir, &[&making[..], &["lane/train-1", &at, "main"]].concat());
            }),
        ),
        (
            "its place made",
            Box::new(|| {
                made();
            }),
        ),
        (
            "let go of its branch",
            Box::new(|| {
                made();
                git(&place, &["checkout", "--quiet", "--detach"]);
            }),
        ),
        (
            "one merged",
            Box::new(|| {
                made();
                git(&place, &["checkout", "--quiet", "--detach"]);
                gather(1);
            }),
        ),
        (
            "two merged and the third under way",
            Box::new(|| {
                made();
                git(&place, &["checkout", "--quiet", "--detach"]);
                gather(2);
                git(&place, &["merge", "--quiet", "--no-ff", "--no-commit", "lane/shop-3"]);
            }),
        ),
        (
            "all merged, its branch not yet moved",
            Box::new(|| {
                made();
                git(&place, &["checkout", "--quiet", "--detach"]);
                gather(3);
            }),
        ),
        (
            "its branch moved, and the place not yet back on it",
            Box::new(|| {
                made();
                git(&place, &["checkout", "--quiet", "--detach"]);
                gather(3);
                git(&place, &["update-ref", "refs/heads/lane/train-1", "HEAD"]);
            }),
        ),
    ];
    for (state, left) in states {
        // Taken back to before it was asked, and left as a stop would leave it.
        git(&dir, &["worktree", "remove", "--force", &at]);
        git(&dir, &["branch", "--quiet", "-D", "lane/train-1"]);
        git(&dir, &["worktree", "prune"]);
        left();
        let gave = asked(&rig, "place.gather", with.clone()).await;
        assert_eq!(gave["came"], "gathered", "{state}: {gave}");
        assert_eq!(subjects(&dir, "lane/train-1"), GATHERED, "{state}");
        assert_eq!(git(&dir, &["rev-parse", "lane/train-1^{tree}"]), tree, "{state}");
        assert_eq!(git(&place, &["rev-parse", "--abbrev-ref", "HEAD"]), "lane/train-1", "{state}");
        assert_eq!(git(&place, &["status", "--porcelain"]), "", "{state}");
        assert!(!git(&dir, &["worktree", "list", "--porcelain"]).contains("locked"), "{state}");
    }
}
