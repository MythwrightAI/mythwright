//! What an environment holds to, as plain values and where it is held: a
//! table of cases for the branches it lets run, who approves what is
//! started under it and the words each is said in; what a child of a run
//! under none may name; and the hold itself, in the turn that holds the
//! environment, asking whatever keeps go-aheads.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use environments::held::{Approvals, Chosen, Held, chosen};
use environments::last::{Once, Read, Thing, runs};
use environments::project::Environment;
use environments::rules::{Comes, Entering, Rules, branch_refused, unapproved};
use kernel::{ErrorKind, Principal};

fn rules(approve: &[&str], branches: &[&str]) -> Rules {
    Rules {
        approve: approve.iter().map(|one| one.to_string()).collect(),
        branches: branches.iter().map(|one| one.to_string()).collect(),
    }
}

fn entering<'a>(by: &'a str, branch: Option<&'a str>, run: Option<&'a str>) -> Entering<'a> {
    Entering { by, branch, run }
}

#[test]
fn a_branch_runs_where_the_rule_names_it_whole_or_by_a_beginning_ended_by_a_star_and_anywhere_it_names_none()
{
    let cases: &[(&[&str], &str, bool)] = &[
        (&[], "anything/at-all", true),
        (&["main"], "main", true),
        (&["main"], "main2", false),
        (&["main"], "origin/main", false),
        (&["main", "origin/main"], "origin/main", true),
        (&["release/*"], "release/2", true),
        (&["release/*"], "release/", true),
        (&["release/*"], "releases/2", false),
        (&["*"], "anything", true),
    ];
    for (named, branch, lets) in cases {
        assert_eq!(rules(&[], named).lets(branch), *lets, "{named:?} and `{branch}`");
    }
}

#[test]
fn what_enters_goes_waits_for_one_of_those_who_approve_or_is_refused_for_its_branch_first() {
    let shop = |rule: &Rules, entering: Entering<'_>| rule.comes("shop", "prod", &entering);
    // Nothing said: anything goes, from anybody, of any branch.
    assert_eq!(shop(&Rules::default(), entering("user:ada", Some("x"), None)), Comes::Goes);
    // Who approves: one of them goes, anybody else waits for one of them.
    let approve = rules(&["user:grace", "user:ada"], &[]);
    assert_eq!(shop(&approve, entering("user:ada", None, None)), Comes::Goes);
    let waits = Comes::Waits(vec!["user:grace".into(), "user:ada".into()]);
    assert_eq!(shop(&approve, entering("user:bob", None, Some("run_1"))), waits);
    assert_eq!(shop(&approve, entering("agent:helper", None, None)), waits);
    // Branches: one it does not let run is refused before anybody is asked, whoever it is.
    let both = rules(&["user:grace"], &["main"]);
    let Comes::Refused(why) = shop(&both, entering("user:grace", Some("feature/x"), None)) else {
        panic!("a branch it does not let run is refused")
    };
    assert!(
        why.starts_with("`feature/x` may not be run under `prod` of `shop`, which lets main alone"),
        "{why}"
    );
    assert!(why.contains("project environment shop prod branches"), "{why}");
    assert_eq!(shop(&both, entering("user:grace", Some("main"), None)), Comes::Goes);
    // Handed no branch, it is held to who approves alone.
    assert_eq!(shop(&both, entering("user:bob", None, None)), Comes::Waits(vec!["user:grace".into()]));
}

#[test]
fn what_is_refused_for_want_of_a_go_ahead_says_what_would_let_it() {
    let by_a_run = unapproved("shop", "prod", true);
    assert!(
        by_a_run.starts_with("what is started under `prod` of `shop` is approved by those its rule names")
    );
    assert!(
        by_a_run.contains("a step of `approval.ask` with `environment: prod` before the one that starts it")
    );
    let by_hand = unapproved("shop", "prod", false);
    assert!(by_hand.contains("and you are not one of them"), "{by_hand}");
}

#[test]
fn a_branch_a_rule_names_is_named_as_git_names_one_with_a_star_only_at_its_end() {
    for good in ["main", "origin/main", "release/*", "*", "feature/x-1.2"] {
        assert_eq!(branch_refused(good), None, "{good}");
    }
    for bad in ["", "a b", "re*lease", "a..b", "-x", "/x", "x:y", "x~1", "x^", "a?", "a[b", "a\\b"] {
        assert!(branch_refused(bad).is_some(), "`{bad}` is taken");
    }
}

