//! Actions that come and go while the conductor runs: asked for each time,
//! so they are there from the moment they are brought and gone from the
//! moment they are taken away.

use conductor::RunStatus;
use serde_json::json;

use crate::{Rig, step, user};

const SOURCE: &str = "\
blueprint: borrower
permissions: [lent]
steps:
  first: { do: lent.echo }
on:
  asked: { event: asked.again, do: lent.echo }
outputs:
  lent: ${ steps.first.out.lent }
";

fn listed(rig: &Rig) -> Vec<String> {
    let all = rig.engine.registry().all_specs();
    all.into_iter().map(|spec| spec.name).filter(|name| name.ends_with(".echo")).collect()
}

#[tokio::test]
async fn an_action_that_is_brought_is_there_from_then_on_and_gone_when_it_is_taken_away() {
    let rig = Rig::new().await;

    // Not here: a blueprint that does it is refused, in the words of
    // whoever would bring it.
    assert_eq!(listed(&rig), ["test.echo"]);
    let refused = rig.engine.apply(rig.request(SOURCE, json!({}))).await.unwrap_err();
    assert!(refused.message.contains("`lent.echo` has not been lent"), "{}", refused.message);

    // Brought: listed, checked against like any other, and done.
    *rig.shared.lent.lock().unwrap() = true;
    assert_eq!(listed(&rig), ["test.echo", "lent.echo"]);
    let unlisted = SOURCE.replace("permissions: [lent]", "permissions: [shell]");
    let refused = rig.engine.apply(rig.request(&unlisted, json!({}))).await.unwrap_err();
    assert!(
        refused.message.contains("this uses `lent`, which is not listed under `permissions`"),
        "{}",
        refused.message
    );
    let run = rig.run(SOURCE, json!({})).await;
    assert_eq!((run.status, &run.outputs), (RunStatus::Ok, &json!({ "lent": true })));

    // Taken away: a step of what is still running fails, saying why.
    *rig.shared.lent.lock().unwrap() = false;
    assert_eq!(listed(&rig), ["test.echo"]);
    rig.journal.emit("asked.again", kernel::Source::actor(user().actor()), json!({})).await.unwrap();
    let handled = rig.event("run.finished", |event| event.data["flow"] == "on.asked").await;
    let said = handled.data["error"].as_str().unwrap();
    assert!(said.contains("`lent.echo` was lent and has been taken back"), "{said}");
}

#[tokio::test]
async fn an_action_registered_with_the_conductor_is_never_one_that_comes_and_goes() {
    let rig = Rig::new().await;
    *rig.shared.lent.lock().unwrap() = true;
    assert_eq!(listed(&rig), ["test.echo", "lent.echo"], "it is listed once");
    let run =
        rig.run("blueprint: own\nsteps:\n  say: { do: test.echo, with: { word: mine } }\n", json!({})).await;
    assert_eq!(step(&run, "say").output, json!({ "word": "mine" }), "and the one registered is the one done");
}
