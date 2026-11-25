//! A channel moved to the Trash with everything said and attached in it,
//! seen there by whoever could read it, given back whole and deleted for
//! good.

use kernel::principal::MEMBER_SCOPES;
use kernel::trash::Kind;
use kernel::{Principal, PrincipalKind};
use studio::Channels;
use talk::Place;
use talk::inbox::Tell;

use crate::{Rig, ada};

fn person(id: &str, name: &str, scopes: &[&str]) -> Principal {
    Principal {
        kind: PrincipalKind::User,
        id: id.into(),
        name: name.into(),
        scopes: scopes.iter().map(|scope| scope.to_string()).collect(),
        key: None,
    }
}

fn member(id: &str, name: &str) -> Principal {
    person(id, name, MEMBER_SCOPES)
}

async fn counted(rig: &Rig, table: &'static str) -> i64 {
    let asked = move |connection: &mut journal::copy::Connection| {
        connection.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))
    };
    rig.journal.db().call(asked).await.unwrap()
}

/// `general`, with Bo in it, a file attached to the first of two things
/// said, and a line of Bo's inbox that opens it: its id, and the file's.
async fn spoken_in(rig: &Rig) -> (String, String) {
    let channels = &rig.studio.channels;
    let made = channels.create("general", None, "", &ada()).await.unwrap();
    channels.join(&Channels::daemon("a test"), &made.id, "user:bo", "Bo").await.unwrap();
    let reader = channels.may_write(&made.id, &ada()).await.unwrap();
    let by = ada();
    let attached = channels.attachments.add(&reader, &made.id, &by, "plan.txt", None, b"quince".to_vec());
    let file = attached.await.unwrap().id;
    let with = [file.clone()];
    let place = Place { attachments: &with, ..Place::default() };
    let first = channels.post_at(&made.id, &ada(), "the marmalade ledger", place).await.unwrap();
    channels.post(&made.id, &ada(), "second thoughts", None).await.unwrap();
    channels.inbox.tell(Tell::named("user:bo", &first.id, &made.id, "user:ada")).await.unwrap();
    (made.id, file)
}

#[tokio::test]
async fn a_channel_moved_to_the_trash_takes_everything_of_it_and_is_given_back_whole() {
    let rig = Rig::new().await;
    let (id, file) = spoken_in(&rig).await;
    let channels = &rig.studio.channels;
    let kept = channels.attachments.path(&file);
    let bo = member("bo", "Bo");

    let (gone, trash) = rig.trash.delete_channel("general", &ada()).await.unwrap();
    let trash = trash.expect("into the Trash");
    assert_eq!(gone.id, id);
    let own = Channels::daemon("a test");
    assert!(channels.get(&own, &id).await.is_err());
    assert!(!kept.exists(), "what was attached left the channels' folder");
    for table in ["channels", "channel_members", "messages", "attachments", "inbox"] {
        assert_eq!(counted(&rig, table).await, 0, "{table}");
    }
    assert!(channels.inbox.list(&bo, true).await.unwrap().is_empty());
    let topics = rig.topics("channel.removed, trash.put").await;
    assert_eq!(topics, ["trash.put", "channel.removed"]);
    let removed = rig.events("channel.removed").await.remove(0);
    assert_eq!(
        (removed.data["messages"].as_i64(), removed.data["trash"].as_str()),
        (Some(2), Some(trash.as_str()))
    );

    let listed = rig.trash.list(&ada()).await.unwrap();
    let entry = listed.iter().find(|entry| entry.id == trash).unwrap();
    assert_eq!(
        (entry.kind, entry.called.as_str(), entry.thing.as_str(), entry.may),
        (Kind::Channel, "#general", id.as_str(), true)
    );
    // A member reads channels and sees it there, and may not give it back:
    // taking a channel away is the owner's.
    let seen = rig.trash.list(&bo).await.unwrap();
    assert!(seen.iter().any(|entry| entry.id == trash && !entry.may));
    assert!(rig.trash.restore(&trash, false, &bo).await.is_err());

    let (_, at, _) = rig.trash.restore(&trash, false, &ada()).await.unwrap();
    assert_eq!(at, id);
    let back = channels.get(&own, &id).await.unwrap();
    let members: Vec<&str> = back.members.iter().map(|member| member.actor.as_str()).collect();
    assert_eq!((back.name.as_str(), members), ("general", vec!["user:ada", "user:bo"]));
    let reader = channels.may_read(&id, &ada()).await.unwrap();
    let said = channels.messages(&reader, &id, None, 10).await.unwrap();
    let bodies: Vec<&str> = said.iter().map(|message| message.body.as_str()).collect();
    assert_eq!(bodies, ["the marmalade ledger", "second thoughts"]);
    assert_eq!(said[0].attachments.len(), 1);
    assert_eq!(std::fs::read(&kept).unwrap(), b"quince");
    assert_eq!(channels.inbox.list(&bo, true).await.unwrap().len(), 1);
    assert!(rig.trash.list(&ada()).await.unwrap().is_empty());
    let restored = rig.events("channel.created").await.pop().unwrap();
    assert_eq!(restored.data["restored"], true);
    assert!(
        rig.topics("inbox.told, trash.restored")
            .await
            .ends_with(&["inbox.told".to_string(), "trash.restored".to_string()])
    );
}

