//! The Trash: what is deleted is kept, left out of everything that reads
//! what it was of, given back where it was, and goes for good after the
//! days the factory keeps it, or when somebody deletes it for good.

use std::time::Duration;

use kernel::trash::DAY_MS;
use serde_json::{Map, Value, json};
use studio::collections::Spec;
use studio::trash::Kind;

use crate::{Rig, ada, agent};

fn titled(title: &str) -> Map<String, Value> {
    json!({ "title": title }).as_object().unwrap().clone()
}

async fn notes(rig: &Rig, project: &str, reads: Option<&str>) {
    let fields = json!({ "title": { "type": "text" } });
    let spec = Spec::read(&fields, &json!(null), None).unwrap().read_by(reads).unwrap();
    let spec = spec.with_history(&json!({ "record.*": "" })).unwrap();
    rig.studio.collections.ensure(project, "notes", spec, None, &ada()).await.unwrap();
}

#[tokio::test]
async fn a_card_deleted_is_in_the_trash_and_on_no_board_and_comes_back_to_its_place() {
    let rig = Rig::new().await;
    rig.board("shop").await;
    let board = &rig.studio.board;
    let first = board.add("shop", "First", "", Some("ready"), vec![], &ada()).await.unwrap();
    let second = board.add("shop", "Second", "", Some("ready"), vec![], &ada()).await.unwrap();
    let third = board.add("shop", "Third", "", Some("ready"), vec![], &ada()).await.unwrap();
    let mut lines = rig.journal.follow_every(journal::Places::default(), studio::history::TOPIC);
    rig.studio.records.comment(&second.id, "Before it went.", false, &ada(), None).await.unwrap();
    // The line of its history that says so is written a moment after it is
    // said, by what keeps histories: deleted before that, the card has no
    // such line to keep, and what the Trash keeps of it is what it had.
    let commented =
        |history: Vec<studio::history::Line>| history.iter().any(|line| line.topic == "record.commented");
    while !commented(rig.studio.collections.history(&second.id).await.unwrap()) {
        lines.next().await.unwrap();
    }

    board.remove(&second.id, &ada()).await.unwrap();
    let left: Vec<String> = board.list("shop").await.unwrap().into_iter().map(|item| item.id).collect();
    assert_eq!(left, [first.id.clone(), third.id.clone()]);
    assert_eq!(board.column("shop", "ready").await.unwrap().len(), 2);
    assert!(board.get(&second.id).await.is_err(), "found by its id only by the Trash");
    assert!(rig.studio.collections.record(&second.id).await.is_err());
    let trash = rig.trash.list(&ada()).await.unwrap();
    assert_eq!(trash.len(), 1);
    let entry = &trash[0];
    assert_eq!(
        (entry.kind, entry.thing.as_str(), entry.called.as_str()),
        (Kind::Record, second.id.as_str(), "Second")
    );
    assert_eq!(entry.by, "user:ada");
    assert_eq!(entry.goes, Some(entry.at + 30 * DAY_MS));
    assert!(entry.may);
    // A card added meanwhile is numbered after it, so that it comes back under its id.
    let fourth = board.add("shop", "Fourth", "", Some("ready"), vec![], &ada()).await.unwrap();
    assert!(fourth.number > second.number);
    assert_eq!(rig.topics("board.item.removed,trash.put").await, ["board.item.removed", "trash.put"]);

    rig.trash.restore(&entry.id, false, &ada()).await.unwrap();
    let back = board.get(&second.id).await.unwrap();
    assert_eq!((back.status.as_str(), back.position), ("ready", second.position));
    let order: Vec<String> =
        board.column("shop", "ready").await.unwrap().into_iter().map(|item| item.id).collect();
    assert_eq!(order, [first.id, second.id.clone(), third.id, fourth.id]);
    assert!(rig.trash.list(&ada()).await.unwrap().is_empty());
    let history = rig.studio.collections.history(&second.id).await.unwrap();
    assert!(
        history.iter().any(|line| line.topic == "record.commented"),
        "its comments are kept: {history:?}"
    );
    let restored = rig.events("board.item.created").await;
    assert_eq!(restored.last().unwrap().data["restored"], true);
    assert_eq!(rig.topics("trash.restored").await.len(), 1);
}

