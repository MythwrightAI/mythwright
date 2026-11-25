//! Playing a blueprint to see what it does with what it is given: a
//! factory started to say so writes down what each step was handed, and a
//! daemon can be stopped as a step somebody names begins.

use std::time::Duration;

use reqwest::Method;
use serde_json::{Value, json};

use crate::{Rig, soon};

/// A blueprint that is handed a name, works something out of it, and
/// hands that on to another.
const GREETER: &str = "\
blueprint: greeter
params:
  who: { default: Ada }
state:
  greeting: \"\"
steps:
  greet: { do: set, with: { greeting: \"hello, ${ params.who }\" } }
  child:
    needs: [greet]
    call: hello
    with: { who: \"${ params.who }\", said: \"${ state.greeting }\" }
";

const HELLO: &str = "\
blueprint: hello
params:
  who: {}
  said: {}
steps:
  tell: { do: emit, with: { topic: hello.said, to: \"${ params.who }\", text: \"${ params.said }\" } }
";

/// One that calls a blueprint whose instances keep nothing as they go.
const CALLER: &str = "\
blueprint: caller
steps:
  child: { call: quiet, with: { who: Grace } }
";

const QUIET: &str = "\
blueprint: quiet
durable: false
params:
  who: {}
steps:
  tell: { do: emit, with: { topic: hello.said, to: \"${ params.who }\" } }
";

/// A node with a project that has `hello` as a blueprint of its own.
async fn rig(handed: bool) -> Rig {
    let rig = Rig::owned(Rig::with("api-played", |config, _| config.handed = handed).await).await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let file = json!({ "path": format!("@blueprints/{}", brand::blueprint_file("hello")), "content": HELLO });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(file)).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{answer}");
    let file = json!({ "path": format!("@blueprints/{}", brand::blueprint_file("quiet")), "content": QUIET });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(file)).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{answer}");
    rig
}

/// What the factory itself wrote in a step's log, a line each.
async fn said_of(rig: &Rig, run: &str, step: &str) -> Vec<String> {
    let log = rig.read(&format!("/api/runs/{run}/steps/{step}/log")).await;
    let lines = log["lines"].as_array().cloned().unwrap_or_default();
    lines
        .iter()
        .filter(|line| line["stream"] == "s")
        .map(|line| line["text"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// Plays the greeter for Grace, and gives back the run of its steps.
async fn played(rig: &Rig) -> String {
    let asked = json!({ "source": GREETER, "project": "shop", "params": { "who": "Grace" }, "wait": true });
    let applied = rig.send("/api/apply", asked).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let instance = applied["instance"].as_str().unwrap().to_string();
    let runs = rig.read(&format!("/api/runs?instance={instance}&limit=10")).await;
    runs["runs"][0]["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn a_factory_started_to_say_so_writes_what_each_step_was_handed_at_the_head_of_its_log() {
    let rig = rig(true).await;
    let run = played(&rig).await;

    // What `${ }` came to, and not what was written.
    assert_eq!(said_of(&rig, &run, "greet").await, [r#"handed: {"greeting":"hello, Grace"}"#]);
    // What was handed on to another blueprint, with which and as what: the
    // param, and what an earlier step had worked out.
    assert_eq!(
        said_of(&rig, &run, "child").await,
        [r#"handed to hello as child: {"said":"hello, Grace","who":"Grace"}"#]
    );
    // And the blueprint that was handed it says what its own step was.
    let told = rig.event("hello.said", |_| true).await;
    let theirs = told.source.run.clone().unwrap();
    assert_eq!(
        said_of(&rig, &theirs, "tell").await,
        [r#"handed: {"text":"hello, Grace","to":"Grace","topic":"hello.said"}"#]
    );
}

#[tokio::test]
async fn a_factory_that_was_not_asked_writes_nothing_of_what_a_step_was_handed() {
    let rig = rig(false).await;
    let run = played(&rig).await;

    assert_eq!(said_of(&rig, &run, "greet").await, Vec::<String>::new());
    assert_eq!(said_of(&rig, &run, "child").await, Vec::<String>::new());
}

#[tokio::test]
async fn a_daemon_asked_to_stop_at_a_step_keeps_that_steps_beginning_and_nothing_after_and_goes_on_from_there()
 {
    let mut rig = rig(false).await;
    let answer = rig.send("/api/system/stop", json!({ "at": "tell", "of": "hello" })).await;
    assert_eq!(answer, json!({ "stopping": false, "at": "tell", "of": "hello" }));
    let asked = rig.event("system.stopping", |_| true).await;
    assert_eq!(asked.data, json!({ "at": "tell", "of": "hello" }));

    // Nothing stops until such a step begins: the greeter's own steps are
    // done, and the one it hands on to is cut off as its step begins.
    let greeter = json!({ "source": GREETER, "project": "shop", "params": { "who": "Grace" } });
    played_until_it_stops(&rig, greeter).await;

    // Started again on what it had, the step is taken up where it was cut
    // off and the play comes to its end.
    rig.restart().await;
    let last = kept_before_it_stopped(&rig).await;
    assert_eq!(
        (last["topic"].as_str(), last["step"].as_str()),
        (Some("step.started"), Some("tell")),
        "{last}"
    );
    let tell = |event: &kernel::Event| event.source.step.as_deref() == Some("tell");
    let again = rig.event("step.started", |event| tell(event) && event.data["resumed"] == true).await;
    assert!(again.seq > 0);
    let told = rig.event("hello.said", |_| true).await;
    assert_eq!(told.data["text"], "hello, Grace");
    // The greeter, which was waiting on what it called, ends its own steps.
    let greeter =
        rig.event("run.finished", |event| event.data["blueprint"] == "greeter" && event.seq > told.seq).await;
    assert_eq!(greeter.data["status"], "ok");
}

#[tokio::test]
async fn a_daemon_is_stopped_at_a_step_of_an_instance_that_keeps_nothing_and_what_called_it_begins_it_again()
{
    let mut rig = rig(false).await;
    rig.send("/api/system/stop", json!({ "at": "tell", "of": "quiet" })).await;
    played_until_it_stops(&rig, json!({ "source": CALLER, "project": "shop" })).await;

    // The step's beginning was passed on and not kept, and nothing was
    // kept after it. The instance that kept nothing is ended, and its
    // caller asks again.
    rig.restart().await;
    assert_eq!(kept_before_it_stopped(&rig).await["topic"], "run.started");
    let ended = rig.event("instance.finished", |event| event.data["status"] == "stopped").await;
    let told = rig.event("hello.said", |_| true).await;
    assert!(told.seq > ended.seq && told.source.instance != ended.source.instance);
    let caller =
        rig.event("run.finished", |event| event.data["blueprint"] == "caller" && event.seq > told.seq).await;
    assert_eq!(caller.data["status"], "ok");
    assert_eq!(rig.count_of("hello.said").await, 1);
}

#[tokio::test]
async fn a_daemon_asked_to_stop_at_a_step_that_never_begins_goes_on_and_stops_when_it_is_asked_plainly() {
    let rig = rig(false).await;
    rig.send("/api/system/stop", json!({ "at": "nowhere" })).await;
    let run = played(&rig).await;
    assert!(!run.is_empty());
    assert!(!rig.node().app.journal.is_shut());

    let stop = {
        let app = rig.node().app.clone();
        tokio::spawn(async move { app.stop.notified().await })
    };
    // The spawned wait has to be listening before the daemon is asked.
    tokio::task::yield_now().await;
    assert_eq!(rig.send("/api/system/stop", json!({})).await, json!({ "stopping": true }));
    tokio::time::timeout(Duration::from_secs(30), stop).await.expect("the daemon is told to stop").unwrap();
}

/// Applies what is asked and waits for the daemon to be told to stop at
/// the step it was told. Whoever applied may never be answered: a daemon
/// cut off at a step answers nobody after it.
async fn played_until_it_stops(rig: &Rig, asked: Value) {
    let app = rig.node().app.clone();
    let stop = tokio::spawn(async move { app.stop.notified().await });
    soon("the daemon to be told to stop", async {
        let applying = rig.post("/api/apply", asked);
        tokio::pin!(applying, stop);
        let mut answered = false;
        loop {
            tokio::select! {
                _ = &mut applying, if !answered => answered = true,
                told = &mut stop => break told.unwrap(),
            }
        }
    })
    .await;
}

/// The last thing the journal kept before the daemon said it had stopped:
/// its topic and the step that said it. A journal that has shut answers
/// nothing until its daemon stops, so this is read once it is back.
async fn kept_before_it_stopped(rig: &Rig) -> Value {
    let stopped = rig.event("system.stopped", |_| true).await;
    let query = journal::EventQuery { limit: 10_000, ..Default::default() };
    let events = rig.said(query).await;
    // Nothing the play said after the place was kept.
    assert!(!events.iter().any(|event| event.topic == "hello.said" && event.seq < stopped.seq));
    let last = events.iter().rfind(|event| event.seq < stopped.seq).unwrap();
    json!({ "topic": last.topic, "step": last.source.step })
}
