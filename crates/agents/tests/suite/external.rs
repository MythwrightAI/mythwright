//! An agent this factory does not start (`agents::external`): its slot,
//! the invitation it joins by, what its credential holds as it is used,
//! and every place that gives agents work leaving it out or refusing it
//! by name. Nothing is started for one at any point: the suite's fake
//! records what it was asked to run, and here that is only ever what
//! the agent beside it was asked.

use std::time::Duration;

use agents::{EXTERNAL, Invite, Order, SlotSpec, TurnRequest};
use conductor::{RunInfo, RunStatus};
use kernel::principal::AGENT_SCOPES;
use kernel::{ErrorKind, Principal, PrincipalKind};
use serde_json::json;
use studio::CredentialKind;

use crate::{Rig, ada, own};

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

fn named(name: &str) -> Invite {
    Invite { name: name.into(), ..Invite::default() }
}

fn scopes(held: &[&str]) -> Vec<String> {
    held.iter().map(|scope| scope.to_string()).collect()
}

/// `walls`, invited by Ada for the project the rig's other agents are in.
async fn invited(rig: &Rig) -> String {
    // The project, its board and its channel, and an agent this factory starts.
    rig.agent("helper", "fake").await;
    let asked = Invite { project: Some("shop".into()), ..named("walls") };
    rig.agents.invite(asked, &ada()).await.unwrap().invitation
}

/// The same, having joined: its token.
async fn joined(rig: &Rig) -> String {
    let invitation = invited(rig).await;
    rig.agents.join(&invitation).await.unwrap().token
}

fn step<'a>(run: &'a RunInfo, id: &str) -> &'a conductor::StepInfo {
    run.steps.iter().find(|step| step.step == id).unwrap()
}

/// The sentence a run of one step failed with.
fn failure(run: &RunInfo) -> String {
    assert_eq!(run.status, RunStatus::Failed, "{:?}", run.outputs);
    step(run, "it").error.clone().unwrap()
}

/// What says why nothing is given to it, in every such refusal.
const WHY: &str = "`walls` is an external agent: this factory starts no program for it and gives it no work";

#[tokio::test]
async fn an_external_agent_is_a_slot_with_a_credential_and_nothing_this_factory_starts() {
    let rig = Rig::new().await;
    let invitation = invited(&rig).await;
    let slot = rig.agents.slot("walls").await.unwrap();
    assert!(slot.external());
    assert_eq!((slot.adapter.as_str(), slot.project.as_deref(), slot.active), (EXTERNAL, Some("shop"), true));
    assert_eq!(slot.scopes, agents::external::usual(), "told nothing, it holds what an agent usually does");
    // And its own inbox beside that: nothing is started to tell it what names it.
    let more: Vec<&String> =
        slot.scopes.iter().filter(|scope| !AGENT_SCOPES.contains(&scope.as_str())).collect();
    assert_eq!(more, ["inbox"]);
    assert!(!slot.wall && slot.role.is_empty() && slot.signins.is_empty() && slot.secrets.is_empty());
    // It is listed with the others, and stands as invited and not joined.
    let listed: Vec<String> = rig.agents.slots().await.unwrap().into_iter().map(|slot| slot.name).collect();
    assert_eq!(listed, ["helper", "walls"]);
    let standing = rig.agents.standing(&slot).await.unwrap();
    assert!(!standing.joined && standing.invited_until.is_some());
    // It has no credential until it hands the invitation over.
    assert!(rig.studio.identity.holder("agent:walls").await.unwrap().is_none());
    let joined = rig.agents.join(&invitation).await.unwrap();
    assert_eq!((joined.agent.as_str(), joined.project.as_deref()), ("walls", Some("shop")));
    assert_eq!(joined.scopes, slot.scopes);
    // The token is an agent's, of that slot, holding what the slot holds.
    let who = rig.studio.identity.authenticate(&joined.token).await.unwrap();
    assert_eq!((who.kind, who.actor().as_str()), (PrincipalKind::Agent, "agent:walls"));
    assert_eq!(who.scopes, slot.scopes);
    assert_eq!(who.key, None);
    let standing = rig.agents.standing(&slot).await.unwrap();
    assert!(standing.joined && standing.invited_until.is_none());
    // Each is said: made, invited, joined, the last by the agent itself.
    let made = rig.events("agent.slot.created").await;
    let made = made.iter().find(|event| event.data["slot"] == "walls").unwrap();
    assert_eq!(
        (made.data["adapter"].as_str(), made.source.actor.as_deref()),
        (Some(EXTERNAL), Some("user:ada"))
    );
    let invited = rig.events("agent.slot.invited").await.remove(0);
    assert_eq!(
        (invited.data["slot"].as_str(), invited.data["project"].as_str()),
        (Some("walls"), Some("shop"))
    );
    assert!(!invited.data.to_string().contains(invitation.trim_start_matches("enrol_")));
    let said = rig.events("agent.slot.joined").await.remove(0);
    assert_eq!(
        (said.data["slot"].as_str(), said.source.actor.as_deref()),
        (Some("walls"), Some("agent:walls"))
    );
    assert_eq!(said.data["project"], "shop");

    // The factory keeps nothing of it on the disk: no token where a
    // started agent's is, and no folder of its own. The agent beside it
    // has both.
    assert!(rig.dir.join("slots/helper/token").exists());
    assert!(!rig.dir.join("slots/walls").exists());
    // And nothing was started: no turn ever reached what runs agents.
    assert!(rig.fake.seen.lock().unwrap().is_empty());
    assert!(rig.events("agent.turn.*").await.is_empty());
    // What the factory keeps of the invitation and the token is a hash of each.
    let kept = rig.journal.kv_list(studio::enrolled::INVITATIONS).await.unwrap();
    assert!(kept.is_empty(), "an invitation that was used is gone: {kept:?}");
}

