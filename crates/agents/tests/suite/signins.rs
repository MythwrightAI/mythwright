//! Sign-ins the factory holds: which a turn is handed, that no more hold
//! one than may, what happens at a limit, and that it is given back.
//!
//! The program is a stand-in: a script that writes down what it was handed
//! in its environment, and says it has reached a limit when the test has
//! said the sign-in it holds has one. No real program is run, and nothing
//! here is anybody's sign-in.
//!
//! Unix's alone: the stand-in for an agent's program is a file of a Unix
//! shell that reads its own state from files, made a program by its mode;
//! one test holds it at a pipe with a name.
#![cfg(unix)]

use std::path::PathBuf;
use std::time::Duration;

use agents::rule::{Asked, Decision, Offered, RULE, ask, in_turn};
use agents::{Extension, NewSignIn, SignIn, SlotSpec, TurnInfo, TurnRequest};
use conductor::Library;
use kernel::{Principal, PrincipalKind};
use serde_json::json;

use crate::{Rig, ada, soon};

const SCRIPT: &str = r#"#!/bin/sh
here="HERE"
prompt=$(cat)
env > "$here/env"
held="${STANDIN_KEY-}${STANDIN_TOKEN-}${STANDIN_FOLDER-}"
printf '%s|%s|%s|%s|%s\n' "${STANDIN_KEY-}" "${STANDIN_TOKEN-}" "${STANDIN_FOLDER-}" "${STANDIN_HOME-}" "$*" >> "$here/starts"
if [ -n "${STANDIN_FOLDER-}" ]; then echo renewed >> "$STANDIN_FOLDER/credentials"; fi
if [ -f "$here/limited" ]; then
  while read -r one; do
    if [ "$one" = "$held" ] || [ "$one" = "all" ]; then
      printf '{"kind":"end","bad":true,"text":"limit reached, good again at %s"}\n' "$(cat "$here/until" 2>/dev/null)"
      exit 0
    fi
  done < "$here/limited"
