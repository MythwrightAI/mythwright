//! A workspace is a name for the projects that belong together, and for
//! the channels that are of no one project of them. It is grouping and
//! nothing else: nothing of a project or a channel changes by it but
//! where it is listed. A factory always has one, from when somebody owns
//! it, every project is always in one, and every channel is a project's
//! or a workspace's own.

use kernel::ErrorKind;
use studio::{Belongs, Settled};

use crate::{Rig, ada, agent};

#[tokio::test]
async fn a_factory_nobody_owns_has_no_workspace_and_one_is_made_for_its_owner_with_every_project_and_channel_in_it()
 {
    let rig = Rig::new().await;
    let shop = rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let talk = rig.studio.channels.create("shop", Some("shop"), "", &ada()).await.unwrap();
    let chat = rig.studio.channels.create("chat", None, "", &ada()).await.unwrap();
    // Nobody owns it: there is nobody to call a workspace after, and none is made.
    assert_eq!(rig.studio.settle().await.unwrap(), Settled::default());
    assert!(rig.studio.workspaces.list().await.unwrap().is_empty());
    assert_eq!(rig.studio.workspaces.usual().await.unwrap(), None);
    assert_eq!(rig.studio.workspaces.of_project("shop").await.unwrap(), None);

    // Somebody does: one is made, called after them, and the project that was in none is in it.
    rig.studio
        .identity
        .register("ada@example.com", " Ada Lovelace ", "correct horse", None, true)
        .await
        .unwrap();
    let settled = rig.studio.settle().await.unwrap();
    let id = "ada-lovelace-s-workspace";
    assert_eq!(
        settled,
        Settled { made: Some(id.into()), placed: vec!["shop".into()], channels: vec![chat.id.clone()] }
    );
    let all = rig.studio.workspaces.list().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(
        (all[0].id.as_str(), all[0].name.as_str(), all[0].usual),
        (id, "Ada Lovelace's workspace", true)
    );
    assert_eq!(all[0].projects, ["shop"]);
    // Each thing was said, once, as the system; and nothing else of anything changed.
    let made = rig.events("workspace.created").await;
    assert_eq!(made.len(), 1);
    assert_eq!((made[0].source.actor.as_deref(), &made[0].data["workspace"]), (Some("system"), &id.into()));
    let placed = rig.events("project.placed").await;
    assert_eq!(placed.len(), 1);
    assert_eq!(placed[0].source.actor.as_deref(), Some("system"));
    assert_eq!((&placed[0].data["project"], &placed[0].data["workspace"]), (&"shop".into(), &id.into()));
    assert_eq!(rig.studio.projects.get("shop").await.unwrap(), shop);
    // A project's channel is its project's still, and the one that
    // belonged to nothing is the workspace's own, said once as the system.
    assert_eq!(all[0].channels, std::slice::from_ref(&chat.id));
    assert_eq!(
        rig.studio.channels.get(&crate::own(), "shop").await.unwrap().project.as_deref(),
        Some("shop")
    );
    assert_eq!(rig.studio.workspaces.of_channel(&talk.id).await.unwrap(), None);
    assert_eq!(rig.studio.workspaces.of_channel(&chat.id).await.unwrap().as_deref(), Some(id));
    let placed = rig.events("channel.placed").await;
    assert_eq!(placed.len(), 1);
    assert_eq!(placed[0].source.actor.as_deref(), Some("system"));
    assert_eq!(
        (&placed[0].data["channel"], &placed[0].data["workspace"], &placed[0].data["project"]),
        (&chat.id.as_str().into(), &id.into(), &serde_json::Value::Null)
    );
    assert_eq!(rig.studio.channels.get(&crate::own(), "chat").await.unwrap(), chat);

    // Done again there is nothing to bring about: nothing is written and nothing said.
    assert_eq!(rig.studio.settle().await.unwrap(), Settled::default());
    assert_eq!(rig.studio.workspaces.settle(Some("Somebody else")).await.unwrap(), Settled::default());
    assert_eq!(rig.events("workspace.created").await.len() + rig.events("project.placed").await.len(), 2);
    assert_eq!(rig.events("channel.placed").await.len(), 1);
    assert_eq!(rig.studio.workspaces.list().await.unwrap(), all);
}

#[tokio::test]
async fn a_project_is_made_in_the_workspace_asked_for_or_in_the_one_made_first_that_is_still_there() {
    let rig = Rig::new().await;
    let workspaces = &rig.studio.workspaces;
    workspaces.settle(Some("Ada")).await.unwrap();
    workspaces.create("Annex", &ada()).await.unwrap();
    // The usual one is the one made first, whatever each is called.
    let all = workspaces.list().await.unwrap();
    let usual: Vec<(&str, bool)> = all.iter().map(|one| (one.id.as_str(), one.usual)).collect();
    assert_eq!(usual, [("ada-s-workspace", true), ("annex", false)]);
    workspaces.rename("ada-s-workspace", "Zed", &ada()).await.unwrap();
    let all = workspaces.list().await.unwrap();
    let usual: Vec<(&str, bool)> = all.iter().map(|one| (one.id.as_str(), one.usual)).collect();
    assert_eq!(usual, [("annex", false), ("ada-s-workspace", true)]);
    assert_eq!(workspaces.usual().await.unwrap().as_deref(), Some("ada-s-workspace"));

    // Nothing said: the usual one. Its place is written with its row, and said in that write.
    let shop = rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    assert_eq!(workspaces.of_project("shop").await.unwrap().as_deref(), Some("ada-s-workspace"));
    // One said: that one, and it was never in another.
    rig.studio.projects.create_in("Depot", None, &ada(), true, Some("annex")).await.unwrap();
    assert_eq!(workspaces.of_project("depot").await.unwrap().as_deref(), Some("annex"));
    // One that is not there: the usual one, since a project is always in one.
    rig.studio.projects.create_in("Shed", None, &ada(), true, Some("cellar")).await.unwrap();
    assert_eq!(workspaces.of_project("shed").await.unwrap().as_deref(), Some("ada-s-workspace"));
    let placed = rig.events("project.placed").await;
    let said: Vec<(&str, &str)> = placed
        .iter()
        .map(|event| (event.data["project"].as_str().unwrap(), event.data["workspace"].as_str().unwrap()))
        .collect();
    assert_eq!(said, [("shop", "ada-s-workspace"), ("depot", "annex"), ("shed", "ada-s-workspace")]);
    assert!(placed.iter().all(|event| event.data.get("was").is_none()));
    assert_eq!(rig.studio.projects.get("shop").await.unwrap(), shop);

    // The usual one taken away, the next that was made is the usual one.
    workspaces.remove("ada-s-workspace", Some("annex"), &ada()).await.unwrap();
    assert_eq!(workspaces.usual().await.unwrap().as_deref(), Some("annex"));
    rig.studio.projects.create("Yard", None, &ada()).await.unwrap();
    assert_eq!(workspaces.get("annex").await.unwrap().projects, ["depot", "shed", "shop", "yard"]);
}

