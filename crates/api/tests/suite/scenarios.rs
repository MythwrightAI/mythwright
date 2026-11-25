//! Each word shown working comes to what its sentence says.
//!
//! A scenario is an example run in a sandbox with a few things done to
//! it, and a sentence that says what to notice in what came of it
//! (`blueprint::vocabulary::SCENARIOS`, handed out by
//! `/api/blueprints/language/scenarios`). Each is run here as the
//! Reference runs it, by the routes, and what its sentence says is held
//! by a few checks of the sandbox's own answer: a scenario with no checks
//! here fails, and so does a check with no scenario.

use serde_json::{Value, json};

use crate::Rig;

/// What came of one scenario: the sandbox's answer once its last act is at rest.
struct Came(Value);

impl Came {
    fn timeline(&self) -> &[Value] {
        self.0["timeline"].as_array().map(Vec::as_slice).unwrap_or_default()
    }

    /// Where the first line that holds `test` is in the order it happened, if one does.
    fn at(&self, test: impl Fn(&Value) -> bool) -> Option<usize> {
        self.timeline().iter().position(test)
    }

    /// Where the last line that holds `test` is.
    fn last(&self, test: impl Fn(&Value) -> bool) -> Option<usize> {
        self.timeline().iter().rposition(test)
    }

    /// How many times `topic` was said.
    fn said(&self, topic: &str) -> usize {
        self.timeline().iter().filter(|line| line["what"] == "said" && line["topic"] == topic).count()
    }

    /// How many runs of `flow` were begun.
    fn runs(&self, flow: &str) -> usize {
        self.timeline()
            .iter()
            .filter(|line| line["what"] == "run" && line["did"] == "started" && line["flow"] == flow)
            .count()
    }

    /// The lines of each step called `step`, in the order they happened.
    fn steps(&self, step: &str) -> Vec<&Value> {
        self.timeline().iter().filter(|line| line["what"] == "step" && line["step"] == step).collect()
    }

    /// What was said in the channel called `name`.
    fn told(&self, name: &str) -> Vec<String> {
        self.0["channels"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|channel| channel["name"] == name)
            .flat_map(|channel| channel["messages"].as_array().cloned().unwrap_or_default())
            .map(|message| message["body"].as_str().unwrap_or_default().to_string())
            .collect()
    }

    /// The instance whose key is `key`.
    fn instance(&self, key: &str) -> &Value {
        self.0["instances"]
            .as_array()
            .and_then(|all| all.iter().find(|one| one["key"] == key))
            .unwrap_or(&Value::Null)
    }

    /// What each stand-in was asked, in the order it was asked.
    fn asked(&self) -> &[Value] {
        self.0["asked"].as_array().map(Vec::as_slice).unwrap_or_default()
    }
}

