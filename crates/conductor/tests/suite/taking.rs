//! How what is running comes by new texts is a setting of the factory:
//! held until somebody takes them, or followed as they change; and a
//! text that is taken lands in place, or by its steps being run again.

use conductor::taking::{Come, Land, Take};
use conductor::{Apply, InstanceFilter, InstanceInfo};
use kernel::{ErrorKind, Principal, PrincipalKind, Source};
use serde_json::json;

use crate::handlers::{active, handled};
use crate::{Rig, user};

/// Does something once when it is applied, and something each time it is poked.
fn keeper(version: u32, claims: &str, says: &str) -> String {
    format!(
        "blueprint: keeper\nversion: {version}\nsteps:\n  claim: {{ do: test.count, with: {{ key: {claims} }} }}\non:\n  \
         poke:\n    event: test.poke\n    steps:\n      say: {{ do: test.count, with: {{ key: {says} }} }}\n"
    )
}

fn take(come: Option<&str>, land: Option<&str>) -> Take {
    Take { come: come.map(str::to_string), land: land.map(str::to_string) }
}

/// The keeper applied as it first is, and settled: its id.
async fn kept(rig: &Rig) -> String {
    let first = keeper(1, "claimed", "old");
    rig.library.put("keeper", &first);
    let applied = rig.apply(&first, json!({})).await;
    active(rig, &applied.instance).await;
    applied.instance
}

async fn poke(rig: &Rig, times: usize) {
    rig.journal.emit("test.poke", Source::default(), json!({})).await.unwrap();
    handled(rig, "on.poke", times).await;
}

fn holding(scopes: &[&str]) -> Principal {
    Principal {
        kind: PrincipalKind::User,
        id: "bo".into(),
        name: "Bo".into(),
        scopes: scopes.iter().map(|scope| scope.to_string()).collect(),
        key: None,
    }
}

#[tokio::test]
async fn where_texts_are_followed_what_is_running_takes_a_text_as_it_changes_and_nobody_is_asked() {
    let rig = Rig::new().await;
    // What the library holds came with the program, as in an installed factory.
    rig.library.ship();
    let id = kept(&rig).await;
    // Nothing said: what came with the program is held, so what is running goes on, behind it.
    let second = keeper(2, "claimed", "new");
    rig.library.put("keeper", &second);
    assert!(rig.engine.follow(None, None).await.is_empty());
    assert_eq!(rig.instance(&id).await.revision, blueprint::revision(&keeper(1, "claimed", "old")));

    // Said to be followed, by somebody who may: everything takes what is there, now.
    let (now, left) = rig.engine.set_taking(take(Some("follow"), None), &user()).await.unwrap();
    assert_eq!((now.come, now.land, left.len()), (Some(Come::Follow), Land::InPlace, 0));
    assert_eq!(rig.instance(&id).await.revision, blueprint::revision(&second));
    poke(&rig, 1).await;
    // It took the text where it is: what it does later is the new text's, and nothing ran again.
    assert_eq!((rig.shared.count("new"), rig.shared.count("old"), rig.shared.count("claimed")), (1, 0, 1));

    // And each change after it, as it is told of: whatever follows the files says which.
    let third = keeper(3, "claimed", "newer");
    rig.library.put("keeper", &third);
    assert!(rig.engine.follow(None, Some("keeper")).await.is_empty());
    assert_eq!(rig.instance(&id).await.revision, blueprint::revision(&third));
    let said = rig.event("system.texts", |_| true).await;
    assert_eq!((&said.data["come"], &said.data["was"]), (&json!("follow"), &json!("usual")));
    assert!(said.data["gives_up"].as_str().unwrap().contains("with the authority of whoever applied it"));
}

