//! A comparison's shares grown by a rule (`kernel::compared::rolled`): what
//! the counts come to, a case for each way it can come out; what shares a
//! part of a hundred comes to; and what is wrong with a rule as it is given.

use kernel::compared::rolled::{
    Rolled, Rolling, Rule, Seen, came_to, decide, percent, shares_at, tally, wrong,
};
use kernel::compared::{Comparison, Counted, Way, next};

/// Held to at least as often, the share now, what was read of the way that
/// grows and of the other (things, how many came to it), what it comes to.
type Decided = (bool, u32, (u32, u32), (u32, u32), &'static str);

/// The least, and then as [`Decided`].
type Fallen = (u32, bool, u32, (u32, u32), (u32, u32), &'static str);

/// The shares, the way that grows, its part, the shares it comes to.
type Parted = (&'static [(&'static str, u32)], &'static str, u32, &'static [u32]);

fn rule(at_least: bool) -> Rule {
    Rule {
        grows: "slate".into(),
        against: "usual".into(),
        count: "verdict".into(),
        came: "failed".into(),
        at_least,
        over: 20,
        step: 10,
        most: 50,
        least: None,
    }
}

fn read(things: u32, came: u32) -> kernel::compared::rolled::Tally {
    kernel::compared::rolled::Tally { things, came }
}

fn ways(shares: &[(&str, u32)]) -> Vec<Way> {
    shares
        .iter()
        .map(|(name, share)| Way { name: name.to_string(), share: *share, ..Default::default() })
        .collect()
}

#[test]
fn what_a_rule_comes_to_is_read_from_the_counts_of_both_ways_and_nothing_else() {
    // (held to at least as often, the share now, what was read of the way that grows and of the other, what it comes to)
    let cases: &[Decided] = &[
        // Not enough read of either: nothing is decided.
        (false, 10, (19, 0), (200, 40), "wait"),
        (false, 10, (20, 0), (19, 0), "wait"),
        (false, 10, (0, 0), (0, 0), "wait"),
        // Sent back no more often: it grows by its step.
        (false, 10, (20, 2), (180, 18), "raised 20"),
        (false, 10, (20, 0), (20, 5), "raised 20"),
        (false, 10, (20, 0), (20, 0), "raised 20"),
        // More often, by one thing in a hundred and eighty: it stays.
        (false, 10, (20, 3), (180, 26), "held"),
        (false, 10, (20, 20), (20, 19), "held"),
        // To its most, and no further: done.
        (false, 40, (20, 0), (20, 0), "raised 50 done"),
        (false, 45, (20, 0), (20, 0), "raised 50 done"),
        // At its most already, by somebody's hand: nothing more is decided.
        (false, 50, (20, 0), (20, 0), "wait"),
        (false, 70, (20, 20), (20, 0), "wait"),
        // Held to coming to it at least as often (passed): the other way about.
        (true, 10, (20, 18), (20, 18), "raised 20"),
        (true, 10, (20, 19), (40, 36), "raised 20"),
        (true, 10, (20, 17), (20, 18), "held"),
        (true, 10, (20, 0), (20, 0), "raised 20"),
    ];
    for (at_least, now, grows, against, wanted) in cases {
        let came = decide(&rule(*at_least), *now, read(grows.0, grows.1), read(against.0, against.1));
        let said = match &came {
            Rolled::Wait => "wait".to_string(),
            Rolled::Held { .. } => "held".to_string(),
            Rolled::Lowered { to, .. } => format!("lowered {to}"),
            Rolled::Raised { to, done: true, .. } => format!("raised {to} done"),
            Rolled::Raised { to, .. } => format!("raised {to}"),
        };
        assert_eq!(said, *wanted, "{at_least} at {now}% of {grows:?} against {against:?}: {came:?}");
    }
}