fi
case "$prompt" in
  *crash*) echo "boom" >&2; exit 3 ;;
  *repeat*) printf '{"kind":"end","bad":false,"text":"what I hold is %s"}\n' "$held"; exit 0 ;;
  *spill*) printf '{"kind":"end","bad":true,"text":"rejected: %s"}\n' "$held"; exit 0 ;;
  *hold:*) name=${prompt##*hold:}; name=${name%%]*}; echo here > "$here/at-$name"; read -r go < "$here/gate-$name" ;;
esac
printf '{"kind":"end","bad":false,"text":"done","talk":"s-1"}\n'
"#;

const MANIFEST: &str = r#"
extension: standin
title: Stand-in
description: Not a real agent.
agent:
  program: PROGRAM
  args:
    - ["run"]
    - when: ${ turn.session != null }
      args: ["--carry-on", "${ turn.session }"]
  input: ${ turn.prompt }
  output:
    - when: ${ event.talk != null }
      session: ${ event.talk }
    - when: ${ event.kind == "end" }
      reply: ${ event.text }
      failed: ${ event.bad == true }
      error: ${ event.text }
SIGNIN"#;

const SIGNIN: &str = r#"  signin:
    key: STANDIN_KEY
    token: STANDIN_TOKEN
    folder: STANDIN_FOLDER
    home: STANDIN_HOME
    without: [STANDIN_PROFILE]
    limit:
      when: ${ matches(error, "limit reached") }
      until: ${ int(split(error, "good again at ")[1]) }
      for: 5h
"#;

/// A rig with the stand-in's package installed, and where the stand-in
/// keeps what it writes down.
struct Pool {
    rig: Rig,
    here: PathBuf,
}

impl Pool {
    async fn new() -> Pool {
        Pool::with(SIGNIN).await
    }

    /// With a package that says `signin` of how its program signs in.
    async fn with(signin: &str) -> Pool {
        let rig = Rig::new().await;
        let here = rig.dir.join("standin");
        std::fs::create_dir_all(&here).unwrap();
        let program = here.join("program");
        crate::program(&program, &SCRIPT.replace("HERE", &here.to_string_lossy()));
        let manifest = MANIFEST.replace("PROGRAM", &program.to_string_lossy()).replace("SIGNIN", signin);
        rig.agents.add_extension(&manifest, &ada()).await.unwrap();
        Pool { rig, here }
    }

    /// Adds a sign-in that holds `value`.
    async fn holds(&self, name: &str, kind: &str, value: &str, at_once: u32) -> SignIn {
        let new = NewSignIn {
            name: name.into(),
            package: "standin".into(),
            kind: kind.into(),
            value: Some(value.into()),
            folder: Some(value.into()),
            at_once: Some(at_once),
        };
        self.rig.agents.signin_add(new, &ada()).await.unwrap()
    }

    /// An agent on the stand-in that draws its sign-in from `draws`.
    async fn agent(&self, name: &str, draws: &[&str]) {
        let spec = SlotSpec {
            name: name.into(),
            adapter: Some("standin".into()),
            signins: Some(draws.iter().map(|draw| draw.to_string()).collect()),
            ..SlotSpec::default()
        };
        self.rig.agents.ensure(spec, &ada()).await.unwrap();
    }

    /// Gives an agent a piece of work, and returns the turn's id.
    async fn asks(&self, slot: &str, prompt: &str) -> String {
        let request = TurnRequest {
            slot: slot.into(),
            kind: "task".into(),
            thread: "the-thread".into(),
            prompt: prompt.into(),
            ..TurnRequest::default()
        };
        self.rig.agents.enqueue(request).await.unwrap()
    }

    async fn ended(&self, turn: &str) -> TurnInfo {
        soon("the turn to end", self.rig.agents.wait(turn)).await.unwrap()
    }

    /// Gives an agent a piece of work and waits for it. Time moves on
    /// first, so that no two turns are handed a sign-in at one instant.
    async fn does(&self, slot: &str, prompt: &str) -> TurnInfo {
        self.rig.clock.advance(Duration::from_secs(1));
        let turn = self.asks(slot, prompt).await;
        self.ended(&turn).await
    }

    /// Something a turn whose prompt has `[hold:<name>]` waits at, and
    /// what its program says it has got there through.
    fn gate(&self, name: &str) {
        for pipe in ["gate", "at"] {
            let made =
                std::process::Command::new("mkfifo").arg(self.here.join(format!("{pipe}-{name}"))).status();
            assert!(made.unwrap().success());
        }
    }

    /// Waits until a turn that is to wait at a gate is there: handed a
    /// sign-in, its program started, what it was started with written
    /// down. Says which sign-in. Until then another turn's program can be
    /// the first to write, whichever was asked first.
    async fn held(&self, turn: &str, gate: &str) -> String {
        let signin = self.handed(turn).await;
        soon("the program to reach its gate", tokio::fs::read(self.here.join(format!("at-{gate}"))))
            .await
            .unwrap();
        signin
    }

    async fn open(&self, name: &str) {
        // In a thread of its own: opening a pipe to write waits for its
        // reader for as long as it takes, and a runtime waits for a thread
        // of its own as it is put away, so a reader that never came would
        // have the test hang where it should fail.
        let (done, written) = tokio::sync::oneshot::channel();
        let gate = self.here.join(format!("gate-{name}"));
        std::thread::spawn(move || {
            let _ = done.send(std::fs::write(gate, "go\n"));
        });
        soon("the program to be waiting at its gate", written).await.unwrap().unwrap();
    }

    /// Waits until a turn has been handed a sign-in, and says which.
    async fn handed(&self, turn: &str) -> String {
        let held = self.rig.event("agent.signin.held", |event| event.data["turn"] == turn).await;
        held.data["signin"].as_str().unwrap().to_string()
    }

    /// What the stand-in wrote down each time it was started: the key, the
    /// token and the folder it was handed, its home, and its arguments.
    fn starts(&self) -> Vec<Vec<String>> {
        let written = std::fs::read_to_string(self.here.join("starts")).unwrap_or_default();
        written.lines().map(|line| line.split('|').map(str::to_string).collect()).collect()
    }

    /// The last environment the stand-in was started with.
    fn env(&self, name: &str) -> Option<String> {
        let written = std::fs::read_to_string(self.here.join("env")).unwrap_or_default();
        written.lines().find_map(|line| line.strip_prefix(&format!("{name}=")).map(str::to_string))
    }

    fn signin(&self, name: &str) -> SignIn {
        let all = self.rig.agents.signins(&ada()).unwrap();
        all.into_iter().find(|signin| signin.name == name).unwrap()
    }

    /// Says which sign-ins have reached their limit, by what they hold.
    fn limit(&self, held: &[&str]) {
        std::fs::write(self.here.join("limited"), held.join("\n") + "\n").unwrap();
    }
}

#[tokio::test]
async fn a_turn_is_handed_the_free_sign_in_that_was_used_longest_ago() {
    let pool = Pool::new().await;
    for name in ["a", "b", "c"] {
        pool.holds(name, "token", &format!("token-of-{name}"), 1).await;
    }
    pool.agent("one", &["subscription"]).await;
    pool.agent("two", &["subscription"]).await;

    for _ in 0..4 {
        assert_eq!(pool.does("one", "work").await.error, None);
    }
    let handed: Vec<String> = pool.starts().into_iter().map(|start| start[1].clone()).collect();
    assert_eq!(
        handed,
        ["token-of-a", "token-of-b", "token-of-c", "token-of-a"],
        "each in turn, and then the one that has waited longest"
    );
    assert_eq!(pool.env("STANDIN_KEY"), None, "it was handed a token and no key");
    // What is kept of each is sealed where it is kept: the program was
    // handed the value, and the file does not have it.
    let kept = std::fs::read(pool.rig.dir.join("signins").join("a")).unwrap();
    assert!(kept.starts_with(b"sealed 1\n"), "{:?}", &kept[..12.min(kept.len())]);
    assert!(!String::from_utf8_lossy(&kept).contains("token-of-a"));

    // `b` has now waited longest. While another agent holds it, the one
    // that has waited longest of those that are free is `c`.
    pool.gate("first");
    pool.rig.clock.advance(Duration::from_secs(1));
    let holding = pool.asks("two", "wait here [hold:first]").await;
    assert_eq!(pool.held(&holding, "first").await, "b");
    assert_eq!(pool.does("one", "work").await.error, None);
    assert_eq!(pool.starts().last().unwrap()[1], "token-of-c");
    pool.open("first").await;
    assert_eq!(pool.ended(&holding).await.error, None);
}

#[tokio::test]
async fn a_sign_in_is_held_by_one_turn_at_a_time_and_the_next_waits_for_it_to_be_given_back() {
    let pool = Pool::new().await;
    pool.holds("a", "token", "token-of-a", 1).await;
    pool.agent("one", &["subscription"]).await;
    pool.agent("two", &["subscription"]).await;
    pool.gate("first");

    let first = pool.asks("one", "wait here [hold:first]").await;
    assert_eq!(pool.held(&first, "first").await, "a");
    let second = pool.asks("two", "work").await;
    let waiting = pool.rig.event("agent.signin.waiting", |event| event.data["turn"] == second).await;
    assert_eq!(waiting.data["slot"], "two");
    assert!(pool.starts().len() < 2, "the second agent's program is not started without a sign-in");
    let held: Vec<String> = pool.signin("a").held_by.into_iter().map(|holder| holder.slot).collect();
    assert_eq!(held, ["one"]);

    // Given back, it is the second's: it was told, and looked for nothing.
    pool.open("first").await;
    assert_eq!(pool.ended(&first).await.error, None);
    assert_eq!(pool.ended(&second).await.error, None);
    assert_eq!(pool.starts().len(), 2);
    assert!(pool.signin("a").held_by.is_empty(), "and it is given back again");
    pool.rig.journal.flush_metrics().await.unwrap();
    let query = journal::MetricQuery { name: "agent.signin.wait".into(), ..Default::default() };
    let waits = pool.rig.journal.query_metrics(query).await.unwrap();
    assert!(!waits.is_empty(), "how long an agent waited for a sign-in is measured");
}

#[tokio::test]
async fn a_sign_in_that_says_two_may_hold_it_is_held_by_two_at_once() {
    let pool = Pool::new().await;
    pool.holds("a", "token", "token-of-a", 2).await;
    for agent in ["one", "two", "three"] {
        pool.agent(agent, &["subscription"]).await;
    }
    pool.gate("first");
    pool.gate("second");
    let first = pool.asks("one", "wait here [hold:first]").await;
    let second = pool.asks("two", "wait here [hold:second]").await;
    assert_eq!(
        (pool.held(&first, "first").await, pool.held(&second, "second").await),
        ("a".to_string(), "a".to_string())
    );
    assert_eq!(pool.signin("a").held_by.len(), 2);

    // And by no more than two.
    let third = pool.asks("three", "work").await;
    pool.rig.event("agent.signin.waiting", |event| event.data["turn"] == third).await;
    pool.open("first").await;
    pool.open("second").await;
    for turn in [first, second, third] {
        assert_eq!(pool.ended(&turn).await.error, None);
    }
}

#[tokio::test]
async fn a_key_is_used_when_no_subscription_is_free_and_the_agent_may_draw_one() {
    let pool = Pool::new().await;
    pool.holds("plan", "token", "token-of-plan", 1).await;
    pool.holds("paid", "key", "key-that-is-paid-for", 1).await;
    pool.agent("one", &["subscription", "key"]).await;
    pool.agent("two", &["subscription", "key"]).await;
    pool.gate("first");

    let first = pool.asks("one", "wait here [hold:first]").await;
    assert_eq!(pool.held(&first, "first").await, "plan", "a subscription first, as it says");
    let second = pool.asks("two", "work").await;
    assert_eq!(pool.ended(&second).await.error, None);
    let start = pool.starts().pop().unwrap();
    assert_eq!((start[0].as_str(), start[1].as_str()), ("key-that-is-paid-for", ""), "the key, and no token");
    // With a value it is given a home of its own, so that it finds no
    // sign-in of the device's where it would look for one.
    assert_eq!(start[3], pool.rig.dir.join("slots/two/home").to_string_lossy());
    pool.open("first").await;
    pool.ended(&first).await;

    // An agent that may draw only a subscription waits for one instead.
    pool.agent("three", &["plan"]).await;
    assert_eq!(pool.does("three", "work").await.error, None);
    assert_eq!(pool.starts().pop().unwrap()[1], "token-of-plan");
}

#[tokio::test]
async fn at_a_limit_the_sign_in_is_set_aside_and_the_turn_begins_again_on_the_next() {
    let pool = Pool::new().await;
    pool.holds("a", "token", "token-of-a", 1).await;
    pool.holds("b", "token", "token-of-b", 1).await;
    pool.agent("one", &["subscription"]).await;
    pool.limit(&["token-of-a"]);
    // The program says when it is good again: in two hours.
    let now = pool.rig.journal.now_ms();
    std::fs::write(pool.here.join("until"), (now / 1000 + 7200).to_string()).unwrap();

    let turn = pool.does("one", "build the cart").await;
    assert_eq!((turn.error, turn.reply.as_str()), (None, "done"), "the work was done, on the next");
    let handed: Vec<String> = pool.starts().into_iter().map(|start| start[1].clone()).collect();
    assert_eq!(handed, ["token-of-a", "token-of-b"], "it began again, from what it was asked");

    let aside = pool.rig.events("agent.signin.set_aside").await;
    assert_eq!(aside.len(), 1);
    assert_eq!(aside[0].data["signin"], "a", "said by its name");
    assert_eq!(aside[0].data["until"], json!((now / 1000 + 7200) * 1000), "until when its program said");
    let said = pool.rig.events("agent.**").await;
    assert!(
        said.iter().all(|event| !event.data.to_string().contains("token-of-")),
        "no event has the value of a sign-in"
    );
    let a = pool.signin("a");
    assert_eq!(a.aside_until, Some((now / 1000 + 7200) * 1000));
    assert!(a.held_by.is_empty(), "and it was given back");

    // The next piece of work does not try it.
    assert_eq!(pool.does("one", "more").await.error, None);
    assert_eq!(pool.starts().len(), 3);
    assert_eq!(pool.starts().pop().unwrap()[1], "token-of-b");

    // Made good again by hand, it is the one that has waited longest.
    pool.limit(&[]);
    pool.rig.agents.signin_restore("a", &ada()).await.unwrap();
    assert_eq!(pool.does("one", "more").await.error, None);
    assert_eq!(pool.starts().pop().unwrap()[1], "token-of-a");
}

#[tokio::test]
async fn a_sign_in_that_was_set_aside_is_used_again_when_its_time_has_come() {
    let pool = Pool::new().await;
    pool.holds("a", "token", "token-of-a", 1).await;
    pool.agent("one", &["subscription"]).await;
    pool.limit(&["token-of-a"]);

    let sleeps = pool.rig.clock.sleep_count();
    let turn = pool.asks("one", "build the cart").await;
    // It met the limit, there is no other, and so it waits: for the five
    // hours its package says, the program having said nothing.
    let waiting = pool.rig.event("agent.signin.waiting", |event| event.data["turn"] == turn).await;
    let until = pool.signin("a").aside_until.unwrap();
    assert_eq!(until, pool.rig.journal.now_ms() + 5 * 3600 * 1000);
    assert_eq!(waiting.data["until"], json!(until));
    assert_eq!(pool.starts().len(), 1);

    // One timer, for that moment. The limit lifts with it.
    soon("the wait for the sign-in to be good again", pool.rig.clock.sleeps_started(sleeps + 1)).await;
    pool.limit(&[]);
    pool.rig.clock.advance(Duration::from_secs(5 * 3600));
    assert_eq!(pool.ended(&turn).await.error, None);
    assert_eq!(pool.starts().len(), 2, "it began again on the same sign-in, good again");
    assert_eq!(pool.signin("a").aside_until, None);
}

#[tokio::test]
async fn a_turn_does_not_begin_again_more_often_than_there_are_sign_ins_it_could_be_handed() {
    let pool = Pool::new().await;
    pool.holds("a", "token", "token-of-a", 1).await;
    pool.holds("b", "token", "token-of-b", 1).await;
    pool.agent("one", &["subscription"]).await;
    // A package that takes everything for a limit.
    pool.limit(&["all"]);

    let sleeps = pool.rig.clock.sleep_count();
    pool.rig.clock.advance(Duration::from_secs(1));
    let turn = pool.asks("one", "build the cart").await;
    pool.rig.event("agent.signin.waiting", |event| event.data["turn"] == turn).await;
    assert_eq!(pool.starts().len(), 2, "once on each");
    soon("the wait for one to be good again", pool.rig.clock.sleeps_started(sleeps + 1)).await;
    pool.rig.clock.advance(Duration::from_secs(5 * 3600));

    let ended = pool.ended(&turn).await;
    let error = ended.error.unwrap();
    assert!(error.contains("limit reached") && error.contains("begun again 2 times"), "{error}");
    assert_eq!(pool.starts().len(), 3, "and once more when one was good again, and then no more");
}

#[tokio::test]
async fn a_turn_that_fails_or_is_cancelled_gives_its_sign_in_back() {
    let pool = Pool::new().await;
    pool.holds("a", "token", "token-of-a", 1).await;
    pool.agent("one", &["subscription"]).await;

    let failed = pool.does("one", "please crash").await;
    assert!(failed.error.unwrap().contains("boom"));
    assert!(pool.signin("a").held_by.is_empty(), "a failure that is no limit gives it back");
    assert_eq!(pool.signin("a").aside_until, None, "and sets nothing aside");

    pool.gate("first");
    let held = pool.asks("one", "wait here [hold:first]").await;
    assert_eq!(pool.held(&held, "first").await, "a");
    pool.rig.agents.cancel(&held).await;
    assert_eq!(pool.ended(&held).await.error.as_deref(), Some("the turn was cancelled"));
    assert!(pool.signin("a").held_by.is_empty());
    let released = pool.rig.events("agent.signin.released").await;
    assert_eq!(released.len(), 2, "each time it is given back is said");

    // A turn cancelled while it waits for one holds nothing to give back.
    pool.rig.agents.signin_remove("a", &ada()).await.unwrap();
    pool.holds("a", "token", "token-of-a", 1).await;
    pool.gate("second");
    pool.agent("two", &["subscription"]).await;
    let holding = pool.asks("one", "wait here [hold:second]").await;
    pool.held(&holding, "second").await;
    let waiting = pool.asks("two", "work").await;
    pool.rig.event("agent.signin.waiting", |event| event.data["turn"] == waiting).await;
    pool.rig.agents.cancel(&waiting).await;
    assert_eq!(pool.ended(&waiting).await.error.as_deref(), Some("the turn was cancelled"));
    pool.open("second").await;
    assert_eq!(pool.ended(&holding).await.error, None);
}

#[tokio::test]
async fn what_a_program_says_of_the_value_it_was_handed_is_written_down_without_it() {
    let pool = Pool::new().await;
    pool.holds("a", "token", "token-of-a", 1).await;
    pool.agent("one", &["subscription"]).await;
    // A program that repeats its token, in its answer and in an error.
    let said = pool.does("one", "repeat it").await;
    assert_eq!((said.error, said.reply.as_str()), (None, "what I hold is [the sign-in `a`]"));
    let failed = pool.does("one", "spill it").await;
    assert_eq!(failed.error.as_deref(), Some("rejected: [the sign-in `a`]"));
    assert_eq!(pool.starts().len(), 2);
    let events = pool.rig.events("**").await;
    assert!(events.len() > 8 && events.iter().all(|event| !event.data.to_string().contains("token-of-a")));
}

#[tokio::test]
async fn after_the_daemon_is_started_again_no_sign_in_is_held() {
    let mut pool = Pool::new().await;
    pool.holds("a", "token", "token-of-a", 1).await;
    pool.agent("one", &["subscription"]).await;
    pool.gate("first");
    let held = pool.asks("one", "wait here [hold:first]").await;
    assert_eq!(pool.held(&held, "first").await, "a");
    assert_eq!(pool.signin("a").held_by.len(), 1);
    // So that the turn is not simply taken up again and handed it anew.
    pool.rig.agents.deactivate("one").await.unwrap();

    pool.rig.restart().await;
    assert_eq!(pool.ended(&held).await.error.as_deref(), Some("the agent has been stopped"));
    let a = pool.signin("a");
    assert!(a.held_by.is_empty(), "a turn's program ends with the daemon that ran it");
    assert!(a.last_used.is_some(), "when it was last used is still known");
}

#[tokio::test]
async fn a_folder_is_pointed_at_where_it_is_and_nothing_is_copied() {
    let pool = Pool::new().await;
    let account = pool.rig.dir.join("account-1");
    std::fs::create_dir_all(&account).unwrap();
    std::fs::write(account.join("credentials"), "the-sign-in-its-program-keeps\n").unwrap();
    let added = pool.holds("plan", "folder", &account.to_string_lossy(), 1).await;
    let settled = account.canonicalize().unwrap();
    assert_eq!(added.folder.as_deref(), Some(&*settled.to_string_lossy()));
    pool.agent("one", &["subscription"]).await;

    assert_eq!(pool.does("one", "work").await.error, None);
    let start = pool.starts().pop().unwrap();
    assert_eq!(start[2], settled.to_string_lossy(), "its program is pointed at the folder itself");
    assert_eq!(start[3], "", "and needs no other home");
    // What its program renewed there is renewed there, and nowhere else.
    let kept = std::fs::read_to_string(account.join("credentials")).unwrap();
    assert_eq!(kept, "the-sign-in-its-program-keeps\nrenewed\n");
    let mut copies = Vec::new();
    let mut folders = vec![pool.rig.dir.clone()];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
            } else if std::fs::read(&path)
                .is_ok_and(|bytes| bytes.windows(15).any(|window| window == b"the-sign-in-its"))
            {
                copies.push(path);
            }
        }
    }
    assert_eq!(copies, [account.join("credentials")], "the sign-in is in one place");

    // One folder is one sign-in.
    let twice = NewSignIn {
        name: "again".into(),
        package: "standin".into(),
        kind: "folder".into(),
        folder: Some(account.to_string_lossy().into_owned()),
        ..NewSignIn::default()
    };
    let refused = pool.rig.agents.signin_add(twice, &ada()).await.unwrap_err();
    assert!(refused.message.contains("is already the sign-in `plan`"), "{}", refused.message);
}

