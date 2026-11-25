//! A page deleted takes every page under it into the Trash, in one write
//! and one entry, by whatever door deletes a record, through what the
//! keeper installs (`studio::factory::Written::goes_with`): given back
//! with it in one write, under its page where that is there and of its
//! workspace and a wiki of its own where it is not, which is said; and,
//! where the factory keeps nothing deleted, gone at once with every
//! page's revisions. What deletes for good from the Trash is the `trash`
//! crate's, and the API suite's (`wikitrash.rs`).

use journal::Journal;
use kernel::topics::{Line, Tree};
use serde_json::{Value, json};

use crate::{Rig, ada};

async fn make(rig: &Rig, title: &str, under: &str) -> String {
    let made = rig.topics.make_under(title, "", under, None, &ada()).await.unwrap();
    made["id"].as_str().unwrap().to_string()
}

/// The titles of the pages directly under the page called `parent` (the
/// wikis, for none), in the order they stand.
async fn under(rig: &Rig, parent: Option<&str>) -> Vec<String> {
    let pages = rig.topics.list(&ada()).await.unwrap();
    let lines: Vec<Line> = pages
        .iter()
        .map(|page| Line {
            id: page["id"].as_str().unwrap().to_string(),
            parent: page["data"]["parent"].as_str().map(str::to_string),
            position: page["position"].as_f64().unwrap(),
            space: None,
        })
        .collect();
    let tree = Tree::read(&lines);
    let title = |id: &str| -> String {
        let page = pages.iter().find(|page| page["id"] == json!(id)).unwrap();
        page["data"]["title"].as_str().unwrap().to_string()
    };
    let parent = parent.map(|named| {
        let page = pages.iter().find(|page| page["data"]["title"] == json!(named)).unwrap();
        page["id"].as_str().unwrap().to_string()
    });
    tree.beside(parent.as_deref()).iter().map(|line| title(&line.id)).collect()
}

/// What the Trash keeps, every row whole.
async fn trash(journal: &Journal) -> Vec<studio::trash::Trashed> {
    let mut whole = Vec::new();
    for listed in studio::trash::rows(journal, None).await.unwrap() {
        whole.extend(studio::trash::rows(journal, Some(listed.id)).await.unwrap());
    }
    whole
}

async fn revisions(rig: &Rig, id: &str) -> i64 {
    let id = id.to_string();
    rig.journal
        .db()
        .call(move |connection| {
            connection
                .query_row("SELECT count(*) FROM topic_revisions WHERE topic = ?1", [&id], |row| row.get(0))
        })
        .await
        .unwrap()
}

/// A wiki with a page under it holding two pages, and another page beside it.
async fn handbook(rig: &Rig) -> (String, String, String, String) {
    rig.topics.make("Handbook", "", (None, None), None, &ada()).await.unwrap();
    let started = make(rig, "Getting started", "Handbook").await;
    let deploying = make(rig, "Deploying", "Handbook").await;
    let staging = make(rig, "To staging", &deploying).await;
    let back = make(rig, "Rolling back", &staging).await;
    (started, deploying, staging, back)
}

