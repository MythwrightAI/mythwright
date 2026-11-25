//! An agent taken away for good (`agents::removed`): who may, what is in
//! the way, what goes with it, what stays, and that a later agent of its
//! name is no heir of it.

use agents::{Order, Removed, TurnRequest};
use conductor::Apply;
use kernel::{ErrorKind, Principal, PrincipalKind};
use serde_json::json;

use crate::{Rig, ada, soon};

fn work(slot: &str, thread: &str, prompt: &str) -> TurnRequest {
    TurnRequest {
        slot: slot.into(),
        kind: "task".into(),
        thread: thread.into(),
        prompt: prompt.into(),
        ..Default::default()
    }
}

/// A person who holds `scopes` and nothing else.
fn person(name: &str, scopes: &[&str]) -> Principal {
    Principal {
        kind: PrincipalKind::User,
        id: name.to_lowercase(),
        name: name.into(),
        scopes: scopes.iter().map(|scope| scope.to_string()).collect(),
        key: None,
    }
}

/// What the factory's small durable state keeps under `namespace` whose
/// key begins as `start` does.
async fn kept(rig: &Rig, namespace: &str, start: &str) -> Vec<String> {
    let all = rig.journal.kv_list(namespace).await.unwrap();
    all.into_iter().map(|(key, _)| key).filter(|key| key.starts_with(start)).collect()
}

/// How many turns there are of an agent called `slot`, in every file.
async fn turns_kept(rig: &Rig, slot: &'static str) -> i64 {
    let counted = rig
        .of_turns(move |connection| {
            connection
                .query_row("SELECT COUNT(*) FROM turns WHERE slot = ?1", [slot], |row| row.get::<_, i64>(0))
        })
        .await;
    counted.into_iter().sum()
}

/// `helper`, asked two things: one in the channel, which it answered
/// there, and a piece of work in a conversation of its own.
async fn asked_twice(rig: &Rig) -> (String, String) {
    rig.agent("helper", "fake").await;
    *rig.fake.names_its_model.lock().unwrap() = Some("big".into());
    rig.fake.will_say("I am here.");
    rig.say("@helper are you there?").await;
    let said = rig.turns_finished(1).await.remove(0);
    let work = rig.agents.enqueue(work("helper", "lane:shop-1", "do the card")).await.unwrap();
    soon("the work to end", rig.agents.wait(&work)).await.unwrap();
    (said.data["turn"].as_str().unwrap().to_string(), work)
}