/// What each scenario's sentence says, as checks of what came of it.
fn holds(name: &str, came: &Came) -> Result<(), String> {
    let sure = |holds: bool, what: &str| if holds { Ok(()) } else { Err(what.to_string()) };
    match name {
        "handler" => {
            sure(came.said("order.shipped") == 2, "`order.shipped` was said twice")?;
            sure(came.runs("on.shipped") == 1, "its handler ran once, for the first")?;
            sure(came.told("general") == ["Order 7 is on its way to Ada."], "the first was said in general")?;
            sure(came.instance("shipped-news")["status"] == "active", "it stayed, listening")
        }
        "for" => {
            sure(came.said("order.shipped") == 2, "both orders were said to be shipped")?;
            sure(came.runs("on.shipped") == 1, "its handler was woken once")?;
            sure(came.instance("one-order")["state"]["where"] == "Lyon", "it kept where order 7 went")
        }
        "turn" => {
            let asked: Vec<&str> =
                came.asked().iter().map(|one| one["asked"]["cmd"][1].as_str().unwrap_or_default()).collect();
            // Asked in this order: order 7, then order 8 beside it, then order 7's second once the first was answered.
            sure(asked == ["7", "8", "7"], "order 8 was asked for before order 7's second payment")?;
            let waits: Vec<bool> = came.asked().iter().map(|one| one["waiting"] == true).collect();
            sure(waits == [false, true, true], "the first was answered, and the other two wait")?;
            let first_over = came
                .at(|line| line["what"] == "run" && line["did"] == "finished" && line["flow"] == "on.paid")
                .ok_or("the first payment's run was over")?;
            let third_begun = came
                .last(|line| line["what"] == "run" && line["did"] == "started" && line["flow"] == "on.paid")
                .ok_or("three runs were begun")?;
            sure(
                came.runs("on.paid") == 3 && first_over < third_begun,
                "the second of order 7 began after the first",
            )
        }
        "debounce" => {
            sure(came.said("record.updated") == 3, "the record was changed three times")?;
            sure(came.runs("on.changed") == 1, "its handler ran once")?;
            let changed = came
                .last(|line| line["what"] == "said" && line["topic"] == "record.updated")
                .ok_or("a change was said")?;
            let ran =
                came.at(|line| line["what"] == "run" && line["flow"] == "on.changed").ok_or("it ran")?;
            sure(ran > changed, "it ran after the last change, when the clock had moved on")?;
            sure(came.said("news.changed") == 1, "it said its news once")
        }
        "when" => {
            let ship = came.steps("ship");
            sure(ship.len() == 2, "the step was come to twice")?;
            sure(ship[0]["status"] == "skipped", "the first time it was passed over")?;
            sure(
                ship[1]["status"] == "ok"
                    && ship[1]["stood"].as_array().is_some_and(|stood| stood.len() == 1),
                "the second time it ran",
            )
        }
        "one_at_a_time" => {
            sure(came.said("lock.take") == 3, "the lock was asked for three times")?;
            sure(came.runs("on.take") == 2, "nothing ran for the asking that found it held")?;
            sure(came.runs("on.give") == 1, "it was given back once")?;
            sure(came.instance("lock")["state"]["holder"] == "grace", "Grace has it in the end")
        }
        "retry" => {
            let fetch = came.steps("fetch");
            sure(fetch.len() == 1 && fetch[0]["status"] == "ok", "one step, which went well")?;
            sure(fetch[0]["stood"] == json!([1, 2]), "whose command was asked for twice")?;
            let asked = came.asked();
            sure(asked.len() == 2 && asked[0]["failed"].is_string(), "the first try failed")?;
            let waited = asked[1]["at"].as_i64().unwrap_or(0) - asked[0]["at"].as_i64().unwrap_or(0);
            sure(waited >= 2000, "the second try waited for its backoff, two seconds")
        }
        "on_fail" => {
            let build = came.steps("build");
            sure(build.len() == 1 && build[0]["status"] == "failed", "the step failed in the end")?;
            sure(build[0]["stood"] == json!([1, 2, 3]), "it was tried three times")?;
            sure(came.said("build.broken") == 3, "the failure was said as an event each time")?;
            sure(came.told("ops").len() == 3, "and in ops each time")?;
            sure(came.instance("build")["status"] == "failed", "the run failed")
        }
        "wait" => {
            sure(
                came.at(|line| line["what"] == "waits" && line["step"] == "asked").is_some(),
                "the first step waited",
            )?;
            sure(came.said("question.asked") == 1, "it asked its question")?;
            let asked = came.steps("asked");
            sure(asked.len() == 1 && asked[0]["gave"]["timed_out"] == false, "it was answered in time")?;
            sure(came.instance("asked-and-answered")["status"] == "done", "it went on to its end")
        }
        "call" => {
            let watched = came
                .at(|line| {
                    line["what"] == "run" && line["did"] == "finished" && line["blueprint"] == "std/watch"
                })
                .ok_or("std/watch ran its steps")?;
            let step =
                came.at(|line| line["what"] == "step" && line["step"] == "watch").ok_or("the step ended")?;
            sure(watched < step, "the step was over only once what it called had run its steps")
        }
        "spawn" => {
            let step =
                came.at(|line| line["what"] == "step" && line["step"] == "watch").ok_or("the step ended")?;
            let watch = came
                .at(|line| {
                    line["what"] == "run" && line["did"] == "started" && line["blueprint"] == "std/watch"
                })
                .ok_or("std/watch began its steps")?;
            sure(step < watch, "the step was over before what it started had begun its steps")?;
            sure(came.instance("order-watch")["status"] == "active", "what it started is called order-watch")
        }
        other => {
            Err(format!("the scenario `{other}` has no checks here: write what its sentence says as checks"))
        }
    }
}