#[test]
fn a_folder_a_windows_program_from_before_wrote_at_length_is_read_as_the_folder_it_is() {
    let cases: &[(&str, &str, bool, &str)] = &[
        // (what it is, what the row holds, whether on Windows, what is read)
        ("the long form a program from before wrote", r"\\?\C:\Users\ada\.tool", true, r"C:\Users\ada\.tool"),
        ("a share on another machine", r"\\?\UNC\server\share\.tool", true, r"\\server\share\.tool"),
        ("one written plainly, as it is now", r"C:\Users\ada\.tool", true, r"C:\Users\ada\.tool"),
        ("one that can only be said at length", r"\\?\C:\Users\ada\tool.", true, r"\\?\C:\Users\ada\tool."),
        ("a folder of another system", "/srv/ada/.tool", false, "/srv/ada/.tool"),
        (
            "a name that only begins as a long form does, where there is none",
            r"\\?\C:\odd",
            false,
            r"\\?\C:\odd",
        ),
    ];
    for (what, written, windows, read) in cases {
        assert_eq!(agents::signins::folder_kept(written.to_string(), *windows), *read, "{what}");
    }
    // What one folder is refused twice by: the row from before, read,
    // is the text the folder is made whole to now.
    let now = kernel::whole::plainly(r"\\?\C:\Users\ada\.tool");
    assert_eq!(agents::signins::folder_kept(r"\\?\C:\Users\ada\.tool".into(), true), now);
}

