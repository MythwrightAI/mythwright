//! What is passed on with a number and not kept.

use std::sync::Arc;
use std::time::Duration;

use journal::{EventQuery, Journal, Order};
use kernel::Source;
use serde_json::json;

async fn topics(journal: &Journal) -> Vec<(u64, String)> {
    let kept = journal.read(EventQuery::default()).await.unwrap();
    kept.into_iter().map(|event| (event.seq, event.topic)).collect()
}

#[tokio::test]
async fn what_is_passed_on_is_numbered_in_its_order_heard_live_and_never_read_back() {
    let (journal, _) = crate::journal().await;
    let mut live = journal.live();
    let (_, said) = journal
        .write(|writing| {
            let kept = writing.say("thing.made", Source::default(), json!({}))?;
            let passed = writing.pass_on("thing.passing", Source::default(), json!({}))?;
            Ok((kept, passed))
        })
        .await
        .unwrap();
    let by_itself = journal.pass_on("thing.alone", Source::default(), json!({})).await.unwrap();
    let numbers: Vec<u64> = said.iter().map(|event| event.seq).chain([by_itself.seq]).collect();
    assert!(numbers.windows(2).all(|two| two[0] < two[1]), "numbered in the order said: {numbers:?}");
    // Heard as it is said, in that order and each with its number.
    let mut heard = Vec::new();
    for _ in 0..3 {
        let event = live.recv().await.unwrap();
        heard.push((event.seq, event.topic.clone()));
    }
    let said: Vec<(u64, String)> =
        [("thing.made", numbers[0]), ("thing.passing", numbers[1]), ("thing.alone", numbers[2])]
            .into_iter()
            .map(|(topic, seq)| (seq, topic.to_string()))
            .collect();
    assert_eq!(heard, said);
    // And only what was kept is read back; the journal has gone on past
    // what it passed on.
    assert_eq!(topics(&journal).await, [(numbers[0], "thing.made".to_string())]);
    assert_eq!(journal.last_seq(), numbers[2]);
}

#[tokio::test]
async fn a_follower_that_catches_up_hears_what_was_passed_on_while_it_read_and_nothing_twice() {
    let (journal, _) = crate::journal().await;
    journal.emit("thing.first", Source::default(), json!({})).await.unwrap();
    // It begins to follow, and before it reads anything something is
    // passed on and something after it is kept: what it reads from the
    // log is past what was passed on, which it has only as it came live.
    let mut follow = journal.follow(0, "");
    let passed = journal.pass_on("thing.passing", Source::default(), json!({})).await.unwrap();
    let kept = journal.emit("thing.kept", Source::default(), json!({})).await.unwrap();
    let mut heard = Vec::new();
    for _ in 0..3 {
        let event = tokio::time::timeout(Duration::from_secs(5), follow.next()).await.unwrap().unwrap();
        heard.push(event.topic.clone());
    }
    heard.sort();
    assert_eq!(heard, ["thing.first", "thing.kept", "thing.passing"]);
    assert!(passed.seq < kept.seq);
    // Nothing more comes of what it heard.
    let later = journal.emit("thing.later", Source::default(), json!({})).await.unwrap();
    let next = tokio::time::timeout(Duration::from_secs(5), follow.next()).await.unwrap().unwrap();
    assert_eq!(next.seq, later.seq);
}

#[tokio::test]
async fn a_journal_that_passed_something_on_counts_the_newest_it_keeps_from_what_it_holds() {
    let (journal, clock) = crate::journal().await;
    for _ in 0..4 {
        journal.emit("thing.kept", Source::default(), json!({})).await.unwrap();
        journal.pass_on("thing.passing", Source::default(), json!({})).await.unwrap();
    }
    clock.advance(Duration::from_secs(60));
    // Four kept, with a number passed on after each: the newest two are
    // two of what it keeps, and not two numbers.
    assert_eq!(journal.prune_events(journal.now_ms(), 2).await.unwrap(), 2);
    assert_eq!(topics(&journal).await.len(), 2);
}

#[tokio::test]
async fn an_order_hands_a_test_what_was_passed_on_beside_what_was_kept() {
    let (journal, _) = crate::journal().await;
    let order = Arc::new(Order::default());
    journal.orders_by(order.clone());
    journal.emit("thing.kept", Source::default(), json!({})).await.unwrap();
    journal.pass_on("thing.passing", Source::default(), json!({})).await.unwrap();
    let all = order.read_every(&journal, EventQuery::default()).await.unwrap();
    let topics: Vec<&str> = all.iter().map(|event| event.topic.as_str()).collect();
    assert_eq!(topics, ["thing.kept", "thing.passing"]);
    // And followed from the beginning, both, once each.
    let mut follow = order.follow_every(&journal, "");
    let mut heard = Vec::new();
    for _ in 0..2 {
        heard.push(follow.next().await.unwrap().topic.clone());
    }
    heard.sort();
    assert_eq!(heard, ["thing.kept", "thing.passing"]);
    let later = journal.emit("thing.later", Source::default(), json!({})).await.unwrap();
    assert_eq!(follow.next().await.unwrap().seq, later.seq);
}
