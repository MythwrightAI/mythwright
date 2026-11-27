//! A run's steps are read in the order they began, whatever they are
//! called and however fast they went.
//!
//! When a step began is kept in milliseconds, and several steps of a run
//! begin in one: here every one of them does, since the clock stands
//! still. Read by when they began and then by name, a run that did
//! `zulu` and then `alpha` read `alpha`, `zulu`. So a step's record says
//! which of its run's steps it was to begin, from the write that first
//! keeps it, and that is what its run's steps are read by: while the run
//! goes and afterwards, kept as it goes or all at its end, passed over,
//! tried again, done again inside steps done as one, and taken up after
//! a stop.

use conductor::{RunInfo, RunStatus};
use serde_json::json;

use crate::Rig;

/// Undoes the step of the runner's tables by which a step's row
/// says which of its run's steps it was to begin: no row says, and the
/// tables are a version back. Said after `BEFORE_TEXTS_WERE_KEPT_BY_VERSION`,
/// which undoes the step after it, and before `BEFORE_KEPT_BESIDE`, which
/// undoes the step before it, by what plays a journal from further back.
pub const BEFORE_STEPS_SAID_THEIR_PLACE: &str = "
    ALTER TABLE steps DROP COLUMN nth;
    UPDATE schema_versions SET version = version - 1 WHERE namespace = 'conductor';
";

/// Undoes the step of the runner's tables which keeps the text an
/// instance runs in the version that has it and names the version in the
/// instance's row: the text is put back in the row, the column goes, and
/// the tables are a version back. Said after [`BEFORE_RUNS_KEPT_THEIR_PLACE`]
/// by what plays a journal from further back, and before
/// [`BEFORE_STEPS_SAID_THEIR_PLACE`].
pub const BEFORE_TEXTS_WERE_KEPT_BY_VERSION: &str = "
    UPDATE instances SET source = (SELECT kept.source FROM instance_versions kept
                                   WHERE kept.instance = instances.id AND kept.version = instances.text)
    WHERE text IS NOT NULL;
    ALTER TABLE instances DROP COLUMN text;
    UPDATE schema_versions SET version = version - 1 WHERE namespace = 'conductor';
";

/// Undoes the last step of the runner's tables, by which a version keeps
/// no text that a version of its project keeps already and names that
/// one: each is given its text back, the columns go, and
/// the tables are a version back. Said first by what plays a journal from
/// further back, before [`BEFORE_ASKERS_WERE_KEPT`].
pub const BEFORE_TEXTS_WERE_HELD: &str = "
    UPDATE instance_versions SET source = (SELECT held.source FROM instance_versions held
                                           WHERE held.instance = instance_versions.held_by
                                             AND held.version = instance_versions.held_at)
    WHERE held_by IS NOT NULL;
    ALTER TABLE instance_versions DROP COLUMN held_at;
    ALTER TABLE instance_versions DROP COLUMN held_by;
    UPDATE schema_versions SET version = version - 1 WHERE namespace = 'conductor';
";

/// Undoes the step of the runner's tables by which a run that carries
/// its instance over says which step asked: the column goes, and the
/// tables are a version back. Said after [`BEFORE_TEXTS_WERE_HELD`] by
/// what plays a journal from further back, before [`BEFORE_ENVIRONMENTS`].
pub const BEFORE_ASKERS_WERE_KEPT: &str = "
    ALTER TABLE runs DROP COLUMN asked_by;
    UPDATE schema_versions SET version = version - 1 WHERE namespace = 'conductor';
";

/// Undoes the step of the runner's tables by which an instance says the
/// environment of its project's it runs under: the rule and the column
/// go, and the tables are a version back. Said after
/// [`BEFORE_ASKERS_WERE_KEPT`] by what plays a journal from further back,
/// before [`BEFORE_BEGINNINGS_WERE_WRITTEN`].
pub const BEFORE_ENVIRONMENTS: &str = "
    DROP TRIGGER instances_environment_stays;
    ALTER TABLE instances DROP COLUMN environment;
    UPDATE schema_versions SET version = version - 1 WHERE namespace = 'conductor';
";

/// Undoes the step of the runner's tables by which a step's row says
/// where it began and a run's that it began, where the journal passed
/// that on and did not keep it: no row says, and the tables are a
/// version back. Said after [`BEFORE_ENVIRONMENTS`] by what plays a
/// journal from further back, before [`BEFORE_RUNS_KEPT_THEIR_PLACE`].
pub const BEFORE_BEGINNINGS_WERE_WRITTEN: &str = "
    ALTER TABLE steps DROP COLUMN began;
    ALTER TABLE steps DROP COLUMN began_at;
    ALTER TABLE runs DROP COLUMN begun;
    UPDATE schema_versions SET version = version - 1 WHERE namespace = 'conductor';
