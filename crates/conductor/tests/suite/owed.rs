//! What a journal from a program before this one is owed, once.

use serde_json::json;

use crate::Rig;

const STAYS: &str = "\
blueprint: stays
on:
  go: { event: stays.go, do: test.count, with: { key: went } }
";

async fn said_active(rig: &Rig, id: &str) -> usize {
    let said = rig.events("instance.active").await;
    said.iter().filter(|event| event.source.instance.as_deref() == Some(id)).count()
}

/// Takes out of the journal that anything was said to be active.
async fn never_said(rig: &Rig) {
    rig.journal
        .db()
        .call(|db| db.execute("DELETE FROM events WHERE topic = 'instance.active'", []))
        .await
        .unwrap();
}

#[tokio::test]
async fn an_instance_a_program_from_before_wrote_as_active_and_never_said_to_be_is_said_to_be_once() {
    let mut rig = Rig::new().await;
    let applied = rig.apply(STAYS, json!({})).await;
    rig.event("instance.active", |event| event.source.instance.as_deref() == Some(applied.instance.as_str()))
        .await;

    // As a program from before left it, stopped between writing the
    // instance as active and saying so: nothing said, and the tables as
    // they were before a row could say what it is owed (and so before
    // one could say how far its stop had come, or a run what it is to
    // carry its instance over to, how its text hears, whose a row is, or
    // which of its run's steps a step was to begin, which were added
    // after).
    never_said(&rig).await;
    rig.journal
        .db()
        .call(|db| {
            db.execute_batch(crate::ordered::BEFORE_TEXTS_WERE_HELD)?;
            db.execute_batch(crate::ordered::BEFORE_ASKERS_WERE_KEPT)?;
            db.execute_batch(crate::ordered::BEFORE_ENVIRONMENTS)?;
            db.execute_batch(crate::ordered::BEFORE_BEGINNINGS_WERE_WRITTEN)?;
            db.execute_batch(crate::ordered::BEFORE_RUNS_KEPT_THEIR_PLACE)?;
            db.execute_batch(crate::ordered::BEFORE_TEXTS_WERE_KEPT_BY_VERSION)?;
            db.execute_batch(crate::ordered::BEFORE_STEPS_SAID_THEIR_PLACE)?;
            db.execute_batch(crate::BEFORE_KEPT_BESIDE)?;
            db.execute_batch(crate::BEFORE_ROWS_WERE_APART)?;
            db.execute_batch(
                "ALTER TABLE instances DROP COLUMN heard;
                 ALTER TABLE runs DROP COLUMN carries;
                 ALTER TABLE instances DROP COLUMN hooks;
                 DROP INDEX instances_owing;
                 ALTER TABLE instances DROP COLUMN owes;
                 UPDATE schema_versions SET version = version - 4 WHERE namespace = 'conductor';",
            )
        })
        .await
        .unwrap();

    // The first conductor of this program to start on it says so.
    rig.restart().await;
    assert_eq!(said_active(&rig, &applied.instance).await, 1);
    // The next has nothing to say.
    rig.restart().await;
    assert_eq!(said_active(&rig, &applied.instance).await, 1, "said once");

    // And nothing is looked for after that: it was owed once, by a
    // journal from before. What this program writes as active it says in
    // the same write, so there is no such thing to find.
    never_said(&rig).await;
    rig.restart().await;
    assert_eq!(said_active(&rig, &applied.instance).await, 0, "no start looks again");
}
