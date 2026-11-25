use kernel::Event;
use studio::search::{MARK_END, MARK_START};
use studio::{Changes, Hit};

use crate::{Rig, ada};

/// A project with a few documents, a channel with messages and a card.
async fn seeded() -> Rig {
    let rig = Rig::new().await;
    let studio = &rig.studio;
    rig.board("Riverside Shop").await;
    studio
        .projects
        .write(
            "riverside-shop",
            "docs/checkout.md",
            "# Checkout\n\nThe checkout flow charges the card and then reserves stock.\n",
            &ada(),
        )
        .await
        .unwrap();
    studio
        .projects
        .write(
            "riverside-shop",
            "fleet.bp",
            "blueprint: fleet\ndescription: Runs the checkout service.\nsteps:\n  a: { do: noop }\n",
            &ada(),
        )
        .await
        .unwrap();
    studio.projects.write("riverside-shop", "logo.png", "not really an image", &ada()).await.unwrap();
    studio.channels.create("riverside-shop", Some("riverside-shop"), "", &ada()).await.unwrap();
    studio
        .channels
        .post("riverside-shop", &ada(), "Should checkout reserve stock before charging?", None)
        .await
        .unwrap();
    studio.channels.post("riverside-shop", &ada(), "Lunch?", None).await.unwrap();
    studio
        .board
        .add("riverside-shop", "Checkout page", "Build the page customers pay on.", None, vec![], &ada())
        .await
        .unwrap();
    rig
}

fn kinds(hits: &[Hit]) -> Vec<&str> {
    let mut kinds: Vec<&str> = hits.iter().map(|hit| hit.kind.as_str()).collect();
    kinds.sort();
    kinds
}

#[tokio::test]
async fn everything_a_project_knows_is_searchable_in_one_place() {
    let rig = seeded().await;
    let search = &rig.studio.search;
    assert_eq!(
        search.reindex().await.unwrap(),
        8,
        "four scaffold docs, two written, a message with the word, a card... all of it"
    );

    let hits = search.query("checkout", None, None, 20).await.unwrap();
    assert_eq!(kinds(&hits), ["blueprint", "card", "file", "message"]);
    // The card and the file have the word in their titles; the blueprint and
    // the message only in their text. Title matches come first.
    assert_eq!(kinds(&hits[..2]), ["card", "file"]);

    let doc = hits.iter().find(|hit| hit.kind == "file").unwrap();
    assert_eq!(doc.reference, "file:riverside-shop/docs/checkout.md");
    assert_eq!(doc.link["path"], "docs/checkout.md");
    assert!(doc.snippet.contains(&format!("{MARK_START}checkout{MARK_END}")), "{:?}", doc.snippet);

    let message = hits.iter().find(|hit| hit.kind == "message").unwrap();
    assert_eq!(message.title, "#riverside-shop · Ada");
    assert!(message.link["channel"].is_string());

    assert!(
        search.query("png image", None, None, 5).await.unwrap().is_empty(),
        "files that are not text are not indexed"
    );
    assert_eq!(search.progress().docs, 8);
}

#[tokio::test]
async fn a_search_narrows_as_it_is_typed_and_can_be_filtered() {
    let rig = seeded().await;
    let search = &rig.studio.search;
    search.reindex().await.unwrap();

    assert_eq!(
        search.query("chec", None, None, 20).await.unwrap().len(),
        4,
        "the last word may be unfinished"
    );
    assert_eq!(
        kinds(&search.query("checkout stock", None, None, 20).await.unwrap()),
        ["file", "message"],
        "every word must appear"
    );
    assert_eq!(kinds(&search.query("checkout", Some("blueprint"), None, 20).await.unwrap()), ["blueprint"]);
    assert!(search.query("checkout", None, Some("another-project"), 20).await.unwrap().is_empty());
    assert_eq!(search.query("checkout", None, None, 2).await.unwrap().len(), 2);

    // Punctuation that means something to the index is just punctuation here.
    for odd in ["", "   ", "\"", "checkout AND (", "NOT", "a:b*", "--"] {
        search
            .query(odd, None, None, 5)
            .await
            .unwrap_or_else(|err| panic!("{odd:?} broke the search: {}", err.message));
    }
}

