//! What is written and what is said of it, as one write.

use journal::EventQuery;
use kernel::Source;
use serde_json::json;
use tokio::sync::broadcast::error::TryRecvError;

async fn with_a_table() -> journal::Journal {
    let (journal, _) = crate::journal().await;
    journal.db().call(|db| db.execute_batch("CREATE TABLE things (name TEXT PRIMARY KEY);")).await.unwrap();
    journal
}

async fn things(journal: &journal::Journal) -> Vec<String> {
    journal
        .db()
        .call(|db| {
            let mut read = db.prepare("SELECT name FROM things ORDER BY name")?;
            read.query_map([], |row| row.get(0))?.collect()
        })
        .await
        .unwrap()
}

async fn topics(journal: &journal::Journal) -> Vec<(u64, String)> {
    let kept = journal.read(EventQuery::default()).await.unwrap();
    kept.into_iter().map(|event| (event.seq, event.topic)).collect()
}

#[tokio::test]
async fn what_is_written_and_what_is_said_of_it_are_kept_together_and_numbered_in_order() {
    let journal = with_a_table().await;
    journal.emit("before", Source::default(), json!({})).await.unwrap();
    let (gave, said) = journal
        .write(|writing| {
            writing.transaction().execute("INSERT INTO things (name) VALUES ('door')", [])?;
            let first = writing.say("thing.made", Source::instance("i1"), json!({ "name": "door" }))?;
            let second = writing.say("thing.shown", Source::default(), json!({}))?;
            Ok((first, second, writing.said()))
        })
        .await
        .unwrap();
    assert_eq!(gave, (2, 3, 2), "each is told its number as it is said");
    assert_eq!(things(&journal).await, ["door"]);
    assert_eq!(said.iter().map(|event| event.seq).collect::<Vec<_>>(), [2, 3]);
    assert_eq!(said[0].source.instance.as_deref(), Some("i1"));
    assert!(said[0].source.device.is_some(), "said on this device, as any event is");
    assert_eq!(said[0].ts, said[1].ts, "said in the one moment");
    assert_eq!(journal.last_seq(), 3);
    let after = journal.emit("after", Source::default(), json!({})).await.unwrap();
    assert_eq!(after.seq, 4, "what is said next goes on from there");
    let kept = topics(&journal).await;
    assert_eq!(
        kept,
        [(1, "before".into()), (2, "thing.made".into()), (3, "thing.shown".into()), (4, "after".into())]
    );
}

#[tokio::test]
async fn a_write_that_fails_keeps_nothing_says_nothing_and_gives_no_number_away() {
    let journal = with_a_table().await;
    journal.emit("before", Source::default(), json!({})).await.unwrap();
    let mut live = journal.live();
    let failed = journal
        .write(|writing| {
            writing.transaction().execute("INSERT INTO things (name) VALUES ('door')", [])?;
            writing.say("thing.made", Source::default(), json!({}))?;
            // The same name again: refused, and everything with it.
            writing.transaction().execute("INSERT INTO things (name) VALUES ('door')", [])?;
            Ok(())
        })
        .await;
    assert!(failed.is_err());
    assert!(things(&journal).await.is_empty(), "what it wrote before it failed is not kept");
    assert_eq!(topics(&journal).await, [(1, "before".into())], "nor what it said");
    assert_eq!(live.try_recv().unwrap_err(), TryRecvError::Empty, "and nobody following heard it");
    assert_eq!(journal.last_seq(), 1);
    let next = journal.emit("after", Source::default(), json!({})).await.unwrap();
    assert_eq!(next.seq, 2, "the number it would have had is the next one's");
}

#[tokio::test]
async fn followers_hear_what_was_said_once_it_is_kept_and_in_the_order_it_was_said() {
    let journal = with_a_table().await;
    let mut follow = journal.follow(0, "");
    let mut during = journal.live();
    journal.emit("before", Source::default(), json!({})).await.unwrap();
    assert_eq!(follow.next().await.unwrap().topic, "before");
    assert_eq!(during.try_recv().unwrap().topic, "before");
    let writing = journal.write(move |writing| {
        writing.transaction().execute("INSERT INTO things (name) VALUES ('door')", [])?;
        writing.say("one", Source::default(), json!({}))?;
        writing.say("two", Source::default(), json!({}))?;
        // Nothing is heard while it is still being written.
        Ok(matches!(during.try_recv(), Err(TryRecvError::Empty)))
    });
    // Said while that waits its turn, and so after it.
    let after = journal.emit("after", Source::default(), json!({}));
    let (unheard, _) = writing.await.unwrap();
    assert!(unheard, "a follower heard an event before what was written with it was kept");
    after.await.unwrap();
    let mut heard = Vec::new();
    for _ in 0..3 {
        let event = follow.next().await.unwrap();
        heard.push((event.seq, event.topic.clone()));
    }
    assert_eq!(heard, [(2, "one".into()), (3, "two".into()), (4, "after".into())]);
    assert_eq!(things(&journal).await, ["door"], "and what was written is there to be read by then");
}

