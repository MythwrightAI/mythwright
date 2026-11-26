//! One test binary for the crate; add modules here.

mod added;
mod addressed;
mod anchor;
mod approvals;
mod branches;
mod columns;
mod compared;
mod contents;
mod discussed;
mod experiments;
mod flags;
mod following;
mod go;
mod outline;
mod parts;
mod partsread;
mod pressed;
mod ranked;
mod repository;
mod rolled;
mod scopes;
mod sought;
mod topics;
mod trash;
mod whole;
mod window;
mod worded;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use kernel::clock::rfc3339;
use kernel::event::{topic_matches, topic_matches_any};
use kernel::{Cancel, Clock, ManualClock, Principal, PrincipalKind};

#[tokio::test]
async fn a_manual_clock_only_moves_when_told() {
    let clock = ManualClock::new();
    let start = clock.now_ms();
    let woke = Arc::new(AtomicBool::new(false));
    let task = tokio::spawn({
        let (clock, woke) = (clock.clone(), woke.clone());
        async move {
            clock.sleep(Duration::from_secs(300 * 60)).await;
            woke.store(true, Ordering::SeqCst);
        }
    });
    clock.sleeps_started(1).await;
    assert_eq!(clock.sleeping(), 1);

    clock.advance(Duration::from_secs(299 * 60));
    tokio::task::yield_now().await;
    assert!(!woke.load(Ordering::SeqCst), "a minute early");

    clock.advance(Duration::from_secs(60));
    task.await.unwrap();
    assert!(woke.load(Ordering::SeqCst));
    assert_eq!(clock.now_ms() - start, 300 * 60 * 1000);
    assert_eq!(clock.sleeping(), 0);
}

#[tokio::test]
async fn a_sleep_until_a_time_wakes_at_that_time_however_far_the_clock_moved_after_it_was_read() {
    let clock = ManualClock::new();
    let wake_at = clock.now_ms() + 60_000;
    // Moved half way between the time being worked out and the sleep: a
    // sleep of the minute from then would wake half a minute late.
    clock.advance(Duration::from_secs(30));
    let sleep = clock.sleep_until(wake_at);
    clock.advance(Duration::from_secs(30));
    sleep.await;
    // And one whose time has gone by is over at once.
    clock.sleep_until(wake_at).await;
}

#[tokio::test]
async fn a_zero_sleep_returns_at_once() {
    ManualClock::new().sleep(Duration::ZERO).await;
}

#[tokio::test]
async fn cancelling_a_parent_cancels_its_children_but_not_the_reverse() {
    let parent = Cancel::new();
    let child = parent.child();
    let sibling = parent.child();
    let grandchild = child.child();

    child.cancel();
    assert!(child.is_cancelled() && grandchild.is_cancelled());
    assert!(!parent.is_cancelled() && !sibling.is_cancelled());
    grandchild.cancelled().await;

    let waiting = tokio::spawn({
        let sibling = sibling.clone();
        async move { sibling.cancelled().await }
    });
    tokio::task::yield_now().await;
    assert!(!waiting.is_finished());
    parent.cancel();
    waiting.await.unwrap();
}

#[test]
fn ids_sort_by_time_and_secrets_are_long() {
    let earlier = kernel::id::new("run", 1_767_225_600_000);
    let later = kernel::id::new("run", 1_767_225_600_001);
    assert!(earlier < later, "{earlier} {later}");
    assert!(earlier.starts_with("run_") && earlier.len() == 20);
    assert_ne!(kernel::id::new("run", 0), kernel::id::new("run", 0));

    let secret = kernel::id::secret();
    assert_eq!(secret.len(), 52);
    assert_ne!(secret, kernel::id::secret());
    assert!((0..100).all(|_| kernel::id::random_below(10) < 10));
}

#[test]
fn ids_made_in_one_millisecond_sort_in_the_order_they_were_made_whatever_is_made_between() {
    // A clock that stands still, as a test's does, with another's ids
    // made in between: each id is put after the last of its millisecond
    // where rows are kept by them, and never among the pages before it.
    let still = 1_767_225_600_000;
    let mut made = Vec::new();
    for n in 0..500 {
        made.push(kernel::id::new("run", still));
        kernel::id::new("run", still + 1 + n);
    }
    let mut sorted = made.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted, made, "ids of one millisecond did not follow one another");
}