#[test]
fn an_environment_says_its_branches_once_each_in_order_and_says_what_changed() {
    let mut prod = Environment::new("prod", &Principal::system(), 1).unwrap();
    let said = |names: &[&str]| names.iter().map(|one| one.to_string()).collect::<Vec<_>>();
    let changed = prod.say_branches(&said(&["main", " release/* ", "main"])).unwrap();
    assert!(changed.rules);
    assert_eq!(prod.rules.branches, said(&["main", "release/*"]));
    assert!(prod.say_branches(&said(&["main", "release/*"])).unwrap().is_empty());
    assert_eq!(prod.say_branches(&said(&["a b"])).unwrap_err().kind, ErrorKind::Invalid);
    assert_eq!(prod.rules.branches, said(&["main", "release/*"]), "refused, nothing changed");
    let many: Vec<String> = (0..51).map(|at| format!("b{at}")).collect();
    assert_eq!(prod.say_branches(&many).unwrap_err().kind, ErrorKind::Invalid);
    assert!(prod.say_branches(&[]).unwrap().rules);
    assert!(prod.rules.is_empty());
    // Kept with it only where it says something.
    assert_eq!(prod.kept().get("rules"), None);
}

#[test]
fn a_child_of_what_runs_under_none_may_name_an_environment_and_of_what_runs_under_one_no_other() {
    let under = |name: &str| Some(name.to_string());
    assert_eq!(
        chosen(under(""), Some("prod"), "deploy", Some("shop"), false),
        Ok(Chosen::Named("shop".into(), "prod".into()))
    );
    assert_eq!(chosen(under(""), Some(""), "deploy", Some("shop"), false), Ok(Chosen::Is(String::new())));
    assert_eq!(chosen(under(""), None, "deploy", Some("shop"), false), Ok(Chosen::Is(String::new())));
    assert_eq!(chosen(under(""), Some("prod"), "deploy", None, false).unwrap_err().kind, ErrorKind::Invalid);
    assert_eq!(
        chosen(under("prod"), Some("prod"), "deploy", Some("shop"), false),
        Ok(Chosen::Is("prod".into()))
    );
    let refused = chosen(under("dev"), Some("prod"), "deploy", Some("shop"), false).unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(refused.message.contains("runs under dev, and this asks for prod"), "{}", refused.message);
}

/// Answers from a list of what was approved, `(project, run, environment, by)`, noting each asking.
#[derive(Clone, Default)]
struct Given(Arc<Mutex<(Vec<Approved>, usize)>>);

/// A go-ahead: its project, its run, its environment and who gave it.
type Approved = (String, String, String, String);

impl Approvals for Given {
    fn approved<'a>(
        &'a self,
        project: &'a str,
        run: &'a str,
        environment: &'a str,
        by: &'a [String],
    ) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        let mut kept = self.0.lock().unwrap();
        kept.1 += 1;
        let found = kept.0.iter().any(|(of, at, under, who)| {
            (of.as_str(), at.as_str(), under.as_str()) == (project, run, environment) && by.contains(who)
        });
        Box::pin(async move { found })
    }
}

fn prod_held(approve: &[&str], branches: &[&str]) -> Held {
    let mut prod = Environment::new("prod", &Principal::system(), 1).unwrap();
    prod.rules = rules(approve, branches);
    let held = Held::default();
    held.put("shop", &prod);
    held
}