#[tokio::test]
async fn nothing_gives_an_external_agent_work_and_each_way_says_why_by_name() {
    let rig = Rig::new().await;
    joined(&rig).await;
    // A turn is queued in one place, which refuses it.
    let asked = TurnRequest {
        slot: "walls".into(),
        kind: "task".into(),
        thread: "lane:shop-1".into(),
        prompt: "do the card".into(),
        ..Default::default()
    };
    let refused = rig.agents.enqueue(asked).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(refused.message.starts_with(WHY), "{}", refused.message);
    let give = format!("give it a card (`{} board give <card> walls`)", brand::CLI);
    assert!(refused.message.contains(&give), "{}", refused.message);

    // Whoever is free is never it: of the project, of no project, in any order.
    for order in [Order::First, Order::Turn, Order::Least, Order::Rank] {
        let could = rig.agents.could_take(Some("shop"), None, &[], order).await.unwrap();
        let names: Vec<&str> = could.iter().map(|one| one.slot.name.as_str()).collect();
        assert_eq!(names, ["helper"]);
    }
    assert_eq!(rig.agents.available(Some("shop"), None).await.unwrap().unwrap().name, "helper");
    rig.agents.deactivate("helper").await.unwrap();
    assert!(rig.agents.available(Some("shop"), None).await.unwrap().is_none());
    assert!(rig.agents.available(None, None).await.unwrap().is_none());
    assert!(rig.agents.available_apart(Some("shop"), None, &[], Order::Least).await.unwrap().is_none());

    // A step that names it fails saying why, by its name or as a record
    // names an agent; one that says there need be nobody is told there
    // is nobody, and who was named.
    let task = |with: &str| format!("blueprint: asks\nsteps:\n  it: {{ do: agent.task, with: {with} }}\n");
    for with in ["{ slot: walls, prompt: Do it }", "{ given: 'agent:walls', prompt: Do it }"] {
        let said = failure(&rig.run(&task(with), json!({})).await);
        assert!(said.starts_with(WHY), "{said}");
    }
    let run = rig
        .run(&task("{ given: 'agent:walls', role: worker, prompt: Do it, if_available: true }"), json!({}))
        .await;
    assert_eq!(run.status, RunStatus::Ok);
    let gave = step(&run, "it").output.clone();
    assert_eq!((gave["available"].as_bool(), gave["named"].as_str()), (Some(false), Some("walls")));
    let consult = "blueprint: consults\nsteps:\n  it: { do: agent.consult, with: { slot: walls, prompt: Which?, options: [a, b] } }\n";
    assert!(failure(&rig.run(consult, json!({})).await).starts_with(WHY));

    // A blueprint brings up no agent of its name: it is never a fleet's.
    let ensure =
        "blueprint: brings\nsteps:\n  it: { do: agent.ensure, with: { name: walls, role: worker } }\n";
    let said = failure(&rig.run(ensure, json!({})).await);
    assert!(said.starts_with("`walls` is an external agent, which this factory does not start: nothing that runs here brings it up or counts it"), "{said}");
    assert!(rig.agents.slot("walls").await.unwrap().role.is_empty());

    // There is nothing of it to try.
    let refused = rig.agents.try_program_in("walls", None, &ada()).await.unwrap_err();
    assert!(refused.message.contains("there is nothing to try"), "{}", refused.message);

    // Nothing was started for any of it, and no turn was ever written.
    assert!(rig.fake.seen.lock().unwrap().is_empty());
    assert!(rig.agents.turns("walls", 10).await.unwrap().is_empty());
    assert!(!rig.dir.join("slots/walls").exists());
}