#[tokio::test]
async fn an_agent_taken_away_is_gone_with_what_was_its_own_and_what_it_said_and_did_stays() {
    let rig = Rig::new().await;
    let (said, worked) = asked_twice(&rig).await;
    // A card given to it, and something in the home its program was shown.
    let card = rig.studio.board.add("shop", "Paint the door", "", None, Vec::new(), &ada()).await.unwrap();
    let given = studio::board::Changes { assignee: Some(Some("agent:helper".into())), ..Default::default() };
    rig.studio.board.update(&card.id, given, &ada()).await.unwrap();
    let folder = rig.dir.join("slots/helper");
    std::fs::create_dir_all(folder.join("home")).unwrap();
    std::fs::write(folder.join("home/notes"), "what its program kept").unwrap();
    let token = std::fs::read_to_string(folder.join("token")).unwrap();
    assert!(rig.studio.identity.authenticate(token.trim()).await.is_ok());
    assert_eq!(kept(&rig, "agent.session", "helper/").await.len(), 2, "a conversation each, remembered");
    assert_eq!(kept(&rig, "agent.model", "helper").await, ["helper"]);

    // Asked about first, it says what would go and changes nothing.
    let looked = rig.agents.remove("helper", &ada(), &rig.engine, true).await.unwrap();
    let would = Removed {
        name: "helper".into(),
        project: Some("shop".into()),
        active: true,
        channels: vec!["shop".into()],
        turns: 2,
        cards: vec![card.id.clone()],
        ..Removed::default()
    };
    assert_eq!(looked, would);
    assert!(rig.agents.slot("helper").await.is_ok());
    assert!(rig.studio.identity.authenticate(token.trim()).await.is_ok());

    // Taken away.
    let mut left = rig.from_now("channel.left");
    let mut gone = rig.from_now("agent.slot.removed");
    let removed = rig.agents.remove("helper", &ada(), &rig.engine, false).await.unwrap();
    assert_eq!(removed, Removed { removed: true, ..would });
    let gone = soon("it to be said", gone.next()).await.unwrap();
    assert_eq!(gone.source.actor.as_deref(), Some("user:ada"));
    assert_eq!(gone.data, json!({ "slot": "helper", "turns": 2, "channels": ["shop"], "project": "shop" }));
    let left = soon("it to be said to have left", left.next()).await.unwrap();
    assert_eq!(
        (left.data["member"].as_str(), left.source.actor.as_deref()),
        (Some("agent:helper"), Some("agent:helper"))
    );

    // What was its own is gone: its slot, its credential and the file it
    // was in, its place in the channel, what its program remembered, and
    // its folder.
    assert_eq!(rig.agents.slot("helper").await.unwrap_err().kind, ErrorKind::NotFound);
    assert!(rig.agents.slots().await.unwrap().iter().all(|slot| slot.name != "helper"));
    assert!(rig.studio.identity.authenticate(token.trim()).await.is_err());
    assert!(!folder.exists(), "the folder that was its own");
    let channel = rig.studio.channels.get(&crate::own(), "shop").await.unwrap();
    assert!(channel.members.iter().all(|member| member.actor != "agent:helper"), "{:?}", channel.members);
    assert_eq!(kept(&rig, "agent.session", "helper/").await, Vec::<String>::new());
    assert_eq!(kept(&rig, "agent.model", "helper").await, Vec::<String>::new());
    assert_eq!(rig.agents.available(Some("shop"), None).await.unwrap(), None);
    // Nothing is given to it, and nothing said to it is heard.
    let refused = rig.agents.enqueue(work("helper", "x", "more")).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::NotFound);
    rig.say("@helper and now?").await;

    // What it said and did stays, and names it: what it said in the
    // channel, what it was asked and answered, and the card given to it.
    let transcript = rig.transcript().await;
    assert!(transcript.contains(&"helper: I am here.".to_string()), "{transcript:?}");
    for id in [&said, &worked] {
        let turn = rig.agents.turn(id).await.unwrap();
        assert_eq!((turn.slot.as_str(), turn.status.as_str()), ("helper", "done"), "{id}");
    }
    assert_eq!(turns_kept(&rig, "helper").await, 2);
    assert_eq!(rig.studio.board.get(&card.id).await.unwrap().assignee.as_deref(), Some("agent:helper"));
    // Its work is still listed under its name, while nobody else has it.
    assert_eq!(rig.agents.turns("helper", 10).await.unwrap().len(), 2);

    // And there is nothing to take away a second time.
    let again = rig.agents.remove("helper", &ada(), &rig.engine, false).await.unwrap_err();
    assert_eq!(again.kind, ErrorKind::NotFound);
    assert!(again.message.contains("there is no agent called `helper`"), "{}", again.message);
}