#[tokio::test]
async fn a_conversation_is_carried_on_only_under_the_sign_in_it_was_held_under() {
    let pool = Pool::new().await;
    pool.holds("a", "token", "token-of-a", 1).await;
    pool.agent("one", &["subscription"]).await;
    pool.does("one", "first").await;
    pool.does("one", "second").await;
    let carried: Vec<String> = pool.starts().into_iter().map(|start| start[4].clone()).collect();
    assert_eq!(carried, ["run", "run --carry-on s-1"], "under the same sign-in it is carried on");

    // Another sign-in has waited longest. The conversation is not its.
    pool.holds("b", "token", "token-of-b", 1).await;
    pool.does("one", "third").await;
    let start = pool.starts().pop().unwrap();
    assert_eq!((start[1].as_str(), start[4].as_str()), ("token-of-b", "run"));
}

#[tokio::test]
async fn an_agent_that_draws_nothing_is_started_as_it_was_and_told_where_it_runs() {
    let pool = Pool::new().await;
    pool.holds("a", "token", "token-of-a", 1).await;
    pool.agent("one", &[]).await;
    assert_eq!(pool.does("one", "work").await.error, None);
    let start = pool.starts().pop().unwrap();
    assert_eq!(start[..4].join(""), "", "it is handed nothing the factory holds");
    assert_eq!(pool.env("HOME"), std::env::var("HOME").ok(), "and signs in as whoever runs the daemon");
    assert!(pool.signin("a").last_used.is_none());
    // What an agent is deliberately given is still there, and what
    // everything the daemon starts is told.
    assert_eq!(pool.env(&brand::env_var("URL")).as_deref(), Some("http://127.0.0.1:7700"));
    assert_eq!(pool.env(&brand::env_var("SLOT")).as_deref(), Some("one"));
    let token = pool.rig.dir.join("slots/one/token");
    assert_eq!(pool.env(&brand::env_var("TOKEN_FILE")), Some(token.to_string_lossy().into_owned()));
    assert_eq!(pool.env("FACTORY_URL").as_deref(), Some("http://127.0.0.1:7700"));
    assert!(pool.env("FACTORY_CLI").is_some());
}

