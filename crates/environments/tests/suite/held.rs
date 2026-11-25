//! A project's environments as what runs under one reads them: held for
//! every conductor, with what an instance applied under one sees of it,
//! which secret a name it writes is, and what is said of an apply that
//! names none or another.

use environments::held::{Chosen, Held, Sought, chosen, must_choose, not_another};
use environments::project::Environment;
use kernel::ErrorKind;
use kernel::Principal;
use serde_json::json;

fn prod() -> Environment {
    let mut prod = Environment::new("prod", &Principal::system(), 1).unwrap();
    prod.set(&kernel::fields(json!({ "api": "https://shop.example", "region": "eu" })), None).unwrap();
    prod.mark(&["region".to_string()], true).unwrap();
    prod.keep_secret("stripe", &Principal::system(), 2).unwrap();
    prod.keep_secret("mail", &Principal::system(), 2).unwrap();
    prod.mark_secret("mail", Some("MAIL_KEY")).unwrap();
    prod
}

fn held() -> Held {
    let held = Held::default();
    held.put("shop", &prod());
    held
}

#[test]
fn what_runs_under_an_environment_reads_its_name_and_variables_and_an_agents_the_marked_alone() {
    let held = held();
    assert_eq!(
        held.view(Some("shop"), "prod", false),
        json!({ "name": "prod", "vars": { "api": "https://shop.example", "region": "eu" } })
    );
    assert_eq!(held.view(Some("shop"), "prod", true), json!({ "name": "prod", "vars": { "region": "eu" } }));
    // Under none, under one taken away since, and of no project: no variables.
    assert_eq!(held.view(Some("shop"), "", false), json!({ "name": "", "vars": {} }));
    assert_eq!(held.view(None, "prod", false), json!({ "name": "prod", "vars": {} }));
    held.forget("shop", Some("prod"));
    assert_eq!(held.view(Some("shop"), "prod", false), json!({ "name": "prod", "vars": {} }));
}

#[test]
fn a_secret_a_text_names_is_its_environments_where_it_lists_it_and_else_the_factorys() {
    let held = held();
    let sought = |name: &str, agent: bool| held.sought(Some("shop"), "prod", name, agent);
    assert_eq!(sought("stripe", false), Ok(Sought::Environment("shop/prod/stripe".into())));
    assert_eq!(sought("other", false), Ok(Sought::Factory("other".into())));
    assert_eq!(held.sought(Some("shop"), "", "stripe", false), Ok(Sought::Factory("stripe".into())));
    assert_eq!(held.sought(None, "prod", "stripe", false), Ok(Sought::Factory("stripe".into())));
    // Never another project's by its whole name, and never the factory's
    // in place of one that is not marked, for what an agent applied.
    for name in ["blog/prod/stripe", "shop/prod/stripe"] {
        assert!(sought(name, false).unwrap_err().contains("is not a secret's name a text may say"), "{name}");
    }
    assert!(
        sought("stripe", true)
            .unwrap_err()
            .contains("does not hand its secret `stripe` to what an agent applied")
    );
    assert_eq!(sought("mail", true), Ok(Sought::Environment("shop/prod/mail".into())));
    assert_eq!(sought("other", true), Ok(Sought::Factory("other".into())));
    // Under one taken away since, nothing is handed, the factory's least of all.
    held.forget("shop", Some("prod"));
    for name in ["stripe", "other"] {
        assert!(sought(name, false).unwrap_err().contains("which is not there any more"), "{name}");
    }
}

#[test]
fn what_is_held_is_what_each_journal_keeps_and_goes_with_its_project() {
    let held = Held::default();
    let kept = vec![
        ("shop\nprod".to_string(), prod().kept()),
        ("shop\nbroken".to_string(), json!("not one")),
        ("blog\nprod".to_string(), prod().kept()),
    ];
    held.load(kept);
    assert_eq!(held.names("shop"), ["prod"]);
    assert_eq!(held.names("blog"), ["prod"]);
    held.forget("shop", None);
    assert!(held.names("shop").is_empty());
    assert_eq!(held.names("blog"), ["prod"], "another project's is left");
}

#[test]
fn an_apply_that_names_none_or_another_is_told_what_there_is() {
    let names = ["dev".to_string(), "prod".to_string()];
    assert_eq!(
        must_choose("shop", &names),
        "shop runs everything under one of its environments, and this names none: say one of dev, prod \
         (`--environment dev`)"
    );
    assert!(
        not_another("deploy", "prod", "stage")
            .starts_with("`deploy` runs under prod, and this asks for stage")
    );
    assert!(not_another("deploy", "", "stage").starts_with("`deploy` runs under no environment"));
}

#[test]
fn an_apply_runs_under_what_it_is_started_under_else_what_it_names_else_one_somebody_has_to_name() {
    let under = |parent: Option<&str>| parent.map(str::to_string);
    // Started under something, it is under that, and asking for another is refused.
    assert_eq!(chosen(under(Some("prod")), None, "kid", Some("shop"), false), Ok(Chosen::Is("prod".into())));
    assert_eq!(chosen(under(Some("")), None, "kid", Some("shop"), true), Ok(Chosen::Is(String::new())));
    let refused = chosen(under(Some("prod")), Some("stage"), "kid", Some("shop"), false).unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    // Named, it is the project's of that name; for no project, refused.
    assert_eq!(
        chosen(None, Some("prod"), "deploy", Some("shop"), true),
        Ok(Chosen::Named("shop".into(), "prod".into()))
    );
    assert_eq!(chosen(None, Some("prod"), "deploy", None, true).unwrap_err().kind, ErrorKind::Invalid);
    // Named by nobody: somebody's asking is for one of the project's, and nothing else is under any.
    assert_eq!(chosen(None, None, "deploy", Some("shop"), true), Ok(Chosen::OneOf("shop".into())));
    assert_eq!(chosen(None, None, "deploy", Some("shop"), false), Ok(Chosen::Is(String::new())));
    assert_eq!(chosen(None, None, "deploy", None, true), Ok(Chosen::Is(String::new())));
    // Asked for as the empty name: none, on purpose, whatever the project has.
    assert_eq!(chosen(None, Some(""), "board", Some("shop"), true), Ok(Chosen::Is(String::new())));
    assert_eq!(chosen(None, Some(""), "board", None, true), Ok(Chosen::Is(String::new())));
}
