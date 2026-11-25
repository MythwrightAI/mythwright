use std::collections::BTreeMap;
use std::time::Duration;

use conductor::{InstanceStatus, RunStatus, StepStatus};
use serde_json::json;

use crate::{Rig, ada, soon};

async fn shop(rig: &Rig) {
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    rig.studio.channels.create("shop", Some("shop"), "", &ada()).await.unwrap();
}

fn step<'a>(run: &'a conductor::RunInfo, id: &str) -> &'a conductor::StepInfo {
    run.steps.iter().find(|step| step.step == id).unwrap()
}

#[tokio::test]
async fn a_blueprint_brings_an_agent_in_and_takes_it_away_when_it_stops() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let applied = rig
        .apply(
            "blueprint: crew\npermissions: [agents]\nsteps:\n  hire: { do: agent.ensure, with: { name: builder, channel: shop, adapter: fake } }\n",
            json!({}),
        )
        .await;
    let run = soon("the run", rig.engine.wait_run(applied.run.as_ref().unwrap())).await.unwrap();
    assert_eq!(step(&run, "hire").output, json!({ "name": "builder", "adapter": "fake", "project": "shop" }));

    let slot = rig.agents.slot("builder").await.unwrap();
    assert_eq!(slot.project.as_deref(), Some("shop"), "it works in the project the blueprint was applied in");
    let instance = rig.engine.instance(&applied.instance).await.unwrap();
    assert_eq!(
        (instance.status, instance.holds.as_slice()),
        (InstanceStatus::Active, &["agent:builder".to_string()][..])
    );

    // It answers in its channel while the fleet is up...
    rig.say("hello").await;
    rig.turns_finished(1).await;
    // ...and stops when the fleet does.
    rig.engine.stop(&applied.instance).await.unwrap();
    assert!(!rig.agents.slot("builder").await.unwrap().active);
}

