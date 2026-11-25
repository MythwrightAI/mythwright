//! A small text of each shape, run in a sandbox: its steps, a handler
//! said an event, a wait and the clock moved on, a stand-in that waits
//! and one that answers at once, a failure, a child called, and the
//! tables' own actions over the sandbox's journal.

use sandbox::{Answers, Limits};
use serde_json::json;

use crate::{Rig, steps, topics};

#[tokio::test]
async fn a_text_applied_in_a_sandbox_runs_its_steps_and_hands_back_its_run_with_each_step_in_the_order_it_began()
 {
    let rig = Rig::new(
        "blueprint: greets\nsteps:\n  first:\n    description: Say hello. And more that is not said.\n    do: log\n    with: { message: hello }\n  second: { do: set, with: { said: true } }\n  third: { do: noop }\nstate:\n  said: false\n",
    )
    .await;

    let did = rig.apply().await;

    assert_eq!(did["did"]["applied"]["created"], true, "{did}");
    assert_eq!(did["applying"], false);
    let shown = rig.read().await;
    assert_eq!(steps(&shown), ["first: ok", "second: ok", "third: ok"], "{shown}");
    let run = &shown["runs"][0];
    assert_eq!(run["flow"], "main");
    assert_eq!(run["status"], "ok");
    let ran: Vec<&str> =
        run["steps"].as_array().unwrap().iter().map(|step| step["step"].as_str().unwrap()).collect();
    assert_eq!(ran, ["first", "second", "third"]);
    // A step is said by what it says of itself, as far as its first full stop.
    let first = shown["timeline"].as_array().unwrap().iter().find(|line| line["step"] == "first").unwrap();
    assert_eq!(first["says"], "Say hello.");
    assert_eq!(first["does"], "do log");
    assert_eq!(first["handed"]["message"], "hello", "{first}");
    assert_eq!(shown["instances"][0]["project"], "sandbox");
    assert_eq!(shown["instances"][0]["state"]["said"], true);
    // And all of it says what it was, and what was stood in, in sentences
    // of the sandbox's own; and what can be done to it by hand.
    assert!(shown["said"].as_str().unwrap().starts_with("This was a sandbox test"), "{shown}");
    assert!(shown["stood_said"].as_str().unwrap().starts_with("Nothing was stood in"), "{shown}");
    assert_eq!(shown["stood"], json!([]));
    assert_eq!(shown["does"].as_array().unwrap().len(), 9, "{}", shown["does"]);
}

#[tokio::test]
async fn a_handler_hears_an_event_said_to_the_sandbox_and_what_it_carries() {
    let rig = Rig::new(
        "blueprint: hears\nsteps:\n  ready: { do: noop }\non:\n  rung:\n    event: bell.rung\n    steps:\n      note: { do: set, with: { last: \"${event.data.by}\" } }\nstate:\n  last: nobody\n",
    )
    .await;
    rig.apply().await;

    let said = rig.act(json!({ "say": { "topic": "bell.rung", "data": { "by": "ada" } } })).await;

    assert_eq!(said["did"]["said"], "bell.rung");
    let shown = rig.read().await;
    assert_eq!(shown["instances"][0]["state"]["last"], "ada", "{shown}");
    assert!(topics(&shown).contains(&"bell.rung".to_string()));
    let runs: Vec<&str> =
        shown["runs"].as_array().unwrap().iter().map(|run| run["flow"].as_str().unwrap()).collect();
    assert_eq!(runs, ["main", "on.rung"], "the runs are in the order they began");
    assert_eq!(shown["runs"][1]["event"]["topic"], "bell.rung");
}

