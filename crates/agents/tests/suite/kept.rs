//! An agent in a channel kept to its members (`agents::kept`): when its
//! key opens the channel, who is told of a turn asked there and who is
//! shown its words, and that an agent with no wall is in none.
//!
//! The agents here run on the suite's fake, which starts no program, so
//! the wall their rig holds is one nothing is started behind: an agent
//! of a factory that holds no walls is in no kept channel.

use std::path::PathBuf;

use agents::{SlotSpec, TurnRequest};
use conductor::{Apply, RunStatus};
use kernel::{Error, ErrorKind, Event, Principal, PrincipalKind};

use crate::{Rig, ada, own, soon};

/// A wall that nothing here is started behind.
pub(crate) fn walls() -> hosting::Walls {
    hosting::Walls::Held(PathBuf::from("a-wall-that-no-program-here-is-started-behind"))
}

/// Somebody signed in who holds what a person is given, and nothing
/// that is kept private: not what the factory's owner holds.
pub(crate) fn person(id: &str) -> Principal {
    Principal {
        kind: PrincipalKind::User,
        id: id.into(),
        name: id.into(),
        scopes: kernel::principal::MEMBER_SCOPES.iter().map(|scope| scope.to_string()).collect(),
        key: None,
    }
}

/// A rig with the agent `builder` in the open channel of its project,
/// and a channel kept to its members that Mira made and brought it into.
async fn kept() -> (Rig, studio::Channel) {
    let rig = Rig::holding(walls()).await;
    rig.agent("builder", "fake").await;
    let mira = person("mira");
    let kept = rig.studio.channels.create_kept("Pay review", None, "who is paid what", &mira).await.unwrap();
    let into = SlotSpec { name: "builder".into(), channel: Some(kept.id.clone()), ..SlotSpec::default() };
    rig.agents.ensure(into, &mira).await.unwrap();
    (rig, kept)
}

/// What a refusal says, with what was asked for taken out of it.
pub(crate) fn said(refused: &Error, asked: &str) -> (ErrorKind, String) {
    (refused.kind, refused.message.replace(asked, "<it>"))
}

/// Whether the agent's own key reads `channel` just now: and where it
/// does not, that it is answered as for a channel that is not there.
async fn opens(rig: &Rig, channel: &str) -> bool {
    let builder = rig.agents.slot("builder").await.unwrap().principal();
    let channels = &rig.studio.channels;
    match channels.may_read(channel, &builder).await {
        Ok(reader) => {
            assert!(channels.messages(&reader, channel, None, 10).await.is_ok());
            assert!(channels.readable(&builder).await.unwrap().iter().any(|one| one.id == channel));
            true
        }
        Err(refused) => {
            let nowhere = channels.may_read("ch_nonesuch", &builder).await.unwrap_err();
            assert_eq!(said(&refused, channel), said(&nowhere, "ch_nonesuch"));
            assert!(channels.readable(&builder).await.unwrap().iter().all(|one| one.id != channel));
            assert!(channels.may_write(channel, &builder).await.is_err());
            false
        }
    }
}

/// The turns the database keeps as in hand, each with the kept channel
/// it says it is of.
async fn in_hand(rig: &Rig) -> Vec<Option<String>> {
    let asked = |connection: &mut rusqlite::Connection| {
        let mut statement =
            connection.prepare("SELECT kept FROM turns WHERE status IN ('running', 'answered')")?;
        let rows = statement.query_map([], |row| row.get(0))?;
        rows.collect::<rusqlite::Result<Vec<Option<String>>>>()
    };
    rig.of_turns(asked).await.into_iter().flatten().collect()
}

/// Waits until `count` turns were ever put in a queue.
async fn queued(rig: &Rig, count: usize) {
    let mut follow = rig.journal.follow_every(journal::Places::default(), "agent.turn.queued");
    soon(&format!("{count} turns to be queued"), async {
        for _ in 0..count {
            follow.next().await.unwrap();
        }
    })
    .await;
}