#[tokio::test]
async fn a_blueprint_gives_an_agent_work_and_uses_what_it_says() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    rig.fake.will_say("1. a carts table 2. an API");
    rig.fake.will_say("Built both.");
    let run = rig
        .run(
            "\
blueprint: work
permissions: [agents]
steps:
  plan: { do: agent.task, with: { slot: builder, prompt: Plan the cart. } }
  build: { do: agent.task, with: { prompt: \"Now build: ${steps.plan.out.reply}\" } }
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Ok);
    assert_eq!(step(&run, "build").output["reply"], "Built both.");
    assert_eq!(
        step(&run, "build").output["slot"],
        "builder",
        "with no slot named, the project's agent is used"
    );

    let seen = rig.fake.seen.lock().unwrap().clone();
    assert_eq!(seen[1].prompt, "Now build: 1. a carts table 2. an API");
    assert_eq!(seen[1].session.as_deref(), Some("session-1"), "steps of one instance are one conversation");
    assert_eq!(
        rig.studio.channels.messages(&crate::own(), "shop", None, 10).await.unwrap().len(),
        0,
        "work for a blueprint is not chatter"
    );
}

#[tokio::test]
async fn an_agent_says_what_it_did_in_the_channel_its_task_names_and_in_its_own_name() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    rig.fake.will_say("Built the cart.");
    let run = rig
        .run(
            "\
blueprint: work
permissions: [agents, channels]
steps:
  build: { do: agent.task, with: { slot: builder, channel: shop, prompt: Build the cart. } }
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(step(&run, "build").output["reply"], "Built the cart.", "and the blueprint still has it");
    let said = rig.studio.channels.messages(&crate::own(), "shop", None, 10).await.unwrap();
    let said: Vec<(&str, &str)> =
        said.iter().map(|message| (message.author.as_str(), message.body.as_str())).collect();
    assert_eq!(said, [("agent:builder", "Built the cart.")]);

    // A channel that is not there is said of the step, before the agent
    // is given anything to do.
    let run = rig
        .run(
            "\
blueprint: lost
permissions: [agents, channels]
steps:
  build: { do: agent.task, with: { slot: builder, channel: nowhere, prompt: Build the cart. } }
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Failed);
    assert!(
        run.error.as_deref().unwrap_or_default().contains("there is no channel `nowhere`"),
        "{:?}",
        run.error
    );
    assert_eq!(rig.fake.seen.lock().unwrap().len(), 1, "the agent was not asked a second time");
}

#[tokio::test]
async fn work_that_may_end_more_than_one_way_says_which_and_what_the_agent_said_is_kept_without_the_choice() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    rig.fake.will_say("shop-1: Which cart: the one in the header, or the page?\n**DECISION: ask**");
    rig.fake.will_say("Built the cart.");
    let run = rig
        .run(
            "\
blueprint: work
permissions: [agents, channels]
steps:
  first: { do: agent.task, with: { slot: builder, channel: shop, options: [done, ask], prompt: Build the cart. } }
  again: { do: agent.task, with: { slot: builder, options: [done, ask], prompt: The one in the header. }, needs: [first] }
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    let asked = rig.fake.seen.lock().unwrap()[0].prompt.clone();
    assert!(
        asked.starts_with("Build the cart.") && asked.ends_with("DECISION: <done | ask>"),
        "the agent is told how to say which: {asked}"
    );

    // The choice is the blueprint's to read, and is not part of what was said.
    let first = &step(&run, "first").output;
    assert_eq!(first["decision"], "ask");
    assert_eq!(first["reply"], "shop-1: Which cart: the one in the header, or the page?");
    let said = rig.studio.channels.messages(&crate::own(), "shop", None, 10).await.unwrap();
    let said: Vec<(&str, &str)> =
        said.iter().map(|message| (message.author.as_str(), message.body.as_str())).collect();
    assert_eq!(said, [("agent:builder", "shop-1: Which cart: the one in the header, or the page?")]);

    // Work whose agent chose nothing is not failed for it, as a consult is:
    // there is no choice, and the blueprint takes that as it likes.
    let again = &step(&run, "again").output;
    assert_eq!((again["decision"].is_null(), again["reply"].as_str()), (true, Some("Built the cart.")));
}

#[tokio::test]
async fn a_task_given_a_directory_is_done_there_and_a_conversation_is_carried_on_only_where_it_was_held() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    let shop = rig.studio.projects.dir_of("shop").unwrap();
    std::fs::create_dir_all(shop.join(".lanes/shop-1")).unwrap();
    for reply in ["Started.", "Carried on.", "Somewhere else."] {
        rig.fake.will_say(reply);
    }
    let run = rig
        .run(
            "\
blueprint: rounds
permissions: [agents]
steps:
  first: { do: agent.task, with: { slot: builder, thread: \"lane:shop-1\", cwd: .lanes/shop-1, prompt: Take the card. } }
  again: { do: agent.task, with: { slot: builder, thread: \"lane:shop-1\", cwd: .lanes/shop-1, prompt: It was sent back. } }
  beside: { do: agent.task, with: { slot: builder, thread: \"lane:shop-1\", prompt: And in the project itself? } }
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);

    let seen = rig.fake.seen.lock().unwrap().clone();
    // The folder as the product says a whole path: plainly, on every system.
    let lane = kernel::whole::whole(&shop.join(".lanes/shop-1")).unwrap();
    assert_eq!((&seen[0].workdir, &seen[1].workdir), (&lane, &lane), "the program is run in the directory");
    assert!(
        seen[0].system.contains("Your working directory is `.lanes/shop-1`, in the project `shop`."),
        "and the agent is told where it is: {}",
        seen[0].system
    );
    assert_eq!(seen[0].session, None);
    assert_eq!(seen[1].session.as_deref(), Some("session-1"), "the second round carries the first on");
    // The same thread, somewhere else: what was said in the lane's
    // directory is kept by the program under that directory.
    assert_eq!(seen[2].workdir, shop);
    assert_eq!(seen[2].session, None, "a conversation is not carried on where it was not held");
}

#[tokio::test]
async fn an_agent_consulted_about_a_failure_can_fix_it_and_send_the_step_round_again() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    rig.fake.will_say("The semicolon was missing; I added it.\nDECISION: retry");
    let run = rig
        .run(
            "\
blueprint: heal
permissions: [agents]
steps:
  build:
    do: test.flaky
    with: { times: 1 }
    on_fail:
      - do: agent.consult
        with:
          prompt: \"The build failed: ${self.error}. Fix it if you can.\"
          options: [retry, fail]
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Ok);
    assert_eq!(step(&run, "build").attempt, 2);

    let asked = rig.fake.prompts().remove(0);
    assert!(asked.starts_with("The build failed: not yet. Fix it if you can."), "{asked}");
    assert!(asked.ends_with("DECISION: <retry | fail>"), "the agent is told how to answer: {asked}");
    let turn = rig.agents.turns("builder", 1).await.unwrap().remove(0);
    assert_eq!((turn.kind.as_str(), turn.decision.as_deref()), ("consult", Some("retry")));
}

#[tokio::test]
async fn a_step_can_say_which_agent_to_ask_when_it_fails_and_what_it_fails_with_otherwise() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    let source = |times: u32| {
        format!(
            "blueprint: heal\npermissions: [agents]\nsteps:\n  build:\n    do: test.flaky\n    with: {{ times: {times} }}\n    if_fail_ask_agent: \"@builder\"\n    else: build-broken\n"
        )
    };

    // The agent puts it right and sends the step round again.
    rig.fake.will_say("The semicolon was missing; I added it.\nDECISION: retry");
    let run = rig.run(&source(1), json!({})).await;
    assert_eq!((run.status, step(&run, "build").attempt), (RunStatus::Ok, 2));
    let asked = rig.fake.prompts().remove(0);
    assert!(
        asked.starts_with(
            "The step `build` failed while running `test.flaky`:\nnot yet\n\nPut it right if you can."
        ),
        "{asked}"
    );
    assert_eq!(rig.fake.seen.lock().unwrap()[0].slot, "builder", "the agent that was named is the one asked");

    // It cannot: the step fails, under the name it was given.
    rig.fake.will_say("This needs a person.\nDECISION: fail");
    let run = rig.run(&source(9), json!({})).await;
    assert_eq!(run.status, RunStatus::Failed);
    let build = step(&run, "build");
    assert_eq!(build.output["code"], "build-broken");
    let failed = rig.event("step.failed", |event| event.data["code"] == "build-broken").await;
    assert_eq!(failed.data["error"], "not yet");
}

#[tokio::test]
async fn an_agent_that_says_fail_or_does_not_choose_leaves_the_step_failed() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    let source = "\
blueprint: heal
permissions: [agents]
steps:
  build:
    do: test.flaky
    with: { times: 9 }
    on_fail:
      - do: agent.consult
        with: { prompt: \"It failed: ${self.error}\", options: [retry, fail] }
";
    rig.fake.will_say("This needs a person.\n**DECISION: Fail.**");
    let run = rig.run(source, json!({})).await;
    assert_eq!((run.status, step(&run, "build").attempt), (RunStatus::Failed, 1));
    assert_eq!(
        rig.agents.turns("builder", 1).await.unwrap()[0].decision.as_deref(),
        Some("fail"),
        "formatting around the answer is forgiven"
    );

    // No decision at all: the hook is reported, and the failure stands.
    rig.fake.will_say("I am not sure what to do.");
    let run = rig.run(source, json!({})).await;
    assert_eq!(run.status, RunStatus::Failed);
    let reported = rig.event("hook.failed", |_| true).await;
    assert_eq!(reported.data["error"], "builder did not decide: it did not choose one of: retry, fail");
}

#[tokio::test]
async fn consulting_when_there_is_no_agent_fails_unless_that_is_allowed() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let source = "\
blueprint: lonely
permissions: [agents]
params: { optional: false }
steps:
  ask:
    do: agent.consult
    with: { prompt: Anyone there?, options: [yes, no], if_available: ${params.optional} }
  after: { do: noop, when: ${ !steps.ask.out.available } }
";
    let run = rig.run(source, json!({})).await;
    assert_eq!(
        step(&run, "ask").error.as_deref(),
        Some("no agent is available; bring one in with `agent.ensure`, or pass `if_available: true`")
    );

    let run = rig.run(source, json!({ "optional": true })).await;
    assert_eq!(step(&run, "ask").output, json!({ "available": false, "reply": "", "decision": null }));
    assert_eq!(
        step(&run, "after").status,
        StepStatus::Ok,
        "the blueprint carries on and can tell nobody was there"
    );
}

#[tokio::test]
async fn a_step_that_gives_up_on_an_agent_stops_its_turn() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    let since = rig.clock.sleep_count();
    let applied = rig
        .apply(
            "blueprint: impatient\npermissions: [agents]\nsteps:\n  ask: { do: agent.task, timeout: 1m, with: { prompt: \"think [gate:never]\" } }\n",
            json!({}),
        )
        .await;
    rig.fake.saw(1).await;
    soon("the timeout to be set", rig.clock.sleeps_started(since + 1)).await;
    rig.clock.advance(Duration::from_secs(60));

    let run = soon("the run", rig.engine.wait_run(applied.run.as_ref().unwrap())).await.unwrap();
    assert_eq!(step(&run, "ask").error.as_deref(), Some("timed out after 1m"));
    let finished = rig.turns_finished(1).await;
    assert_eq!(finished[0].data["error"], "the turn was cancelled", "the agent was not left thinking");
}

#[tokio::test]
async fn the_builtin_assistant_refuses_work_it_cannot_do_rather_than_pretending() {
    let rig = Rig::new().await;
    rig.agent("helper", "builtin").await;
    rig.studio.board.add("shop", "Add a cart", "Customers need one.", None, vec![], &ada()).await.unwrap();
    // A work prompt that happens to contain the words of a request it knows.
    let run = rig
        .run(
            "\
blueprint: handoff
permissions: [agents]
steps:
  work: { do: agent.task, with: { prompt: \"Please take card shop-1: Add a cart\" } }
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Failed);
    let error = step(&run, "work").error.clone().unwrap();
    assert!(
        error.starts_with("helper could not do it: the built-in assistant cannot do this work"),
        "{error}"
    );
    // Nothing that is running brought it up, so it is said again by hand.
    assert!(error.ends_with(&format!("`{} agent add helper --adapter <package>`", brand::CLI)), "{error}");
    assert_eq!(
        rig.studio.board.list("shop").await.unwrap().len(),
        1,
        "it did not add a card called `Add a cart`"
    );
}

/// How often `name` was measured for each combination of the labels in
/// `by`, as `label=value` words.
async fn measured(rig: &Rig, name: &str, by: &[&str]) -> Vec<(String, u64)> {
    let query = journal::MetricQuery {
        name: name.into(),
        since_ms: 0,
        group_by: by.iter().map(|key| key.to_string()).collect(),
        ..Default::default()
    };
    let series = rig.journal.query_metrics(query).await.unwrap();
    let words = |labels: &BTreeMap<String, String>| {
        labels.iter().map(|(name, value)| format!("{name}={value}")).collect::<Vec<_>>().join(" ")
    };
    series.iter().map(|one| (words(&one.labels), one.total.count)).collect()
}

#[tokio::test]
async fn a_turn_an_agent_takes_for_a_labelled_step_is_measured_with_the_labels() {
    let mut rig = Rig::new().await;
    rig.agent("ada", "fake").await;
    rig.agent("grace", "fake").await;
    // One in a channel first, which is for no step and so under no label.
    rig.say("@ada hello").await;
    rig.turns_finished(1).await;

    rig.fake.will_say("Built it.");
    rig.fake.will_say("Built it another way.");
    rig.fake.will_say("It holds.\nDECISION: ship");
    rig.apply(
        "\
blueprint: trial
permissions: [agents]
steps:
  a:
    labels: { experiment: cart, variant: a }
    task:
      work: { do: agent.task, with: { slot: ada, prompt: Build the cart. } }
  b:
    labels: { experiment: cart, variant: b }
    task:
      work: { do: agent.task, with: { slot: grace, prompt: Build the cart. } }
      sure: { do: agent.consult, with: { slot: grace, prompt: Does it hold?, options: [ship, hold] } }
      last: { do: agent.task, with: { slot: grace, prompt: \"Say so. [gate:said]\" } }
",
        json!({}),
    )
    .await;
    // The last of them is under way when everything stops. The turn is
    // taken again when it starts again, and what it is measured with was
    // kept with it.
    rig.fake.saw(5).await;
    rig.restart().await;
    rig.fake.saw(6).await;
    rig.fake.open("said");
    rig.turns_finished(5).await;

    assert_eq!(
        measured(&rig, "agent.turn", &["slot", "kind", "experiment", "variant"]).await,
        [
            ("experiment=cart kind=consult slot=grace variant=b".to_string(), 1),
            ("experiment=cart kind=task slot=ada variant=a".to_string(), 1),
            ("experiment=cart kind=task slot=grace variant=b".to_string(), 2),
            ("kind=message slot=ada".to_string(), 1),
        ]
    );
    // What each lot cost is told apart the same way.
    assert_eq!(
        measured(&rig, "agent.cost_usd", &["variant"]).await,
        [("".to_string(), 1), ("variant=a".to_string(), 1), ("variant=b".to_string(), 3)]
    );
    let turns = rig.agents.turns("grace", 1).await.unwrap();
    assert_eq!(json!(turns[0].labels), json!({ "experiment": "cart", "variant": "b" }));
}
