//! A topic as the factory keeps it: each change one write with its
//! revision and what its status came to, whatever door it came by; its
//! own cells written through its own doors; its replies nested; and each
//! topic somewhere there is.

use kernel::ErrorKind;
use serde_json::{Map, Value, json};

use crate::{Rig, ada};

fn data(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap()
}

/// The replies of a topic once there are `lines` lines of its history.
async fn read_once(rig: &Rig, topic: &str, lines: usize) -> Value {
    let mut written = rig.journal.follow(0, studio::history::TOPIC);
    loop {
        let id = rig.topics.find(topic, &ada()).await.unwrap().id;
        if rig.studio.collections.history(&id).await.unwrap().len() >= lines {
            return rig.topics.read(topic, &ada()).await.unwrap();
        }
        written.next().await.unwrap();
    }
}

#[tokio::test]
async fn a_topic_made_is_its_first_revision_and_each_change_of_what_it_says_another_by_any_door() {
    let rig = Rig::new().await;
    let made =
        rig.topics.make("Payment provider options", "# Which", (None, None), None, &ada()).await.unwrap();
    let id = made["id"].as_str().unwrap().to_string();
    assert!(id.starts_with("@-topics-"), "{id}");
    assert_eq!(
        (made["data"]["status"].clone(), made["data"]["workspace"].clone()),
        (Value::Null, json!("ada-s-workspace")),
        "made with nothing said, it has no status"
    );
    // Changed at the door any record is changed by, as a table's row is.
    let records = &rig.studio.records;
    records.update(&id, data(json!({ "body": "# Which, and why" })), &ada(), None).await.unwrap();
    records.update(&id, data(json!({ "status": "closed" })), &ada(), None).await.unwrap();
    records.update(&id, data(json!({ "summary": "Two are left." })), &ada(), None).await.unwrap();
    let revisions = rig.topics.revisions(&id, &ada()).await.unwrap();
    let said: Vec<(i64, &str, &str)> = revisions
        .iter()
        .map(|one| {
            (one["nth"].as_i64().unwrap(), one["body"].as_str().unwrap(), one["summary"].as_str().unwrap())
        })
        .collect();
    assert_eq!(
        said,
        [(1, "# Which", ""), (2, "# Which, and why", ""), (3, "# Which, and why", "Two are left.")]
    );
    let revised = rig.events("topic.revised").await;
    assert_eq!(revised.len(), 3, "a change of its status is no revision");
    assert_eq!(revised[2].data["nth"], json!(3));
    let closed = rig.events("topic.closed").await;
    assert_eq!(closed.len(), 1);
    assert_eq!(closed[0].data["workspace"], json!("ada-s-workspace"));
    assert!(rig.events("topic.opened").await.is_empty(), "made with no status, nothing was said of it");
    // A record of the factory's own place is said with it, and the event is the factory's.
    let created = rig.events("record.created").await;
    assert_eq!(created.last().unwrap().data["project"], json!("@"));
}

#[tokio::test]
async fn where_a_topic_is_and_who_follows_it_are_written_through_its_own_doors_and_refused_at_any_other() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let made = rig.topics.make("Why the build is slow", "", (None, None), None, &ada()).await.unwrap();
    let id = made["id"].as_str().unwrap();
    let refused =
        rig.studio.records.update(id, data(json!({ "project": "shop" })), &ada(), None).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Invalid);
    assert!(
        refused.message.contains("`project` of a `topic` is written by the topic's own commands"),
        "{}",
        refused.message
    );
    let owner = ada();
    let added =
        rig.studio.records.add("@", "topics", data(json!({ "title": "X", "workspace": "w" })), &owner, None);
    assert!(added.await.unwrap_err().message.contains("`workspace` of a `topic`"));

    let moved = rig.topics.place("why the BUILD is slow", (None, Some("shop")), &ada()).await.unwrap();
    assert_eq!(
        (moved["data"]["project"].clone(), moved["data"]["workspace"].clone()),
        (json!("shop"), Value::Null)
    );
    let placed = rig.events("topic.placed").await;
    assert_eq!(placed.len(), 1);
    assert_eq!(
        (placed[0].data["project"].clone(), placed[0].data["was"].clone()),
        (json!("shop"), json!("workspace:ada-s-workspace"))
    );
    // Put where it is, nothing is written or said.
    rig.topics.place(id, (None, Some("shop")), &ada()).await.unwrap();
    assert_eq!(rig.events("topic.placed").await.len(), 1);

    let followed = rig.topics.follow(id, true, &ada()).await.unwrap();
    assert_eq!(followed["data"]["followers"], json!(["user:ada"]));
    rig.topics.follow(id, true, &ada()).await.unwrap();
    let unfollowed = rig.topics.follow(id, false, &ada()).await.unwrap();
    assert_eq!(unfollowed["data"]["followers"], json!([]));
    assert_eq!(rig.events("topic.followed").await.len(), 2, "following what one follows says nothing");
    assert_eq!(rig.topics.revisions(id, &ada()).await.unwrap().len(), 1, "none of it is a revision");
}