#[tokio::test]
async fn a_message_that_names_an_external_agent_starts_nothing_and_is_answered_by_nobody() {
    let rig = Rig::new().await;
    joined(&rig).await;
    // Alone in a channel with a person, it is not the channel's one
    // agent: what names nobody there is said to nobody.
    rig.studio.channels.create("yard", Some("shop"), "", &ada()).await.unwrap();
    rig.agents.bring_in("yard", "walls", &ada()).await.unwrap();
    let yard = rig.studio.channels.get(&own(), "yard").await.unwrap();
    assert!(yard.members.iter().any(|member| member.actor == "agent:walls"));
    rig.studio.channels.post("yard", &ada(), "is anybody there?", None).await.unwrap();
    rig.studio.channels.post("yard", &ada(), "@walls take the gate", None).await.unwrap();
    // And beside an agent this factory starts: named, it is still asked
    // nothing, and the other is asked what names it.
    rig.agents.bring_in("shop", "walls", &ada()).await.unwrap();
    // What names it alone is not said to the other in its place.
    rig.say("@walls is the gate hung?").await;
    rig.say("@walls and @helper who has the till?").await;
    // Messages are heard in the order they were said: the last one's
    // turn has ended, so each before it was looked at.
    rig.turns_finished(1).await;
    assert_eq!(rig.fake.seen.lock().unwrap().len(), 1);
    assert_eq!(rig.fake.seen.lock().unwrap()[0].slot, "helper");
    assert!(
        rig.fake.prompts()[0].ends_with("@walls and @helper who has the till?"),
        "{:?}",
        rig.fake.prompts()
    );
    let queued = rig.events("agent.turn.queued").await;
    assert_eq!(queued.len(), 1, "one turn, the other agent's: {queued:?}");
    // The factory's bot says nothing of it: what was said is there for
    // it to read, as it is for anybody.
    let said = rig.studio.channels.messages(&own(), "yard", None, 10).await.unwrap();
    let said: Vec<&str> = said.iter().map(|message| message.body.as_str()).collect();
    assert_eq!(said, ["is anybody there?", "@walls take the gate"]);
    assert!(!rig.transcript().await.iter().any(|line| line.contains("was not asked")));

    // What named it waits in its box, in the channel it was said in or
    // not: one line a message, the latest first, each of the message
    // and from whoever said it. The agent this factory starts was given
    // its turn and has no line, nor has whoever said them.
    let inbox = &rig.studio.channels.inbox;
    let walls = rig.agents.slot("walls").await.unwrap().principal();
    let lines = inbox.list(&walls, false).await.unwrap();
    let (shown, unread) = rig.studio.channels.inbox_of(&walls, false).await.unwrap();
    let words: Vec<&str> =
        shown.iter().filter_map(|one| one.said.as_ref()).map(|said| said.body.as_str()).collect();
    assert_eq!(
        words,
        ["@walls and @helper who has the till?", "@walls is the gate hung?", "@walls take the gate"]
    );
    assert_eq!((unread, lines[2].place.as_str(), lines[2].from.as_str()), (3, yard.id.as_str(), "user:ada"));
    assert!(lines.iter().all(|line| line.kind == studio::inbox::Kind::Named && line.owner == "agent:walls"));
    let helper = rig.agents.slot("helper").await.unwrap().principal();
    let helper = Principal { scopes: scopes(&["inbox", "channels"]), ..helper };
    assert!(inbox.list(&helper, true).await.unwrap().is_empty());
    assert!(inbox.list(&Principal { scopes: scopes(&["*"]), ..ada() }, true).await.unwrap().is_empty());
    // Heard again, as after a stop, a message tells it nothing twice.
    let again = studio::inbox::Tell::named("agent:walls", &lines[0].about, &lines[0].place, "user:ada");
    assert_eq!(inbox.tell(again).await.unwrap(), None);
    // It says what names itself to nobody, and what it may not read
    // names it to no purpose: holding no channels, it is told nothing.
    rig.studio.channels.post("yard", &walls, "@walls is here", None).await.unwrap();
    rig.agents.give("walls", scopes(&["inbox", "board"]), &ada()).await.unwrap();
    rig.studio.channels.post("yard", &ada(), "@walls and now?", None).await.unwrap();
    rig.say("@helper and the till?").await;
    rig.turns_finished(2).await;
    assert_eq!(inbox.list(&walls, false).await.unwrap().len(), 3);

    // Who is there to be told of counts it as nobody a word could be
    // for: the other is the channel's one agent, to whom what names
    // nobody is said.
    let worker = SlotSpec { name: "helper".into(), role: Some("worker".into()), ..SlotSpec::default() };
    rig.agents.ensure(worker, &ada()).await.unwrap();
    let present = "blueprint: present\nsteps:\n  it: { do: agent.present, with: { channel: shop, here: [worker], counted: [worker] } }\n";
    let gave = step(&rig.run(present, json!({})).await, "it").output.clone();
    assert_eq!(
        (gave["first"].as_str(), gave["alone"].as_bool(), gave["count"].as_i64()),
        (Some("helper"), Some(true), Some(1))
    );
}

