use agents::{Order, SlotSpec};
use kernel::{ErrorKind, PrincipalKind};

use crate::{Rig, ada};

#[tokio::test]
async fn a_slot_gets_a_credential_only_it_is_pointed_at() {
    let rig = Rig::new().await;
    rig.agent("Builder", "fake").await;
    let slot = rig.agents.slot("builder").await.unwrap();
    assert_eq!(
        (slot.name.as_str(), slot.adapter.as_str(), slot.project.as_deref(), slot.active),
        ("builder", "fake", Some("shop"), true)
    );

    let token_file = rig.dir.join("slots/builder/token");
    let token = std::fs::read_to_string(&token_file).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&token_file).unwrap().permissions().mode() & 0o777,
            0o600,
            "readable by its owner only"
        );
    }
    let who = rig.studio.identity.authenticate(&token).await.unwrap();
    assert_eq!((who.kind, who.actor().as_str()), (PrincipalKind::Agent, "agent:builder"));
    assert!(who.can("board:write") && who.can("projects:write"));
    assert!(
        !who.can("shell") && !who.can("admin") && !who.can("agents:write"),
        "an agent cannot run commands or make agents by default"
    );

    let members: Vec<String> = rig
        .studio
        .channels
        .get(&crate::own(), "shop")
        .await
        .unwrap()
        .members
        .into_iter()
        .map(|m| m.actor)
        .collect();
    assert_eq!(members, ["user:ada", "agent:builder"], "it joined its channel");
}

#[tokio::test]
async fn ensuring_a_slot_again_updates_it_and_keeps_a_credential_that_is_still_right() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    let token_file = rig.dir.join("slots/builder/token");
    let first = std::fs::read_to_string(&token_file).unwrap();

    let spec = SlotSpec {
        name: "builder".into(),
        model: Some("otter".into()),
        instructions: Some("Prefer small PRs.".into()),
        ..SlotSpec::default()
    };
    let slot = rig.agents.ensure(spec, &ada()).await.unwrap();
    assert_eq!((slot.model.as_deref(), slot.instructions.as_str()), (Some("otter"), "Prefer small PRs."));
    assert_eq!(
        (slot.adapter.as_str(), slot.project.as_deref()),
        ("fake", Some("shop")),
        "what was not mentioned is kept"
    );
    assert_eq!(rig.agents.slots().await.unwrap().len(), 1);
    // Applying a fleet again ensures its agents again; one in the middle of
    // its work must not find its token pulled out from under it.
    assert_eq!(std::fs::read_to_string(&token_file).unwrap(), first, "nothing about its rights changed");
    assert!(rig.studio.identity.authenticate(&first).await.is_ok());

    // What it may do changed: the old credential would say the wrong thing.
    let narrower = SlotSpec {
        name: "builder".into(),
        scopes: Some(vec!["channels".into(), "board:read".into()]),
        ..SlotSpec::default()
    };
    rig.agents.ensure(narrower, &ada()).await.unwrap();
    let second = std::fs::read_to_string(&token_file).unwrap();
    assert_ne!(first, second);
    assert!(rig.studio.identity.authenticate(&first).await.is_err(), "the old credential no longer works");
    assert_eq!(rig.studio.identity.authenticate(&second).await.unwrap().scopes, ["channels", "board:read"]);

    // The file was lost: the slot gets a new one rather than staying broken.
    std::fs::remove_file(&token_file).unwrap();
    rig.agents.ensure(SlotSpec { name: "builder".into(), ..SlotSpec::default() }, &ada()).await.unwrap();
    let third = std::fs::read_to_string(&token_file).unwrap();
    assert!(rig.studio.identity.authenticate(&second).await.is_err());
    assert!(rig.studio.identity.authenticate(&third).await.is_ok());

    let topics: Vec<String> = rig.events("agent.slot.*").await.into_iter().map(|event| event.topic).collect();
    assert_eq!(
        topics,
        ["agent.slot.created", "agent.slot.updated", "agent.slot.updated", "agent.slot.updated"]
    );
}