#[tokio::test]
async fn a_wait_goes_on_only_when_the_sandbox_s_clock_is_moved_on() {
    let rig = Rig::new(
        "blueprint: waits\nsteps:\n  pause: { do: wait, with: { for: 5m } }\n  after: { do: set, with: { woke: true } }\nstate:\n  woke: false\n",
    )
    .await;

    let did = rig.apply().await;

    // At rest with the wait under way: nothing moves the clock but an act.
    assert_eq!(did["applying"], false, "the apply answers once its run has begun: {did}");
    let before = rig.read().await;
    assert_eq!(before["instances"][0]["state"]["woke"], false, "{before}");
    assert_eq!(steps(&before), ["pause: running"]);

    rig.act(json!({ "advance": { "by": "4m" } })).await;
    assert_eq!(rig.read().await["instances"][0]["state"]["woke"], false);
    let moved = rig.act(json!({ "advance": { "by": "2m" } })).await;

    assert_eq!(moved["clock"].as_i64().unwrap() - before["clock"].as_i64().unwrap(), 360_000);
    let shown = rig.read().await;
    assert_eq!(shown["instances"][0]["state"]["woke"], true, "{shown}");
    assert_eq!(steps(&shown), ["pause: ok", "after: ok"]);
}

#[tokio::test]
async fn a_stand_in_waits_until_it_is_answered_and_its_step_gives_what_it_was_answered() {
    let rig = Rig::new(
        "blueprint: asks\nsteps:\n  ask: { do: outside.ask, with: { prompt: \"which way?\" } }\n  keep: { do: set, with: { way: \"${steps.ask.out.reply}\" } }\nstate:\n  way: none\n",
    )
    .await;

    let did = rig.apply().await;

    let waiting = did["waiting"].as_array().unwrap();
    assert_eq!(waiting.len(), 1, "{did}");
    assert_eq!(waiting[0]["action"], "outside.ask");
    assert_eq!(waiting[0]["step"], "ask");
    assert_eq!(waiting[0]["asked"]["prompt"], "which way?");
    assert_eq!(waiting[0]["summary"], "Ask somebody outside the factory.");
    let shown = rig.read().await;
    assert_eq!(steps(&shown), ["ask: running"], "{shown}");

    let answered =
        rig.act(json!({ "answer": { "asked": waiting[0]["id"], "gives": { "reply": "left" } } })).await;

    assert_eq!(answered["waiting"], json!([]));
    let shown = rig.read().await;
    assert_eq!(shown["instances"][0]["state"]["way"], "left", "{shown}");
    assert_eq!(shown["asked"][0]["answered"]["reply"], "left");
    assert_eq!(shown["asked"][0]["by"], "answered");
    // The step's line says which asking was stood in for it.
    let ask = shown["timeline"].as_array().unwrap().iter().find(|line| line["step"] == "ask").unwrap();
    assert_eq!(ask["stood"], json!([1]));
    // Answered again, nothing waits by that number.
    let again = rig.refused(json!({ "answer": { "asked": 1, "gives": {} } })).await;
    assert!(again.contains("no stand-in numbered 1 is waiting"), "{again}");
}

#[tokio::test]
async fn a_stand_in_told_to_fails_its_step_with_the_words_it_was_given() {
    let rig = Rig::new(
        "blueprint: asks\nsteps:\n  ask: { do: outside.ask, with: { prompt: \"anybody?\" } }\n  after: { do: noop }\n",
    )
    .await;
    rig.apply().await;

    rig.act(json!({ "answer": { "asked": 1, "fails": "nobody was there" } })).await;

    let shown = rig.read().await;
    assert_eq!(steps(&shown), ["ask: failed", "after: cancelled"], "{shown}");
    assert_eq!(shown["runs"][0]["status"], "failed");
    assert!(shown["runs"][0]["error"].as_str().unwrap().contains("nobody was there"), "{shown}");
    assert_eq!(shown["instances"][0]["status"], "failed");
    assert_eq!(shown["asked"][0]["failed"], "nobody was there");
}

