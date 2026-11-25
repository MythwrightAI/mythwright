//! A turn that is one of several pieces of work on one thing at once:
//! given to none of the agents it may not go to, told folders of its
//! project it is not shown, and saying where nothing could hide them.

use conductor::RunStatus;
use serde_json::json;

use crate::Rig;

fn step<'a>(run: &'a conductor::RunInfo, id: &str) -> &'a conductor::StepInfo {
    run.steps.iter().find(|step| step.step == id).unwrap()
}

#[tokio::test]
async fn work_goes_to_none_of_those_that_may_not_take_it_and_one_named_among_them_is_refused() {
    let rig = Rig::new().await;
    rig.agent("first", "fake").await;
    rig.agent("second", "fake").await;
    rig.fake.will_say("Looked.");
    rig.fake.will_say("Looked again.");
    // Of the two, the one brought up first would take it, and may not.
    let run = rig
        .run(
            "\
blueprint: judged
permissions: [agents]
steps:
  look: { do: agent.task, with: { thread: \"judge:shop-1\", not: [first], prompt: Look. } }
  again: { needs: [look], do: agent.task, with: { thread: \"judge:shop-1\", not: [second], prompt: Look again. } }
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(step(&run, "look").output["slot"], "second");
    // Not even the one the conversation was held with, where it may not take it.
    assert_eq!(step(&run, "again").output["slot"], "first");

    // With nobody left, there is nobody to ask; and one named among them is refused, saying why.
    let run = rig
        .run(
            "\
blueprint: nobody
permissions: [agents]
steps:
  look: { do: agent.task, with: { not: [first, second], if_available: true, prompt: Look. } }
  named: { needs: [look], do: agent.task, with: { slot: first, not: [first], prompt: Look. } }
",
            json!({}),
        )
        .await;
    assert_eq!(step(&run, "look").output["available"], false);
    assert_eq!(run.status, RunStatus::Failed);
    let why = run.error.unwrap_or_default();
    assert!(why.contains("`first` is named for this, and is one of those that may not take it"), "{why}");
    assert_eq!(rig.fake.seen.lock().unwrap().len(), 2, "nobody was asked for either");
}

#[tokio::test]
async fn folders_a_turn_is_not_shown_are_within_its_project_and_where_nothing_hides_them_it_says_so() {
    let rig = Rig::new().await;
    rig.agent("worker", "fake").await;
    rig.fake.will_say("Done.");
    rig.fake.will_say("Done too.");
    // The tests hold no walls: told to hide something, nothing hid it, and that is said.
    let run = rig
        .run(
            "\
blueprint: apart
permissions: [agents]
steps:
  told: { do: agent.task, with: { hidden: [\".lanes/shop-1--b\"], prompt: Work. } }
  plain: { needs: [told], do: agent.task, with: { prompt: Work. } }
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(step(&run, "told").output["unhidden"], true);
    assert_eq!(step(&run, "plain").output["unhidden"], false);

    // A folder that would lead out of the project is said of the step, and nobody is asked.
    let run = rig
        .run(
            "\
blueprint: astray
permissions: [agents]
steps:
  told: { do: agent.task, with: { hidden: [\"../elsewhere\"], prompt: Work. } }
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Failed);
    let why = run.error.unwrap_or_default();
    assert!(
        why.contains("`hidden` is folders within the project") && why.contains("`../elsewhere`"),
        "{why}"
    );
    assert_eq!(rig.fake.seen.lock().unwrap().len(), 2);
}

#[cfg(unix)]
#[tokio::test]
async fn behind_a_real_wall_a_turn_finds_the_folders_it_is_not_shown_empty_and_its_own_place_as_it_is() {
    let walls = hosting::Walls::system();
    if let Err(why) = walls.can() {
        println!("skipped: this system holds no wall ({why})");
        return;
    }
    let walled = crate::walls::Walled::holding(walls).await;
    let project = walled.project();
    for (place, file) in [("shop-1--a", "mine.md"), ("shop-1--b", "theirs.md")] {
        std::fs::create_dir_all(project.join(".lanes").join(place)).unwrap();
        std::fs::write(project.join(".lanes").join(place).join(file), "work\n").unwrap();
    }
    let tries = [
        "cat mine.md >/dev/null && echo 'own: there'",
        "cat ../shop-1--b/theirs.md 2>/dev/null || echo 'theirs: not there'",
        "touch ../shop-1--b/more.md 2>/dev/null || echo 'theirs: not written'",
    ]
    .join("; ");
    let request = agents::TurnRequest {
        slot: "builder".into(),
        kind: "task".into(),
        thread: "contend:shop-1:a".into(),
        prompt: format!("Please see to this.\n{tries}"),
        cwd: Some(".lanes/shop-1--a".into()),
        hidden: vec![".lanes/shop-1--b".into()],
        ..agents::TurnRequest::default()
    };
    let turn = walled.rig.agents.enqueue(request).await.unwrap();
    let turn = crate::soon("the turn to end", walled.rig.agents.wait(&turn)).await.unwrap();
    assert_eq!(turn.status, "done", "{:?}", turn.error);
    for line in ["own: there", "theirs: not there", "theirs: not written"] {
        assert!(turn.reply.lines().any(|said| said.trim() == line), "`{line}` in:\n{}", turn.reply);
    }
    assert!(
        project.join(".lanes/shop-1--b/theirs.md").exists()
            && !project.join(".lanes/shop-1--b/more.md").exists()
    );
}
