//! An agent whose program goes on without the daemon, and what becomes of
//! a turn that a daemon is started and finds in hand.
//!
//! The agent here is a program that says it was asked, waits to be let go,
//! and answers. The daemon is stopped while it waits, as a crash would stop
//! it, and started again.

use std::path::{Path, PathBuf};

use agents::{SlotSpec, TurnInfo, TurnRequest};
use serde_json::json;

use crate::{Rig, ada, gone, soon};

/// A rig whose agent `builder` runs a program that says it was asked and
/// waits at a hold, and the hold: one asking is let go at a time, whether
/// it waits already or has yet to come.
async fn rig(outlives: bool) -> (Rig, PathBuf) {
    let rig = Rig::new().await;
    let file = |name: &str| rig.dir.join(name).to_string_lossy().into_owned();
    let (asked, hold, badly) = (file("asked"), file("hold"), file("badly"));
    // Where the test has said so (a file called `badly`), it complains
    // and ends badly once it is let go; otherwise it answers.
    let waits = ["swallow", "append", &asked, "asked", "held", &hold, "builder"];
    let fails = ["there", &badly, "(", "err", "I could not find my settings.", "exit", "3", ")"];
    let does = crate::standin::argv(&[&waits[..], &fails[..], &["say", "I did it."][..]].concat());
    rig.fake.programs.lock().unwrap().push(("waits".into(), does));
    rig.agents.add_adapter(
        "waits",
        std::sync::Arc::new(agents::adapter::Command {
            argv: rig.fake.programs.lock().unwrap()[0].1.clone(),
        }),
    );
    let hold = rig.dir.join("hold");
    crate::standin::hold(&hold);
    rig.agent("builder", "waits").await;
    let spec = SlotSpec { name: "builder".into(), outlives: Some(outlives), ..SlotSpec::default() };
    rig.agents.ensure(spec, &ada()).await.unwrap();
    (rig, hold)
}

fn asked(rig: &Rig) -> usize {
    std::fs::read_to_string(rig.dir.join("asked")).unwrap_or_default().lines().count()
}

fn let_go(hold: &Path) {
    crate::standin::pass(hold, 1);
}

/// Asks `builder` for something, and waits until its program has it.
async fn in_hand(rig: &Rig, request: TurnRequest) -> String {
    let request =
        TurnRequest { slot: "builder".into(), kind: "task".into(), thread: "work".into(), ..request };
    let id = rig.agents.enqueue(TurnRequest { prompt: "Add a cart".into(), ..request }).await.unwrap();
    // Until it has written its line, and not only made the file: a program
    // stopped between the two was asked, and left nothing to count.
    let asked = rig.dir.join("asked");
    until(rig, &asked, |file| std::fs::read_to_string(file).is_ok_and(|said| said.ends_with('\n'))).await;
    id
}

/// Waits for `so` of a file, told by the system when the file changes.
/// One that is given up on says what the file held and what had become of
/// the agent's turns: the wait alone says only that something did not
/// come, and it has been seen to give up once in some two thousand runs
/// of this suite on a crowded machine, with nothing to say which.
async fn until(rig: &Rig, file: &Path, so: impl Fn(&Path) -> bool) {
    let mut changes = rig.hosting.watch().files(&[file]).or_looking();
    let waited = async {
        while !so(file) {
            changes.next().await;
        }
    };
    let was = async {
        let held = std::fs::read_to_string(file).unwrap_or_default();
        let turns = rig.agents.turns("builder", 10).await.unwrap_or_default();
        let turns: Vec<String> =
            turns.iter().map(|turn| format!("{} {:?} {:?}", turn.status, turn.error, turn.reply)).collect();
        let told = rig.hosting.watch().problem();
        format!("it held {held:?}; the agent's turns, the newest first: {turns:?}; told of files: {told:?}")
    };
    crate::soon_or(&format!("{} to be as expected", file.display()), waited, was).await;
}

fn files(rig: &Rig, turn: &str, what: &str) -> PathBuf {
    rig.dir.join("run").join(format!("turn.{turn}.{what}"))
}