#[tokio::test]
async fn an_agents_key_opens_a_kept_channel_only_while_a_turn_asked_there_is_in_hand() {
    let (mut rig, kept) = kept().await;
    let mira = person("mira");
    let channels = rig.studio.channels.clone();
    // With nothing in hand it is closed to it, in it or not.
    assert!(!opens(&rig, &kept.id).await);

    // Asked there, and at work on it: its key reads the channel.
    channels.post(&kept.id, &mira, "@builder count the quince [gate:first]", None).await.unwrap();
    rig.fake.saw(1).await;
    assert_eq!(in_hand(&rig).await, [Some(kept.id.clone())]);
    assert!(opens(&rig, &kept.id).await);
    // What it answers is said there, as itself; and with that it is closed again.
    rig.fake.open("first");
    rig.turns_finished(1).await;
    assert!(!opens(&rig, &kept.id).await);
    let there = channels.messages(&own(), &kept.id, None, 10).await.unwrap();
    assert_eq!(
        there.last().map(|last| (last.author.as_str(), last.body.as_str())),
        Some(("agent:builder", "done"))
    );

    // At work on something asked elsewhere, it is closed: an agent does
    // one thing at a time, so this is the only other turn it has in hand.
    rig.say("@builder and the open one [gate:second]").await;
    rig.fake.saw(2).await;
    assert_eq!(in_hand(&rig).await, [None]);
    assert!(!opens(&rig, &kept.id).await);
    // What waits behind that for the kept channel opens nothing either.
    channels.post(&kept.id, &mira, "@builder and the medlar [gate:third]", None).await.unwrap();
    queued(&rig, 3).await;
    assert!(!opens(&rig, &kept.id).await);
    rig.fake.open("second");
    rig.fake.saw(3).await;
    assert!(opens(&rig, &kept.id).await);

    // The daemon stops with that turn in hand. One that comes back
    // knows of it from the turn's row, before anything is begun again.
    rig.stop();
    rig.open_again().await;
    assert!(opens(&rig, &kept.id).await, "the turn its row says is in hand opens the channel");
    rig.stop();
    rig.start().await;
    rig.fake.saw(4).await;
    assert!(opens(&rig, &kept.id).await);
    rig.fake.open("third");
    rig.turns_finished(3).await;
    assert!(!opens(&rig, &kept.id).await, "and with that turn ended, nothing does");
    assert_eq!(in_hand(&rig).await, Vec::<Option<String>>::new());
    // And a daemon that comes back with nothing in hand opens nothing.
    rig.restart().await;
    assert!(!opens(&rig, &kept.id).await);
}