#[tokio::test]
async fn an_agent_is_taken_away_by_a_person_who_manages_the_factory_and_by_nobody_else() {
    let rig = Rig::new().await;
    rig.agent("helper", "fake").await;
    let card = rig.studio.board.add("shop", "Paint the door", "", None, Vec::new(), &ada()).await.unwrap();
    let given = studio::board::Changes { assignee: Some(Some("agent:helper".into())), ..Default::default() };
    rig.studio.board.update(&card.id, given, &ada()).await.unwrap();
    // Its own credential, whatever it holds; another agent; the daemon itself.
    let itself =
        Principal { scopes: vec!["*".into()], ..rig.agents.slot("helper").await.unwrap().principal() };
    let another = Principal { id: "other".into(), name: "other".into(), ..itself.clone() };
    for nobody in [itself, another, Principal::system()] {
        for look in [true, false] {
            let refused = rig.agents.remove("helper", &nobody, &rig.engine, look).await.unwrap_err();
            assert_eq!(refused.kind, ErrorKind::Denied);
            let why = format!("taking an agent away is a person's to do, and {} is not one", nobody.name);
            assert!(refused.message.starts_with(&why), "{}", refused.message);
            assert!(refused.message.contains("agent remove helper"), "{}", refused.message);
        }
    }
    // A person who may not change agents, in the words that always says;
    // and one who may, and does not manage the factory.
    let reader = person("Rhea", &["agents:read", "admin"]);
    let refused = rig.agents.remove("helper", &reader, &rig.engine, false).await.unwrap_err();
    assert_eq!(
        (refused.kind, refused.message.as_str()),
        (ErrorKind::Denied, "Rhea is not allowed to do this (needs `agents:write`)")
    );
    let member = person("Mia", kernel::principal::MEMBER_SCOPES);
    let refused = rig.agents.remove("helper", &member, &rig.engine, false).await.unwrap_err();
    assert_eq!(
        (refused.kind, refused.message.as_str()),
        (ErrorKind::Denied, "Mia is not allowed to do this (needs `admin`)")
    );
    assert!(rig.agents.slot("helper").await.unwrap().active, "it is as it was");
    assert_eq!(rig.events("agent.slot.removed").await.len(), 0);

    // One who holds both, and no more of anything, may. The card given
    // to it is on a board they may not read, and is not told them.
    let manager = person("Noor", &["agents:write", "admin"]);
    let looked = rig.agents.remove("helper", &ada(), &rig.engine, true).await.unwrap();
    assert_eq!(looked.cards.as_slice(), std::slice::from_ref(&card.id));
    let removed = rig.agents.remove("helper", &manager, &rig.engine, false).await.unwrap();
    assert!(removed.removed);
    assert_eq!(removed.cards, Vec::<String>::new());
}

#[tokio::test]
async fn one_with_work_in_hand_is_not_taken_away_until_it_is_stopped_or_has_finished() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    // One thing at work, and one waiting behind it.
    let first = rig.agents.enqueue(work("builder", "a", "the first [gate:first]")).await.unwrap();
    let second = rig.agents.enqueue(work("builder", "b", "the second")).await.unwrap();
    rig.fake.saw(1).await;
    let refused = rig.agents.remove("builder", &ada(), &rig.engine, false).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    let why = format!(
        "`builder` has 2 pieces of work in hand, and taking an agent away is not a way of calling work off. Stop \
         it first, which ends what it is doing and calls off what waits behind that (`{cli} agent stop builder`), \
         or let it finish what it has (`{cli} agent stop builder --idle`); then take it away",
        cli = brand::CLI
    );
    assert_eq!(refused.message, why);
    // Asked about, the same is said and is no error.
    let looked = rig.agents.remove("builder", &ada(), &rig.engine, true).await.unwrap();
    assert_eq!((looked.removed, looked.refused.as_deref()), (false, Some(why.as_str())));
    // Nothing of it went: it goes on with what it has.
    rig.fake.open("first");
    for id in [&first, &second] {
        let turn = soon("its work to end", rig.agents.wait(id)).await.unwrap();
        assert_eq!((turn.status.as_str(), turn.error), ("done", None), "{id}");
    }

    // Stopped with something in hand, it has nothing in hand once that
    // has ended.
    let third = rig.agents.enqueue(work("builder", "c", "the third [gate:third]")).await.unwrap();
    rig.fake.saw(3).await;
    rig.agents.deactivate("builder").await.unwrap();
    let turn = soon("what it had to end", rig.agents.wait(&third)).await.unwrap();
    assert_eq!(turn.status, "failed");

    // One whose answer is kept and not yet said to have ended has it in
    // hand still, as everywhere work in hand is asked for. Such a turn
    // is a daemon's to find as it starts, and is planted with nothing
    // at work here to take it up: an agent's worker ends one it finds.
    rig.agents.halt();
    let said = async |status: &'static str| {
        let lookup = second.clone();
        let asked = move |connection: &mut rusqlite::Connection| {
            connection.execute("UPDATE turns SET status = ?2 WHERE id = ?1", [lookup.as_str(), status])
        };
        rig.turns_of("shop").db().call(asked).await.unwrap();
    };
    said("answered").await;
    let refused = rig.agents.remove("builder", &ada(), &rig.engine, false).await.unwrap_err();
    assert!(refused.message.starts_with("`builder` has 1 piece of work in hand"), "{}", refused.message);
    said("done").await;

    // With nothing in hand, it is taken away.
    let removed = rig.agents.remove("builder", &ada(), &rig.engine, false).await.unwrap();
    assert_eq!((removed.removed, removed.active, removed.turns), (true, false, 3));
}

