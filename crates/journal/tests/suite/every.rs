//! What every journal of a factory says, read and followed as one: each
//! journal's events in their order, the journals' as they come; a place
//! among them that is a place in each, and a number alone a place in the
//! factory's; and a journal made meanwhile followed from its beginning.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use journal::{EventQuery, FollowEvery, Journal, Places};
use kernel::{Clock, ManualClock, Source};
use serde_json::json;

const PATIENCE: Duration = Duration::from_secs(40);

/// A factory's journal in memory that keeps its projects' beside it, and
/// the clock they go by.
async fn factory() -> (Journal, ManualClock) {
    let clock = ManualClock::new();
    let journal = Journal::memory(Arc::new(clock.clone()) as Arc<dyn Clock>).await.unwrap();
    journal.keeps_projects_in_memory().await.unwrap();
    (journal, clock)
}

async fn say(journal: &Journal, topic: &str) -> u64 {
    journal.emit(topic, Source::default(), json!({})).await.unwrap().seq
}

/// The next `many` events a follower is handed, by their numbers.
async fn heard(follow: &mut FollowEvery, many: usize) -> Vec<u64> {
    let mut heard = Vec::new();
    for _ in 0..many {
        let next = tokio::time::timeout(PATIENCE, follow.next()).await.expect("an event was owed");
        heard.push(next.unwrap().seq);
    }
    heard
}

#[test]
fn a_place_among_the_journals_is_a_place_in_each_and_a_number_alone_is_above_it_in_every_one() {
    let mut places = Places::above(12);
    assert_eq!((places.of(journal::FACTORY), places.of("project:shop")), (12, 12));
    assert_eq!(places.text(), "12", "a number alone is written as the number it always was");
    assert_eq!(Places::parse("12"), Some(places.clone()));
    assert_eq!(Places::from_kept(&json!(12)), Some(places.clone()));

    places.put("project:shop", 2050);
    places.put("project:a;b=c d%", 7);
    places.put(journal::FACTORY, 0);
    for written in [places.text(), format!(" {} ", places.text())] {
        assert_eq!(Places::parse(&written), Some(places.clone()), "{written}");
    }
    assert!(!places.text().contains(' '), "{}", places.text());
    assert_eq!(Places::from_kept(&places.kept()), Some(places.clone()));
    assert_eq!((places.of(journal::FACTORY), places.of("project:made-later")), (0, 12));
    // A place taken now names every journal there is, and one made
    // afterwards is read from its beginning.
    let mut now = Places::default();
    now.put(journal::FACTORY, 40);
    assert_eq!((now.of(journal::FACTORY), now.of("project:made-later")), (40, 0));
    assert_eq!(Places::from_kept(&now.kept()), Some(now.clone()));
    assert_eq!(Places::parse(&now.text()), Some(now));
    assert_eq!(
        Places::parse("0~factory~40~project:shop~2050").map(|read| read.of("project:shop")),
        Some(2050)
    );
    for wrong in ["", "at", "12~shop", "12~project:shop~x", "-1"] {
        assert_eq!(Places::parse(wrong), None, "{wrong}");
    }
    assert_eq!(Places::from_kept(&json!("12")), None);
}