#[tokio::test]
async fn a_card_whose_place_is_taken_comes_back_at_the_bottom_of_its_column() {
    let rig = Rig::new().await;
    rig.board("shop").await;
    let board = &rig.studio.board;
    let first = board.add("shop", "First", "", Some("ready"), vec![], &ada()).await.unwrap();
    let second = board.add("shop", "Second", "", Some("ready"), vec![], &ada()).await.unwrap();
    board.remove(&first.id, &ada()).await.unwrap();
    // Moved to the top, where the first was.
    board.move_to(&second.id, "ready", None, &ada()).await.unwrap();
    let taken = board.get(&second.id).await.unwrap().position;
    assert_eq!(taken, first.position, "the second has the first's place");
    let entry = rig.trash.list(&ada()).await.unwrap().remove(0);
    rig.trash.restore(&entry.id, false, &ada()).await.unwrap();
    let order: Vec<String> =
        board.column("shop", "ready").await.unwrap().into_iter().map(|item| item.id).collect();
    assert_eq!(order, [second.id, first.id.clone()]);
    assert!(board.get(&first.id).await.unwrap().position > taken);
}

#[tokio::test]
async fn a_record_deleted_is_in_no_list_and_is_deleted_for_good_by_whoever_could_have_deleted_it() {
    let rig = Rig::new().await;
    rig.studio.projects.create("shop", None, &ada()).await.unwrap();
    notes(&rig, "shop", None).await;
    let records = &rig.studio.records;
    let kept = records.add("shop", "notes", titled("Kept"), &ada(), None).await.unwrap();
    let gone = records.add("shop", "notes", titled("Gone"), &ada(), None).await.unwrap();
    records.remove(&gone.id, &ada(), None).await.unwrap();
    let listed: Vec<String> =
        records.list("shop", "notes", &ada()).await.unwrap().into_iter().map(|record| record.id).collect();
    assert_eq!(listed, [kept.id]);
    assert!(records.get(&gone.id, &ada()).await.is_err());

    // Somebody who may read records but not change them sees it, and may
    // neither give it back nor delete it for good.
    let reader = agent("reader", &["records:read", "projects:read"]);
    let seen = rig.trash.list(&reader).await.unwrap();
    assert_eq!(seen.len(), 1);
    assert!(!seen[0].may);
    assert!(rig.trash.restore(&seen[0].id, false, &reader).await.is_err());
    assert!(rig.trash.forget(&seen[0].id, &reader).await.is_err());
    // Somebody who may not read records does not see it at all.
    let outside = agent("outside", &["projects:read"]);
    assert!(rig.trash.list(&outside).await.unwrap().is_empty());
    assert!(rig.trash.forget(&seen[0].id, &outside).await.is_err());

    rig.trash.forget(&seen[0].id, &ada()).await.unwrap();
    assert!(rig.trash.list(&ada()).await.unwrap().is_empty());
    assert!(records.get(&gone.id, &ada()).await.is_err());
    assert_eq!(rig.events("trash.gone").await[0].data["why"], "asked");
}

#[tokio::test]
async fn a_private_tables_row_in_the_trash_is_seen_only_by_who_may_read_the_table() {
    let rig = Rig::new().await;
    rig.studio.projects.create("shop", None, &ada()).await.unwrap();
    notes(&rig, "shop", Some("private:notes")).await;
    let record = rig.studio.records.add("shop", "notes", titled("Secret"), &ada(), None).await.unwrap();
    rig.studio.records.remove(&record.id, &ada(), None).await.unwrap();
    let member = agent("member", &["records:read", "records:write", "projects:read"]);
    assert!(rig.trash.list(&member).await.unwrap().is_empty());
    let id = rig.trash.list(&ada()).await.unwrap()[0].id.clone();
    let refused = rig.trash.restore(&id, false, &member).await.unwrap_err();
    assert_eq!(refused.kind, kernel::ErrorKind::NotFound, "as one that is not there");
    let trusted = agent("trusted", &["records:read", "records:write", "projects:read", "private:notes"]);
    assert_eq!(rig.trash.list(&trusted).await.unwrap().len(), 1);
    rig.trash.restore(&id, false, &trusted).await.unwrap();
    assert!(rig.studio.records.get(&record.id, &ada()).await.is_ok());
    let said = rig.events("trash.put").await;
    assert_eq!(said[0].source.takes.as_deref(), Some("private:notes"));
}

