//! One thing done several ways, side by side: said from outside of a
//! blueprint by its name, and done where a step of the conductor starts it.

use std::collections::BTreeMap;
use std::time::Duration;

use comparisons::{CAME, Compare, Counted, GIVEN, Reads, Way};
use conductor::taking::Take;
use conductor::{Apply, InstanceFilter, InstanceInfo};
use journal::MetricQuery;
use kernel::{ErrorKind, Principal, PrincipalKind, Source};
use serde_json::json;

use crate::{Rig, active, handled, user};

/// What somebody applies: for each thing asked for it has the work done,
/// and then has it looked at.
const SHOP: &str = "\
blueprint: shop
on:
  asked:
    event: test.poke
    steps:
      done: { call: work, as: \"work-${ event.data.id }\", with: { id: ${ event.data.id } } }
      seen: { call: look, as: \"look-${ event.data.id }\", with: { id: ${ event.data.id } } }
";

/// The work as it ships: it counts under the tone it is handed.
const WORK: &str = "\
blueprint: work
params: { id: 0, tone: plain }
steps:
  claim: { do: test.count, with: { key: \"${ params.tone }\" } }
";

/// Somebody's own way of doing the same work.
const MINE: &str = "\
blueprint: my-work
params: { id: 0, tone: plain }
steps:
  first: { do: test.count, with: { key: \"mine-${ params.tone }\" } }
  second: { do: test.echo }
";

const LOOK: &str = "blueprint: look\nparams: { id: 0 }\nsteps:\n  see: { do: test.echo }\n";

pub(crate) async fn shop(rig: &Rig) -> String {
    for (name, text) in [("shop", SHOP), ("work", WORK), ("my-work", MINE), ("look", LOOK)] {
        rig.library.put(name, text);
    }
    let applied = rig.apply(SHOP, json!({})).await;
    active(rig, &applied.instance).await;
    applied.instance
}

/// Asks for the things `ids`, one after another, `before` having been asked already.
pub(crate) async fn ask(rig: &Rig, ids: &[u64], before: usize) {
    for (at, id) in ids.iter().enumerate() {
        rig.journal.emit("test.poke", Source::default(), json!({ "id": id })).await.unwrap();
        handled(rig, "on.asked", before + at + 1).await;
        let failed = rig.events("run.failed").await;
        assert!(failed.is_empty(), "{:?}", failed.last().map(|event| &event.data));
    }
}

pub(crate) fn way(name: &str) -> Way {
    Way { name: name.into(), share: 1, ..Default::default() }
}

fn handing(name: &str, tone: &str) -> Way {
    Way { with: json!({ "tone": tone }).as_object().unwrap().clone(), ..way(name) }
}

pub(crate) fn tones(ways: Vec<Way>) -> Compare {
    Compare { name: "tones".into(), of: "work".into(), per: Some("id".into()), ways, ..Default::default() }
}

pub(crate) async fn child(rig: &Rig, key: &str) -> InstanceInfo {
    let all = rig.engine.instances(InstanceFilter::default()).await.unwrap();
    all.into_iter().find(|one| one.key == key).unwrap_or_else(|| panic!("no `{key}`"))
}

/// How many things `name` says went each way.
pub(crate) async fn went(rig: &Rig, name: &str) -> Vec<(String, u64)> {
    let one = rig.compared.comparison(name).await.unwrap();
    one.ways.into_iter().map(|way| (way.name, way.went)).collect()
}

/// How often `measurement` was measured under each way of `name`.
async fn counted(rig: &Rig, name: &str, measurement: &str) -> BTreeMap<String, u64> {
    rig.journal.flush_metrics().await.unwrap();
    let query = MetricQuery {
        name: measurement.into(),
        group_by: vec!["way".into()],
        labels: vec![("compared".into(), name.into())],
        ..Default::default()
    };
    let series = rig.journal.query_metrics(query).await.unwrap();
    series.into_iter().map(|one| (one.labels["way"].clone(), one.total.count)).collect()
}

#[tokio::test]
async fn each_thing_goes_one_of_the_ways_by_their_shares_and_keeps_to_it() {
    let mut rig = Rig::new().await;
    shop(&rig).await;
    // Nothing is compared: the work is done as it is written.
    ask(&rig, &[1], 0).await;
    assert_eq!(rig.shared.count("plain"), 1);
    assert!(child(&rig, "work-1").await.labels.is_empty());

    let ways = vec![Way { share: 2, ..handing("loud", "loud") }, handing("quiet", "quiet")];
    rig.compared.compare(tones(ways), &user()).await.unwrap();
    ask(&rig, &[11, 12, 13, 14, 15, 16], 1).await;
    // Two of every three, and never by chance: the same things in the same order.
    assert_eq!((rig.shared.count("loud"), rig.shared.count("quiet")), (4, 2));
    assert_eq!(went(&rig, "tones").await, [("loud".to_string(), 4), ("quiet".to_string(), 2)]);
    let first = child(&rig, "work-11").await;
    assert_eq!((first.labels["compared"].as_str(), first.labels["way"].as_str()), ("tones", "loud"));
    assert_eq!(child(&rig, "work-12").await.labels["way"], "quiet");

    // A thing asked for again goes the way it went, and is not counted twice;
    // so it does when the conductor has been stopped and started in between.
    ask(&rig, &[12, 12], 7).await;
    rig.restart().await;
    ask(&rig, &[12, 11], 9).await;
    assert_eq!((rig.shared.count("loud"), rig.shared.count("quiet")), (5, 5));
    assert_eq!(went(&rig, "tones").await, [("loud".to_string(), 4), ("quiet".to_string(), 2)]);
}

#[tokio::test]
async fn what_is_done_under_a_way_is_measured_with_it_and_so_is_what_else_is_named_for_the_same_thing() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let asked =
        Compare { also: vec!["look".into()], ..tones(vec![handing("loud", "loud"), way("as-written")]) };
    rig.compared.compare(asked, &user()).await.unwrap();
    ask(&rig, &[1, 2, 3], 0).await;
    // A way that says nothing is the work as it is written, and is counted beside the other.
    assert_eq!((rig.shared.count("loud"), rig.shared.count("plain")), (2, 1));
    // One step of the work for each thing, and one of what looked at it
    // afterwards: started as written, and counted with the thing's way.
    let steps = counted(&rig, "tones", "step").await;
    assert_eq!(steps, BTreeMap::from([("loud".to_string(), 4), ("as-written".to_string(), 2)]));
    assert_eq!(child(&rig, "look-2").await.labels["way"], "as-written");
    // The step that started them is the shop's own, and is under no way.
    assert!(child(&rig, "shop").await.labels.is_empty());
}

#[tokio::test]
async fn a_way_starts_a_text_of_ones_own_as_it_was_when_the_comparison_began() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let mine = Way { blueprint: Some("my-work".into()), ..handing("mine", "kind") };
    rig.compared.compare(tones(vec![way("theirs"), mine]), &user()).await.unwrap();
    ask(&rig, &[1, 2], 0).await;
    assert_eq!((rig.shared.count("plain"), rig.shared.count("mine-kind")), (1, 1));
    assert_eq!(child(&rig, "work-2").await.blueprint, "my-work");
    // Their own has two steps where the one that ships has one: counted, not judged.
    let steps = counted(&rig, "tones", "step").await;
    assert_eq!(steps, BTreeMap::from([("theirs".to_string(), 1), ("mine".to_string(), 2)]));

    // Written again while it is compared, it is not what the way starts:
    // a way that changed half way would be two ways counted as one.
    rig.library.put("my-work", &MINE.replace("mine-", "later-"));
    ask(&rig, &[3, 4], 2).await;
    assert_eq!((rig.shared.count("mine-kind"), rig.shared.count("later-kind")), (2, 0));
}

#[tokio::test]
async fn a_way_that_sends_its_text_starts_that_text_under_the_name_and_keeps_to_it() {
    let rig = Rig::new().await;
    shop(&rig).await;
    // A text with no name where the work is: what ships under a name a
    // fork stands in for, or one not saved.
    let text = WORK.replace("${ params.tone }", "sent-${ params.tone }");
    let sent = Way { source: Some(text), ..way("sent") };
    let one = rig.compared.compare(tones(vec![way("theirs"), sent]), &user()).await.unwrap();
    ask(&rig, &[1, 2], 0).await;
    assert_eq!((rig.shared.count("plain"), rig.shared.count("sent-plain")), (1, 1));
    // Started under the name of what is compared, as the text it was sent as.
    assert_eq!(child(&rig, "work-2").await.blueprint, "work");
    // What is kept of the way says it was sent, and holds no text: that is kept by its revision.
    let kept = serde_json::to_value(&one).unwrap();
    assert_eq!(kept["ways"][1]["sent"], true, "{kept}");
    assert_eq!(kept["ways"][1]["blueprint"], "work", "{kept}");
    assert!(kept["ways"][1].get("source").is_none(), "{kept}");
    assert!(kept["ways"][0].get("sent").is_none(), "{kept}");

    // Whatever the file under the name comes to, the way starts what was sent.
    rig.library.put("work", &WORK.replace("${ params.tone }", "later-${ params.tone }"));
    ask(&rig, &[3, 4], 2).await;
    assert_eq!(rig.shared.count("sent-plain"), 2);
}

/// Somebody's own way that does something first and then has the work
/// done as it ships, by calling what it stands in for.
const AROUND: &str = "\
blueprint: around-work
params: { id: 0, tone: plain }
steps:
  first: { do: test.count, with: { key: \"around-${ params.tone }\" } }
  then: { call: work, as: \"inner-${ params.id }\", with: { id: ${ params.id }, tone: ${ params.tone } } }
";

#[tokio::test]
async fn a_ways_own_text_that_calls_what_it_stands_in_for_has_the_thing_itself_started_and_counted_with_it() {
    let rig = Rig::new().await;
    shop(&rig).await;
    rig.library.put("around-work", AROUND);
    let around = Way { blueprint: Some("around-work".into()), ..handing("around", "kind") };
    rig.compared.compare(tones(vec![way("theirs"), around]), &user()).await.unwrap();
    // Given its way again where it calls the work, the text was started
    // under itself without end, and asking never came back from here.
    ask(&rig, &[1, 2], 0).await;
    assert_eq!(
        (rig.shared.count("plain"), rig.shared.count("around-kind"), rig.shared.count("kind")),
        (1, 1, 1)
    );
    assert_eq!(child(&rig, "work-2").await.blueprint, "around-work");
    let inner = child(&rig, "inner-2").await;
    assert_eq!(inner.blueprint, "work", "what the way's text calls is the work as it ships");
    assert_eq!(inner.labels.get("way").map(String::as_str), Some("around"), "and it is counted with its way");
    // One thing went each way, and nothing was started beside what was asked.
    assert_eq!(went(&rig, "tones").await, [("theirs".to_string(), 1), ("around".to_string(), 1)]);
    let all = rig.engine.instances(InstanceFilter::default()).await.unwrap();
    assert_eq!(all.iter().filter(|one| one.blueprint == "around-work").count(), 1);
    // The way's two steps and the work's one, beside the one of the work as it ships.
    let steps = counted(&rig, "tones", "step").await;
    assert_eq!(steps, BTreeMap::from([("theirs".to_string(), 1), ("around".to_string(), 3)]));
}

#[tokio::test]
async fn ending_says_which_way_everything_goes_from_then_on_and_that_can_be_said_again() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let mine = Way { blueprint: Some("my-work".into()), ..way("mine") };
    rig.compared.compare(tones(vec![way("theirs"), mine]), &user()).await.unwrap();
    ask(&rig, &[1, 2], 0).await;

    let ended = rig.compared.end_comparison("tones", Some("mine".into())).await.unwrap();
    assert_eq!(ended.goes.as_deref(), Some("mine"));
    ask(&rig, &[3, 4, 1], 2).await;
    // Everything goes that way, the thing that went the other included,
    // and none of it is counted as part of what was compared.
    assert_eq!((rig.shared.count("plain"), rig.shared.count("mine-plain")), (1, 4));
    assert_eq!(went(&rig, "tones").await, [("theirs".to_string(), 1), ("mine".to_string(), 1)]);
    assert!(child(&rig, "work-3").await.labels.is_empty());

    // Said again with no way, everything is as it is written.
    let back = rig.compared.end_comparison("tones", None).await.unwrap();
    assert!(!back.in_force());
    ask(&rig, &[5], 5).await;
    assert_eq!(rig.shared.count("plain"), 2);
    assert_eq!(child(&rig, "work-5").await.blueprint, "work");
    let said: Vec<_> = rig.events("system.comparison.ended").await;
    assert_eq!(said.len(), 2);
    assert_eq!((&said[0].data["goes"], &said[1].data["goes"]), (&json!("mine"), &json!(null)));
}

#[tokio::test]
async fn a_comparison_is_refused_where_it_could_not_be_done_and_says_why() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let refused = |asked: Compare| {
        let compared = rig.compared.clone();
        async move { compared.compare(asked, &user()).await.unwrap_err() }
    };
    let nothing = refused(Compare { of: "nowhere".into(), ..tones(vec![way("a")]) }).await;
    assert_eq!(nothing.kind, ErrorKind::NotFound, "{nothing}");
    let unknown = refused(tones(vec![Way { blueprint: Some("gone".into()), ..way("a") }])).await;
    assert!(unknown.message.contains("the way `a` starts `gone`, and there is none"), "{unknown}");
    let mut hands = handing("a", "loud");
    hands.with.insert("volume".into(), json!(11));
    let untaken = refused(tones(vec![hands])).await;
    assert!(untaken.message.contains("hands `volume`, which `work` does not take"), "{untaken}");
    let twice = refused(tones(vec![way("a"), way("a")])).await;
    assert_eq!(twice.kind, ErrorKind::Invalid, "{twice}");

    rig.compared.compare(tones(vec![way("a"), way("b")]), &user()).await.unwrap();
    let second = refused(Compare { name: "others".into(), ..tones(vec![way("a")]) }).await;
    assert!(second.message.contains("`tones` already says how `work` is started here"), "{second}");
    let no_way = rig.compared.end_comparison("tones", Some("c".into())).await.unwrap_err();
    assert!(no_way.message.contains("its ways are a, b"), "{no_way}");
    // A name is used once: what was measured under it is still there.
    rig.compared.end_comparison("tones", None).await.unwrap();
    let again = refused(tones(vec![way("a")])).await;
    assert_eq!(again.kind, ErrorKind::Conflict, "{again}");
}

#[tokio::test]
async fn a_comparison_started_by_somebody_who_holds_less_than_an_instance_runs_with_leaves_it_as_written() {
    let rig = Rig::new().await;
    shop(&rig).await;
    // The shop was applied by somebody who holds everything; this one may
    // run blueprints and no more, and so may not say what the shop runs.
    let less = Principal {
        kind: PrincipalKind::User,
        id: "bo".into(),
        name: "Bo".into(),
        scopes: vec!["blueprints".into()],
        key: None,
    };
    rig.compared.compare(tones(vec![handing("loud", "loud")]), &less).await.unwrap();
    ask(&rig, &[1], 0).await;
    assert_eq!((rig.shared.count("plain"), rig.shared.count("loud")), (1, 0));
    assert!(child(&rig, "work-1").await.labels.is_empty());
    let said = rig.event("instance.call_kept", |event| event.data["blueprint"] == "work").await;
    assert!(
        said.data["error"].as_str().unwrap().contains("does not hold everything this instance runs with")
    );
}

#[tokio::test]
async fn where_texts_are_followed_a_way_follows_its_file_and_what_each_text_did_is_counted_apart() {
    let rig = Rig::new().await;
    shop(&rig).await;
    rig.engine.set_taking(Take { come: Some("follow".into()), land: None }, &user()).await.unwrap();
    let mine = Way { blueprint: Some("my-work".into()), ..way("mine") };
    rig.compared.compare(tones(vec![way("theirs"), mine]), &user()).await.unwrap();
    ask(&rig, &[1, 2], 0).await;
    assert_eq!(rig.shared.count("mine-plain"), 1);

    // Their own way is written again. Followed, it is what the way starts from now.
    let later = MINE.replace("mine-", "later-");
    rig.library.put("my-work", &later);
    ask(&rig, &[3, 4], 2).await;
    assert_eq!((rig.shared.count("mine-plain"), rig.shared.count("later-plain")), (1, 1));

    // The way says each text it ran, the one it runs now last.
    let short = |text: &str| blueprint::revision(text)[..8].to_string();
    let one = rig.compared.comparison("tones").await.unwrap();
    let ran = |way: usize| one.ways[way].texts.iter().map(|ran| ran.text.clone()).collect::<Vec<_>>();
    assert_eq!((ran(0), ran(1)), (vec![short(WORK)], vec![short(MINE), short(&later)]));
    assert_eq!(one.ways[1].texts[1].version, None, "it says no version, and is told by its revision");
    // And what was measured under one text is never added to what was under another.
    rig.journal.flush_metrics().await.unwrap();
    let query = MetricQuery {
        name: "step".into(),
        group_by: vec!["way".into(), "text".into()],
        labels: vec![("compared".into(), "tones".into())],
        ..Default::default()
    };
    let series = rig.journal.query_metrics(query).await.unwrap();
    let counted: BTreeMap<(String, String), u64> = series
        .into_iter()
        .map(|one| ((one.labels["way"].clone(), one.labels["text"].clone()), one.total.count))
        .collect();
    let of = |way: &str, text: &str| (way.to_string(), short(text));
    assert_eq!(
        counted,
        BTreeMap::from([(of("theirs", WORK), 2), (of("mine", MINE), 2), (of("mine", &later), 2)])
    );
}

pub(crate) fn count(name: &str, topic: &str, by: &[&str]) -> Counted {
    Counted {
        name: name.into(),
        topic: topic.into(),
        thing: "id".into(),
        by: by.iter().map(|field| field.to_string()).collect(),
        accepts: Vec::new(),
        once: Vec::new(),
    }
}

/// What was counted of what came of the things of `name`: for each way,
/// count and what it came to, how many, and the longest it took.
pub(crate) async fn came(rig: &Rig, name: &str) -> BTreeMap<(String, String, String), (u64, f64)> {
    rig.journal.flush_metrics().await.unwrap();
    let query = MetricQuery {
        name: CAME.into(),
        group_by: vec!["way".into(), "counted".into(), "came".into()],
        labels: vec![("compared".into(), name.into())],
        ..Default::default()
    };
    let series = rig.journal.query_metrics(query).await.unwrap();
    let said = |one: &journal::MetricSeries, label: &str| one.labels[label].clone();
    series
        .iter()
        // How many things each way was given is counted beside these, and asked for apart (`given`).
        .filter(|one| one.labels["counted"] != GIVEN)
        .map(|one| {
            ((said(one, "way"), said(one, "counted"), said(one, "came")), (one.total.count, one.total.max))
        })
        .collect()
}

/// How many things each text of each way of `name` was given.
async fn given(rig: &Rig, name: &str) -> Vec<(String, String, u64)> {
    rig.journal.flush_metrics().await.unwrap();
    let query = MetricQuery {
        name: CAME.into(),
        group_by: vec!["way".into(), "text".into()],
        labels: vec![("compared".into(), name.into()), ("counted".into(), GIVEN.into())],
        ..Default::default()
    };
    let series = rig.journal.query_metrics(query).await.unwrap();
    let mut all: Vec<(String, String, u64)> = series
        .iter()
        .map(|one| (one.labels["way"].clone(), one.labels["text"].clone(), one.total.count))
        .collect();
    all.sort();
    all
}

pub(crate) async fn say(rig: &Rig, topic: &str, data: serde_json::Value) {
    rig.journal.emit(topic, Source::default(), data).await.unwrap();
}

#[tokio::test]
async fn what_came_of_each_thing_is_counted_under_its_way_with_how_long_since_it_was_given_it() {
    let rig = Rig::new().await;
    shop(&rig).await;
    // Whoever starts it says which events say what came of a thing, and by which of their fields.
    let counts = vec![
        count("ended", "shop.finished", &["how"]),
        count("ended", "shop.dropped", &[]),
        count("moved", "shop.moved", &["from", "to"]),
    ];
    rig.compared.compare(Compare { counts, ..tones(vec![way("a"), way("b")]) }, &user()).await.unwrap();
    ask(&rig, &[1, 2, 3, 4], 0).await;

    // Half a minute on, what looks after the things says what came of them.
    rig.clock.advance(Duration::from_secs(30));
    say(&rig, "shop.finished", json!({ "id": 1, "how": "well" })).await;
    say(&rig, "shop.finished", json!({ "id": 2, "how": "badly" })).await;
    say(&rig, "shop.moved", json!({ "id": 3, "from": "bench", "to": "shelf" })).await;
    say(&rig, "shop.dropped", json!({ "id": 4 })).await;
    // Of something the comparison gave no way, and of something nobody asked to have counted: nothing.
    say(&rig, "shop.finished", json!({ "id": 99, "how": "well" })).await;
    say(&rig, "shop.painted", json!({ "id": 1 })).await;
    let of = |way: &str, counted: &str, came: &str| (way.to_string(), counted.to_string(), came.to_string());
    let wanted = BTreeMap::from([
        (of("a", "ended", "well"), (1, 30_000.0)),
        (of("b", "ended", "badly"), (1, 30_000.0)),
        (of("a", "moved", "bench>shelf"), (1, 30_000.0)),
        (of("b", "ended", "dropped"), (1, 30_000.0)),
    ]);
    crate::soon("each to be counted", async {
        while came(&rig, "tones").await != wanted {
            tokio::task::yield_now().await;
        }
    })
    .await;
    // Every thing is counted once as given to the text of its way it ran, whatever is asked to be
    // counted: what is added up under a way is divided by this.
    let text = blueprint::revision(WORK)[..8].to_string();
    assert_eq!(given(&rig, "tones").await, [("a".to_string(), text.clone(), 2), ("b".to_string(), text, 2)]);
    let taken = Compare { counts: vec![count("given", "shop.finished", &[])], ..tones(vec![way("a")]) };
    let refused = rig.compared.compare(Compare { name: "other".into(), ..taken }, &user()).await.unwrap_err();
    assert!(refused.message.contains("a count cannot be called `given`"), "{refused}");
    // Each is under the text its thing last ran, as everything measured of the thing is.
    let texts = MetricQuery { name: CAME.into(), group_by: vec!["text".into()], ..Default::default() };
    let by_text = rig.journal.query_metrics(texts).await.unwrap();
    // Four things given, and four things that came of them.
    assert_eq!((by_text.len(), by_text[0].total.count), (1, 8), "{by_text:?}");
    assert_eq!(by_text[0].labels["text"], blueprint::revision(WORK)[..8]);

    // Ended, it counts no more: the place it counted from is put away.
    rig.compared.end_comparison("tones", None).await.unwrap();
    crate::soon("the counting to be over", async {
        while rig.journal.kv_get("compared.counted", "cursor").await.unwrap().is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    say(&rig, "shop.finished", json!({ "id": 1, "how": "again" })).await;
    assert_eq!(came(&rig, "tones").await, wanted);
}

/// How many of `counted` were counted under `name`, whatever they came to.
async fn how_many(rig: &Rig, name: &str, counted: &str) -> u64 {
    came(rig, name).await.iter().filter(|(of, _)| of.1 == counted).map(|(_, (count, _))| count).sum()
}

#[tokio::test]
async fn the_place_events_are_counted_from_is_kept_before_whoever_started_the_comparison_is_answered() {
    let rig = Rig::new().await;
    shop(&rig).await;
    // Not when whoever follows the journal comes to be run: what is said the moment after a
    // comparison was started is after its place, and is counted.
    let before = rig.journal.last_seq();
    let asked = Compare { counts: vec![count("ended", "shop.finished", &[])], ..tones(vec![way("a")]) };
    rig.compared.compare(asked, &user()).await.unwrap();
    // A place in each journal: here, in the one there is.
    let kept = rig.journal.kv_get("compared.counted", "cursor").await.unwrap();
    let place = kept.and_then(|kept| journal::Places::from_kept(&kept)).map(|kept| kept.of(journal::FACTORY));
    // Where the journal was as it was started: before its being started was said.
    let started = rig.events("system.comparison.started").await[0].seq;
    assert!(
        place.is_some_and(|place| before <= place && place < started),
        "{place:?} of {before}..{started}"
    );
}

#[tokio::test]
async fn what_is_counted_having_changed_is_taken_in_by_one_that_was_in_the_middle_of_counting() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let asked = Compare { counts: vec![count("ended", "shop.finished", &[])], ..tones(vec![way("a")]) };
    rig.compared.compare(asked, &user()).await.unwrap();
    ask(&rig, &[1], 0).await;

    // A great many events at once: whoever counts is in the middle of them for a while. A second
    // comparison, of what else is started for each thing, is started in that while. It was told
    // only where it waited for an event, and one in the middle of one never came to follow the
    // second comparison's events.
    let many = 40;
    for _ in 0..many {
        say(&rig, "shop.finished", json!({ "id": 1 })).await;
    }
    let looks = Compare {
        name: "looks".into(),
        of: "look".into(),
        counts: vec![count("seen", "shop.seen", &[])],
        ..tones(vec![way("a")])
    };
    rig.compared.compare(looks, &user()).await.unwrap();
    ask(&rig, &[2], 1).await;
    say(&rig, "shop.seen", json!({ "id": 2 })).await;
    crate::soon("what the second comparison counts to be counted", async {
        while how_many(&rig, "looks", "seen").await != 1 || how_many(&rig, "tones", "ended").await != many {
            tokio::task::yield_now().await;
        }
    })
    .await;

    // And both ended while it is in the middle of as many again: it counts no more, and puts its
    // place away, though it was waiting for nothing when it was told.
    for _ in 0..many {
        say(&rig, "shop.finished", json!({ "id": 1 })).await;
    }
    rig.compared.end_comparison("tones", None).await.unwrap();
    rig.compared.end_comparison("looks", None).await.unwrap();
    crate::soon("the counting to be over", async {
        while rig.journal.kv_get("compared.counted", "cursor").await.unwrap().is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    // Nothing said once a comparison has been ended is counted under it.
    let ended = (how_many(&rig, "tones", "ended").await, how_many(&rig, "looks", "seen").await);
    say(&rig, "shop.finished", json!({ "id": 1 })).await;
    say(&rig, "shop.seen", json!({ "id": 2 })).await;
    // One started after that is counted from where it is started, by another that follows.
    let third = Compare {
        name: "third".into(),
        counts: vec![count("ended", "shop.finished", &[])],
        ..tones(vec![way("a")])
    };
    rig.compared.compare(third, &user()).await.unwrap();
    ask(&rig, &[3], 2).await;
    say(&rig, "shop.finished", json!({ "id": 3 })).await;
    crate::soon("what is said of the third to be counted", async {
        while how_many(&rig, "third", "ended").await != 1 {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert_eq!((how_many(&rig, "tones", "ended").await, how_many(&rig, "looks", "seen").await), ended);
}

#[tokio::test]
async fn what_is_counted_goes_on_being_counted_when_the_conductor_is_stopped_and_started() {
    let mut rig = Rig::new().await;
    shop(&rig).await;
    let asked =
        Compare { counts: vec![count("ended", "shop.finished", &[])], ..tones(vec![way("a"), way("b")]) };
    rig.compared.compare(asked, &user()).await.unwrap();
    // Said of a thing before it was given a way: it did not come of it, and is not counted when it
    // is met again from the place kept, by when the thing has one.
    say(&rig, "shop.finished", json!({ "id": 2 })).await;
    rig.clock.advance(Duration::from_secs(1));
    ask(&rig, &[1, 2], 0).await;
    // Said while nothing was there to hear it: counted when the conductor is back.
    rig.engine.halt();
    say(&rig, "shop.finished", json!({ "id": 2 })).await;
    rig.restart().await;
    let wanted = BTreeMap::from([(("b".to_string(), "ended".to_string(), "finished".to_string()), (1, 0.0))]);
    crate::soon("what was said meanwhile to be counted", async {
        while came(&rig, "tones").await != wanted {
            tokio::task::yield_now().await;
        }
    })
    .await;
}

#[tokio::test]
async fn a_count_is_refused_where_it_is_not_one_or_whoever_asks_may_not_be_told_of_the_event() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let with = |one: Counted| Compare { counts: vec![one], ..tones(vec![way("a")]) };
    let unnamed =
        rig.compared.compare(with(count("Came Of It", "shop.finished", &[])), &user()).await.unwrap_err();
    assert!(unnamed.message.contains("cannot be what a count is called"), "{unnamed}");
    let no_topic = rig.compared.compare(with(count("ended", "finished", &[])), &user()).await.unwrap_err();
    assert!(no_topic.message.contains("is not an event's topic"), "{no_topic}");
    let no_thing = Counted { thing: String::new(), ..count("ended", "shop.finished", &[]) };
    let unsaid = rig.compared.compare(with(no_thing), &user()).await.unwrap_err();
    assert!(unsaid.message.contains("does not say which field"), "{unsaid}");
    // Somebody who may run blueprints and may not read what runs is not counted `run.finished` for.
    let narrow = Principal {
        kind: PrincipalKind::User,
        id: "bo".into(),
        name: "Bo".into(),
        scopes: vec!["blueprints:run".into()],
        key: None,
    };
    let kept_from =
        rig.compared.compare(with(count("ended", "run.finished", &[])), &narrow).await.unwrap_err();
    assert_eq!(kept_from.kind, ErrorKind::Denied, "{kept_from}");
    assert!(rig.compared.comparisons(None).await.is_empty(), "nothing was started");
}

/// The work as a later program ships it.
const WORK_LATER: &str = "\
blueprint: work
version: 2
params: { id: 0, tone: plain }
steps:
  claim: { do: test.count, with: { key: \"later-${ params.tone }\" } }
";

#[tokio::test]
async fn a_way_named_by_a_version_starts_the_text_the_factory_kept_and_is_refused_where_it_kept_none() {
    let rig = Rig::new().await;
    // What the library holds came with the program, so a running tree is held to it as it reached it.
    rig.library.ship();
    let first = WORK.replace("blueprint: work\n", "blueprint: work\nversion: 1\n");
    for (name, text) in [("shop", SHOP), ("work", first.as_str()), ("my-work", MINE), ("look", LOOK)] {
        rig.library.put(name, text);
    }
    let applied = rig.apply(SHOP, json!({})).await;
    active(&rig, &applied.instance).await;
    // A later program ships another, and somebody takes it: the tree can still be put back on the first.
    rig.library.put("work", WORK_LATER);
    rig.engine.apply(Apply { take: true, ..rig.request(SHOP, json!({})) }).await.unwrap();

    // The one from before against the one there now, each thing one way.
    let before = Way { version: Some("1".into()), ..way("before") };
    let started = rig.compared.compare(tones(vec![before.clone(), way("now")]), &user()).await.unwrap();
    assert_eq!(
        (started.ways[0].blueprint.as_deref(), started.ways[0].revision.as_deref()),
        (Some("work"), Some(blueprint::revision(&first).as_str())),
        "it is held to the text that called itself that"
    );
    ask(&rig, &[1, 2], 0).await;
    assert_eq!((rig.shared.count("plain"), rig.shared.count("later-plain")), (1, 1));
    assert_eq!(child(&rig, "work-1").await.revision, blueprint::revision(&first));
    assert_eq!(child(&rig, "work-2").await.revision, blueprint::revision(WORK_LATER));
    // The one there now is a version too, and is found without having been kept.
    rig.compared.end_comparison("tones", None).await.unwrap();
    let now = Way { version: Some("2".into()), ..way("same") };
    rig.compared.compare(Compare { name: "again".into(), ..tones(vec![now]) }, &user()).await.unwrap();
    rig.compared.end_comparison("again", None).await.unwrap();

    // A version nothing running reached is not kept: refused, saying what there is and what to do.
    let never = Way { version: Some("7".into()), ..way("before") };
    let refused = rig
        .compared
        .compare(Compare { name: "never".into(), ..tones(vec![never]) }, &user())
        .await
        .unwrap_err();
    assert_eq!(refused.kind, ErrorKind::NotFound, "{refused}");
    for said in [
        "the way `before`: is `work` at version 7, and the factory has not kept that version",
        "What is there now calls itself version 2, and what is kept besides is version 1",
        "under a name of its own and name that as the way",
    ] {
        assert!(refused.message.contains(said), "{refused}");
    }
}

/// The work, which says what a file it links says of the thing it is for.
const TOLD: &str = "\
blueprint: work
params: { id: 0 }
steps:
  words: { do: text.put, with: { parts: [{ from: work.md, with: { id: \"${ params.id }\" } }] } }
  say: { do: emit, with: { topic: shop.told, id: ${ params.id }, text: \"${ steps.words.out.text }\" } }
";

/// What the work says of the thing `id`, once it is asked for.
async fn told(rig: &Rig, id: u64) -> String {
    rig.journal.emit("test.poke", Source::default(), json!({ "id": id })).await.unwrap();
    let said = rig.event("shop.told", move |event| event.data["id"] == id).await;
    said.data["text"].as_str().unwrap().to_string()
}

/// A way that reads `other` where the work links its file.
fn reading(name: &str, other: &str) -> Way {
    let reads = Reads { name: other.into(), ..Default::default() };
    Way { prompts: [("prompts/work.md".to_string(), reads)].into(), ..way(name) }
}

#[tokio::test]
async fn a_way_reads_other_words_where_a_file_is_linked_held_or_followed_as_texts_are() {
    let rig = Rig::new().await;
    for (name, text) in [("shop", SHOP), ("work", TOLD), ("look", LOOK)] {
        rig.library.put(name, text);
    }
    rig.library.put("prompts/work.md", "Make ${id}.\n");
    rig.library.put("prompts/other.md", "Make ${id} with care.\n");
    let applied = rig.apply(SHOP, json!({})).await;
    active(&rig, &applied.instance).await;

    // What is wrong with what a way is to read is said when the comparison is started.
    let refused = async |way: Way, said: &str| {
        let wrong = rig.compared.compare(tones(vec![way]), &user()).await.unwrap_err();
        assert!(wrong.message.contains(said), "{wrong}");
    };
    refused(reading("a", "prompts/nowhere.md"), "and there is no such file: write it as").await;
    refused(reading("a", "other"), "`other` is not the name of such a file").await;
    let elsewhere = Reads { name: "prompts/other.md".into(), ..Default::default() };
    let unlinked = Way { prompts: [("prompts/look.md".to_string(), elsewhere)].into(), ..way("a") };
    refused(unlinked, "nothing this comparison starts links that file. What they link: prompts/work.md")
        .await;
    rig.library.put("prompts/unfit.md", "Make ${id} in ${place}.\n");
    refused(reading("a", "prompts/unfit.md"), "its blank `${place}` is handed nothing").await;

    // Two prompts for one step, with nothing else changed: each thing goes one way and keeps to it.
    let started =
        rig.compared.compare(tones(vec![way("usual"), reading("other", "prompts/other.md")]), &user()).await;
    let started = started.unwrap();
    let reads = &started.ways[1].prompts["prompts/work.md"];
    assert_eq!(reads.revision.as_deref(), Some(blueprint::revision("Make ${id} with care.\n").as_str()));
    assert_eq!(
        reads.handed,
        ["id"],
        "what links the file hands this, which is what the other may have blanks for"
    );
    assert_eq!((told(&rig, 1).await, told(&rig, 2).await), ("Make 1.".into(), "Make 2 with care.".into()));
    // Texts are held: the words are the ones from when the comparison was started, whatever the file comes to.
    rig.library.put("prompts/other.md", "Make ${id} twice.\n");
    assert_eq!((told(&rig, 3).await, told(&rig, 4).await), ("Make 3.".into(), "Make 4 with care.".into()));
    // Which words each way ran is kept with the text it ran: one column a way.
    let shown = rig.compared.comparison("tones").await.unwrap();
    let words = |one: &comparisons::Comparison, way: usize| -> Vec<Vec<String>> {
        one.ways[way].texts.iter().map(|ran| ran.prompts.values().cloned().collect()).collect()
    };
    assert_eq!(words(&shown, 0), [Vec::<String>::new()]);
    assert_eq!(words(&shown, 1), [vec![blueprint::revision("Make ${id} with care.\n")]]);
    assert_ne!(shown.ways[0].texts[0].text, shown.ways[1].texts[0].text, "other words are counted apart");
    // Ended saying that way goes on: everything reads the other words, uncounted.
    rig.compared.end_comparison("tones", Some("other".into())).await.unwrap();
    assert_eq!(told(&rig, 5).await, "Make 5 with care.");
    rig.compared.end_comparison("tones", None).await.unwrap();
    assert_eq!(told(&rig, 6).await, "Make 6.");

    // Where texts are followed a way's words follow their file, and each it ran is counted apart.
    rig.engine.set_taking(Take { come: Some("follow".into()), land: None }, &user()).await.unwrap();
    let again =
        Compare { name: "again".into(), ..tones(vec![way("usual"), reading("other", "prompts/other.md")]) };
    rig.compared.compare(again, &user()).await.unwrap();
    assert_eq!((told(&rig, 7).await, told(&rig, 8).await), ("Make 7.".into(), "Make 8 twice.".into()));
    rig.library.put("prompts/other.md", "Make ${id} thrice.\n");
    assert_eq!((told(&rig, 9).await, told(&rig, 10).await), ("Make 9.".into(), "Make 10 thrice.".into()));
    // One that can no longer be filled is not taken: it goes on with the last that could.
    rig.library.put("prompts/other.md", "Make ${id} in ${place}.\n");
    assert_eq!((told(&rig, 11).await, told(&rig, 12).await), ("Make 11.".into(), "Make 12 thrice.".into()));
    let shown = rig.compared.comparison("again").await.unwrap();
    let ran = words(&shown, 1);
    let twice_then_thrice =
        [vec![blueprint::revision("Make ${id} twice.\n")], vec![blueprint::revision("Make ${id} thrice.\n")]];
    assert_eq!(ran, twice_then_thrice, "a column for each of the words it ran");
    assert!(rig.events("run.failed").await.is_empty());
}

#[tokio::test]
async fn what_a_thing_reads_under_its_way_is_read_by_what_it_starts_and_after_a_restart() {
    let mut rig = Rig::new().await;
    // The work is a step further down: what is started for the thing starts what links the file.
    let outer = "blueprint: outer\nparams: { id: 0 }\nsteps:\n  go: { call: work, as: inner, with: { id: ${ params.id } } }\n";
    let shop = SHOP.replace("call: work, as: \"work-", "call: outer, as: \"work-");
    assert_ne!(shop, SHOP);
    for (name, text) in [("shop", shop.as_str()), ("outer", outer), ("work", TOLD), ("look", LOOK)] {
        rig.library.put(name, text);
    }
    rig.library.put("prompts/work.md", "Make ${id}.\n");
    rig.library.put("prompts/other.md", "Make ${id} with care.\n");
    let applied = rig.apply(&shop, json!({})).await;
    active(&rig, &applied.instance).await;
    let asked =
        Compare { of: "outer".into(), ..tones(vec![way("usual"), reading("other", "prompts/other.md")]) };
    rig.compared.compare(asked, &user()).await.unwrap();
    assert_eq!((told(&rig, 1).await, told(&rig, 2).await), ("Make 1.".into(), "Make 2 with care.".into()));
    handled(&rig, "on.asked", 2).await;
    rig.restart().await;
    assert_eq!((told(&rig, 3).await, told(&rig, 4).await), ("Make 3.".into(), "Make 4 with care.".into()));
}

/// The keys a journal keeps under `name` in its small durable state.
#[tokio::test]
async fn what_came_of_a_projects_things_is_counted_while_its_conductor_is_stopped_alone_and_after() {
    if crate::one_journal() {
        return;
    }
    let rig = Rig::new().await;
    rig.journal.keeps_projects_in_memory().await.unwrap();
    let own = rig.journal.own("yard").await.unwrap();
    for (name, text) in [("shop", SHOP), ("work", WORK), ("look", LOOK)] {
        rig.library.put(name, text);
    }
    let asked = conductor::Apply { project: Some("yard".into()), ..rig.request(SHOP, json!({})) };
    rig.engine.apply(asked).await.unwrap();
    // What is said of the project is in its own journal: waited for there.
    let said = async |topic: &'static str, times: usize| {
        let mut follow = own.follow(0, topic);
        crate::soon(&format!("`{topic}` to be said {times} times"), async {
            for _ in 0..times {
                follow.next().await.unwrap();
            }
        })
        .await;
    };
    said("instance.active", 1).await;
    let counts = vec![count("ended", "shop.finished", &[])];
    let asked = Compare { project: Some("yard".into()), counts, ..tones(vec![way("a")]) };
    rig.compared.compare(asked, &user()).await.unwrap();
    let thing = async |id: u64| {
        own.emit("test.poke", Source::default(), json!({ "project": "yard", "id": id })).await.unwrap();
        said("instance.finished", 2 * id as usize).await;
    };
    let ended = async |id: u64| {
        own.emit("shop.finished", Source::default(), json!({ "project": "yard", "id": id })).await.unwrap();
        crate::soon("what came of the thing to be counted", async {
            while how_many(&rig, "tones", "ended").await != id {
                tokio::task::yield_now().await;
            }
        })
        .await;
    };
    thing(1).await;

    // The project's conductor is stopped and no other: what counts is
    // the factory's, follows every journal, and goes on.
    assert!(rig.engine.halt_one("yard"));
    rig.engine.halted_one("yard").await;
    ended(1).await;
    // Started again, what it is given goes a way as before, and what
    // comes of that is counted.
    rig.engine.start_one("yard").await.unwrap();
    thing(2).await;
    ended(2).await;
    assert_eq!(went(&rig, "tones").await, [("a".to_string(), 2)]);
}

async fn keys(journal: &journal::Journal, name: &'static str) -> Vec<String> {
    let read = journal.db().call(move |connection| {
        let mut statement = connection.prepare("SELECT key FROM kv WHERE namespace = ?1 ORDER BY key")?;
        let rows = statement.query_map([name], |row| row.get(0))?;
        rows.collect::<rusqlite::Result<Vec<String>>>()
    });
    read.await.unwrap()
}

#[tokio::test]
async fn what_a_thing_of_a_project_reads_under_its_way_is_kept_in_the_projects_own_journal() {
    if crate::one_journal() {
        return;
    }
    let mut rig = Rig::new().await;
    // The project has a journal of its own, and so a conductor of its
    // own: what is applied for it is kept there, with what is its.
    rig.journal.keeps_projects_in_memory().await.unwrap();
    let own = rig.journal.own("yard").await.unwrap();
    for (name, text) in [("shop", SHOP), ("work", TOLD), ("look", LOOK)] {
        rig.library.put(name, text);
    }
    rig.library.put("prompts/work.md", "Make ${id}.\n");
    rig.library.put("prompts/other.md", "Make ${id} with care.\n");
    let asked = conductor::Apply { project: Some("yard".into()), ..rig.request(SHOP, json!({})) };
    let shop = rig.engine.apply(asked).await.unwrap().instance;
    let its = shop.clone();
    let mut active = own.follow(0, "instance.active");
    crate::soon("the shop to be active", async {
        while active.next().await.unwrap().source.instance.as_deref() != Some(its.as_str()) {}
    })
    .await;
    let told = async |id: u64| {
        own.emit("test.poke", Source::default(), json!({ "project": "yard", "id": id })).await.unwrap();
        let mut said = own.follow(0, "shop.told");
        crate::soon("the work to say what it was told", async {
            loop {
                let event = said.next().await.unwrap();
                if event.data["id"] == id {
                    return event.data["text"].as_str().unwrap().to_string();
                }
            }
        })
        .await
    };
    let asked = Compare {
        project: Some("yard".into()),
        ..tones(vec![way("usual"), reading("other", "prompts/other.md")])
    };
    rig.compared.compare(asked, &user()).await.unwrap();
    assert_eq!((told(1).await, told(2).await), ("Make 1.".into(), "Make 2 with care.".into()));

    // What the second reads is written for what started it, which is
    // the project's: beside its rows, and not in the factory's journal.
    const READS: &str = "compared.reads";
    assert_eq!(keys(&own, READS).await, [format!("{shop}\nwork-2")]);
    assert!(keys(&rig.journal, READS).await.is_empty());
    // The comparison itself is the factory's.
    assert_eq!(keys(&rig.journal, "compared").await, ["tones"]);
    assert!(keys(&own, "compared").await.is_empty());

    // Read from there when the conductors start again, and put away
    // from there once what started it is gone.
    rig.restart().await;
    assert_eq!((told(3).await, told(4).await), ("Make 3.".into(), "Make 4 with care.".into()));
    assert_eq!(keys(&own, READS).await, [format!("{shop}\nwork-2"), format!("{shop}\nwork-4")]);
    rig.engine.stop(&shop).await.unwrap();
    rig.restart().await;
    assert!(keys(&own, READS).await.is_empty(), "kept for what a stopped instance started");
    assert!(keys(&rig.journal, READS).await.is_empty());
}

#[tokio::test]
async fn what_a_comparison_counts_of_what_the_runner_says_of_its_runs_is_kept_while_it_counts() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let kept = |rig: &Rig| {
        let journal = rig.journal.clone();
        async move {
            let query =
                journal::EventQuery { topics: "run.finished".into(), limit: 1000, ..Default::default() };
            journal.read(query).await.unwrap().len()
        }
    };
    // Nothing here hears that a run finished: it is passed on, and the
    // journal keeps none of it.
    assert_eq!(kept(&rig).await, 0);
    // Counted, it is read again from the place counting keeps, so it is
    // kept from when the comparison is started.
    let asked = Compare { counts: vec![count("ended", "run.finished", &[])], ..tones(vec![way("a")]) };
    rig.compared.compare(asked, &user()).await.unwrap();
    ask(&rig, &[1], 0).await;
    assert!(kept(&rig).await > 0, "what is counted is kept");
}
