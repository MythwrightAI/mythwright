//! Where the text an instance runs is kept: in the version that has it,
//! once, and not again in the instance's own row, which is written again
//! whenever its state or its status changes.

use journal::Journal;
use kernel::{Fields, Principal};
use runs::{Becomes, Definition, InstanceInfo, InstanceStatus, Store, Taken};

use crate::{ending, instance, kept, of_project, run, says};

/// What an instance's first version keeps of its text, and which
/// version it names as keeping it.
async fn first_version(journal: &Journal, id: &str) -> (String, Option<String>, Option<i64>) {
    let id = id.to_string();
    let asked = "SELECT source, held_by, held_at FROM instance_versions WHERE instance = ?1 AND version = 1";
    let read = move |connection: &mut rusqlite::Connection| {
        connection.query_row(asked, [&id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
    };
    journal.db().call(read).await.unwrap()
}

/// What the instance's own row holds of its text, and which version it
/// says keeps it.
async fn row(journal: &Journal, id: &str) -> (String, Option<i64>) {
    let id = id.to_string();
    journal
        .db()
        .call(move |connection| {
            connection.query_row("SELECT source, text FROM instances WHERE id = ?1", [&id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
        })
        .await
        .unwrap()
}

fn definition(source: &str) -> Definition {
    Definition {
        revision: blueprint::revision(source),
        source: source.into(),
        params: Fields::new(),
        principal: Some(Principal::system()),
        state: None,
        before: None,
        heard: None,
    }
}

async fn text(store: &Store, id: &str) -> String {
    store.instance(id).await.unwrap().unwrap().source
}

#[tokio::test]
async fn an_instance_runs_the_text_its_version_keeps_and_its_own_row_keeps_none_of_it() {
    let (journal, store) = kept().await;
    store.insert_instance(instance("one"), None, Vec::new()).await.unwrap();
    assert_eq!(row(&journal, "one").await, (String::new(), Some(1)));
    assert_eq!(text(&store, "one").await, "name: greet\n");

    // Written again as it becomes active, the row is as small as it was.
    store.insert_run(run("first", "one")).await.unwrap();
    store.end_run(ending("first", "one", true)).await.unwrap();
    assert_eq!(store.instance("one").await.unwrap().unwrap().status, InstanceStatus::Active);
    assert_eq!(row(&journal, "one").await, (String::new(), Some(1)));

    // A text taken where it is: its next version keeps it.
    let taken = definition("name: greet\nsteps: []\n");
    store.take_text("one", taken, Vec::new(), 10, Vec::new()).await.unwrap();
    assert_eq!(row(&journal, "one").await, (String::new(), Some(2)));
    assert_eq!(text(&store, "one").await, "name: greet\nsteps: []\n");
    let versions = store.versions("one", 10).await.unwrap();
    let kept: Vec<_> = versions.iter().map(|version| (version.version, version.source.as_str())).collect();
    assert_eq!(kept, [(2, "name: greet\nsteps: []\n"), (1, "name: greet\n")]);
}

#[tokio::test]
async fn a_text_applied_over_an_instance_is_kept_by_its_version_and_one_gone_back_to_in_the_row() {
    let (journal, store) = kept().await;
    store.insert_instance(instance("one"), None, Vec::new()).await.unwrap();
    let applied = Taken {
        definition: definition("name: greet\nversion: 2\n"),
        state: Fields::new(),
        previous: Some(definition("name: greet\n")),
        below: false,
        places: Vec::new(),
    };
    assert!(store.update_instance_definition("one", applied, 10, Vec::new()).await.unwrap().is_some());
    assert_eq!(row(&journal, "one").await, (String::new(), Some(2)));
    assert_eq!(text(&store, "one").await, "name: greet\nversion: 2\n");

    // Its steps failed: it goes back to what it ran, which is in its row.
    store.insert_run(run("update", "one")).await.unwrap();
    let mut ended = ending("update", "one", true);
    let failed = "the update's steps failed".to_string();
    ended.becomes = Some(Becomes {
        status: None,
        update: Some((Some(definition("name: greet\n")), Some(failed))),
        says: says("instance.updated", "one"),
        ..ended.becomes.unwrap()
    });
    store.end_run(ended).await.unwrap();
    assert_eq!(row(&journal, "one").await, ("name: greet\n".to_string(), None));
    assert_eq!(text(&store, "one").await, "name: greet\n");
    let outcomes: Vec<_> =
        store.versions("one", 10).await.unwrap().into_iter().map(|version| version.outcome).collect();
    assert_eq!(outcomes, ["reverted", "applied"]);
}

#[tokio::test]
async fn an_instance_a_program_from_before_wrote_runs_the_text_in_its_row() {
    let (journal, store) = kept().await;
    store.insert_instance(instance("one"), None, Vec::new()).await.unwrap();
    // As one was written before the version said where its text is.
    journal
        .db()
        .call(|connection| {
            connection.execute("UPDATE instances SET source = 'name: from before\n', text = NULL", [])
        })
        .await
        .unwrap();
    assert_eq!(text(&store, "one").await, "name: from before\n");
    let listed = store.instances(runs::InstanceFilter::default()).await.unwrap();
    assert_eq!(listed[0].source, "name: from before\n");
}

#[tokio::test]
async fn an_instance_made_with_a_text_its_project_keeps_already_names_the_version_that_keeps_it() {
    let (journal, store) = kept().await;
    let text_of = |id: &str| of_project(id, Some("shop"), None);
    store.insert_instance(text_of("one"), None, Vec::new()).await.unwrap();
    store.insert_instance(text_of("two"), None, Vec::new()).await.unwrap();
    assert_eq!(first_version(&journal, "one").await, ("name: greet\n".into(), None, None));
    assert_eq!(first_version(&journal, "two").await, (String::new(), Some("one".into()), Some(1)));
    // One made after it names the one that keeps it, through the one before.
    store.insert_instance(text_of("three"), None, Vec::new()).await.unwrap();
    assert_eq!(first_version(&journal, "three").await, (String::new(), Some("one".into()), Some(1)));
    for id in ["one", "two", "three"] {
        assert_eq!(text(&store, id).await, "name: greet\n");
        let versions = store.versions(id, 10).await.unwrap();
        assert_eq!(versions[0].source, "name: greet\n");
    }

    // Of another project, or of none, it keeps its own: a project's rows go
    // where it goes.
    store.insert_instance(of_project("four", Some("mill"), None), None, Vec::new()).await.unwrap();
    store.insert_instance(instance("five"), None, Vec::new()).await.unwrap();
    assert_eq!(first_version(&journal, "four").await, ("name: greet\n".into(), None, None));
    assert_eq!(first_version(&journal, "five").await, ("name: greet\n".into(), None, None));

    // The same revision is not the same text: another text under it is
    // kept whole by its own version.
    let other = InstanceInfo { source: "name: other\n".into(), ..text_of("six") };
    store.insert_instance(other, None, Vec::new()).await.unwrap();
    assert_eq!(first_version(&journal, "six").await, ("name: other\n".into(), None, None));
    assert_eq!(text(&store, "six").await, "name: other\n");

    // One that has since taken another text is not named for the first.
    store.take_text("six", definition("name: greet\nsteps: []\n"), Vec::new(), 10, Vec::new()).await.unwrap();
    store.insert_instance(text_of("seven"), None, Vec::new()).await.unwrap();
    assert_eq!(first_version(&journal, "seven").await, ("name: greet\n".into(), None, None));
    assert_eq!(text(&store, "seven").await, "name: greet\n");
}