#[test]
fn a_rule_that_says_a_least_goes_back_a_step_where_it_does_not_hold_and_no_further() {
    let falls = |least: u32, at_least: bool| Rule { least: Some(least), ..rule(at_least) };
    // (the least, held to at least as often, the share now, what was read of each, what it comes to)
    let cases: &[Fallen] = &[
        // Sent back more often: a step back.
        (10, false, 40, (20, 5), (20, 1), "lowered 30"),
        (10, false, 20, (20, 5), (20, 1), "lowered 10"),
        // No further than the least, and from the least itself nowhere.
        (15, false, 20, (20, 5), (20, 1), "lowered 15"),
        (10, false, 10, (20, 5), (20, 1), "held"),
        (10, false, 5, (20, 5), (20, 1), "held"),
        // Where it holds it grows as any rule does, and with too little read nothing is decided.
        (10, false, 40, (20, 1), (20, 5), "raised 50 done"),
        (10, false, 40, (19, 9), (20, 0), "wait"),
        // Held to at least as often: passed less often, a step back.
        (10, true, 30, (20, 10), (20, 15), "lowered 20"),
    ];
    for (least, at_least, now, grows, against, wanted) in cases {
        let came =
            decide(&falls(*least, *at_least), *now, read(grows.0, grows.1), read(against.0, against.1));
        let said = match &came {
            Rolled::Wait => "wait".to_string(),
            Rolled::Held { .. } => "held".to_string(),
            Rolled::Lowered { to, .. } => format!("lowered {to}"),
            Rolled::Raised { to, done: true, .. } => format!("raised {to} done"),
            Rolled::Raised { to, .. } => format!("raised {to}"),
        };
        assert_eq!(
            said, *wanted,
            "least {least} {at_least} at {now}% of {grows:?} against {against:?}: {came:?}"
        );
    }
    let Rolled::Lowered { why, .. } = decide(&falls(15, false), 20, read(20, 5), read(20, 1)) else {
        panic!()
    };
    assert!(
        why.ends_with(
            "for 1 of 20: more often, so its share goes back from 20% to 15%, the least its rule gives it."
        ),
        "{why}"
    );
    let said = kernel::compared::rolled::said(&falls(10, false));
    assert!(said.ends_with("and goes back as much each time it did not, to 10 at least"), "{said}");
}

#[test]
fn what_is_decided_says_what_was_read_and_what_it_came_to() {
    let Rolled::Raised { why, .. } = decide(&rule(false), 10, read(20, 2), read(180, 18)) else { panic!() };
    assert_eq!(
        why,
        "`slate` came to `failed` of its `verdict` for 2 of the 20 things read since its share was last set, and \
         `usual` for 18 of 180: no more often, so its share goes from 10% to 20%."
    );
    let Rolled::Raised { why, .. } = decide(&rule(false), 40, read(20, 0), read(20, 0)) else { panic!() };
    assert!(why.ends_with("goes from 40% to 50%, as far as its rule takes it."), "{why}");
    let Rolled::Held { why } = decide(&rule(false), 10, read(20, 5), read(20, 1)) else { panic!() };
    assert!(
        why.ends_with(
            "for 1 of 20: more often, so its share stays at 10%, and what comes next is read again."
        )
    );
    let Rolled::Held { why } = decide(&rule(true), 10, read(20, 1), read(20, 5)) else { panic!() };
    assert!(why.contains(": less often, so its share stays at 10%"), "{why}");
    for word in ["better", "worse", "score", "best"] {
        assert!(!why.contains(word), "{why}");
    }
}

#[test]
fn a_part_of_a_hundred_gives_the_rest_to_the_others_as_their_shares_were_and_one_at_least_each() {
    // (the shares, the way that grows, its part, the shares it comes to)
    let cases: &[Parted] = &[
        (&[("usual", 9), ("slate", 1)], "slate", 20, &[80, 20]),
        (&[("usual", 80), ("slate", 20)], "slate", 50, &[50, 50]),
        (&[("slate", 1), ("usual", 1)], "slate", 75, &[75, 25]),
        // The rest shared as the others' shares were, what is left over to who lost most by rounding.
        (&[("a", 1), ("b", 1), ("c", 2)], "a", 50, &[50, 17, 33]),
        (&[("a", 3), ("b", 1), ("c", 1)], "c", 40, &[45, 15, 40]),
        // None given nothing, however small its share was.
        (&[("big", 100), ("tiny", 1), ("slate", 1)], "slate", 98, &[1, 1, 98]),
        // Nothing to do: no such way, no other, nothing or all of it.
        (&[("usual", 9), ("slate", 1)], "otter", 50, &[9, 1]),
        (&[("slate", 3)], "slate", 50, &[3]),
        (&[("usual", 9), ("slate", 1)], "slate", 100, &[9, 1]),
        (&[("usual", 9), ("slate", 1)], "slate", 0, &[9, 1]),
    ];
    for (shares, grows, to, wanted) in cases {
        let ways = ways(shares);
        let given = shares_at(&ways, grows, *to);
        assert_eq!(given, *wanted, "{shares:?}: {grows} to {to}");
        if given != shares.iter().map(|(_, share)| *share).collect::<Vec<_>>() {
            assert_eq!(given.iter().sum::<u32>(), 100, "{shares:?}");
        }
    }
    // A part is read to the nearest.
    assert_eq!(percent(&ways(&[("usual", 9), ("slate", 1)]), "slate"), 10);
    assert_eq!(percent(&ways(&[("a", 1), ("b", 2)]), "a"), 33);
    assert_eq!(percent(&ways(&[("a", 1), ("b", 2)]), "b"), 67);
    assert_eq!(percent(&ways(&[("a", 1), ("b", 2)]), "c"), 0);
}

