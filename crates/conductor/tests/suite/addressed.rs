//! Addressed delivery: a handler says what is for it with `for:`, and the
//! conductor, not the handler, decides which events reach it.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use conductor::{Apply, InstanceStatus};
use journal::{MetricQuery, MetricStats};
use kernel::Source;
use serde_json::{Value, json};

use crate::Rig;
use crate::carried::at_once;
use crate::handlers::{active, handled};

/// One instance for each card: each hears only of its own.
const CARD: &str = "\
blueprint: card
params:
  item:
on:
  moved:
    event: board.item.moved
    for: { id: \"${params.item}\" }
    do: test.echo
    with: { n: ${ event.data.n } }
";

/// Something on another topic, to tell by: events are handed out in the
/// order they happened, so once the bell has been heard, whatever was
/// published before it has been handed to whoever it was for.
const BELL: &str = "\
blueprint: bell
on:
  rung:
    event: bell.rung
    do: noop
";

/// Starts an instance of `source` for `item`, and waits until it is
/// active: what is said from then on is its handlers' to hear, from the
/// journal until they listen where events are handed out, which is a
/// moment later.
async fn card(rig: &Rig, source: &str, item: Value) -> String {
    let key = format!("card-{}", item.as_str().map(str::to_string).unwrap_or_else(|| item.to_string()));
    let request = Apply { key: Some(key), ..rig.request(source, json!({ "item": item })) };
    let applied = rig.engine.apply(request).await.unwrap_or_else(|err| panic!("{}", err.message));
    active(rig, &applied.instance).await;
    applied.instance
}

async fn publish(rig: &Rig, topic: &str, data: Value) -> u64 {
    rig.journal.emit(topic, Source::default(), data).await.unwrap().seq
}

async fn moved(rig: &Rig, data: Value) {
    publish(rig, "board.item.moved", data).await;
}

/// What each run of an instance's handlers was fired by, the oldest first.
async fn fired(rig: &Rig, instance: &str) -> Vec<Value> {
    let runs = rig.engine.runs(instance, 100_000).await.unwrap();
    runs.into_iter().rev().filter(|run| run.flow != "main").map(|run| run.event["data"].clone()).collect()
}

/// A measurement, over everything measured so far.
async fn measured(rig: &Rig, name: &str, labels: &[(&str, &str)]) -> Option<MetricStats> {
    let labels = labels.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect();
    let query = MetricQuery { name: name.into(), labels, ..Default::default() };
    rig.journal.query_metrics(query).await.unwrap().into_iter().next().map(|series| series.total)
}

#[tokio::test]
async fn a_handler_with_for_runs_for_an_event_that_carries_the_value_and_for_no_other() {
    let rig = Rig::new().await;
    let card = card(&rig, CARD, json!("shop-12")).await;

    moved(&rig, json!({ "id": "shop-13", "n": 1 })).await;
    moved(&rig, json!({ "n": 2 })).await;
    moved(&rig, json!({ "id": null, "n": 3 })).await;
    moved(&rig, json!({ "id": "shop-12", "n": 4 })).await;
    handled(&rig, "on.moved", 1).await;

    // A handler takes its events in order, one run at a time: had one of
    // the first three been taken for its, that would be its first run.
    assert_eq!(fired(&rig, &card).await, [json!({ "id": "shop-12", "n": 4 })]);
}

#[tokio::test]
async fn every_field_named_in_for_must_have_its_value() {
    let rig = Rig::new().await;
    let source = CARD.replace("for: { id: \"${params.item}\" }", "for: { id: \"${params.item}\", to: done }");
    let card = card(&rig, &source, json!("shop-12")).await;

    moved(&rig, json!({ "id": "shop-12", "to": "doing", "n": 1 })).await;
    moved(&rig, json!({ "id": "shop-13", "to": "done", "n": 2 })).await;
    moved(&rig, json!({ "id": "shop-12", "to": "done", "n": 3 })).await;
    handled(&rig, "on.moved", 1).await;

    assert_eq!(fired(&rig, &card).await, [json!({ "id": "shop-12", "to": "done", "n": 3 })]);
}

#[tokio::test]
async fn a_value_is_the_one_asked_for_when_it_would_be_equal_to_it_in_an_expression() {
    let rig = Rig::new().await;
    // No quotes: the value is the number the instance was started with.
    let source = CARD.replace("\"${params.item}\"", "${params.item}");
    let card = card(&rig, &source, json!(7)).await;

    moved(&rig, json!({ "id": "7", "n": 1 })).await;
    moved(&rig, json!({ "id": 7.0, "n": 2 })).await;
    handled(&rig, "on.moved", 1).await;

    assert_eq!(fired(&rig, &card).await, [json!({ "id": 7.0, "n": 2 })], "as `7 == 7.0` and `7 != \"7\"`");
}

#[tokio::test]
async fn what_is_addressed_to_a_handler_while_the_conductor_is_down_is_handled_once_when_it_returns() {
    let mut rig = Rig::new().await;
    let first = card(&rig, CARD, json!("a")).await;
    let second = card(&rig, CARD, json!("b")).await;

    rig.engine.halt();
    moved(&rig, json!({ "id": "a", "n": 1 })).await;
    moved(&rig, json!({ "id": "b", "n": 2 })).await;
    moved(&rig, json!({ "id": "a", "n": 3 })).await;
    rig.restart().await;
    handled(&rig, "on.moved", 3).await;

    rig.restart().await;
    moved(&rig, json!({ "id": "a", "n": 4 })).await;
    handled(&rig, "on.moved", 4).await;

    let ns = |fired: Vec<Value>| fired.iter().map(|data| data["n"].as_u64().unwrap()).collect::<Vec<_>>();
    assert_eq!(ns(fired(&rig, &first).await), [1, 3, 4], "each exactly once, in order");
    assert_eq!(ns(fired(&rig, &second).await), [2]);
}

#[tokio::test]
async fn a_handler_catches_up_on_more_than_it_reads_at_a_time_whether_it_waits_on_a_topic_or_a_pattern() {
    let mut rig = Rig::new().await;
    let on_topic = card(&rig, CARD, json!("a")).await;
    let on_pattern = card(&rig, &CARD.replace("board.item.moved", "\"board.*.moved\""), json!("b")).await;

    rig.engine.halt();
    for n in 0..700 {
        let id = if n % 300 == 0 { "a" } else { "elsewhere" };
        moved(&rig, json!({ "id": id, "n": n })).await;
        let id = if n % 300 == 1 { "b" } else { "elsewhere" };
        publish(&rig, "board.note.moved", json!({ "id": id, "n": n })).await;
        publish(&rig, "board.item.updated", json!({ "id": "b", "n": n })).await;
    }
    rig.restart().await;
    handled(&rig, "on.moved", 6).await;
    // And from there it is handed what happens, as before.
    moved(&rig, json!({ "id": "a", "n": 700 })).await;
    moved(&rig, json!({ "id": "b", "n": 701 })).await;
    handled(&rig, "on.moved", 8).await;

    let ns = |fired: Vec<Value>| fired.iter().map(|data| data["n"].as_u64().unwrap()).collect::<Vec<_>>();
    assert_eq!(ns(fired(&rig, &on_topic).await), [0, 300, 600, 700]);
    assert_eq!(ns(fired(&rig, &on_pattern).await), [1, 301, 601, 701]);
}

#[tokio::test]
async fn runs_of_a_handler_with_for_never_overlap() {
    let rig = Rig::new().await;
    let source = "\
blueprint: serial
params:
  item:
on:
  job:
    event: job.wanted
    for: { card: \"${params.item}\" }
    do: test.gate
    with: { name: \"job-${event.data.n}\" }
";
    card(&rig, source, json!("x")).await;
    for n in [1, 2] {
        publish(&rig, "job.wanted", json!({ "card": "x", "n": n })).await;
    }
    rig.reached("job-1").await;
    assert_eq!(rig.shared.calls(), ["run:test.gate"], "the second waits for the first");
    rig.open("job-2");
    rig.open("job-1");
    handled(&rig, "on.job", 2).await;
    assert_eq!(rig.shared.calls().len(), 2);
}

/// A burst of five changes, three of them in `docs`, heard by a handler
/// that says with `guard` that only those in `docs` are its.
async fn a_burst_becomes_one_run(guard: &str) {
    let rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    let source = format!(
        "blueprint: rebuild\nparams:\n  item:\non:\n  changed:\n    event: file.changed\n    {guard}\n    \
         debounce: 2s\n    do: test.echo\n    with: {{ path: ${{ event.data.path }} }}\n"
    );
    let instance = card(&rig, &source, json!("docs")).await;
    for (dir, path) in
        [("docs", "a.md"), ("src", "x.rs"), ("docs", "b.md"), ("src", "y.rs"), ("docs", "c.md")]
    {
        publish(&rig, "file.changed", json!({ "dir": dir, "path": path })).await;
    }
    // Each change in `docs` starts the quiet again, so three have begun.
    rig.advance_after(since, 3, "2s").await;
    handled(&rig, "on.changed", 1).await;

    assert_eq!(fired(&rig, &instance).await, [json!({ "dir": "docs", "path": "c.md" })], "{guard}");
    assert_eq!(rig.clock.sleep_count(), since + 3, "what is not its does not start the quiet again: {guard}");
}

#[tokio::test]
async fn debounce_collapses_a_burst_of_a_handlers_own_events_however_it_says_which_are_its() {
    a_burst_becomes_one_run("for: { dir: \"${params.item}\" }").await;
    a_burst_becomes_one_run("when: ${ event.data.dir == params.item }").await;
}

/// Rings the bell, and waits until it has been heard `times` in all.
///
/// Heard a second time or later, the bell says that every event said
/// before that ring has been looked up for whoever waits on its topic:
/// by then the bell's own handler is where events are handed out, so a
/// ring reaches it only from there, and events are handed out in the
/// order they were said. (Its first ring it may have read from the
/// journal itself, as a handler does that is not listening yet.)
async fn rung(rig: &Rig, times: usize) {
    publish(rig, "bell.rung", json!({})).await;
    handled(rig, "on.rung", times).await;
}

#[tokio::test]
async fn among_many_handlers_an_event_wakes_only_the_one_it_is_addressed_to() {
    const CARDS: usize = 50;
    let rig = Rig::new().await;
    let mut cards = Vec::new();
    for n in 0..CARDS {
        cards.push(card(&rig, CARD, json!(format!("shop-{n}"))).await);
    }
    let bell = rig.apply(BELL, json!({})).await;
    active(&rig, &bell.instance).await;
    let round = async |round: u64| {
        for n in 0..CARDS {
            moved(&rig, json!({ "id": format!("shop-{n}"), "n": round })).await;
        }
    };
    // How many events on the topic were looked up, and to how many
    // handlers they went in all; nothing where none was looked up yet.
    let delivered = async || match measured(&rig, "handler.delivery", &[("topic", "board.item.moved")]).await
    {
        Some(so_far) => (so_far.count, (so_far.avg * so_far.count as f64).round(), so_far.min, so_far.max),
        None => (0, 0.0, 0.0, 0.0),
    };
    let woken = async || measured(&rig, "handler.event", &[("blueprint", "card")]).await.unwrap().count;

    // A first round, which brings every handler to where events are
    // handed out. An instance is active a moment before its handlers
    // listen there, and what is said for one that is not there yet is
    // looked up for nobody and read by it from the journal instead. That
    // is the last card's, where whoever says the round does not give way
    // between the card being active and the saying, as on a busy machine
    // nobody does who asks the database several things in a row; so the
    // round is said that way here, and what is counted is counted after.
    // Each has then taken its first event, so each listens; and the
    // bell, rung twice, says that everything so far has been looked up.
    at_once(round(0));
    handled(&rig, "on.moved", CARDS).await;
    rung(&rig, 1).await;
    rung(&rig, 2).await;
    let (looked_up, went_to, ..) = delivered().await;
    assert_eq!(woken().await, CARDS as u64, "each handler was woken once, by its own event");

    // One for nobody, then one for each card; and the bell.
    moved(&rig, json!({ "id": "elsewhere" })).await;
    round(1).await;
    handled(&rig, "on.moved", 2 * CARDS).await;
    rung(&rig, 3).await;

    // Each handler takes what it is handed in order, and has by now taken
    // its second: had it been woken for any of this round but its own,
    // that came before and is counted.
    assert_eq!(woken().await, 2 * CARDS as u64, "each handler was woken twice, by its own two events");

    let (now_looked_up, now_went_to, least, most) = delivered().await;
    assert_eq!(now_looked_up - looked_up, CARDS as u64 + 1, "every event on the topic was looked up");
    assert_eq!((least, most), (0.0, 1.0), "to nobody, or to the one it was for");
    assert_eq!(now_went_to - went_to, CARDS as f64, "each but the one for nobody went to one handler");

    for (n, card) in cards.iter().enumerate() {
        let own = |round: u64| json!({ "id": format!("shop-{n}"), "n": round });
        assert_eq!(fired(&rig, card).await, [own(0), own(1)]);
    }
}

#[tokio::test]
async fn a_handler_that_was_passed_by_does_not_read_it_all_again_after_a_restart() {
    let mut rig = Rig::new().await;
    for item in ["a", "b", "c"] {
        card(&rig, CARD, json!(item)).await;
    }
    // Each has handled one of its own and gone back to waiting.
    for item in ["a", "b", "c"] {
        moved(&rig, json!({ "id": item, "n": 0 })).await;
    }
    handled(&rig, "on.moved", 3).await;

    for n in 0..600 {
        moved(&rig, json!({ "id": "elsewhere", "n": n })).await;
    }
    // Handed out after the six hundred, so they have all been passed on.
    moved(&rig, json!({ "id": "a", "n": 1 })).await;
    handled(&rig, "on.moved", 4).await;

    rig.restart().await;
    for item in ["a", "b", "c"] {
        moved(&rig, json!({ "id": item, "n": 2 })).await;
    }
    handled(&rig, "on.moved", 7).await;

    // A handler has read the journal from its place before it takes
    // anything, so each of the three has by now, twice: once when it
    // first listened and once after the restart.
    let read = measured(&rig, "handler.catch_up", &[]).await.unwrap();
    assert_eq!(read.count, 6);
    assert!(
        read.max < 400.0,
        "one handler read {} events again; the places of `b` and `c`, which were never woken, \
         were not moved past the six hundred that were not theirs",
        read.max
    );
}

#[tokio::test]
async fn an_instance_that_stops_is_no_longer_waited_for() {
    let rig = Rig::new().await;
    let first = card(&rig, CARD, json!("a")).await;
    let second = card(&rig, CARD, json!("b")).await;
    let bell = rig.apply(BELL, json!({})).await;
    active(&rig, &bell.instance).await;
    let delivered =
        async || measured(&rig, "handler.delivery", &[("topic", "board.item.moved")]).await.unwrap();

    rig.engine.stop(&first).await.unwrap();
    moved(&rig, json!({ "id": "a", "n": 1 })).await;
    moved(&rig, json!({ "id": "b", "n": 2 })).await;
    handled(&rig, "on.moved", 1).await;
    let so_far = delivered().await;
    assert_eq!((so_far.count, so_far.min, so_far.max), (2, 0.0, 1.0), "the one for `a` went to nobody");
    assert_eq!(fired(&rig, &first).await, [] as [Value; 0]);

    rig.engine.stop(&second).await.unwrap();
    moved(&rig, json!({ "id": "b", "n": 3 })).await;
    publish(&rig, "bell.rung", json!({})).await;
    handled(&rig, "on.rung", 1).await;
    assert_eq!(delivered().await.count, 2, "nobody waits on the topic any more, so nothing is looked up");
}

#[tokio::test]
async fn a_handler_that_starts_to_listen_while_events_arrive_loses_none_and_takes_none_twice() {
    let mut rig = Rig::new().await;
    // Every tenth is wanted, so that most are passed over, as most are.
    let source = "\
blueprint: listener
params:
  item:
on:
  job:
    event: job.wanted
    for: { card: \"${params.item}\" }
    when: ${ event.data.n % 10 == 0 }
    do: noop
";
    let enough = Arc::new(AtomicBool::new(false));
    let publisher = tokio::spawn({
        let (journal, enough) = (rig.journal.clone(), enough.clone());
        async move {
            let mut n = 0;
            while !enough.load(Ordering::SeqCst) && n < 20_000 {
                n += 1;
                journal.emit("job.wanted", Source::default(), json!({ "card": "x", "n": n })).await.unwrap();
            }
            n
        }
    });
    rig.event("job.wanted", |_| true).await;

    // It starts to listen while they arrive,
    let instance = card(&rig, source, json!("x")).await;
    let active_at =
        rig.event("instance.active", |event| event.source.instance.as_deref() == Some(&instance)).await.seq;
    handled(&rig, "on.job", 3).await;
    // and again after a restart, from the journal this time.
    rig.restart().await;
    let restarted_at = rig.journal.last_seq();
    let before = rig.events("run.finished").await.iter().filter(|run| run.data["flow"] == "on.job").count();
    handled(&rig, "on.job", before + 5).await;

    enough.store(true, Ordering::SeqCst);
    let last = publisher.await.unwrap();
    assert!(last < 20_000, "the publisher was to be stopped, not to run out");
    let end = (last / 10 + 1) * 10;
    let ended_at = publish(&rig, "job.wanted", json!({ "card": "x", "n": end })).await;
    assert!(restarted_at < ended_at - 1, "events were still arriving after the restart");
    let mut finished = rig.follow("run.finished");
    rig.soon("the last event to be handled", async {
        loop {
            let event = finished.next().await.unwrap();
            if event.data["flow"] != "on.job" {
                continue;
            }
            let run = rig.engine.run(event.source.run.as_deref().unwrap()).await.unwrap();
            if run.event["data"]["n"] == end {
                break;
            }
        }
    })
    .await;

    let handled: Vec<u64> =
        fired(&rig, &instance).await.iter().map(|data| data["n"].as_u64().unwrap()).collect();
    let wanted_after_active = rig
        .events("job.wanted")
        .await
        .iter()
        .filter(|event| event.seq > active_at)
        .map(|event| event.data["n"].as_u64().unwrap())
        .find(|n| n % 10 == 0)
        .unwrap();
    assert!(
        handled[0] <= wanted_after_active,
        "the first it handled was {}, and {wanted_after_active} was published after it said it was active",
        handled[0]
    );
    let in_order_once = handled.windows(2).all(|pair| pair[1] == pair[0] + 10);
    assert!(in_order_once && handled.last() == Some(&end), "one was lost, or taken twice: {handled:?}");
}

#[tokio::test]
async fn a_handler_that_falls_far_behind_what_is_handed_to_it_still_takes_each_once_and_in_order() {
    let rig = Rig::new().await;
    // The first holds the handler at a gate while the rest arrive.
    let source = "\
blueprint: slow
params:
  item:
on:
  job:
    event: job.wanted
    for: { card: \"${params.item}\" }
    when: ${ event.data.n % 500 == 0 }
    do: test.gate
    with: { name: \"job-${event.data.n}\" }
";
    let slow = card(&rig, source, json!("x")).await;
    let bell = rig.apply(BELL, json!({})).await;
    active(&rig, &bell.instance).await;

    publish(&rig, "job.wanted", json!({ "card": "x", "n": 0 })).await;
    rig.reached("job-0").await;
    for n in 1..=3000 {
        publish(&rig, "job.wanted", json!({ "card": "x", "n": n })).await;
    }
    // All three thousand have been handed to it by the time the bell is
    // heard, which is more than is kept waiting for one handler.
    publish(&rig, "bell.rung", json!({})).await;
    handled(&rig, "on.rung", 1).await;
    for n in (0..=3000).step_by(500) {
        rig.open(&format!("job-{n}"));
    }
    handled(&rig, "on.job", 7).await;

    let ns: Vec<u64> = fired(&rig, &slow).await.iter().map(|data| data["n"].as_u64().unwrap()).collect();
    assert_eq!(ns, [0, 500, 1000, 1500, 2000, 2500, 3000]);
}

#[tokio::test]
async fn an_event_is_sent_to_one_instance_by_naming_it_in_the_event() {
    let rig = Rig::new().await;
    let reviewer = "\
blueprint: reviewer
on:
  asked:
    event: review.asked
    for: { to: \"${instance.key}\" }
    do: test.echo
    with: { about: ${ event.data.about } }
";
    let mut reviewers = Vec::new();
    for key in ["card-shop-12", "card-shop-13"] {
        let request = Apply { key: Some(key.into()), ..rig.request(reviewer, json!({})) };
        let applied = rig.engine.apply(request).await.unwrap();
        active(&rig, &applied.instance).await;
        reviewers.push(applied.instance);
    }

    let ask = |to: &str, about: &str| {
        format!(
            "blueprint: ask\nsteps:\n  ask: {{ do: emit, with: {{ topic: review.asked, to: {to}, about: {about} }} }}\n"
        )
    };
    rig.run(&ask("card-shop-12", "the price"), json!({})).await;
    rig.run(&ask("card-shop-13", "the colour"), json!({})).await;
    handled(&rig, "on.asked", 2).await;

    assert_eq!(fired(&rig, &reviewers[0]).await, [json!({ "to": "card-shop-12", "about": "the price" })]);
    assert_eq!(fired(&rig, &reviewers[1]).await, [json!({ "to": "card-shop-13", "about": "the colour" })]);
}

#[tokio::test]
async fn a_handler_whose_for_cannot_be_worked_out_hears_nothing_and_says_so() {
    let rig = Rig::new().await;
    let source = CARD.replace("\"${params.item}\"", "${ int(params.item) }");
    let instance = card(&rig, &source, json!("twelve")).await;

    let failed =
        rig.event("handler.failed", |event| event.source.instance.as_deref() == Some(&instance)).await;
    assert_eq!(failed.data["handler"], "moved");
    let error = failed.data["error"].as_str().unwrap();
    assert!(error.contains("the handler `moved` hears nothing"), "{error}");
    assert!(error.contains("`twelve` is not a whole number"), "{error}");
    assert_eq!(
        rig.instance(&instance).await.status,
        InstanceStatus::Active,
        "the instance itself is as it was"
    );
}