#[tokio::test]
async fn a_project_whose_row_names_a_workspace_that_is_gone_is_in_none_and_is_placed_when_settled() {
    let rig = Rig::new().await;
    let workspaces = &rig.studio.workspaces;
    workspaces.settle(Some("Ada")).await.unwrap();
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    rig.studio.projects.create("Depot", None, &ada()).await.unwrap();
    // As a factory an earlier program made may hold them: one with no
    // row, and one whose row names a workspace nobody has any more.
    rig.journal
        .db()
        .call(|connection| {
            connection.execute("DELETE FROM workspace_projects WHERE project = 'shop'", [])?;
            connection.execute("UPDATE workspace_projects SET workspace = 'gone' WHERE project = 'depot'", [])
        })
        .await
        .unwrap();
    assert_eq!(workspaces.of_project("shop").await.unwrap(), None);
    assert_eq!(workspaces.of_project("depot").await.unwrap(), None);
    let before = rig.events("project.placed").await.len();
    let settled = workspaces.settle(Some("Ada")).await.unwrap();
    assert_eq!(settled, Settled { placed: vec!["depot".into(), "shop".into()], ..Settled::default() });
    assert_eq!(workspaces.get("ada-s-workspace").await.unwrap().projects, ["depot", "shop"]);
    assert_eq!(rig.events("project.placed").await.len(), before + 2);
    assert_eq!(workspaces.settle(None).await.unwrap(), Settled::default());
}

#[tokio::test]
async fn a_workspace_is_made_called_something_else_and_listed_by_name_with_what_is_in_it() {
    let rig = Rig::new().await;
    let made = rig.studio.workspaces.create("Shop Front", &ada()).await.unwrap();
    assert_eq!((made.id.as_str(), made.name.as_str()), ("shop-front", "Shop Front"));
    assert!(made.projects.is_empty() && made.channels.is_empty(), "{made:?}");
    rig.studio.workspaces.create("annex", &ada()).await.unwrap();
    // The same name again is refused, and says which there is.
    let again = rig.studio.workspaces.create("shop front", &ada()).await.unwrap_err();
    assert_eq!(again.kind, ErrorKind::Conflict, "{again}");
    assert!(again.message.contains("there is already a workspace called `shop-front`"), "{again}");
    let unnamed = rig.studio.workspaces.create(" ! ", &ada()).await.unwrap_err();
    assert_eq!(unnamed.kind, ErrorKind::Invalid, "{unnamed}");

    // By name, whichever was made first; one with nothing in it is listed.
    let listed = |all: Vec<studio::Workspace>| all.into_iter().map(|one| one.id).collect::<Vec<_>>();
    assert_eq!(listed(rig.studio.workspaces.list().await.unwrap()), ["annex", "shop-front"]);

    // Called something else, its id is what it was, by either name.
    let renamed = rig.studio.workspaces.rename("Shop Front", "The whole shop", &ada()).await.unwrap();
    assert_eq!((renamed.id.as_str(), renamed.name.as_str()), ("shop-front", "The whole shop"));
    assert_eq!(rig.studio.workspaces.get("shop-front").await.unwrap().name, "The whole shop");
    assert_eq!(rig.studio.workspaces.get("The whole shop").await.unwrap().id, "shop-front");
    assert_eq!(listed(rig.studio.workspaces.list().await.unwrap()), ["annex", "shop-front"]);
    let said = rig.events("workspace.renamed").await;
    assert_eq!(said.len(), 1, "{said:?}");
    assert_eq!(said[0].data["was"], "Shop Front");
    // The same name again says nothing a second time.
    rig.studio.workspaces.rename("shop-front", "The whole shop", &ada()).await.unwrap();
    assert_eq!(rig.events("workspace.renamed").await.len(), 1);
    assert_eq!(rig.events("workspace.created").await.len(), 2);
}

#[tokio::test]
async fn a_project_is_put_in_another_workspace_and_never_taken_out_of_all_and_nothing_else_of_it_changes() {
    let rig = Rig::new().await;
    rig.studio.workspaces.create("Front", &ada()).await.unwrap();
    rig.studio.workspaces.create("Back", &ada()).await.unwrap();
    // Each is made in the usual one, which is the one made first.
    let shop = rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    rig.studio.projects.create("Depot", None, &ada()).await.unwrap();
    assert_eq!(rig.studio.workspaces.get("front").await.unwrap().projects, ["depot", "shop"]);

    // It is in one: put in another, it is out of the first.
    let now = rig.studio.workspaces.place_project("shop", Some("back"), &ada()).await.unwrap();
    assert_eq!((now.id.as_str(), now.projects.as_slice()), ("back", ["shop".to_string()].as_slice()));
    assert_eq!(rig.studio.workspaces.get("front").await.unwrap().projects, ["depot"]);
    assert_eq!(rig.studio.workspaces.of_project("shop").await.unwrap().as_deref(), Some("back"));
    assert_eq!(rig.studio.projects.get("shop").await.unwrap(), shop, "the project is as it was");
    // Done again it is the same, and is said once.
    rig.studio.workspaces.place_project("shop", Some("back"), &ada()).await.unwrap();
    let placed = rig.events("project.placed").await;
    assert_eq!(placed.len(), 3, "{placed:?}");
    assert_eq!((&placed[2].data["workspace"], &placed[2].data["was"]), (&"back".into(), &"front".into()));

    // Taken out of its workspace it would be in none: refused, saying to put it in another.
    let out = rig.studio.workspaces.place_project("shop", None, &ada()).await.unwrap_err();
    assert_eq!(out.kind, ErrorKind::Invalid, "{out}");
    assert!(out.message.contains("a project is always in a workspace"), "{out}");
    assert!(out.message.contains("workspace put <workspace> shop"), "{out}");
    assert_eq!(rig.studio.workspaces.of_project("shop").await.unwrap().as_deref(), Some("back"));

    // One there is not, either way, is refused and nothing is written.
    let nowhere = rig.studio.workspaces.place_project("shop", Some("cellar"), &ada()).await.unwrap_err();
    assert!(nowhere.message.contains("there is no workspace `cellar`"), "{nowhere}");
    let nothing = rig.studio.workspaces.place_project("attic", Some("front"), &ada()).await.unwrap_err();
    assert!(nothing.message.contains("there is no project `attic`"), "{nothing}");
    assert_eq!(rig.studio.workspaces.get("front").await.unwrap().projects, ["depot"]);
    assert_eq!(rig.events("project.placed").await.len(), 3);
}