#[tokio::test]
async fn a_slot_cannot_be_given_more_than_its_creator_has_or_an_adapter_that_does_not_exist() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    let builder = rig.agents.slot("builder").await.unwrap().principal();

    // An agent has no right to make agents at all.
    let err = rig
        .agents
        .ensure(SlotSpec { name: "minion".into(), ..SlotSpec::default() }, &builder)
        .await
        .unwrap_err();
    assert_eq!(err.kind, ErrorKind::Denied);

    // And nobody can hand out a scope they do not hold.
    let mut limited = ada();
    limited.scopes = vec!["agents".into(), "channels".into()];
    let spec = SlotSpec {
        name: "minion".into(),
        scopes: Some(vec!["channels".into(), "shell".into()]),
        ..SlotSpec::default()
    };
    let err = rig.agents.ensure(spec, &limited).await.unwrap_err();
    assert_eq!(
        (err.kind, err.message.as_str()),
        (ErrorKind::Denied, "Ada cannot give an agent `shell`, not holding it")
    );

    let spec = SlotSpec { name: "minion".into(), adapter: Some("nonesuch".into()), ..SlotSpec::default() };
    let err = rig.agents.ensure(spec, &ada()).await.unwrap_err();
    assert!(
        err.message.contains("no adapter called `nonesuch`") && err.message.contains("builtin, fake"),
        "{}",
        err.message
    );

    let spec = SlotSpec { name: "minion".into(), project: Some("nowhere".into()), ..SlotSpec::default() };
    assert_eq!(rig.agents.ensure(spec, &ada()).await.unwrap_err().kind, ErrorKind::NotFound);
    assert_eq!(rig.agents.slots().await.unwrap().len(), 1, "none of those created anything");
}

#[tokio::test]
async fn an_agents_process_is_told_where_the_fleet_is_but_is_not_handed_its_token() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    rig.say("hello").await;
    rig.turns_finished(1).await;

    let seen = rig.fake.seen.lock().unwrap()[0].clone();
    let prefix = brand::ENV_PREFIX;
    assert_eq!(seen.env[&format!("{prefix}_URL")], "http://127.0.0.1:7700");
    assert_eq!(seen.env[&format!("{prefix}_SLOT")], "builder");
    assert_eq!(seen.env[&format!("{prefix}_PROJECT")], "shop");
    let token_file = &seen.env[&format!("{prefix}_TOKEN_FILE")];
    let token = std::fs::read_to_string(token_file).unwrap();
    assert!(
        seen.env.values().all(|value| !value.contains(&token)),
        "the credential is in a file, not in the environment"
    );
    assert!(!seen.system.contains(&token) && !seen.prompt.contains(&token));
    assert!(
        seen.env["PATH"].starts_with(&rig.dir.join("bin").to_string_lossy().into_owned()),
        "the client comes first on its PATH"
    );
    assert_eq!(seen.workdir, rig.studio.projects.dir_of("shop").unwrap(), "it works in its project");

    // It is told who it is and how to act, in the product's own words.
    assert!(seen.system.contains("You are `builder`"));
    assert!(seen.system.contains(&format!("{} project create", brand::CLI)));
}

#[tokio::test]
async fn a_stopped_slot_takes_no_work_and_its_credential_stops_working() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    let token = std::fs::read_to_string(rig.dir.join("slots/builder/token")).unwrap();
    rig.agents.deactivate("builder").await.unwrap();

    assert!(!rig.agents.slot("builder").await.unwrap().active);
    assert!(rig.studio.identity.authenticate(&token).await.is_err());
    assert!(!rig.dir.join("slots/builder/token").exists());
    let request = agents::TurnRequest {
        slot: "builder".into(),
        kind: "task".into(),
        prompt: "hi".into(),
        ..Default::default()
    };
    assert_eq!(rig.agents.enqueue(request).await.unwrap_err().kind, ErrorKind::Conflict);
    assert_eq!(rig.agents.available(Some("shop"), None).await.unwrap(), None);
}