#[tokio::test]
async fn a_reply_goes_under_what_it_answers_and_beside_it_eight_deep_and_one_answers_the_topic() {
    let rig = Rig::new().await;
    rig.topics.make("Which store", "", (None, None), Some("open"), &ada()).await.unwrap();
    assert_eq!(rig.events("topic.opened").await.len(), 1, "made open, it is said so");
    let mut parent = None;
    let mut said = Vec::new();
    for depth in 1..=9 {
        let seq = rig.topics.reply("Which store", &format!("reply {depth}"), parent, &ada()).await.unwrap();
        said.push(seq);
        parent = Some(seq);
    }
    let read = read_once(&rig, "Which store", 10).await;
    let replies = read["replies"].as_array().unwrap();
    assert_eq!(replies.len(), 9);
    assert_eq!(replies[0]["parent"], Value::Null);
    assert_eq!(replies[1]["parent"], json!(said[0]));
    assert_eq!(replies[7]["parent"], json!(said[6]), "the eighth is under the seventh");
    assert_eq!(replies[8]["parent"], json!(said[6]), "the ninth is beside the eighth");

    let none = rig.topics.reply("Which store", "nothing", Some(1), &ada()).await.unwrap_err();
    assert_eq!(none.kind, ErrorKind::NotFound);
    let answered = rig.topics.answered("Which store", Some(said[2]), &ada()).await.unwrap();
    assert_eq!(
        (answered["data"]["status"].clone(), answered["data"]["answer"].clone()),
        (json!("answered"), json!(said[2].to_string()))
    );
    let event = rig.events("topic.answered").await;
    assert_eq!(event[0].data["answer"], json!(said[2].to_string()));
    let opened = rig.topics.answered("Which store", None, &ada()).await.unwrap();
    assert_eq!(opened["data"]["status"], json!("open"));
    assert_eq!(rig.events("topic.opened").await.len(), 2);
}

#[tokio::test]
async fn a_topic_of_a_project_or_workspace_that_is_gone_is_made_the_own_of_where_its_things_went() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    rig.studio.workspaces.create("Research", &ada()).await.unwrap();
    rig.studio.workspaces.create("Later", &ada()).await.unwrap();
    rig.topics.make("Of shop", "", (None, Some("shop")), None, &ada()).await.unwrap();
    rig.topics.make("Of research", "", (Some("research"), None), None, &ada()).await.unwrap();
    assert_eq!(rig.topics.settle(None).await.unwrap(), 0, "everything is somewhere there is");

    rig.studio.projects.forget("shop", &ada()).await.unwrap();
    rig.studio.workspaces.remove("research", Some("later"), &ada()).await.unwrap();
    assert_eq!(rig.topics.settle(Some(("research", "later"))).await.unwrap(), 2);
    let shop = rig.topics.find("Of shop", &ada()).await.unwrap();
    assert_eq!(shop.data["workspace"], json!("ada-s-workspace"));
    let research = rig.topics.find("Of research", &ada()).await.unwrap();
    assert_eq!(research.data["workspace"], json!("later"));
    let placed = rig.events("topic.placed").await;
    assert!(placed.iter().all(|event| event.source.actor.as_deref() == Some("system")), "said as the system");
}
