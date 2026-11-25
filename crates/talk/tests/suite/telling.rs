//! What a blueprint tells somebody (`talk::telling`): a line in the box
//! of whoever it names, in its own words, from the factory's bot, the
//! same done twice, and told nobody where a word names nobody.

use std::sync::Arc;

use kernel::{ErrorKind, Principal};
use serde_json::json;
use talk::inbox::Kind;
use talk::telling::{Telling, Told};

use crate::kept::who;
use crate::{Rig, agent};

/// A rig that knows these people by the ids they have, and what each of
/// them and of `agents` holds.
async fn rig_of(people: &[Principal], agents: &[Principal]) -> Rig {
    let rig = Rig::new().await;
    let named = people.to_vec();
    rig.channels.names_with(Arc::new(move |_, names| {
        Ok(named.iter().filter(|one| names.contains(&one.id)).cloned().collect())
    }));
    let all: Vec<Principal> = people.iter().chain(agents).cloned().collect();
    rig.channels.inbox.holds_with(Arc::new(move |_, actor| {
        Ok(all.iter().find(|one| one.actor() == actor).map(|one| one.scopes.clone()).unwrap_or_default())
    }));
    rig
}

fn telling(to: &[&str], says: &str, about: &str, said_by: &str) -> Telling {
    Telling {
        to: to.iter().map(|word| word.to_string()).collect(),
        says: says.into(),
        why: "asked".into(),
        about: about.into(),
        place: String::new(),
        said_by: said_by.into(),
        parts: Vec::new(),
    }
}

fn told(told: &[&str], passed: &[&str]) -> Told {
    let actors = |named: &[&str]| named.iter().map(|actor| actor.to_string()).collect();
    Told { told: actors(told), passed: actors(passed) }
}

/// What each line of `by`'s box says, the latest told first, with how
/// often it was told.
async fn says(rig: &Rig, by: &Principal) -> Vec<(String, u32)> {
    let lines = rig.channels.inbox.list(by, false).await.unwrap();
    lines.into_iter().map(|line| (line.says.unwrap_or_default(), line.count)).collect()
}

#[tokio::test]
async fn a_telling_is_a_line_from_the_bot_in_the_box_of_each_it_names_and_of_nobody_else() {
    let (ada, mira, nell) = (who("ada", &["*"]), who("mira", &["inbox"]), who("nell", &["inbox"]));
    let scout = agent("scout", &["inbox", "channels"]);
    let rig = rig_of(&[ada.clone(), mira.clone(), nell.clone()], std::slice::from_ref(&scout)).await;
    let channels = &rig.channels;

    // Whoever applied it, a handle with its `@` and without, an actor,
    // and an agent by what it is called: each once, however often named.
    let to = ["applier", "@mira", "mira", "user:mira", "scout", "agent:scout"];
    let came =
        channels.tell(&ada, telling(&to, "shop-4 asks: with tax?", "shop-4", "run_1 tell try 1")).await;
    assert_eq!(came.unwrap(), told(&["user:ada", "user:mira", "agent:scout"], &[]));
    for by in [&ada, &mira, &scout] {
        let lines = channels.inbox.list(by, false).await.unwrap();
        assert_eq!(lines.len(), 1, "{by:?}");
        let line = &lines[0];
        assert_eq!((line.kind, line.owner.as_str()), (Kind::Told, by.actor().as_str()));
        assert_eq!(line.says.as_deref(), Some("shop-4 asks: with tax?"));
        // It opens what it is about, is from the bot whatever that is
        // called now, and keeps which step said it.
        assert_eq!(
            (line.about.as_str(), line.place.as_str(), line.why.as_str()),
            ("shop-4", "shop-4", "asked")
        );
        assert_eq!(line.from, format!("bot:{}", channels.speaker.name()));
        assert_eq!(line.said_by.as_deref(), Some("run_1 tell try 1"));
        assert_eq!((line.count, line.read), (1, None));
    }
    // Nobody it did not name, and each is told of their own alone.
    assert!(channels.inbox.list(&nell, false).await.unwrap().is_empty());
    let heard = rig.events("inbox.told").await;
    assert_eq!(heard.len(), 3);
    for event in &heard {
        let owner = event.data["owner"].as_str().unwrap();
        assert_eq!(event.source.takes.as_deref(), Some(format!("member:inbox:{owner}").as_str()));
        assert_eq!((&event.data["kind"], &event.data["about"]), (&json!("told"), &json!("shop-4")));
        assert!(
            !event.data.to_string().contains("with tax"),
            "an event of a box carries no words: {event:?}"
        );
    }

    // A place of its own is what opens it.
    let elsewhere =
        Telling { place: "ch_9".into(), ..telling(&["nell"], "Look here.", "shop-5", "run_1 look try 1") };
    channels.tell(&ada, elsewhere).await.unwrap();
    let line = channels.inbox.list(&nell, false).await.unwrap().remove(0);
    assert_eq!((line.about.as_str(), line.place.as_str()), ("shop-5", "ch_9"));
}