#[tokio::test]
async fn an_invitation_lets_its_agent_in_once_and_for_a_while_and_every_refusal_reads_the_same() {
    let rig = Rig::new().await;
    let first = invited(&rig).await;
    assert!(first.starts_with("enrol_") && first.len() > 50, "{first}");
    let refusal = |invitation: String| {
        let agents = rig.agents.clone();
        async move {
            let refused = agents.join(&invitation).await.unwrap_err();
            (refused.kind, refused.message)
        }
    };
    // One that is no invitation at all.
    let unknown = refusal("enrol_nothing-this-factory-made".into()).await;
    assert_eq!(unknown.0, ErrorKind::Denied);
    assert!(unknown.1.starts_with("that invitation lets no agent in"), "{}", unknown.1);
    assert_eq!(refusal("no invitation".into()).await, unknown);
    // A person's invitation to register is none.
    assert_eq!(refusal(format!("inv_{}", first.trim_start_matches("enrol_"))).await, unknown);
    // Used, it is used up.
    let token = rig.agents.join(&first).await.unwrap().token;
    assert_eq!(refusal(first.clone()).await, unknown);
    assert!(rig.studio.identity.authenticate(&token).await.is_ok());

    // One made again works for as long as was said, and no longer: read
    // from the clock as it is handed over.
    let again = Invite { lasts: Some(Duration::from_secs(2 * 3600)), ..named("walls") };
    let second = rig.agents.invite(again, &ada()).await.unwrap();
    assert_eq!(second.expires - rig.journal.now_ms(), 2 * 3600 * 1000);
    rig.clock.advance(Duration::from_secs(2 * 3600));
    assert_eq!(refusal(second.invitation.clone()).await, unknown);
    assert!(rig.agents.standing(&second.agent).await.unwrap().invited_until.is_none());
    // The credential it had goes on working: an invitation nobody used changed nothing.
    assert!(rig.studio.identity.authenticate(&token).await.is_ok());

    // Unsaid, it works for a day; and never for more than fourteen.
    let usual = rig.agents.invite(named("walls"), &ada()).await.unwrap();
    assert_eq!(usual.expires - rig.journal.now_ms(), 24 * 3600 * 1000);
    let long = Invite { lasts: Some(Duration::from_secs(15 * 24 * 3600)), ..named("walls") };
    let refused = rig.agents.invite(long, &ada()).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Invalid);
    assert!(refused.message.contains("no more than fourteen days"), "{}", refused.message);
    let none = Invite { lasts: Some(Duration::ZERO), ..named("walls") };
    assert_eq!(rig.agents.invite(none, &ada()).await.unwrap_err().kind, ErrorKind::Invalid);
    // The one before a refusal still works, and one made after it takes its place.
    let last = rig.agents.invite(named("walls"), &ada()).await.unwrap();
    assert_eq!(refusal(usual.invitation).await, unknown, "an earlier invitation is withdrawn by the next");
    // Used, it replaces the credential the agent had: a lost token is replaced so.
    let replaced = rig.agents.join(&last.invitation).await.unwrap().token;
    assert_ne!(replaced, token);
    assert!(rig.studio.identity.authenticate(&token).await.is_err());
    assert_eq!(rig.studio.identity.authenticate(&replaced).await.unwrap().actor(), "agent:walls");
    // Handed over twice at once, one invitation lets in once.
    let both = rig.agents.invite(named("walls"), &ada()).await.unwrap().invitation;
    let (one, other) = tokio::join!(rig.agents.join(&both), rig.agents.join(&both));
    assert_eq!(usize::from(one.is_ok()) + usize::from(other.is_ok()), 1);
}

#[tokio::test]
async fn whoever_invites_may_add_agents_and_gives_nothing_they_do_not_hold() {
    let rig = Rig::new().await;
    rig.agent("helper", "fake").await;
    // Somebody who may not add agents.
    let reads = person("Rita", &["agents:read", "board", "projects"]);
    let refused = rig.agents.invite(named("walls"), &reads).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied);
    assert!(rig.agents.slot("walls").await.is_err());
    // Somebody who may, and holds less than an agent usually does: what
    // is unsaid is refused by the first thing they lack, and what they
    // do hold is theirs to give.
    let narrow = person("Nell", &["agents", "board:read"]);
    let refused = rig.agents.invite(named("walls"), &narrow).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied);
    assert_eq!(refused.message, "Nell cannot give an agent `projects`, not holding it");
    let asked = Invite { scopes: Some(scopes(&["board:write"])), ..named("walls") };
    assert_eq!(
        rig.agents.invite(asked, &narrow).await.unwrap_err().message,
        "Nell cannot give an agent `board:write`, not holding it"
    );
    assert!(rig.agents.slot("walls").await.is_err(), "nothing was made by an invitation that was refused");
    assert!(rig.journal.kv_list(studio::enrolled::INVITATIONS).await.unwrap().is_empty());
    let asked = Invite { scopes: Some(scopes(&["board:read"])), ..named("walls") };
    let invited = rig.agents.invite(asked, &narrow).await.unwrap();
    assert_eq!(invited.agent.scopes, ["board:read"]);
    assert_eq!(rig.agents.join(&invited.invitation).await.unwrap().scopes, ["board:read"]);
    // Nor does anybody change what it holds to more than they hold.
    let refused = rig.agents.give("walls", scopes(&["board"]), &narrow).await.unwrap_err();
    assert_eq!(refused.message, "Nell cannot give an agent `board`, not holding it");

    // A project that is not there, a name that is none, the name the
    // factory speaks under, and the name of an agent this factory starts.
    let nowhere = Invite { project: Some("nowhere".into()), ..named("gate") };
    assert_eq!(rig.agents.invite(nowhere, &ada()).await.unwrap_err().kind, ErrorKind::NotFound);
    assert_eq!(rig.agents.invite(named("!!"), &ada()).await.unwrap_err().kind, ErrorKind::Invalid);
    let speaker = rig.studio.channels.speaker.name();
    let refused = rig.agents.invite(named(&speaker), &ada()).await.unwrap_err();
    assert!(refused.message.contains("that is what this factory itself speaks as"), "{}", refused.message);
    let refused = rig.agents.invite(named("helper"), &ada()).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(
        refused.message.starts_with("`helper` is an agent this factory starts (on `fake`)"),
        "{}",
        refused.message
    );
    assert_eq!(rig.agents.slot("helper").await.unwrap().adapter, "fake");
    for name in ["gate", "helper"] {
        assert!(rig.studio.identity.invited_until(CredentialKind::Agent, name).await.unwrap().is_none());
    }
}

