//! Topics as wikis, pages in a tree: a page made under
//! another, moved within its wiki, to the top and under another wiki of
//! its workspace, each in one write and said; refused under itself, below
//! itself, beside a page it is not going beside and into another
//! workspace's wiki; staying put writing nothing; and every page of its
//! wiki's place after every move, place and settle.

use kernel::ErrorKind;
use kernel::topics::{Line, Place, Tree};
use serde_json::{Map, Value, json};

use crate::{Rig, ada};

/// Every page as the tree reads it: its id, title, parent and position.
async fn pages(rig: &Rig) -> Vec<Value> {
    rig.topics.list(&ada()).await.unwrap()
}

/// The titles of the pages directly under `parent` (the wikis, for none),
/// in the order they stand.
async fn under(rig: &Rig, parent: Option<&str>) -> Vec<String> {
    let pages = pages(rig).await;
    let lines = lines(&pages);
    let tree = Tree::read(&lines);
    let title = |id: &str| {
        pages.iter().find(|page| page["id"] == json!(id)).unwrap()["data"]["title"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let parent = parent.map(|named| pages.iter().find(|page| page["data"]["title"] == json!(named)).unwrap());
    let parent = parent.map(|page| page["id"].as_str().unwrap().to_string());
    tree.beside(parent.as_deref()).iter().map(|line| title(&line.id)).collect()
}

fn lines(pages: &[Value]) -> Vec<Line> {
    pages
        .iter()
        .map(|page| Line {
            id: page["id"].as_str().unwrap().to_string(),
            parent: page["data"]["parent"].as_str().map(str::to_string),
            position: page["position"].as_f64().unwrap(),
            space: None,
        })
        .collect()
}

/// That every page says its wiki's place, whatever was done.
async fn each_of_its_wikis_place(rig: &Rig) {
    let pages = pages(rig).await;
    let lines = lines(&pages);
    let tree = Tree::read(&lines);
    let place = |id: &str| {
        let page = pages.iter().find(|page| page["id"] == json!(id)).unwrap();
        Place::of(page["data"].as_object().unwrap())
    };
    for page in &pages {
        let id = page["id"].as_str().unwrap();
        let wiki = tree.wiki(id).unwrap();
        assert_eq!(place(id), place(wiki), "{id} is where its wiki {wiki} is");
        assert!(place(id).is_some(), "{id} is somewhere");
    }
}

async fn make(rig: &Rig, title: &str, under: &str) -> String {
    let made = rig.topics.make_under(title, "", under, None, &ada()).await.unwrap();
    made["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn a_page_made_under_another_is_of_its_wikis_place_and_stands_after_every_page() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let handbook = rig.topics.make("Handbook", "", (None, Some("shop")), None, &ada()).await.unwrap();
    let deploying = make(&rig, "Deploying", "Handbook").await;
    let staging = rig.topics.make_under("To staging", "Run it.", &deploying, None, &ada()).await.unwrap();
    assert_eq!(staging["data"]["parent"], json!(deploying));
    assert_eq!(
        (staging["data"]["project"].clone(), staging["data"]["workspace"].clone()),
        (json!("shop"), Value::Null),
        "a page is where its wiki is"
    );
    assert!(staging["position"].as_f64() > handbook["position"].as_f64());
    assert_eq!(under(&rig, Some("Handbook")).await, ["Deploying"]);
    assert_eq!(under(&rig, Some("Deploying")).await, ["To staging"]);
    let read = rig.topics.read("To staging", &ada()).await.unwrap();
    let path: Vec<&str> =
        read["path"].as_array().unwrap().iter().map(|one| one["title"].as_str().unwrap()).collect();
    assert_eq!(path, ["Handbook", "Deploying"], "{read}");
    let held = rig.topics.read("Deploying", &ada()).await.unwrap();
    assert_eq!(held["pages"], json!([{ "id": staging["id"], "title": "To staging", "status": null }]));

    let nothing = rig.topics.make_under("Nowhere", "", "Nothing like it", None, &ada()).await.unwrap_err();
    assert_eq!(nothing.kind, ErrorKind::NotFound);
    each_of_its_wikis_place(&rig).await;
}

#[tokio::test]
async fn a_page_moves_within_its_wiki_to_the_top_and_under_another_wiki_in_one_write_said() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    rig.topics.make("Handbook", "", (None, None), None, &ada()).await.unwrap();
    rig.topics.make("Payments", "", (None, Some("shop")), None, &ada()).await.unwrap();
    make(&rig, "Getting started", "Handbook").await;
    let deploying = make(&rig, "Deploying", "Handbook").await;
    make(&rig, "To staging", &deploying).await;
    make(&rig, "Rolling back", &deploying).await;
    assert_eq!(under(&rig, Some("Handbook")).await, ["Getting started", "Deploying"]);

    // Before another of its level.
    rig.topics.move_to("Deploying", Some("Handbook"), Some("Getting started"), &ada()).await.unwrap();
    assert_eq!(under(&rig, Some("Handbook")).await, ["Deploying", "Getting started"]);
    rig.topics.move_to("Rolling back", Some("Deploying"), Some("To staging"), &ada()).await.unwrap();
    assert_eq!(under(&rig, Some("Deploying")).await, ["Rolling back", "To staging"]);
    let moved = rig.events("topic.moved").await;
    assert_eq!(moved.len(), 2);
    assert_eq!(moved[0].data["parent"], json!(rig.topics.find("Handbook", &ada()).await.unwrap().id));
    assert_eq!(moved[0].data["was"]["parent"], moved[0].data["parent"], "it was under the same page");
    assert_eq!(moved[0].data["by"], json!("user:ada"));
    assert_eq!(moved[0].source.actor.as_deref(), Some("user:ada"));

    // Under another wiki of the workspace, a project's: every page under it goes with it.
    let before = rig.events("topic.placed").await.len();
    let came = rig.topics.move_to("Deploying", Some("Payments"), None, &ada()).await.unwrap();
    assert_eq!(came["data"]["project"], json!("shop"), "{came}");
    assert_eq!(under(&rig, Some("Payments")).await, ["Deploying"]);
    assert_eq!(
        under(&rig, Some("Deploying")).await,
        ["Rolling back", "To staging"],
        "its pages came with it"
    );
    let placed = rig.events("topic.placed").await;
    assert_eq!(placed.len() - before, 3, "said of each page whose place changed");
    assert!(placed[before..].iter().all(|event| event.data["project"] == json!("shop")));
    each_of_its_wikis_place(&rig).await;

    // To the top, where it is a wiki of its own and keeps its place.
    let top = rig.topics.move_to("Rolling back", None, None, &ada()).await.unwrap();
    assert_eq!(top["data"]["parent"], Value::Null);
    assert_eq!(top["data"]["project"], json!("shop"));
    assert_eq!(under(&rig, None).await, ["Handbook", "Payments", "Rolling back"]);
    let last = rig.events("topic.moved").await;
    assert_eq!(last.last().unwrap().data["parent"], Value::Null);
    // And a wiki under a page: a page of that page's wiki.
    rig.topics.move_to("Handbook", Some("To staging"), None, &ada()).await.unwrap();
    assert_eq!(under(&rig, Some("To staging")).await, ["Handbook"]);
    assert_eq!(under(&rig, Some("Handbook")).await, ["Getting started"]);
    each_of_its_wikis_place(&rig).await;
}

#[tokio::test]
async fn a_move_under_itself_below_itself_beside_nothing_or_into_another_workspace_is_refused_and_writes_nothing()
 {
    let rig = Rig::new().await;
    rig.studio.workspaces.create("Research", &ada()).await.unwrap();
    rig.topics.make("Handbook", "", (None, None), None, &ada()).await.unwrap();
    rig.topics.make("Papers", "", (Some("research"), None), None, &ada()).await.unwrap();
    let deploying = make(&rig, "Deploying", "Handbook").await;
    make(&rig, "To staging", &deploying).await;
    make(&rig, "Getting started", "Handbook").await;
    let written = rig.events("record.*").await.len();
    assert!(written >= 5, "each page made was said: {written}");

    let refused = |error: kernel::Error| (error.kind, error.message);
    let itself = rig.topics.move_to("Deploying", Some("Deploying"), None, &ada()).await.unwrap_err();
    let (kind, said) = refused(itself);
    assert_eq!(kind, ErrorKind::Invalid);
    assert!(said.contains("`Deploying` cannot go under itself or a page that is under it"), "{said}");
    let below = rig.topics.move_to("Handbook", Some("To staging"), None, &ada()).await.unwrap_err();
    assert!(below.message.contains("cannot go under itself"), "{}", below.message);
    let owner = ada();
    let beside = rig.topics.move_to("Getting started", Some("Handbook"), Some("To staging"), &owner);
    let beside = beside.await.unwrap_err();
    assert!(
        beside.message.contains("`To staging` is not beside where `Getting started` is to go"),
        "{}",
        beside.message
    );
    let elsewhere = rig.topics.move_to("Deploying", Some("Papers"), None, &ada()).await.unwrap_err();
    assert!(
        elsewhere.message.contains("`Papers` is a page of a wiki of another workspace")
            && elsewhere.message.contains("topic move <wiki> --workspace W"),
        "{}",
        elsewhere.message
    );
    // Where it is, nothing is written and nothing said.
    rig.topics.move_to("Deploying", Some("Handbook"), Some("Getting started"), &ada()).await.unwrap();
    rig.topics.move_to("Handbook", None, Some("Papers"), &ada()).await.unwrap();
    assert_eq!(rig.events("record.*").await.len(), written, "nothing was written");
    assert!(rig.events("topic.moved").await.is_empty(), "nothing was said");
}

#[tokio::test]
async fn a_wiki_is_put_somewhere_else_with_its_pages_and_a_page_is_where_its_wiki_is() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    rig.topics.make("Handbook", "", (None, None), None, &ada()).await.unwrap();
    let deploying = make(&rig, "Deploying", "Handbook").await;
    make(&rig, "To staging", &deploying).await;
    let placed = rig.topics.place("Handbook", (None, Some("shop")), &ada()).await.unwrap();
    assert_eq!(placed["data"]["project"], json!("shop"));
    assert_eq!(rig.events("topic.placed").await.len(), 3, "the wiki and each page under it");
    each_of_its_wikis_place(&rig).await;
    let page = rig.topics.place("Deploying", (Some("ada-s-workspace"), None), &ada()).await.unwrap_err();
    assert_eq!(page.kind, ErrorKind::Invalid);
    assert!(page.message.contains("`Deploying` is a page of the wiki `Handbook`"), "{}", page.message);
    assert!(page.message.contains("--top"), "{}", page.message);
    each_of_its_wikis_place(&rig).await;
}

#[tokio::test]
async fn a_wiki_whose_project_is_gone_is_settled_with_its_pages_and_a_page_astray_is_brought_to_its_wiki() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    rig.topics.make("Handbook", "", (None, Some("shop")), None, &ada()).await.unwrap();
    let deploying = make(&rig, "Deploying", "Handbook").await;
    make(&rig, "To staging", &deploying).await;
    rig.studio.projects.forget("shop", &ada()).await.unwrap();
    assert_eq!(rig.topics.settle(None).await.unwrap(), 3, "the wiki and its two pages");
    let staging = rig.topics.find("To staging", &ada()).await.unwrap();
    assert_eq!(staging.data["workspace"], json!("ada-s-workspace"));
    each_of_its_wikis_place(&rig).await;
    assert_eq!(rig.topics.settle(None).await.unwrap(), 0, "everything is where it is to be");
}