#[tokio::test]
async fn a_telling_done_twice_is_one_line_and_told_again_of_the_same_thing_for_the_same_reason_is_raised() {
    let (ada, mira) = (who("ada", &["*"]), who("mira", &["inbox"]));
    let rig = rig_of(&[ada.clone(), mira.clone()], &[]).await;
    let channels = &rig.channels;
    let first = telling(&["mira"], "shop-4 asks: with tax?", "shop-4", "run_1 tell try 1");

    // The same step done again: told once, said once, and it answers as it did.
    assert_eq!(channels.tell(&ada, first.clone()).await.unwrap(), told(&["user:mira"], &[]));
    assert_eq!(channels.tell(&ada, first.clone()).await.unwrap(), told(&["user:mira"], &[]));
    assert_eq!(says(&rig, &mira).await, [("shop-4 asks: with tax?".to_string(), 1)]);
    assert_eq!(rig.events("inbox.told").await.len(), 1);

    // Another step tells of the same card for the same reason: the line
    // there is, read and put away meanwhile, is back, unread, counted,
    // and says the latest.
    let line = channels.inbox.list(&mira, false).await.unwrap().remove(0);
    channels.inbox.away(&mira, &line.id).await.unwrap();
    let second = telling(&["mira"], "shop-4 asks: in which currency?", "shop-4", "run_2 tell try 1");
    channels.tell(&ada, second).await.unwrap();
    assert_eq!(says(&rig, &mira).await, [("shop-4 asks: in which currency?".to_string(), 2)]);
    assert_eq!(channels.inbox.unread(&mira).await.unwrap(), 1);
    let raised = channels.inbox.list(&mira, false).await.unwrap().remove(0);
    assert_eq!((raised.id, raised.away), (line.id, None));

    // For another reason, and of another card, it is a line of its own.
    let left =
        Telling { why: "left".into(), ..telling(&["mira"], "shop-4 is left.", "shop-4", "run_3 tell try 1") };
    channels.tell(&ada, left).await.unwrap();
    channels.tell(&ada, telling(&["mira"], "shop-5 asks: why?", "shop-5", "run_4 tell try 1")).await.unwrap();
    assert_eq!(says(&rig, &mira).await.len(), 3);

    // About nothing, each telling is a line of its own, and each done
    // again is still the one.
    for run in ["run_5", "run_6", "run_5"] {
        let nightly = telling(&["mira"], "The night's build is green.", "", &format!("{run} tell try 1"));
        channels.tell(&ada, nightly).await.unwrap();
    }
    let green = says(&rig, &mira).await;
    assert_eq!(
        green.iter().filter(|(says, count)| says.contains("green") && *count == 1).count(),
        2,
        "{green:?}"
    );
    assert_eq!(green.len(), 5);
    assert_eq!(rig.events("inbox.told").await.len(), 6);
}

