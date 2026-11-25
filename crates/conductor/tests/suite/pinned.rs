//! What a running tree reads of the texts that ship: the ones it reached
//! when somebody applied it, whatever program is running it since, until
//! somebody says to take them as they now are.

use conductor::{Apply, InstanceFilter, InstanceInfo, Moved};
use kernel::Source;
use serde_json::json;

use crate::handlers::{active, handled};
use crate::{Rig, user};

/// What somebody applies: for each thing that is made ready it starts a
/// child by name, handing it what the child of its day took.
const PARENT: &str = "\
blueprint: parent
on:
  ready:
    event: board.item.moved
    steps:
      go: { spawn: child, as: \"kid-${ event.data.id }\", with: { id: ${ event.data.id } } }
";

/// The child as it shipped when the parent was applied.
const CHILD: &str = "\
blueprint: child
version: 1
params: { id: 0 }
steps:
  claim: { do: test.count, with: { key: old } }
";

/// The child as a later program ships it: it has to be told something the
/// parent of before never said.
const LATER: &str = "\
blueprint: child
version: 2
params:
  id: 0
  place: { type: string }
steps:
  claim: { do: test.count, with: { key: new } }
";

async fn moved(rig: &Rig, id: u64) {
    rig.journal.emit("board.item.moved", Source::default(), json!({ "id": id })).await.unwrap();
}

/// Each text as it is said to have moved: its name, and the version it
/// was and the one it is.
fn versions(moved: &[Moved]) -> Vec<(&str, &str, &str)> {
    fn said(version: &Option<String>) -> &str {
        version.as_deref().unwrap_or("none")
    }
    moved.iter().map(|one| (one.name.as_str(), said(&one.was), said(&one.now))).collect()
}

/// The child started for `id`, once the parent's handler has run `times`
/// and the child's own steps have: a `spawn` does not wait for them.
async fn kid(rig: &Rig, id: u64, times: usize) -> InstanceInfo {
    moved(rig, id).await;
    handled(rig, "on.ready", times).await;
    let key = format!("kid-{id}");
    let all = rig.engine.instances(InstanceFilter::default()).await.unwrap();
    let kid = all.into_iter().find(|one| one.key == key).unwrap_or_else(|| panic!("no `{key}`"));
    let its = kid.id.clone();
    rig.event("run.finished", move |event| event.source.instance.as_deref() == Some(its.as_str())).await;
    kid
}

/// The parent applied while `CHILD` is what ships: its id.
async fn applied(rig: &Rig) -> String {
    // What the library holds came with the program, as the standard ones do.
    rig.library.ship();
    rig.library.put("parent", PARENT);
    rig.library.put("child", CHILD);
    let applied = rig.apply(PARENT, json!({})).await;
    active(rig, &applied.instance).await;
    applied.instance
}

#[tokio::test]
async fn what_a_running_instance_starts_by_name_is_the_text_it_was_applied_with_whatever_ships_now() {
    let rig = Rig::new().await;
    applied(&rig).await;
    assert_eq!(kid(&rig, 1, 1).await.revision, blueprint::revision(CHILD));

    // A new program: under the child's name is a text the parent was never written for.
    rig.library.put("child", LATER);
    let second = kid(&rig, 2, 2).await;
    assert_eq!(second.revision, blueprint::revision(CHILD), "it was started as the parent reached it");
    assert_eq!(rig.shared.count("old"), 2);
    assert_eq!(rig.shared.count("new"), 0);
    // Nothing failed for it. What moved between the two can be said in
    // words, for whoever decides whether to take what ships now.
    assert!(rig.events("run.failed").await.is_empty());
    let moved = conductor::differences(CHILD, LATER);
    for said in ["the param `place` is new", "the step `claim` is changed"] {
        assert!(moved.iter().any(|one| one == said), "{moved:?}");
    }
}

#[tokio::test]
async fn applying_again_leaves_what_it_reaches_and_says_what_changed_until_it_is_told_to_take_it() {
    let rig = Rig::new().await;
    applied(&rig).await;
    kid(&rig, 1, 1).await;
    rig.library.put("child", LATER);

    // Applied again as it is: nothing moves, and whoever asked is told what has.
    let again = rig.engine.apply(rig.request(PARENT, json!({}))).await.unwrap();
    assert!(!again.changed);
    // By the version each calls itself, and then what moved between them.
    assert!(again.reached.taken.is_empty());
    assert_eq!(versions(&again.reached.left), [("child", "1", "2")]);
    assert!(again.reached.left[0].changed.contains(&"the param `place` is new".to_string()));
    assert_eq!(kid(&rig, 2, 2).await.revision, blueprint::revision(CHILD));

    // Told to take the texts as they now are, it is held to them first:
    // the parent of before does not fit the child of now, so it is refused
    // with what does not fit, and nothing has moved.
    let taking = |source: &str| Apply { take: true, ..rig.request(source, json!({})) };
    let refused = rig.engine.apply(taking(PARENT)).await.unwrap_err();
    assert!(refused.message.contains("needs param `place`"), "{refused}");
    assert_eq!(kid(&rig, 3, 3).await.revision, blueprint::revision(CHILD));

    // A parent written for the child of now, applied taking it, moves, and says what to.
    let fitted = PARENT.replace("id: ${ event.data.id } }", "id: ${ event.data.id }, place: here }");
    assert_ne!(fitted, PARENT);
    let taken = rig.engine.apply(taking(&fitted)).await.unwrap();
    assert_eq!((versions(&taken.reached.taken), taken.reached.left.len()), (vec![("child", "1", "2")], 0));
    assert_eq!(kid(&rig, 4, 4).await.revision, blueprint::revision(LATER));
    assert_eq!((rig.shared.count("old"), rig.shared.count("new")), (3, 1));
    assert!(rig.events("run.failed").await.is_empty());
    // Nothing is said to have changed once it has been taken.
    let after = rig.engine.apply(rig.request(&fitted, json!({}))).await.unwrap();
    assert!(after.reached.is_empty(), "{:?}", after.reached);
}

#[tokio::test]
async fn what_was_reached_is_kept_across_a_restart_and_read_though_the_program_no_longer_has_it() {
    let mut rig = Rig::new().await;
    applied(&rig).await;
    kid(&rig, 1, 1).await;
    rig.library.put("child", LATER);
    rig.restart().await;
    assert_eq!(kid(&rig, 2, 2).await.revision, blueprint::revision(CHILD));
    assert_eq!(rig.shared.count("new"), 0);
}

#[tokio::test]
async fn a_tree_applied_before_anything_was_pinned_goes_on_with_what_its_project_last_ran() {
    let mut rig = Rig::new().await;
    let parent = applied(&rig).await;
    kid(&rig, 1, 1).await;
    // As a program from before left it: nothing pinned, and the last text
    // of each name that checked remembered.
    rig.journal.kv_set("pinned", &parent, &serde_json::Value::Null).await.unwrap();
    rig.library.put("child", LATER);
    rig.restart().await;
    assert_eq!(kid(&rig, 2, 2).await.revision, blueprint::revision(CHILD));
    assert!(rig.events("run.failed").await.is_empty());
}

/// Something that ships and is applied by its name, which names another.
const OUTER: &str = "\
blueprint: outer
on:
  asked:
    event: test.poke
    steps:
      go: { call: inner, as: inner, with: { word: ${ event.data.word } } }
";
const INNER: &str = "\
blueprint: inner
params: { word: x }
steps:
  say: { do: test.count, with: { key: \"inner-old\" } }
";
/// The two as a later program ships them, written for each other.
const OUTER_LATER: &str = "\
blueprint: outer
on:
  asked:
    event: test.poke
    steps:
      go: { call: inner, as: inner, with: { word: ${ event.data.word }, loud: true } }
";
const INNER_LATER: &str = "\
blueprint: inner
params:
  word: x
  loud: { type: bool }
steps:
  say: { do: test.count, with: { key: \"inner-new\" } }
";

async fn poke(rig: &Rig, word: &str) {
    rig.journal.emit("test.poke", Source::default(), json!({ "word": word })).await.unwrap();
}

#[tokio::test]
async fn an_instance_that_comes_to_another_text_that_ships_takes_what_that_text_names_with_it() {
    let rig = Rig::new().await;
    rig.library.ship();
    rig.library.put("outer", OUTER);
    rig.library.put("inner", INNER);
    let by_name = || Apply::new("outer", user());
    let first = rig.engine.apply(by_name()).await.unwrap();
    active(&rig, &first.instance).await;
    poke(&rig, "one").await;
    handled(&rig, "on.asked", 1).await;
    let failed = rig.events("run.failed").await;
    assert!(failed.is_empty(), "{:?}", failed.last().map(|event| &event.data));
    assert_eq!(rig.shared.count("inner-old"), 1);

    // A new program ships both. Until somebody applies, the old one calls the old one.
    rig.library.put("outer", OUTER_LATER);
    rig.library.put("inner", INNER_LATER);
    poke(&rig, "two").await;
    handled(&rig, "on.asked", 2).await;
    assert_eq!((rig.shared.count("inner-old"), rig.shared.count("inner-new")), (2, 0));

    // Applied by its name, it comes to the text that ships now, and what
    // that names comes with it: the two were written for each other.
    let moved = rig.engine.apply(by_name()).await.unwrap();
    assert!(moved.changed);
    assert_eq!(moved.reached.taken.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["inner"]);
    active(&rig, &moved.instance).await;
    poke(&rig, "three").await;
    handled(&rig, "on.asked", 3).await;
    assert_eq!((rig.shared.count("inner-old"), rig.shared.count("inner-new")), (2, 1));
    assert!(rig.events("run.failed").await.is_empty());
}

/// The parent as somebody rewrote it for the child of now.
fn fitted() -> String {
    let fitted = PARENT.replace("id: ${ event.data.id } }", "id: ${ event.data.id }, place: here }");
    assert_ne!(fitted, PARENT);
    fitted
}

#[tokio::test]
async fn a_tree_that_was_moved_is_put_back_on_the_texts_it_had_and_says_what_it_put_back() {
    let rig = Rig::new().await;
    applied(&rig).await;
    kid(&rig, 1, 1).await;
    // Nothing was moved yet, so there is nothing to go back to.
    let unmoved = rig.engine.put_back(rig.request(PARENT, json!({})), false).await.unwrap_err();
    assert!(unmoved.message.contains("has not been moved to other texts"), "{unmoved}");

    rig.library.put("child", LATER);
    let fitted = fitted();
    rig.engine.apply(Apply { take: true, ..rig.request(&fitted, json!({})) }).await.unwrap();
    assert_eq!(kid(&rig, 2, 2).await.revision, blueprint::revision(LATER));

    // The parent written for the child of now does not fit the one of
    // before: put back as it is, it is refused with what does not fit.
    let unfit = rig.engine.put_back(rig.request(&fitted, json!({})), false).await.unwrap_err();
    assert!(
        unfit.message.contains("does not fit them") && unfit.message.contains("has no param `place`"),
        "{unfit}"
    );
    assert_eq!(kid(&rig, 3, 3).await.revision, blueprint::revision(LATER), "nothing was changed");

    // What would be put back is said first, and saying it changes nothing.
    let would = rig.engine.put_back(rig.request(PARENT, json!({})), true).await.unwrap();
    assert_eq!(versions(&would.reached.taken), [("child", "2", "1")]);
    assert_eq!(kid(&rig, 4, 4).await.revision, blueprint::revision(LATER));

    // Put back with the parent of before: the tree is on the child it had,
    // though the program still ships the later one.
    let back = rig.engine.put_back(rig.request(PARENT, json!({})), false).await.unwrap();
    assert_eq!(versions(&back.reached.taken), [("child", "2", "1")]);
    assert_eq!(kid(&rig, 5, 5).await.revision, blueprint::revision(CHILD));
    assert!(rig.events("run.failed").await.is_empty());
    // And it says, as before it was moved, that what ships is another.
    let again = rig.engine.apply(rig.request(PARENT, json!({}))).await.unwrap();
    assert_eq!(versions(&again.reached.left), [("child", "1", "2")]);

    // Put back once more, it is where it was put back from.
    let forth = rig.engine.put_back(rig.request(&fitted, json!({})), false).await.unwrap();
    assert_eq!(versions(&forth.reached.taken), [("child", "1", "2")]);
    assert_eq!(kid(&rig, 6, 6).await.revision, blueprint::revision(LATER));
}

#[tokio::test]
async fn an_instance_whose_own_text_ships_goes_back_with_what_it_names() {
    let rig = Rig::new().await;
    rig.library.ship();
    rig.library.put("outer", OUTER);
    rig.library.put("inner", INNER);
    let by_name = || Apply::new("outer", user());
    let first = rig.engine.apply(by_name()).await.unwrap();
    active(&rig, &first.instance).await;
    rig.library.put("outer", OUTER_LATER);
    rig.library.put("inner", INNER_LATER);
    // Applied by its name it moves, and what it names with it.
    assert!(rig.engine.apply(by_name()).await.unwrap().changed);
    poke(&rig, "one").await;
    handled(&rig, "on.asked", 1).await;
    assert_eq!((rig.shared.count("inner-old"), rig.shared.count("inner-new")), (0, 1));

    // Put back, it runs the text it had and calls the one that was written for.
    let back = rig.engine.put_back_instance(&first.instance, user(), false).await.unwrap();
    assert_eq!(back.reached.taken.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["inner"]);
    assert_eq!(rig.instance(&first.instance).await.revision, blueprint::revision(OUTER));
    poke(&rig, "two").await;
    handled(&rig, "on.asked", 2).await;
    assert_eq!((rig.shared.count("inner-old"), rig.shared.count("inner-new")), (1, 1));
    assert!(rig.events("run.failed").await.is_empty());
}