#[tokio::test]
async fn an_agent_whose_package_does_not_say_how_it_is_handed_a_sign_in_is_refused_one() {
    // A package that says nothing of signing in.
    let pool = Pool::with("").await;
    let new = |kind: &str, package: &str| NewSignIn {
        name: "a".into(),
        package: package.into(),
        kind: kind.into(),
        value: Some("token-of-a".into()),
        ..NewSignIn::default()
    };
    let refused = pool.rig.agents.signin_add(new("token", "standin"), &ada()).await.unwrap_err();
    assert!(
        refused.message.contains("does not say how its program is handed a sign-in"),
        "{}",
        refused.message
    );
    let draws = |adapter: &str, draw: &str| SlotSpec {
        name: "one".into(),
        adapter: Some(adapter.into()),
        signins: Some(vec![draw.into()]),
        ..SlotSpec::default()
    };
    let refused = pool.rig.agents.ensure(draws("standin", "subscription"), &ada()).await.unwrap_err();
    assert!(
        refused.message.contains("does not say how its program is handed a sign-in")
            && refused.message.contains("`signins: []`"),
        "{}",
        refused.message
    );
    // Nor is anything that is not a package at all.
    let refused = pool.rig.agents.ensure(draws("fake", "key"), &ada()).await.unwrap_err();
    assert!(refused.message.contains("no such package called `fake`"), "{}", refused.message);
    assert!(
        pool.rig.agents.slots().await.unwrap().is_empty(),
        "no agent was made that would sign in otherwise"
    );

    // One that can be handed a key and nothing else is handed nothing else.
    let pool = Pool::with("  signin:\n    key: STANDIN_KEY\n").await;
    let refused = pool.rig.agents.signin_add(new("token", "standin"), &ada()).await.unwrap_err();
    assert!(refused.message.contains("handed a token; it can be handed: key"), "{}", refused.message);
    let refused = pool.rig.agents.ensure(draws("standin", "subscription"), &ada()).await.unwrap_err();
    assert!(refused.message.contains("cannot draw `subscription`"), "{}", refused.message);
    pool.rig.agents.ensure(draws("standin", "key"), &ada()).await.unwrap();

    // With no sign-in to be had, the work fails and says what to do.
    let failed = pool.does("one", "work").await.error.unwrap();
    assert!(
        failed.contains("the factory holds none of those for `standin`") && failed.contains("signin add"),
        "{failed}"
    );
    assert!(pool.starts().is_empty(), "its program was not started to sign in some other way");
}