/// Applies the role that ships for the project, so many of them.
async fn role(rig: &Rig, count: i64) {
    let request = Apply {
        params: kernel::fields(json!({ "role": "worker", "count": count, "adapter": "fake" })),
        key: Some("workers".into()),
        project: Some("shop".into()),
        ..Apply::new("std/role", ada())
    };
    let applied = rig.engine.apply(request).await.unwrap_or_else(|err| panic!("{}", err.message));
    let run = soon("the role to be brought up", rig.engine.wait_run(applied.run.as_ref().unwrap())).await;
    assert_eq!(run.unwrap().status, conductor::RunStatus::Ok);
}

#[tokio::test]
async fn one_that_something_running_counts_is_changed_where_it_is_counted_and_taken_away_once_it_is_not() {
    let rig = Rig::new().await;
    rig.agent("helper", "fake").await;
    role(&rig, 2).await;
    let holder =
        rig.engine.instances(conductor::InstanceFilter { live_only: true, limit: 10, ..Default::default() });
    let holder = holder.await.unwrap().into_iter().find(|one| one.key == "workers").unwrap();
    let refused = rig.agents.remove("worker-2", &ada(), &rig.engine, false).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    let why = format!(
        "`worker-2` is brought up by `workers` (std/role, {id}), which is running, counts it, and would bring it up \
         again. An agent of a fleet is changed in its row: in the `agents` table of the project `shop`, lower the \
         row's count or take the row away, and an agent that is no longer counted can be taken away. One that \
         something else brought up goes when that is stopped (`{cli} stop {id}`)",
        id = holder.id,
        cli = brand::CLI
    );
    assert_eq!(refused.message, why);
    let looked = rig.agents.remove("worker-2", &ada(), &rig.engine, true).await.unwrap();
    assert_eq!(looked.refused.as_deref(), Some(why.as_str()));
    assert!(rig.agents.slot("worker-2").await.unwrap().active);
    // It goes by what holds an agent and not by what one is called: one
    // added by hand under a fleet's kind of name is nobody's but its own.
    rig.agent("worker-9", "fake").await;
    assert!(rig.agents.remove("worker-9", &ada(), &rig.engine, false).await.unwrap().removed);

    // Counted no longer, it has left and nothing holds it: it is taken
    // away, and the one still counted is not.
    role(&rig, 1).await;
    assert!(!rig.agents.slot("worker-2").await.unwrap().active);
    assert!(rig.agents.remove("worker-2", &ada(), &rig.engine, false).await.unwrap().removed);
    let refused = rig.agents.remove("worker-1", &ada(), &rig.engine, false).await.unwrap_err();
    assert!(refused.message.starts_with("`worker-1` is brought up by `workers`"), "{}", refused.message);

    // Counted in again, there is an agent of that name once more: the
    // name was free, and what brought the first up brings this one up.
    role(&rig, 2).await;
    assert!(rig.agents.slot("worker-2").await.unwrap().active);

    // What brought them up stopped, they are stopped and nobody's.
    rig.engine.stop(&holder.id).await.unwrap();
    for name in ["worker-1", "worker-2"] {
        let removed = rig.agents.remove(name, &ada(), &rig.engine, false).await.unwrap();
        assert_eq!((removed.removed, removed.active), (true, false), "{name}");
    }
}

/// What a choice between the project's agents would go by, for `name`.
async fn given(rig: &Rig, name: &str) -> (i64, i64) {
    let could = rig.agents.could_take(Some("shop"), None, &[], Order::First).await.unwrap();
    let one = could.into_iter().find(|one| one.slot.name == name).unwrap();
    (one.given, one.last)
}

