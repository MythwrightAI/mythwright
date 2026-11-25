//! What a count says makes an event one thing that happened to its thing
//! (`once`): said again of the same, as what a blueprint that keeps
//! nothing says where it is done again after a stop, it is counted once;
//! and one said of the same thing by another, or another of the count's
//! topics said by the same, is another.

use comparisons::{CAME, Compare, Counted};
use journal::MetricQuery;
use kernel::Source;
use serde_json::json;

use crate::asked::{ask, asked, asks, tone};
use crate::{Rig, user};

/// What followed a thing, said by what it names `by`: counted once for
/// each, of each of the two topics the count is of.
fn followed(topic: &str) -> Counted {
    Counted {
        name: "after".into(),
        topic: topic.into(),
        thing: "card".into(),
        once: vec!["by".into()],
        ..Default::default()
    }
}

/// Said last, so that once it is counted everything said before it was.
fn seen() -> Counted {
    Counted { name: "seen".into(), topic: "test.seen".into(), thing: "card".into(), ..Default::default() }
}

/// How many of `counted` were counted under each way of `ab`.
async fn counted(rig: &Rig, counted: &str) -> Vec<(String, u64)> {
    rig.journal.flush_metrics().await.unwrap();
    let query = MetricQuery {
        name: CAME.into(),
        group_by: vec!["way".into()],
        labels: vec![("compared".into(), "ab".into()), ("counted".into(), counted.into())],
        ..Default::default()
    };
    let mut all: Vec<(String, u64)> = rig
        .journal
        .query_metrics(query)
        .await
        .unwrap()
        .into_iter()
        .map(|one| (one.labels["way"].clone(), one.total.count))
        .collect();
    all.sort();
    all
}

async fn say(rig: &Rig, topic: &str, data: serde_json::Value) {
    rig.journal.emit(topic, Source::default(), data).await.unwrap();
}

#[tokio::test]
async fn what_is_said_again_of_a_thing_is_counted_once_and_what_another_says_of_it_is_counted_too() {
    let rig = Rig::new().await;
    asks(&rig).await;
    let ab = Compare {
        per: Some("id".into()),
        counts: vec![followed("test.followed"), followed("test.redone"), seen()],
        ..asked("ab", vec![tone("loud", 1), tone("quiet", 1)])
    };
    rig.compared.compare(ab, &user()).await.unwrap();
    ask(&rig, &[("e1", "ab", "loud"), ("e2", "ab", "quiet")], 0).await;

    // e1 followed by shop-5 three times over (done again after stops), and by shop-6 once; e2 by
    // shop-5 once; and one event that says nothing of what made it, counted as one of its own.
    for by in ["shop-5", "shop-5", "shop-6", "shop-5"] {
        say(&rig, "test.followed", json!({ "card": "e1", "by": by })).await;
    }
    say(&rig, "test.followed", json!({ "card": "e2", "by": "shop-5" })).await;
    say(&rig, "test.followed", json!({ "card": "e2" })).await;
    // The same card doing another thing of the same count is another thing that happened.
    say(&rig, "test.redone", json!({ "card": "e2", "by": "shop-5" })).await;
    say(&rig, "test.seen", json!({ "card": "e2" })).await;
    crate::soon("the last to be counted", async {
        while counted(&rig, "seen").await.is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    let wanted = [("loud".to_string(), 2), ("quiet".to_string(), 3)];
    assert_eq!(counted(&rig, "after").await, wanted);
}