#[test]
fn timestamps_format_as_rfc3339() {
    assert_eq!(rfc3339(0), "1970-01-01T00:00:00.000Z");
    assert_eq!(rfc3339(1_767_225_600_000), "2026-01-01T00:00:00.000Z");
    assert_eq!(rfc3339(1_709_210_096_789), "2024-02-29T12:34:56.789Z");
    assert_eq!(rfc3339(-1), "1969-12-31T23:59:59.999Z");
}

#[test]
fn topic_patterns() {
    assert!(topic_matches("board.item.moved", "board.item.moved"));
    assert!(topic_matches("board.item.*", "board.item.moved"));
    assert!(!topic_matches("board.*", "board.item.moved"));
    assert!(topic_matches("board.**", "board.item.moved"));
    assert!(topic_matches("**", "anything.at.all"));
    assert!(topic_matches("**.moved", "board.item.moved"));
    assert!(topic_matches("board.**", "board"));
    assert!(!topic_matches("board.item.moved", "board.item"));
    assert!(topic_matches_any("step.*, board.**", "board.item.moved"));
    assert!(!topic_matches_any("step.*, run.*", "board.item.moved"));
}

#[test]
fn scopes_grant_by_area_verb_or_wildcard() {
    let agent = Principal {
        kind: PrincipalKind::Agent,
        id: "builder".into(),
        name: "builder".into(),
        scopes: vec!["board".into(), "channels:read".into()],
        key: None,
    };
    assert!(agent.can("board:write") && agent.can("board:read"));
    assert!(agent.can("channels:read") && !agent.can("channels:write"));
    assert!(!agent.can("admin"));
    assert_eq!(agent.actor(), "agent:builder");
    let err = agent.require("shell").unwrap_err();
    assert!(err.message.contains("needs `shell`"), "{}", err.message);
    assert!(Principal::system().can("anything:at-all"));
}

#[test]
fn what_is_kept_to_its_members_is_held_by_nobody_whatever_they_hold() {
    use kernel::principal::{MEMBER, covers, kept_to};
    // What is held, what is asked, and whether holding the one is holding the other.
    let cases: &[(&[&str], &str, bool)] = &[
        (&["*"], "member:ch_1", false),
        (&["member"], "member:ch_1", false),
        (&["member:ch_1"], "member:ch_1", false),
        (&["private", "admin", "channels"], "member:ch_1", false),
        (&[], "member:ch_1", false),
        (&["*"], "member:", false),
        // What only begins as it does, or says it further on, is a scope like any other.
        (&["*"], "members:read", true),
        (&["members"], "members:read", true),
        (&["member"], "member", true),
        (&["private"], "private:member:ch_1", true),
        (&["channels:read"], "channels:read", true),
        (&["channels:read"], "channels:member:", false),
    ];
    for (holds, scope, so) in cases {
        let held: Vec<String> = holds.iter().map(|scope| scope.to_string()).collect();
        assert_eq!(covers(&held, scope), *so, "whether holding {holds:?} is holding `{scope}`");
    }
    assert!(!Principal::system().can("member:ch_1"), "the system holds everything, and is no member by that");
    assert!(Principal::system().require("member:ch_1").is_err());
    assert_eq!(kept_to("member:ch_1"), Some("ch_1"));
    assert_eq!(kept_to(&format!("{MEMBER}a:b")), Some("a:b"));
    assert_eq!((kept_to("member"), kept_to("private:journal"), kept_to("members:read")), (None, None, None));
}