#[tokio::test]
async fn what_an_external_agent_holds_is_read_as_its_credential_is_used() {
    let rig = Rig::new().await;
    let token = joined(&rig).await;
    let holds = |token: String| {
        let identity = rig.studio.identity.clone();
        async move { identity.authenticate(&token).await.unwrap().scopes }
    };
    assert_eq!(holds(token.clone()).await, agents::external::usual());
    // Changed as any agent's is, the same token holds the new list from
    // its next use: it is kept where this factory cannot write another.
    rig.agents.give("walls", scopes(&["board:read", "projects:read"]), &ada()).await.unwrap();
    assert_eq!(holds(token.clone()).await, ["board:read", "projects:read"]);
    assert_eq!(rig.agents.slot("walls").await.unwrap().scopes, ["board:read", "projects:read"]);
    // And so does who an event's actor is now, for what asks what they hold.
    let holder = rig.studio.identity.holder("agent:walls").await.unwrap().unwrap();
    assert_eq!(holder.scopes, ["board:read", "projects:read"]);
    // Said again by the way an agent is added, the same.
    let more =
        SlotSpec { name: "walls".into(), scopes: Some(scopes(&["board", "records"])), ..SlotSpec::default() };
    let slot = rig.agents.ensure(more, &ada()).await.unwrap();
    assert!(slot.external());
    assert_eq!(holds(token.clone()).await, ["board", "records"]);
    // An invitation made again with another list changes it too, before it is used.
    let asked = Invite { scopes: Some(scopes(&["search"])), ..named("walls") };
    rig.agents.invite(asked, &ada()).await.unwrap();
    assert_eq!(holds(token.clone()).await, ["search"]);
    // The agent beside it is changed as it always was: its token is replaced.
    let before = std::fs::read_to_string(rig.dir.join("slots/helper/token")).unwrap();
    rig.agents.give("helper", scopes(&["board:read"]), &ada()).await.unwrap();
    assert_ne!(std::fs::read_to_string(rig.dir.join("slots/helper/token")).unwrap(), before);
    assert!(rig.studio.identity.authenticate(&before).await.is_err());
    assert!(!rig.dir.join("slots/walls").exists());
}