#[tokio::test]
async fn a_channel_belongs_to_a_project_or_to_a_workspace_and_never_to_both() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    rig.studio.workspaces.create("Front", &ada()).await.unwrap();
    let plans = rig.studio.channels.create("plans", Some("shop"), "", &ada()).await.unwrap();
    let workspaces = &rig.studio.workspaces;

    // A project's, made a workspace's: it is the project's no more.
    let now = workspaces.place_channel("plans", Belongs::Workspace("front"), &ada()).await.unwrap();
    assert_eq!(now.project, None);
    assert_eq!(workspaces.of_channel(&plans.id).await.unwrap().as_deref(), Some("front"));
    assert_eq!(workspaces.get("front").await.unwrap().channels, std::slice::from_ref(&plans.id));
    // And back: it is the workspace's no more.
    let now = workspaces.place_channel("plans", Belongs::Project("shop"), &ada()).await.unwrap();
    assert_eq!(now.project.as_deref(), Some("shop"));
    assert_eq!(workspaces.of_channel(&plans.id).await.unwrap(), None);
    assert!(workspaces.get("front").await.unwrap().channels.is_empty());
    // Nothing is refused: it is always one or the other, and is as it was.
    let nothing = workspaces.place_channel("plans", Belongs::Nothing, &ada()).await.unwrap_err();
    assert_eq!(nothing.kind, ErrorKind::Invalid, "{nothing}");
    assert!(
        nothing.message.contains("a channel is a project's or a workspace's, and never nobody's"),
        "{nothing}"
    );
    assert!(nothing.message.contains("channel belongs plans --workspace <workspace>"), "{nothing}");
    assert_eq!(
        rig.studio.channels.get(&crate::own(), "plans").await.unwrap().project.as_deref(),
        Some("shop")
    );
    // The same said again is said once; what was said in it is as it was.
    workspaces.place_channel("plans", Belongs::Project("shop"), &ada()).await.unwrap();
    let placed = rig.events("channel.placed").await;
    assert_eq!(placed.len(), 2, "{placed:?}");
    assert_eq!(placed[0].data["workspace"], "front");
    assert_eq!(placed[1].data["project"], "shop");
    assert_eq!(rig.studio.channels.get(&crate::own(), "plans").await.unwrap().id, plans.id);

    let nowhere = workspaces.place_channel("plans", Belongs::Workspace("cellar"), &ada()).await.unwrap_err();
    assert!(nowhere.message.contains("there is no workspace `cellar`"), "{nowhere}");
    let nothing = workspaces.place_channel("plans", Belongs::Project("attic"), &ada()).await.unwrap_err();
    assert!(nothing.message.contains("there is no project `attic`"), "{nothing}");
}

