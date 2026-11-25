//! A handler's places among the events: each is a place in one journal,
//! kept by what that journal is called, and a place in one is never moved
//! by what is written of another. And where a run's journal had got to as
//! the run was written, which everything the run says is after.

use runs::{Definition, Place, Taken};

use crate::{instance, kept, of_project, run};

/// What a project's journal is called, as whoever opens one says it.
fn of_shop() -> String {
    journal::of_project("shop")
}

fn handlers(names: &[&str]) -> Vec<(String, String)> {
    names.iter().map(|name| ("one".to_string(), name.to_string())).collect()
}

#[tokio::test]
async fn the_place_of_a_handler_is_never_moved_back_and_one_with_none_is_given_none() {
    let (journal, store) = kept().await;
    let here = journal.name();
    store.set_place("one", "heard", here, 10).await.unwrap();
    store.advance_places(handlers(&["heard", "gone"]), here, 5).await.unwrap();
    assert_eq!(store.place("one", "heard", here).await.unwrap(), Some(10));
    store.advance_places(handlers(&["heard", "gone"]), here, 20).await.unwrap();
    assert_eq!(store.place("one", "heard", here).await.unwrap(), Some(20));
    assert_eq!(store.place("one", "gone", here).await.unwrap(), None);
}

#[tokio::test]
async fn a_handlers_places_in_two_journals_are_kept_apart_and_neither_moves_the_other() {
    let (journal, store) = kept().await;
    let (factory, project) = (journal.name().to_string(), of_shop());
    assert_ne!(factory, project);
    store.insert_instance(of_project("one", Some("shop"), None), None, Vec::new()).await.unwrap();
    let place = |journal: &str| {
        let (store, journal) = (store.clone(), journal.to_string());
        async move { store.place("one", "heard", &journal).await.unwrap() }
    };

    // A place in one journal is no place in the other.
    store.set_place("one", "heard", &factory, 4_000).await.unwrap();
    assert_eq!((place(&factory).await, place(&project).await), (Some(4_000), None));
    store.set_place("one", "heard", &project, 30).await.unwrap();
    assert_eq!((place(&factory).await, place(&project).await), (Some(4_000), Some(30)));

    // Moved in one, whatever the numbers of the other are: the project's
    // is behind the factory's by number and is brought forward by its
    // own journal's events, and the factory's is not taken back to them.
    store.advance_places(handlers(&["heard"]), &project, 50).await.unwrap();
    assert_eq!((place(&factory).await, place(&project).await), (Some(4_000), Some(50)));
    store.advance_places(handlers(&["heard"]), &factory, 4_100).await.unwrap();
    assert_eq!((place(&factory).await, place(&project).await), (Some(4_100), Some(50)));
    store.set_place("one", "heard", &project, 60).await.unwrap();
    assert_eq!((place(&factory).await, place(&project).await), (Some(4_100), Some(60)));

    // An event of one journal starts a run and moves the place in that
    // journal with it, in the one write.
    store.insert_triggered_run(run("first", "one"), "heard", &project, 70).await.unwrap();
    assert_eq!((place(&factory).await, place(&project).await), (Some(4_100), Some(70)));
    store.insert_triggered_run(run("second", "one"), "heard", &factory, 4_200).await.unwrap();
    assert_eq!((place(&factory).await, place(&project).await), (Some(4_200), Some(70)));
    // And a run that no event of a journal's started moves neither.
    store.insert_triggered_run(run("third", "one"), "heard", &factory, 0).await.unwrap();
    assert_eq!((place(&factory).await, place(&project).await), (Some(4_200), Some(70)));

    // Both are the project's, each said with its journal.
    let places = store.places_of(Some("shop")).await.unwrap();
    let at = |journal: &str, seq: u64| Place {
        instance: "one".into(),
        handler: "heard".into(),
        journal: journal.into(),
        seq,
    };
    assert_eq!(places, [at(&factory, 4_200), at(&project, 70)]);
    assert!(store.places_of(None).await.unwrap().is_empty());

    // A handler the text no longer has keeps a place in neither.
    store.set_place("one", "other", &factory, 1).await.unwrap();
    store.keep_places("one", vec!["other".into()]).await.unwrap();
    assert_eq!((place(&factory).await, place(&project).await), (None, None));
    assert_eq!(store.place("one", "other", &factory).await.unwrap(), Some(1));
    store.set_place("one", "other", &project, 2).await.unwrap();
    store.drop_places("one", "other").await.unwrap();
    assert!(store.places_of(Some("shop")).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_text_that_is_taken_gives_each_handler_it_brings_its_place_and_leaves_one_that_was_there() {
    let (journal, store) = kept().await;
    let here = journal.name().to_string();
    store.insert_instance(instance("one"), None, Vec::new()).await.unwrap();
    store.set_place("one", "heard", &here, 12).await.unwrap();
    let definition = Definition {
        revision: "r2".into(),
        source: "name: greet\ndescription: again\n".into(),
        params: kernel::Fields::new(),
        principal: None,
        state: None,
        before: None,
        heard: None,
    };
    let taken = Taken {
        definition,
        state: kernel::Fields::new(),
        previous: None,
        below: false,
        places: vec![
            ("heard".into(), here.clone(), 40),
            ("new".into(), here.clone(), 40),
            ("new".into(), of_shop(), 3),
        ],
    };
    assert!(store.update_instance_definition("one", taken, 5, Vec::new()).await.unwrap().is_some());
    assert_eq!(store.place("one", "heard", &here).await.unwrap(), Some(12));
    assert_eq!(store.place("one", "new", &here).await.unwrap(), Some(40));
    assert_eq!(store.place("one", "new", &of_shop()).await.unwrap(), Some(3));
    // Its instance is nobody's, and so are its places.
    assert_eq!(store.places_of(None).await.unwrap().len(), 3);
}

#[tokio::test]
async fn a_run_keeps_where_its_journal_had_got_to_as_it_was_written_and_says_everything_after_it() {
    let (journal, store) = kept().await;
    store.insert_instance(instance("one"), None, Vec::new()).await.unwrap();
    let before = journal.emit("said.before", kernel::Source::default(), serde_json::json!({})).await.unwrap();
    store.insert_run(run("first", "one")).await.unwrap();
    assert_eq!(store.run_place("first").await.unwrap(), before.seq);
    let of_run = kernel::Source { run: Some("first".into()), ..kernel::Source::instance("one") };
    let said = journal.emit("run.started", of_run, serde_json::json!({})).await;
    assert!(said.unwrap().seq > store.run_place("first").await.unwrap());
    store.insert_triggered_run(run("heard", "one"), "on", journal.name(), before.seq).await.unwrap();
    assert_eq!(store.run_place("heard").await.unwrap(), journal.last_seq());
    // One that is not there, as one a program from before wrote, says nought.
    assert_eq!(store.run_place("nowhere").await.unwrap(), 0);
}