#[tokio::test]
async fn a_page_deleted_takes_every_page_under_it_into_one_entry_of_the_trash_in_one_write() {
    let rig = Rig::new().await;
    let (_, deploying, staging, back) = handbook(&rig).await;
    let gone = rig.studio.records.removing(&deploying, &ada(), None).await.unwrap();
    assert_eq!(gone.with, 2, "the two pages under it, at any depth");
    assert_eq!(under(&rig, Some("Handbook")).await, ["Getting started"]);
    assert_eq!(rig.topics.list(&ada()).await.unwrap().len(), 2, "none of the three left behind as a wiki");
    for id in [&deploying, &staging, &back] {
        assert!(rig.studio.collections.record(id).await.is_err(), "{id} is no record");
    }
    let kept = trash(&rig.journal).await;
    assert_eq!(kept.len(), 1, "one entry for the whole of it");
    assert_eq!(
        (kept[0].thing.as_str(), kept[0].called.as_str(), kept[0].with),
        (deploying.as_str(), "Deploying", 2)
    );
    let listed = studio::trash::rows(&rig.journal, None).await.unwrap();
    assert_eq!(listed[0].with, 2, "a listing says how many, reading nothing of them");
    let went: Vec<&str> =
        kept[0].kept["with"].as_array().unwrap().iter().map(|one| one["id"].as_str().unwrap()).collect();
    assert_eq!(went, [staging.as_str(), back.as_str()], "each before the pages under it");

    let said = rig.events("record.removed,trash.put").await;
    let topics: Vec<(&str, &str)> = said
        .iter()
        .map(|event| {
            (event.topic.as_str(), event.data["thing"].as_str().or(event.data["id"].as_str()).unwrap())
        })
        .collect();
    assert_eq!(
        topics,
        [
            ("record.removed", deploying.as_str()),
            ("record.removed", staging.as_str()),
            ("record.removed", back.as_str()),
            ("trash.put", deploying.as_str())
        ]
    );
    assert!(said.iter().all(|event| event.ts == said[0].ts), "said in the one write");
    assert_eq!(said[1].data["with"], json!(deploying));
    assert_eq!(said[1].data["trash"], json!(kept[0].id));
    assert_eq!(said[1].data["into"], json!("the Trash"));

    // A page made now is numbered after every one in the Trash, those that went with another too.
    let next = make(&rig, "Next", "Handbook").await;
    let number = |id: &str| id.rsplit('-').next().unwrap().parse::<u64>().unwrap();
    assert!(number(&next) > number(&back), "{next} after {back}");
}

#[tokio::test]
async fn given_back_a_page_is_under_its_page_again_with_every_page_that_went_with_it_in_one_write() {
    let rig = Rig::new().await;
    let (_, deploying, _, _) = handbook(&rig).await;
    rig.studio.records.remove(&deploying, &ada(), None).await.unwrap();
    let entry = trash(&rig.journal).await.remove(0);
    let mark = rig.events("record.created,trash.restored").await.len();
    let said = rig.studio.collections.restore_record(&rig.journal, &entry, &ada()).await.unwrap();
    assert_eq!(said, None, "where it was, which says nothing more");
    assert_eq!(under(&rig, Some("Handbook")).await, ["Getting started", "Deploying"]);
    assert_eq!(under(&rig, Some("Deploying")).await, ["To staging"]);
    assert_eq!(under(&rig, Some("To staging")).await, ["Rolling back"]);
    assert!(trash(&rig.journal).await.is_empty());
    let after = rig.events("record.created,trash.restored").await;
    let created: Vec<&Value> = after[mark..]
        .iter()
        .filter(|event| event.topic == "record.created")
        .map(|event| &event.data)
        .collect();
    assert_eq!(created.len(), 3, "{created:?}");
    assert!(created.iter().all(|data| data["restored"] == json!(true)));
    assert_eq!(created[1]["with"], json!(deploying));
    let restored = after[mark..].iter().find(|event| event.topic == "trash.restored").unwrap();
    assert_eq!(restored.data["with"], json!(2));
    assert!(after[mark..].iter().all(|event| event.ts == restored.ts), "in one write");
}