#[tokio::test]
async fn a_workspace_that_holds_something_is_taken_away_with_where_that_goes_and_the_last_one_stays() {
    let rig = Rig::new().await;
    let workspaces = &rig.studio.workspaces;
    workspaces.create("Front", &ada()).await.unwrap();
    let shop = rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let plans = rig.studio.channels.create("plans", None, "", &ada()).await.unwrap();
    workspaces.place_channel("plans", Belongs::Workspace("front"), &ada()).await.unwrap();

    // The only one there is stays.
    let last = workspaces.remove("front", None, &ada()).await.unwrap_err();
    assert_eq!(last.kind, ErrorKind::Conflict, "{last}");
    assert!(last.message.contains("workspace rename front <name>"), "{last}");

    // With another there: not without where what is in it goes, not to
    // itself, and not to one there is not.
    workspaces.create("Back", &ada()).await.unwrap();
    let holds = workspaces.remove("front", None, &ada()).await.unwrap_err();
    assert_eq!(holds.kind, ErrorKind::Conflict, "{holds}");
    assert!(holds.message.contains("workspace remove front --to <workspace>"), "{holds}");
    let itself = workspaces.remove("front", Some("Front"), &ada()).await.unwrap_err();
    assert_eq!(itself.kind, ErrorKind::Invalid, "{itself}");
    let nowhere = workspaces.remove("front", Some("cellar"), &ada()).await.unwrap_err();
    assert!(nowhere.message.contains("there is no workspace `cellar`"), "{nowhere}");
    assert_eq!(workspaces.list().await.unwrap().len(), 2, "nothing was taken away by what was refused");
    assert_eq!(rig.events("workspace.removed").await.len(), 0);

    let before = (rig.events("project.placed").await.len(), rig.events("channel.placed").await.len());
    let gone = workspaces.remove("front", Some("back"), &ada()).await.unwrap();
    assert_eq!((gone.projects, gone.channels), (vec!["shop".to_string()], vec![plans.id.clone()]));
    // Nothing that was in it is removed or changed: each is in the other, and is said to be, once.
    assert_eq!(rig.studio.projects.get("shop").await.unwrap(), shop);
    assert_eq!(rig.studio.channels.get(&crate::own(), "plans").await.unwrap().id, plans.id);
    assert_eq!(workspaces.of_project("shop").await.unwrap().as_deref(), Some("back"));
    assert_eq!(workspaces.of_channel(&plans.id).await.unwrap().as_deref(), Some("back"));
    let said = rig.events("workspace.removed").await.pop().unwrap();
    assert_eq!((&said.data["projects"], &said.data["to"]), (&serde_json::json!(["shop"]), &"back".into()));
    let placed = rig.events("project.placed").await;
    assert_eq!(placed.len(), before.0 + 1);
    let moved = &placed.last().unwrap().data;
    assert_eq!(
        (&moved["project"], &moved["workspace"], &moved["was"]),
        (&"shop".into(), &"back".into(), &"front".into())
    );
    let placed = rig.events("channel.placed").await;
    assert_eq!(placed.len(), before.1 + 1);
    let moved = &placed.last().unwrap().data;
    assert_eq!((&moved["channel"], &moved["workspace"]), (&plans.id.as_str().into(), &"back".into()));
    assert_eq!(moved["project"], serde_json::Value::Null);

    // One made again under the name has nothing in it, and goes as it came.
    let again = workspaces.create("Front", &ada()).await.unwrap();
    assert!(again.projects.is_empty() && again.channels.is_empty(), "{again:?}");
    workspaces.remove("front", None, &ada()).await.unwrap();
    assert_eq!(rig.events("workspace.removed").await.pop().unwrap().data.get("to"), None);
    assert_eq!(workspaces.list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn what_keeps_every_project_in_a_workspace_is_written_whole_or_not_at_all_and_said_once() {
    let rig = Rig::new().await;
    let workspaces = &rig.studio.workspaces;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    rig.studio.projects.create("Depot", None, &ada()).await.unwrap();
    // Something in the way of one statement of a write, as a stop in the
    // middle of it is: planted in the database, and taken away again.
    let plant = |rule: &'static str| rig.journal.db().call(move |connection| connection.execute_batch(rule));
    let stops_placing = |project: &'static str| {
        plant(match project {
            "shop" => {
                "CREATE TRIGGER stops BEFORE INSERT ON workspace_projects WHEN NEW.project = 'shop'
                       BEGIN SELECT RAISE(ABORT, 'stopped here'); END"
            }
            _ => {
                "CREATE TRIGGER stops BEFORE INSERT ON workspace_projects WHEN NEW.project = 'shed'
                  BEGIN SELECT RAISE(ABORT, 'stopped here'); END"
            }
        })
    };
    let said = || async {
        let mut said = Vec::new();
        for topic in ["workspace.created", "workspace.removed", "project.placed", "project.created"] {
            said.push(rig.events(topic).await.len());
        }
        said
    };

    // Settling, cut at the last project it places: the workspace it made
    // and the project it had placed are not kept, and nothing is said.
    stops_placing("shop").await.unwrap();
    workspaces.settle(Some("Ada")).await.unwrap_err();
    assert!(workspaces.list().await.unwrap().is_empty());
    assert_eq!(said().await, [0, 0, 0, 2]);
    // Asked again, as the next start asks: all of it, each thing said once.
    plant("DROP TRIGGER stops").await.unwrap();
    let settled = workspaces.settle(Some("Ada")).await.unwrap();
    assert_eq!(settled.placed, ["depot", "shop"]);
    assert_eq!(said().await, [1, 0, 2, 2]);

    // A project is made with its place or not at all: there is no
    // moment in which it is written and in no workspace.
    stops_placing("shed").await.unwrap();
    rig.studio.projects.create("Shed", None, &ada()).await.unwrap_err();
    assert!(rig.studio.projects.list().await.unwrap().iter().all(|project| project.id != "shed"));
    assert_eq!(said().await, [1, 0, 2, 2]);
    plant("DROP TRIGGER stops").await.unwrap();

    // A workspace taken away with where what it holds goes, cut as the
    // workspace itself goes: its projects are where they were, and
    // nothing is said of any of it.
    workspaces.create("Back", &ada()).await.unwrap();
    plant("CREATE TRIGGER stops BEFORE DELETE ON workspaces BEGIN SELECT RAISE(ABORT, 'stopped here'); END")
        .await
        .unwrap();
    workspaces.remove("ada-s-workspace", Some("back"), &ada()).await.unwrap_err();
    assert_eq!(workspaces.get("ada-s-workspace").await.unwrap().projects, ["depot", "shop"]);
    assert!(workspaces.get("back").await.unwrap().projects.is_empty());
    assert_eq!(said().await, [2, 0, 2, 2]);
    plant("DROP TRIGGER stops").await.unwrap();
    workspaces.remove("ada-s-workspace", Some("back"), &ada()).await.unwrap();
    assert_eq!(workspaces.get("back").await.unwrap().projects, ["depot", "shop"]);
    assert_eq!(said().await, [2, 1, 4, 2]);
    // And every project is in a workspace at the end of it.
    assert_eq!(workspaces.settle(None).await.unwrap(), Settled::default());
}

#[tokio::test]
async fn a_channel_made_for_no_project_is_a_workspaces_own_from_the_write_that_makes_it() {
    let rig = Rig::new().await;
    let (workspaces, channels) = (&rig.studio.workspaces, &rig.studio.channels);
    workspaces.create("Front", &ada()).await.unwrap();
    workspaces.create("Back", &ada()).await.unwrap();
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    // With nothing said: the usual workspace's own, which is the one made first.
    let chat = channels.create("chat", None, "", &ada()).await.unwrap();
    assert_eq!(workspaces.of_channel(&chat.id).await.unwrap().as_deref(), Some("front"));
    // With a workspace said: that one's, and it was never another's.
    let plans = channels.asked_in("back").create("plans", None, "", &ada()).await.unwrap();
    assert_eq!(workspaces.of_channel(&plans.id).await.unwrap().as_deref(), Some("back"));
    // With one said that is not there: the usual one, since it is always one's.
    let notes = channels.asked_in("cellar").create("notes", None, "", &ada()).await.unwrap();
    assert_eq!(workspaces.of_channel(&notes.id).await.unwrap().as_deref(), Some("front"));
    // A project's is its project's, and no workspace's own, whatever is said beside it.
    let talk = channels.asked_in("back").create("shop-talk", Some("shop"), "", &ada()).await.unwrap();
    assert_eq!(workspaces.of_channel(&talk.id).await.unwrap(), None);
    // And one that is made sure of: made as any is, and found as it is afterwards.
    let sure = channels.ensure("standup", None, "", &ada()).await.unwrap();
    assert_eq!(workspaces.of_channel(&sure.id).await.unwrap().as_deref(), Some("front"));
    assert_eq!(channels.ensure("standup", None, "", &ada()).await.unwrap().id, sure.id);
    // One kept to its members is placed like any other, and that it was is said to them.
    let kept = channels.create_kept("Pay review", None, "", &ada()).await.unwrap();
    assert_eq!(workspaces.of_channel(&kept.id).await.unwrap().as_deref(), Some("front"));

    // Each was said to be placed in the write that made it, by whoever made it.
    let placed = rig.events("channel.placed").await;
    let said: Vec<(&str, &str)> = placed
        .iter()
        .map(|event| (event.data["channel"].as_str().unwrap(), event.data["workspace"].as_str().unwrap()))
        .collect();
    let wanted = [(&chat, "front"), (&plans, "back"), (&notes, "front"), (&sure, "front"), (&kept, "front")];
    assert_eq!(said, wanted.map(|(channel, workspace)| (channel.id.as_str(), workspace)));
    assert!(placed.iter().all(|event| event.source.actor.as_deref() == Some("user:ada")), "{placed:?}");
    assert!(placed[..4].iter().all(|event| event.source.takes.is_none()));
    let takes = placed[4].source.takes.clone().unwrap_or_default();
    assert!(takes.starts_with(&format!("member:{}", kept.id)), "{takes}");
    let front = workspaces.get("front").await.unwrap();
    assert_eq!(front.channels.len(), 4);

    // The write cut as the place is written, as a stop there is: there is no channel either.
    let plant = |rule: &'static str| rig.journal.db().call(move |connection| connection.execute_batch(rule));
    plant("CREATE TRIGGER stops BEFORE INSERT ON workspace_channels BEGIN SELECT RAISE(ABORT, 'stopped here'); END")
        .await
        .unwrap();
    channels.create("late", None, "", &ada()).await.unwrap_err();
    channels.get(&crate::own(), "late").await.unwrap_err();
    assert_eq!(rig.events("channel.created").await.len(), 6);
    assert_eq!(rig.events("channel.placed").await.len(), 5);
    plant("DROP TRIGGER stops").await.unwrap();
}

#[tokio::test]
async fn a_channel_that_belongs_to_nothing_or_to_a_project_that_is_gone_is_the_usual_workspaces_own_once_settled()
 {
    let rig = Rig::new().await;
    let (workspaces, channels) = (&rig.studio.workspaces, &rig.studio.channels);
    workspaces.settle(Some("Ada")).await.unwrap();
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let chat = channels.create("chat", None, "", &ada()).await.unwrap();
    let kept = channels.create_kept("Pay review", None, "", &ada()).await.unwrap();
    let talk = channels.create("shop-talk", Some("shop"), "", &ada()).await.unwrap();
    // As a factory an earlier program made may hold them: two that
    // belong to nothing, one of them kept to its members, and one that
    // says a project nobody has any more.
    let old = channels.create("old", Some("gone"), "", &ada()).await.unwrap();
    rig.journal
        .db()
        .call(|connection| connection.execute("DELETE FROM workspace_channels", []))
        .await
        .unwrap();
    for channel in [&chat, &kept, &old] {
        assert_eq!(workspaces.of_channel(&channel.id).await.unwrap(), None);
    }
    let before = rig.events("channel.placed").await.len();

    let mut settled = workspaces.settle(None).await.unwrap().channels;
    settled.sort();
    let mut wanted = vec![chat.id.clone(), kept.id.clone(), old.id.clone()];
    wanted.sort();
    assert_eq!(settled, wanted);
    for channel in [&chat, &kept, &old] {
        assert_eq!(workspaces.of_channel(&channel.id).await.unwrap().as_deref(), Some("ada-s-workspace"));
    }
    // The one that said a project that is gone says none now; one of a
    // project that is there is its project's, and nothing of it moved.
    assert_eq!(channels.get(&crate::own(), "old").await.unwrap().project, None);
    assert_eq!(channels.get(&crate::own(), "shop-talk").await.unwrap(), talk);
    assert_eq!(workspaces.of_channel(&talk.id).await.unwrap(), None);
    // Each said once, as the system; of the kept one, to its members.
    let said = rig.events("channel.placed").await.split_off(before);
    assert_eq!(said.len(), 3);
    for event in &said {
        assert_eq!(event.source.actor.as_deref(), Some("system"));
        let takes = (event.data["channel"] == kept.id.as_str()).then(|| format!("member:{}", kept.id));
        assert_eq!(event.source.takes, takes, "{}", event.data);
        assert_eq!(event.data["workspace"], "ada-s-workspace");
    }
    // Settled again there is nothing to bring about.
    assert_eq!(workspaces.settle(None).await.unwrap(), Settled::default());
    assert_eq!(rig.events("channel.placed").await.len(), before + 3);
}

#[tokio::test]
async fn a_project_that_is_forgotten_leaves_its_channels_to_the_workspace_it_was_in() {
    let rig = Rig::new().await;
    let (workspaces, channels) = (&rig.studio.workspaces, &rig.studio.channels);
    workspaces.create("Front", &ada()).await.unwrap();
    workspaces.create("Back", &ada()).await.unwrap();
    rig.studio.projects.create_in("Shop", None, &ada(), true, Some("back")).await.unwrap();
    rig.studio.projects.create("Depot", None, &ada()).await.unwrap();
    let talk = channels.create("shop-talk", Some("shop"), "", &ada()).await.unwrap();
    let kept = channels.create_kept("Shop pay", Some("shop"), "", &ada()).await.unwrap();
    let other = channels.create("depot-talk", Some("depot"), "", &ada()).await.unwrap();
    channels.post("shop-talk", &ada(), "the awning goes up", None).await.unwrap();
    let before = rig.events("channel.placed").await.len();

    // Cut as the project itself goes, as a stop there is: it is there
    // still, in its workspace, with its channels its own.
    let plant = |rule: &'static str| rig.journal.db().call(move |connection| connection.execute_batch(rule));
    plant("CREATE TRIGGER stops BEFORE DELETE ON projects BEGIN SELECT RAISE(ABORT, 'stopped here'); END")
        .await
        .unwrap();
    rig.studio.projects.forget("shop", &ada()).await.unwrap_err();
    assert_eq!(workspaces.of_project("shop").await.unwrap().as_deref(), Some("back"));
    assert_eq!(channels.get(&crate::own(), "shop-talk").await.unwrap(), talk);
    assert_eq!(rig.events("channel.placed").await.len(), before);
    plant("DROP TRIGGER stops").await.unwrap();

    rig.studio.projects.forget("shop", &ada()).await.unwrap();
    // No workspace lists it, and its channels are the own of the one it
    // was in, which is not the usual one: nothing else of them changed.
    assert_eq!(workspaces.of_project("shop").await.unwrap(), None);
    let back = workspaces.get("back").await.unwrap();
    assert!(back.projects.is_empty(), "{back:?}");
    assert_eq!(back.channels.len(), 2);
    for channel in [&talk, &kept] {
        assert_eq!(workspaces.of_channel(&channel.id).await.unwrap().as_deref(), Some("back"));
        let now = channels.get(&crate::own(), &channel.id).await.unwrap();
        assert_eq!(now, studio::Channel { project: None, ..(*channel).clone() });
    }
    assert_eq!(channels.messages(&crate::own(), "shop-talk", None, 10).await.unwrap().len(), 1);
    // Each said once, by whoever forgot the project; of the kept one, to its members.
    let said = rig.events("channel.placed").await.split_off(before);
    assert_eq!(said.len(), 2);
    for event in &said {
        assert_eq!(event.source.actor.as_deref(), Some("user:ada"));
        let takes = (event.data["channel"] == kept.id.as_str()).then(|| format!("member:{}", kept.id));
        assert_eq!(event.source.takes, takes, "{}", event.data);
        assert_eq!(
            (&event.data["workspace"], &event.data["project"]),
            (&"back".into(), &serde_json::Value::Null)
        );
    }
    // Another project's channel is its project's, as it was.
    assert_eq!(channels.get(&crate::own(), "depot-talk").await.unwrap(), other);
    // And there is nothing left for a start to bring about.
    assert_eq!(workspaces.settle(None).await.unwrap(), Settled::default());
}

#[tokio::test]
async fn a_channel_a_workspace_names_that_is_gone_is_passed_over() {
    let rig = Rig::new().await;
    rig.studio.workspaces.create("Front", &ada()).await.unwrap();
    let plans = rig.studio.channels.create("plans", None, "", &ada()).await.unwrap();
    assert_eq!(rig.studio.workspaces.get("front").await.unwrap().channels, std::slice::from_ref(&plans.id));
    // The channel removed, with nobody having told the workspaces: it is not listed.
    rig.studio.channels.remove("plans", &ada()).await.unwrap();
    let front = rig.studio.workspaces.get("front").await.unwrap();
    assert!(front.channels.is_empty(), "{front:?}");
    assert_eq!(rig.studio.workspaces.of_channel(&plans.id).await.unwrap().as_deref(), Some("front"));
    // And told, there is nothing left to pass over.
    rig.studio.workspaces.forgot_channel(&plans.id).await.unwrap();
    assert_eq!(rig.studio.workspaces.of_channel(&plans.id).await.unwrap(), None);
}

#[tokio::test]
async fn somebody_who_may_not_add_a_project_makes_and_fills_no_workspace() {
    let rig = Rig::new().await;
    rig.studio.workspaces.create("Front", &ada()).await.unwrap();
    rig.studio.workspaces.create("Side", &ada()).await.unwrap();
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    rig.studio.channels.create("plans", None, "", &ada()).await.unwrap();
    // Reads everything, and changes channels but not projects.
    let reader = agent("reader", &["projects:read", "channels"]);
    for refused in [
        rig.studio.workspaces.create("Back", &reader).await.map(|_| ()),
        rig.studio.workspaces.rename("front", "Mine", &reader).await.map(|_| ()),
        rig.studio.workspaces.remove("side", None, &reader).await.map(|_| ()),
        rig.studio.workspaces.remove("front", Some("side"), &reader).await.map(|_| ()),
        rig.studio.workspaces.place_project("shop", Some("side"), &reader).await.map(|_| ()),
        rig.studio.workspaces.place_project("shop", None, &reader).await.map(|_| ()),
    ] {
        let refused = refused.unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Denied, "{refused}");
        assert!(refused.message.contains("projects:write"), "{refused}");
    }
    // What a channel belongs to is a channel's, which it may change.
    rig.studio.workspaces.place_channel("plans", Belongs::Workspace("front"), &reader).await.unwrap();
    // And one who may not change channels may not say so.
    let looker = agent("looker", &["projects", "channels:read"]);
    let refused = rig.studio.workspaces.place_channel("plans", Belongs::Nothing, &looker).await.unwrap_err();
    assert!(refused.message.contains("channels:write"), "{refused}");
    assert_eq!(rig.studio.workspaces.list().await.unwrap().len(), 2);
    assert_eq!(rig.studio.workspaces.get("front").await.unwrap().name, "Front");
    assert_eq!(rig.studio.workspaces.of_project("shop").await.unwrap().as_deref(), Some("front"));
}

