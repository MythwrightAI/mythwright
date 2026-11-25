use std::sync::{Arc, Mutex};

use conductor::{
    ActionCx, ActionResult, Apply, InstanceFilter, InstanceStatus, Remote, RunStatus, StepStatus,
};
use kernel::{BoxFut, Fields, Source};
use serde_json::{Value, json};

use crate::{Rig, step};

const DOUBLER: &str = "\
blueprint: doubler
params: { n: 1 }
steps:
  double: { do: test.echo, with: { v: ${ params.n * 2 } } }
outputs:
  doubled: ${ steps.double.out.v }
";

async fn children(rig: &Rig, parent: &str) -> Vec<conductor::InstanceInfo> {
    let mut children = rig
        .engine
        .instances(InstanceFilter { parent: Some(parent.to_string()), ..Default::default() })
        .await
        .unwrap();
    children.sort_by(|a, b| a.key.cmp(&b.key));
    children
}

#[tokio::test]
async fn call_runs_another_blueprint_and_returns_its_outputs() {
    let rig = Rig::new().await;
    rig.library.put("doubler", DOUBLER);
    let applied = rig
        .apply(
            "\
blueprint: parent
steps:
  kid: { call: doubler, as: the-kid, with: { n: 21 } }
  use: { do: test.echo, with: { got: ${ steps.kid.out.doubled } } }
",
            json!({}),
        )
        .await;
    let run = rig.finished(&applied).await;
    assert_eq!(step(&run, "use").output, json!({ "got": 42 }));

    let kids = children(&rig, &applied.instance).await;
    assert_eq!(kids.len(), 1);
    assert_eq!(
        (kids[0].key.as_str(), kids[0].blueprint.as_str(), kids[0].status),
        ("the-kid", "doubler", InstanceStatus::Done)
    );
    assert_eq!(step(&run, "kid").output["instance"], kids[0].id.as_str());
}

#[tokio::test]
async fn a_failing_callee_fails_the_calling_step_with_its_reason() {
    let rig = Rig::new().await;
    rig.library.put(
        "fragile",
        "blueprint: fragile\nsteps:\n  work: { do: test.flaky, with: { message: no route to host } }\n",
    );
    let run = rig
        .run(
            "\
blueprint: parent
steps:
  kid:
    call: fragile
    on_fail:
      - emit: { topic: parent.noticed, why: ${ self.error } }
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(step(&run, "kid").error.as_deref(), Some("step `work` failed: no route to host"));
    assert_eq!(
        rig.event("parent.noticed", |_| true).await.data["why"],
        "step `work` failed: no route to host"
    );
}

#[tokio::test]
async fn spawn_starts_a_child_without_waiting_and_the_parent_outlives_its_steps() {
    let rig = Rig::new().await;
    rig.library
        .put("worker", "blueprint: worker\nsteps:\n  work: { do: test.gate, with: { name: worker } }\n");
    let applied = rig
        .apply(
            "blueprint: boss\nsteps:\n  hire: { spawn: worker }\n  carry_on: { do: test.echo }\n",
            json!({}),
        )
        .await;
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "the parent did not wait for the worker");

    let kid = &children(&rig, &applied.instance).await[0];
    assert_eq!(kid.status, InstanceStatus::Starting);
    rig.event("instance.active", |event| event.source.instance.as_deref() == Some(applied.instance.as_str()))
        .await;
    assert_eq!(rig.instance(&applied.instance).await.status, InstanceStatus::Active, "it has a live child");

    rig.open("worker");
    rig.event("instance.finished", |event| event.source.instance.as_deref() == Some(kid.id.as_str())).await;
}

