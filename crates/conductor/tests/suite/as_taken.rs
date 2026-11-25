//! What is said once an instance is said to have taken a text where it
//! is, is heard by the text it took.
//!
//! `instance.updated` is what says a text was taken, to whoever saved a
//! followed file as to whoever applied. It was said in the write that
//! took the text, and the handlers were told to go over to it after that
//! write had been answered and a few more things had been asked of the
//! database: something said in between was handled by the text from
//! before, and a handler the text brought began to listen after it and
//! never heard it. A test of a followed file that said something as soon
//! as it heard the text was taken failed five times in forty-eight on
//! two processors, with a journal for each project, where what the test
//! says and what the instance's conductor asks go to two databases.
//!
//! The order is made here and not left to the machine. The database is
//! answered a turn at a time ([`Stepped`]), whoever applies the text is
//! looked at by the test alone, and is not looked at again from the
//! moment the text is said to be taken until what was said then has come
//! to all it comes to with nobody applying.
//!
//! And whoever applies may go away in that moment: a request is let go
//! of where it waits. The handlers were told to take no more by then,
//! and nothing was left to start them in the text: the instance heard
//! nothing from there on. A text is taken whole, whoever was waiting.

use std::future::poll_fn;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::Poll;

use kernel::Source;
use serde_json::json;

use crate::Rig;
use crate::handlers::{active, handled};
use crate::stepped::Stepped;

/// Counts what it hears under the name of the text that heard it.
const FIRST: &str = "\
blueprint: hearer
version: 1
steps:
  claim: { do: test.count, with: { key: claimed } }
on:
  poke:
    event: test.poke
    do: test.count
    with: { key: old }
";

/// The same, saying so of itself: it differs only in its handler.
fn second() -> String {
    FIRST.replace("key: old", "key: new").replace("version: 1", "version: 2")
}

/// And with a handler the text before had not.
fn third() -> String {
    format!("{}  nudge:\n    event: test.nudge\n    do: test.count\n    with: {{ key: nudged }}\n", second())
}

/// How many turns in a row nothing is asked of any database before
/// what is running is taken to be at rest.
const STILL: usize = 6;

/// Applies `text` over what runs under its name, and has `topic` said
/// in the moment the text is said to be taken: by something that hears
/// `instance.updated` and says it at once, with whoever applies not
/// looked at again until what that came to is at rest. Then the apply
/// goes on to its end; or, where whoever applied `goes` away, is let go
/// of in that moment and never looked at again.
async fn said_as_it_is_taken(rig: &Rig, text: &str, topic: &str, goes: bool) {
    let revision = blueprint::revision(text);
    let mut follow = rig.follow_after(rig.mark(), "instance.updated");
    let heard = Arc::new(AtomicBool::new(false));
    let saying = {
        let (journal, heard, topic) = (rig.journal.clone(), heard.clone(), topic.to_string());
        tokio::spawn(async move {
            while follow.next().await.unwrap().data["revision"] != revision.as_str() {}
            heard.store(true, Ordering::SeqCst);
            journal.emit(&topic, Source::default(), json!({})).await.unwrap();
        })
    };
    let mut applying = Some(Box::pin(rig.engine.apply(rig.request(text, json!({})))));
    let mut applied = None;
    // Held before whoever applies is first looked at: looked at with
    // nothing held, it may go on to its end at once.
    let mut stepped = Stepped::holding(&rig.journal);
    rig.soon("the text to be said to be taken", async {
        while !heard.load(Ordering::SeqCst) {
            if let Some(asked) = applying.as_mut()
                && let Poll::Ready(done) = poll_fn(|cx| Poll::Ready(asked.as_mut().poll(cx))).await
            {
                (applied, applying) = (Some(done), None);
            }
            stepped.turn(false).await;
            // Whatever follows the journal hears what the turn kept
            // before whoever applies is looked at again.
            for _ in 0..8 {
                tokio::task::yield_now().await;
            }
        }
    })
    .await;
    assert!(
        applied.is_none(),
        "the apply had come to its end before anything could be said: the order this is there for did not \
         come about"
    );
    if goes {
        applying = None;
    }
    // What was said is kept, and whatever is listening does with it all
    // it will, while whoever applies has not gone on.
    rig.soon("what was said as the text was taken to come to rest", async {
        let mut still = 0;
        while still < STILL || !saying.is_finished() {
            still = if stepped.turn(false).await == Some(0) { still + 1 } else { 0 };
        }
    })
    .await;
    saying.await.unwrap();
    drop(stepped);
    if let Some(asked) = applying {
        let applied = asked.await.unwrap();
        assert_eq!(applied.took, Some(conductor::Took::InPlace), "the text was taken where the instance is");
    }
}

#[tokio::test]
async fn what_is_said_once_a_text_is_said_to_be_taken_in_place_is_handled_by_that_text() {
    let rig = Rig::new().await;
    let applied = rig.apply(FIRST, json!({})).await;
    active(&rig, &applied.instance).await;

    said_as_it_is_taken(&rig, &second(), "test.poke", false).await;
    handled(&rig, "on.poke", 1).await;
    assert_eq!(
        (rig.shared.count("old"), rig.shared.count("new")),
        (0, 1),
        "it was said after `instance.updated`, and is the taken text's to handle"
    );
    assert_eq!(rig.shared.count("claimed"), 1, "its steps were not run again");
}

#[tokio::test]
async fn a_handler_a_text_taken_in_place_brings_hears_what_is_said_once_the_text_is_said_to_be_taken() {
    let rig = Rig::new().await;
    let applied = rig.apply(FIRST, json!({})).await;
    active(&rig, &applied.instance).await;
    // Said before the text is taken: it is nobody's.
    rig.journal.emit("test.nudge", Source::default(), json!({})).await.unwrap();

    said_as_it_is_taken(&rig, &third(), "test.nudge", false).await;
    handled(&rig, "on.nudge", 1).await;
    assert_eq!(rig.shared.count("nudged"), 1, "what was said after `instance.updated`, once");
    let flows: Vec<String> =
        rig.engine.runs(&applied.instance, 10).await.unwrap().into_iter().map(|run| run.flow).collect();
    assert_eq!(flows, ["on.nudge", "main"]);
}

#[tokio::test]
async fn a_text_whoever_applied_it_went_away_from_as_it_was_said_to_be_taken_is_taken_whole_all_the_same() {
    let rig = Rig::new().await;
    let applied = rig.apply(FIRST, json!({})).await;
    active(&rig, &applied.instance).await;

    said_as_it_is_taken(&rig, &third(), "test.poke", true).await;
    handled(&rig, "on.poke", 1).await;
    assert_eq!(
        (rig.shared.count("old"), rig.shared.count("new")),
        (0, 1),
        "what was said as whoever applied went away was heard, by the text that was taken"
    );
    assert_eq!(rig.instance(&applied.instance).await.revision, blueprint::revision(&third()));
    // And it goes on hearing, the handler the text brought with the rest.
    rig.journal.emit("test.nudge", Source::default(), json!({})).await.unwrap();
    rig.journal.emit("test.poke", Source::default(), json!({})).await.unwrap();
    handled(&rig, "on.poke", 2).await;
    handled(&rig, "on.nudge", 1).await;
    assert_eq!((rig.shared.count("new"), rig.shared.count("nudged")), (2, 1));
}