#[tokio::test]
async fn what_has_been_in_the_trash_as_long_as_the_factory_keeps_it_goes_and_nought_days_deletes_at_once() {
    let rig = Rig::new().await;
    rig.board("shop").await;
    let board = &rig.studio.board;
    let old = board.add("shop", "Old", "", None, vec![], &ada()).await.unwrap();
    board.remove(&old.id, &ada()).await.unwrap();
    rig.clock.advance(Duration::from_millis((20 * DAY_MS) as u64));
    let young = board.add("shop", "Young", "", None, vec![], &ada()).await.unwrap();
    board.remove(&young.id, &ada()).await.unwrap();
    assert_eq!(rig.trash.empty().await.unwrap(), 0);
    rig.clock.advance(Duration::from_millis((10 * DAY_MS) as u64));
    assert_eq!(rig.trash.empty().await.unwrap(), 1);
    let left = rig.trash.list(&ada()).await.unwrap();
    assert_eq!(left.iter().map(|entry| entry.thing.as_str()).collect::<Vec<_>>(), [young.id.as_str()]);
    assert_eq!(rig.events("trash.gone").await[0].data["why"], "days");

    // Kept for ever, nothing goes however long it has been there.
    let member = agent("member", &["projects:write", "board:write"]);
    assert!(rig.trash.keep_for(None, &member).await.is_err(), "the owner's to say");
    rig.trash.keep_for(None, &ada()).await.unwrap();
    rig.clock.advance(Duration::from_millis((400 * DAY_MS) as u64));
    assert_eq!(rig.trash.empty().await.unwrap(), 0);
    assert_eq!(rig.trash.list(&ada()).await.unwrap()[0].goes, None);

    // Nought days: what is deleted goes at once, as before there was a Trash.
    rig.trash.keep_for(Some(0), &ada()).await.unwrap();
    assert_eq!(rig.trash.empty().await.unwrap(), 1);
    let now = board.add("shop", "Now", "", None, vec![], &ada()).await.unwrap();
    board.remove(&now.id, &ada()).await.unwrap();
    assert!(rig.trash.list(&ada()).await.unwrap().is_empty());
    assert!(board.get(&now.id).await.is_err());
    assert!(rig.trash.keep_for(Some(kernel::trash::MOST_DAYS + 1), &ada()).await.is_err());
}

#[tokio::test]
async fn a_project_forgotten_is_in_the_trash_and_comes_back_in_its_workspace_or_the_usual_one() {
    let rig = Rig::new().await;
    let spaces = &rig.studio.workspaces;
    spaces.create("First", &ada()).await.unwrap();
    let usual = spaces.usual().await.unwrap();
    let other = spaces.create("Other", &ada()).await.unwrap();
    for project in ["shop", "yard"] {
        rig.studio.projects.create_in(project, None, &ada(), false, Some(&other.id)).await.unwrap();
    }
    rig.board("shop").await;
    let card = rig.studio.board.add("shop", "Kept with it", "", None, vec![], &ada()).await.unwrap();
    let trashed = rig.studio.projects.forget("shop", &ada()).await.unwrap().unwrap();
    assert!(rig.studio.projects.get("shop").await.is_err());
    let path = rig.dir.join("projects").join("shop");
    assert!(path.is_dir(), "its files are where they were");
    let entry = rig.trash.list(&ada()).await.unwrap().remove(0);
    assert_eq!(
        (entry.id.as_str(), entry.kind, entry.called.as_str()),
        (trashed.as_str(), Kind::Project, "shop")
    );
    assert!(rig.trash.list(&agent("nobody", &[])).await.unwrap().is_empty());

    rig.trash.restore(&trashed, false, &ada()).await.unwrap();
    assert_eq!(rig.studio.projects.get("shop").await.unwrap().path, path.to_string_lossy());
    assert_eq!(spaces.of_project("shop").await.unwrap(), Some(other.id.clone()));
    assert_eq!(rig.studio.board.get(&card.id).await.unwrap().title, "Kept with it");

    // Its workspace gone while it was in the Trash, it comes back in the usual one.
    let trashed = rig.studio.projects.forget("yard", &ada()).await.unwrap().unwrap();
    spaces.remove(&other.id, usual.as_deref(), &ada()).await.unwrap();
    rig.trash.restore(&trashed, false, &ada()).await.unwrap();
    assert_eq!(spaces.of_project("yard").await.unwrap(), usual);

    // One forgotten and added again is not given back over it.
    let trashed = rig.studio.projects.forget("yard", &ada()).await.unwrap().unwrap();
    rig.studio.projects.create("yard", Some(rig.dir.join("elsewhere")), &ada()).await.unwrap();
    let refused = rig.trash.restore(&trashed, false, &ada()).await.unwrap_err();
    assert!(refused.message.contains("again"), "{}", refused.message);
}

