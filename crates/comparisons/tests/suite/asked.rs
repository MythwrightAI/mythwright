//! A comparison whose ways are named by the start: it speaks only for a
//! start that names it and one of its ways, gives it that way whatever
//! the shares, and stands beside any number of others of one blueprint;
//! and a blueprint starts and ends one, the same done twice.

use std::collections::BTreeMap;
use std::time::Duration;

use comparisons::{Compare, Way};
use kernel::{ErrorKind, Principal, PrincipalKind};
use serde_json::json;

use crate::compared::{came, child, count, say};
use crate::{Rig, active, handled, user};

/// What somebody applies: for each thing asked for it starts the work,
/// handing on which comparison and which way the asking names, if any.
const ASKS: &str = "\
blueprint: asks
on:
  asked:
    event: test.named
    steps:
      done:
        call: work
        as: \"${ event.data.key }\"
        with:
          id: \"${ event.data.key }\"
          compared: \"${ event.data.compared ?? '' }\"
          way: \"${ event.data.way ?? '' }\"
";

/// The work, which takes the two params a start names a comparison by:
/// it counts under the tone it is handed.
const WORK: &str = "\
blueprint: work
params: { id: '', tone: plain, compared: '', way: '' }
steps:
  claim: { do: test.count, with: { key: \"${ params.tone }\" } }
";

pub(crate) async fn asks(rig: &Rig) {
    rig.library.put("asks", ASKS);
    rig.library.put("work", WORK);
    let applied = rig.apply(ASKS, json!({})).await;
    active(rig, &applied.instance).await;
}

/// Asks for each `(key, comparison, way)` one after another, `before`
/// having been asked already.
pub(crate) async fn ask(rig: &Rig, each: &[(&str, &str, &str)], before: usize) {
    for (at, (key, compared, way)) in each.iter().enumerate() {
        let mut data = json!({ "key": key });
        if !compared.is_empty() {
            data["compared"] = json!(compared);
        }
        if !way.is_empty() {
            data["way"] = json!(way);
        }
        say(rig, "test.named", data).await;
        handled(rig, "on.asked", before + at + 1).await;
        let failed = rig.events("run.failed").await;
        assert!(failed.is_empty(), "{:?}", failed.last().map(|event| &event.data));
    }
}

pub(crate) fn tone(name: &str, share: u32) -> Way {
    Way {
        name: name.into(),
        share,
        with: json!({ "tone": name }).as_object().unwrap().clone(),
        ..Default::default()
    }
}

pub(crate) fn asked(name: &str, ways: Vec<Way>) -> Compare {
    Compare { name: name.into(), of: "work".into(), ways, asked: true, ..Default::default() }
}

/// The way each child was started under, by its key.
async fn under(rig: &Rig, key: &str) -> (String, String) {
    let one = child(rig, key).await;
    let label = |name: &str| one.labels.get(name).cloned().unwrap_or_default();
    (label("compared"), label("way"))
}

fn pair(compared: &str, way: &str) -> (String, String) {
    (compared.to_string(), way.to_string())
}

/// What each instance that was left as it was written was told, by the
/// key of what was started.
async fn left(rig: &Rig) -> BTreeMap<String, String> {
    let said = rig.events("instance.call_kept").await;
    let text = |event: &kernel::Event, key: &str| event.data[key].as_str().unwrap_or_default().to_string();
    said.iter().map(|event| (text(event, "key"), text(event, "error"))).collect()
}

#[tokio::test]
async fn a_start_that_names_its_way_is_given_that_way_whatever_the_shares_and_keeps_to_it() {
    let mut rig = Rig::new().await;
    asks(&rig).await;
    // Shares that would send nearly everything one way.
    rig.compared.compare(asked("ab", vec![tone("loud", 9), tone("quiet", 1)]), &user()).await.unwrap();
    ask(&rig, &[("t1", "ab", "quiet"), ("t2", "ab", "quiet"), ("t3", "ab", "loud")], 0).await;
    assert_eq!((rig.shared.count("loud"), rig.shared.count("quiet")), (1, 2));
    assert_eq!(under(&rig, "t1").await, pair("ab", "quiet"));
    assert_eq!(under(&rig, "t3").await, pair("ab", "loud"));
    let one = rig.compared.comparison("ab").await.unwrap();
    let went: Vec<u64> = one.ways.iter().map(|way| way.went).collect();
    assert_eq!(went, [1, 2]);
    assert!(one.asked);

    // The same thing named again goes the same way and is not counted
    // twice, across a restart too.
    rig.restart().await;
    ask(&rig, &[("t1", "ab", "quiet")], 3).await;
    assert_eq!(rig.shared.count("quiet"), 3);
    let one = rig.compared.comparison("ab").await.unwrap();
    assert_eq!(one.ways.iter().map(|way| way.went).collect::<Vec<_>>(), [1, 2]);
}

#[tokio::test]
async fn a_start_that_names_nothing_is_no_asked_comparisons_and_one_that_shares_ways_out_goes_on_beside_it() {
    let rig = Rig::new().await;
    asks(&rig).await;
    rig.compared.compare(asked("ab", vec![tone("loud", 1), tone("quiet", 1)]), &user()).await.unwrap();
    // Nothing named: started as written, and under no way.
    ask(&rig, &[("plain-1", "", "")], 0).await;
    assert_eq!(rig.shared.count("plain"), 1);
    assert!(child(&rig, "plain-1").await.labels.is_empty());
    // One that shares its ways out may be in force for the same blueprint
    // beside it: it takes what names nothing, and nothing that names `ab`.
    let shared = Compare {
        name: "shares".into(),
        of: "work".into(),
        ways: vec![tone("kind", 1)],
        ..Default::default()
    };
    rig.compared.compare(shared, &user()).await.unwrap();
    ask(&rig, &[("s-1", "", ""), ("n-1", "ab", "loud")], 1).await;
    assert_eq!(under(&rig, "s-1").await, pair("shares", "kind"));
    assert_eq!(under(&rig, "n-1").await, pair("ab", "loud"));
    // Naming only one of the two names nothing.
    ask(&rig, &[("half", "ab", "")], 3).await;
    assert_eq!(under(&rig, "half").await, pair("shares", "kind"));
}

#[tokio::test]
async fn any_number_of_asked_comparisons_of_one_blueprint_are_in_force_together_each_with_any_number_of_ways()
{
    let rig = Rig::new().await;
    asks(&rig).await;
    // Ten ways in one, and another of the same blueprint beside it.
    let ten: Vec<Way> = (1..=10).map(|at| tone(&format!("w{at}"), 1)).collect();
    rig.compared.compare(asked("ten", ten), &user()).await.unwrap();
    rig.compared.compare(asked("two", vec![tone("loud", 1), tone("quiet", 1)]), &user()).await.unwrap();
    let each: Vec<(String, String)> = (1..=10).map(|at| (format!("k{at}"), format!("w{at}"))).collect();
    let mut asking: Vec<(&str, &str, &str)> =
        each.iter().rev().map(|(key, way)| (key.as_str(), "ten", way.as_str())).collect();
    asking.push(("x", "two", "quiet"));
    ask(&rig, &asking, 0).await;
    for (key, way) in &each {
        assert_eq!(under(&rig, key).await, pair("ten", way));
        assert_eq!(rig.shared.count(way), 1);
    }
    assert_eq!(under(&rig, "x").await, pair("two", "quiet"));
    let one = rig.compared.comparison("ten").await.unwrap();
    assert!(one.ways.iter().all(|way| way.went == 1));
}

#[tokio::test]
async fn a_start_that_names_what_cannot_be_given_is_started_as_written_and_says_why() {
    let rig = Rig::new().await;
    asks(&rig).await;
    rig.compared.compare(asked("ab", vec![tone("loud", 1), tone("quiet", 1)]), &user()).await.unwrap();
    let shared = Compare {
        name: "shares".into(),
        of: "work".into(),
        ways: vec![tone("kind", 1)],
        ..Default::default()
    };
    rig.compared.compare(shared, &user()).await.unwrap();
    ask(
        &rig,
        &[
            ("no-way", "ab", "shrill"),
            ("nobody", "missing", "loud"),
            ("not-asked", "shares", "kind"),
            ("t1", "ab", "loud"),
        ],
        0,
    )
    .await;
    // The thing that went one way, named again with the other.
    ask(&rig, &[("t1", "ab", "quiet")], 4).await;
    rig.compared.end_comparison("ab", None).await.unwrap();
    ask(&rig, &[("late", "ab", "loud")], 5).await;
    for key in ["no-way", "nobody", "not-asked", "late"] {
        assert!(child(&rig, key).await.labels.is_empty(), "{key}");
    }
    assert_eq!(rig.shared.count("plain"), 5);
    // `t1` went `loud` and keeps to it: started as written the second time.
    assert_eq!(rig.shared.count("loud"), 1);
    let said = left(&rig).await;
    assert!(said["no-way"].contains("which has no such way (its ways are loud, quiet)"), "{said:?}");
    assert!(said["nobody"].contains("no comparison of that name"), "{said:?}");
    assert!(said["not-asked"].contains("no comparison of that name"), "{said:?}");
    assert!(said["t1"].contains("went the way `loud` of `ab`, and this start names `quiet`"), "{said:?}");
    assert!(said["late"].contains("which has ended"), "{said:?}");
}

#[tokio::test]
async fn one_started_by_somebody_who_holds_less_than_an_instance_runs_with_leaves_its_starts_as_written() {
    let rig = Rig::new().await;
    asks(&rig).await;
    let less = Principal {
        kind: PrincipalKind::User,
        id: "bo".into(),
        name: "Bo".into(),
        scopes: vec!["blueprints".into()],
        key: None,
    };
    rig.compared.compare(asked("ab", vec![tone("loud", 1)]), &less).await.unwrap();
    ask(&rig, &[("t1", "ab", "loud")], 0).await;
    assert_eq!((rig.shared.count("plain"), rig.shared.count("loud")), (1, 0));
    assert!(left(&rig).await["t1"].contains("does not hold everything this instance runs with"));
    // Nor may they add a way to one somebody holding more started.
    rig.compared.compare(asked("cd", vec![tone("loud", 1)]), &user()).await.unwrap();
    let more = asked("cd", vec![tone("loud", 1), tone("quiet", 1)]);
    let refused = rig.compared.compare(more, &less).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied, "{refused}");
}

#[tokio::test]
async fn asked_for_again_it_is_answered_as_it_is_with_the_ways_that_are_new_added() {
    let rig = Rig::new().await;
    asks(&rig).await;
    let first = rig.compared.compare(asked("ab", vec![tone("loud", 1)]), &user()).await.unwrap();
    ask(&rig, &[("t1", "ab", "loud")], 0).await;
    // The same again changes nothing, and says nothing.
    let again = rig.compared.compare(asked("ab", vec![tone("loud", 1)]), &user()).await.unwrap();
    assert_eq!((again.started, again.ways.len(), again.ways[0].went), (first.started, 1, 1));
    assert!(rig.events("system.comparison.added").await.is_empty());
    // A way more is added, and what was counted is as it was.
    let more =
        rig.compared.compare(asked("ab", vec![tone("loud", 1), tone("quiet", 1)]), &user()).await.unwrap();
    let ways: Vec<(&str, u64)> = more.ways.iter().map(|way| (way.name.as_str(), way.went)).collect();
    assert_eq!(ways, [("loud", 1), ("quiet", 0)]);
    assert_eq!(rig.events("system.comparison.added").await.len(), 1);
    ask(&rig, &[("t2", "ab", "quiet")], 1).await;
    assert_eq!(under(&rig, "t2").await, pair("ab", "quiet"));

    // What cannot be the same comparison asked again is refused, saying why.
    let refused = |asking: Compare| async { rig.compared.compare(asking, &user()).await.unwrap_err() };
    let other = Way { with: json!({ "tone": "shrill" }).as_object().unwrap().clone(), ..tone("loud", 1) };
    let wrong = refused(asked("ab", vec![other])).await;
    assert!(wrong.message.contains("has a way `loud` already"), "{wrong}");
    let wrong = refused(Compare { of: "asks".into(), ..asked("ab", vec![tone("loud", 1)]) }).await;
    assert!(wrong.message.contains("is of `work`"), "{wrong}");
    let wrong = refused(Compare { asked: false, ..asked("ab", vec![tone("loud", 1)]) }).await;
    assert!(wrong.message.contains("there has already been a comparison called `ab`"), "{wrong}");
    // Ended, it is answered as it is where nothing new is asked, and has
    // nothing added; and nothing goes one way after it.
    let goes = rig.compared.end_comparison("ab", Some("loud".into())).await.unwrap_err();
    assert!(goes.message.contains("names its way"), "{goes}");
    rig.compared.end_comparison("ab", None).await.unwrap();
    let ended = rig.compared.compare(asked("ab", vec![tone("quiet", 1)]), &user()).await.unwrap();
    assert!(ended.ended.is_some());
    let wrong = refused(asked("ab", vec![tone("soft", 1)])).await;
    assert!(wrong.message.contains("has ended, and nothing is added to it"), "{wrong}");
}

/// Waits until `count` runs of `blueprint`'s own steps have finished.
async fn ran(rig: &Rig, blueprint: &str, count: usize) {
    let mut follow = rig.order.follow_every(&rig.journal, "run.finished");
    let mut seen = 0;
    crate::soon(&format!("{count} runs of {blueprint}"), async {
        while seen < count {
            let event = follow.next().await.unwrap();
            if event.data["blueprint"] == blueprint && event.data["flow"] == "main" {
                seen += 1;
            }
        }
    })
    .await;
}

/// What a blueprint that compares asks: the comparison, a start under
/// each of the ways it names, and the comparison ended.
const COMPARES: &str = "\
blueprint: compares
permissions: [blueprints:run]
params: { key: one }
steps:
  start:
    do: compare.start
    with:
      name: tones
      of: work
      ways: [{ name: loud, with: { tone: loud } }, quiet]
  loud:
    call: work
    needs: [start]
    as: \"loud-${ params.key }\"
    with: { id: \"loud-${ params.key }\", compared: tones, way: loud }
  quiet:
    call: work
    needs: [start]
    as: \"quiet-${ params.key }\"
    with: { id: \"quiet-${ params.key }\", compared: tones, way: quiet }
  end:
    do: compare.end
    needs: [loud, quiet]
    with: { name: tones }
";

#[tokio::test]
async fn a_blueprint_starts_and_ends_one_as_whoever_applied_it_and_the_same_done_twice_is_answered_as_it_is()
{
    let rig = Rig::new().await;
    asks(&rig).await;
    rig.library.put("compares", COMPARES);
    rig.apply(COMPARES, json!({ "key": "one" })).await;
    ran(&rig, "compares", 1).await;
    assert!(rig.events("run.failed").await.is_empty());
    assert_eq!(under(&rig, "loud-one").await, pair("tones", "loud"));
    assert_eq!(under(&rig, "quiet-one").await, pair("tones", "quiet"));
    assert_eq!((rig.shared.count("loud"), rig.shared.count("plain")), (1, 1));
    let one = rig.compared.comparison("tones").await.unwrap();
    assert!(one.asked && one.ended.is_some());
    assert_eq!((one.by.as_str(), one.project.as_deref()), ("user:ada", None));

    // Its steps done again, as after a stop: its start is answered as it
    // is, ended, and so is its end; what it starts under the ended one is
    // started as written, saying why.
    rig.apply(COMPARES, json!({ "key": "two" })).await;
    ran(&rig, "compares", 2).await;
    let failed = rig.events("run.failed").await;
    assert!(failed.is_empty(), "{:?}", failed.last().map(|event| &event.data));
    assert!(child(&rig, "loud-two").await.labels.is_empty());
    assert!(left(&rig).await["loud-two"].contains("which has ended"));
    assert_eq!(rig.compared.comparison("tones").await.unwrap().ended, one.ended);
}

#[tokio::test]
async fn a_blueprint_ends_only_an_asked_comparison_of_its_own_project() {
    let rig = Rig::new().await;
    asks(&rig).await;
    let shared = Compare {
        name: "shares".into(),
        of: "work".into(),
        ways: vec![tone("kind", 1)],
        ..Default::default()
    };
    rig.compared.compare(shared, &user()).await.unwrap();
    const ENDS: &str = "\
blueprint: ends
permissions: [blueprints:run]
steps:
  end: { do: compare.end, with: { name: shares } }
";
    rig.apply(ENDS, json!({})).await;
    ran(&rig, "ends", 1).await;
    let failed = rig.events("step.failed").await;
    let said = failed.last().map(|event| event.data.to_string()).unwrap_or_default();
    assert!(said.contains("is not a comparison whose ways are named by the start in this project"), "{said}");
    assert!(rig.compared.comparison("shares").await.unwrap().ended.is_none());
}

#[tokio::test]
async fn a_count_reads_the_field_an_event_says_by_its_path() {
    let rig = Rig::new().await;
    asks(&rig).await;
    let judged =
        comparisons::Counted { thing: "row.id".into(), ..count("judged", "test.judged", &["chose.verdict"]) };
    let ways = vec![tone("loud", 1), tone("quiet", 1)];
    rig.compared
        .compare(Compare { per: Some("id".into()), counts: vec![judged], ..asked("ab", ways) }, &user())
        .await
        .unwrap();
    ask(&rig, &[("e1", "ab", "loud"), ("e2", "ab", "quiet")], 0).await;
    rig.clock.advance(Duration::from_secs(5));
    say(&rig, "test.judged", json!({ "row": { "id": "e1" }, "chose": { "verdict": "won" } })).await;
    say(&rig, "test.judged", json!({ "row": { "id": "e2" }, "chose": { "verdict": "lost" } })).await;
    let of = |way: &str, came: &str| (way.to_string(), "judged".to_string(), came.to_string());
    let wanted = BTreeMap::from([(of("loud", "won"), (1, 5_000.0)), (of("quiet", "lost"), (1, 5_000.0))]);
    crate::soon("each to be counted", async {
        while came(&rig, "ab").await != wanted {
            tokio::task::yield_now().await;
        }
    })
    .await;
}