#[tokio::test]
async fn a_taken_text_lands_in_place_unless_the_factory_says_its_steps_run_again() {
    let rig = Rig::new().await;
    let id = kept(&rig).await;
    rig.engine.set_taking(take(Some("follow"), None), &user()).await.unwrap();
    // Other steps: in place there is nothing it can take, so it goes on and is behind.
    let second = keeper(2, "claimed_again", "old");
    rig.library.put("keeper", &second);
    rig.engine.follow(None, Some("keeper")).await;
    assert_eq!(rig.instance(&id).await.revision, blueprint::revision(&keeper(1, "claimed", "old")));
    assert_eq!(rig.shared.count("claimed_again"), 0);

    // Said to land by its steps being run again, it is begun again on the text there.
    let (now, _) = rig.engine.set_taking(take(None, Some("again")), &user()).await.unwrap();
    assert_eq!((now.come, now.land), (Some(Come::Follow), Land::Again));
    rig.engine.follow(None, Some("keeper")).await;
    rig.event("run.finished", |event| event.data["flow"] == "main" && event.seq > 0).await;
    rig.soon("its steps to have run again", async {
        while rig.shared.count("claimed_again") == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert_eq!(rig.instance(&id).await.revision, blueprint::revision(&second));
    // So does one that could have been taken where it is: that is what was asked for.
    let third = keeper(3, "claimed_again", "new");
    rig.library.put("keeper", &third);
    rig.engine.follow(None, Some("keeper")).await;
    rig.soon("its steps to have run once more", async {
        while rig.shared.count("claimed_again") < 2 {
            tokio::task::yield_now().await;
        }
    })
    .await;
}

#[tokio::test]
async fn a_text_that_does_not_check_is_not_taken_and_that_is_said_once() {
    let rig = Rig::new().await;
    let id = kept(&rig).await;
    rig.engine.set_taking(take(Some("follow"), None), &user()).await.unwrap();
    let first = blueprint::revision(&keeper(1, "claimed", "old"));
    let broken =
        keeper(2, "claimed", "new").replace("do: test.count, with: { key: new }", "do: test.nothing");
    rig.library.put("keeper", &broken);
    let left = rig.engine.follow(None, Some("keeper")).await;
    assert_eq!(left.len(), 1, "{left:?}");
    assert_eq!((left[0].instance.as_str(), left[0].kept.as_str()), (id.as_str(), first.as_str()));
    assert!(left[0].error.contains("test.nothing"), "{}", left[0].error);
    // It goes on with the last that did, and says so beside itself.
    poke(&rig, 1).await;
    assert_eq!(rig.shared.count("old"), 1);
    let beside = rig.instance(&id).await.error.unwrap_or_default();
    assert!(beside.starts_with("`keeper` was changed and does not check"), "{beside}");
    // Told of the same text again, nothing more is said.
    assert!(rig.engine.follow(None, Some("keeper")).await.is_empty());
    assert_eq!(rig.events("instance.text_left").await.len(), 1);

    // Put right, it is taken, and what it said of itself is taken back.
    let mended = keeper(3, "claimed", "new");
    rig.library.put("keeper", &mended);
    assert!(rig.engine.follow(None, Some("keeper")).await.is_empty());
    let now = rig.instance(&id).await;
    assert_eq!((now.revision, now.error), (blueprint::revision(&mended), None));
}

#[tokio::test]
async fn having_texts_followed_is_the_owners_to_say_and_takes_what_taking_a_wall_away_takes() {
    let rig = Rig::new().await;
    let member =
        rig.engine.set_taking(take(Some("held"), None), &holding(&["blueprints"])).await.unwrap_err();
    assert_eq!(member.kind, ErrorKind::Denied, "{member}");
    // Somebody who runs the factory and may not take a wall away: held and landing are theirs, following is not.
    let admin = holding(&["admin", "blueprints"]);
    let refused = rig.engine.set_taking(take(Some("follow"), None), &admin).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied);
    assert!(
        refused.message.contains("takes `shell`") && refused.message.contains("an agent among them"),
        "{refused}"
    );
    assert_eq!(rig.engine.taking().come, None, "nothing was changed");
    let (now, _) = rig.engine.set_taking(take(Some("held"), Some("again")), &admin).await.unwrap();
    assert_eq!((now.come, now.land, now.by.as_deref()), (Some(Come::Held), Land::Again, Some("user:bo")));
    let unknown = rig.engine.set_taking(take(Some("sometimes"), None), &user()).await.unwrap_err();
    assert!(unknown.message.contains("there are `held`"), "{unknown}");
    // It is kept: a factory started again says what it was told.
    let mut rig = rig;
    rig.restart().await;
    assert_eq!((rig.engine.taking().come, rig.engine.taking().land), (Some(Come::Held), Land::Again));
}

/// Starts a child by name for each thing asked for.
const PARENT: &str = "\
blueprint: parent
on:
  asked:
    event: test.poke
    steps:
      go: { spawn: child, as: \"kid-${ event.data.id }\" }
";

async fn kid(rig: &Rig, id: u64, times: usize) -> InstanceInfo {
    rig.journal.emit("test.poke", Source::default(), json!({ "id": id })).await.unwrap();
    handled(rig, "on.asked", times).await;
    let all = rig.engine.instances(InstanceFilter::default()).await.unwrap();
    all.into_iter().find(|one| one.key == format!("kid-{id}")).unwrap()
}

#[tokio::test]
async fn said_to_be_held_a_tree_goes_on_with_what_it_reaches_wherever_the_texts_are_kept() {
    let rig = Rig::new().await;
    let child =
        |version: u32| format!("blueprint: child\nversion: {version}\nsteps:\n  go: {{ do: test.echo }}\n");
    rig.library.put("parent", PARENT);
    rig.library.put("child", &child(1));
    let applied = rig.apply(PARENT, json!({})).await;
    active(&rig, &applied.instance).await;
    // Nothing said, and texts that did not come with the program: read where they are.
    rig.library.put("child", &child(2));
    assert_eq!(kid(&rig, 1, 1).await.revision, blueprint::revision(&child(2)));

    // Said to be held: from now the tree goes on with what it reaches now.
    rig.engine.set_taking(take(Some("held"), None), &user()).await.unwrap();
    rig.library.put("child", &child(3));
    assert_eq!(kid(&rig, 2, 2).await.revision, blueprint::revision(&child(2)));
    let again = rig.engine.apply(rig.request(PARENT, json!({}))).await.unwrap();
    assert_eq!(again.reached.left.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["child"]);
    // Taken, it moves; and with texts followed there is nothing held to go back to.
    rig.engine.apply(Apply { take: true, ..rig.request(PARENT, json!({})) }).await.unwrap();
    assert_eq!(kid(&rig, 3, 3).await.revision, blueprint::revision(&child(3)));
    rig.engine.set_taking(take(Some("follow"), None), &user()).await.unwrap();
    rig.library.put("child", &child(4));
    assert_eq!(kid(&rig, 4, 4).await.revision, blueprint::revision(&child(4)));
    let back = rig.engine.put_back(rig.request(PARENT, json!({})), true).await.unwrap_err();
    assert!(back.message.contains("texts are followed here"), "{back}");
    // And an apply is held to the texts as they are, not to any the tree once reached.
    rig.library.put(
        "child",
        "blueprint: child\nversion: 5\nparams:\n  place: { type: string }\nsteps:\n  go: { do: test.echo }\n",
    );
    let unfit = rig.engine.apply(rig.request(PARENT, json!({}))).await.unwrap_err();
    assert!(unfit.message.contains("needs param `place`"), "{unfit}");
}

#[tokio::test]
async fn with_nothing_said_a_text_that_did_not_come_with_the_program_is_followed_and_one_that_did_is_held() {
    // A library of files somebody writes where the factory runs, as a checkout's blueprints are.
    let rig = Rig::new().await;
    let id = kept(&rig).await;
    let second = keeper(2, "claimed", "new");
    rig.library.put("keeper", &second);
    assert!(rig.engine.follow(None, Some("keeper")).await.is_empty());
    assert_eq!(rig.instance(&id).await.revision, blueprint::revision(&second), "taken as it changed");
    assert_eq!(rig.engine.taking().come, None, "and nobody said anything");
    // Said to be held, the same library's texts are held like any that ship.
    rig.engine.set_taking(take(Some("held"), None), &user()).await.unwrap();
    rig.library.put("keeper", &keeper(3, "claimed", "newer"));
    assert!(rig.engine.follow(None, Some("keeper")).await.is_empty());
    assert_eq!(rig.instance(&id).await.revision, blueprint::revision(&second));
}