#[test]
fn a_place_in_a_journal_a_project_was_moved_to_is_the_lesser_of_where_it_was_read_to_and_the_move() {
    // (its number for the rest, its place in the journal the project was
    // moved from, where the giver stood at the move) and what it comes to.
    let cases = [
        // Read as far as 500 there, moved at 1000: what the project said
        // up to 500 was heard there, and the rest is still to be.
        ((0, 500, 1000), 500),
        // Read past the move there: everything from before was heard,
        // and nothing the project's own journal said since.
        ((0, 1500, 1000), 1000),
        // Nothing read there: from the beginning.
        ((0, 0, 1000), 0),
        // Exactly at the move.
        ((0, 1000, 1000), 1000),
        // A number alone, above the move, is after everything numbered
        // so in whichever journal, what the project said since among it.
        ((2000, 2000, 1000), 2000),
        // A number alone below the move is that number.
        ((400, 400, 1000), 400),
    ];
    for ((rest, there, at), comes_to) in cases {
        assert_eq!(journal::joined(rest, there, at), comes_to, "({rest}, {there}, {at})");
    }
    // A place worked out is said by name, and stays said where it is
    // the number for the rest: it is not worked out a second time.
    let mut places = Places::above(0);
    assert_eq!(places.named("project:shop"), None);
    places.name("project:shop", 0);
    assert_eq!(places.named("project:shop"), Some(0));
    let kept = Places::from_kept(&places.kept()).unwrap();
    assert_eq!((kept.named("project:shop"), kept.of("factory")), (Some(0), 0));
    assert_eq!(Places::parse(&places.text()).unwrap().named("project:shop"), Some(0));
}

#[tokio::test]
async fn a_follower_of_every_journal_hears_each_journals_events_in_order_once_and_goes_on_from_its_place() {
    let (of_factory, _clock) = factory().await;
    let shop = of_factory.own("shop").await.unwrap();
    let mut said: BTreeMap<&str, Vec<u64>> = BTreeMap::new();
    for round in 0..6 {
        said.entry("shop").or_default().push(say(&shop, "said").await);
        if round % 2 == 0 {
            said.entry("factory").or_default().push(say(&of_factory, "said").await);
        }
        said.entry("shop").or_default().push(say(&shop, "not.wanted").await);
    }
    let wanted =
        |journal: &str, from: usize| -> Vec<u64> { said[journal].iter().skip(from).copied().collect() };
    let of_shop: Vec<u64> = said["shop"].iter().step_by(2).copied().collect();

    // From the beginning: all nine that were wanted, each journal's in
    // the order it said them.
    let mut follow = of_factory.follow_every(Places::default(), "said");
    let first = heard(&mut follow, 4).await;
    let place = follow.places().clone();
    let rest = heard(&mut follow, 5).await;
    let all = [first.clone(), rest].concat();
    let from = |these: &Vec<u64>| all.iter().filter(|seq| these.contains(seq)).copied().collect::<Vec<_>>();
    assert_eq!(from(&of_shop), of_shop);
    assert_eq!(from(&wanted("factory", 0)), wanted("factory", 0));
    assert_eq!(all.len(), 9);
    // What is said from now on is heard as it comes, whichever says it.
    let live = [say(&of_factory, "said").await, say(&shop, "said").await];
    let mut next = heard(&mut follow, 2).await;
    next.sort_unstable();
    assert_eq!(next, {
        let mut live = live.to_vec();
        live.sort_unstable();
        live
    });

    // Another, begun at the place the first had come to after four, is
    // handed what the first was handed after them and nothing before.
    let mut again = of_factory.follow_every(place, "said");
    let mut after = heard(&mut again, 7).await;
    let mut owed: Vec<u64> = all[4..].iter().chain(&live).copied().collect();
    after.sort_unstable();
    owed.sort_unstable();
    assert_eq!(after, owed);
    assert!(first.iter().all(|seq| !after.contains(seq)));
}

#[tokio::test]
async fn a_journal_made_while_every_journal_is_followed_is_followed_from_its_beginning() {
    if crate::one_journal() {
        return;
    }
    let (of_factory, _clock) = factory().await;
    let before = say(&of_factory, "said").await;
    // From now, with nothing to hear: the follower waits.
    let mut follow = of_factory.follow_every(of_factory.now(), "");
    let waiting = tokio::spawn(async move {
        let next = heard(&mut follow, 2).await;
        (next, follow)
    });
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }
    assert!(!waiting.is_finished());
    // A project is given a journal, and says something in it.
    let shop = of_factory.own("shop").await.unwrap();
    let said = [say(&shop, "said").await, say(&shop, "said").await];
    let (next, follow) = tokio::time::timeout(PATIENCE, waiting).await.expect("it was never woken").unwrap();
    assert_eq!(next, said);
    assert_eq!(follow.heard_in(), Some("project:shop"));
    assert_eq!((follow.places().of("project:shop"), follow.places().of(journal::FACTORY)), (said[1], before));

    // A place that is a number alone, as was kept while there was one
    // journal, is before everything a journal made since has said.
    let mut from_before = of_factory.follow_every(Places::above(before), "");
    assert_eq!(heard(&mut from_before, 2).await, said);
}