async fn ended(rig: &Rig, id: &str) -> TurnInfo {
    soon("the turn to end", rig.agents.wait(id)).await.unwrap()
}

/// What `agent.turn.started` said each time, of `again` and `taken_up`.
async fn started(rig: &Rig) -> Vec<(serde_json::Value, serde_json::Value)> {
    let events = rig.events("agent.turn.started").await;
    events.iter().map(|event| (event.data["again"].clone(), event.data["taken_up"].clone())).collect()
}

#[tokio::test]
async fn a_program_that_outlives_the_daemon_is_taken_up_where_it_is_and_its_agent_is_asked_once() {
    let (mut rig, hold) = rig(true).await;
    let id = in_hand(&rig, TurnRequest::default()).await;

    rig.restart().await;
    // It is in hand still, and nobody has asked it anything again.
    rig.event("agent.turn.started", |event| event.data["again"] == 1).await;
    assert_eq!(rig.agents.turn(&id).await.unwrap().status, "running");
    let_go(&hold);

    let turn = ended(&rig, &id).await;
    assert_eq!(
        (turn.status.as_str(), turn.reply.as_str(), turn.decision.as_deref()),
        ("done", "I did it.", None)
    );
    assert_eq!(asked(&rig), 1, "asked once, by the daemon that is gone");
    assert_eq!(started(&rig).await, [(json!(null), json!(null)), (json!(1), json!(true))]);
    assert_eq!((turn.restarts, turn.outlives), (1, true));
    // And what it wrote for a later daemon to find is taken away.
    assert!(!files(&rig, &id, "out").exists() && !files(&rig, &id, "pid").exists());
}

#[tokio::test]
async fn one_that_finished_while_no_daemon_was_there_has_its_answer_when_one_is() {
    let (mut rig, hold) = rig(true).await;
    let id = in_hand(&rig, TurnRequest::default()).await;

    rig.stop();
    let_go(&hold);
    let exit = files(&rig, &id, "exit");
    until(&rig, &exit, |exit| exit.exists()).await;
    rig.start().await;

    let turn = ended(&rig, &id).await;
    assert_eq!((turn.status.as_str(), turn.reply.as_str()), ("done", "I did it."));
    assert_eq!(asked(&rig), 1);
}

#[tokio::test]
async fn one_that_ended_badly_by_itself_while_no_daemon_was_there_fails_with_what_it_said() {
    let (mut rig, hold) = rig(true).await;
    let id = in_hand(&rig, TurnRequest::default()).await;

    rig.stop();
    std::fs::write(rig.dir.join("badly"), "").unwrap();
    let_go(&hold);
    let exit = files(&rig, &id, "exit");
    until(&rig, &exit, |exit| exit.exists()).await;
    rig.start().await;

    // It said how it ended, so it was not cut off: its turn ends as it
    // would have with the daemon there, and is not begun again.
    let turn = ended(&rig, &id).await;
    let why = turn.error.unwrap();
    assert!(why.contains("I could not find my settings."), "{why}");
    assert_eq!((turn.status.as_str(), asked(&rig)), ("failed", 1));
}

/// Kills the whole of a program that went on by itself, wrapper and all,
/// as the machine being started again would: nothing is left to say how
/// it ended.
///
/// Unix's, with the two tests that ask it: that a program was ended so
/// is known by the signal it was ended by, and where a group is ended by
/// letting go of it (Windows) one that was ended reads as one that
/// ended well.
#[cfg(unix)]
fn kill_outright(rig: &Rig, id: &str) {
    let pid = std::fs::read_to_string(files(rig, id, "pid")).unwrap();
    hosting::group::stop(pid.trim().parse().unwrap(), true).unwrap();
    // Until it is gone, and not only told to go: a daemon that looks
    // while it is going still finds it there.
    while crate::alive(&pid) {
        std::thread::yield_now();
    }
}