#[tokio::test]
async fn only_the_owner_manages_sign_ins_and_telling_an_agent_to_draw_one_takes_leave_to_use_secrets() {
    let pool = Pool::new().await;
    pool.holds("a", "key", "key-that-is-paid-for", 1).await;
    let member = |scopes: &[&str]| Principal {
        kind: PrincipalKind::User,
        id: "bo".into(),
        name: "Bo".into(),
        scopes: scopes.iter().map(|scope| scope.to_string()).collect(),
        key: None,
    };
    // Everything a person is given by default, which is not the owner's.
    let bo = member(kernel::principal::MEMBER_SCOPES);
    let new = NewSignIn {
        name: "b".into(),
        package: "standin".into(),
        kind: "key".into(),
        value: Some("another".into()),
        ..NewSignIn::default()
    };
    for refused in [
        pool.rig.agents.signin_add(new, &bo).await.err(),
        pool.rig.agents.signins(&bo).err(),
        pool.rig.agents.signin_remove("a", &bo).await.err(),
        pool.rig.agents.signin_restore("a", &bo).await.err(),
    ] {
        assert!(refused.unwrap().message.contains("needs `admin`"));
    }
    assert_eq!(pool.rig.agents.signins(&ada()).unwrap().len(), 1);

    // A person may tell an agent to draw; an agent, and whoever may bring
    // agents in and no more, may not.
    let draws = |name: &str| SlotSpec {
        name: name.into(),
        adapter: Some("standin".into()),
        signins: Some(vec!["key".into()]),
        ..SlotSpec::default()
    };
    pool.rig.agents.ensure(draws("one"), &bo).await.unwrap();
    let narrow = member(&["agents", "projects", "channels", "board", "blueprints", "fleet:read", "search"]);
    let refused = pool.rig.agents.ensure(draws("two"), &narrow).await.unwrap_err();
    assert!(refused.message.contains("needs `secrets:use`"), "{}", refused.message);
    assert!(
        !kernel::principal::AGENT_SCOPES
            .iter()
            .any(|scope| ["secrets", "secrets:use", "admin"].contains(scope))
    );

    // What a sign-in is called, and what it is.
    for (name, kind, value) in
        [("subscription", "key", "v"), ("Work One", "key", "v"), ("c", "password", "v"), ("c", "key", " ")]
    {
        let wrong = NewSignIn {
            name: name.into(),
            package: "standin".into(),
            kind: kind.into(),
            value: Some(value.into()),
            ..NewSignIn::default()
        };
        assert!(pool.rig.agents.signin_add(wrong, &ada()).await.is_err(), "{name} {kind}");
    }
}

#[tokio::test]
async fn a_blueprint_says_what_an_agent_draws_its_sign_in_from() {
    let pool = Pool::new().await;
    pool.rig.agent("seed", "fake").await;
    pool.holds("plan", "token", "token-of-plan", 1).await;
    pool.holds("paid", "key", "key-that-is-paid-for", 1).await;
    let fleet = r#"
blueprint: fleet
permissions: [agents, secrets:use]
steps:
  builder:
    do: agent.ensure
    with: { name: builder, adapter: standin, signins: [subscription, paid] }
  work:
    do: agent.task
    with: { slot: builder, prompt: "build the cart" }
"#;
    let run = pool.rig.run(fleet, json!({})).await;
    assert_eq!(run.status, conductor::RunStatus::Ok, "{:?}", run.error);
    let slot = pool.rig.agents.slot("builder").await.unwrap();
    assert_eq!(slot.signins, ["subscription", "paid"]);
    assert_eq!(pool.starts().pop().unwrap()[1], "token-of-plan");
}