#[tokio::test]
async fn a_page_given_back_whose_page_is_gone_or_in_another_workspace_is_a_wiki_of_its_own_and_says_so() {
    let rig = Rig::new().await;
    let (_, deploying, _, _) = handbook(&rig).await;
    rig.studio.records.remove(&deploying, &ada(), None).await.unwrap();
    let entry = trash(&rig.journal).await.remove(0);
    let wiki = rig.topics.find("Handbook", &ada()).await.unwrap().id;
    rig.studio.records.remove(&wiki, &ada(), None).await.unwrap();
    let said = rig.studio.collections.restore_record(&rig.journal, &entry, &ada()).await.unwrap();
    assert_eq!(
        said.as_deref(),
        Some("The page `Deploying` was under is gone, so `Deploying` is a wiki of its own now.")
    );
    assert_eq!(under(&rig, None).await, ["Deploying"]);
    assert_eq!(
        under(&rig, Some("To staging")).await,
        ["Rolling back"],
        "what went with it came back under it"
    );
    let read = rig.topics.read("Deploying", &ada()).await.unwrap();
    assert_eq!(read["topic"]["data"]["parent"], Value::Null);
    assert_eq!(read["topic"]["data"]["workspace"], json!("ada-s-workspace"), "where it was");

    // A page whose wiki went to another workspace meanwhile.
    let rig = Rig::new().await;
    let (_, deploying, _, _) = handbook(&rig).await;
    rig.studio.records.remove(&deploying, &ada(), None).await.unwrap();
    let entry = trash(&rig.journal).await.remove(0);
    let research = rig.studio.workspaces.create("Research", &ada()).await.unwrap();
    rig.topics.place("Handbook", (Some(&research.id), None), &ada()).await.unwrap();
    let said = rig.studio.collections.restore_record(&rig.journal, &entry, &ada()).await.unwrap();
    assert!(said.as_deref().unwrap_or_default().contains("is in another workspace now"), "{said:?}");
    assert_eq!(under(&rig, None).await, ["Handbook", "Deploying"]);
    assert_eq!(under(&rig, Some("Handbook")).await, ["Getting started"]);

    // And one whose wiki went to a project of its workspace is under it, of that place.
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let (_, deploying, staging, _) = handbook(&rig).await;
    rig.studio.records.remove(&deploying, &ada(), None).await.unwrap();
    let entry = trash(&rig.journal).await.remove(0);
    rig.topics.place("Handbook", (None, Some("shop")), &ada()).await.unwrap();
    let said = rig.studio.collections.restore_record(&rig.journal, &entry, &ada()).await.unwrap();
    assert_eq!(said, None);
    assert_eq!(under(&rig, Some("Handbook")).await, ["Getting started", "Deploying"]);
    for page in [deploying, staging] {
        let read = rig.topics.read(&page, &ada()).await.unwrap();
        assert_eq!(read["topic"]["data"]["project"], json!("shop"), "{page} is where its wiki is");
        assert_eq!(read["topic"]["data"]["workspace"], Value::Null);
    }
}

#[tokio::test]
async fn where_nothing_deleted_is_kept_a_page_goes_at_once_with_every_page_under_it_and_their_revisions() {
    let rig = Rig::new().await;
    let (_, deploying, staging, back) = handbook(&rig).await;
    let body = |words: &str| serde_json::Map::from_iter([("body".to_string(), json!(words))]);
    for id in [&deploying, &back] {
        rig.studio.records.update(id, body("Written."), &ada(), None).await.unwrap();
    }
    assert!(revisions(&rig, &back).await > 0);
    studio::trash::keep_for(&rig.journal, Some(0), &ada()).await.unwrap();
    let gone = rig.studio.records.removing(&deploying, &ada(), None).await.unwrap();
    assert_eq!((gone.trash, gone.with), (None, 2));
    assert!(trash(&rig.journal).await.is_empty());
    assert_eq!(under(&rig, Some("Handbook")).await, ["Getting started"]);
    assert_eq!(rig.topics.list(&ada()).await.unwrap().len(), 2);
    for id in [&deploying, &staging, &back] {
        assert_eq!(revisions(&rig, id).await, 0, "{id}'s revisions went with it");
    }
    let said = rig.events("record.removed").await;
    assert_eq!(said.len(), 3);
    assert!(said[1].data.get("trash").is_none(), "{:?}", said[1].data);
}