#[tokio::test]
async fn only_the_keeper_writes_a_pages_place_in_the_tree_and_a_change_by_any_other_door_keeps_it() {
    let rig = Rig::new().await;
    rig.topics.make("Handbook", "", (None, None), None, &ada()).await.unwrap();
    let page = make(&rig, "Deploying", "Handbook").await;
    let was = rig.topics.find(&page, &ada()).await.unwrap();
    let mut body = Map::new();
    body.insert("body".into(), json!("# How"));
    rig.studio.records.update(&page, body, &ada(), None).await.unwrap();
    let now = rig.topics.find(&page, &ada()).await.unwrap();
    assert_eq!((now.position, now.data["parent"].clone()), (was.position, was.data["parent"].clone()));
    let mut parent = Map::new();
    parent.insert("parent".into(), json!(null));
    let refused = rig.studio.records.update(&page, parent, &ada(), None).await.unwrap_err();
    assert!(refused.message.contains("`parent` of a `topic` is written by"), "{}", refused.message);
    assert!(refused.message.contains("`topic make --under`"), "{}", refused.message);
}

#[tokio::test]
async fn two_moves_at_once_that_would_make_a_ring_are_one_moved_and_one_refused() {
    let rig = Rig::new().await;
    rig.topics.make("One", "", (None, None), None, &ada()).await.unwrap();
    rig.topics.make("Two", "", (None, None), None, &ada()).await.unwrap();
    let owner = ada();
    let (first, second) = tokio::join!(
        rig.topics.move_to("One", Some("Two"), None, &owner),
        rig.topics.move_to("Two", Some("One"), None, &owner),
    );
    assert!(first.is_ok() != second.is_ok(), "one of them was moved: {first:?} {second:?}");
    let refused = first.err().or(second.err()).unwrap();
    assert!(refused.message.contains("cannot go under itself"), "{}", refused.message);
    assert_eq!(under(&rig, None).await.len(), 1, "one wiki, with the other under it");
    each_of_its_wikis_place(&rig).await;
}