#[cfg(unix)]
#[tokio::test]
async fn one_that_was_killed_outright_while_no_daemon_was_there_was_cut_off_after_all_and_is_begun_again() {
    let (mut rig, hold) = rig(true).await;
    let id = in_hand(&rig, TurnRequest::default()).await;

    rig.stop();
    kill_outright(&rig, &id);
    rig.start().await;

    // There is nothing to take up, so it is what a program that ends with
    // the daemon is: begun again, unless it was asked for to be left.
    let_go(&hold);
    let turn = ended(&rig, &id).await;
    assert_eq!((turn.status.as_str(), turn.reply.as_str(), turn.restarts), ("done", "I did it.", 1));
    assert_eq!(asked(&rig), 2);
    assert_eq!(started(&rig).await.last().unwrap(), &(json!(1), json!(false)));
}

#[cfg(unix)]
#[tokio::test]
async fn one_that_was_killed_outright_and_was_asked_for_to_be_left_is_left() {
    let (mut rig, _hold) = rig(true).await;
    let id = in_hand(&rig, TurnRequest { again: Some(false), ..TurnRequest::default() }).await;

    rig.stop();
    kill_outright(&rig, &id);
    rig.start().await;

    let turn = ended(&rig, &id).await;
    let why = turn.error.unwrap();
    assert!(why.contains("it was asked for to be left as it was then and not begun again"), "{why}");
    assert_eq!(asked(&rig), 1);
}

#[tokio::test]
async fn a_program_that_does_not_outlive_the_daemon_is_begun_again_by_the_next_one() {
    let (mut rig, hold) = rig(false).await;
    let id = in_hand(&rig, TurnRequest::default()).await;

    rig.restart().await;
    let asked_again = rig.dir.join("asked");
    until(&rig, &asked_again, |_| asked(&rig) == 2).await;
    let_go(&hold);

    let turn = ended(&rig, &id).await;
    assert_eq!((turn.status.as_str(), turn.reply.as_str()), ("done", "I did it."));
    assert_eq!(started(&rig).await, [(json!(null), json!(null)), (json!(1), json!(false))]);
}

#[tokio::test]
async fn one_asked_for_to_be_left_is_not_begun_again_and_says_so() {
    let (mut rig, _hold) = rig(false).await;
    let id = in_hand(&rig, TurnRequest { again: Some(false), ..TurnRequest::default() }).await;

    rig.restart().await;

    let turn = ended(&rig, &id).await;
    assert_eq!(turn.status, "failed");
    assert_eq!(
        turn.error.unwrap(),
        "the daemon was stopped while builder had this in hand, and it was asked for to be left as it was then \
         and not begun again. Ask again to have it done"
    );
    assert_eq!(asked(&rig), 1);
}

#[tokio::test]
async fn a_turn_is_begun_again_only_so_many_times_whatever_was_asked() {
    let (mut rig, _hold) = rig(false).await;
    let id = in_hand(&rig, TurnRequest { most: Some(1), ..TurnRequest::default() }).await;

    // Once it is begun again.
    rig.restart().await;
    let asked_again = rig.dir.join("asked");
    until(&rig, &asked_again, |_| asked(&rig) == 2).await;
    // The second time it is left for a person.
    rig.restart().await;

    let turn = ended(&rig, &id).await;
    assert_eq!(turn.status, "failed");
    assert_eq!(
        turn.error.unwrap(),
        "the daemon was stopped while builder had this in hand, 2 times now, and it is begun again after 1 such \
         stops at most. It is left as it was for a person: ask again to have it done"
    );
    assert_eq!(asked(&rig), 2);
}

#[tokio::test]
async fn one_that_outlives_the_daemon_and_is_left_all_the_same_is_stopped_where_it_is() {
    // The number holds for a program that goes on by itself too: left, it
    // is not left at work.
    let (mut rig, hold) = rig(true).await;
    let id = in_hand(&rig, TurnRequest { most: Some(0), ..TurnRequest::default() }).await;
    let pid = std::fs::read_to_string(files(&rig, &id, "pid")).unwrap();

    rig.restart().await;

    let turn = ended(&rig, &id).await;
    assert_eq!(turn.status, "failed");
    assert!(turn.error.unwrap().contains("It is left as it was for a person"));
    gone("its program to be stopped", &[&pid]).await;
    // Let go now, nobody is there to answer.
    let_go(&hold);
    assert_eq!(asked(&rig), 1);
}