#[tokio::test]
async fn what_every_journal_holds_is_read_in_the_order_of_when_it_was_said() {
    if crate::one_journal() {
        return;
    }
    let (of_factory, clock) = factory().await;
    let (shop, yard) = (of_factory.own("shop").await.unwrap(), of_factory.own("yard").await.unwrap());
    // The yard draws its numbers last and speaks first: numbers say
    // nothing of the order between journals.
    let mut said = Vec::new();
    for journal in [&of_factory, &shop, &yard] {
        say(journal, "drew").await;
    }
    let now = of_factory.now();
    for journal in [&yard, &of_factory, &shop, &yard] {
        clock.advance(Duration::from_millis(5));
        said.push(say(journal, "said").await);
    }
    assert!(said[0] > said[1] && said[0] > said[2], "{said:?}");
    let asked = |newest_first| EventQuery { topics: "said".into(), newest_first, ..Default::default() };
    let read = |query| {
        let of_factory = of_factory.clone();
        async move {
            of_factory.read_every(query).await.unwrap().iter().map(|event| event.seq).collect::<Vec<_>>()
        }
    };
    assert_eq!(read(asked(false)).await, said);
    let newest: Vec<u64> = said.iter().rev().copied().collect();
    assert_eq!(read(asked(true)).await, newest);
    assert_eq!(read(EventQuery { limit: 2, ..asked(true) }).await, newest[..2]);
    // What was said after a place, read with the place that is after it.
    let (after, to) = of_factory.read_after(&now, "", 100).await.unwrap();
    assert_eq!(after.iter().map(|event| event.seq).collect::<Vec<_>>(), said);
    assert_eq!(to, of_factory.now());
    assert!(of_factory.read_after(&to, "", 100).await.unwrap().0.is_empty());
}

#[tokio::test]
async fn what_was_said_is_looked_for_and_taken_out_in_whichever_journal_keeps_it() {
    if crate::one_journal() {
        return;
    }
    let (of_factory, _clock) = factory().await;
    let shop = of_factory.own("shop").await.unwrap();
    let room = |room: &str| json!({ "room": room });
    shop.emit("room.said", Source::default(), room("r_1")).await.unwrap();
    of_factory.emit("room.said", Source::default(), room("r_1")).await.unwrap();
    shop.emit("room.said", Source::default(), room("r_2")).await.unwrap();
    assert!(of_factory.said_in_any("room.said", "room", "r_2").await.unwrap());
    assert!(
        !of_factory.said("room.said", "room", "r_2").await.unwrap(),
        "the factory's own does not keep it"
    );
    assert_eq!(of_factory.forget_in_every("room.said", "room", "r_1").await.unwrap(), 2);
    assert!(!of_factory.said_in_any("room.said", "room", "r_1").await.unwrap());
    assert!(of_factory.said_in_any("room.said", "room", "r_2").await.unwrap());
}