#[tokio::test]
async fn a_later_agent_of_the_name_is_no_heir_of_the_one_taken_away() {
    let mut rig = Rig::new().await;
    let (_, worked) = asked_twice(&rig).await;
    let folder = rig.dir.join("slots/helper");
    std::fs::create_dir_all(folder.join("home")).unwrap();
    std::fs::write(folder.join("home/notes"), "what its program kept").unwrap();
    let token = std::fs::read_to_string(folder.join("token")).unwrap();
    assert_eq!(
        rig.agents.holding("lane:shop-1").await.unwrap().map(|slot| slot.name).as_deref(),
        Some("helper")
    );
    assert_eq!(given(&rig, "helper").await.0, 2);
    // Another agent, asked beside it about the same thing.
    rig.agent("second", "fake").await;
    let beside = rig.agents.enqueue(work("second", "lane:shop-2", "the other card")).await.unwrap();
    soon("the work to end", rig.agents.wait(&beside)).await.unwrap();
    assert_eq!(rig.agents.beside("lane:", "lane:shop-2").await.unwrap(), ["helper"]);

    rig.agents.remove("helper", &ada(), &rig.engine, false).await.unwrap();
    assert_eq!(rig.agents.holding("lane:shop-1").await.unwrap(), None);

    // An agent is made and called what that one was called.
    rig.agent("helper", "fake").await;
    let slot = rig.agents.slot("helper").await.unwrap();
    assert!(slot.active);
    // It has a credential of its own, and the first one's is still nobody's.
    let fresh = std::fs::read_to_string(folder.join("token")).unwrap();
    assert_ne!(fresh, token);
    assert!(rig.studio.identity.authenticate(token.trim()).await.is_err());
    assert_eq!(rig.studio.identity.authenticate(fresh.trim()).await.unwrap().actor(), "agent:helper");
    // Nothing the first kept in its folder is there for it.
    assert!(!folder.join("home/notes").exists());
    // The conversation the first held is held with nobody, and is not
    // one held beside another's.
    assert_eq!(rig.agents.holding("lane:shop-1").await.unwrap(), None);
    assert_eq!(rig.agents.beside("lane:", "lane:shop-2").await.unwrap(), Vec::<String>::new());
    // It was given nothing, and has done no work: the first one's turns
    // are there, and are not its own.
    assert_eq!(given(&rig, "helper").await, (0, 0));
    assert_eq!(rig.agents.turns("helper", 10).await.unwrap(), Vec::new());
    assert_eq!(turns_kept(&rig, "helper").await, 2);
    assert_eq!(rig.agents.turn(&worked).await.unwrap().status, "done");

    // Asked in the conversation the first held, it begins it: its
    // program is handed nothing the first one's remembered.
    let before = rig.fake.seen.lock().unwrap().len();
    let own = rig.agents.enqueue(work("helper", "lane:shop-1", "do the card again")).await.unwrap();
    soon("the work to end", rig.agents.wait(&own)).await.unwrap();
    let seen = rig.fake.seen.lock().unwrap()[before].clone();
    assert_eq!((seen.slot.as_str(), seen.session), ("helper", None));
    // From then on the conversation is its own, with what it was given.
    assert_eq!(
        rig.agents.holding("lane:shop-1").await.unwrap().map(|slot| slot.name).as_deref(),
        Some("helper")
    );
    assert_eq!(rig.agents.beside("lane:", "lane:shop-2").await.unwrap(), ["helper"]);
    let listed = rig.agents.turns("helper", 10).await.unwrap();
    assert_eq!(listed.iter().map(|turn| turn.id.as_str()).collect::<Vec<_>>(), [own.as_str()]);
    let counted = given(&rig, "helper").await;
    assert_eq!(counted.0, 1);

    // And a daemon that starts counts what it was given the same: what
    // each was given is read again from the turns there are.
    rig.restart().await;
    assert_eq!(given(&rig, "helper").await, counted);
    assert_eq!(given(&rig, "second").await.0, 1);
    assert_eq!(rig.agents.turns("helper", 10).await.unwrap().len(), 1);
}
