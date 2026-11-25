//! A project's environment, as a change of it is worked out and as it
//! is shown, with no conductor: what is kept and said of it is the
//! runner's, and is played in the API suite (`environs.rs`).

use environments::project::{Changed, Environment, check_name};
use kernel::{ErrorKind, Fields, Principal};
use serde_json::{Value, json};

fn vars(value: Value) -> Fields {
    kernel::fields(value)
}

fn names(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
}

fn made() -> Environment {
    Environment::new("staging", &Principal::system(), 7).unwrap()
}

#[test]
fn a_name_is_lowercase_letters_digits_and_dashes_and_short() {
    for good in ["dev", "staging", "prod-eu-2", "2026"] {
        assert!(check_name(good).is_ok(), "{good}");
    }
    let long = "a".repeat(41);
    for wrong in ["", "Prod", "prod eu", "-dev", "dev_1", "prød", long.as_str()] {
        let refused = check_name(wrong).unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Invalid, "{wrong}");
        assert!(
            refused.message.contains("lowercase letters, digits and `-`"),
            "{wrong}: {}",
            refused.message
        );
    }
}

#[test]
fn setting_says_what_was_set_changed_and_marked_and_leaves_the_rest() {
    let mut environment = made();
    let first = environment.set(&vars(json!({ "api": "one", "replicas": 3 })), None).unwrap();
    assert_eq!(first.set, ["api", "replicas"]);
    assert!(environment.vars.values().all(|var| !var.agents), "nothing is handed to agents unless said");
    let second = environment.set(&vars(json!({ "api": "two" })), Some(true)).unwrap();
    assert_eq!(second, Changed { changed: names(&["api"]), marked: names(&["api"]), ..Changed::default() });
    assert_eq!(environment.vars["replicas"].value, json!(3), "what was not named is as it was");
    let again = environment.set(&vars(json!({ "api": "two" })), None).unwrap();
    assert!(again.is_empty(), "the same again changes nothing, the mark kept: {again:?}");
    assert!(environment.vars["api"].agents);
}

#[test]
fn what_cannot_be_read_back_or_handed_on_is_refused_with_nothing_changed() {
    let mut environment = made();
    environment.set(&vars(json!({ "api": "one" })), None).unwrap();
    let before = environment.clone();
    for (wrong, agents, says) in [
        (json!({ "1st": "x" }), None, "cannot be the name of a variable"),
        (json!({ "hosts": ["a"] }), None, "is a list, and a variable is plain"),
        (json!({ "gone": null }), None, "is nothing (null)"),
        (json!({ "path": "/x" }), Some(true), "cannot be handed to an agent"),
        (json!({ "MW_TOKEN": "x" }), Some(true), "cannot be handed to an agent"),
        (json!({ "factory_url": "x" }), Some(true), "cannot be handed to an agent"),
    ] {
        let refused = environment.set(&vars(wrong.clone()), agents).unwrap_err();
        assert!(refused.message.contains(says), "{wrong}: {}", refused.message);
        assert_eq!(environment, before, "{wrong}");
    }
    let refused = environment.mark(&names(&["api", "region"]), true).unwrap_err();
    assert_eq!(refused.kind, ErrorKind::NotFound);
    assert!(refused.message.contains("has no variable `region`; it has `api`"), "{}", refused.message);
    assert_eq!(environment, before, "the one that is there is not marked either");
    assert!(environment.unset(&names(&["region"])).is_err());
}

#[test]
fn an_agent_is_shown_the_variables_marked_for_agents_and_nothing_of_the_rest() {
    let mut environment = made();
    environment.set(&vars(json!({ "database": "prod-db", "region": "eu" })), None).unwrap();
    environment.mark(&names(&["region"]), true).unwrap();
    let to_a_person = environment.shown(false, true);
    assert_eq!(to_a_person["vars"].as_object().unwrap().len(), 2);
    let to_an_agent = environment.shown(true, true);
    assert_eq!(to_an_agent["vars"], json!({ "region": { "value": "eu", "agents": true } }));
    assert!(!to_an_agent.to_string().contains("prod-db"));
    assert_eq!(environment.values(true), vars(json!({ "region": "eu" })));
    let unmarked = environment.mark(&names(&["region"]), false).unwrap();
    assert_eq!(unmarked.marked, ["region"]);
    assert_eq!(environment.values(true), Fields::new());
}

