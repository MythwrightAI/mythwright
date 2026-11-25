//! A running tree left on the blueprints it was applied with, moved to
//! what ships now, and put back, from the terminal.
//!
//! What ships here is a package's blueprint: it is another when its
//! package is, as the standard ones are when the program is, and is held
//! for a running tree the same way.

use kernel::Source;
use serde_json::json;

use crate::{Rig, soon};

/// The project's own: for each thing wanted it has the package's round done.
const SHOP: &str = "\
blueprint: shop
on:
  wanted:
    event: shop.wanted
    steps:
      done: { call: reach/round, as: \"round-${ event.data.id }\", with: { id: ${ event.data.id } } }
";

/// The package as it is released: its round says its version, what it is
/// handed, and one thing it does.
fn package(version: u32, more: &str, says: &str) -> String {
    format!(
        "extension: reach\ntitle: Reach\nblueprints:\n  round: |\n    blueprint: reach/round\n    version: {version}\n    \
         params:\n      id: {{ type: int }}\n{more}    steps:\n      made: {{ do: emit, with: {{ topic: {says} }} }}\n"
    )
}

/// A thing is wanted, and what its round says it did is waited for.
async fn wanted(rig: &Rig, id: u32, says: &str) {
    let journal = &rig.node.as_ref().unwrap().app.journal;
    let mut made = journal.follow_every(journal.now(), says);
    journal.emit("shop.wanted", Source::default(), json!({ "id": id })).await.unwrap();
    soon(&format!("the round for {id} to say {says}"), made.next()).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_running_tree_is_left_moved_and_put_back_from_the_terminal_each_said_by_version() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    let blueprints = rig.placed("shop");
    std::fs::create_dir_all(&blueprints).unwrap();
    let shop = blueprints.join(brand::blueprint_file("shop"));
    let shop = shop.to_str().unwrap();
    std::fs::write(shop, SHOP).unwrap();
    let folder = rig.dir.join("reach");
    std::fs::create_dir_all(&folder).unwrap();
    let release = |text: String| std::fs::write(folder.join("extension.yaml"), text).unwrap();
    release(package(1, "", "shop.made_old"));
    rig.run(&["extension", "add", folder.to_str().unwrap()]).await.ok();
    rig.run(&["apply", shop, "--project", "shop"]).await.ok();
    wanted(&rig, 1, "shop.made_old").await;

    // The package is another: its round is version 2, and needs something
    // the shop never hands. What is running goes on with the round it had.
    release(package(2, "      place: { type: string }\n", "shop.made_new"));
    rig.run(&["extension", "add", folder.to_str().unwrap()]).await.ok();
    wanted(&rig, 2, "shop.made_old").await;

    // Applied again as it is, it says what ships now by version, and leaves it.
    let moved = "  reach/round 1 to 2: the param `place` is new; the step `made` is changed\n";
    let left = rig.run(&["apply", shop, "--project", "shop"]).await.ok();
    assert!(left.contains("what is running goes on with it as it was") && left.contains(moved), "{left}");
    // Taking it is held to what ships now first: the shop of before does not fit it.
    let refused = rig.run(&["apply", shop, "--project", "shop", "--take"]).await;
    assert_eq!(refused.code, 1, "{}", refused.out);
    assert!(refused.err.contains("needs param `place`"), "{}", refused.err);
    wanted(&rig, 3, "shop.made_old").await;

    // The shop written for it, applied taking it: moved, and said.
    std::fs::write(shop, SHOP.replace("id: ${ event.data.id } }", "id: ${ event.data.id }, place: here }"))
        .unwrap();
    let taken = rig.run(&["apply", shop, "--project", "shop", "--take"]).await.ok();
    assert!(taken.contains(&format!("moved to what ships now:\n{moved}")), "{taken}");
    wanted(&rig, 4, "shop.made_new").await;

    // Put back as it is, the shop written for the new round does not fit the old one.
    let unfit = rig.run(&["apply", shop, "--project", "shop", "--back"]).await;
    assert_eq!(unfit.code, 1, "{}", unfit.out);
    assert!(
        unfit.err.contains("cannot be put back on the texts it had")
            && unfit.err.contains("has no param `place`"),
        "{}",
        unfit.err
    );
    // The shop as it was: first only said, which changes nothing.
    std::fs::write(shop, SHOP).unwrap();
    let back = "  reach/round 2 to 1: the param `place` is gone; the step `made` is changed\n";
    let would = rig.run(&["apply", shop, "--project", "shop", "--back", "--say"]).await.ok();
    assert_eq!(would, format!("putting it back would put it on (nothing was changed):\n{back}"));
    wanted(&rig, 5, "shop.made_new").await;
    // Then done: it is on the round it had, though the package still brings the later one.
    let put = rig.run(&["apply", shop, "--project", "shop", "--back"]).await.ok();
    assert!(put.contains(&format!("put back on the texts it had:\n{back}")), "{put}");
    wanted(&rig, 6, "shop.made_old").await;
}

/// The project's own, which keeps the package's keeper running for what it hears.
const MINDER: &str = "\
blueprint: minder
on:
  wanted:
    event: shop.wanted
    steps:
      keep: { spawn: reach/keeper, as: keeper }
";

/// The package's keeper as it is released: what it does as it begins says its version.
fn keeper(version: u32) -> String {
    format!(
        "extension: reach\ntitle: Reach\nblueprints:\n  keeper: |\n    blueprint: reach/keeper\n    version: \
         {version}\n    steps:\n      began: {{ do: emit, with: {{ topic: shop.kept_{version} }} }}\n    on:\n      \
         poke: {{ event: shop.poke, do: emit, with: {{ topic: shop.poked }} }}\n"
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn taking_what_ships_at_the_top_from_the_terminal_brings_what_runs_under_it_along() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    let blueprints = rig.placed("shop");
    std::fs::create_dir_all(&blueprints).unwrap();
    let minder = blueprints.join(brand::blueprint_file("minder"));
    let minder = minder.to_str().unwrap();
    std::fs::write(minder, MINDER).unwrap();
    let folder = rig.dir.join("reach");
    std::fs::create_dir_all(&folder).unwrap();
    let release = |text: String| std::fs::write(folder.join("extension.yaml"), text).unwrap();
    release(keeper(1));
    rig.run(&["extension", "add", folder.to_str().unwrap()]).await.ok();
    rig.run(&["apply", minder, "--project", "shop"]).await.ok();
    wanted(&rig, 1, "shop.kept_1").await;

    // A later keeper, which begins otherwise: what is running goes on with the one it had.
    release(keeper(2));
    rig.run(&["extension", "add", folder.to_str().unwrap()]).await.ok();
    let journal = &rig.node.as_ref().unwrap().app.journal;
    let mut kept = journal.follow_every(journal.now(), "shop.kept_2");

    // Taken at the top, whose own text is what it was: the keeper under it
    // begins again on what ships now, and the terminal says so.
    let taken = rig.run(&["apply", minder, "--project", "shop", "--take"]).await.ok();
    assert!(taken.contains("moved to what ships now:\n  reach/keeper 1 to 2"), "{taken}");
    assert!(taken.contains("what is under it is brought to what it now goes by"), "{taken}");
    assert!(!taken.contains("nothing to do"), "{taken}");
    soon("the keeper to begin on what ships now", kept.next()).await.unwrap();
    let listed = rig.run(&["instances"]).await.ok();
    assert!(!listed.contains("behind"), "{listed}");
}