#[tokio::test]
async fn what_was_said_in_a_channel_is_found_within_the_project_the_channel_is_of_now() {
    let rig = Rig::new().await;
    let studio = &rig.studio;
    studio.projects.create("Shop", None, &ada()).await.unwrap();
    studio.projects.create("Depot", None, &ada()).await.unwrap();
    studio.workspaces.create("Front", &ada()).await.unwrap();
    studio.channels.create("plans", Some("shop"), "", &ada()).await.unwrap();
    studio.channels.post("plans", &ada(), "The awning goes up on Friday.", None).await.unwrap();
    studio.search.catch_up().await.unwrap();
    let within = |project: &'static str| async move {
        studio.search.catch_up().await.unwrap();
        studio.search.query("awning", None, Some(project), 5).await.unwrap().len()
    };
    assert_eq!((within("shop").await, within("depot").await), (1, 0));

    // Made a workspace's, the channel is no project's, and neither is
    // what was said in it; it is found all the same where nothing narrows.
    studio.workspaces.place_channel("plans", Belongs::Workspace("front"), &ada()).await.unwrap();
    assert_eq!((within("shop").await, within("depot").await), (0, 0));
    assert_eq!(studio.search.query("awning", None, None, 5).await.unwrap().len(), 1);
    // Made another project's, it is found within that one.
    studio.workspaces.place_channel("plans", Belongs::Project("depot"), &ada()).await.unwrap();
    assert_eq!((within("shop").await, within("depot").await), (0, 1));
}