/// The pid its program's files say, of the one turn in hand.
fn pid_of(rig: &Rig, id: &str) -> String {
    std::fs::read_to_string(files(rig, id, "pid")).unwrap()
}

/// `builder` said to go on without the daemon, or not, from now.
async fn outliving(rig: &Rig, outlives: bool) {
    let spec = SlotSpec { name: "builder".into(), outlives: Some(outlives), ..SlotSpec::default() };
    rig.agents.ensure(spec, &ada()).await.unwrap();
}

// Whether a program goes on without the daemon is asked of its slot as
// the daemon stops, so a slot changed while its turn is in hand holds for
// that turn: every turn's program is started by its wrapper.

#[tokio::test]
async fn a_slot_turned_to_outlive_the_daemon_while_its_turn_is_in_hand_has_that_turn_taken_up_and_asked_once()
{
    let (mut rig, hold) = rig(false).await;
    let id = in_hand(&rig, TurnRequest::default()).await;
    let pid = pid_of(&rig, &id);
    outliving(&rig, true).await;

    rig.restart().await;
    rig.event("agent.turn.started", |event| event.data["again"] == 1).await;
    assert!(crate::alive(&pid), "its program went on");
    let_go(&hold);

    let turn = ended(&rig, &id).await;
    assert_eq!((turn.status.as_str(), turn.reply.as_str()), ("done", "I did it."));
    assert_eq!(asked(&rig), 1, "asked once, by the daemon that is gone");
    assert_eq!(started(&rig).await, [(json!(null), json!(null)), (json!(1), json!(true))]);
}

#[tokio::test]
async fn a_slot_turned_not_to_outlive_the_daemon_while_its_turn_is_in_hand_has_its_program_ended_and_the_turn_begun_again()
 {
    let (mut rig, hold) = rig(true).await;
    let id = in_hand(&rig, TurnRequest::default()).await;
    let pid = pid_of(&rig, &id);
    outliving(&rig, false).await;

    rig.restart().await;
    gone("its program to be ended as the daemon stopped", &[&pid]).await;
    until(&rig, &rig.dir.join("asked"), |_| asked(&rig) == 2).await;
    let_go(&hold);

    let turn = ended(&rig, &id).await;
    assert_eq!((turn.status.as_str(), turn.reply.as_str()), ("done", "I did it."));
    assert_eq!(started(&rig).await.last().unwrap(), &(json!(1), json!(false)));
}

#[tokio::test]
async fn a_slot_turned_not_to_outlive_the_daemon_has_its_turn_left_where_whoever_asked_said_so() {
    let (mut rig, _hold) = rig(true).await;
    let id = in_hand(&rig, TurnRequest { again: Some(false), ..TurnRequest::default() }).await;
    let pid = pid_of(&rig, &id);
    outliving(&rig, false).await;

    rig.restart().await;
    let turn = ended(&rig, &id).await;
    let why = turn.error.unwrap();
    assert!(why.contains("it was asked for to be left as it was then and not begun again"), "{why}");
    gone("its program to be ended as the daemon stopped", &[&pid]).await;
    assert_eq!(asked(&rig), 1);
}

// What a step that was waiting for the agent is given.

/// A step that asks `builder` for work, in a blueprint that is kept step by
/// step or keeps nothing, as `more` says.
fn asks(more: &str, with: &str) -> String {
    format!(
        "blueprint: asks\npermissions: [agents]\n{more}steps:\n  work:\n    do: agent.task\n    timeout: 5m\n    \
         with: {{ slot: builder, thread: work, prompt: Add a cart{with} }}\n"
    )
}

async fn asked_by_a_step(rig: &Rig, source: &str) -> conductor::Applied {
    let applied = rig.apply(source, json!({})).await;
    let asked = rig.dir.join("asked");
    until(rig, &asked, |file| std::fs::read_to_string(file).is_ok_and(|said| said.ends_with('\n'))).await;
    applied
}

fn step<'a>(run: &'a conductor::RunInfo, id: &str) -> &'a conductor::StepInfo {
    run.steps.iter().find(|step| step.step == id).unwrap()
}

