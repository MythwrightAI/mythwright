//! A run that waits for a person to let it go on (`kernel::go`): who may
//! decide it, whom it is told to, how long it waits and what it is said
//! as, each a table of cases.

use kernel::approvals::{LASTS_MS, Status};
use kernel::go::{Asking, Deciding, decided, first, line, may, stands, told, until};
use serde_json::json;

/// Who decides, whether a person, whether they hold it, who started the
/// run, the environment's rule, and what that comes to.
type Case<'a> = (&'a str, bool, bool, &'a str, Option<&'a [String]>, Result<(), &'a str>);

fn actors(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
}

/// Who may decide, and why not, for each case the rule comes to: a person holding `blueprints:run`; where the
/// environment names who approves, one of them, its starter among them
/// only where it names them; and with no list, never its starter.
#[test]
fn who_may_let_a_run_go_on_is_a_person_who_holds_what_it_takes_and_whom_the_rule_names() {
    let prod = actors(&["user:ada", "user:joan"]);
    let none: Vec<String> = Vec::new();
    let cases: &[Case<'_>] = &[
        // Somebody else, holding it, with no rule: yes.
        ("user:grace", true, true, "user:ada", None, Ok(())),
        // An empty list is no rule.
        ("user:grace", true, true, "user:ada", Some(&none), Ok(())),
        // Whoever started it, with no rule: no.
        (
            "user:ada",
            true,
            true,
            "user:ada",
            None,
            Err(
                "you started this run, and somebody else lets it go on: only an environment whose rule names you lets you approve your own",
            ),
        ),
        // Whoever started it, named by the rule: yes.
        ("user:ada", true, true, "user:ada", Some(&prod), Ok(())),
        // Whoever started it, not named by the rule: no, as anybody it does not name.
        (
            "user:grace",
            true,
            true,
            "user:grace",
            Some(&prod),
            Err("what goes under `prod` is approved by those its rule names, and you are not one of them"),
        ),
        // Somebody else the rule names: yes.
        ("user:joan", true, true, "user:grace", Some(&prod), Ok(())),
        // Somebody else the rule does not name: no.
        (
            "user:grace",
            true,
            true,
            "user:ada",
            Some(&prod),
            Err("what goes under `prod` is approved by those its rule names, and you are not one of them"),
        ),
        // Named, and not holding what the next stage takes: no.
        (
            "user:joan",
            true,
            false,
            "user:ada",
            Some(&prod),
            Err("letting a run go on takes `blueprints:run`, which you do not hold"),
        ),
        // An agent, whatever it holds and whatever names it: never.
        (
            "agent:ada",
            false,
            true,
            "user:grace",
            None,
            Err("a run is let go on by a person, never by an agent"),
        ),
        (
            "user:ada",
            false,
            true,
            "user:grace",
            Some(&prod),
            Err("a run is let go on by a person, never by an agent"),
        ),
    ];
    for (by, person, holds, starter, rule, came) in cases {
        let deciding =
            Deciding { by, person: *person, holds: *holds, starter, rule: *rule, environment: "prod" };
        assert_eq!(may(&deciding), came.map_err(str::to_string), "{by} on {starter}'s run, {rule:?}");
    }
}

/// Whether a run a row of a project's triggers begins under an
/// environment waits for one of those its rule names: whom it is begun
/// for (whoever applied the triggers, whoever last wrote the row's
/// environment, whoever asked by hand), and what that comes to.
#[test]
fn a_run_begun_for_a_row_waits_for_a_go_ahead_unless_everybody_it_is_begun_for_is_named_by_the_rule() {
    let prod = actors(&["user:ada", "user:joan"]);
    let none: Vec<String> = Vec::new();
    let cases: &[(&[String], &[&str], bool)] = &[
        // A rule that names nobody asks nothing, whoever wrote the row.
        (&none, &["user:ada", "user:grace"], false),
        (&none, &["user:ada", ""], false),
        // Applied and written by those it names: it goes on.
        (&prod, &["user:ada", "user:ada"], false),
        (&prod, &["user:ada", "user:joan"], false),
        // Written by somebody it does not name, though whoever applied the triggers is named: it waits.
        (&prod, &["user:ada", "user:grace"], true),
        // Written with a key, which is itself and never its maker: it waits.
        (&prod, &["user:ada", "key:k_1"], true),
        // Written before who wrote it was kept: nobody it names, so it waits.
        (&prod, &["user:ada", ""], true),
        // Applied by somebody it does not name: it waits, whoever wrote the row.
        (&prod, &["user:grace", "user:ada"], true),
        // Asked for by hand by somebody it does not name: it waits.
        (&prod, &["user:ada", "user:joan", "user:grace"], true),
        (&prod, &["user:ada", "user:joan", "user:joan"], false),
    ];
    for (rule, whom, waits) in cases {
        assert_eq!(first(rule, whom), *waits, "{whom:?} under {rule:?}");
    }
}

