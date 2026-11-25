//! A sandbox is to be light: it is stopped where it comes to more than
//! it may, and says so.

use sandbox::{Answers, Limits};
use serde_json::json;

use crate::{Rig, steps};

#[tokio::test]
async fn a_sandbox_that_runs_more_steps_than_it_may_is_stopped_says_so_and_is_still_read() {
    let mut text = "blueprint: long\nsteps:\n".to_string();
    for at in 0..12 {
        text.push_str(&format!("  s{at}: {{ do: noop }}\n"));
    }
    let limits = Limits { steps: 5, ..Limits::default() };
    let rig = Rig::with(&text, Answers::Asked, limits, &[]).await;

    let did = rig.apply().await;

    let over = did["over"].as_str().unwrap();
    assert!(over.contains("a sandbox test runs 5 at most"), "{over}");
    assert!(over.contains("It is to be light"), "{over}");
    // Nothing more is done to it, in the same words; what it did is read.
    let refused = rig.refused(json!({ "say": { "topic": "bell.rung" } })).await;
    assert_eq!(refused, over);
    let shown = rig.read().await;
    assert!(!steps(&shown).is_empty(), "{shown}");
    assert_eq!(shown["over"], over);
}

#[tokio::test]
async fn a_sandbox_that_says_more_events_than_it_may_is_stopped() {
    let text = "blueprint: chatty\nsteps:\n  a: { do: emit, with: { topic: chat.said, data: { n: 1 } } }\n  b: { do: emit, with: { topic: chat.said, data: { n: 2 } } }\n  c: { do: emit, with: { topic: chat.said, data: { n: 3 } } }\n";
    let limits = Limits { events: 4, ..Limits::default() };
    let rig = Rig::with(text, Answers::Asked, limits, &[]).await;

    let did = rig.apply().await;

    assert!(did["over"].as_str().unwrap().contains("a sandbox test says 4 at most"), "{did}");
}

#[tokio::test]
async fn what_a_sandbox_s_steps_write_to_their_logs_counts_toward_what_it_may_keep() {
    // Each step's log is a file of the sandbox's folder: a line for what
    // the step was handed, and a line for what it says.
    let words = "a line of a step's log. ".repeat(400);
    let mut text = "blueprint: wordy\nsteps:\n".to_string();
    for at in 0..24 {
        text.push_str(&format!("  s{at}: {{ do: log, with: {{ message: \"${{params.words}}\" }} }}\n"));
    }
    text.push_str("params:\n  words: { type: string }\n");
    let apply = json!({ "apply": { "params": { "words": words } } });

    // With room for it, it runs to its end, and says how much of what it
    // keeps is its logs.
    let roomy = Rig::new(&text).await;
    let did = roomy.act(apply.clone()).await;
    assert!(did["over"].is_null(), "{did}");
    let (keeps, logged) = (did["keeps"].as_u64().unwrap(), did["logged"].as_u64().unwrap());
    assert!(logged > 24 * 2 * 9_000, "each step wrote what it was handed and what it said: {logged}");
    assert!(keeps > logged, "what is kept is the journals and the logs together: {keeps}, {logged}");

    // Held to less than that, and to more than its journals alone come
    // to, it is stopped for what it keeps: the logs are what took it over.
    let limits = Limits { bytes: keeps - logged / 2, ..Limits::default() };
    let tight = Rig::with(&text, Answers::Asked, limits, &[]).await;
    let did = tight.act(apply).await;
    let over = did["over"].as_str().unwrap_or_else(|| panic!("it was not stopped: {did}"));
    assert!(over.contains("in its journals and the logs of its steps"), "{over}");
}