#[test]
fn an_environment_is_read_back_as_it_was_kept() {
    let mut environment = made();
    environment.set(&vars(json!({ "api": "one" })), Some(true)).unwrap();
    assert_eq!(environments::project::read(environment.kept()), Some(environment));
    assert_eq!(environments::project::read(json!("not one")), None);
    assert_eq!(environments::project::named("shop\nstaging", "shop"), Some("staging"));
    assert_eq!(environments::project::named("shopfront\nstaging", "shop"), None);
}

#[test]
fn a_secret_is_listed_by_its_name_who_set_it_and_what_an_agent_is_handed_it_as_and_never_its_value() {
    let mut environment = made();
    let by = Principal::system();
    let kept = environment.keep_secret("stripe", &by, 9).unwrap();
    assert_eq!(kept.secrets, ["stripe"]);
    assert_eq!(environment.secrets["stripe"].agents, None, "handed to no agent unless said");
    let marked = environment.mark_secret("stripe", Some("STRIPE_KEY")).unwrap();
    assert_eq!(marked.secrets_marked, ["stripe"]);
    assert!(environment.mark_secret("stripe", Some("STRIPE_KEY")).unwrap().is_empty(), "the same again");
    let again = environment.keep_secret("stripe", &by, 10).unwrap();
    assert_eq!(again.secrets, ["stripe"], "set again is said again: its value is another");
    assert_eq!(environment.secrets["stripe"].agents.as_deref(), Some("STRIPE_KEY"), "its mark kept");
    assert_eq!((environment.secrets["stripe"].set, environment.vars.len()), (10, 0));

    // What it is handed as is one name an agent's program is handed one thing by.
    environment.keep_secret("hook", &by, 11).unwrap();
    let before = environment.clone();
    for (variable, says) in [
        ("STRIPE_KEY", "is handed the secret `stripe` as `STRIPE_KEY` already"),
        ("PATH", "cannot be handed to an agent"),
        ("2x", "cannot be the variable an agent is handed `hook` as"),
    ] {
        let refused = environment.mark_secret("hook", Some(variable)).unwrap_err();
        assert!(refused.message.contains(says), "{variable}: {}", refused.message);
    }
    let refused = environment.set(&vars(json!({ "STRIPE_KEY": "x" })), Some(true)).unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict, "{}", refused.message);
    assert!(environment.mark_secret("nowhere", None).is_err());
    assert!(environment.keep_secret("../up", &by, 12).is_err());
    assert_eq!(environment, before, "nothing refused changed anything");

    // Shown by name to whoever may name secrets, an agent the marked alone, and nobody else any.
    let to_a_person = environment.shown(false, true);
    assert_eq!(to_a_person["secrets"].as_object().unwrap().len(), 2);
    let to_an_agent = environment.shown(true, true);
    assert_eq!(to_an_agent["secrets"].as_object().unwrap().keys().collect::<Vec<_>>(), ["stripe"]);
    assert!(environment.shown(false, false).get("secrets").is_none(), "not an empty list: not said");

    let dropped = environment.drop_secret("stripe").unwrap();
    assert_eq!(dropped.secrets_removed, ["stripe"]);
    assert!(environment.drop_secret("stripe").is_err());
    assert_eq!(environments::project::read(environment.kept()), Some(environment));
}

#[test]
fn a_secret_is_sought_by_its_project_environment_and_name() {
    assert_eq!(environments::project::sought("shop", "prod", "stripe"), "shop/prod/stripe");
    for wrong in ["", "a/b", "..", "a\\b"] {
        assert!(environments::project::secret_refused(wrong).is_some(), "{wrong}");
    }
    assert!(environments::project::secret_refused("stripe_key-2").is_none());
}