#[tokio::test]
async fn a_step_that_waited_for_an_agent_that_outlives_the_daemon_has_its_answer_as_if_nothing_had_happened()
{
    let (mut rig, hold) = rig(true).await;
    let applied = asked_by_a_step(&rig, &asks("", "")).await;

    rig.restart().await;
    let_go(&hold);

    let run = soon("the run", rig.engine.wait_run(applied.run.as_ref().unwrap())).await.unwrap();
    let work = step(&run, "work");
    assert_eq!(work.status, conductor::StepStatus::Ok, "{work:?}");
    assert_eq!((work.output["reply"].clone(), work.output["again"].clone()), (json!("I did it."), json!(1)));
    assert_eq!(asked(&rig), 1);
}

#[tokio::test]
async fn a_step_that_waited_for_one_that_does_not_is_given_the_answer_of_its_being_asked_again() {
    let (mut rig, hold) = rig(false).await;
    let applied = asked_by_a_step(&rig, &asks("", "")).await;

    rig.restart().await;
    let asked_again = rig.dir.join("asked");
    until(&rig, &asked_again, |_| asked(&rig) == 2).await;
    let_go(&hold);

    let run = soon("the run", rig.engine.wait_run(applied.run.as_ref().unwrap())).await.unwrap();
    assert_eq!(step(&run, "work").output["reply"], "I did it.");
    // One turn all the same, begun twice: nothing asked for a second.
    assert_eq!(rig.agents.turns("builder", 10).await.unwrap().len(), 1);
}

#[tokio::test]
async fn a_step_that_says_to_leave_it_fails_saying_its_work_was_left_and_its_agent_is_not_asked_again() {
    let (mut rig, _hold) = rig(false).await;
    let applied = asked_by_a_step(&rig, &asks("", ", after_restart: leave")).await;

    rig.restart().await;

    let run = soon("the run", rig.engine.wait_run(applied.run.as_ref().unwrap())).await.unwrap();
    let why = step(&run, "work").error.clone().unwrap();
    assert!(why.contains("it was asked for to be left as it was then and not begun again"), "{why}");
    assert_eq!((asked(&rig), rig.agents.turns("builder", 10).await.unwrap().len()), (1, 1));
}

#[tokio::test]
async fn a_step_that_says_to_leave_it_takes_the_answer_its_agent_had_given_before_the_daemon_stopped() {
    let (mut rig, hold) = rig(false).await;
    // What asks is cut off while it waits for a second thing: the first
    // was answered, and its step is done again from what it kept.
    let source = "blueprint: asks\npermissions: [agents]\nsteps:\n  work:\n    do: agent.task\n    timeout: 5m\n    \
                  with: { slot: builder, thread: work, prompt: Add a cart, after_restart: leave }\n  then:\n    \
                  needs: [work]\n    do: agent.task\n    timeout: 5m\n    \
                  with: { slot: builder, thread: work, prompt: Sweep up }\n";
    let applied = asked_by_a_step(&rig, source).await;
    let_go(&hold);
    until(&rig, &rig.dir.join("asked"), |_| asked(&rig) == 2).await;

    rig.restart().await;
    until(&rig, &rig.dir.join("asked"), |_| asked(&rig) == 3).await;
    let_go(&hold);

    let run = soon("the run", rig.engine.wait_run(applied.run.as_ref().unwrap())).await.unwrap();
    assert_eq!(step(&run, "work").output["reply"], "I did it.");
    assert_eq!(rig.agents.turns("builder", 10).await.unwrap().len(), 2);
}