#[tokio::test]
async fn nothing_of_a_program_is_said_of_an_external_agent_and_it_is_one_or_the_other_for_good() {
    let rig = Rig::new().await;
    joined(&rig).await;
    let of = |spec: SlotSpec| {
        let agents = rig.agents.clone();
        async move { agents.ensure(SlotSpec { name: "walls".into(), ..spec }, &ada()).await.unwrap_err().message }
    };
    let cases: Vec<(&str, SlotSpec)> = vec![
        ("adapter", SlotSpec { adapter: Some("fake".into()), ..SlotSpec::default() }),
        ("model", SlotSpec { model: Some("big".into()), ..SlotSpec::default() }),
        ("instructions", SlotSpec { instructions: Some("Be brief.".into()), ..SlotSpec::default() }),
        ("permission_mode", SlotSpec { permission_mode: Some("full".into()), ..SlotSpec::default() }),
        ("signins", SlotSpec { signins: Some(vec!["key".into()]), ..SlotSpec::default() }),
        ("role", SlotSpec { role: Some("worker".into()), ..SlotSpec::default() }),
        ("rank", SlotSpec { rank: Some(3), ..SlotSpec::default() }),
        ("wall", SlotSpec { wall: Some(true), ..SlotSpec::default() }),
        ("reads", SlotSpec { reads: Some(vec!["~/notes".into()]), ..SlotSpec::default() }),
        ("changes", SlotSpec { changes: Some(vec!["~/notes".into()]), ..SlotSpec::default() }),
        ("git", SlotSpec { git: Some("write".into()), ..SlotSpec::default() }),
        ("secrets", SlotSpec { secrets: Some(vec!["KEY=bot".into()]), ..SlotSpec::default() }),
        ("outlives", SlotSpec { outlives: Some(true), ..SlotSpec::default() }),
        ("web", SlotSpec { web: Some(true), ..SlotSpec::default() }),
    ];
    let before = rig.agents.slot("walls").await.unwrap();
    for (what, spec) in cases {
        let said = of(spec).await;
        let wanted = format!(
            "`walls` is an external agent: this factory starts no program for it, so `{what}` says nothing of it"
        );
        assert!(said.starts_with(&wanted), "{what}: {said}");
    }
    assert_eq!(rig.agents.slot("walls").await.unwrap(), before, "a refusal changed nothing");
    // Said as nothing, or as what it has, each is nothing said.
    let plain = SlotSpec {
        name: "walls".into(),
        adapter: Some(EXTERNAL.into()),
        role: Some(String::new()),
        signins: Some(Vec::new()),
        wall: Some(false),
        project: Some("shop".into()),
        ..SlotSpec::default()
    };
    assert_eq!(rig.agents.ensure(plain, &ada()).await.unwrap().scopes, before.scopes);

    // An external agent is invited, and not added by the way a started one is.
    let added = SlotSpec { name: "gate".into(), adapter: Some(EXTERNAL.into()), ..SlotSpec::default() };
    let refused = rig.agents.ensure(added, &ada()).await.unwrap_err();
    let invite = format!("an external agent is invited, not added: `{} agent invite gate`", brand::CLI);
    assert!(refused.message.starts_with(&invite), "{}", refused.message);
    assert!(rig.agents.slot("gate").await.is_err());
    // One this factory starts does not become external, nor the other way.
    let outside = SlotSpec { name: "helper".into(), adapter: Some(EXTERNAL.into()), ..SlotSpec::default() };
    let refused = rig.agents.ensure(outside, &ada()).await.unwrap_err();
    assert!(
        refused.message.starts_with("`helper` is an agent this factory starts (on `fake`)"),
        "{}",
        refused.message
    );
    // And nothing runs agents under that word: no package, no program set up here.
    assert!(!rig.agents.adapters().contains(&EXTERNAL.to_string()));
    rig.agents
        .add_adapter(EXTERNAL, std::sync::Arc::new(agents::adapter::Command { argv: vec!["true".into()] }));
    assert!(!rig.agents.adapters().contains(&EXTERNAL.to_string()));
    assert!(!rig.agents.providers().iter().any(|provider| provider.name == EXTERNAL));
}

#[tokio::test]
async fn stopping_an_external_agent_or_taking_it_away_withdraws_its_credential_and_its_invitation() {
    let rig = Rig::new().await;
    let token = joined(&rig).await;
    let waiting = rig.agents.invite(named("walls"), &ada()).await.unwrap().invitation;
    // Stopped: the token is nobody's, and the invitation that waited lets nobody in.
    rig.agents.deactivate("walls").await.unwrap();
    assert!(rig.studio.identity.authenticate(&token).await.is_err());
    assert!(rig.agents.join(&waiting).await.is_err());
    let slot = rig.agents.slot("walls").await.unwrap();
    assert!(!slot.active);
    assert_eq!(rig.agents.standing(&slot).await.unwrap(), agents::Standing::default());
    // Said again by the way an agent is added it is in, and has no
    // credential until it is invited again.
    rig.agents.ensure(SlotSpec { name: "walls".into(), ..SlotSpec::default() }, &ada()).await.unwrap();
    let slot = rig.agents.slot("walls").await.unwrap();
    assert!(slot.active && !rig.agents.standing(&slot).await.unwrap().joined);
    assert!(rig.studio.identity.authenticate(&token).await.is_err());
    // Told only what it holds, one that is stopped stays stopped.
    rig.agents.deactivate("walls").await.unwrap();
    rig.agents.give("walls", scopes(&["board"]), &ada()).await.unwrap();
    assert!(!rig.agents.slot("walls").await.unwrap().active);
    // Invited again it is in again, and joins with a new credential.
    let again = rig.agents.invite(named("walls"), &ada()).await.unwrap();
    assert!(again.agent.active);
    let second = rig.agents.join(&again.invitation).await.unwrap();
    assert_eq!(second.scopes, ["board"]);
    assert_ne!(second.token, token);
    // One stopped while its invitation waits is not let in by it.
    let waiting = rig.agents.invite(named("walls"), &ada()).await.unwrap().invitation;
    assert!(rig.agents.leave("walls").await.unwrap(), "with nothing in hand, ever, it is stopped at once");
    assert!(rig.agents.join(&waiting).await.is_err());
    assert!(rig.studio.identity.authenticate(&second.token).await.is_err());

    // Taken away: a card given to it stays given to that name, its slot
    // goes, and an invitation that waited for it lets nobody in.
    let again = rig.agents.invite(named("walls"), &ada()).await.unwrap();
    let third = rig.agents.join(&again.invitation).await.unwrap().token;
    let waiting = rig.agents.invite(named("walls"), &ada()).await.unwrap().invitation;
    let card = rig.studio.board.add("shop", "Hang the gate", "", None, Vec::new(), &ada()).await.unwrap();
    let given = studio::board::Changes { assignee: Some(Some("agent:walls".into())), ..Default::default() };
    rig.studio.board.update(&card.id, given, &ada()).await.unwrap();
    // What was addressed to it meanwhile is in its box, which goes with it: an
    // agent that is later called the same finds nothing there.
    let walls =
        Principal { scopes: scopes(&["inbox"]), ..rig.agents.slot("walls").await.unwrap().principal() };
    let line = studio::inbox::Tell {
        kind: studio::inbox::Kind::Told,
        says: Some("the gate is hung".into()),
        ..studio::inbox::Tell::named("agent:walls", "gate", "ch_1", "user:ada")
    };
    rig.studio.channels.inbox.tell(line).await.unwrap().unwrap();
    assert_eq!(rig.studio.channels.inbox.list(&walls, true).await.unwrap().len(), 1);
    let removed = rig.agents.remove("walls", &ada(), &rig.engine, false).await.unwrap();
    assert!(removed.removed);
    assert!(rig.studio.channels.inbox.list(&walls, true).await.unwrap().is_empty());
    assert_eq!((removed.turns, removed.cards.as_slice()), (0, std::slice::from_ref(&card.id)));
    assert!(rig.agents.slot("walls").await.is_err());
    assert!(rig.studio.identity.authenticate(&third).await.is_err());
    assert!(rig.agents.join(&waiting).await.is_err());
    assert!(rig.journal.kv_list(studio::enrolled::INVITATIONS).await.unwrap().is_empty());
    // An agent itself never takes one away, an external one's own credential included.
    let again = rig.agents.invite(named("walls"), &ada()).await.unwrap();
    rig.agents.join(&again.invitation).await.unwrap();
    let itself =
        Principal { scopes: vec!["*".into()], ..rig.agents.slot("walls").await.unwrap().principal() };
    assert_eq!(
        rig.agents.remove("walls", &itself, &rig.engine, false).await.unwrap_err().kind,
        ErrorKind::Denied
    );
}