#[tokio::test]
async fn a_stand_in_answers_what_its_description_says_at_once_where_the_sandbox_was_told_to() {
    let text = "blueprint: runs\nsteps:\n  build: { do: outside.run, with: { cmd: [make, all] } }\n  ask: { do: outside.ask, with: { prompt: \"well?\" } }\n  keep: { do: set, with: { code: \"${steps.build.out.code}\", reply: \"${steps.ask.out.reply ?? 'nothing'}\" } }\nstate:\n  code: -1\n  reply: unset\n";
    let rig = Rig::with(text, Answers::Default, Limits::default(), &[]).await;

    let did = rig.apply().await;

    assert_eq!(did["waiting"], json!([]), "{did}");
    let shown = rig.read().await;
    assert_eq!(steps(&shown), ["build: ok", "ask: ok", "keep: ok"], "{shown}");
    // One that says what a stand-in answers gave that; one that says nothing, nothing.
    assert_eq!(shown["instances"][0]["state"]["code"], 0);
    assert_eq!(shown["instances"][0]["state"]["reply"], "nothing");
    assert_eq!(shown["asked"][0]["asked"]["cmd"], json!(["make", "all"]));
    assert_eq!(shown["asked"][0]["by"], "usual");
    assert_eq!(shown["asked"][1]["answered"], json!({}));
    // What was stood in is counted by action and said in a sentence.
    let stood = json!([{ "action": "outside.ask", "times": 1 }, { "action": "outside.run", "times": 1 }]);
    assert_eq!(shown["stood"], stood);
    assert_eq!(
        shown["stood_said"],
        "Stood in, and so not exercised: `outside.ask`, `outside.run`. Asked 2 times in all, they ran nothing, \
         and answered what a stand-in was told to answer."
    );
}

#[tokio::test]
async fn stand_ins_that_wait_are_each_answered_their_usual_when_the_sandbox_is_told_to_answer_at_once() {
    let rig = Rig::new(
        "blueprint: runs\nsteps:\n  build: { do: outside.run, with: { cmd: [make] } }\n  test: { do: outside.run, with: { cmd: [make, test] } }\n",
    )
    .await;
    let did = rig.apply().await;
    assert_eq!(did["waiting"].as_array().unwrap().len(), 1);

    let told = rig.act(json!({ "answers": { "answers": "default" } })).await;

    assert_eq!(told["waiting"], json!([]));
    assert_eq!(steps(&rig.read().await), ["build: ok", "test: ok"]);
}

#[tokio::test]
async fn a_step_that_fails_says_why_and_what_is_done_when_it_fails_is_done() {
    let rig = Rig::new(
        "blueprint: fails\nsteps:\n  broken: { do: fail, with: { message: \"it is broken\" } }\n  never: { do: noop }\non_fail:\n  - { do: emit, with: { topic: fails.noticed, data: { what: broken } } }\n",
    )
    .await;

    rig.apply().await;

    let shown = rig.read().await;
    assert_eq!(steps(&shown), ["broken: failed", "never: cancelled"], "{shown}");
    let broken = shown["timeline"].as_array().unwrap().iter().find(|line| line["step"] == "broken").unwrap();
    assert!(broken["error"].as_str().unwrap().contains("it is broken"), "{broken}");
    assert!(topics(&shown).contains(&"fails.noticed".to_string()), "{:?}", topics(&shown));
}

#[tokio::test]
async fn a_child_called_runs_in_the_sandbox_from_the_texts_it_was_handed_and_gives_back_what_it_gives() {
    let child = "blueprint: helper\nparams:\n  word: { type: string }\nsteps:\n  shout: { do: noop }\noutputs:\n  loud: \"${params.word}!\"\n";
    let rig = Rig::with(
        "blueprint: parent\nsteps:\n  ask: { call: helper, with: { word: hey } }\n  keep: { do: set, with: { heard: \"${steps.ask.out.loud}\" } }\nstate:\n  heard: nothing\n",
        Answers::Asked,
        Limits::default(),
        &[("helper", child)],
    )
    .await;

    rig.apply().await;

    let shown = rig.read().await;
    let instances = shown["instances"].as_array().unwrap();
    assert_eq!(instances.len(), 2, "{shown}");
    let parent = instances.iter().find(|one| one["blueprint"] == "parent").unwrap();
    assert_eq!(parent["state"]["heard"], "hey!", "{shown}");
    let helper = instances.iter().find(|one| one["blueprint"] == "helper").unwrap();
    assert_eq!(helper["parent"], parent["id"]);
    // What nobody handed the sandbox is not there to be called, as anywhere.
    let lost = Rig::new("blueprint: lost\nsteps:\n  ask: { call: nowhere/to-be-found }\n").await;
    lost.apply().await;
    let shown = lost.read().await;
    assert!(shown["runs"][0]["error"].as_str().unwrap().contains("nowhere/to-be-found"), "{shown}");
}

