//! What an environment last ran, in words: a table of cases, one for each
//! thing it can say and for what it must never say.

use environments::last::{Last, Once, RUNS, Read, Thing, last};
use environments::project::Ran;

fn thing(id: &str, parent: Option<&str>, environment: &str, blueprint: &str, key: &str, live: bool) -> Thing {
    Thing {
        id: id.into(),
        parent: parent.map(Into::into),
        environment: environment.into(),
        blueprint: blueprint.into(),
        key: key.into(),
        by_name: "Ada".into(),
        live,
    }
}

fn once(id: &str, instance: &str, started: i64, status: &str) -> Once {
    Once { id: id.into(), instance: instance.into(), started, status: status.into() }
}

fn ran() -> Ran {
    Ran {
        branch: "origin/dev".into(),
        commit: "abcdef0123456789".into(),
        blueprint: "deploy".into(),
        at: 5,
        by: "user:ada".into(),
        by_name: "Ada".into(),
        instance: "in_d".into(),
        run: Some("run_d".into()),
        ended: None,
        down: None,
    }
}

/// A project with a preview applied under `dev` and still running, a
/// stage of it under the preview, and something under `staging`.
fn read() -> Read {
    Read {
        runs: vec![
            once("run_s", "in_s", 40, "ok"),
            once("run_c", "in_c", 30, "running"),
            once("run_p", "in_p", 20, "ok"),
        ],
        things: vec![
            thing("in_s", None, "staging", "smoke", "smoke", false),
            thing("in_c", Some("in_p"), "dev", "std/working", "working-1", true),
            thing("in_p", None, "dev", "preview", "dev-preview", true),
            thing("in_n", None, "", "std/board", "board", true),
        ],
        whole: true,
    }
}

fn said(last: &Last) -> (&'static str, &str, Option<i64>) {
    (last.kind, last.said.as_str(), last.at)
}

#[test]
fn what_was_loaded_through_run_is_said_first_whatever_else_ran() {
    let ran = ran();
    let loaded = last("shop", "dev", Some("deploy"), Some((&ran, Some("ok"))), Some(&read()));
    assert_eq!(said(&loaded), ("loaded", "origin/dev at abcdef0, went well, by Ada", Some(5)));
    assert_eq!((loaded.run.as_deref(), loaded.instance.as_deref()), (Some("run_d"), Some("in_d")));
    let commitless = Ran { commit: String::new(), ..ran.clone() };
    let loaded = last("shop", "dev", None, Some((&commitless, None)), None);
    assert_eq!(
        said(&loaded),
        ("loaded", "origin/dev, not known, by Ada", Some(5)),
        "a commit nothing says is not said"
    );
    let failed = last("shop", "dev", None, Some((&ran, Some("failed"))), None);
    assert_eq!(failed.said, "origin/dev at abcdef0, failed, by Ada");
}

#[test]
fn otherwise_the_latest_run_under_it_is_said_as_what_was_applied() {
    // The latest under dev is a run of a stage under the preview: said as the preview, by its key.
    let applied = last("shop", "dev", None, None, Some(&read()));
    assert_eq!(said(&applied), ("applied", "preview as dev-preview, running, by Ada", Some(30)));
    assert_eq!((applied.run.as_deref(), applied.instance.as_deref()), (Some("run_c"), Some("in_p")));
    // A key that is its blueprint's name is said once.
    let staging = last("shop", "staging", Some("deploy"), None, Some(&read()));
    assert_eq!(said(&staging), ("applied", "smoke, went well, by Ada", Some(40)));
    // What is under no environment is nobody's last run.
    let mut none = read();
    none.runs.push(once("run_n", "in_n", 50, "ok"));
    assert_eq!(last("shop", "dev", None, None, Some(&none)).said, "preview as dev-preview, running, by Ada");
    // A run whose instance was not read says nothing of which environment it was under.
    let mut unread = read();
    unread.runs.insert(0, once("run_x", "in_x", 60, "ok"));
    assert_eq!(last("shop", "dev", None, None, Some(&unread)).run.as_deref(), Some("run_c"));
}

#[test]
fn something_running_under_it_with_no_run_read_is_said_and_never_that_it_never_ran() {
    let mut read = read();
    read.runs.clear();
    let whole = last("shop", "dev", Some("deploy"), None, Some(&read));
    assert_eq!(said(&whole), ("running", "2 running under it, with no run yet", None));
    read.whole = false;
    let cut = last("shop", "dev", None, None, Some(&read));
    assert_eq!(cut.said, format!("2 running under it, its last run older than the project's latest {RUNS}"));
    for (deploy, read) in [(None, &read), (Some("deploy"), &read)] {
        assert!(!last("shop", "dev", deploy, None, Some(read)).said.contains("never"), "{read:?}");
    }
}

#[test]
fn where_nothing_ran_one_that_names_no_deploy_blueprint_says_the_command_that_names_one() {
    let nothing = Read { whole: true, ..Read::default() };
    let unsaid = last("shop", "dev", None, None, Some(&nothing));
    let command = format!("`{} project environment shop dev deploy <blueprint>`", brand::CLI);
    assert_eq!(
        said(&unsaid),
        (
            "unsaid",
            format!("nothing has run under it, and no blueprint loads it from a branch: {command} names one")
                .as_str(),
            None
        )
    );
    let cut = Read { whole: false, ..Read::default() };
    assert_eq!(
        last("shop", "dev", None, None, Some(&cut)).said,
        format!(
            "nothing among the project's latest {RUNS} runs ran under it, and no blueprint loads it from a branch: {command} names one"
        )
    );
    // Whoever may not be told what runs is told only what the environment says of itself.
    assert_eq!(
        last("shop", "dev", None, None, None).said,
        format!("No blueprint loads it from a branch: {command} names one")
    );
}

#[test]
fn never_run_is_said_only_where_what_was_read_is_all_there_is() {
    let nothing = Read { whole: true, ..Read::default() };
    assert_eq!(
        said(&last("shop", "dev", Some("deploy"), None, Some(&nothing))),
        ("never", "never run", None)
    );
    let cut = Read { whole: false, ..Read::default() };
    let among = last("shop", "dev", Some("deploy"), None, Some(&cut));
    assert_eq!(among.said, format!("nothing among the project's latest {RUNS} runs ran under it"));
    assert_eq!(last("shop", "dev", Some("deploy"), None, None).said, "never loaded from a branch");
    // Something done under another environment is not this one's.
    assert_eq!(
        last("shop", "prod", Some("deploy"), None, Some(&Read { whole: true, ..read() })).said,
        "never run"
    );
}