#[tokio::test]
async fn a_call_that_is_given_up_on_stops_its_callee() {
    let rig = Rig::new().await;
    rig.library.put("stuck", "blueprint: stuck\nsteps:\n  work: { do: test.gate, with: { name: never } }\n");
    let since = rig.clock.sleep_count();
    let applied =
        rig.apply("blueprint: impatient\nsteps:\n  kid: { call: stuck, timeout: 1m }\n", json!({})).await;
    rig.reached("never").await;
    rig.advance_after(since, 1, "1m").await;
    let run = rig.finished(&applied).await;

    assert_eq!(step(&run, "kid").error.as_deref(), Some("timed out after 1m"));
    let kid = &children(&rig, &applied.instance).await[0];
    assert_eq!(kid.status, InstanceStatus::Stopped);
}

/// Something that lives on once started, whose steps wait at a door
/// where it is told to.
const WATCHED: &str = "\
blueprint: watched
params: { held: false }
steps:
  work: { when: ${ params.held }, do: test.gate, with: { name: door } }
on:
  poke: { event: test.poke, do: noop }
";

/// Something that calls for it as its steps run, and again, told to wait
/// at the door, when it is asked.
const KEEPER: &str = "\
blueprint: keeper
params: { n: 1 }
steps:
  first: { call: watched, as: kid }
on:
  again:
    event: keeper.again
    steps:
      more: { call: watched, as: kid, with: { held: true }, timeout: 1m }
";

/// A keeper whose child is there and at rest, then asked to call for it
/// again: the keeper, and the child as it waits at the door.
async fn kept_and_called_again(rig: &Rig) -> (conductor::Applied, conductor::InstanceInfo) {
    rig.library.put("watched", WATCHED);
    let applied = rig.apply(KEEPER, json!({})).await;
    rig.finished(&applied).await;
    rig.event("instance.active", |event| event.source.instance.as_deref() == Some(applied.instance.as_str()))
        .await;
    rig.engine.emit("keeper.again", Source::default(), json!({})).await;
    rig.reached("door").await;
    let kid = children(rig, &applied.instance).await.remove(0);
    (applied, kid)
}

#[tokio::test]
async fn a_call_that_is_given_up_on_leaves_a_callee_that_was_there_before_it_and_its_steps_go_on() {
    let rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    let (applied, kid) = kept_and_called_again(&rig).await;
    rig.advance_after(since, 1, "1m").await;
    let ended = rig.event("run.finished", |event| event.data["flow"] == "on.again").await;
    assert_eq!(ended.data["error"], "step `more` failed: timed out after 1m");

    // It was not the call's making, and is not the call's to stop.
    assert_eq!(rig.events("instance.stopped").await.len(), 0);
    assert_eq!(rig.instance(&kid.id).await.status, InstanceStatus::Starting);
    // What the call asked for goes on, and it is then as it was asked to be.
    rig.open("door");
    rig.event("instance.active", |event| {
        event.seq > ended.seq && event.source.instance.as_deref() == Some(kid.id.as_str())
    })
    .await;
    let now = children(&rig, &applied.instance).await.remove(0);
    assert_eq!((now.id, now.status, &now.params["held"]), (kid.id, InstanceStatus::Active, &json!(true)));
}

#[tokio::test]
async fn something_applied_over_while_a_handler_of_its_is_in_a_call_keeps_what_was_called_and_all_under_it() {
    let rig = Rig::new().await;
    let (applied, kid) = kept_and_called_again(&rig).await;
    // Its steps run again, which cancels the handler in its call, and
    // call for the child themselves: as it was first asked to be.
    let again = rig.apply(KEEPER, json!({ "n": 2 })).await;
    assert_eq!(rig.finished(&again).await.status, RunStatus::Ok);

    assert_eq!(rig.events("instance.stopped").await.len(), 0, "nothing was stopped for the call being cut");
    let now = children(&rig, &applied.instance).await;
    assert_eq!(now.len(), 1, "and nothing was made in its place");
    assert_eq!(
        (&now[0].id, now[0].status, &now[0].params["held"]),
        (&kid.id, InstanceStatus::Active, &json!(false))
    );
}