#[tokio::test]
async fn a_word_that_names_nobody_and_a_bot_are_refused_with_nobody_told_and_one_with_no_box_is_passed_over()
{
    let (ada, mira) = (who("ada", &["*"]), who("mira", &["inbox"]));
    // One the factory starts holds no reading of a box, and has none.
    let (worker, scout) = (agent("shop-worker-1", &["channels", "board"]), agent("scout", &["inbox"]));
    let rig = rig_of(&[ada.clone(), mira.clone()], &[worker.clone(), scout.clone()]).await;
    let channels = &rig.channels;

    for (word, why) in [
        ("@ghost", "names nobody to tell"),
        ("user:ghost", "names nobody to tell"),
        ("agent:ghost", "names nobody to tell"),
        ("device:laptop", "names nobody to tell"),
        ("bot:conductor", "is a bot, which has no inbox"),
        ("conductor", "names nobody to tell"),
        ("@", "names nobody to tell"),
    ] {
        // Named beside somebody who could be told: nobody is.
        let refused = channels
            .tell(&ada, telling(&["mira", word], "Look.", "shop-4", "run_1 tell try 1"))
            .await
            .unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Invalid, "{word}");
        assert!(refused.message.contains(why), "{word}: {}", refused.message);
    }
    assert!(says(&rig, &mira).await.is_empty());
    assert!(rig.events("inbox.told").await.is_empty());

    // An agent the factory starts is passed over, by its name and as
    // whoever applied the text; one that holds its own inbox is told.
    let to = ["shop-worker-1", "scout", "mira"];
    let came = channels.tell(&ada, telling(&to, "Look.", "shop-4", "run_1 tell try 1")).await.unwrap();
    assert_eq!(came, told(&["agent:scout", "user:mira"], &["agent:shop-worker-1"]));
    let hers = Principal { scopes: vec!["inbox".into(), "channels".into()], ..worker.clone() };
    let came =
        channels.tell(&hers, telling(&["applier"], "Look.", "shop-4", "run_2 tell try 1")).await.unwrap();
    assert_eq!(came, told(&[], &["agent:shop-worker-1"]));
    assert_eq!(rig.events("inbox.told").await.len(), 2);

    // Telling takes what changing a box takes, of whoever applied the text;
    // and it says something, for a reason that is one word.
    let reads = who("nell", &["inbox:read", "channels", "records"]);
    let refused =
        channels.tell(&reads, telling(&["mira"], "Look.", "", "run_3 tell try 1")).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied);
    let refused = channels.tell(&ada, telling(&["mira"], " ", "", "run_3 tell try 1")).await.unwrap_err();
    assert!(refused.message.contains("`says` is empty"), "{}", refused.message);
    let wordy = Telling { why: "asked again".into(), ..telling(&["mira"], "Look.", "", "run_3 tell try 1") };
    assert!(channels.tell(&ada, wordy).await.unwrap_err().message.contains("one word"));
    // Whoever applied it being nobody with a box, it cannot tell its applier.
    let refused = channels
        .tell(&Principal::system(), telling(&["applier"], "Look.", "", "run_3 tell try 1"))
        .await
        .unwrap_err();
    assert!(refused.message.contains("has no inbox"), "{}", refused.message);
    // Nobody at all is nobody told, and no failure.
    assert_eq!(
        channels.tell(&ada, telling(&[], "Look.", "", "run_3 tell try 1")).await.unwrap(),
        told(&[], &[])
    );
    assert_eq!(rig.events("inbox.told").await.len(), 2);
}

#[tokio::test]
async fn a_line_keeps_the_parts_it_was_told_with_and_told_again_says_the_latest() {
    let ada = who("ada", &["*"]);
    let rig = rig_of(std::slice::from_ref(&ada), &[]).await;
    let channels = &rig.channels;
    let parts = |column: &str| vec![json!({ "kind": "cards", "project": "shop", "column": column })];
    let first =
        Telling { parts: parts("staging"), ..telling(&["applier"], "Ready.", "in_1", "run_1 told try 1") };
    channels.tell(&ada, first).await.unwrap();
    let line = channels.inbox.list(&ada, false).await.unwrap().remove(0);
    assert_eq!(line.parts, Some(json!(parts("staging"))));
    // Raised by another telling of the same, it keeps that one's parts.
    let again = Telling {
        parts: parts("review"),
        ..telling(&["applier"], "Ready again.", "in_1", "run_2 told try 1")
    };
    channels.tell(&ada, again).await.unwrap();
    let line = channels.inbox.one(&ada, &line.id).await.unwrap();
    assert_eq!((line.count, line.parts), (2, Some(json!(parts("review")))));
    // One told with none keeps none, and says none where it is handed out.
    channels.tell(&ada, telling(&["applier"], "Plain.", "in_2", "run_3 told try 1")).await.unwrap();
    let plain = channels.inbox.list(&ada, false).await.unwrap().remove(0);
    assert_eq!(plain.parts, None);
    assert!(serde_json::to_value(&plain).unwrap().get("parts").is_none());
}