#[test]
fn a_manifest_that_says_how_its_program_signs_in_is_checked_like_the_rest_of_it() {
    let wrong = MANIFEST.replace("PROGRAM", "standin").replace(
        "SIGNIN",
        &format!(
            "  signin:\n    key: {own}\n    token: not a variable\n    sideways: X\n    without: [1]\n    limit:\n      when: ${{ matches(output, \"limit\") }}\n",
            own = brand::env_var("TOKEN")
        ),
    );
    let Err(problems) = Extension::load(&wrong) else { panic!("this must not load") };
    let said = problems.join("\n");
    for expected in [
        "`agent.signin.sideways` is not a known field",
        "`agent.signin.key` must be the name of a variable",
        "`agent.signin.token` must be the name of a variable",
        "`agent.signin.without` lists 1",
        "`agent.signin.limit` needs `when`",
    ] {
        assert!(said.contains(expected), "missing `{expected}` in:\n{said}");
    }
    let reads = MANIFEST.replace("PROGRAM", "standin").replace(
        "SIGNIN",
        "  signin:\n    key: STANDIN_KEY\n    limit:\n      when: ${ matches(output, \"limit\") }\n      for: 1h\n",
    );
    let Err(problems) = Extension::load(&reads) else { panic!("this must not load") };
    assert!(problems.join("\n").contains("reads `output`, which is not there to read"), "{problems:?}");
}

// ---- the rule ---------------------------------------------------------------------------

/// A sign-in as the rule is shown it.
fn offered(name: &str, choice: usize, free: bool, last_used: i64) -> Offered {
    Offered {
        name: name.into(),
        kind: if choice == 0 { "token" } else { "key" },
        choice,
        at_once: 1,
        held: usize::from(!free),
        free,
        last_used,
        aside_until: None,
    }
}

fn asked(signins: Vec<Offered>) -> Asked {
    Asked {
        slot: "one".into(),
        package: "standin".into(),
        turn: "turn_1".into(),
        conversation: None,
        now: 1_000,
        signins,
    }
}

#[test]
fn the_rule_that_ships_takes_the_one_used_longest_ago_of_the_first_choice_with_any_free_and_else_waits() {
    let rule = conductor::standard().load(RULE, None).expect("the rule ships with the product");
    let says = |signins: Vec<Offered>| ask(&rule, &asked(signins)).unwrap();
    let take = |name: &str| Decision::Take(name.to_string());

    let pool = vec![offered("a", 0, true, 30), offered("b", 0, true, 10), offered("c", 0, true, 20)];
    assert_eq!(says(pool), take("b"), "the one handed out longest ago");
    let pool = vec![offered("a", 0, true, 30), offered("b", 0, false, 10), offered("c", 0, true, 20)];
    assert_eq!(says(pool), take("c"), "of those that are free");
    let pool = vec![offered("a", 0, true, 30), offered("new", 0, true, 0)];
    assert_eq!(says(pool), take("new"), "one never used has waited longest");
    // A key is a later choice: taken only when no subscription is free,
    // however long ago it was used.
    let pool = vec![offered("paid", 1, true, 1), offered("a", 0, true, 30)];
    assert_eq!(says(pool), take("a"));
    let pool = vec![offered("paid", 1, true, 1), offered("a", 0, false, 30)];
    assert_eq!(says(pool), take("paid"));
    let pool = vec![offered("paid", 1, false, 1), offered("a", 0, false, 30)];
    assert_eq!(says(pool), Decision::Wait, "with nothing free it waits");
}

#[test]
fn those_that_are_free_are_handed_the_rule_in_turn_by_choice_and_then_by_how_long_ago_each_was_used() {
    let names = |signins: Vec<Offered>| -> Vec<String> {
        in_turn(&signins).into_iter().map(|one| one.name).collect()
    };
    let cases: Vec<(Vec<Offered>, &[&str])> = vec![
        // None to draw, and none of them free.
        (vec![], &[]),
        (vec![offered("a", 0, false, 30), offered("b", 1, false, 10)], &[]),
        // Of one choice, whichever was handed out longest ago; one never used has waited longest.
        (
            vec![offered("a", 0, true, 30), offered("b", 0, true, 10), offered("c", 0, true, 20)],
            &["b", "c", "a"],
        ),
        (vec![offered("a", 0, true, 30), offered("new", 0, true, 0)], &["new", "a"]),
        // Those that are held or set aside are not among them.
        (vec![offered("a", 0, true, 30), offered("b", 0, false, 10), offered("c", 0, true, 20)], &["c", "a"]),
        // An earlier choice before a later, however long ago the later was used.
        (vec![offered("paid", 1, true, 1), offered("a", 0, true, 30)], &["a", "paid"]),
        (vec![offered("paid", 1, true, 1), offered("a", 0, false, 30)], &["paid"]),
        (
            vec![
                offered("k2", 2, true, 5),
                offered("k1", 1, true, 9),
                offered("s2", 0, true, 7),
                offered("s1", 0, true, 3),
            ],
            &["s1", "s2", "k1", "k2"],
        ),
        // Equals stay as they were offered.
        (
            vec![offered("x", 0, true, 5), offered("y", 0, true, 5), offered("z", 0, true, 5)],
            &["x", "y", "z"],
        ),
    ];
    for (signins, comes_to) in cases {
        assert_eq!(names(signins.clone()), *comes_to, "{signins:?}");
    }
    // Each is handed whole, as the rule is shown it among all of them.
    let whole = in_turn(&[offered("a", 0, true, 30)]);
    assert_eq!(whole, [offered("a", 0, true, 30)]);
}

#[test]
fn a_rule_is_handed_those_in_turn_whatever_it_says_of_its_own_params_and_may_go_by_them_or_not() {
    let pool = vec![offered("a", 0, true, 30), offered("b", 0, true, 10), offered("held", 0, false, 1)];
    // One that goes by them: the second in turn.
    let second = "blueprint: std/signin\npermissions: []\nparams:\n  in_turn: { type: list, default: [] }\noutputs:\n  \
                  take: ${ params.in_turn[1].name }\n";
    assert_eq!(ask(second, &asked(pool.clone())).unwrap(), Decision::Take("a".to_string()));
    // One from before, which works it out for itself, says what it always did.
    let before = "blueprint: std/signin\npermissions: []\nparams:\n  signins: { type: list, default: [] }\noutputs:\n  \
                  take: ${ sort(sort(where(params.signins, \"free\", true), \"last_used\"), \"choice\")[0].name }\n  wait: true\n";
    assert_eq!(ask(before, &asked(pool)).unwrap(), Decision::Take("b".to_string()));
    assert_eq!(ask(before, &asked(vec![offered("held", 0, false, 1)])).unwrap(), Decision::Wait);
}