#[tokio::test]
async fn a_private_channel_is_seen_in_the_trash_by_its_readers_alone_and_comes_back_private_to_them() {
    let rig = Rig::new().await;
    let channels = &rig.studio.channels;
    let (bo, carol) = (member("bo", "Bo"), member("carol", "Carol"));
    let plans = channels.create_kept("Winter plans", None, "", &bo).await.unwrap();
    channels.post(&plans.id, &bo, "the quince harvest", None).await.unwrap();

    let (_, trash) = rig.trash.delete_channel(&plans.id, &ada()).await.unwrap();
    let trash = trash.unwrap();
    // What is said of it going is for whoever was in it.
    let put = rig.events("trash.put").await.remove(0);
    let takes = put.source.takes.unwrap_or_default();
    assert!(takes.starts_with("member:") && takes.contains("user:bo"), "{takes}");
    assert!(!put.data.to_string().contains("Winter"), "{}", put.data);

    // Somebody who is not in it is shown nothing of it, and cannot give
    // it back, as for one that is not there; whoever was in it is.
    assert!(rig.trash.list(&carol).await.unwrap().is_empty());
    let refused = rig.trash.restore(&trash, false, &carol).await.unwrap_err();
    assert_eq!(refused.kind, kernel::ErrorKind::NotFound, "{}", refused.message);
    let seen = rig.trash.list(&bo).await.unwrap();
    assert_eq!(seen.iter().map(|entry| entry.called.as_str()).collect::<Vec<_>>(), ["Winter plans"]);

    rig.trash.restore(&trash, false, &ada()).await.unwrap();
    let back = channels.may_read(&plans.id, &bo).await.unwrap();
    assert_eq!(channels.messages(&back, &plans.id, None, 10).await.unwrap().len(), 1);
    let found = channels.get(&back, &plans.id).await.unwrap();
    assert!(found.kept && found.title.as_deref() == Some("Winter plans"));
    assert!(channels.may_read(&plans.id, &carol).await.is_err(), "kept to its members still");
}

#[tokio::test]
async fn a_key_that_reads_no_channels_neither_sees_one_in_the_trash_nor_gives_it_back() {
    let rig = Rig::new().await;
    spoken_in(&rig).await;
    let (_, trash) = rig.trash.delete_channel("general", &ada()).await.unwrap();
    let trash = trash.unwrap();
    let narrow = person("ada", "Ada", &["admin", "projects:read", "records"]);
    assert!(rig.trash.list(&narrow).await.unwrap().is_empty());
    let refused = rig.trash.restore(&trash, false, &narrow).await.unwrap_err();
    assert_eq!(refused.kind, kernel::ErrorKind::NotFound, "{}", refused.message);
    assert!(rig.trash.forget(&trash, &narrow).await.is_err());
    // And nobody but the owner takes one away.
    rig.trash.restore(&trash, false, &ada()).await.unwrap();
    assert!(rig.trash.delete_channel("general", &member("bo", "Bo")).await.is_err());
}

#[tokio::test]
async fn a_channel_whose_name_was_taken_meanwhile_comes_back_beside_it() {
    let rig = Rig::new().await;
    let (id, _) = spoken_in(&rig).await;
    let (_, trash) = rig.trash.delete_channel("general", &ada()).await.unwrap();
    let again = rig.studio.channels.create("general", None, "", &ada()).await.unwrap();
    rig.trash.restore(&trash.unwrap(), false, &ada()).await.unwrap();
    let own = Channels::daemon("a test");
    assert_eq!(rig.studio.channels.get(&own, &again.id).await.unwrap().name, "general");
    assert_eq!(rig.studio.channels.get(&own, &id).await.unwrap().name, "general-2");
}

#[tokio::test]
async fn a_channel_deleted_for_good_leaves_nothing_and_one_kept_no_days_goes_at_once() {
    let rig = Rig::new().await;
    let (_, file) = spoken_in(&rig).await;
    let (_, trash) = rig.trash.delete_channel("general", &ada()).await.unwrap();
    let trash = trash.unwrap();
    let folder = rig.dir.join("data").join(studio::trash::FOLDER).join(&trash);
    assert!(folder.join("kept").join(&file).exists(), "what was attached is kept with it in the Trash");
    rig.trash.forget(&trash, &ada()).await.unwrap();
    assert!(!folder.exists() && !rig.studio.channels.attachments.path(&file).exists());
    assert_eq!(counted(&rig, "trash").await, 0);
    assert!(rig.events("channel.message").await.is_empty());

    rig.trash.keep_for(Some(0), &ada()).await.unwrap();
    spoken_in(&rig).await;
    let (_, none) = rig.trash.delete_channel("general", &ada()).await.unwrap();
    assert!(none.is_none());
    assert_eq!((counted(&rig, "trash").await, counted(&rig, "messages").await), (0, 0));
}

#[tokio::test]
async fn a_channel_whose_project_is_gone_comes_back_the_own_of_no_project() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let channels = &rig.studio.channels;
    let made = channels.create("shop-talk", Some("shop"), "", &ada()).await.unwrap();
    channels.post(&made.id, &ada(), "the till is short", None).await.unwrap();
    let (_, trash) = rig.trash.delete_channel(&made.id, &ada()).await.unwrap();
    rig.studio.projects.forget("shop", &ada()).await.unwrap();
    rig.trash.restore(&trash.unwrap(), false, &ada()).await.unwrap();
    let back = channels.get(&Channels::daemon("a test"), &made.id).await.unwrap();
    assert_eq!(back.project, None);
    let reader = channels.may_read(&made.id, &ada()).await.unwrap();
    assert_eq!(channels.messages(&reader, &made.id, None, 10).await.unwrap().len(), 1);
}
