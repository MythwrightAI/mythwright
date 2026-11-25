//! A wait that runs out says what the journal held.
//!
//! The rig's waits give up after a while, and a test that fails for one
//! once in fifty runs is read afterwards from what the gate kept of it.
//! "Timed out waiting for an event" says what did not happen; what did is
//! the journal's last events, and the rig says those with it.

use std::time::Duration;

use serde_json::json;

use crate::Rig;

const SAYS: &str = "\
blueprint: says
steps:
  first: { do: emit, with: { topic: test.said, word: one } }
  second: { do: emit, needs: [first], with: { topic: test.said, word: two } }
";

#[tokio::test]
async fn a_wait_that_runs_out_says_the_last_events_the_journal_held_the_latest_first() {
    let rig = Rig::new().await;
    rig.run(SAYS, json!({})).await;

    // Waited for no time at all: nothing here waits to see a wait run out.
    let waited = tokio::spawn(async move {
        rig.within(Duration::ZERO, "an event on `test.never`", std::future::pending::<()>()).await
    });
    let ran_out = waited.await.unwrap_err().into_panic();
    let said = ran_out.downcast_ref::<String>().expect("the rig says why in words");

    let (what, held) = said.split_once('\n').expect("what was waited for, and then what there was");
    assert_eq!(what, "timed out waiting for: an event on `test.never`");
    assert!(held.starts_with("what the journal held last, the latest first:\n"), "{said}");
    // Each thing said is a line, with the step that said it and what it said.
    let line = |word: &str| {
        held.lines()
            .position(|line| {
                line.contains(" test.said [") && line.contains(&format!(r#"{{"word":"{word}"}}"#))
            })
            .unwrap_or_else(|| panic!("nothing is said of `{word}`: {said}"))
    };
    assert!(line("two") < line("one"), "{said}");
    assert!(held.lines().any(|line| line.contains(" test.said [second] ")), "{said}");
    // And what the runner said of the run itself is there with it.
    assert!(held.lines().any(|line| line.contains(" run.finished ")), "{said}");
}