#[tokio::test]
async fn a_journal_that_has_shut_keeps_nothing_of_a_write_that_says_something() {
    let journal = with_a_table().await;
    journal.shut_after_saying(|event| event.topic == "the.place");
    journal.emit("the.place", Source::default(), json!({})).await.unwrap();
    assert!(journal.is_shut());
    let said = journal.write(|writing| {
        writing.transaction().execute("INSERT INTO things (name) VALUES ('late')", [])?;
        writing.say("thing.made", Source::default(), json!({}))?;
        Ok(())
    });
    // Its database is open, and what says nothing is written as ever.
    let quiet = journal.write(|writing| {
        writing.transaction().execute("INSERT INTO things (name) VALUES ('quiet')", []).map(|_| ())
    });
    tokio::select! {
        biased;
        _ = said => panic!("a write that said something after the place was answered"),
        // Asked after it, and so answered after it was looked at.
        wrote = quiet => assert!(wrote.unwrap().1.is_empty()),
    }
    assert_eq!(things(&journal).await, ["quiet"], "what was written with something said is not kept");
    assert_eq!(topics(&journal).await, [(1, "the.place".into())]);
    assert_eq!(journal.last_seq(), 1);
}

#[tokio::test]
async fn the_place_a_journal_shuts_at_may_be_said_in_a_write_and_all_of_that_write_is_kept() {
    let journal = with_a_table().await;
    let asked = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let noting = asked.clone();
    journal.shut_after_saying(move |event| {
        noting.lock().unwrap().push(event.topic.clone());
        event.topic == "two"
    });
    let (_, said) = journal
        .write(|writing| {
            writing.transaction().execute("INSERT INTO things (name) VALUES ('door')", [])?;
            for topic in ["one", "two", "three"] {
                writing.say(topic, Source::default(), json!({}))?;
            }
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(said.len(), 3);
    assert!(journal.is_shut());
    assert_eq!(things(&journal).await, ["door"]);
    assert_eq!(topics(&journal).await.len(), 3, "what was kept together is before the place, all of it");
    assert_eq!(*asked.lock().unwrap(), ["one", "two", "three"], "and each was asked whether it is the place");
}

#[tokio::test]
async fn what_is_asked_of_a_journal_that_shuts_meanwhile_is_given_up_when_it_shuts() {
    let journal = with_a_table().await;
    // Asked of one that does not shut, it is done.
    journal.measure("thing.made", &[], 1.0, true);
    assert!(journal.unless_shut(journal.flush_metrics()).await.is_some_and(|wrote| wrote.is_ok()));

    // The database does its work in the order it was asked: it is kept at
    // one piece until it is let go, and the piece behind that shuts the
    // journal. What is asked while it is kept there finds a journal that
    // has not shut, and is behind the shutting: it is never done.
    let (go, kept) = std::sync::mpsc::channel::<()>();
    let waiting = journal.db().call(move |_| {
        let _ = kept.recv();
        Ok(())
    });
    let its = journal.clone();
    let shutting = journal.db().call(move |_| {
        its.shut_now();
        Ok(())
    });
    journal.measure("thing.made", &[], 1.0, true);
    assert!(!journal.is_shut());
    let asked = journal.unless_shut(journal.flush_metrics());
    tokio::pin!(asked);
    tokio::select! {
        biased;
        _ = &mut asked => panic!("it was answered with the database kept at something else"),
        () = std::future::ready(()) => {}
    }
    go.send(()).unwrap();
    assert!(asked.await.is_none(), "given up, the journal having shut before it was come to");
    assert!(journal.is_shut());
    drop((waiting, shutting));
    // And asked of one that has shut, it is given up without being asked.
    assert!(journal.unless_shut(journal.flush_metrics()).await.is_none());
}

#[tokio::test]
async fn a_small_value_kept_in_a_write_is_kept_with_what_is_written_there_or_not_at_all() {
    let journal = with_a_table().await;
    // Where something that follows the events has read to, kept with
    // what it wrote for the last of them and the news of that.
    journal
        .write(|writing| {
            writing.transaction().execute("INSERT INTO things (name) VALUES ('door')", [])?;
            writing.say("thing.made", Source::default(), json!({}))?;
            writing.keep("reader", "place", &json!({ "at": 7 }))
        })
        .await
        .unwrap();
    assert_eq!(journal.kv_get("reader", "place").await.unwrap(), Some(json!({ "at": 7 })));

    // A write that fails keeps none of it: the place is where it was.
    let failed = journal
        .write(|writing| {
            writing.keep("reader", "place", &json!({ "at": 9 }))?;
            writing.transaction().execute("INSERT INTO things (name) VALUES ('door')", [])?;
            Ok(())
        })
        .await;
    assert!(failed.is_err());
    assert_eq!(journal.kv_get("reader", "place").await.unwrap(), Some(json!({ "at": 7 })));
    assert_eq!(things(&journal).await, ["door"]);
}