/// Every name `holds` has checks for.
const CHECKED: &[&str] = &[
    "handler",
    "for",
    "turn",
    "debounce",
    "when",
    "one_at_a_time",
    "retry",
    "on_fail",
    "wait",
    "call",
    "spawn",
];

/// Every example the language is shown by, wherever it is said.
fn examples(value: &Value, found: &mut Vec<String>) {
    match value {
        Value::Object(fields) => {
            for (key, value) in fields {
                if key == "example"
                    && let Some(text) = value.as_str()
                {
                    found.push(text.to_string());
                }
                examples(value, found);
            }
        }
        Value::Array(all) => all.iter().for_each(|one| examples(one, found)),
        _ => {}
    }
}

#[tokio::test]
async fn each_word_shown_working_comes_to_what_its_sentence_says() {
    let rig = Rig::new().await;
    let (status, scenarios) = rig.get("/api/blueprints/language/scenarios").await;
    assert_eq!(status, 200, "{scenarios}");
    let (_, language) = rig.get("/api/blueprints/language").await;
    let mut shown = Vec::new();
    examples(&language, &mut shown);

    let scenarios = scenarios.as_array().expect("a list of scenarios");
    let names: Vec<&str> = scenarios.iter().filter_map(|one| one["name"].as_str()).collect();
    for checked in CHECKED {
        assert!(names.contains(checked), "there are checks for `{checked}` and no scenario of that name");
    }
    for scenario in scenarios {
        let name = scenario["name"].as_str().unwrap();
        let example = scenario["example"].as_str().unwrap();
        assert!(
            shown.iter().any(|one| one == example),
            "`{name}` runs a text no word or action of the language is shown by: a scenario is offered beside its example"
        );
        let acts = scenario["acts"].as_array().unwrap_or_else(|| panic!("`{name}` says its acts as a list"));
        assert!(
            acts.iter().any(|act| act.get("apply").is_some()),
            "`{name}` never applies its text: applying it is one of the things done"
        );
        let notice = scenario["notice"].as_str().unwrap_or_default();
        assert!(
            notice.ends_with('.') && notice.split_whitespace().count() > 8,
            "`{name}` says what to notice in a sentence"
        );

        let made = rig
            .send(
                "/api/sandboxes",
                json!({ "source": example, "apply": false, "answers": scenario["answers"] }),
            )
            .await;
        let id = made["sandbox"].as_str().unwrap_or_else(|| panic!("`{name}` made a sandbox: {made}"));
        for act in acts {
            let (status, answer) = rig.post(&format!("/api/sandboxes/{id}/acts"), act.clone()).await;
            assert_eq!(status, 200, "`{name}`: {act} was refused: {answer}");
        }
        let (status, came) = rig.get(&format!("/api/sandboxes/{id}")).await;
        assert_eq!(status, 200, "{came}");
        assert_eq!(came["said"], sandbox::SAID, "every answer says it was a sandbox test");
        let ended =
            rig.call(reqwest::Method::DELETE, &format!("/api/sandboxes/{id}"), Some(&rig.token), None).await;
        assert_eq!(ended.0, 200, "{}", ended.1);
        let came = Came(came);
        if let Err(what) = holds(name, &came) {
            panic!(
                "`{name}` does not come to what its sentence says: it says {notice:?}, and not so: {what}. What came of \
                 it:\n{}",
                serde_json::to_string_pretty(came.timeline()).unwrap_or_default()
            );
        }
    }
    assert_eq!(rig.node().app.sandboxes.count(), 0, "nothing of a scenario is left once it has ended");
}