#[tokio::test]
async fn a_channel_kept_to_its_members_is_left_out_of_a_workspace_and_of_a_search_for_whoever_does_not_read_it()
 {
    let rig = Rig::new().await;
    let studio = &rig.studio;
    let person = |id: &str| kernel::Principal {
        kind: kernel::PrincipalKind::User,
        ..crate::agent(id, &["channels", "projects", "search"])
    };
    let (mira, nell) = (person("mira"), person("nell"));
    studio.workspaces.create("Front", &ada()).await.unwrap();
    studio.workspaces.create("Yard", &ada()).await.unwrap();
    let plans = studio.channels.create("plans", None, "", &mira).await.unwrap();
    let kept = studio.channels.create_kept("Pay review", None, "", &mira).await.unwrap();
    for channel in [&plans.id, &kept.id] {
        studio.workspaces.place_channel(channel, Belongs::Workspace("front"), &mira).await.unwrap();
        studio.channels.post(channel, &mira, "the awning goes up", None).await.unwrap();
    }
    // What it belongs to is said to its members, as everything of it is,
    // and by one of them: to anybody else it is not there to place.
    let placed = rig.events("channel.placed").await;
    assert_eq!(placed.len(), 2, "each was placed as it was made, and saying the same again says nothing");
    assert_eq!(placed[0].source.takes, None);
    let takes = placed[1].source.takes.clone().unwrap_or_default();
    assert!(takes.starts_with(&format!("member:{}", kept.id)), "{takes}");
    let refused = studio.workspaces.place_channel(&kept.id, Belongs::Nothing, &nell).await.unwrap_err();
    let nowhere = studio.workspaces.place_channel("ch_nonesuch", Belongs::Nothing, &nell).await.unwrap_err();
    assert_eq!(refused.message.replace(&kept.id, "<it>"), nowhere.message.replace("ch_nonesuch", "<it>"));

    // The workspace lists it for whoever reads it, and for nobody else;
    // an open channel of it is listed for everybody, as it always was.
    let listed = |by: kernel::Principal| async move {
        let one = studio.workspaces.get("front").await.unwrap();
        let shown = studio.workspaces.shown_to(one, &by).await.unwrap().channels;
        let all = studio.workspaces.list_for(&by).await.unwrap();
        assert_eq!(all[0].channels, shown);
        shown
    };
    assert_eq!(listed(mira.clone()).await.len(), 2);
    assert_eq!(listed(ada()).await.len(), 2, "whoever holds everything reads it");
    assert_eq!(listed(nell.clone()).await, std::slice::from_ref(&plans.id));
    assert_eq!(studio.workspaces.get("front").await.unwrap().channels.len(), 2);

    // A search finds what was said in it for whoever reads it.
    studio.search.catch_up().await.unwrap();
    let found = |by: kernel::Principal| async move {
        let hits = studio.search.query("awning", None, None, 10).await.unwrap();
        let shown = studio.search.readable(hits, &by).await.unwrap();
        shown.into_iter().map(|hit| hit.link["channel"].as_str().unwrap().to_string()).collect::<Vec<_>>()
    };
    assert_eq!(found(mira.clone()).await.len(), 2);
    assert_eq!(found(nell.clone()).await, std::slice::from_ref(&plans.id));

    // Taken away with where what it held goes, the kept one goes with
    // the rest: so by somebody who could have placed it, and not by Nell,
    // who does not read it. She is told that it holds a channel that is
    // not hers to move, and nothing of which; nothing is moved or said.
    let before = rig.events("channel.placed").await.len();
    let refused = studio.workspaces.remove("front", Some("yard"), &nell).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied, "{refused}");
    assert!(refused.message.contains("holds a channel that is not yours to move"), "{refused}");
    assert!(!refused.message.contains(&kept.id) && !refused.message.contains("Pay review"), "{refused}");
    assert_eq!(studio.workspaces.get("front").await.unwrap().channels.len(), 2);
    assert_eq!(rig.events("channel.placed").await.len(), before);
    assert!(rig.events("workspace.removed").await.is_empty());
    // By Mira, who is in it: the workspace is said to have had the open
    // one, she is handed what she reads of it, and that the kept one
    // moved is said to its members and nobody else.
    let gone = studio.workspaces.remove("front", Some("yard"), &mira).await.unwrap();
    assert_eq!(gone.channels.len(), 2);
    let said = rig.events("workspace.removed").await.pop().unwrap();
    assert_eq!(said.data["channels"], serde_json::json!([plans.id]));
    assert!(!said.data.to_string().contains(&kept.id), "{}", said.data);
    assert_eq!(studio.workspaces.of_channel(&kept.id).await.unwrap().as_deref(), Some("yard"));
    assert_eq!(studio.workspaces.of_channel(&plans.id).await.unwrap().as_deref(), Some("yard"));
    let moved = rig.events("channel.placed").await.split_off(before);
    assert_eq!(moved.len(), 2);
    for event in moved {
        let takes = (event.data["channel"] == kept.id.as_str()).then(|| format!("member:{}", kept.id));
        assert_eq!(event.source.takes, takes, "{}", event.data);
        assert_eq!(event.data["workspace"], "yard");
    }
}