#[tokio::test]
async fn for_each_over_a_call_makes_one_child_per_element() {
    let rig = Rig::new().await;
    rig.library.put("doubler", DOUBLER);
    let applied = rig
        .apply(
            "blueprint: many\nsteps:\n  all: { call: doubler, for_each: ${ [1, 2, 3] }, with: { n: ${item} } }\n",
            json!({}),
        )
        .await;
    let run = rig.finished(&applied).await;
    let doubled: Vec<i64> = step(&run, "all")
        .output
        .as_array()
        .unwrap()
        .iter()
        .map(|out| out["doubled"].as_i64().unwrap())
        .collect();
    assert_eq!(doubled, [2, 4, 6]);
    let keys: Vec<String> = children(&rig, &applied.instance).await.into_iter().map(|kid| kid.key).collect();
    assert_eq!(keys, ["all#0", "all#1", "all#2"]);
}

/// Something that lives on once started, and counts that it was.
const KEPT: &str = "\
blueprint: kept
params: { of: none }
steps:
  claim: { do: test.count, with: { key: \"claimed-${ params.of }\" } }
on:
  poke: { event: test.poke, do: noop }
";

/// A child for each of a list, each called what `as` says.
const NAMED: &str = "\
blueprint: board
params: { cards: [a, b] }
steps:
  each: { spawn: kept, for_each: ${ params.cards }, as: \"card-${ item }\", with: { of: ${ item } } }
on:
  poke: { event: test.poke, do: noop }
";

async fn live_keys(rig: &Rig, parent: &str) -> Vec<String> {
    let mut keys: Vec<String> = children(rig, parent)
        .await
        .into_iter()
        .filter(|kid| kid.status.is_live())
        .map(|kid| kid.key)
        .collect();
    keys.sort();
    keys
}

#[tokio::test]
async fn a_child_named_with_as_in_a_for_each_is_called_what_as_says_and_found_again_by_it() {
    let rig = Rig::new().await;
    rig.library.put("kept", KEPT);
    let applied = rig.apply(NAMED, json!({})).await;
    rig.finished(&applied).await;
    assert_eq!(live_keys(&rig, &applied.instance).await, ["card-a", "card-b"], "with no position added");

    // Applied again, with the list in another order and one more: each is
    // the child it was, wherever it now comes, and only the new one starts.
    let again = rig.apply(NAMED, json!({ "cards": ["c", "b", "a"] })).await;
    rig.finished(&again).await;
    assert_eq!(live_keys(&rig, &applied.instance).await, ["card-a", "card-b", "card-c"]);
    let counts = ["claimed-a", "claimed-b", "claimed-c"].map(|key| rig.shared.count(key));
    assert_eq!(counts, [1, 1, 1], "none of those that were there was started again");
}

#[tokio::test]
async fn a_name_that_is_the_same_for_every_element_is_refused_when_checked_and_one_that_comes_out_the_same_when_run()
 {
    let rig = Rig::new().await;
    rig.library.put("kept", KEPT);
    let fixed = NAMED.replace("card-${ item }", "the-card");
    let refused = rig.engine.apply(rig.request(&fixed, json!({}))).await.unwrap_err();
    assert!(
        refused.message.contains(
            "`as: the-card` is the same for every element of `for_each`, so each would be applied over the \
             one before; put what tells them apart in it (`${ item }`, `${ index }`), or leave `as` out and \
             each is named `each#<position>`"
        ),
        "{}",
        refused.message
    );

    // Two elements that the name does not tell apart are found when it runs.
    let run = rig.run(NAMED, json!({ "cards": ["a", "a"] })).await;
    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(
        run.error.as_deref(),
        Some(
            "step `each` failed: `as` came to `card-a` for two elements of `for_each`, which would be one \
             child applied over twice. Put what tells the elements apart in it (`${ item }`, `${ index }`), \
             or leave `as` out and each is named `each#<position>`"
        )
    );
}

#[tokio::test]
async fn children_that_were_named_with_their_position_added_are_put_right_when_their_parent_is_applied_again()
{
    let rig = Rig::new().await;
    rig.library.put("kept", KEPT);
    let applied = rig.apply(NAMED, json!({})).await;
    rig.finished(&applied).await;
    let parent = applied.instance.clone();
    let under = |key: &str, of: &str| Apply {
        key: Some(key.to_string()),
        parent: Some(parent.clone()),
        params: kernel::fields(json!({ "of": of })),
        ..Apply::new("kept", crate::user())
    };
    // As an earlier fault left things: a second instance for `a` under a
    // name with its position added, and `z` under such a name alone.
    let twin = rig.engine.apply(under("card-a#0", "a")).await.unwrap();
    rig.finished(&twin).await;
    let alone = rig.engine.apply(under("card-z#7", "z")).await.unwrap();
    rig.finished(&alone).await;
    // And something that only looks like one: another blueprint's.
    rig.library.put("other", "blueprint: other\non:\n  poke: { event: test.poke, do: noop }\n");
    let unlike = Apply {
        key: Some("card-b#1".into()),
        parent: Some(parent.clone()),
        ..Apply::new("other", crate::user())
    };
    rig.engine.apply(unlike).await.unwrap();
    assert_eq!(live_keys(&rig, &parent).await, ["card-a", "card-a#0", "card-b", "card-b#1", "card-z#7"]);

    let again = rig.apply(NAMED, json!({ "cards": ["a", "b", "z"] })).await;
    rig.finished(&again).await;
    assert_eq!(
        live_keys(&rig, &parent).await,
        ["card-a", "card-b", "card-b#1", "card-z"],
        "the second `a` is stopped, `z` has its name, and what is another blueprint's is left"
    );
    assert_eq!(rig.instance(&twin.instance).await.status, InstanceStatus::Stopped);
    let z = rig.instance(&alone.instance).await;
    assert_eq!((z.key.as_str(), z.status), ("card-z", InstanceStatus::Active), "the same instance, renamed");
    let counts = ["claimed-a", "claimed-b", "claimed-z"].map(|key| rig.shared.count(key));
    assert_eq!(counts, [2, 1, 1], "nothing was started for any of them, and nothing run again");
    let said = rig.engine.step_log(again.run.as_ref().unwrap(), "each", 0, 10);
    let said: Vec<&str> = said.iter().map(|line| line.text.as_str()).collect();
    assert_eq!(
        said,
        [
            "a second instance for `card-a`, started under a name with its position added, was stopped",
            "`card-z` was running under a name with its position added; it is `card-z` again",
        ]
    );

    // Applied once more there is nothing to put right.
    let once_more = Apply { force: true, ..rig.request(NAMED, json!({ "cards": ["a", "b", "z"] })) };
    let run = rig.finished(&rig.engine.apply(once_more).await.unwrap()).await;
    assert!(rig.engine.step_log(&run.id, "each", 0, 10).is_empty());
    assert_eq!(live_keys(&rig, &parent).await, ["card-a", "card-b", "card-b#1", "card-z"]);
}

#[tokio::test]
async fn the_blueprint_to_call_can_be_chosen_by_a_param() {
    let rig = Rig::new().await;
    rig.library.put("doubler", DOUBLER);
    let run = rig
        .run(
            "blueprint: chooser\nparams: { which: doubler }\nsteps:\n  kid: { call: \"${params.which}\", with: { n: 4 } }\n",
            json!({}),
        )
        .await;
    assert_eq!(step(&run, "kid").output["doubled"], 8);
}

#[tokio::test]
async fn calling_a_blueprint_that_does_not_exist_fails_the_step() {
    let rig = Rig::new().await;
    let run = rig.run("blueprint: lost\nsteps:\n  kid: { call: nowhere }\n", json!({})).await;
    assert_eq!(step(&run, "kid").error.as_deref(), Some("there is no blueprint called `nowhere`"));
}

#[tokio::test]
async fn an_attached_blueprint_watches_the_instance_it_is_attached_to() {
    let rig = Rig::new().await;
    rig.library.put(
        "watcher",
        "blueprint: watcher\non:\n  seen:\n    event: thing.done\n    scope: target\n    do: test.count\n    with: { key: seen }\n",
    );
    let applied = rig
        .apply(
            "\
blueprint: host
uses:
  w: watcher
on:
  go: { event: host.go, do: emit, with: { topic: thing.done } }
",
            json!({}),
        )
        .await;
    let watcher = children(&rig, &applied.instance).await.remove(0);
    assert_eq!((watcher.key.as_str(), watcher.target.as_deref()), ("w", Some(applied.instance.as_str())));
    for id in [&watcher.id, &applied.instance] {
        rig.event("instance.active", |event| event.source.instance.as_deref() == Some(id.as_str())).await;
    }

    // From somewhere else entirely: not the watcher's business.
    rig.journal.emit("thing.done", Source::default(), json!({})).await.unwrap();
    // From the host, by way of its own handler.
    rig.journal.emit("host.go", Source::default(), json!({})).await.unwrap();
    rig.event("run.finished", |event| {
        event.source.instance.as_deref() == Some(watcher.id.as_str()) && event.data["flow"] == "on.seen"
    })
    .await;
    assert_eq!(rig.shared.count("seen"), 1);

    // Stopping the host takes its attachment with it.
    rig.engine.stop(&applied.instance).await.unwrap();
    assert_eq!(rig.instance(&watcher.id).await.status, InstanceStatus::Stopped);
}

/// Records what it was asked to run and answers for the far device.
#[derive(Default)]
struct FarDevice {
    asked: Mutex<Vec<(String, String, Fields)>>,
}

impl Remote for FarDevice {
    fn run_action(
        &self,
        device: &str,
        action: &str,
        args: Fields,
        cx: ActionCx,
    ) -> BoxFut<'static, ActionResult> {
        self.asked.lock().unwrap().push((device.to_string(), action.to_string(), args));
        let step = cx.step().to_string();
        Box::pin(async move { Ok(json!({ "ran_on": "builder-1", "step": step })) })
    }
}

#[tokio::test]
async fn a_step_for_another_device_goes_through_the_gateway() {
    let rig = Rig::new().await;
    let source = "\
blueprint: spread
params: { where: builder-1 }
steps:
  far: { do: test.echo, device: \"${params.where}\", with: { v: 1 } }
  near: { do: test.echo, device: test-device, with: { v: 2 } }
";
    // With no gateway installed, the step says so rather than running here.
    let run = rig.run(source, json!({})).await;
    assert_eq!(
        step(&run, "far").error.as_deref(),
        Some("this step is for device `builder-1`, but this device has no gateway to reach it")
    );

    let far = Arc::new(FarDevice::default());
    rig.engine.set_remote(far.clone());
    let run = rig.run(source, json!({})).await;
    assert_eq!(step(&run, "far").output, json!({ "ran_on": "builder-1", "step": "far" }));
    assert_eq!(step(&run, "near").output, json!({ "v": 2 }), "this device's own name means here");
    let asked = far.asked.lock().unwrap().clone();
    assert_eq!(asked.len(), 1);
    assert_eq!((asked[0].0.as_str(), asked[0].1.as_str()), ("builder-1", "test.echo"));
    assert_eq!(Value::Object(asked[0].2.clone()), json!({ "v": 1 }));
    assert_eq!(step(&run, "near").status, StepStatus::Ok);

    // Another device is the fleet's to use: applied by one who may see the
    // fleet and not change it, the step is sent nowhere.
    let scopes = ["blueprints", "fleet:read"].iter().map(|scope| scope.to_string()).collect();
    let looker = kernel::Principal { scopes, name: "Grace".into(), ..crate::user() };
    let request =
        conductor::Apply { principal: looker, key: Some("hers".into()), ..rig.request(source, json!({})) };
    let run = rig.finished(&rig.engine.apply(request).await.unwrap()).await;
    assert_eq!(
        step(&run, "far").error.as_deref(),
        Some("Grace may not send a step to another device (needs `fleet:write`)")
    );
    assert_eq!(far.asked.lock().unwrap().len(), 1, "nothing more was asked of the far device");
}

#[tokio::test]
async fn a_secret_is_named_where_it_is_written_and_never_by_what_an_expression_came_to() {
    let rig = Rig::new().await;
    // What a param carries would otherwise choose which secret a program is handed.
    let source = "\
blueprint: handed
params: { which: { type: any, default: plain } }
steps:
  run: { do: test.echo, env: { TOKEN: \"${params.which}\" }, with: { v: 1 } }
";
    let run = rig.run(source, json!({ "which": "plain" })).await;
    assert_eq!(step(&run, "run").status, StepStatus::Ok);
    let run = rig.run(source, json!({ "which": { "secret": "deploy-key" } })).await;
    let why = step(&run, "run").error.clone().unwrap_or_default();
    assert!(why.contains("an expression came to naming the secret `deploy-key`"), "{why}");
}

#[tokio::test]
async fn a_secret_named_by_the_empty_text_is_none_and_asks_nothing_of_whoever_applied() {
    let rig = Rig::new().await;
    // A blueprint that can be told which key to use, as a fleet can.
    let source = "\
blueprint: told
params: { key: { type: string, default: \"\" } }
steps:
  run: { do: test.echo, env: { TOKEN: { secret: \"${params.key}\" } }, with: { v: 1 } }
";
    let scopes = ["blueprints"].iter().map(|scope| scope.to_string()).collect();
    let plain = kernel::Principal { scopes, name: "Grace".into(), ..crate::user() };
    let by = |params: Value| conductor::Apply {
        principal: plain.clone(),
        force: true,
        ..rig.request(source, params)
    };
    // Told none, nothing is handed, and somebody who may use no secret runs it.
    let run = rig.finished(&rig.engine.apply(by(json!({}))).await.unwrap()).await;
    assert_eq!(step(&run, "run").status, StepStatus::Ok, "{:?}", step(&run, "run").error);
    // Told one, it is theirs to be allowed.
    let run = rig.finished(&rig.engine.apply(by(json!({ "key": "deploy-key" }))).await.unwrap()).await;
    assert_eq!(step(&run, "run").error.as_deref(), Some("Grace may not use `secrets:use`"));
}

#[tokio::test]
async fn an_attachment_does_not_react_to_what_it_causes_itself() {
    let rig = Rig::new().await;
    // An attachment that answers every `thing.done` in its target with a
    // `thing.done` of its own. Left unchecked, that never ends.
    rig.library.put(
        "echoer",
        "blueprint: echoer\non:\n  seen:\n    event: thing.done\n    scope: target\n    steps:\n      count: { do: test.count, with: { key: echoes } }\n      again: { do: emit, with: { topic: thing.done } }\n",
    );
    let applied = rig
        .apply("blueprint: host\nuses:\n  e: echoer\non:\n  go: { event: host.go, do: emit, with: { topic: thing.done } }\n", json!({}))
        .await;
    let echoer = children(&rig, &applied.instance).await.remove(0);
    for id in [&echoer.id, &applied.instance] {
        rig.event("instance.active", |event| event.source.instance.as_deref() == Some(id.as_str())).await;
    }
    rig.journal.emit("host.go", Source::default(), json!({})).await.unwrap();
    rig.event("run.finished", |event| {
        event.source.instance.as_deref() == Some(echoer.id.as_str()) && event.data["flow"] == "on.seen"
    })
    .await;
    // A second event from the host is handled once more, and by then the
    // attachment's own event has been seen and passed over.
    rig.journal.emit("host.go", Source::default(), json!({})).await.unwrap();
    let mut finished = rig.follow("run.finished");
    let mut handled = 0;
    while handled < 2 {
        let event = rig.soon("the echoer's runs", finished.next()).await.unwrap();
        let mine = event.source.instance.as_deref() == Some(echoer.id.as_str());
        handled += usize::from(mine && event.data["flow"] == "on.seen");
    }
    assert_eq!(rig.shared.count("echoes"), 2, "one per event from the host, none for its own");
}