#[tokio::test]
async fn which_journal_keeps_an_event_is_asked_and_one_made_meanwhile_is_told_to_whoever_waits_for_it() {
    if crate::one_journal() {
        return;
    }
    let (of_factory, _clock) = factory().await;
    let shop = of_factory.own("shop").await.unwrap();
    let (in_shop, in_factory) = (say(&shop, "said").await, say(&of_factory, "said").await);
    let keeps = |seq| {
        let of_factory = of_factory.clone();
        async move { of_factory.keeping(seq).await.unwrap().map(|journal| journal.name().to_string()) }
    };
    assert_eq!(keeps(in_shop).await.as_deref(), Some("project:shop"));
    assert_eq!(keeps(in_factory).await.as_deref(), Some(journal::FACTORY));
    assert_eq!((keeps(in_shop + 500).await, keeps(0).await), (None, None));

    // There are two; whoever waits for a third is told when it is made.
    tokio::time::timeout(PATIENCE, of_factory.more_than(1)).await.expect("there are two already");
    let waiting = tokio::spawn({
        let of_factory = of_factory.clone();
        async move { of_factory.more_than(2).await }
    });
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }
    assert!(!waiting.is_finished());
    of_factory.own("yard").await.unwrap();
    tokio::time::timeout(PATIENCE, waiting).await.expect("it was never told").unwrap();
}

#[tokio::test]
async fn the_order_events_are_written_in_is_written_down_across_journals_where_a_factory_is_told_to() {
    if crate::one_journal() {
        return;
    }
    let (of_factory, _clock) = factory().await;
    let order = Arc::new(journal::Order::default());
    let early = say(&of_factory, "a.b").await;
    of_factory.orders_by(order.clone());
    // A journal made afterwards keeps the same order as the factory's.
    let shop = of_factory.own("shop").await.unwrap();
    let mut said = Vec::new();
    for n in 0..6 {
        said.push(say(if n % 2 == 0 { &shop } else { &of_factory }, "a.b").await);
    }
    // The numbers say nothing of it: the shop's are of a later run.
    assert!(said[0] > said[1] && said[2] > said[5], "{said:?}");
    let places: Vec<u64> = said.iter().map(|seq| order.of(*seq).unwrap()).collect();
    assert!(places.windows(2).all(|pair| pair[0] < pair[1]), "{places:?}");
    assert_eq!(order.so_far(), places[5]);
    assert_eq!(order.number(places[2]), Some(said[2]));
    assert!(places[0] > shop.last_seq().max(of_factory.last_seq()), "no place is taken for a number");
    assert_eq!(order.of(early), None, "what was written before the order was kept has no place in it");
}

#[tokio::test]
async fn what_is_only_passed_on_of_a_project_goes_out_where_what_is_kept_of_it_is_kept() {
    if crate::one_journal() {
        return;
    }
    let (of_factory, _clock) = factory().await;
    let shop = of_factory.own("shop").await.unwrap();
    // Which journal is asked for a project is what an event that names
    // it would be kept in: its own, and the factory's for one that has
    // none and for nobody's.
    assert!(of_factory.said_of(Some("shop")).db().is(shop.db()));
    assert!(of_factory.said_of(Some("yard")).db().is(of_factory.db()));
    assert!(of_factory.said_of(None).db().is(of_factory.db()));

    let (mut in_shop, mut in_factory) = (shop.live(), of_factory.live());
    of_factory.emit_ephemeral("worker.writing", Source::default(), json!({ "project": "shop", "n": 1 }));
    of_factory.emit_ephemeral("worker.writing", Source::default(), json!({ "project": "yard", "n": 2 }));
    of_factory.emit_ephemeral("device.sample", Source::default(), json!({ "n": 3 }));
    // Said after what was passed on, it is after it in the one stream.
    of_factory.emit("worker.done", Source::default(), json!({ "project": "shop" })).await.unwrap();
    let next = async |live: &mut tokio::sync::broadcast::Receiver<Arc<kernel::Event>>| {
        let event = tokio::time::timeout(PATIENCE, live.recv()).await.expect("an event was owed").unwrap();
        (event.topic.clone(), event.data["n"].as_u64().unwrap_or(0))
    };
    assert_eq!(next(&mut in_shop).await, ("worker.writing".to_string(), 1));
    assert_eq!(next(&mut in_shop).await, ("worker.done".to_string(), 0));
    assert_eq!(next(&mut in_factory).await, ("worker.writing".to_string(), 2));
    assert_eq!(next(&mut in_factory).await, ("device.sample".to_string(), 3));
}