#[tokio::test]
async fn whichever_agent_is_available_is_one_with_nothing_in_hand_before_one_that_is_at_work() {
    let rig = Rig::new().await;
    rig.agent("first", "fake").await;
    rig.agent("second", "fake").await;
    let available = || async { rig.agents.available(Some("shop"), None).await.unwrap().unwrap().name };
    assert_eq!(available().await, "first", "both idle: the one that was there first");

    // The first is given something that takes a while.
    let work = |slot: &str, prompt: &str| agents::TurnRequest {
        slot: slot.into(),
        kind: "task".into(),
        prompt: prompt.into(),
        ..Default::default()
    };
    rig.agents.enqueue(work("first", "take your time [gate:slow]")).await.unwrap();
    rig.fake.saw(1).await;
    assert_eq!(available().await, "second", "what is given to a busy one waits behind what it has");
    // Both at work: neither is better, and it is the first again.
    rig.agents.enqueue(work("second", "and you [gate:slower]")).await.unwrap();
    rig.fake.saw(2).await;
    assert_eq!(available().await, "first");

    // The first has finished, and has nothing in hand.
    let mut follow = rig.from_now("agent.turn.finished");
    rig.fake.open("slow");
    crate::soon("the first to finish", follow.next()).await.unwrap();
    assert_eq!(available().await, "first");
    rig.fake.open("slower");
}

/// Whichever agent of a role has nothing in hand, by name.
async fn free(rig: &Rig, role: &str) -> Option<String> {
    rig.agents.available(Some("shop"), Some(role)).await.unwrap().map(|slot| slot.name)
}

#[tokio::test]
async fn work_given_to_a_role_goes_to_an_agent_of_that_role_and_to_no_other() {
    let rig = Rig::new().await;
    // Two that write and two that look, and one with no role at all. What
    // a role is for is nothing the factory knows: it is a word on a slot.
    rig.agent("spare", "fake").await;
    let of = |name: &str, role: &str| SlotSpec {
        name: name.into(),
        adapter: Some("fake".into()),
        project: Some("shop".into()),
        role: Some(role.into()),
        ..SlotSpec::default()
    };
    for (name, role) in [("worker-1", "worker"), ("reviewer-1", "reviewer"), ("reviewer-2", "reviewer")] {
        let slot = rig.agents.ensure(of(name, role), &ada()).await.unwrap();
        assert_eq!(slot.role, role);
    }
    assert_eq!(free(&rig, "reviewer").await.as_deref(), Some("reviewer-1"));
    assert_eq!(free(&rig, "worker").await.as_deref(), Some("worker-1"));
    assert_eq!(free(&rig, "architect").await, None, "a role nobody is in has nobody free");
    // With no role asked for, anyone will do, as before there were roles:
    // of those with nothing in hand, the one there first (by name, for
    // those that came in the same instant, as here).
    assert_eq!(rig.agents.available(Some("shop"), None).await.unwrap().unwrap().name, "reviewer-1");

    // The first reviewer has something in hand: the next piece of work for
    // the role goes to the other, and never to a worker, however idle.
    let work = agents::TurnRequest {
        slot: "reviewer-1".into(),
        kind: "task".into(),
        prompt: "look at this [gate:look]".into(),
        ..Default::default()
    };
    rig.agents.enqueue(work).await.unwrap();
    rig.fake.saw(1).await;
    assert_eq!(free(&rig, "reviewer").await.as_deref(), Some("reviewer-2"));
    rig.fake.open("look");

    // A role is kept when a slot is ensured again without a word of it,
    // changed when another is said, and taken away when nothing is.
    let again = SlotSpec { name: "reviewer-2".into(), ..SlotSpec::default() };
    assert_eq!(rig.agents.ensure(again, &ada()).await.unwrap().role, "reviewer");
    assert_eq!(rig.agents.ensure(of("reviewer-2", "worker"), &ada()).await.unwrap().role, "worker");
    assert_eq!(rig.agents.ensure(of("reviewer-2", ""), &ada()).await.unwrap().role, "");
    let wrong = rig.agents.ensure(of("reviewer-2", "Code Reviewer"), &ada()).await.unwrap_err();
    assert!(wrong.message.contains("cannot be a role: a role is a word"), "{}", wrong.message);
}