#[test]
fn leave_to_use_one_extension_package_is_not_leave_for_another_or_for_a_shell() {
    let with = |scopes: &[&str]| Principal {
        kind: PrincipalKind::Agent,
        id: "builder".into(),
        name: "builder".into(),
        scopes: scopes.iter().map(|scope| scope.to_string()).collect(),
        key: None,
    };
    let one = with(&["extensions:example"]);
    assert!(one.can("extensions:example"));
    assert!(!one.can("extensions:another") && !one.can("extensions") && !one.can("shell"));
    assert!(with(&["extensions"]).can("extensions:example"), "the area is every package's");
    assert!(!with(&["shell"]).can("extensions:example"), "and a shell is not a package");

    // A person may use them unless that was taken away; an agent may not
    // unless it was given.
    use kernel::principal::{AGENT_SCOPES, MEMBER_SCOPES, SCOPES};
    assert!(SCOPES.iter().any(|(scope, _)| *scope == "extensions"), "it is one of the scopes there are");
    assert!(with(MEMBER_SCOPES).can("extensions:example") && with(MEMBER_SCOPES).can("shell"));
    assert!(!with(AGENT_SCOPES).can("extensions:example") && !with(AGENT_SCOPES).can("shell"));
}

/// What a command would be started with: its arguments, and what it adds
/// to the environment.
fn asked(command: &std::process::Command) -> (Vec<String>, Vec<(String, String)>) {
    let text = |text: &std::ffi::OsStr| text.to_string_lossy().into_owned();
    let args = command.get_args().map(text).collect();
    let env = command.get_envs().filter_map(|(name, value)| Some((text(name), text(value?)))).collect();
    (args, env)
}

#[test]
fn git_is_asked_with_nothing_of_the_repositorys_run_for_it_and_nobody_prompted() {
    // A hook is a file in the repository, or runs one, and so is the
    // program a repository may name to say what changed; and the daemon
    // asks with no wall around it.
    let git = kernel::git::git(std::path::Path::new("/a/project"));
    assert_eq!(git.get_program(), "git");
    let (args, env) = asked(&git);
    assert_eq!(args, ["-C", "/a/project", "-c", "core.hooksPath=/dev/null", "-c", "core.fsmonitor=false"]);
    // No lock is taken that the asking can do without; and a remote that
    // wants a password fails at once, where a prompt nobody sees would
    // wait for ever.
    let told = |name: &str| (name.to_string(), "0".to_string());
    assert_eq!(env, [told("GIT_OPTIONAL_LOCKS"), told("GIT_TERMINAL_PROMPT")]);
}

#[test]
fn git_is_not_handed_who_commits_by_the_environment_it_is_asked_from() {
    // Git goes by an author and a committer named in its environment
    // before what the repository says, and the daemon's environment is
    // whatever it was started in. Each is taken out, asked plainly and
    // asked as a key: a variable a command is told to do without is one it
    // names with nothing to hand on.
    let dir = std::path::Path::new("/a/project");
    for git in [kernel::git::git(dir), kernel::git::git_as_key(dir, "https://example.invalid/shop.git")] {
        let without: Vec<String> = git
            .get_envs()
            .filter(|(_, value)| value.is_none())
            .map(|(name, _)| name.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            without,
            ["GIT_AUTHOR_EMAIL", "GIT_AUTHOR_NAME", "GIT_COMMITTER_EMAIL", "GIT_COMMITTER_NAME"],
            "the four that name who commits, and nothing else is taken out"
        );
    }
    assert_eq!(kernel::git::WHO.len(), 4);
}

#[test]
fn asked_as_a_key_git_is_told_it_has_no_helper_and_then_that_it_has_this_program() {
    let git = kernel::git::git_as_key(std::path::Path::new("/a/project"), "https://example.invalid/shop.git");
    let (args, env) = asked(&git);
    // As it is asked without one, and then the two things, in this order:
    // the first is what shuts out whoever runs the daemon.
    let (plain, _) = asked(&kernel::git::git(std::path::Path::new("/a/project")));
    assert_eq!(args[..plain.len()], plain[..]);
    assert_eq!(
        args[plain.len()..],
        ["-c", "credential.helper=", "-c", "credential.helper=!\"$FACTORY_CLI\" credential"]
    );
    // Which remote the key is for is said beside it; the key itself is not
    // here at all: whoever starts the command hands it over as it starts.
    assert!(env.contains(&("FACTORY_REMOTE".to_string(), "https://example.invalid/shop.git".to_string())));
    assert!(env.iter().all(|(name, _)| name != kernel::git::KEY), "{env:?}");
    assert_eq!((kernel::git::KEY, kernel::git::REMOTE), ("FACTORY_REMOTE_KEY", "FACTORY_REMOTE"));
}