#[tokio::test]
async fn what_is_said_of_a_turn_asked_in_a_kept_channel_is_said_to_its_members_and_its_words_are_shown_to_nobody_else()
 {
    let (rig, kept) = kept().await;
    let (mira, nell) = (person("mira"), person("nell"));
    let channels = &rig.studio.channels;
    // One turn in the open, and one in the kept channel, which says
    // what it prints as it works.
    rig.say("@builder the open word").await;
    rig.turns_finished(1).await;
    let mut printed = rig.from_now("agent.output");
    rig.fake.will_say("the greengage figures");
    channels.post(&kept.id, &mira, "@builder count the quince", None).await.unwrap();
    rig.turns_finished(2).await;
    let output = soon("what it printed", printed.next()).await.unwrap();

    // Everything said of the kept one takes being a member; of the open
    // one, nothing, as it always did.
    let member = format!("member:{}", kept.id);
    let mut said = rig.events("agent.turn.*").await;
    said.push((*output).clone());
    let of_the_kept: Vec<&Event> = said.iter().filter(|event| event.source.takes.is_some()).collect();
    let topics: Vec<&str> = of_the_kept.iter().map(|event| event.topic.as_str()).collect();
    assert_eq!(topics, ["agent.turn.queued", "agent.turn.started", "agent.turn.finished", "agent.output"]);
    assert_eq!(said.len(), 7, "three of the open turn, which take nothing: {said:?}");
    for event in &of_the_kept {
        assert_eq!(event.source.takes.as_deref(), Some(member.as_str()), "{}", event.topic);
        assert!(rig.journal.may_tell(&mira, event), "{} is told to a member", event.topic);
        assert!(!rig.journal.may_tell(&nell, event), "{} is told to nobody else", event.topic);
        assert!(!rig.journal.may_tell(&person("nils"), event));
    }
    assert_eq!(output.data["text"], "the greengage figures");
    // And nothing it is measured with names the channel.
    let mut measured = String::new();
    for name in ["agent.turn", "agent.cost_usd", "agent.input_tokens", "agent.output_tokens"] {
        // Split by everything a turn is measured with, and by what
        // would name where it was asked if anything did.
        let by = ["slot", "adapter", "project", "kind", "decision", "channel", "thread"];
        let group_by = by.iter().map(|key| key.to_string()).collect();
        let query = journal::MetricQuery { name: name.into(), group_by, ..Default::default() };
        measured.push_str(&format!("{:?}", rig.journal.query_metrics(query).await.unwrap()));
    }
    assert!(measured.contains("builder"), "{measured}");
    assert!(!measured.contains(&kept.id) && !measured.contains("Pay review"), "{measured}");

    // Its turns, as each is shown them: a member the words, anybody
    // else when it was, how long it took and what it cost.
    let whole = rig.agents.turns_for("builder", 50, &mira).await.unwrap();
    assert_eq!(whole.len(), 2);
    assert!(whole[0].prompt.contains("count the quince") && whole[0].reply == "the greengage figures");
    assert_eq!((whole[0].channel.as_deref(), whole[0].kept_from), (Some(kept.id.as_str()), false));
    // With each, apart from all it was sent, what the message that asked for it said.
    let asked = |turns: &[agents::TurnInfo]| -> Vec<Option<String>> {
        turns.iter().map(|turn| turn.asked.clone()).collect()
    };
    assert_eq!(
        asked(&whole),
        [Some("@builder count the quince".to_string()), Some("@builder the open word".to_string())]
    );
    let bare = rig.agents.turns_for("builder", 50, &nell).await.unwrap();
    // Somebody who does not read the kept channel is handed that of the open turn and nothing of the kept one's.
    assert_eq!(asked(&bare), [None, Some("@builder the open word".to_string())]);
    assert!(serde_json::to_value(&bare[0]).unwrap().get("asked").is_none());
    let shown = serde_json::to_string(&bare[0]).unwrap();
    for word in ["quince", "greengage", kept.id.as_str(), "Pay review", "channel:", "msg_"] {
        assert!(
            !shown.contains(word),
            "`{word}` is shown to somebody who does not read the channel: {shown}"
        );
    }
    assert_eq!(
        (bare[0].id.as_str(), bare[0].status.as_str(), bare[0].kept_from),
        (whole[0].id.as_str(), "done", true)
    );
    assert_eq!(
        (bare[0].created, bare[0].started, bare[0].finished, bare[0].cost_usd, bare[0].output_tokens),
        (whole[0].created, whole[0].started, whole[0].finished, 0.01, 20)
    );
    // The open one is shown whole to both, and says nothing of being kept from anybody.
    assert_eq!(bare[1], whole[1]);
    assert!(bare[1].prompt.contains("the open word"));
    assert!(!serde_json::to_string(&bare[1]).unwrap().contains("kept_from"));

    // Turns that never began are said to have ended to the same people.
    channels.post(&kept.id, &mira, "@builder hold this [gate:held]", None).await.unwrap();
    rig.fake.saw(3).await;
    channels.post(&kept.id, &mira, "@builder and this behind it", None).await.unwrap();
    queued(&rig, 4).await;
    rig.agents.deactivate("builder").await.unwrap();
    rig.turns_finished(4).await;
    let finished = rig.events("agent.turn.finished").await;
    let called_off: Vec<&Event> = finished.iter().filter(|event| event.data["called_off"] == true).collect();
    assert_eq!(called_off.len(), 2, "{finished:?}");
    assert!(called_off.iter().all(|event| event.source.takes.as_deref() == Some(member.as_str())));
}