#[tokio::test]
async fn what_keeps_nothing_and_asks_the_same_again_is_given_the_turn_that_was_left_in_hand_and_no_second_asking()
 {
    let (mut rig, hold) = rig(true).await;
    let source = asks("durable: false\n", "");
    asked_by_a_step(&rig, &source).await;

    rig.restart().await;
    // What asked is over, as anything that keeps nothing is. Whatever
    // wanted the work asks for it again, as a card's lane does its round.
    let again = rig.apply(&source, json!({})).await;
    let_go(&hold);

    let run = soon("the run", rig.engine.wait_run(again.run.as_ref().unwrap())).await.unwrap();
    let work = step(&run, "work");
    assert_eq!((work.output["reply"].clone(), work.output["again"].clone()), (json!("I did it."), json!(1)));
    assert_eq!((asked(&rig), rig.agents.turns("builder", 10).await.unwrap().len()), (1, 1));

    // And once only: the same asked a third time is a new asking.
    let third = rig.apply(&source, json!({})).await;
    until(&rig, &rig.dir.join("asked"), |_| asked(&rig) == 2).await;
    let_go(&hold);
    let run = soon("the run", rig.engine.wait_run(third.run.as_ref().unwrap())).await.unwrap();
    assert_eq!(step(&run, "work").output["again"], 0);
}

/// Two things asked of `builder` one after the other by something that
/// keeps nothing, the first in the word `asking`.
fn asks_twice(asking: &str) -> String {
    format!(
        "blueprint: asks\npermissions: [agents]\ndurable: false\nsteps:\n  work:\n    do: agent.task\n    \
         timeout: 5m\n    with: {{ slot: builder, thread: work, prompt: Add a cart, asking: {asking} }}\n  then:\n    \
         needs: [work]\n    do: agent.task\n    timeout: 5m\n    with: {{ slot: builder, thread: work, prompt: Sweep up }}\n"
    )
}

#[tokio::test]
async fn what_says_which_asking_it_is_is_given_the_answer_its_agent_had_given_and_the_agent_is_not_asked_twice()
 {
    let (mut rig, hold) = rig(false).await;
    // Its agent has answered the first thing and has the second in hand
    // when the daemon stops: nothing is left of the first asking but the
    // turn, which has ended.
    asked_by_a_step(&rig, &asks_twice("first")).await;
    let_go(&hold);
    until(&rig, &rig.dir.join("asked"), |_| asked(&rig) == 2).await;

    rig.restart().await;
    // Asked again in the same word, the first is that asking and has its
    // answer; the second was in hand, and is begun again as any is.
    let again = rig.apply(&asks_twice("first"), json!({})).await;
    until(&rig, &rig.dir.join("asked"), |_| asked(&rig) == 3).await;
    let_go(&hold);
    let run =
        soon("the run that asked again", rig.engine.wait_run(again.run.as_ref().unwrap())).await.unwrap();
    assert_eq!(step(&run, "work").output["reply"], "I did it.");
    assert_eq!(rig.agents.turns("builder", 10).await.unwrap().len(), 2);

    // In another word it is another asking, of the same thing.
    let another = rig.apply(&asks_twice("second"), json!({})).await;
    until(&rig, &rig.dir.join("asked"), |_| asked(&rig) == 4).await;
    let_go(&hold);
    until(&rig, &rig.dir.join("asked"), |_| asked(&rig) == 5).await;
    let_go(&hold);
    soon("the run in another word", rig.engine.wait_run(another.run.as_ref().unwrap())).await.unwrap();
    assert_eq!(rig.agents.turns("builder", 10).await.unwrap().len(), 4);
}

#[tokio::test]
async fn an_answer_is_given_to_nobody_but_what_asked_for_it_whatever_word_another_says() {
    let (rig, hold) = rig(false).await;
    let source = asks("durable: false\n", ", asking: first");
    let first = asked_by_a_step(&rig, &source).await;
    let_go(&hold);
    soon("the run", rig.engine.wait_run(first.run.as_ref().unwrap())).await.unwrap();

    // Something else asks the same of the same agent, in the same
    // conversation and the same word: it is asking, and is answered anew.
    let other = rig.apply(&source.replace("blueprint: asks", "blueprint: asks-too"), json!({})).await;
    until(&rig, &rig.dir.join("asked"), |_| asked(&rig) == 2).await;
    let_go(&hold);
    let run = soon("the run", rig.engine.wait_run(other.run.as_ref().unwrap())).await.unwrap();
    assert_eq!(step(&run, "work").output["reply"], "I did it.");
    assert_eq!(rig.agents.turns("builder", 10).await.unwrap().len(), 2);
}