#[test]
fn an_environment_says_the_blueprint_it_is_loaded_by_and_a_name_that_cannot_be_one_is_refused() {
    let mut environment = made();
    assert_eq!(environment.shown(false, false)["deploy"], Value::Null, "none until it is said");
    let said = environment.say_deploy(Some("site/deploy")).unwrap();
    assert_eq!(said, Changed { deploy: true, ..Changed::default() });
    assert!(!said.is_empty());
    assert!(
        environment.say_deploy(Some("site/deploy")).unwrap().is_empty(),
        "the same again changes nothing"
    );
    for wrong in ["", "../up", "/abs", "a b", "a//b", "deploy/", "x;y"] {
        let refused = environment.say_deploy(Some(wrong)).unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Invalid, "{wrong}");
        assert!(
            refused.message.contains("cannot be the name of a blueprint"),
            "{wrong}: {}",
            refused.message
        );
    }
    assert_eq!(environment.deploy.as_deref(), Some("site/deploy"), "a name refused changes nothing");
    assert_eq!(environment.say_deploy(None).unwrap(), Changed { deploy: true, ..Changed::default() });
    assert_eq!(environment.deploy, None);
}

#[test]
fn what_an_environment_last_ran_is_shown_to_a_person_and_nothing_of_it_to_an_agent_and_read_back() {
    let mut environment = made();
    environment.say_deploy(Some("deploy")).unwrap();
    environment.ran = Some(environments::project::Ran {
        branch: "origin/dev".into(),
        commit: "abc123".into(),
        blueprint: "deploy".into(),
        at: 9,
        by: "user:ada".into(),
        by_name: "Ada".into(),
        instance: "in_1".into(),
        run: Some("run_1".into()),
        ended: None,
        down: None,
    });
    let shown = environment.shown(false, false);
    assert_eq!(
        (&shown["deploy"], &shown["ran"]["branch"], &shown["ran"]["run"]),
        (&json!("deploy"), &json!("origin/dev"), &json!("run_1"))
    );
    let to_an_agent = environment.shown(true, true);
    assert!(to_an_agent.get("deploy").is_none() && to_an_agent.get("ran").is_none(), "{to_an_agent}");
    assert_eq!(environments::project::read(environment.kept()), Some(environment.clone()));
    // One kept before an environment could say either reads as saying neither.
    let before = json!({ "name": "staging", "vars": {}, "made": 7, "by": "system", "by_name": "system" });
    let read = environments::project::read(before).unwrap();
    assert_eq!((read.deploy, read.ran), (None, None));
}

#[test]
fn how_a_load_came_out_is_read_from_what_its_run_gave_out() {
    use environments::project::Ended;
    let given = json!({ "address": " http://127.0.0.1:9/live ", "cards": ["shop-1", "shop-2"] });
    let ended = Ended::of(true, 7, &given, Some("never said".into()));
    assert_eq!(
        (ended.ready, ended.at, ended.address.as_deref(), ended.cards.clone(), ended.reason),
        (true, 7, Some("http://127.0.0.1:9/live"), vec!["shop-1".to_string(), "shop-2".to_string()], None),
        "where it went well, no reason is kept"
    );
    // Cards given as words with commas between, and an address that is nothing, which is none.
    let given = json!({ "address": "  ", "cards": "shop-3, ,shop-4" });
    let ended = Ended::of(false, 9, &given, Some("it broke".into()));
    assert_eq!(
        (ended.address, ended.cards, ended.reason.as_deref()),
        (None, vec!["shop-3".into(), "shop-4".into()], Some("it broke"))
    );
    // A run that gave out nothing holds no cards and is reached nowhere.
    let ended = Ended::of(true, 1, &Value::Null, None);
    assert_eq!((ended.address, ended.cards), (None, Vec::<String>::new()));
}