";

/// Undoes the step of the runner's tables by which a run keeps where
/// the journal it says things in had got to as it was written: no run
/// says, and the tables are a version back. Said after
/// [`BEFORE_BEGINNINGS_WERE_WRITTEN`] by what plays a journal from further
/// back, before [`BEFORE_TEXTS_WERE_KEPT_BY_VERSION`].
pub const BEFORE_RUNS_KEPT_THEIR_PLACE: &str = "
    ALTER TABLE runs DROP COLUMN place;
    UPDATE schema_versions SET version = version - 1 WHERE namespace = 'conductor';
";

/// The names of a run's steps, in the order the run is read in.
fn read(run: &RunInfo) -> Vec<&str> {
    run.steps.iter().map(|step| step.step.as_str()).collect()
}

/// The steps that were said to start, in the order they were said to.
async fn said_to_start(rig: &Rig) -> Vec<String> {
    rig.events("step.started").await.into_iter().filter_map(|event| event.source.step).collect()
}

#[tokio::test]
async fn the_steps_of_a_run_are_read_in_the_order_they_began_and_not_by_what_they_are_called() {
    let rig = Rig::new().await;
    let run = rig
        .run(
            "\
blueprint: chain
steps:
  zulu: { do: test.echo }
  alpha: { needs: [zulu], do: test.echo }
  mid: { needs: [alpha], do: test.echo }
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    let began: Vec<i64> = run.steps.iter().map(|step| step.started).collect();
    assert_eq!(began, [began[0]; 3], "each began in the same millisecond, which says nothing of their order");
    assert_eq!(read(&run), ["zulu", "alpha", "mid"]);
}

#[tokio::test]
async fn steps_that_need_nothing_of_each_other_and_steps_done_as_one_are_read_as_they_were_said_to_start() {
    let rig = Rig::new().await;
    let run = rig
        .run(
            "\
blueprint: wide
steps:
  zulu: { do: test.echo }
  alpha: { do: test.echo }
  mid:
    needs: [zulu, alpha]
    task:
      yard: { do: test.echo }
      bell: { needs: [yard], do: test.echo }
  able: { needs: [mid], do: test.echo }
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(read(&run), ["zulu", "alpha", "mid", "mid.yard", "mid.bell", "able"]);
    assert_eq!(read(&run), said_to_start(&rig).await, "which is the order they were said to start in");
}

/// Steps that are passed over, among steps that are not: two before a
/// step that waits, which are held and written with it, and at the end
/// one whose `when` does not hold and two the run ended before.
const PASSES: &str = "\
blueprint: passes
steps:
  zulu: { do: test.echo }
  mike: { needs: [zulu], when: ${ false }, do: test.echo }
  alpha: { needs: [mike], when: ${ false }, do: test.echo }
  door: { needs: [alpha], do: test.gate, with: { name: door } }
  bravo: { needs: [door], when: ${ false }, do: test.echo }
  stop: { needs: [bravo], do: end, with: { reason: enough } }
  yard: { needs: [stop], do: test.echo }
  able: { needs: [yard], do: test.echo }
";

#[tokio::test]
async fn steps_that_are_passed_over_are_read_where_they_were_passed_while_the_run_goes_and_afterwards() {
    let rig = Rig::new().await;
    let applied = rig.apply(PASSES, json!({})).await;
    rig.reached("door").await;
    let going = rig.engine.run(applied.run.as_ref().unwrap()).await.unwrap();
    assert_eq!(read(&going), ["zulu", "mike", "alpha", "door"]);
    rig.open("door");
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    let whole = ["zulu", "mike", "alpha", "door", "bravo", "stop", "yard", "able"];
    assert_eq!(
        read(&run),
        whole,
        "those the run ended before its turn came to, in the order its text has them"
    );
    let kept = rig.engine.run(&run.id).await.unwrap();
    assert_eq!(read(&kept), whole, "and read again, from what was kept");
}

#[tokio::test]
async fn the_steps_of_a_run_that_keeps_nothing_as_it_goes_are_read_in_the_order_they_began() {
    let rig = Rig::new().await;
    let applied = rig.apply(&PASSES.replace("steps:", "durable: false\nsteps:"), json!({})).await;
    rig.reached("door").await;
    let going = rig.engine.run(applied.run.as_ref().unwrap()).await.unwrap();
    assert_eq!(read(&going), ["zulu", "mike", "alpha", "door"], "from where they are held, while it goes");
    rig.open("door");
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    let whole = ["zulu", "mike", "alpha", "door", "bravo", "stop", "yard", "able"];
    assert_eq!(read(&run), whole, "written once, with the run's ending, in that order");
    assert_eq!(read(&rig.engine.run(&run.id).await.unwrap()), whole);
}

#[tokio::test]
async fn a_step_that_is_tried_again_is_read_where_it_first_began() {
    let rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    let applied = rig
        .apply(
            "\
blueprint: tries
steps:
  zulu: { do: test.echo }
  mike: { needs: [zulu], do: test.flaky, with: { times: 1 }, retry: { max: 2, backoff: 10s } }
  side: { needs: [zulu], do: test.gate, with: { name: side } }
  alpha: { needs: [mike, side], do: test.echo }
",
            json!({}),
        )
        .await;
    rig.reached("side").await;
    rig.advance_after(since, 1, "10s").await;
    rig.open("side");
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(crate::step(&run, "mike").attempt, 2);
    assert_eq!(read(&run), ["zulu", "mike", "side", "alpha"]);
}

#[tokio::test]
async fn steps_done_as_one_that_are_done_again_are_read_after_what_began_before_they_were() {
    let rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    let applied = rig
        .apply(
            "\
blueprint: again
steps:
  zulu: { do: test.echo }
  mid:
    needs: [zulu]
    retry: { max: 2, backoff: 10s }
    task:
      yard: { do: test.count, with: { key: yards } }
      bell: { needs: [yard], do: test.flaky, with: { times: 1 } }
  side: { needs: [zulu], do: test.gate, with: { name: side } }
  able: { needs: [mid, side], do: test.echo }
",
            json!({}),
        )
        .await;
    rig.reached("side").await;
    rig.advance_after(since, 1, "10s").await;
    // The second go has come to its end before the step beside it has.
    rig.event("step.finished", |event| event.source.step.as_deref() == Some("mid")).await;
    let going = rig.engine.run(applied.run.as_ref().unwrap()).await.unwrap();
    // `side` began while the first go was under way, and what the second
    // go did began after it: its steps are the task's steps again, each
    // written anew, and no two of the run's steps have one place.
    assert_eq!(read(&going), ["zulu", "mid", "side", "mid.yard", "mid.bell"]);
    rig.open("side");
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(rig.shared.count("yards"), 2);
    assert_eq!(read(&run), ["zulu", "mid", "side", "mid.yard", "mid.bell", "able"]);
}

#[tokio::test]
async fn a_run_taken_up_after_a_stop_reads_its_steps_in_the_order_they_began() {
    let mut rig = Rig::new().await;
    let applied = rig
        .apply(
            "\
blueprint: stopped
steps:
  zulu: { do: test.echo }
  over: { needs: [zulu], when: ${ false }, do: test.echo }
  mike: { needs: [over], do: test.durable, with: { name: mike } }
  alpha: { needs: [mike], do: test.echo }
",
            json!({}),
        )
        .await;
    rig.reached("mike").await;
    rig.restart().await;
    let taken_up = rig.engine.run(applied.run.as_ref().unwrap()).await.unwrap();
    assert_eq!(read(&taken_up), ["zulu", "over", "mike"]);
    rig.open("mike");
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    // The step taken up is written again as it goes on and as it ends,
    // and is where it was.
    assert_eq!(read(&run), ["zulu", "over", "mike", "alpha"]);
}

#[tokio::test]
async fn a_run_from_before_reads_its_steps_as_it_did_and_those_it_goes_on_to_after_them() {
    let mut rig = Rig::new().await;
    let applied = rig
        .apply(
            "\
blueprint: before
steps:
  zulu: { do: test.echo }
  mike: { needs: [zulu], do: test.durable, with: { name: mike } }
  alpha: { needs: [mike], do: test.echo }
  able: { needs: [alpha], do: test.echo }
",
            json!({}),
        )
        .await;
    rig.reached("mike").await;
    // The tables as the program before left them, with the run under
    // way: no step of it says which it was to begin.
    rig.engine.halt();
    rig.engine.halted().await;
    let undone = format!(
        "{BEFORE_TEXTS_WERE_HELD}{BEFORE_ASKERS_WERE_KEPT}{BEFORE_ENVIRONMENTS}{BEFORE_BEGINNINGS_WERE_WRITTEN}{BEFORE_RUNS_KEPT_THEIR_PLACE}{BEFORE_TEXTS_WERE_KEPT_BY_VERSION}{BEFORE_STEPS_SAID_THEIR_PLACE}"
    );
    rig.journal.db().call(move |db| db.execute_batch(&undone)).await.unwrap();
    // This program's conductor starts on it, and its tables are brought
    // up by the steps they have not had.
    rig.restart().await;
    let taken_up = rig.engine.run(applied.run.as_ref().unwrap()).await.unwrap();
    assert_eq!(read(&taken_up), ["mike", "zulu"], "as they were read: by when each began, then by name");
    rig.open("mike");
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    // The step taken up says nothing still, and what began since is
    // after it, in the order it began.
    assert_eq!(read(&run), ["mike", "zulu", "alpha", "able"]);
}