#[tokio::test]
async fn a_board_that_ships_is_declared_in_the_sandbox_and_a_card_added_by_hand_is_heard_there() {
    let hears = "blueprint: hears\nsteps:\n  board: { call: std/board }\non:\n  added:\n    event: board.item.created\n    steps:\n      note: { do: set, with: { card: \"${event.data.id}\" } }\n  moved:\n    event: board.item.moved\n    steps:\n      note: { do: set, with: { to: \"${event.data.to}\" } }\nstate:\n  card: none\n  to: nowhere\n";
    let rig = Rig::new(hears).await;
    rig.apply().await;

    let added = rig.act(json!({ "do": { "action": "board.add", "with": { "title": "Try it" } } })).await;
    let card = added["did"]["id"].as_str().unwrap().to_string();
    rig.act(json!({ "do": { "action": "board.move", "with": { "id": card, "to": "doing" } } })).await;

    let shown = rig.read().await;
    let hears =
        shown["instances"].as_array().unwrap().iter().find(|one| one["blueprint"] == "hears").unwrap();
    assert_eq!(hears["state"]["card"], card.as_str(), "{shown}");
    assert_eq!(hears["state"]["to"], "doing");
    assert_eq!(card, "sandbox-1", "a card of the sandbox's one project");
    let board = shown["tables"].as_array().unwrap().iter().find(|table| table["name"] == "board").unwrap();
    assert_eq!(board["records"][0]["data"]["title"], "Try it");
    assert_eq!(board["records"][0]["data"]["status"], "doing");
    // What is done by hand is one of a few things, each called as its action is.
    let other = rig.refused(json!({ "do": { "action": "fs.write", "with": {} } })).await;
    assert!(other.contains("is not something done to a sandbox's tables by hand"), "{other}");
}

#[tokio::test]
async fn an_event_that_is_described_is_held_to_what_it_carries_when_it_is_said_by_hand() {
    let rig = Rig::new("blueprint: quiet\nsteps:\n  ready: { do: noop }\n").await;
    rig.apply().await;

    let refused = rig.refused(json!({ "say": { "topic": "run.finished", "data": { "flow": 3 } } })).await;

    assert!(refused.contains("`run.finished`"), "{refused}");
    assert!(refused.contains("Nothing was said"), "{refused}");
    let listed = rig.refused(json!({ "say": { "topic": "bell.rung", "data": [1, 2] } })).await;
    assert!(listed.contains("named fields"), "{listed}");
}