#[tokio::test]
async fn the_rule_is_a_blueprint_and_the_one_put_in_its_place_is_what_shares_sign_ins_out() {
    let pool = Pool::new().await;
    pool.holds("a", "token", "token-of-a", 1).await;
    pool.holds("b", "token", "token-of-b", 1).await;
    pool.agent("one", &["subscription"]).await;
    pool.agent("two", &["subscription"]).await;

    // A rule of a factory's own: stay with the sign-in the conversation is
    // under while it is free; never wait; and say why not, in its words.
    pool.rig.library.put(
        RULE,
        r#"
blueprint: std/signin
permissions: []
params:
  conversation: { type: any, default: null }
  signins: { type: list, default: [] }
  slot: { type: string, default: "" }
  patience: { type: bool, default: false }
outputs:
  take: ${ where(where(params.signins, "free", true), "name", params.conversation)[0].name ?? sort(where(params.signins, "free", true), "last_used")[0].name }
  wait: ${ params.patience }
  says: "every sign-in ${params.slot} may draw is in use, and this factory does not wait"
"#,
    );
    for _ in 0..3 {
        assert_eq!(pool.does("one", "work").await.error, None);
    }
    let starts: Vec<(String, String)> =
        pool.starts().into_iter().map(|start| (start[1].clone(), start[4].clone())).collect();
    let carried = ("token-of-a".to_string(), "run --carry-on s-1".to_string());
    assert_eq!(
        starts,
        [("token-of-a".to_string(), "run".to_string()), carried.clone(), carried],
        "it stays where its conversation is, which the rule that ships would not"
    );

    // With both held, the next fails at once, as this rule says.
    pool.gate("first");
    pool.gate("second");
    pool.rig.clock.advance(Duration::from_secs(1));
    let first = pool.asks("one", "wait here [hold:first]").await;
    pool.held(&first, "first").await;
    let second = pool.asks("two", "wait here [hold:second]").await;
    assert_eq!(pool.held(&second, "second").await, "b");
    pool.agent("three", &["subscription"]).await;
    let refused = pool.does("three", "work").await.error.unwrap();
    assert_eq!(refused, "every sign-in three may draw is in use, and this factory does not wait");
    pool.open("first").await;
    pool.open("second").await;
    pool.ended(&first).await;
    pool.ended(&second).await;
}

#[tokio::test]
async fn a_rule_that_does_not_check_does_anything_or_chooses_what_is_not_free_fails_the_work_and_says_so() {
    let pool = Pool::new().await;
    pool.holds("a", "token", "token-of-a", 1).await;
    pool.agent("one", &["subscription"]).await;
    let rule = |outputs: &str| format!("blueprint: std/signin\npermissions: []\n{outputs}");
    for (text, says) in [
        (
            rule("outputs:\n  take: nonesuch\n"),
            "chose `nonesuch` for `one`, which is not one of the sign-ins it was told are free",
        ),
        (
            rule("outputs:\n  take: ${ nonsense( }\n"),
            "the rule for choosing a sign-in, `std/signin`, does not check",
        ),
        (rule("outputs:\n  take: ${ 1 + 1 }\n"), "gave `take` as 2, which is not the name of a sign-in"),
        (
            rule("steps:\n  loud: { do: noop }\noutputs:\n  take: a\n"),
            "It is asked before every turn and never run",
        ),
        (
            rule("outputs:\n  take: null\n"),
            "says that `one` is to be handed no sign-in, and is not to wait for one",
        ),
    ] {
        pool.rig.library.put(RULE, &text);
        let failed = pool.does("one", "work").await.error.unwrap();
        assert!(failed.contains(says), "{failed}");
    }
    assert!(pool.starts().is_empty(), "and its program is not started to sign in some other way");
    assert!(pool.signin("a").held_by.is_empty());
}

/// What a drain says becomes of a turn in hand that `outlives` the daemon, under such a `signin`.
async fn becomes_at_a_drain(signin: &str) -> String {
    let pool = Pool::with(signin).await;
    pool.holds("k", "key", "key-of-k", 1).await;
    pool.agent("one", &["key"]).await;
    let spec = SlotSpec { name: "one".into(), outlives: Some(true), ..SlotSpec::default() };
    pool.rig.agents.ensure(spec, &ada()).await.unwrap();
    pool.gate("drained");
    let turn = pool.asks("one", "wait here [hold:drained]").await;
    pool.held(&turn, "drained").await;
    let in_hand = pool.rig.agents.in_hand_now().await.unwrap();
    pool.open("drained").await;
    pool.ended(&turn).await;
    assert_eq!(in_hand.len(), 1, "{in_hand:?}");
    in_hand[0].becomes.clone()
}

#[tokio::test]
async fn a_drain_says_a_turn_handed_what_only_the_daemon_can_keep_out_of_its_words_ends_with_the_daemon() {
    // Handed in a variable, it is kept out of its words by its wrapper: it goes on, and is taken up.
    let kept = becomes_at_a_drain("  signin:\n    key: STANDIN_KEY\n").await;
    assert_eq!(kept, "its program goes on, and the next start takes it up where it is");
    // Its key in no variable (the package says its home where it said its
    // key), its program is on pipes and ends with the daemon, whatever its
    // slot says; and the drain says so, and what then becomes of the turn.
    let piped = becomes_at_a_drain("  signin:\n    key: STANDIN_KEY\n    home: STANDIN_KEY\n").await;
    assert_eq!(
        piped,
        "its program ends with the daemon though its agent outlives it, since it was handed something only the \
         daemon can keep out of what it says; it is begun again by the next start"
    );
}
