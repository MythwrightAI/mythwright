//! A comparison whose shares grow by a rule: a way given more of the
//! things each time what is counted of its things holds against another
//! way's, held where it does not, and said each time with why; and the
//! shares said again by somebody, which the rule goes on from.

use std::collections::BTreeMap;

use comparisons::{Compare, Rule, Way};
use kernel::ErrorKind;
use serde_json::json;

use crate::Rig;
use crate::compared::{ask, child, count, say, shop, tones, way, went};

/// `slate` grows against `usual` by how often its things' verdict came to
/// `failed`, over `over` things of each.
fn rule(over: u32, step: u32, most: u32) -> Rule {
    Rule {
        grows: "slate".into(),
        against: "usual".into(),
        count: "verdict".into(),
        came: "failed".into(),
        at_least: false,
        over,
        step,
        most,
        least: None,
    }
}

/// A comparison of the work with `usual` and `slate` at these shares, its
/// things' verdicts counted, and `rule`.
fn rolled(usual: u32, slate: u32, rule: Rule) -> Compare {
    let ways = vec![Way { share: usual, ..way("usual") }, Way { share: slate, ..way("slate") }];
    Compare { counts: vec![count("verdict", "shop.verdict", &["how"])], rule: Some(rule), ..tones(ways) }
}

/// Which way each thing went, by its id.
async fn ways_of(rig: &Rig, ids: &[u64]) -> Vec<String> {
    let mut all = Vec::new();
    for id in ids {
        all.push(child(rig, &format!("work-{id}")).await.labels["way"].clone());
    }
    all
}

/// The shares as they are, and each time they were said again, by whom.
async fn shares(rig: &Rig) -> (BTreeMap<String, u32>, Vec<String>) {
    let one = rig.compared.comparison("tones").await.unwrap();
    let now = one.ways.iter().map(|way| (way.name.clone(), way.share)).collect();
    (now, one.shared.iter().map(|shared| shared.by.clone()).collect())
}

/// Waits until the shares have been said again `times` times.
async fn said_again(rig: &Rig, times: usize) {
    crate::soon("the shares to be said again", async {
        while rig.compared.comparison("tones").await.unwrap().shared.len() < times {
            tokio::task::yield_now().await;
        }
    })
    .await;
}

fn of(pairs: &[(&str, u32)]) -> BTreeMap<String, u32> {
    pairs.iter().map(|(way, share)| (way.to_string(), *share)).collect()
}