#[tokio::test]
async fn nobody_moves_a_channel_by_taking_its_workspace_away_who_could_not_move_it_by_placing_it() {
    let rig = Rig::new().await;
    let workspaces = &rig.studio.workspaces;
    workspaces.create("Front", &ada()).await.unwrap();
    workspaces.create("Side", &ada()).await.unwrap();
    workspaces.create("Attic", &ada()).await.unwrap();
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let plans = rig.studio.channels.create("plans", None, "", &ada()).await.unwrap();
    workspaces.place_channel("plans", Belongs::Workspace("front"), &ada()).await.unwrap();
    // May add a project, and so make, fill and take away a workspace;
    // may not change a channel.
    let adder = agent("adder", &["projects", "channels:read"]);
    // What placing the channel by itself asks of them.
    let placing = workspaces.place_channel("plans", Belongs::Workspace("side"), &adder).await.unwrap_err();
    assert_eq!(placing.kind, ErrorKind::Denied, "{placing}");
    // Taking its workspace away would move it: the same is asked, and
    // nothing of the workspace is moved or said, its project either.
    let refused = workspaces.remove("front", Some("side"), &adder).await.unwrap_err();
    assert_eq!((refused.kind, &refused.message), (placing.kind, &placing.message));
    assert!(refused.message.contains("channels:write"), "{refused}");
    let front = workspaces.get("front").await.unwrap();
    assert_eq!((front.projects, front.channels), (vec!["shop".to_string()], vec![plans.id.clone()]));
    assert!(rig.events("workspace.removed").await.is_empty());
    assert_eq!(rig.events("channel.placed").await.len(), 1);
    // Only where there is a channel to move: one that holds projects
    // alone, or nothing, is theirs to take away as it always was.
    workspaces.place_project("shop", Some("attic"), &adder).await.unwrap();
    workspaces.remove("attic", Some("side"), &adder).await.unwrap();
    assert_eq!(workspaces.of_project("shop").await.unwrap().as_deref(), Some("side"));
    // And somebody who may change channels too takes it away with its channel.
    let both = agent("both", &["projects", "channels"]);
    workspaces.remove("front", Some("side"), &both).await.unwrap();
    assert_eq!(workspaces.of_channel(&plans.id).await.unwrap().as_deref(), Some("side"));
}

