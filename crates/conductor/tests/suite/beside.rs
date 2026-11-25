//! What a conductor keeps beside its rows is kept where its rows are:
//! which text of each name that ships a tree reached, what a handler
//! last read and the last text that checked are in the journal of the
//! project the tree was applied for, where that project has a conductor
//! of its own, and nothing of them is in the factory's. What a program
//! from before kept in the factory's journal for such a tree is brought
//! over once, as the project's conductor starts.

use conductor::{Apply, Engine, InstanceFilter};
use journal::Journal;
use kernel::Source;
use runs::beside::{FITS, KEPT, OWED, PINNED, POLL, TEXTS, Whose, instance_of, project_of};
use serde_json::{Value, json};

use crate::handlers::{active, handled};
use crate::{Rig, registry};

/// What somebody applies for the project `shop`: for each thing made
/// ready it starts a child by name, and it reads something on a timer.
const PARENT: &str = "\
blueprint: parent
on:
  ready:
    event: board.item.moved
    steps:
      go: { spawn: child, as: \"kid-${ event.data.id }\", with: { id: ${ event.data.id } } }
  read:
    poll: { every: 30s, do: test.echo, with: { v: 1 } }
    do: test.count
    with: { key: read }
";

/// The child as it shipped when the parent was applied.
const CHILD: &str = "\
blueprint: child
version: 1
params: { id: 0 }
steps:
  claim: { do: test.count, with: { key: old } }
";

/// The child as a later program ships it.
const LATER: &str = "\
blueprint: child
version: 2
params: { id: 0 }
steps:
  claim: { do: test.count, with: { key: new } }
";

/// A rig whose journal keeps its projects' beside it, with a journal
/// for the project `shop`, which so has a conductor of its own.
async fn rig() -> (Rig, Journal) {
    let rig = Rig::new().await;
    rig.keeps_projects().await;
    let shop = rig.journal.own("shop").await.unwrap();
    (rig, shop)
}

/// The parent applied for `shop`, its handlers listening.
async fn applied(rig: &Rig) -> String {
    rig.library.put("parent", PARENT);
    rig.library.put("child", CHILD);
    let asked = Apply { project: Some("shop".into()), ..rig.request(PARENT, json!({})) };
    let instance = rig.engine.apply(asked).await.unwrap().instance;
    active(rig, &instance).await;
    instance
}

/// Has a thing made ready in `shop` and gives which text the child that
/// was started for it runs, once its steps have.
async fn kid(rig: &Rig, shop: &Journal, id: u64, times: usize) -> String {
    shop.emit("board.item.moved", Source::default(), json!({ "project": "shop", "id": id })).await.unwrap();
    handled(rig, "on.ready", times).await;
    let key = format!("kid-{id}");
    let all = rig.engine.instances(InstanceFilter::default()).await.unwrap();
    let kid = all.into_iter().find(|one| one.key == key).unwrap_or_else(|| panic!("no `{key}`"));
    let its = kid.id.clone();
    rig.event("run.finished", move |event| event.source.instance.as_deref() == Some(its.as_str())).await;
    kid.revision
}

/// Everything a journal keeps in its small durable state: each name,
/// key and value.
async fn kept(journal: &Journal) -> Vec<(String, String, String)> {
    let read = journal.db().call(|connection| {
        let mut statement =
            connection.prepare("SELECT namespace, key, value FROM kv ORDER BY namespace, key")?;
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
    });
    read.await.unwrap()
}

/// The keys a journal keeps under `name`.
async fn keys(journal: &Journal, name: &str) -> Vec<String> {
    kept(journal).await.into_iter().filter(|(kept, ..)| kept == name).map(|(_, key, _)| key).collect()
}

/// Whether the journal's own rows hold the instance `id`.
async fn has(journal: &Journal, id: &str) -> bool {
    let id = id.to_string();
    let read = journal.db().call(move |connection| {
        connection.query_row("SELECT EXISTS (SELECT 1 FROM instances WHERE id = ?1)", [id], |row| row.get(0))
    });
    read.await.unwrap_or(false)
}

/// Holds every name every journal of the rig keeps something under to
/// what the list says of it: it is on the list, or is the journal's
/// own; what is the factory's is in the factory's journal alone; what
/// is an instance's is where that instance's rows are; and what is a
/// project's is where what is applied for that project is run.
async fn held(rig: &Rig) {
    let own = ["journal"];
    for journal in rig.journal.every() {
        let project = journal::project_of(journal.name()).map(str::to_string);
        for (name, key, _) in kept(&journal).await {
            let at = format!("`{name}` `{key}` in the journal `{}`", journal.name());
            if own.contains(&name.as_str()) {
                continue;
            }
            let Some((_, whose, _)) = KEPT.iter().find(|(listed, ..)| *listed == name) else {
                panic!("{at} is on no list of what is kept beside the rows (`runs::beside::KEPT`)");
            };
            match whose {
                Whose::Factory => assert!(project.is_none(), "{at} is the factory's"),
                Whose::Instance => {
                    assert!(
                        has(&journal, instance_of(&key)).await,
                        "{at} is not where its instance's rows are"
                    );
                }
                Whose::Project => assert_eq!(project_of(&key), project.as_deref(), "{at}"),
                Whose::Conductor => {}
            }
        }
    }
}

#[tokio::test]
async fn what_a_tree_reached_and_what_its_handler_last_read_are_kept_in_its_projects_own_journal() {
    if crate::one_journal() {
        return;
    }
    let (mut rig, shop) = rig().await;
    rig.library.ship();
    let since = rig.clock.sleep_count();
    let parent = applied(&rig).await;
    assert_eq!(kid(&rig, &shop, 1, 1).await, blueprint::revision(CHILD));
    // The first reading of what it polls is kept, as what to tell the next from.
    rig.advance_after(since, 1, "33s").await;
    rig.soon("what it read to be kept", async {
        while keys(&shop, POLL).await.is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await;

    // All of it is in the project's journal, by the instance, and the
    // factory's keeps none of it.
    assert_eq!(keys(&shop, PINNED).await, std::slice::from_ref(&parent));
    assert_eq!(keys(&shop, TEXTS).await, [blueprint::revision(CHILD)]);
    assert_eq!(keys(&shop, POLL).await, [format!("{parent}/read")]);
    for name in [PINNED, TEXTS, POLL] {
        assert!(keys(&rig.journal, name).await.is_empty(), "the factory's journal keeps `{name}`");
    }
    held(&rig).await;

    // A later program, with another text under the child's name: the
    // tree goes on with the one it reached, read from where it is kept.
    rig.restart().await;
    rig.library.put("child", LATER);
    active(&rig, &parent).await;
    assert_eq!(kid(&rig, &shop, 2, 2).await, blueprint::revision(CHILD));
    assert_eq!((rig.shared.count("old"), rig.shared.count("new")), (2, 0));
    // And nothing was put away by either conductor's start.
    assert_eq!(keys(&shop, PINNED).await, std::slice::from_ref(&parent));
    assert_eq!(keys(&shop, TEXTS).await, [blueprint::revision(CHILD)]);
    held(&rig).await;

    // Stopped, what was kept for it is put away where it was kept when
    // its conductor next starts.
    rig.engine.stop(&parent).await.unwrap();
    rig.restart().await;
    for name in [PINNED, TEXTS] {
        assert!(keys(&shop, name).await.is_empty(), "`{name}` is kept for a tree that is gone");
    }
}

#[tokio::test]
async fn the_last_text_that_checked_is_kept_where_what_is_applied_for_its_project_is_run() {
    if crate::one_journal() {
        return;
    }
    let (rig, shop) = rig().await;
    // Nothing ships here: a text read by name is what is there, and the
    // last of it that checked is remembered for what is running.
    applied(&rig).await;
    kid(&rig, &shop, 1, 1).await;
    assert_eq!(keys(&shop, FITS).await, ["shop\nchild"]);
    assert!(keys(&rig.journal, FITS).await.is_empty());
    held(&rig).await;
}

/// Moves everything `from` keeps under `name` to `to`, as it stands.
async fn put_in(from: &Journal, to: &Journal, name: &str) {
    for (kept, key, value) in kept(from).await {
        if kept == name {
            let value: Value = serde_json::from_str(&value).unwrap();
            to.kv_set(name, &key, &value).await.unwrap();
            from.kv_set(name, &key, &Value::Null).await.unwrap();
        }
    }
}

#[tokio::test]
async fn what_a_program_from_before_kept_in_the_factorys_journal_is_brought_over_once_as_its_conductor_starts()
 {
    if crate::one_journal() {
        return;
    }
    let (mut rig, shop) = rig().await;
    rig.library.ship();
    let since = rig.clock.sleep_count();
    let parent = applied(&rig).await;
    kid(&rig, &shop, 1, 1).await;
    rig.advance_after(since, 1, "33s").await;
    rig.soon("what it read to be kept", async {
        while keys(&shop, POLL).await.is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    // Something that is nobody's, whose pins are the factory's to keep.
    let nobodys =
        rig.apply(&PARENT.replace("blueprint: parent", "blueprint: other"), json!({})).await.instance;

    // As a program from before left it: all of it in the factory's
    // journal, and the project's journal noting that it may be.
    rig.engine.halt();
    rig.engine.halted().await;
    for name in [PINNED, TEXTS, POLL] {
        put_in(&shop, &rig.journal, name).await;
    }
    shop.kv_set(OWED.0, OWED.1, &json!(true)).await.unwrap();
    rig.restart().await;

    // Brought over, and taken out of the factory's; the note is gone.
    assert_eq!(keys(&shop, PINNED).await, std::slice::from_ref(&parent));
    assert_eq!(keys(&shop, POLL).await, [format!("{parent}/read")]);
    assert_eq!(keys(&shop, TEXTS).await, [blueprint::revision(CHILD)]);
    assert_eq!(keys(&rig.journal, PINNED).await, std::slice::from_ref(&nobodys));
    assert!(keys(&rig.journal, POLL).await.iter().all(|key| instance_of(key) == nobodys));
    assert!(keys(&shop, OWED.0).await.is_empty());
    held(&rig).await;

    // The tree goes on with what it had reached, whatever ships now.
    rig.library.put("child", LATER);
    active(&rig, &parent).await;
    assert_eq!(kid(&rig, &shop, 2, 2).await, blueprint::revision(CHILD));
    assert_eq!((rig.shared.count("old"), rig.shared.count("new")), (2, 0));
}

#[tokio::test]
async fn a_journal_that_had_something_alive_before_this_notes_it_and_one_that_had_nothing_does_not() {
    let mut rig = Rig::on_a_file().await;
    let stays = "blueprint: stays\non:\n  thing: { event: s.thing, do: test.count, with: { key: heard } }\n";
    let applied = rig.apply(stays, json!({})).await;
    active(&rig, &applied.instance).await;
    // A journal made by this program notes nothing.
    assert!(keys(&rig.journal, OWED.0).await.is_empty());

    // The tables as they were before the step that notes it (and so
    // before the one after it), with the instance alive: opened by this
    // program, the journal notes it.
    rig.engine.halt();
    rig.engine.halted().await;
    let undone = rig.journal.db().call(|db| {
        db.execute_batch(crate::ordered::BEFORE_ASKERS_WERE_KEPT)?;
        db.execute_batch(crate::ordered::BEFORE_ENVIRONMENTS)?;
        db.execute_batch(crate::ordered::BEFORE_BEGINNINGS_WERE_WRITTEN)?;
        db.execute_batch(crate::ordered::BEFORE_RUNS_KEPT_THEIR_PLACE)?;
        db.execute_batch(crate::ordered::BEFORE_TEXTS_WERE_KEPT_BY_VERSION)?;
        db.execute_batch(crate::ordered::BEFORE_STEPS_SAID_THEIR_PLACE)?;
        db.execute_batch(crate::BEFORE_KEPT_BESIDE)
    });
    undone.await.unwrap();
    rig.engine =
        Engine::new(rig.journal.clone(), rig.logs.clone(), registry(&rig.shared), rig.library.clone())
            .await
            .unwrap();
    assert_eq!(keys(&rig.journal, OWED.0).await, [OWED.1]);
    // The factory's own conductor has nothing to bring from anywhere,
    // and takes the note off as it starts.
    rig.engine.resume().await.unwrap();
    assert!(keys(&rig.journal, OWED.0).await.is_empty());
    active(&rig, &applied.instance).await;
}