#[tokio::test]
async fn one_that_still_waited_its_turn_when_the_daemon_stopped_is_the_same_asking_and_is_not_queued_twice() {
    let (mut rig, hold) = rig(false).await;
    // The agent has something in hand, so what the step asks for waits:
    // the same words as it happens, asked by nobody in particular.
    in_hand(&rig, TurnRequest::default()).await;
    let source = asks("durable: false\n", ", asking: first");
    rig.apply(&source, json!({})).await;
    let waiting = async {
        while rig.agents.turns("builder", 10).await.unwrap().len() < 2 {
            tokio::task::yield_now().await;
        }
    };
    soon("the step's turn to be waiting", waiting).await;

    rig.restart().await;
    let again = rig.apply(&source, json!({})).await;
    // What was in hand is begun again and let go, and then the step's own.
    until(&rig, &rig.dir.join("asked"), |_| asked(&rig) == 2).await;
    let_go(&hold);
    until(&rig, &rig.dir.join("asked"), |_| asked(&rig) == 3).await;
    let_go(&hold);
    let run = soon("the run", rig.engine.wait_run(again.run.as_ref().unwrap())).await.unwrap();
    assert_eq!(step(&run, "work").output["reply"], "I did it.");
    assert_eq!(rig.agents.turns("builder", 10).await.unwrap().len(), 2);
}

#[tokio::test]
async fn a_program_still_at_work_by_itself_past_the_time_its_step_was_given_is_stopped_when_the_daemon_is_back()
 {
    let (mut rig, hold) = rig(true).await;
    let applied = asked_by_a_step(&rig, &asks("", "")).await;
    let turns = rig.agents.turns("builder", 10).await.unwrap();
    let pid = std::fs::read_to_string(files(&rig, &turns[0].id, "pid")).unwrap();

    rig.stop();
    // The step gave it five minutes, and the daemon is away for ten.
    rig.clock.advance(std::time::Duration::from_secs(600));
    rig.start().await;

    let run = soon("the run", rig.engine.wait_run(applied.run.as_ref().unwrap())).await.unwrap();
    let why = step(&run, "work").error.clone().unwrap();
    assert!(
        why.contains(
            "the time this was given had passed with its program still at work by itself. It was stopped"
        ),
        "{why}"
    );
    gone("its program to be stopped", &[&pid]).await;
    let_go(&hold);
    assert_eq!(asked(&rig), 1);
}

#[tokio::test]
async fn one_that_finished_within_its_time_while_no_daemon_was_there_has_its_answer_however_long_that_was() {
    let (mut rig, hold) = rig(true).await;
    let applied = asked_by_a_step(&rig, &asks("", "")).await;
    let turns = rig.agents.turns("builder", 10).await.unwrap();

    rig.stop();
    let_go(&hold);
    let exit = files(&rig, &turns[0].id, "exit");
    until(&rig, &exit, |exit| exit.exists()).await;
    rig.clock.advance(std::time::Duration::from_secs(600));
    rig.start().await;

    let run = soon("the run", rig.engine.wait_run(applied.run.as_ref().unwrap())).await.unwrap();
    assert_eq!(step(&run, "work").output["reply"], "I did it.");
}

/// Unix's alone: the folder a program at work has its files in is given
/// another name here while the program has them open, which Windows
/// does not allow of a folder with a file in it that is open.
#[cfg(unix)]
#[tokio::test]
async fn a_copy_of_a_factory_takes_up_none_of_the_programs_the_other_one_left_at_work() {
    // The other factory's agent is at work by itself. This database is
    // made a copy, as `up --from` makes one: what was in hand is ended in
    // it, and the folder of what the daemon runs is not the other's.
    let (mut rig, hold) = rig(true).await;
    let id = in_hand(&rig, TurnRequest::default()).await;
    let pid = std::fs::read_to_string(files(&rig, &id, "pid")).unwrap();

    rig.stop();
    std::fs::rename(rig.dir.join("run"), rig.dir.join("the-other-factorys")).unwrap();
    let at = rig.journal.now_ms();
    rig.journal.db().call(move |connection| agents::copy::leave_out(connection, at)).await.unwrap();
    // And in each project's own journal that keeps its agents' turns,
    // as a copy does in each file it takes.
    for own in rig.journal.every_keeping(journal::keeps::TURNS).into_iter().skip(1) {
        own.db().call(move |connection| agents::copy::leave_out_beside(connection, at)).await.unwrap();
    }
    rig.start().await;

    let turn = ended(&rig, &id).await;
    assert_eq!((turn.status.as_str(), turn.error.as_deref()), ("failed", Some(agents::copy::NOT_TAKEN_UP)));
    // The other factory's program is as it was: nothing here looked for it.
    assert!(crate::alive(&pid));
    assert_eq!(asked(&rig), 1);
    // (Ended by hand, since no daemon of this test's is its.)
    let_go(&hold);
    let _ = hosting::group::stop(pid.trim().parse().unwrap(), true);
}