#[test]
fn an_ask_is_told_to_those_it_names_of_those_who_may_and_to_all_of_them_where_it_names_none() {
    let could = actors(&["user:ada", "user:joan", "user:grace"]);
    let cases: &[(&[&str], &[&str])] = &[
        (&[], &["user:ada", "user:joan", "user:grace"]),
        (&["user:joan"], &["user:joan"]),
        (&["user:joan", "user:grace"], &["user:joan", "user:grace"]),
        // Somebody who may not decide it is told nothing for being named.
        (&["user:nobody"], &["user:ada", "user:joan", "user:grace"]),
        (&["user:nobody", "user:ada"], &["user:ada"]),
    ];
    for (to, came) in cases {
        assert_eq!(told(&could, &actors(to)), actors(came), "to {to:?}");
    }
    assert_eq!(told(&[], &actors(&["user:ada"])), Vec::<String>::new());
}

#[test]
fn an_ask_waits_as_long_as_its_step_is_given_and_seven_days_where_it_says_nothing() {
    assert_eq!(until(1_000, Some(60_000)), 61_000);
    assert_eq!(until(1_000, None), 1_000 + LASTS_MS);
    assert_eq!(until(1_000, Some(0)), 1_000);
    for (status, now, came) in [
        (Status::Waiting, 60_999, Status::Waiting),
        (Status::Waiting, 61_000, Status::Expired),
        (Status::Waiting, 90_000, Status::Expired),
        (Status::Approved, 90_000, Status::Approved),
        (Status::Declined, 90_000, Status::Declined),
        (Status::Expired, 10, Status::Expired),
    ] {
        assert_eq!(stands(status, 61_000, now), came, "{status:?} at {now}");
    }
}

#[test]
fn what_a_step_asked_is_kept_and_read_back_whole() {
    let asking = Asking {
        says: "Deploy 4f2a to prod".into(),
        blueprint: "pipelines/deploy".into(),
        instance: "in_1".into(),
        environment: "prod".into(),
        links: vec!["https://example.com/preview".into()],
        until: 42,
    };
    assert_eq!(Asking::read(&asking.kept()), asking);
    assert_eq!(Asking::read(&json!(null)), Asking::default());
}

#[test]
fn the_words_of_its_line_and_of_what_came_of_it_say_whose_run_of_what_and_why() {
    let mut asking =
        Asking { says: " Deploy 4f2a ".into(), blueprint: "pipelines/deploy".into(), ..Asking::default() };
    assert_eq!(line("Ada", &asking), "Ada's run of pipelines/deploy waits for your go-ahead: Deploy 4f2a");
    asking.environment = "prod".into();
    asking.says = String::new();
    assert_eq!(line("Ada", &asking), "Ada's run of pipelines/deploy waits for your go-ahead under prod");
    for (status, why, said) in [
        (Status::Approved, "", "Joan let the run of deploy go on."),
        (Status::Declined, " not today ", "Joan declined to let the run of deploy go on: not today."),
        (Status::Declined, "", "Joan declined to let the run of deploy go on."),
        (
            Status::Expired,
            "",
            "Nobody let the run of deploy go on in the time it waits, and it goes no further.",
        ),
        (Status::Waiting, "", "The run of deploy waits for somebody to let it go on."),
    ] {
        assert_eq!(decided(status, "deploy", "Joan", why), said);
    }
}
