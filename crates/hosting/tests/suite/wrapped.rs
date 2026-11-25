//! A factory inside a program that is not its own, which did not say so.
//!
//! What wraps a program the factory starts is the factory's own program,
//! started again under a word of its own. A test's program does not know
//! the word: started so, it would end at once having started nothing, and
//! whatever waited for the program would wait until its time ran out. So
//! a factory that was not told it is inside another program starts
//! nothing, and says at once what it is to be told.

use conductor::RunStatus;
use hosting::kept::Files;
use serde_json::json;

use crate::{Rig, step};

const SAYS: &str = "`Hosting::stands_between()`, or `stands_between: true` in a node's `Config`";

#[tokio::test]
async fn a_factory_inside_another_program_that_did_not_say_so_starts_no_command_and_is_told_what_to_say() {
    let rig = Rig::that_said_nothing().await;
    let run = rig
        .run(
            "blueprint: said\npermissions: [shell]\nsteps:\n  hello: { do: shell, with: { cmd: [echo, hello] } }\n",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Failed);
    let why = step(&run, "hello").error.clone().unwrap_or_default();
    assert!(why.contains("is inside a program that is not its own") && why.contains(SAYS), "{why}");
    assert!(why.contains("nothing was started"), "{why}");
}

#[tokio::test]
async fn it_starts_no_service_either_and_the_step_that_asked_fails_at_once_saying_the_same() {
    let rig = Rig::that_said_nothing().await;
    let run = rig
        .run(
            "blueprint: served\npermissions: [fleet]\nsteps:\n  start: { do: service.start, with: { name: web, cmd: [sleep, 300] } }\n",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Failed);
    let why = step(&run, "start").error.clone().unwrap_or_default();
    assert!(why.contains(SAYS), "{why}");
    assert!(rig.hosting.services().iter().all(|service| service.pid == 0), "something was started");
}

#[tokio::test]
async fn nor_a_program_that_is_to_go_on_without_the_daemon() {
    let rig = Rig::that_said_nothing().await;
    let files: Files = rig.hosting.kept("turn");
    let refused = hosting::kept::wrapper(&files, &[], &["sleep".into(), "300".into()]).err();
    let why = refused.expect("a wrapper was made of the test's own program").to_string();
    assert!(why.contains(SAYS), "{why}");
}