#[tokio::test]
async fn a_program_taken_up_that_nobody_asks_for_again_is_let_go_on_and_is_held_to_the_time_it_was_given() {
    // What asked for it is gone for good, and nothing asks again. It is not
    // stopped for that: its agent was told to go on by itself. But it is
    // not left at work for ever either.
    let (mut rig, hold) = rig(true).await;
    let given = rig.journal.now_ms() + 300_000;
    let id = in_hand(&rig, TurnRequest { until: Some(given), ..TurnRequest::default() }).await;
    let pid = std::fs::read_to_string(files(&rig, &id, "pid")).unwrap();

    rig.restart().await;
    rig.event("agent.turn.started", |event| event.data["again"] == 1).await;
    assert_eq!(rig.agents.turn(&id).await.unwrap().status, "running", "taken up, and let go on");
    rig.clock.advance(std::time::Duration::from_secs(301));

    let turn = ended(&rig, &id).await;
    assert_eq!(
        turn.error.as_deref(),
        Some(
            "builder went on with this by itself after the daemon was stopped, nobody asked for it again, and the \
             time it was given ran out. Its program was stopped"
        )
    );
    gone("its program to be stopped", &[&pid]).await;
    let_go(&hold);
    assert_eq!(asked(&rig), 1);
}

/// Unix's alone: a wall is held by a program of that system's, and what
/// stands in for it here is a file of its shell (`walls.rs`).
#[cfg(unix)]
#[tokio::test]
async fn a_program_that_outlives_the_daemon_is_behind_the_wall_it_was_behind_and_is_shown_nothing_more() {
    let (walls, told) = crate::walls::stand_in();
    let walled = crate::walls::Walled::holding(walls).await;
    let rig = &walled.rig;
    let turn = walled.does("builder", "echo done", None).await;
    assert_eq!((turn.status.as_str(), turn.outlives), ("done", false), "{:?}", turn.error);
    let before = std::fs::read_to_string(&told).unwrap();

    let spec = SlotSpec { name: "builder".into(), outlives: Some(true), ..SlotSpec::default() };
    rig.agents.ensure(spec, &ada()).await.unwrap();
    let turn = walled.does("builder", "echo done", None).await;
    assert_eq!((turn.status.as_str(), turn.outlives), ("done", true), "{:?}", turn.error);

    // What holds the wall was told to show what it showed before and to
    // start what it started before, word for word: what keeps the program
    // going is outside the wall, and its files are the daemon's.
    assert_eq!(std::fs::read_to_string(&told).unwrap(), before);
    let kept_in = rig.dir.join("run").display().to_string();
    assert!(!before.contains(&kept_in), "{before}");
    // Nor is the program told where they are.
    let env = std::fs::read_to_string(rig.dir.join("standin/1.0/bin/env")).unwrap();
    assert!(!env.contains("CONDUCTOR_KEPT") && !env.contains(&kept_in), "{env}");
    // And nothing of it is left once its turn has ended.
    let left: Vec<_> = std::fs::read_dir(rig.dir.join("run"))
        .map(|dir| dir.flatten().map(|entry| entry.file_name()).collect())
        .unwrap_or_default();
    assert!(left.iter().all(|name| !name.to_string_lossy().starts_with("turn.")), "{left:?}");
}