#[tokio::test]
async fn an_agent_with_no_wall_is_in_no_kept_channel() {
    // A factory that was started holding no walls: no agent of it is
    // brought into one.
    let open = Rig::new().await;
    open.agent("builder", "fake").await;
    let mira = person("mira");
    let made = open.studio.channels.create_kept("Pay review", None, "", &mira).await.unwrap();
    let into = SlotSpec { name: "builder".into(), channel: Some(made.id.clone()), ..SlotSpec::default() };
    let refused = open.agents.ensure(into, &mira).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(refused.message.contains("this factory was started holding no walls"), "{}", refused.message);
    assert!(refused.message.contains("an agent with no wall is in no channel kept to its members"));
    assert_eq!(open.studio.channels.kept_with("agent:builder"), Vec::<String>::new());

    let (rig, kept) = kept().await;
    let channels = &rig.studio.channels;
    // One told to work with no wall is not brought in, and is not made
    // by the asking: nothing was written.
    let bare = SlotSpec {
        name: "roamer".into(),
        adapter: Some("fake".into()),
        wall: Some(false),
        channel: Some(kept.id.clone()),
        ..SlotSpec::default()
    };
    let refused = rig.agents.ensure(bare, &ada()).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(refused.message.contains("`roamer` works with no wall"), "{}", refused.message);
    assert!(refused.message.contains(&format!("`{} agent add roamer --wall`", brand::CLI)));
    assert_eq!(rig.agents.slot("roamer").await.unwrap_err().kind, ErrorKind::NotFound);
    // Somebody who is not in the channel is told what they are told of
    // a channel that is not there, wall or none.
    let outsider = SlotSpec { name: "roamer".into(), wall: Some(false), ..SlotSpec::default() };
    let (here, nowhere) = (
        SlotSpec { channel: Some(kept.id.clone()), ..outsider.clone() },
        SlotSpec { channel: Some("ch_nonesuch".into()), ..outsider },
    );
    let nils =
        Principal { scopes: vec!["agents".into(), "channels".into(), "shell".into()], ..person("nils") };
    assert_eq!(
        said(&rig.agents.ensure(here, &nils).await.unwrap_err(), &kept.id),
        said(&rig.agents.ensure(nowhere, &nils).await.unwrap_err(), "ch_nonesuch")
    );

    // One that is in it and is then told to work with no wall leaves
    // it, which is said there by the factory; it stays where it was in
    // the open.
    assert_eq!(channels.kept_with("agent:builder"), std::slice::from_ref(&kept.id));
    let unwalled = SlotSpec { name: "builder".into(), wall: Some(false), ..SlotSpec::default() };
    rig.agents.ensure(unwalled, &ada()).await.unwrap();
    assert_eq!(channels.kept_with("agent:builder"), Vec::<String>::new());
    let there = channels.get(&own(), &kept.id).await.unwrap();
    assert!(there.members.iter().all(|member| member.actor != "agent:builder"), "{:?}", there.members);
    let said_there = channels.messages(&own(), &kept.id, None, 10).await.unwrap();
    let note = said_there.last().unwrap();
    assert_eq!(note.author, "bot:conductor");
    assert!(note.body.contains("`builder` works with no wall") && note.body.contains("it has left this one"));
    let left = rig.events("channel.left").await;
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].data["member"], "agent:builder");
    let shop = channels.get(&own(), "shop").await.unwrap();
    assert!(shop.members.iter().any(|member| member.actor == "agent:builder"));
    // And it is given no turn of that channel, by a message or by a step.
    let asked = TurnRequest {
        slot: "builder".into(),
        kind: "task".into(),
        thread: format!("channel:{}", kept.id),
        prompt: "read it out".into(),
        ..TurnRequest::default()
    };
    let refused = rig.agents.enqueue(asked.clone()).await.unwrap_err();
    assert!(refused.message.contains("an agent with no wall is in no channel kept to its members"));
    assert!(refused.message.contains("it is asked nothing in one"), "{}", refused.message);
    // Given its wall again it is still not in it, and is asked nothing there.
    let walled = SlotSpec { name: "builder".into(), wall: Some(true), ..SlotSpec::default() };
    rig.agents.ensure(walled, &ada()).await.unwrap();
    let refused = rig.agents.enqueue(asked).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(refused.message.contains("`builder` is not in that channel"), "{}", refused.message);
    assert!(rig.events("agent.turn.queued").await.is_empty());
}