#[tokio::test]
async fn a_way_is_given_more_of_the_things_while_what_is_counted_of_them_holds_and_says_why() {
    let rig = Rig::new().await;
    shop(&rig).await;
    rig.compared.compare(rolled(1, 1, rule(2, 25, 75)), &crate::user()).await.unwrap();
    ask(&rig, &[1, 2, 3, 4], 0).await;
    assert_eq!(ways_of(&rig, &[1, 2, 3, 4]).await, ["usual", "slate", "usual", "slate"]);

    // Three of the four pass; `usual`'s first fails, and passes once looked at again: read once.
    say(&rig, "shop.verdict", json!({ "id": 1, "how": "failed" })).await;
    say(&rig, "shop.verdict", json!({ "id": 1, "how": "passed" })).await;
    say(&rig, "shop.verdict", json!({ "id": 2, "how": "passed" })).await;
    say(&rig, "shop.verdict", json!({ "id": 3, "how": "passed" })).await;
    // A thing nobody gave a way, and a verdict of what the rule does not hold: read as nothing.
    say(&rig, "shop.verdict", json!({ "id": 99, "how": "failed" })).await;
    assert_eq!(shares(&rig).await.1.len(), 0, "one of `slate`'s two is still to come");
    say(&rig, "shop.verdict", json!({ "id": 4, "how": "passed" })).await;
    said_again(&rig, 1).await;

    // `slate` came to `failed` for none of its two and `usual` for one: it grows, to its most.
    assert_eq!(shares(&rig).await, (of(&[("usual", 25), ("slate", 75)]), vec!["rule".to_string()]));
    let said = rig.events("system.comparison.shared").await;
    assert_eq!(said.len(), 1, "said once");
    assert_eq!(said[0].data["by"], "rule");
    assert_eq!(said[0].data["was"], json!({ "usual": 1, "slate": 1 }));
    assert_eq!(said[0].data["now"], json!({ "usual": 25, "slate": 75 }));
    assert_eq!(
        said[0].data["why"],
        "`slate` came to `failed` of its `verdict` for 0 of the 2 things read since its share was last set, and \
         `usual` for 1 of 2: no more often, so its share goes from 50% to 75%, as far as its rule takes it."
    );
    let one = rig.compared.comparison("tones").await.unwrap();
    assert!(one.rolling.as_ref().is_some_and(|rolling| rolling.done && rolling.seen.is_empty()), "{one:?}");

    // What comes next is shared out by the new shares, from where each way was: three of four to `slate`.
    ask(&rig, &[5, 6, 7, 8], 4).await;
    assert_eq!(ways_of(&rig, &[5, 6, 7, 8]).await, ["usual", "slate", "slate", "slate"]);
    assert_eq!(went(&rig, "tones").await, [("usual".to_string(), 3), ("slate".to_string(), 5)]);
    // Done, it decides nothing more however its things come out.
    for id in [6, 7, 8] {
        say(&rig, "shop.verdict", json!({ "id": id, "how": "failed" })).await;
    }
    say(&rig, "shop.verdict", json!({ "id": 5, "how": "passed" })).await;
    crate::soon("the last verdict to be counted", async {
        while crate::compared::came(&rig, "tones").await.values().map(|one| one.0).sum::<u64>() < 9 {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert_eq!(shares(&rig).await.1.len(), 1);

    // Laid out as it is shown: each way's part, the rule and what it read, each time with why.
    let shown = comparisons::beside::beside(&rig.compared.comparison("tones").await.unwrap(), &[]);
    let rolled = &shown["rolled"];
    assert_eq!(
        rolled["parts"],
        json!([
            { "way": "usual", "share": 25, "percent": 25 },
            { "way": "slate", "share": 75, "percent": 75 },
        ])
    );
    assert_eq!(rolled["rule"]["done"], true);
    let rule = rolled["rule"]["said"].as_str().unwrap();
    assert!(
        rule.starts_with("`slate` is given 25 more of every hundred things at a time, up to 75"),
        "{rule}"
    );
    assert_eq!(rolled["shared"].as_array().map(Vec::len), Some(1));
    for word in ["better", "worse", "score", "best", "winner"] {
        assert!(!shown.to_string().contains(word), "{shown}");
    }
}

#[tokio::test]
async fn where_what_is_counted_does_not_hold_the_share_stays_and_what_comes_next_is_read_afresh() {
    let rig = Rig::new().await;
    shop(&rig).await;
    rig.compared.compare(rolled(1, 1, rule(1, 10, 70)), &crate::user()).await.unwrap();
    ask(&rig, &[1, 2], 0).await;
    say(&rig, "shop.verdict", json!({ "id": 1, "how": "passed" })).await;
    say(&rig, "shop.verdict", json!({ "id": 2, "how": "failed" })).await;
    said_again(&rig, 1).await;
    assert_eq!(shares(&rig).await, (of(&[("usual", 1), ("slate", 1)]), vec!["rule".to_string()]));
    let said = rig.events("system.comparison.shared").await;
    assert_eq!(said[0].data["was"], said[0].data["now"], "held");
    let why = said[0].data["why"].as_str().unwrap();
    assert!(
        why.ends_with(
            "for 0 of 1: more often, so its share stays at 50%, and what comes next is read again."
        )
    );

    // What is read now begins afresh: two more, both passed, and it grows.
    ask(&rig, &[3, 4], 2).await;
    assert_eq!(ways_of(&rig, &[3, 4]).await, ["usual", "slate"]);
    say(&rig, "shop.verdict", json!({ "id": 3, "how": "passed" })).await;
    say(&rig, "shop.verdict", json!({ "id": 4, "how": "passed" })).await;
    said_again(&rig, 2).await;
    assert_eq!(shares(&rig).await.0, of(&[("usual", 40), ("slate", 60)]));
}

#[tokio::test]
async fn a_rule_that_says_a_least_gives_the_way_less_where_what_is_counted_does_not_hold() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let falls = Rule { least: Some(25), ..rule(1, 25, 75) };
    rig.compared.compare(rolled(1, 1, falls), &crate::user()).await.unwrap();
    ask(&rig, &[1, 2], 0).await;
    say(&rig, "shop.verdict", json!({ "id": 1, "how": "passed" })).await;
    say(&rig, "shop.verdict", json!({ "id": 2, "how": "failed" })).await;
    said_again(&rig, 1).await;
    assert_eq!(shares(&rig).await, (of(&[("usual", 75), ("slate", 25)]), vec!["rule".to_string()]));
    let said = rig.events("system.comparison.shared").await;
    let why = said[0].data["why"].as_str().unwrap();
    assert!(
        why.ends_with("more often, so its share goes back from 50% to 25%, the least its rule gives it."),
        "{why}"
    );
    // At its least it stays there, and is read again; it is not done.
    ask(&rig, &[3, 4, 5, 6], 2).await;
    let ways = ways_of(&rig, &[3, 4, 5, 6]).await;
    assert_eq!(ways.iter().filter(|way| *way == "slate").count(), 1, "{ways:?}");
    let one = rig.compared.comparison("tones").await.unwrap();
    assert!(one.rolling.is_some_and(|rolling| !rolling.done));
}

#[tokio::test]
async fn the_shares_are_said_again_by_somebody_with_why_and_the_rule_goes_on_from_there() {
    let rig = Rig::new().await;
    shop(&rig).await;
    rig.compared.compare(rolled(3, 1, rule(1, 25, 75)), &crate::user()).await.unwrap();
    ask(&rig, &[1], 0).await;
    say(&rig, "shop.verdict", json!({ "id": 1, "how": "passed" })).await;
    let why = Some("slate is ready for more".to_string());
    let one = rig.compared.share("tones", of(&[("slate", 3)]), why.clone(), &crate::user()).await.unwrap();
    assert_eq!(
        one.shared.last().map(|shared| (shared.by.as_str(), shared.why.as_str())),
        Some(("user:ada", "slate is ready for more"))
    );
    assert_eq!(shares(&rig).await.0, of(&[("usual", 3), ("slate", 3)]));
    // What the rule had read is put by: it reads afresh from the shares said.
    assert!(one.rolling.as_ref().is_some_and(|rolling| rolling.seen.is_empty() && !rolling.done), "{one:?}");
    // The same said again changes nothing and says nothing.
    rig.compared.share("tones", of(&[("slate", 3)]), why, &crate::user()).await.unwrap();
    assert_eq!(rig.events("system.comparison.shared").await.len(), 1);

    // Refused, with what is wrong.
    let ada = crate::user();
    let refused = |shares: &[(&str, u32)]| rig.compared.share("tones", of(shares), None, &ada);
    let wrong = refused(&[("otter", 1)]).await.unwrap_err();
    assert!(
        wrong.message.contains("`tones` has no way called `otter`; its ways are usual, slate"),
        "{wrong}"
    );
    let none = refused(&[("slate", 0)]).await.unwrap_err();
    assert!(none.message.contains("would have a share of 0"), "{none}");
    assert_eq!(refused(&[]).await.unwrap_err().kind, ErrorKind::Invalid);
    let narrow = kernel::Principal { scopes: vec!["blueprints:run".into()], ..crate::user() };
    let held = rig.compared.share("tones", of(&[("slate", 1)]), None, &narrow).await.unwrap_err();
    assert_eq!(held.kind, ErrorKind::Denied, "{held}");

    // A rule given later is checked as it is given, and taken away with none.
    let past = rig.compared.rule("tones", Some(rule(1, 10, 50)), &crate::user()).await.unwrap_err();
    assert!(past.message.contains("`slate` is given 50% of the things already"), "{past}");
    let one = rig.compared.rule("tones", None, &crate::user()).await.unwrap();
    assert!(one.rule.is_none() && one.rolling.is_none());
    let one = rig.compared.rule("tones", Some(rule(1, 10, 80)), &crate::user()).await.unwrap();
    assert_eq!(one.rule, Some(rule(1, 10, 80)));
    assert_eq!(rig.events("system.comparison.ruled").await.len(), 2);
    // Given while whoever counts is counting, it is what is read from then on.
    ask(&rig, &[2, 3], 1).await;
    assert_eq!(ways_of(&rig, &[2, 3]).await, ["usual", "slate"]);
    say(&rig, "shop.verdict", json!({ "id": 2, "how": "passed" })).await;
    say(&rig, "shop.verdict", json!({ "id": 3, "how": "passed" })).await;
    said_again(&rig, 2).await;
    assert_eq!(
        shares(&rig).await,
        (of(&[("usual", 40), ("slate", 60)]), vec!["user:ada".to_string(), "rule".to_string()])
    );

    // Ended, its shares are changed no more.
    rig.compared.end_comparison("tones", None).await.unwrap();
    let ended = refused(&[("slate", 1)]).await.unwrap_err();
    assert!(ended.message.contains("has ended"), "{ended}");
}

#[tokio::test]
async fn a_comparison_whose_ways_are_named_by_the_start_has_no_shares_to_change_or_rule_them() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let asked = Compare { asked: true, ..tones(vec![way("usual"), way("slate")]) };
    rig.compared.compare(asked, &crate::user()).await.unwrap();
    let shares = rig.compared.share("tones", of(&[("slate", 2)]), None, &crate::user()).await.unwrap_err();
    assert!(shares.message.contains("its shares say nothing"), "{shares}");
    let ruled = rolled(1, 1, rule(1, 10, 70));
    let ada = crate::user();
    let refused = rig.compared.compare(Compare { name: "other".into(), asked: true, ..ruled }, &ada);
    let refused = refused.await.unwrap_err();
    assert!(refused.message.contains("its shares say nothing and no rule changes them"), "{refused}");
    // And one given a rule it cannot read is refused as it is started.
    let wrong =
        Compare { name: "third".into(), ..rolled(1, 1, Rule { count: "moved".into(), ..rule(1, 10, 70) }) };
    let wrong = rig.compared.compare(wrong, &crate::user()).await.unwrap_err();
    assert!(
        wrong.message.contains("the rule goes by the count `moved`, and `third` counts verdict"),
        "{wrong}"
    );
}

#[tokio::test]
async fn a_rule_given_to_a_comparison_that_is_counting_is_read_from_then_on() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let unruled = Compare { rule: None, ..rolled(1, 1, rule(1, 25, 75)) };
    rig.compared.compare(unruled, &crate::user()).await.unwrap();
    ask(&rig, &[1], 0).await;
    say(&rig, "shop.verdict", json!({ "id": 1, "how": "passed" })).await;
    crate::soon("the first verdict to be counted", async {
        while crate::compared::came(&rig, "tones").await.is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    rig.compared.rule("tones", Some(rule(1, 25, 75)), &crate::user()).await.unwrap();
    ask(&rig, &[2, 3], 1).await;
    assert_eq!(ways_of(&rig, &[2, 3]).await, ["slate", "usual"]);
    say(&rig, "shop.verdict", json!({ "id": 2, "how": "passed" })).await;
    say(&rig, "shop.verdict", json!({ "id": 3, "how": "passed" })).await;
    said_again(&rig, 1).await;
    assert_eq!(shares(&rig).await.0, of(&[("usual", 25), ("slate", 75)]));
}