#[tokio::test]
async fn an_external_agent_gives_its_own_credential_up_and_nobody_else_does_that_for_it() {
    let rig = Rig::new().await;
    let token = joined(&rig).await;
    let walls = rig.studio.identity.authenticate(&token).await.unwrap();
    // A person does not leave, nor an agent this factory starts.
    for who in [ada(), rig.agents.slot("helper").await.unwrap().principal()] {
        let refused = rig.agents.unjoin(&who).await.unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Invalid);
        assert!(refused.message.starts_with("only an external agent leaves"), "{}", refused.message);
    }
    assert!(rig.studio.identity.authenticate(&token).await.is_ok());
    rig.agents.unjoin(&walls).await.unwrap();
    assert!(rig.studio.identity.authenticate(&token).await.is_err());
    let said = rig.events("agent.slot.unjoined").await.remove(0);
    assert_eq!(
        (said.data["slot"].as_str(), said.source.actor.as_deref()),
        (Some("walls"), Some("agent:walls"))
    );
    // It is still there, taking no work as ever, and is let in again by an invitation.
    let slot = rig.agents.slot("walls").await.unwrap();
    assert!(slot.active && !rig.agents.standing(&slot).await.unwrap().joined);
    let again = rig.agents.invite(named("walls"), &ada()).await.unwrap();
    assert!(rig.agents.join(&again.invitation).await.is_ok());
}

