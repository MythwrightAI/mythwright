//! A project's feature flags as plain values (`kernel::flags`): what a
//! flag is called, what it may say, whether a change of it waits for a
//! go-ahead, which are past their day and a percentage flag started as a
//! comparison, each a table of cases.

use kernel::flags::{
    Came, Compared, Kind, asks, change, compared, day_of, due, is_flags, key, off, refused_name, said, shown,
    variable, widens,
};
use serde_json::{Value, json};

#[test]
fn a_flag_is_known_by_its_name_written_plainly_and_read_by_a_variable_of_that_name() {
    let cases: &[(&str, &str, &str)] = &[
        ("new-checkout", "new-checkout", "FLAG_NEW_CHECKOUT"),
        ("New checkout", "new-checkout", "FLAG_NEW_CHECKOUT"),
        ("  Dark mode!! v2 ", "dark-mode-v2", "FLAG_DARK_MODE_V2"),
        ("search_v2", "search-v2", "FLAG_SEARCH_V2"),
    ];
    for (name, known, read) in cases {
        assert_eq!((key(name).as_str(), variable(name).as_str()), (*known, *read), "{name}");
        assert_eq!(refused_name(name), None, "{name}");
    }
    assert!(refused_name("!!!").unwrap().contains("letters and digits"));
    assert!(refused_name(&"a".repeat(61)).unwrap().contains("too long"));
    // The variables that are a flag's, which the environment's own route leaves to the flag.
    for (var, is) in [("FLAG_X", true), ("flag_x", true), ("FLAGS", false), ("API_URL", false)] {
        assert_eq!(is_flags(var), is, "{var}");
    }
}

#[test]
fn what_a_flag_says_is_read_as_its_kind_and_anything_else_is_refused_saying_what_it_takes() {
    let cases: &[(Kind, Value, Result<Value, &str>)] = &[
        (Kind::Switch, json!("on"), Ok(json!("on"))),
        (Kind::Switch, json!(" OFF "), Ok(json!("off"))),
        (Kind::Switch, json!(true), Ok(json!("on"))),
        (Kind::Switch, json!(false), Ok(json!("off"))),
        (Kind::Switch, json!("maybe"), Err("is `on` or `off`")),
        (Kind::Percentage, json!(25), Ok(json!(25))),
        (Kind::Percentage, json!("25%"), Ok(json!(25))),
        (Kind::Percentage, json!("100"), Ok(json!(100))),
        (Kind::Percentage, json!(101), Err("from 0 to 100")),
        (Kind::Percentage, json!(2.5), Err("from 0 to 100")),
        (Kind::Word, json!("blue"), Ok(json!("blue"))),
        (Kind::Word, json!(""), Ok(json!(""))),
        (Kind::Word, json!("two words"), Err("lowercase letters")),
        (Kind::Word, json!("$(rm)"), Err("lowercase letters")),
    ];
    for (kind, asked, came) in cases {
        match (said(*kind, asked), came) {
            (Ok(now), Ok(meant)) => assert_eq!(&now, meant, "{kind:?} {asked}"),
            (Err(why), Err(part)) => assert!(why.contains(part), "{kind:?} {asked}: {why}"),
            (other, meant) => panic!("{kind:?} {asked} came to {other:?}, meant {meant:?}"),
        }
    }
    let shown_as: &[(Kind, Value, &str)] = &[
        (Kind::Switch, json!("on"), "on"),
        (Kind::Percentage, json!(25), "25%"),
        (Kind::Word, json!("blue"), "blue"),
        (Kind::Word, json!(""), "nothing"),
    ];
    for (kind, value, words) in shown_as {
        assert_eq!(shown(*kind, value), *words);
    }
    assert_eq!(
        (Kind::read("percentage"), Kind::read("word"), Kind::read("")),
        (Kind::Percentage, Kind::Word, Kind::Switch)
    );
    assert_eq!(
        (off(Kind::Switch), off(Kind::Percentage), off(Kind::Word)),
        (json!("off"), json!(0), json!(""))
    );
}

