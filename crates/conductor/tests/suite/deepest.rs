//! How deep a tree may go: a step that would start a child deeper than
//! the bound fails, saying which text, how deep and what went round, and
//! one at the bound is started as any is.

use conductor::deepest::{DEEPEST, too_deep};
use conductor::{InstanceFilter, RunStatus};
use serde_json::json;

use crate::{Rig, step};

fn chain(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
}

#[test]
fn a_child_deeper_than_a_tree_may_go_is_refused_saying_what_went_round() {
    // As deep as a tree goes is started; one more is not.
    let at_most = vec!["deep".to_string(); DEEPEST - 1];
    assert_eq!(too_deep("deep", "call", &at_most), None);
    let past = vec!["deep".to_string(); DEEPEST];
    assert_eq!(
        too_deep("deep", "call", &past).unwrap(),
        format!(
            "`deep` was not called: it would be {} deep in its tree, and a tree goes {DEEPEST} deep at most. \
             `deep` calls itself and the `when` of the step that does keeps holding: say one that stops holding.",
            DEEPEST + 1
        )
    );
    // Two that call each other: the one there more often first.
    let mut ring: Vec<String> =
        (0..DEEPEST).map(|at| if at % 2 == 0 { "ping" } else { "pong" }.into()).collect();
    ring[DEEPEST - 1] = "top".into();
    let said = too_deep("pong", "spawn", &ring).unwrap();
    assert!(said.starts_with("`pong` was not begun:"), "{said}");
    assert!(said.ends_with("`ping`, `pong` call each other in a ring that nothing ends: say a `when` that stops holding where one of them starts the next."), "{said}");
    // Texts under texts with nothing going round.
    let all: Vec<String> = (0..DEEPEST).map(|at| format!("t{at}")).collect();
    assert!(too_deep("last", "call", &all).unwrap().ends_with("which is to be cut into fewer levels."));
    assert_eq!(too_deep("x", "call", &chain(&["a", "b"])), None);
}

#[tokio::test]
async fn a_text_that_calls_itself_with_a_when_that_keeps_holding_fails_at_the_bound_and_not_before() {
    let rig = Rig::new().await;
    let deep = "\
blueprint: deep
params: { n: 0 }
steps:
  again: { when: \"${ params.n > -1 }\", call: deep, as: again, with: { n: \"${ params.n }\" } }
";
    rig.library.put("deep", deep);
    let run = rig.run(deep, json!({})).await;
    assert_eq!(run.status, RunStatus::Failed);
    let all = rig.engine.instances(InstanceFilter::default()).await.unwrap();
    let made = all.iter().filter(|one| one.blueprint == "deep").count();
    assert_eq!(made, DEEPEST, "as many as a tree goes deep, the one at its top among them");
    let why = format!("it would be {} deep in its tree", DEEPEST + 1);
    assert!(
        step(&run, "again").error.as_deref().is_some_and(|said| said.contains(&why)),
        "{:?}",
        step(&run, "again").error
    );
}