#[tokio::test]
async fn an_act_is_answered_only_when_everything_that_came_of_it_has_come_to_its_end() {
    // A chain: each handler hears what the one before said and says the
    // next, each in a run of its own, with a child called at its end.
    let mut text =
        "blueprint: chain\nsteps:\n  first: { do: emit, with: { topic: chain.said, at: 0 } }\non:\n"
            .to_string();
    for at in 0..12 {
        text.push_str(&format!(
            "  link{at}:\n    event: chain.said\n    for: {{ at: {at} }}\n    steps:\n      card: {{ do: record.add, with: {{ collection: links, fields: {{ title: Link {at} }} }} }}\n      next: {{ do: emit, with: {{ topic: chain.said, at: {} }} }}\n",
            at + 1
        ));
    }
    text.push_str("  last:\n    event: chain.said\n    for: { at: 12 }\n    steps:\n      done: { do: set, with: { done: true } }\nstate:\n  done: false\n");
    let links = "blueprint: links\nsteps:\n  table: { do: collection.ensure, with: { name: links, fields: { title: { type: text } } } }\n";
    let rig = Rig::with(&text, sandbox::Answers::Asked, Limits::default(), &[("links", links)]).await;
    rig.act(json!({ "apply": { "blueprint": "links" } })).await;
    rig.apply().await;

    // The first saying was said before anything was listening: said
    // again by hand, the whole chain has run when the act is answered.
    let said = rig.act(json!({ "say": { "topic": "chain.said", "data": { "at": 0 } } })).await;

    assert_eq!(said["waiting"], json!([]));
    let shown = rig.read().await;
    let chain =
        shown["instances"].as_array().unwrap().iter().find(|one| one["blueprint"] == "chain").unwrap();
    assert_eq!(
        chain["state"]["done"], true,
        "the act was answered in the middle of what came of it: {shown}"
    );
    let table = shown["tables"].as_array().unwrap().iter().find(|table| table["name"] == "links").unwrap();
    assert_eq!(table["records"].as_array().unwrap().len(), 12);
    let under_way: Vec<&serde_json::Value> =
        shown["runs"].as_array().unwrap().iter().filter(|run| run["finished"].is_null()).collect();
    assert!(under_way.is_empty(), "{under_way:?}");
}

#[tokio::test]
async fn what_an_action_says_of_a_table_for_a_step_is_put_with_the_run_it_was_said_for() {
    // The studio says a table was declared as whoever applied the text,
    // naming no run; a row added says which step added it.
    let rig = Rig::new(
        "blueprint: tabled\npermissions: [projects, records]\nsteps:\n  notes:\n    do: collection.ensure\n    with: { name: notes, fields: { title: { type: text } } }\non:\n  rung:\n    event: bell.rung\n    steps:\n      add: { do: record.add, with: { collection: notes, fields: { title: Rung } } }\n",
    )
    .await;
    rig.apply().await;
    rig.act(json!({ "say": { "topic": "bell.rung" } })).await;

    let shown = rig.read().await;
    let lines = shown["timeline"].as_array().unwrap();
    let at = |what: &dyn Fn(&serde_json::Value) -> bool| {
        lines.iter().position(what).unwrap_or_else(|| panic!("no such line: {shown}"))
    };
    let run_of = |flow: &str| {
        let runs = shown["runs"].as_array().unwrap();
        runs.iter().find(|run| run["flow"] == flow).unwrap()["id"].clone()
    };
    // Each is under the run whose step said it, before that step's own
    // line, and names that run's instance.
    let declared = at(&|line| line["topic"] == "collection.ensured");
    assert_eq!(lines[declared]["run"], run_of("main"), "{}", lines[declared]);
    assert_eq!(lines[declared]["instance"], shown["instances"][0]["id"]);
    assert!(declared < at(&|line| line["what"] == "step" && line["step"] == "notes"));
    let added = at(&|line| line["topic"] == "record.created");
    assert_eq!(lines[added]["run"], run_of("on.rung"), "{}", lines[added]);
    assert!(added < at(&|line| line["what"] == "step" && line["step"] == "add"));
    // What was said by hand is nobody's run's.
    let rung = at(&|line| line["topic"] == "bell.rung");
    assert!(lines[rung]["run"].is_null(), "{}", lines[rung]);
}

