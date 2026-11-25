//! `instances` and `apply --instance`, for an instance whose blueprint's
//! file now says how one that is running takes it (`on_update`): the
//! terminal says it would be carried over, and says so when it has been.

use kernel::Source;
use serde_json::json;

use crate::{Rig, soon};

/// Starts a worker when it is asked for one; none of its steps names it.
const WATCHER: &str = "\
blueprint: watcher
on:
  wanted:
    event: shop.wanted
    steps:
      go: { spawn: worker, as: the-worker }
";

const ONE: &str = "\
blueprint: worker
version: 1
state: { seen: 4 }
steps:
  claim: { do: emit, with: { topic: shop.claimed } }
on:
  poke: { event: shop.poke, do: noop }
";

const TWO: &str = "\
blueprint: worker
version: 2
state: { heard: 0 }
steps:
  claim: { do: emit, with: { topic: shop.claimed_again } }
on:
  poke: { event: shop.poke, do: noop }
on_update:
  - do: set
    with: { heard: ${ was.state.seen } }
";

#[tokio::test]
async fn the_terminal_says_an_instance_would_be_carried_over_and_that_it_was_when_it_is_applied_again() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    let blueprints = rig.placed("shop");
    std::fs::create_dir_all(&blueprints).unwrap();
    let file = |name: &str| blueprints.join(brand::blueprint_file(name));
    std::fs::write(file("watcher"), WATCHER).unwrap();
    std::fs::write(file("worker"), ONE).unwrap();
    rig.run(&["apply", file("watcher").to_str().unwrap(), "--project", "shop"]).await.ok();

    let journal = &rig.node.as_ref().unwrap().app.journal;
    let mut active = journal.follow_every(Default::default(), "instance.active");
    journal.emit("shop.wanted", Source::default(), json!({})).await.unwrap();
    soon("the watcher and the worker to settle", async {
        for _ in 0..2 {
            active.next().await.unwrap();
        }
    })
    .await;
    let listed = rig.run(&["instances"]).await.ok();
    let row = listed.lines().find(|line| line.contains("the-worker")).unwrap();
    let id = row.split_whitespace().next().unwrap().to_string();

    // Somebody saves the worker's file: other steps, and how to come to it.
    std::fs::write(file("worker"), TWO).unwrap();
    let listed = rig.run(&["instances"]).await.ok();
    let short = |text: &str| blueprint::revision(text)[..8].to_string();
    assert!(listed.contains(&format!("  active  {} v1 behind  ", short(ONE))), "{listed}");
    let why = "    The text of `worker` now there says how one that is running takes it (`on_update`). It is \
               carried over to that text where it is, with that done in place of its steps, which are not \
               run again, when what started it, or it itself, is applied again.";
    assert!(listed.lines().any(|line| line == why), "{listed}");

    let again = rig.run(&["apply", "--instance", &id]).await.ok();
    assert!(
        again.contains(&format!("instance {id} carried over; run run_")) && again.contains(" ok in "),
        "{again}"
    );
    let listed = rig.run(&["instances"]).await.ok();
    assert!(
        listed.contains(&format!("  active  {} v2  ", short(TWO))) && !listed.contains("behind"),
        "{listed}"
    );
    let one = rig.run(&["instance", &id]).await.ok();
    assert!(
        one.contains("  text ") && one.contains(" v2\n") && one.contains("state {\"heard\":4}"),
        "what it held was carried to the key the file now has:\n{one}"
    );
    assert!(one.contains(" on_update ok\n"), "and the run that did it is one of its runs:\n{one}");
    let claims = rig.run(&["events", "--topics", "shop.claimed,shop.claimed_again"]).await.ok();
    assert!(
        claims.contains("shop.claimed") && !claims.contains("shop.claimed_again"),
        "none of its steps ran again:\n{claims}"
    );

    // Asked for its steps, it is an update like any other.
    let forced = rig.run(&["apply", "--instance", &id, "--force"]).await.ok();
    assert!(forced.contains(&format!("instance {id} updated, its steps run again; run run_")), "{forced}");
}