#[tokio::test]
async fn an_external_agent_is_in_a_channel_kept_to_its_members_as_a_person_is_and_a_started_one_as_it_was() {
    let mut rig = Rig::holding(crate::kept::walls()).await;
    joined(&rig).await;
    let mira = crate::kept::person("mira");
    let kept = rig.studio.channels.create_kept("Pay review", None, "", &mira).await.unwrap();
    let other = rig.studio.channels.create_kept("Hiring", None, "", &mira).await.unwrap();
    let channels = rig.studio.channels.clone();
    let walls = rig.agents.slot("walls").await.unwrap().principal();
    let helper = rig.agents.slot("helper").await.unwrap().principal();

    // Brought in by either way an agent is, with no wall and no home
    // asked of it: nothing is started that either could hold.
    assert!(channels.may_read(&kept.id, &walls).await.is_err());
    rig.agents.bring_in(&kept.id, "walls", &mira).await.unwrap();
    let into = SlotSpec { name: "walls".into(), channel: Some(other.id.clone()), ..SlotSpec::default() };
    rig.agents.ensure(into, &mira).await.unwrap();
    assert_eq!(channels.kept_with("agent:walls").len(), 2);
    rig.agents.leave_channel(&other.id, "walls", &mira).await.unwrap();
    // Its own credential reads the one it is in, whenever it asks, and
    // writes there; the one it has left is one that is not there.
    assert!(channels.may_read(&kept.id, &walls).await.is_ok());
    channels.post(&kept.id, &walls, "the gate is hung", None).await.unwrap();
    assert!(channels.may_read(&other.id, &walls).await.is_err());
    // The agent beside it, which this factory starts, is in the same
    // channel and reads it only while a turn asked there is in hand:
    // with none, it is as closed out as it ever was.
    rig.agents.bring_in(&kept.id, "helper", &mira).await.unwrap();
    assert!(channels.may_read(&kept.id, &helper).await.is_err());
    // An open channel it is in is kept with it in it.
    rig.agents.bring_in("shop", "walls", &mira).await.unwrap();
    let shop = rig.agents.keep_channel("shop", &ada()).await.unwrap();
    assert!(shop.kept && channels.may_write(&shop.id, &walls).await.is_ok());
    // A daemon that comes back remembers which agents it does not start.
    rig.restart().await;
    assert!(channels.may_read(&kept.id, &walls).await.is_ok());
    assert!(channels.may_read(&kept.id, &helper).await.is_err());

    // Taken away, it is of nothing; and an agent this factory starts
    // that is later called the same is no heir to how it read.
    assert!(rig.agents.remove("walls", &ada(), &rig.engine, false).await.unwrap().removed);
    assert!(channels.may_read(&kept.id, &walls).await.is_err());
    let again = SlotSpec { name: "walls".into(), adapter: Some("fake".into()), ..SlotSpec::default() };
    rig.agents.ensure(again, &ada()).await.unwrap();
    rig.agents.bring_in(&kept.id, "walls", &mira).await.unwrap();
    let started = rig.agents.slot("walls").await.unwrap().principal();
    assert!(channels.may_read(&kept.id, &started).await.is_err());
}

#[tokio::test]
async fn a_conversation_is_written_with_its_agents_in_it_or_not_at_all() {
    let rig = Rig::holding(crate::kept::walls()).await;
    joined(&rig).await;
    let mira = crate::kept::person("mira");
    let channels = &rig.studio.channels;
    let nell = [studio::Member::new("user:nell", "Nell")];
    let to = |names: &[&str]| names.iter().map(|name| name.to_string()).collect::<Vec<_>>();

    // To an agent from outside and one this factory starts: both are in
    // it from the write that makes it. The one from outside has it in
    // its box; the other, the conversation's one started agent, is given
    // a turn by what was said, of that channel, and no line.
    let (conversation, first) = rig
        .agents
        .converse("The gate", "who hangs it?", &nell, &to(&["walls", "helper"]), &mira)
        .await
        .unwrap();
    assert!(conversation.kept && conversation.conversation);
    assert_eq!(channels.kept_with("agent:helper"), std::slice::from_ref(&conversation.id));
    let walls = rig.agents.slot("walls").await.unwrap().principal();
    let (lines, unread) = channels.inbox_of(&walls, false).await.unwrap();
    assert_eq!((lines.len(), unread), (1, 1));
    assert_eq!(lines[0].line.said_by.as_deref(), Some(first.id.as_str()));
    rig.fake.saw(1).await;
    let asked = rig.agents.turns("helper", 5).await.unwrap().remove(0);
    assert_eq!(
        (asked.channel.as_deref(), asked.reply_to.as_deref()),
        (Some(&*conversation.id), Some(&*first.id))
    );
    assert!(rig.agents.turns("walls", 5).await.unwrap().is_empty());
    let helper = rig.agents.slot("helper").await.unwrap().principal();
    assert!(
        channels
            .inbox
            .list(&Principal { scopes: scopes(&["inbox"]), ..helper }, true)
            .await
            .unwrap()
            .is_empty()
    );

    // An agent this factory starts is brought in by whoever may change
    // agents, as into any channel kept to its members; one from outside
    // is written to as a person is.
    let reads = Principal { scopes: scopes(&["channels", "inbox", "agents:read"]), ..mira.clone() };
    let refused = rig.agents.converse("", "quince", &[], &to(&["helper"]), &reads).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied);
    rig.agents.converse("", "quince", &[], &to(&["walls"]), &reads).await.unwrap();
    // One that is not there, and one that may be in no kept channel,
    // are refused in a sentence, and nothing is made.
    let made = channels.readable(&ada()).await.unwrap().len();
    let nobody = rig.agents.converse("", "quince", &nell, &to(&["nobody"]), &mira).await.unwrap_err();
    assert_eq!(nobody.kind, ErrorKind::NotFound);
    let unwalled = SlotSpec { name: "helper".into(), wall: Some(false), ..SlotSpec::default() };
    rig.agents.ensure(unwalled, &ada()).await.unwrap();
    let refused =
        rig.agents.converse("", "quince", &nell, &to(&["walls", "helper"]), &mira).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(
        refused.message.starts_with("`helper` works with no wall")
            && refused.message.contains(
                "an agent with no wall is in no channel kept to its members, which a conversation is"
            )
            && refused.message.contains("nothing was written"),
        "{}",
        refused.message
    );
    assert_eq!(channels.readable(&ada()).await.unwrap().len(), made);
}