#[tokio::test]
async fn what_a_step_tells_somebody_is_a_line_of_the_made_up_person_s_inbox_and_a_handle_names_nobody_there()
{
    let rig = Rig::new(
        "blueprint: tells\npermissions: [inbox]\nsteps:\n  told:\n    do: inbox.tell\n    with: { to: applier, says: Look at this, why: asked }\n  nobody:\n    do: inbox.tell\n    allow_failure: true\n    with: { to: \"@grace\", says: And you }\n",
    )
    .await;

    rig.apply().await;

    let shown = rig.read().await;
    assert_eq!(steps(&shown), ["told: ok", "nobody: failed"], "{shown}");
    // The one inbox there is, is that of whoever applied the text.
    let lines = shown["inbox"].as_array().unwrap();
    assert_eq!(lines.len(), 1, "{shown}");
    assert_eq!(lines[0]["says"], "Look at this");
    assert_eq!(lines[0]["why"], "asked");
    assert_eq!(lines[0]["owner"], shown["somebody"]);
    assert_eq!(shown["actions"]["inbox.tell"], "kept");
    assert_eq!(shown["actions"]["tells.whom"], "own");
    // A handle names an account of the sandbox's own, and there is one.
    let nobody = shown["timeline"].as_array().unwrap().iter().find(|line| line["step"] == "nobody").unwrap();
    assert!(nobody["error"].as_str().unwrap().contains("grace"), "{nobody}");
}

#[tokio::test]
async fn of_two_runs_at_work_at_once_what_is_said_is_put_with_the_one_whose_action_says_such_things() {
    // A child that declares a table is started, and its parent goes
    // straight on to make a channel: the two actions are at work at
    // once. What is said of the channel names no instance, and is the
    // parent's by what its action is; what is said of the table names
    // the child that owns it.
    let child = "blueprint: maker\npermissions: [projects]\nsteps:\n  notes:\n    do: collection.ensure\n    with: { name: notes, fields: { title: { type: text } } }\n";
    let rig = Rig::with(
        "blueprint: both\npermissions: [projects, channels, blueprints]\nsteps:\n  maker: { spawn: maker, as: maker }\n  room: { do: channel.ensure, with: { name: talk } }\n",
        Answers::Asked,
        Limits::default(),
        &[("maker", child)],
    )
    .await;

    rig.apply().await;

    let shown = rig.read().await;
    let lines = shown["timeline"].as_array().unwrap();
    let run_of = |blueprint: &str| {
        let instances = shown["instances"].as_array().unwrap();
        let instance = &instances.iter().find(|one| one["blueprint"] == blueprint).unwrap()["id"];
        shown["runs"].as_array().unwrap().iter().find(|run| &run["instance"] == instance).unwrap()["id"]
            .clone()
    };
    let said = |topic: &str| {
        lines.iter().find(|line| line["topic"] == topic).unwrap_or_else(|| panic!("`{topic}` in {shown}"))
    };
    assert_eq!(said("channel.created")["run"], run_of("both"), "{}", said("channel.created"));
    assert_eq!(said("channel.joined")["run"], run_of("both"), "{}", said("channel.joined"));
    assert_eq!(said("collection.ensured")["run"], run_of("maker"), "{}", said("collection.ensured"));
}

#[tokio::test]
async fn a_channel_made_by_hand_before_a_text_is_applied_is_the_one_its_steps_say_things_in() {
    let rig = Rig::new(
        "blueprint: says\npermissions: [channels:write]\nsteps:\n  said: { do: channel.post, with: { channel: ops, body: Built. } }\n",
    )
    .await;

    // A text that says things in a channel nobody made fails there; made by hand first, it is said there.
    let made = rig.act(json!({ "do": { "action": "channel.ensure", "with": { "name": "ops" } } })).await;
    assert_eq!(made["did"]["name"], "ops", "{made}");
    rig.apply().await;

    let shown = rig.read().await;
    assert_eq!(steps(&shown), ["said: ok"], "{shown}");
    let ops = shown["channels"].as_array().unwrap().iter().find(|channel| channel["name"] == "ops").unwrap();
    assert_eq!(ops["messages"][0]["body"], "Built.", "{ops}");
    // Asked again it is the same channel, as the action is: nothing is made twice.
    rig.act(json!({ "do": { "action": "channel.ensure", "with": { "name": "ops" } } })).await;
    assert_eq!(rig.read().await["channels"].as_array().unwrap().len(), 1);
}