#[test]
fn turning_a_flag_on_or_wider_waits_for_whom_the_rule_names_and_turning_it_off_never_does() {
    let rule = vec!["user:grace".to_string()];
    let none: Vec<String> = Vec::new();
    // Of what was, what is asked, by whom and under which rule, what it comes to.
    type Case<'a> = (Kind, Option<Value>, Value, &'a str, &'a [String], Came);
    let cases: &[Case<'_>] = &[
        // Nothing said is off, so on is a turning on: it waits, under a rule that names somebody else.
        (Kind::Switch, None, json!("on"), "user:ada", &rule, Came::Asks(json!("on"))),
        // One the rule names sets it at once.
        (Kind::Switch, None, json!("on"), "user:grace", &rule, Came::Set(json!("on"))),
        // With nobody named, nothing waits.
        (Kind::Switch, None, json!("on"), "user:ada", &none, Came::Set(json!("on"))),
        // Off never waits.
        (Kind::Switch, Some(json!("on")), json!("off"), "user:ada", &rule, Came::Set(json!("off"))),
        // What it says already is nothing to do.
        (Kind::Switch, Some(json!("on")), json!(true), "user:ada", &rule, Came::Same),
        (Kind::Switch, None, json!("off"), "user:ada", &rule, Came::Same),
        // A key that holds less is itself, never its maker.
        (Kind::Switch, None, json!("on"), "key:k1", &rule, Came::Asks(json!("on"))),
        // A percentage made larger waits; made smaller does not.
        (Kind::Percentage, Some(json!(10)), json!(25), "user:ada", &rule, Came::Asks(json!(25))),
        (Kind::Percentage, Some(json!(25)), json!("10%"), "user:ada", &rule, Came::Set(json!(10))),
        (Kind::Percentage, None, json!(0), "user:ada", &rule, Came::Same),
        // Another word waits; none does not.
        (Kind::Word, Some(json!("blue")), json!("green"), "user:ada", &rule, Came::Asks(json!("green"))),
        (Kind::Word, Some(json!("blue")), json!(""), "user:ada", &rule, Came::Set(json!(""))),
    ];
    for (kind, was, asked, by, rule, came) in cases {
        assert_eq!(
            &change(*kind, was.as_ref(), asked, rule, by).unwrap(),
            came,
            "{kind:?} {was:?} {asked} {by}"
        );
    }
    assert!(change(Kind::Switch, None, &json!("maybe"), &rule, "user:ada").is_err());
    assert!(widens(Kind::Switch, Some(&json!("off")), &json!("on")));
    assert!(!widens(Kind::Switch, Some(&json!("on")), &json!("on")));
    assert!(!asks(&rule, "user:ada", false));
}

#[test]
fn a_flag_is_due_once_its_day_is_past_and_its_owner_is_told_how_to_take_it_out() {
    let rows = vec![
        json!({ "id": "shop-flags-1", "name": "new-checkout", "owner": "user:ada", "until": "2026-11-20" }),
        json!({ "id": "shop-flags-2", "name": "dark", "owner": "user:grace", "until": "2026-12-01" }),
        json!({ "id": "shop-flags-3", "name": "today", "owner": "", "until": "2026-11-24" }),
        json!({ "id": "shop-flags-4", "name": "never", "owner": "user:ada" }),
        json!({ "id": "shop-flags-5", "name": "wrong", "until": "soon" }),
        json!({ "id": "shop-flags-6", "name": "stamped", "until": "2026-11-01T09:00:00Z" }),
    ];
    let came = due(&rows, "2026-11-24");
    let names: Vec<&str> = came.iter().map(|one| one.name.as_str()).collect();
    assert_eq!(names, ["new-checkout", "stamped"]);
    assert_eq!(
        (came[0].record.as_str(), came[0].owner.as_str(), came[0].until.as_str()),
        ("shop-flags-1", "user:ada", "2026-11-20")
    );
    assert!(came[0].says.contains("FLAG_NEW_CHECKOUT"), "{}", came[0].says);
    assert_eq!(came[1].until, "2026-11-01");
    let days: &[(i64, &str)] = &[
        (0, "1970-01-01"),
        (951_782_400_000, "2000-02-29"),
        (1_795_478_400_000, "2026-11-24"),
        (1_795_564_799_999, "2026-11-24"),
        (-86_400_000, "1969-12-31"),
    ];
    for (ms, day) in days {
        assert_eq!(day_of(*ms), *day, "{ms}");
    }
}

#[test]
fn a_percentage_flag_is_started_as_two_ways_given_its_part_and_the_rest_and_nothing_else_is() {
    let asked = |percentage: Value, param: &'static str| Compared {
        flag: "New checkout",
        environment: "prod",
        percentage: Box::leak(Box::new(percentage)),
        project: "shop",
        blueprint: "std/work",
        per: "item",
        param,
        counts: Vec::new(),
    };
    let compare = compared(asked(json!(25), "flag")).unwrap();
    assert_eq!(compare.name, "flag-new-checkout-prod");
    assert_eq!(
        (compare.of.as_str(), compare.per.as_deref(), compare.project.as_deref()),
        ("std/work", Some("item"), Some("shop"))
    );
    let ways: Vec<(String, u32, Value)> =
        compare.ways.iter().map(|way| (way.name.clone(), way.share, json!(way.with))).collect();
    assert_eq!(
        ways,
        [("on".to_string(), 25, json!({ "flag": "on" })), ("off".to_string(), 75, json!({ "flag": "off" }))]
    );
    for (percentage, param, why) in [
        (json!("on"), "flag", "no percentage"),
        (json!(0), "flag", "nothing to put beside it"),
        (json!(100), "flag", "nothing to put beside it"),
        (json!(30), "", "`param`"),
    ] {
        let refused = compared(asked(percentage, param)).map(|_| ()).unwrap_err();
        assert!(refused.contains(why), "{refused}");
    }
}