#[tokio::test]
async fn the_hold_goes_by_the_rules_and_a_go_ahead_of_the_run_that_asks_given_by_one_of_those_who_approve() {
    let held = prod_held(&["user:grace"], &["main"]);
    // Nothing installed: a go-ahead nowhere.
    let refused = held.holding(Some("shop"), "prod", Some(entering("user:ada", None, Some("run_1")))).await;
    assert_eq!(refused.unwrap_err().kind, ErrorKind::Denied);
    let given = Given::default();
    given.0.lock().unwrap().0.push(("shop".into(), "run_1".into(), "prod".into(), "user:grace".into()));
    held.answers(given.clone());
    assert!(
        held.holding(Some("shop"), "prod", Some(entering("user:ada", None, Some("run_1")))).await.is_ok()
    );
    // Another run's go-ahead, or none of a run, is nobody's.
    for run in [Some("run_2"), None] {
        let refused =
            held.holding(Some("shop"), "prod", Some(entering("user:ada", None, run))).await.unwrap_err();
        assert!(refused.message.contains("approved by those its rule names"), "{}", refused.message);
    }
    // A branch it does not let run is refused, and nobody is asked.
    let asked = given.0.lock().unwrap().1;
    let refused =
        held.holding(Some("shop"), "prod", Some(entering("user:ada", Some("dev"), Some("run_1")))).await;
    assert!(refused.unwrap_err().message.contains("`dev` may not be run under `prod`"));
    assert_eq!(given.0.lock().unwrap().1, asked);
    // One of them, and what is under it already (nothing entering), are not asked.
    assert!(
        held.holding(Some("shop"), "prod", Some(entering("user:grace", Some("main"), None))).await.is_ok()
    );
    assert!(held.holding(Some("shop"), "prod", None).await.is_ok());
    assert_eq!(given.0.lock().unwrap().1, asked);
    // An environment taken away is said to be missing, as it always was.
    assert_eq!(held.holding(Some("shop"), "gone", None).await.unwrap_err().kind, ErrorKind::NotFound);
}

#[test]
fn the_runs_that_acted_under_an_environment_are_the_newest_first_each_said_as_what_was_applied_under_it() {
    let thing = |id: &str, parent: Option<&str>, environment: &str, blueprint: &str, key: &str| Thing {
        id: id.into(),
        parent: parent.map(Into::into),
        environment: environment.into(),
        blueprint: blueprint.into(),
        key: key.into(),
        by_name: "Ada".into(),
        live: false,
    };
    let once = |id: &str, instance: &str, started: i64| Once {
        id: id.into(),
        instance: instance.into(),
        started,
        status: "ok".into(),
    };
    let read = Read {
        // A pipeline under none, its child under prod, a step of that child, and a dev run.
        runs: vec![
            once("r4", "in_dev", 4),
            once("r3", "in_step", 3),
            once("r2", "in_child", 2),
            once("r1", "in_pipe", 1),
        ],
        things: vec![
            thing("in_pipe", None, "", "release", "release"),
            thing("in_child", Some("in_pipe"), "prod", "deploy", "deploy-prod"),
            thing("in_step", Some("in_child"), "prod", "std/run", "build"),
            thing("in_dev", None, "dev", "deploy", "deploy-dev"),
        ],
        whole: true,
    };
    let listed = runs("prod", &read);
    let said: Vec<(&str, &str, &str)> =
        listed.iter().map(|acted| (acted.run.as_str(), acted.what.as_str(), acted.top.as_str())).collect();
    assert_eq!(
        said,
        vec![("r3", "deploy as deploy-prod", "in_child"), ("r2", "deploy as deploy-prod", "in_child")]
    );
    assert!(runs("staging", &read).is_empty());
    // Bounded.
    let many =
        Read { runs: (0..30).map(|at| once(&format!("r{at}"), "in_child", at)).collect(), ..read.clone() };
    assert_eq!(runs("prod", &many).len(), environments::last::LISTED);
    // Its last run is the first of them.
    let last = environments::last::last("shop", "prod", Some("deploy"), None, Some(&read));
    assert_eq!(
        (last.run.as_deref(), last.said.as_str()),
        (Some("r3"), "deploy as deploy-prod, went well, by Ada")
    );
}

#[test]
fn what_enters_is_of_the_branch_it_is_handed_as_its_param_and_of_none_where_that_is_empty() {
    let params =
        |branch: serde_json::Value| kernel::fields(serde_json::json!({ "branch": branch, "other": 1 }));
    let main = params(serde_json::json!("main"));
    assert_eq!(
        Entering::of("user:ada", &main, Some("run_1")),
        entering("user:ada", Some("main"), Some("run_1"))
    );
    for none in [serde_json::json!(""), serde_json::json!(3), serde_json::Value::Null] {
        let handed = params(none);
        assert_eq!(Entering::of("user:ada", &handed, None).branch, None);
    }
    assert_eq!(Entering::of("user:ada", &kernel::Fields::new(), None).branch, None);
}