#[tokio::test]
async fn the_index_follows_what_happens_without_being_rebuilt() {
    let rig = seeded().await;
    let studio = &rig.studio;
    let search = &studio.search;
    search.reindex().await.unwrap();
    rig.journal.kv_set("search", "cursor", &serde_json::json!(rig.journal.last_seq())).await.unwrap();
    let count = |text: &'static str| async move { search.query(text, None, None, 20).await.unwrap().len() };

    studio
        .projects
        .write("riverside-shop", "docs/refunds.md", "Refunds go back to the original card.\n", &ada())
        .await
        .unwrap();
    studio.channels.post("riverside-shop", &ada(), "Who owns refunds?", None).await.unwrap();
    studio.board.add("riverside-shop", "Refunds", "", None, vec![], &ada()).await.unwrap();
    assert_eq!(count("refunds").await, 0, "not until the index has caught up");
    search.catch_up().await.unwrap();
    assert_eq!(count("refunds").await, 3);

    // Edits replace; removals remove.
    studio
        .projects
        .write("riverside-shop", "docs/refunds.md", "Returns go back to the original card.\n", &ada())
        .await
        .unwrap();
    studio
        .board
        .update("riverside-shop-2", Changes { title: Some("Returns".into()), ..Changes::default() }, &ada())
        .await
        .unwrap();
    studio.projects.remove("riverside-shop", "docs/checkout.md", &ada()).await.unwrap();
    search.catch_up().await.unwrap();
    assert_eq!(count("refunds").await, 2, "the message, and the file whose name is still refunds.md");
    assert_eq!(count("returns").await, 2);
    assert!(
        search.query("reserves", Some("file"), None, 5).await.unwrap().is_empty(),
        "the removed file is gone"
    );

    // Catching up again does nothing, and a full pass finds nothing to do.
    search.catch_up().await.unwrap();
    assert_eq!(search.reindex().await.unwrap(), 0, "what events brought in is already current");
}

#[tokio::test]
async fn a_full_pass_drops_what_no_longer_exists_and_reports_progress() {
    let rig = seeded().await;
    let search = &rig.studio.search;
    let mut progress = rig.journal.follow(rig.journal.last_seq(), "search.*");
    search.reindex().await.unwrap();

    let mut seen: Vec<Event> = Vec::new();
    loop {
        let event = progress.next().await.unwrap();
        let done = event.topic == "search.indexed";
        seen.push((*event).clone());
        if done {
            break;
        }
    }
    let phases: Vec<&str> = seen.iter().filter_map(|event| event.data["phase"].as_str()).collect();
    assert_eq!(phases.first(), Some(&"scanning"));
    assert!(phases.contains(&"indexing") && phases.last() == Some(&"idle"), "{phases:?}");
    let finished = seen.last().unwrap();
    assert_eq!((finished.data["docs"].as_u64(), finished.data["written"].as_u64()), (Some(8), Some(8)));
    assert!(
        seen.iter().filter(|event| event.topic == "search.progress").all(|event| !event.is_durable()),
        "progress is not kept"
    );

    // Deleted behind our back: gone after the next pass.
    let project = rig.studio.projects.get("riverside-shop").await.unwrap();
    std::fs::remove_file(std::path::Path::new(&project.path).join("docs/checkout.md")).unwrap();
    search.reindex().await.unwrap();
    assert!(search.query("reserves", Some("file"), None, 5).await.unwrap().is_empty());
    assert_eq!(search.progress().docs, 7);
}

#[tokio::test]
async fn indexing_many_documents_is_done_in_small_batches() {
    let rig = Rig::new().await;
    let project = rig.studio.projects.create("Big", None, &ada()).await.unwrap();
    let dir = std::path::Path::new(&project.path).join("notes");
    std::fs::create_dir_all(&dir).unwrap();
    for n in 0..120 {
        std::fs::write(dir.join(format!("note-{n:03}.md")), format!("Note number {n} mentions zebras.\n"))
            .unwrap();
    }
    let mut progress = rig.journal.follow(rig.journal.last_seq(), "search.progress");
    assert_eq!(rig.studio.search.reindex().await.unwrap(), 123);

    let mut steps = Vec::new();
    while steps.last() != Some(&(123, "idle")) {
        let event = progress.next().await.unwrap();
        steps.push((
            event.data["done"].as_u64().unwrap(),
            if event.data["phase"] == "idle" { "idle" } else { "busy" },
        ));
    }
    let reported: Vec<u64> =
        steps.iter().filter(|(_, phase)| *phase == "busy").map(|(done, _)| *done).collect();
    assert!(reported.len() >= 5, "progress is reported as it goes, not once at the end: {reported:?}");
    assert!(reported.windows(2).all(|pair| pair[0] <= pair[1]), "and never goes backwards: {reported:?}");
    assert_eq!(rig.studio.search.query("zebras", None, None, 200).await.unwrap().len(), 120);
}