#[test]
fn a_key_is_for_a_remote_reached_over_https_and_no_other() {
    for address in ["https://example.invalid/shop.git", "http://127.0.0.1:8080/shop.git"] {
        assert!(kernel::git::takes_a_key(address), "{address}");
    }
    for address in [
        "git@example.invalid:shop.git",
        "ssh://example.invalid/shop.git",
        "/srv/shop.git",
        "file:///srv/shop.git",
        "",
    ] {
        assert!(!kernel::git::takes_a_key(address), "{address}");
    }
}

#[test]
fn a_lock_git_was_refused_is_read_from_what_git_said_and_nothing_else_is_taken_for_one() {
    use kernel::git::lock_met;
    // As git says it of an index, of a branch and of the repository's settings.
    let index = "fatal: Unable to create '/a/project/.git/index.lock': File exists.\n\nAnother git process seems to be running in this repository";
    assert_eq!(lock_met(index).as_deref(), Some("/a/project/.git/index.lock"));
    let branch = "error: cannot lock ref 'refs/heads/lane/shop-1': Unable to create '/a/project/.git/refs/heads/lane/shop-1.lock': File exists.";
    assert_eq!(lock_met(branch).as_deref(), Some("/a/project/.git/refs/heads/lane/shop-1.lock"));
    let settings = "error: could not lock config file .git/config: File exists";
    assert_eq!(lock_met(settings).as_deref(), Some(".git/config.lock"));
    // What is in the way for another reason is not a lock to wait for.
    for other in [
        "fatal: Unable to create '/a/project/.git/index.lock': Permission denied",
        "error: Your local changes to the following files would be overwritten by merge:\n  index.lock",
        "CONFLICT (content): Merge conflict in a.txt",
        "",
    ] {
        assert_eq!(lock_met(other), None, "{other}");
    }
    // And everything asked takes no lock it can do without.
    let git = kernel::git::git(std::path::Path::new("/a/project"));
    let told = git.get_envs().find(|(name, _)| *name == "GIT_OPTIONAL_LOCKS").and_then(|(_, value)| value);
    assert_eq!(told.and_then(|value| value.to_str()), Some("0"));
}

#[test]
fn every_scope_the_system_checks_is_in_one_area_and_holding_the_area_holds_each_of_them() {
    let areas = kernel::principal::areas();
    // Every scope is there once: as an area that is one thing, or as something done in one.
    let mut listed: Vec<String> = Vec::new();
    for area in &areas {
        if area.does.is_some() {
            listed.push(area.area.to_string());
        }
        listed.extend(area.verbs.iter().map(|verb| format!("{}:{}", area.area, verb.verb)));
        assert!(area.does.is_some() || !area.verbs.is_empty(), "{} allows nothing", area.area);
    }
    let mut checked: Vec<String> =
        kernel::principal::SCOPES.iter().map(|(scope, _)| scope.to_string()).collect();
    listed.sort();
    checked.sort();
    assert_eq!(listed, checked);
    // No area is listed twice, and they come in the order they are written.
    let names: Vec<&str> = areas.iter().map(|area| area.area).collect();
    assert_eq!(names[..4], ["admin", "projects", "channels", "inbox"]);
    assert_eq!(names.len(), names.iter().collect::<std::collections::BTreeSet<_>>().len());
    // What is shown of an area is what a credential is held to there.
    let board = areas.iter().find(|area| area.area == "board").unwrap();
    assert_eq!(board.verbs.iter().map(|verb| verb.verb).collect::<Vec<_>>(), ["read", "write"]);
    let holds = |scopes: &[&str]| Principal {
        kind: PrincipalKind::Agent,
        id: "a".into(),
        name: "a".into(),
        scopes: scopes.iter().map(|scope| scope.to_string()).collect(),
        key: None,
    };
    let whole = holds(&["board"]);
    assert!(board.verbs.iter().all(|verb| whole.can(&format!("board:{}", verb.verb))));
    let reads = holds(&["board:read"]);
    assert!(reads.can("board:read") && !reads.can("board:write"));
    // What an agent usually holds is made of scopes there are.
    for scope in kernel::principal::AGENT_SCOPES {
        let area = scope.split(':').next().unwrap();
        assert!(names.contains(&area), "{scope}");
    }
}
