//! What a project's fork of a blueprint that ships was forked from: kept
//! with the saying of it, read whatever ships by then, and forgotten
//! with the saying that the fork is gone.

use conductor::Library;
use conductor::forked::{NAMESPACE, Origin, key};
use kernel::Source;
use serde_json::json;

use crate::{Rig, user};

const CHILD: &str = "\
blueprint: child
version: 1
params: { id: 0 }
steps:
  claim: { do: test.count, with: { key: old } }
";

const LATER: &str = "\
blueprint: child
version: 2
params:
  id: 0
  place: { type: string }
steps:
  claim: { do: test.count, with: { key: new } }
";

fn origin(rig: &Rig) -> Origin {
    Origin::of("child", CHILD, "blueprints/child.bp", &user(), rig.journal.now_ms())
}

#[tokio::test]
async fn the_text_a_fork_was_made_from_is_read_and_compared_after_the_program_brings_another() {
    let mut rig = Rig::new().await;
    rig.library.ship();
    rig.library.put("child", CHILD);
    let applied = rig.apply(CHILD, json!({})).await;
    rig.finished(&applied).await;
    rig.engine.fork_made("shop", origin(&rig), Source::actor(user().actor())).await.unwrap();
    let said = rig.event("project.forked", |event| event.data["project"] == "shop").await;
    assert_eq!((&said.data["blueprint"], &said.data["version"]), (&json!("child"), &json!("1")));
    assert_eq!(said.data["revision"], json!(blueprint::revision(CHILD)));
    assert!(said.data.get("source").is_none(), "the text is kept, and not said: {}", said.data);

    // Another program: what ships under the name is another version, and
    // the conductor starts on what was kept, twice. Nothing a start tidies
    // takes it: it is kept under a name of its own and not among the
    // texts trees are held to, which go when no tree is.
    rig.library.put("child", LATER);
    rig.restart().await;
    rig.restart().await;
    let kept = rig.engine.forked_from("shop", "child").await.expect("it is still kept");
    assert_eq!(kept.source, CHILD, "to the letter");
    assert_eq!((kept.version.as_deref(), kept.by_name.as_str()), (Some("1"), user().name.as_str()));
    let fork = CHILD.replace("old", "mine");
    assert_eq!(conductor::differences(&kept.source, &fork), ["the step `claim` is changed"]);
    let now = rig.library.load("child", None).unwrap();
    assert_eq!(blueprint::version(&now).as_deref(), Some("2"));
    assert_eq!(
        conductor::differences(&kept.source, &now),
        ["the param `place` is new", "the step `claim` is changed"]
    );
    // Another project's fork of the same name is another's.
    assert!(rig.engine.forked_from("yard", "child").await.is_none());
}

#[tokio::test]
async fn a_fork_that_is_leaving_says_nothing_and_one_that_is_gone_is_forgotten_with_the_saying_of_it() {
    let rig = Rig::new().await;
    rig.engine.fork_made("shop", origin(&rig), Source::default()).await.unwrap();
    // Marked before its file goes: what is kept no longer stands, and nothing is said yet.
    let was = rig.engine.fork_leaving("shop", "child", true).await.expect("what was kept");
    assert!(!was.leaving && rig.engine.forked_from("shop", "child").await.is_none());
    assert!(rig.events("project.unforked").await.is_empty());
    // The file could not be taken away: it stands again.
    rig.engine.fork_leaving("shop", "child", false).await;
    assert_eq!(rig.engine.forked_from("shop", "child").await, Some(was.clone()));

    rig.engine.fork_leaving("shop", "child", true).await;
    let by = (&user(), Source::actor(user().actor()));
    rig.engine.fork_gone("shop", "child", ("blueprints/child.bp", true), Some(&was), by).await.unwrap();
    let said = rig.event("project.unforked", |event| event.data["project"] == "shop").await;
    assert_eq!(
        (&said.data["blueprint"], &said.data["version"], &said.data["lost"]),
        (&json!("child"), &json!("1"), &json!(true))
    );
    assert_eq!(rig.journal.kv_get(NAMESPACE, &key("shop", "child")).await.unwrap(), None);
    // One of which nothing was kept is marked as nothing, and nothing is written for it.
    assert!(rig.engine.fork_leaving("shop", "other", true).await.is_none());
    assert_eq!(rig.journal.kv_list(NAMESPACE).await.unwrap(), []);
}

#[tokio::test]
async fn a_fork_moved_into_the_factorys_place_says_where_it_is_now_and_nothing_else_and_one_kept_elsewhere_is_left()
 {
    let mut rig = Rig::new().await;
    rig.engine.fork_made("shop", origin(&rig), Source::actor(user().actor())).await.unwrap();
    let mark = rig.mark();
    // Asked of where it is not: nothing moves.
    let elsewhere = rig.engine.fork_moved("shop", "child", ("tools/child.bp", "@blueprints/child.bp")).await;
    assert!(!elsewhere.unwrap());
    assert_eq!(rig.engine.forked_from("shop", "child").await.unwrap().path, "blueprints/child.bp");
    // Asked of where it is: the record says the new place, and is as it was in every other way.
    let moved = rig.engine.fork_moved("shop", "child", ("blueprints/child.bp", "@blueprints/child.bp")).await;
    assert!(moved.unwrap());
    let kept = rig.engine.forked_from("shop", "child").await.unwrap();
    assert_eq!(kept, Origin { path: "@blueprints/child.bp".into(), at: kept.at, ..origin(&rig) });
    // Nothing was said: the fork is the fork it was.
    let said = rig.events("project.*").await;
    assert!(said.iter().all(|event| event.seq <= mark), "{said:?}");
    rig.restart().await;
    assert_eq!(rig.engine.forked_from("shop", "child").await.unwrap().path, "@blueprints/child.bp");
    // A project that has no record has nothing to move.
    assert!(
        !rig.engine
            .fork_moved("yard", "child", ("blueprints/child.bp", "@blueprints/child.bp"))
            .await
            .unwrap()
    );
    assert!(rig.journal.kv_get(NAMESPACE, &key("yard", "child")).await.unwrap().is_none());
}