/// A factory in memory whose projects each have a journal of their own,
/// as a daemon keeps them: its journal and its studio.
async fn with_journals_apart() -> (journal::Journal, studio::Studio) {
    let journal = journal::Journal::memory(std::sync::Arc::new(kernel::ManualClock::new())).await.unwrap();
    journal.keeps_projects_in_memory().await.unwrap();
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("studio-placed-{}", &kernel::id::secret()[..12]));
    let searched = journal::Db::memory().unwrap();
    let studio =
        studio::Studio::open(journal.clone(), searched, dir.join("projects"), dir.join("attachments"))
            .await
            .unwrap();
    (journal, studio)
}

#[tokio::test]
async fn a_project_with_a_journal_of_its_own_is_placed_and_said_to_be_in_one_write_of_the_factorys() {
    if crate::one_journal() {
        return;
    }
    let (journal, studio) = with_journals_apart().await;
    let workspaces = &studio.workspaces;
    workspaces.settle(Some("Ada")).await.unwrap();
    workspaces.create("Back", &ada()).await.unwrap();
    studio.projects.create("Shop", None, &ada()).await.unwrap();
    let shop = journal.of_project("shop").expect("the project has a journal of its own");
    // Where each journal keeps what was said on `topic`, by the project it names.
    let said = |journal: &journal::Journal, topic: &'static str| {
        let journal = journal.clone();
        async move {
            let query = journal::EventQuery { topics: topic.into(), limit: 1000, ..Default::default() };
            let all = journal.read(query).await.unwrap();
            all.into_iter()
                .map(|event| event.data["workspace"].as_str().unwrap_or("").to_string())
                .collect::<Vec<_>>()
        }
    };
    // That it was made is the project's own journal's to keep; that it
    // was placed is kept where its place is, in the factory's.
    assert_eq!(said(&shop, "project.created").await.len(), 1);
    assert_eq!(said(&journal, "project.placed").await, ["ada-s-workspace"]);
    assert_eq!(said(&shop, "project.placed").await, Vec::<String>::new());

    // The write cut where it says so, as a stop there is: the place is
    // not kept either, by any way a project comes to be placed.
    let plant = |rule: &'static str| journal.db().call(move |connection| connection.execute_batch(rule));
    let cut = "CREATE TRIGGER stops BEFORE INSERT ON events WHEN NEW.topic = 'project.placed'
               BEGIN SELECT RAISE(ABORT, 'stopped here'); END";
    plant(cut).await.unwrap();
    // Put in another.
    workspaces.place_project("shop", Some("back"), &ada()).await.unwrap_err();
    assert_eq!(workspaces.of_project("shop").await.unwrap().as_deref(), Some("ada-s-workspace"));
    // Moved with its workspace.
    workspaces.remove("ada-s-workspace", Some("back"), &ada()).await.unwrap_err();
    assert_eq!(workspaces.list().await.unwrap().len(), 2);
    assert_eq!(workspaces.of_project("shop").await.unwrap().as_deref(), Some("ada-s-workspace"));
    // Made: no project is written whose place was not said.
    studio.projects.create("Depot", None, &ada()).await.unwrap_err();
    assert!(studio.projects.list().await.unwrap().iter().all(|project| project.id != "depot"));
    // Settled, as a start settles a factory from before.
    journal
        .db()
        .call(|connection| connection.execute("DELETE FROM workspace_projects WHERE project = 'shop'", []))
        .await
        .unwrap();
    workspaces.settle(None).await.unwrap_err();
    assert_eq!(workspaces.of_project("shop").await.unwrap(), None);
    assert_eq!(
        said(&journal, "project.placed").await,
        ["ada-s-workspace"],
        "nothing was said by any of them"
    );

    // With nothing in the way each is done, and said once, in the factory's journal.
    plant("DROP TRIGGER stops").await.unwrap();
    workspaces.settle(None).await.unwrap();
    workspaces.place_project("shop", Some("back"), &ada()).await.unwrap();
    workspaces.remove("back", Some("ada-s-workspace"), &ada()).await.unwrap();
    assert_eq!(
        said(&journal, "project.placed").await,
        ["ada-s-workspace", "ada-s-workspace", "back", "ada-s-workspace"]
    );
    assert_eq!(said(&shop, "project.placed").await, Vec::<String>::new());
    // And it is the project's news all the same, by what it names.
    let query = journal::EventQuery { topics: "project.placed".into(), limit: 1000, ..Default::default() };
    assert!(journal.read(query).await.unwrap().iter().all(|event| event.data["project"] == "shop"));
}