#[tokio::test]
async fn a_file_deleted_leaves_the_project_is_kept_by_the_factory_and_comes_back_or_beside() {
    let rig = Rig::new().await;
    let data = rig.dir.join("data");
    let projects = &rig.studio.projects;
    projects.create("shop", None, &ada()).await.unwrap();
    projects.write("shop", "docs/notes.md", "# Notes\n", &ada()).await.unwrap();
    let folder = rig.dir.join("projects").join("shop");

    projects.delete("shop", "docs/notes.md", &ada()).await.unwrap();
    assert!(!folder.join("docs/notes.md").exists(), "out of the folder agents work in");
    let entry = rig.trash.list(&ada()).await.unwrap().remove(0);
    assert_eq!(
        (entry.kind, entry.thing.as_str(), entry.called.as_str()),
        (Kind::File, "docs/notes.md", "notes.md")
    );
    let kept = data.join(studio::trash::FOLDER).join(&entry.id);
    assert_eq!(std::fs::read_to_string(kept.join("kept")).unwrap(), "# Notes\n");

    // Something else at its place: refused, saying how to give it back beside it.
    projects.write("shop", "docs/notes.md", "# Other notes\n", &ada()).await.unwrap();
    let refused = rig.trash.restore(&entry.id, false, &ada()).await.unwrap_err();
    assert!(refused.message.contains("docs/notes (restored).md"), "{}", refused.message);
    let (_, at, _) = rig.trash.restore(&entry.id, true, &ada()).await.unwrap();
    assert_eq!(at, "docs/notes (restored).md");
    assert_eq!(std::fs::read_to_string(folder.join("docs/notes (restored).md")).unwrap(), "# Notes\n");
    assert_eq!(std::fs::read_to_string(folder.join("docs/notes.md")).unwrap(), "# Other notes\n");
    assert!(!kept.exists());

    // Back where it was, where nothing is.
    projects.delete("shop", "docs/notes.md", &ada()).await.unwrap();
    let entry = rig.trash.list(&ada()).await.unwrap().remove(0);
    let (_, at, _) = rig.trash.restore(&entry.id, false, &ada()).await.unwrap();
    assert_eq!(at, "docs/notes.md");
    assert_eq!(std::fs::read_to_string(folder.join("docs/notes.md")).unwrap(), "# Other notes\n");

    // Deleted for good, nothing of it is kept.
    projects.delete("shop", "docs/notes.md", &ada()).await.unwrap();
    let entry = rig.trash.list(&ada()).await.unwrap().remove(0);
    rig.trash.forget(&entry.id, &ada()).await.unwrap();
    assert!(!data.join(studio::trash::FOLDER).join(&entry.id).exists());
    assert!(!folder.join("docs/notes.md").exists());
}

#[tokio::test]
async fn a_daemon_stopped_in_the_middle_of_moving_a_file_finds_what_it_began_and_finishes_it() {
    let rig = Rig::new().await;
    let data = rig.dir.join("data");
    rig.studio.projects.create("shop", None, &ada()).await.unwrap();
    let folder = data.join(studio::trash::FOLDER);
    // Moved, and no row kept: the row is written.
    let moved = folder.join("trash_moved");
    std::fs::create_dir_all(&moved).unwrap();
    let whence = json!({ "id": "trash_moved", "project": "shop", "path": "a.md", "by": "user:ada", "at": 5, "reads": null });
    std::fs::write(moved.join("whence.json"), whence.to_string()).unwrap();
    std::fs::write(moved.join("kept"), "a").unwrap();
    // Never moved: nothing of it is kept.
    let begun = folder.join("trash_begun");
    std::fs::create_dir_all(&begun).unwrap();
    std::fs::write(begun.join("whence.json"), whence.to_string().replace("trash_moved", "trash_begun"))
        .unwrap();
    rig.trash.settle().await.unwrap();
    let listed = rig.trash.list(&ada()).await.unwrap();
    assert_eq!(listed.iter().map(|entry| entry.id.as_str()).collect::<Vec<_>>(), ["trash_moved"]);
    assert!(!begun.exists());
    // Marked as going, with its row still there: it goes.
    std::fs::write(moved.join("going"), "").unwrap();
    rig.trash.settle().await.unwrap();
    assert!(rig.trash.list(&ada()).await.unwrap().is_empty());
    assert!(!moved.exists());
}