/// Whichever of those who work is chosen in this order, and is not one of `apart` while there is another.
async fn chosen(rig: &Rig, order: Order, apart: &[String]) -> String {
    rig.agents.available_apart(Some("shop"), Some("worker"), apart, order).await.unwrap().unwrap().name
}

#[tokio::test]
async fn which_of_those_with_nothing_in_hand_it_is_is_as_whoever_gives_the_work_says() {
    let rig = Rig::new().await;
    // One of no role, which nothing here is ever given, and three of a
    // role, the last of them ranked above the others.
    rig.agent("spare", "fake").await;
    let of = |name: &str, rank: Option<i64>| SlotSpec {
        name: name.into(),
        adapter: Some("fake".into()),
        project: Some("shop".into()),
        role: Some("worker".into()),
        rank,
        ..SlotSpec::default()
    };
    for (name, rank) in [("worker-1", None), ("worker-2", None), ("worker-3", Some(5))] {
        assert_eq!(rig.agents.ensure(of(name, rank), &ada()).await.unwrap().rank, rank.unwrap_or(0));
    }
    // Nobody has been given anything. The first there, unless rank is asked for.
    assert_eq!(chosen(&rig, Order::First, &[]).await, "worker-1");
    assert_eq!(chosen(&rig, Order::Turn, &[]).await, "worker-1");
    assert_eq!(chosen(&rig, Order::Least, &[]).await, "worker-1");
    assert_eq!(chosen(&rig, Order::Rank, &[]).await, "worker-3");

    // The first does two things and then the third does one, each finished before the next.
    for (count, slot) in ["worker-1", "worker-1", "worker-3"].into_iter().enumerate() {
        let work = agents::TurnRequest {
            slot: slot.into(),
            kind: "task".into(),
            prompt: "do it".into(),
            ..Default::default()
        };
        let mut follow = rig.from_now("agent.turn.finished");
        rig.agents.enqueue(work).await.unwrap();
        rig.fake.saw(count + 1).await;
        crate::soon("the turn to finish", follow.next()).await.unwrap();
    }
    // All have nothing in hand again. In turn it is the one never given
    // anything, and so it is by who was given least.
    assert_eq!(chosen(&rig, Order::First, &[]).await, "worker-1");
    assert_eq!(chosen(&rig, Order::Turn, &[]).await, "worker-2");
    assert_eq!(chosen(&rig, Order::Least, &[]).await, "worker-2");
    assert_eq!(chosen(&rig, Order::Rank, &[]).await, "worker-3");
    // Of the other two, in turn it is the one given work longest ago, and
    // by least the one given less: which are not the same agent.
    let apart = ["worker-2".to_string()];
    assert_eq!(chosen(&rig, Order::Turn, &apart).await, "worker-1");
    assert_eq!(chosen(&rig, Order::Least, &apart).await, "worker-3");

    // One with something in hand is never before one with nothing, however ranked.
    let slow = agents::TurnRequest {
        slot: "worker-3".into(),
        kind: "task".into(),
        prompt: "take your time [gate:slow]".into(),
        ..Default::default()
    };
    rig.agents.enqueue(slow).await.unwrap();
    rig.fake.saw(4).await;
    assert_eq!(chosen(&rig, Order::Rank, &[]).await, "worker-1");
    rig.fake.open("slow");

    // A rank is kept when a slot is ensured again without a word of it.
    assert_eq!(rig.agents.ensure(of("worker-3", None), &ada()).await.unwrap().rank, 5);
    // And a word that names no way of choosing is said to be none, with those there are.
    let refused = Order::named("random").unwrap_err().message;
    assert!(
        refused.contains("`random` is not a way to choose between agents") && refused.contains("`turn`"),
        "{refused}"
    );
    assert_eq!((Order::named("").unwrap(), Order::named("least").unwrap()), (Order::First, Order::Least));
}