#[tokio::test]
async fn an_agent_in_a_kept_channel_of_a_factory_that_is_then_started_holding_no_walls_leaves_it() {
    let (mut rig, kept) = kept().await;
    let channels = rig.studio.channels.clone();
    assert_eq!(channels.kept_with("agent:builder"), std::slice::from_ref(&kept.id));
    // The daemon is stopped, and started again holding no walls: how it
    // was started is what the agent's place in the channel rested on.
    rig.stop();
    rig.walls = hosting::Walls::Open;
    rig.start().await;
    soon("the agent to have left the kept channel", async {
        while !channels.kept_with("agent:builder").is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    let note = rig.event("channel.message", |event| event.data["author_kind"] == "bot").await;
    let said = note.data["body"].as_str().unwrap();
    assert!(said.contains("this factory was started holding no walls"), "{said}");
    assert!(
        said.contains("an agent with no wall is in no channel kept to its members: it has left this one")
    );
    assert_eq!(note.source.takes, Some(format!("member:{}", kept.id)), "said to the channel's members");
    // Started so again, nothing more is said: it is in none.
    rig.restart().await;
    rig.say("@builder are you there?").await;
    rig.turns_finished(1).await;
    assert_eq!(channels.messages(&own(), &kept.id, None, 50).await.unwrap().len(), 1);
}

/// A text that has an agent go on with a conversation.
fn goes_on(thread: &str) -> String {
    format!(
        "blueprint: reads-out\nsteps:\n  ask: {{ do: agent.task, with: {{ slot: builder, prompt: \"What was said \
         there?\", thread: \"{thread}\" }} }}\n"
    )
}

/// Applies `source` as `by`, and says how its one step came out: what
/// it gave, or why it failed.
async fn asks(rig: &Rig, source: &str, by: &Principal) -> Result<serde_json::Value, String> {
    let request = Apply {
        source: Some(source.to_string()),
        project: Some("shop".into()),
        ..Apply::new("reads-out".to_string(), by.clone())
    };
    let applied = rig.engine.apply(request).await.unwrap();
    let run = soon("the run to finish", rig.engine.wait_run(applied.run.as_ref().unwrap())).await.unwrap();
    let step = run.steps.iter().find(|step| step.step == "ask").unwrap();
    match run.status {
        RunStatus::Ok => Ok(step.output.clone()),
        _ => Err(step.error.clone().unwrap_or_default()),
    }
}

#[tokio::test]
async fn a_conversation_held_in_a_channel_is_gone_on_with_only_by_whoever_reads_that_channel() {
    let (rig, kept) = kept().await;
    let (mira, nell) = (person("mira"), person("nell"));
    let channels = &rig.studio.channels;
    // Said there with nobody named, it is said to the one agent there.
    let first = channels.post(&kept.id, &mira, "the medlar ledger", None).await.unwrap();
    rig.turns_finished(1).await;

    // Somebody who does not read the channel names its conversation, or
    // one under a message of it: refused as one that is not there is.
    for (named, nowhere, asked, nothing) in [
        (format!("channel:{}", kept.id), "channel:ch_nonesuch".to_string(), kept.id.clone(), "ch_nonesuch"),
        (format!("thread:{}", first.id), "thread:msg_nonesuch".to_string(), first.id.clone(), "msg_nonesuch"),
    ] {
        let refused = asks(&rig, &goes_on(&named), &nell).await.unwrap_err();
        let missing = asks(&rig, &goes_on(&nowhere), &nell).await.unwrap_err();
        assert_eq!(refused.replace(&asked, "<it>"), missing.replace(nothing, "<it>"));
        assert!(refused.contains("there is no"), "{refused}");
        for word in ["medlar", "Pay review"] {
            assert!(!refused.contains(word), "{refused}");
        }
        // And without what reading channels takes, in the words that is always refused in.
        let board = Principal { scopes: vec!["agents".into(), "blueprints".into()], ..person("mira") };
        let refused = asks(&rig, &goes_on(&named), &board).await.unwrap_err();
        assert!(refused.contains("channels:read"), "{refused}");
    }
    assert_eq!(rig.events("agent.turn.queued").await.len(), 1, "nobody was asked anything by any of that");
    // A conversation that is no channel's is anybody's to name, as it always was.
    rig.fake.will_say("nothing was");
    assert_eq!(asks(&rig, &goes_on("lane:shop-1"), &nell).await.unwrap()["reply"], "nothing was");

    // A member has the agent go on with it: a turn of that channel, in
    // hand as one asked there is, and said to its members.
    rig.fake.will_say("the medlar ledger was");
    let gave = asks(&rig, &goes_on(&format!("thread:{}", first.id)), &mira).await.unwrap();
    assert_eq!(gave["reply"], "the medlar ledger was");
    let queued = rig.events("agent.turn.queued").await;
    let takes: Vec<Option<&str>> = queued.iter().map(|event| event.source.takes.as_deref()).collect();
    let member = format!("member:{}", kept.id);
    assert_eq!(takes, [Some(member.as_str()), None, Some(member.as_str())]);
    let shown = rig.agents.turns_for("builder", 50, &nell).await.unwrap();
    assert!(shown[0].kept_from && shown[0].reply.is_empty() && !shown[1].kept_from && shown[2].kept_from);
}

#[tokio::test]
async fn an_agent_of_a_channel_whose_agents_are_asked_nowhere_else_is_given_no_work_anywhere_else() {
    let rig = Rig::holding(walls()).await;
    rig.agent("builder", "fake").await;
    rig.agent("helper", "fake").await;
    let mira = person("mira");
    let channels = &rig.studio.channels;
    let closest = channels.create_kept_alone("Board minutes", None, "", &mira).await.unwrap();
    let kept = channels.create_kept("Pay review", None, "", &mira).await.unwrap();
    let into = |name: &str, channel: &studio::Channel| SlotSpec {
        name: name.into(),
        channel: Some(channel.id.clone()),
        ..SlotSpec::default()
    };
    rig.agents.ensure(into("builder", &closest), &mira).await.unwrap();
    let names_none = |said: &str| {
        for word in [closest.id.as_str(), "Board minutes", kept.id.as_str()] {
            assert!(!said.contains(word), "`{word}` is said to somebody who may not read it: {said}");
        }
    };

    // Said to it by name in the open channel: it is not asked, and the
    // factory says so there, naming no channel.
    rig.say("@builder what is on the board?").await;
    let note = rig.event("channel.message", |event| event.data["author_kind"] == "bot").await;
    let said = note.data["body"].as_str().unwrap();
    assert!(said.starts_with("builder was not asked: "), "{said}");
    assert!(said.contains("whose agents are asked nowhere else") && said.contains("ask it there"), "{said}");
    names_none(said);
    // Nor by a step that names it.
    let asked = TurnRequest {
        slot: "builder".into(),
        kind: "task".into(),
        thread: "lane:shop-1".into(),
        prompt: "see to the card".into(),
        ..TurnRequest::default()
    };
    let refused = rig.agents.enqueue(asked).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    names_none(&refused.message);
    // Nor as whichever agent is free: the other one is, though this one
    // was brought up first and has nothing in hand.
    let free = rig.agents.available(Some("shop"), None).await.unwrap();
    assert_eq!(free.map(|slot| slot.name).as_deref(), Some("helper"));
    assert!(rig.events("agent.turn.queued").await.is_empty(), "nothing was asked of it");

    // Asked there, it answers there.
    rig.fake.will_say("the minutes");
    channels.post(&closest.id, &mira, "@builder the minutes, please", None).await.unwrap();
    rig.turns_finished(1).await;
    let there = channels.messages(&own(), &closest.id, None, 10).await.unwrap();
    assert_eq!(there.last().map(|last| last.body.as_str()), Some("the minutes"));

    // It is brought into no other kept channel, and is told why without
    // being told which holds it; and one that is in another is not
    // brought into this one.
    let refused = rig.agents.ensure(into("builder", &kept), &mira).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(refused.message.contains("is in a channel it is asked nowhere but in"), "{}", refused.message);
    names_none(&refused.message);
    rig.agents.ensure(into("helper", &kept), &mira).await.unwrap();
    let refused = rig.agents.ensure(into("helper", &closest), &mira).await.unwrap_err();
    assert!(refused.message.contains("would be asked nowhere but here"), "{}", refused.message);
    // The other, in a kept channel that holds nobody, is asked anywhere.
    rig.say("@helper and you?").await;
    rig.turns_finished(2).await;

    // Out of the channel that held it, it is asked anywhere again.
    channels.leave(&own(), &closest.id, "agent:builder").await.unwrap();
    rig.say("@builder now you").await;
    rig.turns_finished(3).await;
    assert_eq!(rig.agents.turns("builder", 10).await.unwrap().len(), 2);
}

#[tokio::test]
async fn what_an_agent_kept_of_a_kept_channel_goes_when_it_leaves_it_when_the_channel_is_taken_away_and_with_the_agent()
 {
    let (rig, first) = kept().await;
    let (mira, nell) = (person("mira"), person("nell"));
    let channels = &rig.studio.channels;
    let second = channels.create_kept("Hiring", None, "", &mira).await.unwrap();
    let third = channels.create_kept("Board minutes", None, "", &mira).await.unwrap();
    for channel in [&second, &third] {
        let into =
            SlotSpec { name: "builder".into(), channel: Some(channel.id.clone()), ..SlotSpec::default() };
        rig.agents.ensure(into, &mira).await.unwrap();
    }
    // Asked in each, and in the open: a folder of its own for each kept
    // channel, and a conversation remembered for each of the four.
    for (asked, channel) in [&first, &second, &third].into_iter().enumerate() {
        channels.post(&channel.id, &mira, "@builder note this", None).await.unwrap();
        rig.turns_finished(asked + 1).await;
    }
    rig.say("@builder and in the open").await;
    rig.turns_finished(4).await;
    let place = |channel: &studio::Channel| rig.dir.join("slots/builder/kept").join(&channel.id);
    let remembered = || async {
        let kept = rig.journal.kv_list("agent.session").await.unwrap();
        kept.into_iter().map(|(key, _)| key).collect::<Vec<String>>()
    };
    let remembers = |of: &[String], channel: &studio::Channel| of.iter().any(|key| key.contains(&channel.id));
    assert!([&first, &second, &third].iter().all(|channel| place(channel).join("home").is_dir()));
    let all = remembered().await;
    assert_eq!(all.len(), 4, "{all:?}");

    // It is taken out of the first, by somebody who is in it and may
    // change agents: nobody else, who is told what they are told of a
    // channel that is not there.
    let refused = rig.agents.leave_channel(&first.id, "builder", &nell).await.unwrap_err();
    let nowhere = rig.agents.leave_channel("ch_nonesuch", "builder", &nell).await.unwrap_err();
    assert_eq!(said(&refused, &first.id), said(&nowhere, "ch_nonesuch"));
    let talks = Principal { scopes: vec!["channels".into()], ..person("mira") };
    assert_eq!(
        rig.agents.leave_channel(&first.id, "builder", &talks).await.unwrap_err().kind,
        ErrorKind::Denied
    );
    rig.agents.leave_channel(&first.id, "builder", &mira).await.unwrap();
    assert!(!place(&first).exists() && place(&second).exists() && place(&third).exists());
    let left = remembered().await;
    assert!(!remembers(&left, &first) && remembers(&left, &second) && remembers(&left, &third), "{left:?}");
    assert_eq!(left.len(), 3, "what it remembers of the open channel stays: {left:?}");
    assert_eq!(channels.kept_with("agent:builder").len(), 2);

    // A step has it go on with the second's conversation; then that
    // channel is taken away. Its folder for it and what it remembered of
    // it go, what it was asked by a message there goes as it always
    // did, and the step's work stays without its words.
    let task = TurnRequest {
        slot: "builder".into(),
        kind: "task".into(),
        thread: format!("channel:{}", second.id),
        prompt: "the greengage figures, please".into(),
        ..TurnRequest::default()
    };
    rig.agents.enqueue(task).await.unwrap();
    rig.turns_finished(5).await;
    channels.remove(&second.id, &ada()).await.unwrap();
    rig.agents.forget_channel(&second.id).await.unwrap();
    assert!(!place(&second).exists() && place(&third).exists());
    assert!(!remembers(&remembered().await, &second));
    let turns = rig.agents.turns_for("builder", 50, &ada()).await.unwrap();
    let of_it: Vec<_> = turns.iter().filter(|turn| turn.kind == "task").collect();
    assert_eq!(of_it.len(), 1);
    assert!(of_it[0].kept_from && of_it[0].prompt.is_empty() && of_it[0].reply.is_empty());
    assert!(of_it[0].finished.is_some() && of_it[0].cost_usd == 0.01);
    let kept_in = rig.of_turns(|connection| {
        let mut statement = connection.prepare("SELECT prompt || reply FROM turns")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<Vec<String>>>()
    });
    let words = kept_in.await.concat().join("\n");
    assert!(!words.contains("greengage") && words.contains("and in the open"), "{words}");

    // It is taken out of the third while a turn asked there is in hand:
    // its key is closed to the channel at once, and its folder there
    // goes when its program has ended.
    channels.post(&third.id, &mira, "@builder hold this [gate:held]", None).await.unwrap();
    rig.fake.saw(6).await;
    rig.agents.leave_channel(&third.id, "builder", &mira).await.unwrap();
    assert!(place(&third).exists(), "its program is at work there");
    assert!(!opens(&rig, &third.id).await);
    rig.fake.open("held");
    soon("its folder for the channel to go with its turn", async {
        while place(&third).exists() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(!remembers(&remembered().await, &third));

    // And taken away for good, nothing of its own is left on the device.
    rig.agents
        .ensure(
            SlotSpec { name: "builder".into(), channel: Some(first.id.clone()), ..SlotSpec::default() },
            &mira,
        )
        .await
        .unwrap();
    channels.post(&first.id, &mira, "@builder once more", None).await.unwrap();
    rig.turns_finished(7).await;
    assert!(place(&first).exists());
    rig.agents.remove("builder", &ada(), &rig.engine, false).await.unwrap();
    assert!(!rig.dir.join("slots/builder").exists());
    assert!(remembered().await.is_empty());
}

#[tokio::test]
async fn a_cards_turns_are_found_by_its_conversations_and_one_of_a_kept_channel_is_shown_its_figures_alone() {
    let (rig, kept) = kept().await;
    let (mira, nell) = (person("mira"), person("nell"));
    let work = |thread: &str, channel: Option<&str>, prompt: &str| TurnRequest {
        slot: "builder".into(),
        kind: "task".into(),
        thread: thread.into(),
        prompt: prompt.into(),
        channel: channel.map(str::to_string),
        ..Default::default()
    };
    // Its work, answered in the kept channel; a seat at its review; and another card's.
    rig.agents.enqueue(work("lane:shop-4", Some(&kept.id), "work the quince")).await.unwrap();
    rig.agents.enqueue(work("review:shop-4:1", None, "look at the quince")).await.unwrap();
    rig.agents.enqueue(work("lane:shop-41", None, "another card")).await.unwrap();
    rig.turns_finished(3).await;

    let asked = agents::spent::conversations("shop-4");
    let whole = rig.agents.turns_in(&asked, &mira).await.unwrap();
    let threads: Vec<&str> = whole.iter().map(|(thread, _)| thread.as_str()).collect();
    assert_eq!(
        threads,
        ["lane:shop-4", "review:shop-4:1"],
        "in the order they were asked, and no other card's"
    );
    assert!(whole[0].1.prompt.contains("quince") && !whole[0].1.kept_from);
    // Somebody who does not read the channel is handed the turn by the
    // conversation it is of, with its figures and none of its words.
    let bare = rig.agents.turns_in(&asked, &nell).await.unwrap();
    assert_eq!(bare[0].0, "lane:shop-4");
    let (one, all) = (&bare[0].1, &whole[0].1);
    assert!(one.kept_from && one.prompt.is_empty() && one.reply.is_empty() && one.decision.is_none());
    assert_eq!(
        (one.cost_usd, one.output_tokens, one.started),
        (all.cost_usd, all.output_tokens, all.started)
    );
    assert_eq!(bare[1], whole[1]);
}
