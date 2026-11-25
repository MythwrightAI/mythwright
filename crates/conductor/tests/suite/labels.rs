//! `labels:` on a `call`, a `spawn` or a task: everything measured under
//! the step carries them, so that two ways of doing the same work can be
//! told apart.

use std::collections::BTreeMap;
use std::time::Duration;

use conductor::{Apply, InstanceFilter, RunStatus};
use journal::MetricQuery;
use kernel::Source;
use serde_json::json;

use crate::handlers::{active, handled};
use crate::{Rig, step};

/// One card's worth of work, which stays to hear of its card afterwards.
const LANE: &str = "\
blueprint: lane
params: { item: \"\" }
steps:
  work: { do: test.echo, measure: build }
  report: { do: test.echo }
on:
  moved:
    event: card.moved
    for: { id: \"${params.item}\" }
    steps:
      tidy: { do: test.echo }
";

/// The same work given out twice, each lot said to be a variant.
const TRIAL: &str = "\
blueprint: trial
params: { item: 12 }
steps:
  card: { do: test.echo }
  a:
    call: lane
    as: lane-a
    labels: { experiment: \"cart-${params.item}\", variant: a }
    with: { item: one }
  b:
    needs: [card]
    call: lane
    as: lane-b
    labels: { experiment: \"cart-${params.item}\", variant: b }
    with: { item: two }
";

const LEAF: &str = "blueprint: leaf\nsteps:\n  work: { do: test.echo }\n";

/// How often `name` was measured, for each combination of the labels in
/// `by`, among what carries all of `only`. A label a measurement does not
/// have is not in its row.
async fn measured(
    rig: &Rig,
    name: &str,
    by: &[&str],
    only: &[(&str, &str)],
) -> Vec<(BTreeMap<String, String>, u64)> {
    let query = MetricQuery {
        name: name.into(),
        since_ms: 0,
        group_by: by.iter().map(|key| key.to_string()).collect(),
        labels: only.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect(),
        ..Default::default()
    };
    let series = rig.journal.query_metrics(query).await.unwrap();
    series.into_iter().map(|series| (series.labels, series.total.count)).collect()
}

fn row(labels: &[(&str, &str)], count: u64) -> (BTreeMap<String, String>, u64) {
    (labels.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect(), count)
}

async fn child(rig: &Rig, parent: &str, key: &str) -> conductor::InstanceInfo {
    let children = rig
        .engine
        .instances(InstanceFilter { parent: Some(parent.to_string()), ..Default::default() })
        .await
        .unwrap();
    children.into_iter().find(|child| child.key == key).unwrap_or_else(|| panic!("no child `{key}`"))
}

#[tokio::test]
async fn everything_a_called_blueprint_does_is_measured_with_the_labels_its_call_gave_it() {
    let rig = Rig::new().await;
    rig.library.put("lane", LANE);
    let applied = rig.apply(TRIAL, json!({})).await;
    assert_eq!(rig.finished(&applied).await.status, RunStatus::Ok);

    assert_eq!(
        measured(&rig, "step", &["blueprint", "variant"], &[]).await,
        [
            row(&[("blueprint", "lane"), ("variant", "a")], 2),
            row(&[("blueprint", "lane"), ("variant", "b")], 2),
            // Its own steps, the two calls among them, are under no label.
            row(&[("blueprint", "trial")], 3),
        ]
    );
    assert_eq!(
        measured(&rig, "run", &["variant"], &[]).await,
        [row(&[], 1), row(&[("variant", "a")], 1), row(&[("variant", "b")], 1)]
    );
    assert_eq!(
        measured(&rig, "build", &["variant"], &[("experiment", "cart-12")]).await,
        [row(&[("variant", "a")], 1), row(&[("variant", "b")], 1)],
        "what a step is also measured as, by `measure:`, carries them too"
    );
    assert_eq!(measured(&rig, "step", &[], &[("experiment", "cart-12")]).await, [row(&[], 4)]);
    assert_eq!(measured(&rig, "step", &[], &[("experiment", "cart-13")]).await, []);

    // They are the instance's, and say so.
    let lane = child(&rig, &applied.instance, "lane-a").await;
    assert_eq!(json!(lane.labels), json!({ "experiment": "cart-12", "variant": "a" }));
    assert!(rig.instance(&applied.instance).await.labels.is_empty());
}

#[tokio::test]
async fn a_handler_that_runs_long_after_the_call_finished_is_measured_with_its_labels() {
    let rig = Rig::new().await;
    rig.library.put("lane", LANE);
    let applied = rig.apply(TRIAL, json!({})).await;
    rig.finished(&applied).await;
    active(&rig, &child(&rig, &applied.instance, "lane-a").await.id).await;

    rig.clock.advance(Duration::from_secs(3 * 3600));
    rig.journal.emit("card.moved", Source::default(), json!({ "id": "one" })).await.unwrap();
    handled(&rig, "on.moved", 1).await;

    assert_eq!(
        measured(&rig, "run", &["flow", "variant"], &[("experiment", "cart-12")]).await,
        [
            row(&[("flow", "main"), ("variant", "a")], 1),
            row(&[("flow", "main"), ("variant", "b")], 1),
            row(&[("flow", "on.moved"), ("variant", "a")], 1),
        ]
    );
    assert_eq!(
        measured(&rig, "step", &["experiment", "variant"], &[("step", "tidy")]).await,
        [row(&[("experiment", "cart-12"), ("variant", "a")], 1)]
    );
}

#[tokio::test]
async fn labels_are_inherited_by_what_a_child_starts_and_its_own_add_to_them() {
    let rig = Rig::new().await;
    rig.library.put("leaf", LEAF);
    rig.library.put(
        "middle",
        "\
blueprint: middle
steps:
  own: { do: test.echo }
  deep: { call: leaf, labels: { variant: inner, depth: 2 } }
  plain: { call: leaf, as: other-leaf }
",
    );
    let applied = rig
        .apply(
            "blueprint: outer\nsteps:\n  mid: { call: middle, labels: { experiment: x, variant: outer } }\n",
            json!({}),
        )
        .await;
    assert_eq!(rig.finished(&applied).await.status, RunStatus::Ok);

    assert_eq!(
        measured(&rig, "step", &["blueprint", "experiment", "variant", "depth"], &[]).await,
        [
            // The grandchild its step said more about: what was added, and
            // on the same name what was said nearest to it.
            row(&[("blueprint", "leaf"), ("depth", "2"), ("experiment", "x"), ("variant", "inner")], 1),
            // The one it said nothing about: what its parent was under.
            row(&[("blueprint", "leaf"), ("experiment", "x"), ("variant", "outer")], 1),
            row(&[("blueprint", "middle"), ("experiment", "x"), ("variant", "outer")], 3),
            row(&[("blueprint", "outer")], 1),
        ]
    );
    let middle = child(&rig, &applied.instance, "mid").await;
    assert_eq!(
        json!(child(&rig, &middle.id, "deep").await.labels),
        json!({ "depth": "2", "experiment": "x", "variant": "inner" })
    );
}

#[tokio::test]
async fn each_blueprint_started_for_one_of_a_list_can_be_labelled_with_which() {
    let rig = Rig::new().await;
    rig.library.put("leaf", LEAF);
    let applied = rig
        .apply(
            "\
blueprint: fan
params: { ways: [round-robin, least-busy] }
steps:
  started:
    for_each: ${ params.ways }
    spawn: leaf
    labels: { variant: \"${item}\", round: \"${index}\" }
",
            json!({}),
        )
        .await;
    rig.finished(&applied).await;
    handled(&rig, "main", 3).await;

    assert_eq!(
        measured(&rig, "step", &["variant", "round"], &[("blueprint", "leaf")]).await,
        [
            row(&[("round", "0"), ("variant", "round-robin")], 1),
            row(&[("round", "1"), ("variant", "least-busy")], 1)
        ]
    );
}

#[tokio::test]
async fn a_task_with_labels_labels_the_steps_inside_it_and_not_the_ones_outside() {
    let rig = Rig::new().await;
    rig.library.put("leaf", LEAF);
    let run = rig
        .run(
            "\
blueprint: release
steps:
  before: { do: test.echo }
  build:
    labels: { variant: fast }
    task:
      compile: { do: test.echo, measure: build }
      checks:
        labels: { part: tests }
        task:
          unit: { do: test.echo }
      pack: { call: leaf }
  after: { do: test.echo }
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Ok);

    assert_eq!(
        measured(&rig, "step", &["step", "variant", "part"], &[]).await,
        [
            // A task inside it labels its own steps, under the outer one's.
            row(&[("part", "tests"), ("step", "build.checks.unit"), ("variant", "fast")], 1),
            row(&[("step", "after")], 1),
            row(&[("step", "before")], 1),
            // The task is a step of the blueprint, beside the others.
            row(&[("step", "build")], 1),
            // A task inside it is one of its steps.
            row(&[("step", "build.checks"), ("variant", "fast")], 1),
            row(&[("step", "build.compile"), ("variant", "fast")], 1),
            row(&[("step", "build.pack"), ("variant", "fast")], 1),
            // What a step inside it calls is under it too.
            row(&[("step", "work"), ("variant", "fast")], 1),
        ]
    );
    assert_eq!(measured(&rig, "build", &["variant"], &[]).await, [row(&[("variant", "fast")], 1)]);
    assert_eq!(measured(&rig, "run", &["blueprint", "variant"], &[]).await.len(), 2);
    assert_eq!(
        measured(&rig, "run", &["variant"], &[("blueprint", "release")]).await,
        [row(&[], 1)],
        "the run the task is in is not the task's"
    );
}

#[tokio::test]
async fn labels_survive_a_restart_of_the_conductor() {
    let mut rig = Rig::new().await;
    rig.library.put("lane", LANE);
    let applied = rig.apply(TRIAL, json!({})).await;
    rig.finished(&applied).await;
    let lane = child(&rig, &applied.instance, "lane-b").await;
    active(&rig, &lane.id).await;

    rig.restart().await;
    assert_eq!(
        json!(rig.instance(&lane.id).await.labels),
        json!({ "experiment": "cart-12", "variant": "b" }),
        "they are kept with the instance"
    );
    rig.journal.emit("card.moved", Source::default(), json!({ "id": "two" })).await.unwrap();
    handled(&rig, "on.moved", 1).await;
    assert_eq!(
        measured(&rig, "step", &["experiment", "variant"], &[("step", "tidy")]).await,
        [row(&[("experiment", "cart-12"), ("variant", "b")], 1)],
        "and what it does after the restart is measured with them"
    );
}

#[tokio::test]
async fn a_task_picked_up_after_a_restart_still_labels_its_steps() {
    let mut rig = Rig::new().await;
    let applied = rig
        .apply(
            "\
blueprint: release
params: { way: slow }
steps:
  build:
    labels: { variant: \"${params.way}\" }
    task:
      compile: { do: test.durable, with: { name: compiling } }
      test: { do: test.echo }
",
            json!({}),
        )
        .await;
    rig.reached("compiling").await;

    rig.restart().await;
    rig.open("compiling");
    assert_eq!(rig.finished(&applied).await.status, RunStatus::Ok);
    assert_eq!(
        measured(&rig, "step", &["step", "variant"], &[]).await,
        [
            row(&[("step", "build")], 1),
            row(&[("step", "build.compile"), ("variant", "slow")], 1),
            row(&[("step", "build.test"), ("variant", "slow")], 1),
        ]
    );
}

#[tokio::test]
async fn an_instance_applied_again_takes_the_labels_it_is_applied_with() {
    let rig = Rig::new().await;
    rig.library.put("lane", LANE);
    let source = "\
blueprint: trial
params: { way: a }
steps:
  lane: { call: lane, labels: { variant: \"${params.way}\" }, with: { item: one } }
";
    let first = rig.apply(source, json!({})).await;
    rig.finished(&first).await;
    let lane = child(&rig, &first.instance, "lane").await;
    active(&rig, &lane.id).await;

    // Nothing about the lane itself changed, so its steps do not run again;
    // what it is measured as from here on did.
    let second = rig.apply(source, json!({ "way": "b" })).await;
    rig.finished(&second).await;
    assert_eq!(json!(rig.instance(&lane.id).await.labels), json!({ "variant": "b" }));
    rig.journal.emit("card.moved", Source::default(), json!({ "id": "one" })).await.unwrap();
    handled(&rig, "on.moved", 1).await;
    assert_eq!(
        measured(&rig, "run", &["flow", "variant"], &[("blueprint", "lane")]).await,
        [row(&[("flow", "main"), ("variant", "a")], 1), row(&[("flow", "on.moved"), ("variant", "b")], 1)]
    );
}

#[tokio::test]
async fn a_blueprint_applied_directly_has_the_labels_it_was_applied_with() {
    let rig = Rig::new().await;
    let labels = [("experiment".to_string(), "by-hand".to_string())].into();
    let request = Apply { labels, ..rig.request(LEAF, json!({})) };
    let applied = rig.engine.apply(request).await.unwrap();
    rig.finished(&applied).await;
    assert_eq!(measured(&rig, "step", &["experiment"], &[]).await, [row(&[("experiment", "by-hand")], 1)]);

    let plain = rig.apply("blueprint: other\nsteps:\n  work: { do: test.echo }\n", json!({})).await;
    rig.finished(&plain).await;
    assert_eq!(
        measured(&rig, "step", &["experiment"], &[("blueprint", "other")]).await,
        [row(&[], 1)],
        "and has none when it was applied with none"
    );
}

#[tokio::test]
async fn a_label_that_comes_out_as_neither_text_nor_a_number_fails_the_step_that_said_it() {
    let rig = Rig::new().await;
    rig.library.put("leaf", LEAF);
    let run = rig
        .run(
            "\
blueprint: trial
params: { agents: [ada, grace] }
steps:
  lane: { call: leaf, labels: { variant: \"${params.agents}\" } }
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(
        step(&run, "lane").error.as_deref(),
        Some(
            "`labels` could not be filled in: the label `variant` must be text or a number, and came \
             out as a list"
        )
    );
    assert_eq!(measured(&rig, "step", &[], &[("blueprint", "leaf")]).await, [], "nothing was started");
}

#[tokio::test]
async fn what_an_instance_asks_the_outside_world_is_measured_with_its_labels() {
    let rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    let source = "\
blueprint: upstream
on:
  moved:
    poll: { every: 30s, do: test.count, with: { key: polls } }
    do: test.echo
";
    let labels = [("variant".to_string(), "b".to_string())].into();
    let applied = rig.engine.apply(Apply { labels, ..rig.request(source, json!({})) }).await.unwrap();
    active(&rig, &applied.instance).await;
    // The first reading is taken as it begins to listen; a poller adds up
    // to a tenth of its interval as jitter to the next.
    rig.advance_after(since, 1, "33s").await;
    handled(&rig, "on.moved", 1).await;

    assert_eq!(
        measured(&rig, "poll", &["handler", "variant"], &[]).await,
        [row(&[("handler", "moved"), ("variant", "b")], 2)]
    );
    // How the conductor hands events out is its own business, and is not
    // measured as anybody's variant.
    assert!(measured(&rig, "handler.*", &["variant"], &[]).await.iter().all(|(labels, _)| labels.is_empty()));
}

#[tokio::test]
async fn a_blueprint_measures_something_of_its_own_and_it_is_kept_with_the_labels_it_runs_under() {
    let rig = Rig::new().await;
    let source = "\
blueprint: shop
on:
  paid:
    event: shop.paid
    do: measure
    with:
      name: shop.order
      value: ${ event.data.total }
      ok: ${ event.data.total > 0 }
      labels: { plan: \"${ event.data.plan }\" }
";
    let labels = BTreeMap::from([("variant".to_string(), "b".to_string())]);
    let applied = rig.engine.apply(Apply { labels, ..rig.request(source, json!({})) }).await.unwrap();
    crate::handlers::active(&rig, &applied.instance).await;
    for (total, plan) in [(40, "pro"), (15, "free"), (0, "free")] {
        let said = json!({ "total": total, "plan": plan });
        rig.journal.emit("shop.paid", Source::default(), said).await.unwrap();
    }
    crate::handlers::handled(&rig, "on.paid", 3).await;
    rig.journal.flush_metrics().await.unwrap();

    assert_eq!(
        measured(&rig, "shop.order", &["plan", "variant"], &[]).await,
        [row(&[("plan", "free"), ("variant", "b")], 2), row(&[("plan", "pro"), ("variant", "b")], 1)]
    );
    let query = MetricQuery { name: "shop.order".into(), since_ms: 0, ..Default::default() };
    let all = &rig.journal.query_metrics(query).await.unwrap()[0].total;
    assert_eq!((all.count, all.errors, all.max), (3, 1, 40.0), "{all:?}");

    // What the system measures under is not a blueprint's to say.
    let wrong = "blueprint: wrong\nsteps:\n  count: { do: measure, with: { name: step.count } }\n";
    let run = rig.run(wrong, json!({})).await;
    assert_eq!(
        crate::step(&run, "count").error.as_deref(),
        Some(
            "`step.count` is a name the system measures under; call your own something of your own, such as `checkout.done`"
        )
    );
}