#[test]
fn a_way_is_given_its_part_of_what_comes_after_its_share_is_set_and_not_what_it_missed_before() {
    // Ninety went `usual` and ten `slate`; the shares now say half each.
    let mut ways = ways(&[("usual", 50), ("slate", 50)]);
    ways[0].went = 90;
    ways[1].went = 10;
    (ways[0].before, ways[1].before) = (90, 10);
    let mut went = Vec::new();
    for _ in 0..6 {
        let at = next(&ways);
        ways[at].went += 1;
        went.push(ways[at].name.clone());
    }
    assert_eq!(went, ["usual", "slate", "usual", "slate", "usual", "slate"]);
    // Held to what went in all, it would have been `slate` eighty times running.
    (ways[0].before, ways[1].before) = (0, 0);
    assert_eq!(ways[next(&ways)].name, "slate");
}

#[test]
fn what_is_read_is_each_thing_once_and_whether_it_ever_came_to_what_is_held() {
    let seen = |way: &str, came: bool| Seen { way: way.into(), came };
    let rolling = Rolling {
        since: 0,
        seen: [
            ("1".to_string(), seen("slate", true)),
            ("2".to_string(), seen("slate", false)),
            ("3".to_string(), seen("usual", false)),
            ("4".to_string(), seen("other", true)),
        ]
        .into(),
        done: false,
    };
    assert_eq!(tally(&rolling, "slate"), read(2, 1));
    assert_eq!(tally(&rolling, "usual"), read(1, 0));
    assert_eq!(tally(&rolling, "none"), read(0, 0));
    // What a count came to is held by its whole or by the last of several fields.
    let cases = [
        ("failed", "failed", true),
        ("failed", "passed", false),
        ("doing", "review>doing", true),
        ("review>doing", "review>doing", true),
        ("review", "review>doing", false),
        ("", "", false),
        ("fail", "failed", false),
    ];
    for (held, came, wanted) in cases {
        assert_eq!(came_to(held, came), wanted, "{held} of {came}");
    }
}

#[test]
fn a_rule_is_refused_where_it_cannot_be_read_with_what_is_wrong() {
    let one = |shares: &[(&str, u32)]| Comparison {
        name: "rollout".into(),
        project: Some("shop".into()),
        of: "std/work".into(),
        per: Some("item".into()),
        also: Vec::new(),
        also_per: None,
        ways: ways(shares),
        counts: vec![Counted {
            name: "verdict".into(),
            topic: "review.failed".into(),
            thing: "card".into(),
            ..Default::default()
        }],
        by: "user:owner".into(),
        held: vec!["*".into()],
        started: 0,
        ended: None,
        goes: None,
        asked: false,
        until: None,
        rule: None,
        rolling: None,
        shared: Vec::new(),
    };
    let usual = one(&[("usual", 9), ("slate", 1)]);
    assert_eq!(wrong(&rule(false), &usual), None);
    let with = |change: &dyn Fn(&mut Rule)| {
        let mut said = rule(false);
        change(&mut said);
        wrong(&said, &usual).unwrap_or_default()
    };
    assert_eq!(wrong(&Rule { least: Some(10), ..rule(false) }, &usual), None);
    let cases: Vec<(String, &str)> = vec![
        (
            with(&|rule| rule.grows = "otter".into()),
            "the way that grows is `otter`, and `rollout` has no such way",
        ),
        (with(&|rule| rule.against = "otter".into()), "the way that is held against is `otter`"),
        (with(&|rule| rule.against = "slate".into()), "`slate` cannot be held against itself"),
        (
            with(&|rule| rule.count = "moved".into()),
            "goes by the count `moved`, and `rollout` counts verdict",
        ),
        (with(&|rule| rule.came = " ".into()), "say what `verdict` came to"),
        (with(&|rule| rule.over = 0), "at least one thing of each way"),
        (with(&|rule| rule.step = 0), "the step is 0"),
        (with(&|rule| rule.most = 100), "all of it is ending the comparison"),
        (with(&|rule| rule.most = 10), "given 10% of the things already"),
        (with(&|rule| rule.least = Some(0)), "at least 1"),
        (
            with(&|rule| rule.least = Some(20)),
            "the least `slate` goes back to is 20%, and it is given 10% now",
        ),
    ];
    for (said, wanted) in cases {
        assert!(said.contains(wanted), "{said:?} does not say {wanted:?}");
    }
    // Three ways and a most that would leave the other two less than one each.
    let three = one(&[("a", 1), ("b", 1), ("slate", 1)]);
    let said = wrong(&Rule { most: 99, against: "a".into(), ..rule(false) }, &three).unwrap();
    assert!(said.contains("would leave the other 2 ways less than one each"), "{said}");
    // One whose ways are named by the start, and one that has ended.
    let asked = Comparison { asked: true, ..usual.clone() };
    assert!(wrong(&rule(false), &asked).unwrap().contains("its shares say nothing"));
    let ended = Comparison { ended: Some(1), ..usual };
    assert!(wrong(&rule(false), &ended).unwrap().contains("has ended"));
}
