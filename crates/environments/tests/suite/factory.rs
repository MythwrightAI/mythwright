//! The factory's variables, as they are kept and held.

use environments::factory::{Changes, Held, NAMESPACE};
use kernel::{ErrorKind, Fields};
use serde_json::{Value, json};

use crate::journal;

fn vars(value: Value) -> Fields {
    kernel::fields(value)
}

#[tokio::test]
async fn variables_set_are_what_a_factory_started_again_on_the_same_journal_holds() {
    let journal = journal().await;
    let held = Held::load(&journal).await.unwrap();
    assert_eq!(held.vars(), Fields::new(), "a factory that never set any holds none");
    let set = vars(json!({ "api": "https://api.example", "replicas": 3, "careful": true }));
    held.changing().await.set(&journal, set.clone()).await.unwrap();
    assert_eq!(held.vars(), set);
    // Kept before they were held: read again, they are what was set.
    assert_eq!(Held::load(&journal).await.unwrap().vars(), set);
}

#[tokio::test]
async fn a_change_says_by_name_what_was_set_changed_and_removed_and_none_says_nothing() {
    let journal = journal().await;
    let held = Held::load(&journal).await.unwrap();
    let first =
        held.changing().await.set(&journal, vars(json!({ "api": "one", "replicas": 3 }))).await.unwrap();
    assert_eq!(
        first,
        Changes { set: vec!["api".into(), "replicas".into()], changed: vec![], removed: vec![] }
    );
    let second = held
        .changing()
        .await
        .set(&journal, vars(json!({ "api": "two", "region": "eu-west" })))
        .await
        .unwrap();
    assert_eq!(
        second,
        Changes { set: vec!["region".into()], changed: vec!["api".into()], removed: vec!["replicas".into()] }
    );
    let again = held
        .changing()
        .await
        .set(&journal, vars(json!({ "api": "two", "region": "eu-west" })))
        .await
        .unwrap();
    assert!(again.is_empty(), "the same set again changes nothing: {again:?}");
}

#[tokio::test]
async fn a_factory_whose_variables_were_cleared_keeps_what_one_that_never_had_any_keeps() {
    let journal = journal().await;
    let held = Held::load(&journal).await.unwrap();
    held.changing().await.set(&journal, vars(json!({ "api": "one" }))).await.unwrap();
    let cleared = held.changing().await.set(&journal, Fields::new()).await.unwrap();
    assert_eq!(cleared.removed, ["api"]);
    assert_eq!(journal.kv_get(NAMESPACE, "vars").await.unwrap(), None, "nothing is kept for none");
    assert_eq!(Held::load(&journal).await.unwrap().vars(), Fields::new());
}

#[tokio::test]
async fn a_name_an_expression_cannot_read_and_a_value_that_is_not_plain_are_refused_with_nothing_kept() {
    let journal = journal().await;
    let held = Held::load(&journal).await.unwrap();
    for (wrong, says) in [
        (json!({ "1st": "x" }), "cannot be the name of a variable"),
        (json!({ "api-url": "x" }), "cannot be the name of a variable"),
        (json!({ "hosts": ["a", "b"] }), "is a list, and a variable is plain"),
        (json!({ "limits": { "cpu": 2 } }), "is a map, and a variable is plain"),
        (json!({ "gone": null }), "is nothing (null), and a variable is plain"),
    ] {
        let refused = held.changing().await.set(&journal, vars(wrong.clone())).await.unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Invalid, "{wrong}");
        assert!(refused.message.contains(says), "{wrong}: {}", refused.message);
    }
    assert_eq!(held.vars(), Fields::new());
    assert_eq!(journal.kv_get(NAMESPACE, "vars").await.unwrap(), None);
}

#[tokio::test]
async fn what_was_kept_that_is_not_a_variable_is_left_out_when_it_is_read() {
    // A database edited by hand: what no expression could read, or would
    // read as something other than a plain value, is not handed to every one.
    let journal = journal().await;
    let kept = json!({ "api": "https://api.example", "not-a-name": "x", "hosts": ["a"], "replicas": 2 });
    journal.kv_set(NAMESPACE, "vars", &kept).await.unwrap();
    let held = Held::load(&journal).await.unwrap();
    assert_eq!(held.vars(), vars(json!({ "api": "https://api.example", "replicas": 2 })));
}

#[tokio::test]
async fn what_the_environment_is_called_is_said_once_and_is_empty_until_it_is() {
    let held = Held::load(&journal().await).await.unwrap();
    assert_eq!(held.name(), "");
    assert_eq!(held.view(), json!({ "name": "", "vars": {} }));
    held.called("prod".into());
    held.called("dev1".into());
    assert_eq!(held.name(), "prod", "the first said is what it is called");
    assert_eq!(held.view(), json!({ "name": "prod", "vars": {} }));
}
