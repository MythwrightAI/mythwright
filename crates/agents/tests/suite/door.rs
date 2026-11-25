//! The door of a channel kept to its members, as whoever keeps agents
//! holds it (`agents::door`): what becomes of the agents in a channel,
//! and of what they were asked there, as it is kept, opened again and
//! told that its agents are asked nowhere else; an agent that is there
//! brought into one; and an agent's program tried in the home it has
//! for one.

use agents::{SlotSpec, TurnInfo, TurnRequest};
use kernel::{ErrorKind, Principal};

use crate::kept::{person, said, walls};
use crate::{Rig, ada, own};

/// A step's work for `builder` that goes on with the conversation `thread`.
fn task(thread: &str, prompt: &str) -> TurnRequest {
    TurnRequest {
        slot: "builder".into(),
        kind: "task".into(),
        thread: thread.into(),
        prompt: prompt.into(),
        ..TurnRequest::default()
    }
}

/// What `by` is shown of `builder`'s turns, oldest first: for each
/// whether its words were left out, and the words.
async fn shown(rig: &Rig, by: &Principal) -> Vec<(bool, String)> {
    let mut turns: Vec<TurnInfo> = rig.agents.turns_for("builder", 50, by).await.unwrap();
    turns.reverse();
    turns.into_iter().map(|turn| (turn.kept_from, format!("{} {}", turn.prompt, turn.reply))).collect()
}

/// Which kept channel each turn the files keep says it is of, oldest first.
async fn written(rig: &Rig) -> Vec<Option<String>> {
    let asked = |connection: &mut rusqlite::Connection| {
        let mut statement = connection.prepare("SELECT nth, kept FROM turns")?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get(1)?)))?;
        rows.collect::<rusqlite::Result<Vec<(i64, Option<String>)>>>()
    };
    let mut all: Vec<(i64, Option<String>)> = rig.of_turns(asked).await.into_iter().flatten().collect();
    all.sort();
    all.into_iter().map(|(_, kept)| kept).collect()
}

/// The conversations `builder`'s program remembers.
async fn remembered(rig: &Rig) -> Vec<String> {
    let kept = rig.journal.kv_list("agent.session").await.unwrap();
    kept.into_iter().map(|(key, _)| key).collect()
}

#[tokio::test]
async fn an_open_channel_is_kept_with_what_its_agents_were_asked_there_and_not_while_one_is_asked_or_could_not_stay()
 {
    let rig = Rig::holding(walls()).await;
    rig.agent("builder", "fake").await;
    let (mira, nell) = (person("mira"), person("nell"));
    let channels = &rig.studio.channels;
    let shop = channels.get(&own(), "shop").await.unwrap();
    channels.join(&own(), "shop", "user:mira", "mira").await.unwrap();
    let yard = channels.create("yard", None, "", &ada()).await.unwrap();
    let into = SlotSpec { name: "builder".into(), channel: Some("yard".into()), ..SlotSpec::default() };
    rig.agents.ensure(into, &ada()).await.unwrap();

    // Asked there while it is open, by a message and by a step that
    // goes on with its conversation; and asked elsewhere.
    rig.say("@builder count the quince").await;
    rig.turns_finished(1).await;
    rig.agents.enqueue(task("channel:shop", "the greengage figures")).await.unwrap();
    rig.turns_finished(2).await;
    channels.post("yard", &ada(), "@builder and the medlar", None).await.unwrap();
    rig.turns_finished(3).await;
    // And asked elsewhere to go on with a thread of this channel, which
    // is this channel's conversation wherever its answer is said.
    let root = rig.say("@builder a thread begins here").await;
    rig.turns_finished(4).await;
    let mut under = task(&format!("thread:{}", root.id), "and the sloe under it");
    under.channel = Some("yard".into());
    rig.agents.enqueue(under).await.unwrap();
    rig.turns_finished(5).await;
    assert!(shown(&rig, &nell).await.iter().all(|(left_out, _)| !left_out), "it is open: anybody is shown");
    assert_eq!(remembered(&rig).await.len(), 4, "{:?}", remembered(&rig).await);

    // Nobody keeps a channel they are not in, as any change of an open
    // one is refused them, or read nothing of what it then is.
    let outside = rig.agents.keep_channel("shop", &nell).await.unwrap_err();
    assert_eq!(outside.kind, ErrorKind::Denied);
    assert!(outside.message.contains("is not in #shop"), "{}", outside.message);
    // Not while an agent in it could be in no kept channel: it is not
    // taken out behind anybody's back, and the sentence says the ways.
    let bare = SlotSpec {
        name: "roamer".into(),
        adapter: Some("fake".into()),
        wall: Some(false),
        channel: Some("shop".into()),
        ..SlotSpec::default()
    };
    rig.agents.ensure(bare, &ada()).await.unwrap();
    let refused = rig.agents.keep_channel("shop", &mira).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(refused.message.contains("`roamer` works with no wall"), "{}", refused.message);
    assert!(refused.message.contains("#shop is not kept while it is in it"), "{}", refused.message);
    assert!(refused.message.contains(&format!("`{} agent add roamer --wall`", brand::CLI)));
    assert!(refused.message.contains(&format!("`{} channel leave shop --agent roamer`", brand::CLI)));
    rig.agents.leave_channel("shop", "roamer", &ada()).await.unwrap();
    // Nor while one is being asked something there.
    rig.say("@builder hold this [gate:held]").await;
    rig.fake.saw(6).await;
    let refused = rig.agents.keep_channel("shop", &mira).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(refused.message.contains("`builder` is being asked something in #shop"), "{}", refused.message);
    assert_eq!(channels.get(&own(), "shop").await.unwrap().name, "shop", "open as it was");
    assert!(written(&rig).await.iter().all(Option::is_none) && !channels.is_kept(&shop.id));
    rig.fake.open("held");
    rig.turns_finished(6).await;

    // Kept, by somebody who is in it: its agent stays, behind its wall.
    let kept = rig.agents.keep_channel("shop", &mira).await.unwrap();
    assert_eq!((kept.kept, kept.title.as_deref(), &kept.name), (true, Some("shop"), &shop.id));
    assert_eq!(channels.kept_with("agent:builder"), std::slice::from_ref(&shop.id));
    // What it was asked there is the channel's from here: written so,
    // shown whole to whoever reads the channel, and to anybody else
    // without its words. What it was asked elsewhere is as it was.
    let of_it = Some(shop.id.clone());
    assert_eq!(
        written(&rig).await,
        [of_it.clone(), of_it.clone(), None, of_it.clone(), of_it.clone(), of_it.clone()]
    );
    let (to_mira, to_nell) = (shown(&rig, &mira).await, shown(&rig, &nell).await);
    assert!(to_mira.iter().all(|(left_out, _)| !left_out) && to_mira[0].1.contains("quince"));
    let left_out: Vec<bool> = to_nell.iter().map(|(left_out, _)| *left_out).collect();
    assert_eq!(left_out, [true, true, false, true, true, true]);
    for marked in ["quince", "greengage", "sloe", "hold this"] {
        assert!(to_nell.iter().all(|(_, words)| !words.contains(marked)), "{marked}: {to_nell:?}");
    }
    assert!(to_nell[2].1.contains("medlar"));
    // And what its program remembered of the conversations held there
    // is forgotten: its next turn there begins in the home it has for
    // that channel, where nothing of them is.
    let remembers = remembered(&rig).await;
    assert_eq!(remembers, [format!("builder/channel:{}", yard.id)]);

    // Asked there now, the turn is the channel's from its queueing.
    channels.post(&shop.id, &mira, "@builder and the damson", None).await.unwrap();
    rig.turns_finished(7).await;
    assert_eq!(written(&rig).await.last().unwrap(), &of_it);
    assert!(rig.dir.join("slots/builder/kept").join(&shop.id).is_dir());
    // Kept again it is as it was.
    assert_eq!(
        rig.agents.keep_channel(&shop.id, &mira).await.unwrap(),
        channels.get(&own(), &shop.id).await.unwrap()
    );

    // ── Opened again ─────────────────────────────────────────────────────
    // Not while its agent is being asked something there.
    channels.post(&shop.id, &mira, "@builder hold this too [gate:again]", None).await.unwrap();
    rig.fake.saw(8).await;
    let refused = rig.agents.open_channel(&shop.id, None, &ada()).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(refused.message.contains("`builder` is being asked something in `shop`"), "{}", refused.message);
    assert!(channels.is_kept(&shop.id));
    rig.fake.open("again");
    rig.turns_finished(8).await;
    // Somebody who does not read it is told what they are told of a
    // channel that is not there.
    let refused = rig.agents.open_channel(&shop.id, None, &nell).await.unwrap_err();
    let nowhere = rig.agents.open_channel("ch_nonesuch", None, &nell).await.unwrap_err();
    assert_eq!(said(&refused, &shop.id), said(&nowhere, "ch_nonesuch"));
    let opened = rig.agents.open_channel(&shop.id, None, &ada()).await.unwrap();
    assert_eq!((opened.kept, opened.name.as_str(), opened.title), (false, "shop", None));
    // What the agent kept of it as a kept channel is gone, and the
    // words of what it was asked there are anybody's who reads it.
    assert!(!rig.dir.join("slots/builder/kept").join(&shop.id).exists());
    assert_eq!(remembered(&rig).await, remembers);
    assert!(channels.kept_with("agent:builder").is_empty());
    let to_nell = shown(&rig, &nell).await;
    assert!(to_nell.iter().all(|(left_out, _)| !left_out) && to_nell[0].1.contains("quince"), "{to_nell:?}");
    // Asked there now it is a turn of no kept channel.
    rig.say("@builder and after").await;
    rig.turns_finished(9).await;
    assert_eq!(written(&rig).await.last().unwrap(), &None);
}

#[tokio::test]
async fn a_kept_channels_agents_are_held_to_it_afterwards_only_once_nothing_asked_elsewhere_is_in_hand() {
    let rig = Rig::holding(walls()).await;
    rig.agent("builder", "fake").await;
    let (mira, nell) = (person("mira"), person("nell"));
    let channels = &rig.studio.channels;
    let kept = channels.create_kept("Pay review", None, "", &mira).await.unwrap();
    rig.agents.bring_in(&kept.id, "builder", &mira).await.unwrap();

    // Not while it has something asked elsewhere in hand.
    rig.say("@builder hold this [gate:held]").await;
    rig.fake.saw(1).await;
    let refused = rig.agents.hold_alone(&kept.id, true, &mira).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(
        refused.message.contains("`builder` has something it was asked elsewhere"),
        "{}",
        refused.message
    );
    assert!(
        channels.alone_in("agent:builder").is_none() && !channels.get(&own(), &kept.id).await.unwrap().alone
    );
    rig.fake.open("held");
    rig.turns_finished(1).await;
    // Nobody who is not in it says anything of it.
    let refused = rig.agents.hold_alone(&kept.id, true, &nell).await.unwrap_err();
    let nowhere = rig.agents.hold_alone("ch_nonesuch", true, &nell).await.unwrap_err();
    assert_eq!(said(&refused, &kept.id), said(&nowhere, "ch_nonesuch"));

    // With nothing of it asked elsewhere, it is held: asked nowhere
    // else from there, and where it is.
    let held = rig.agents.hold_alone(&kept.id, true, &mira).await.unwrap();
    assert!(held.alone && channels.alone_in("agent:builder") == Some(kept.id.clone()));
    let refused = rig.agents.enqueue(task("lane:shop-1", "a card")).await.unwrap_err();
    assert!(refused.message.contains("whose agents are asked nowhere else"), "{}", refused.message);
    channels.post(&kept.id, &mira, "@builder count the quince", None).await.unwrap();
    rig.turns_finished(2).await;
    // With a turn of that channel in hand it is still said, and unsaid.
    channels.post(&kept.id, &mira, "@builder hold this [gate:there]", None).await.unwrap();
    rig.fake.saw(3).await;
    rig.agents.hold_alone(&kept.id, false, &mira).await.unwrap();
    rig.agents.hold_alone(&kept.id, true, &mira).await.unwrap();
    rig.fake.open("there");
    rig.turns_finished(3).await;
    // Let go, it is asked anywhere again.
    let free = rig.agents.hold_alone(&kept.id, false, &mira).await.unwrap();
    assert!(!free.alone && channels.alone_in("agent:builder").is_none());
    rig.agents.enqueue(task("lane:shop-1", "a card")).await.unwrap();
    rig.turns_finished(4).await;
}

#[tokio::test]
async fn an_agent_that_is_there_is_brought_into_a_channel_as_it_is_and_into_a_kept_one_only_where_it_may_be()
{
    let rig = Rig::holding(walls()).await;
    rig.agent("builder", "fake").await;
    let (mira, nell) = (person("mira"), person("nell"));
    let channels = &rig.studio.channels;
    let kept = channels.create_kept("Pay review", None, "", &mira).await.unwrap();
    let bare = SlotSpec {
        name: "roamer".into(),
        adapter: Some("fake".into()),
        wall: Some(false),
        ..SlotSpec::default()
    };
    rig.agents.ensure(bare, &ada()).await.unwrap();
    let within = async |channel: &str| {
        let found = channels.get(&own(), channel).await.unwrap();
        found.members.into_iter().map(|one| one.actor).collect::<Vec<_>>()
    };

    // Whether each could be in a channel that is about to be made is
    // asked first: an agent there is not, and one that could be in no
    // kept channel, are refused a kept one, and nothing of an open one.
    let names = |names: &[&str]| names.iter().map(|name| name.to_string()).collect::<Vec<_>>();
    rig.agents.could_be_in(&names(&["builder"]), true, true, &mira).await.unwrap();
    rig.agents.could_be_in(&names(&["builder", "roamer"]), false, false, &mira).await.unwrap();
    let refused =
        rig.agents.could_be_in(&names(&["builder", "roamer"]), true, false, &mira).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(refused.message.contains("`roamer` works with no wall"), "{}", refused.message);
    let nobody = rig.agents.could_be_in(&names(&["nobody"]), false, false, &mira).await.unwrap_err();
    assert_eq!(nobody.kind, ErrorKind::NotFound);
    let reads = Principal { scopes: vec!["channels".into(), "agents:read".into()], ..person("mira") };
    assert_eq!(
        rig.agents.could_be_in(&names(&["builder"]), false, false, &reads).await.unwrap_err().kind,
        ErrorKind::Denied
    );
    rig.agents.could_be_in(&[], true, true, &reads).await.unwrap();

    // Brought in by somebody who is in the channel and may change
    // agents: nothing else of it changes, one that was stopped among
    // them.
    rig.agents.deactivate("builder").await.unwrap();
    let before = rig.agents.slot("builder").await.unwrap();
    rig.agents.bring_in(&kept.id, "builder", &mira).await.unwrap();
    assert_eq!(rig.agents.slot("builder").await.unwrap(), before);
    assert_eq!(within(&kept.id).await, ["user:mira", "agent:builder"]);
    assert_eq!(channels.kept_with("agent:builder"), std::slice::from_ref(&kept.id));
    // Somebody who is not in it is told what they are told of a channel
    // that is not there, and somebody who may not change agents is
    // refused in the scope's words.
    let refused = rig.agents.bring_in(&kept.id, "builder", &nell).await.unwrap_err();
    let nowhere = rig.agents.bring_in("ch_nonesuch", "builder", &nell).await.unwrap_err();
    assert_eq!(said(&refused, &kept.id), said(&nowhere, "ch_nonesuch"));
    assert_eq!(rig.agents.bring_in(&kept.id, "builder", &reads).await.unwrap_err().kind, ErrorKind::Denied);
    // One that may be in no kept channel is refused, and one there is not.
    let refused = rig.agents.bring_in(&kept.id, "roamer", &mira).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(refused.message.contains("an agent with no wall is in no channel kept to its members"));
    assert_eq!(rig.agents.bring_in(&kept.id, "nobody", &mira).await.unwrap_err().kind, ErrorKind::NotFound);
    assert_eq!(within(&kept.id).await, ["user:mira", "agent:builder"]);
    // Into an open channel any agent is brought, as it always was.
    rig.agents.bring_in("shop", "roamer", &mira).await.unwrap();
    assert!(within("shop").await.contains(&"agent:roamer".to_string()));
}

#[cfg(unix)]
#[tokio::test]
async fn an_agents_program_is_tried_in_the_home_it_has_for_a_kept_channel_by_somebody_who_is_in_it() {
    use std::path::Path;

    use crate::walls::{Walled, stand_in, words};
    let (walls, told) = stand_in();
    let walled = Walled::holding(walls).await;
    let rig = &walled.rig;
    walled
        .package_says(
            "  wall:\n    beside: [\"..\"]\n    keeps: [\".standin\", \".needs\"]\n    signin: [\".needs\"]\n",
        )
        .await;
    std::fs::create_dir_all(walled.home.join(".needs")).unwrap();
    std::fs::write(walled.home.join(".needs/thing"), "what it needs").unwrap();
    let (mira, nell) = (person("mira"), person("nell"));
    let kept = rig.studio.channels.create_kept("Pay review", None, "", &mira).await.unwrap();
    rig.agents.bring_in(&kept.id, "builder", &mira).await.unwrap();

    // Started where a turn asked there is: the home that is the
    // channel's is shown where the person's is, and no other of its.
    let tried = rig.agents.try_program_in("builder", Some(&kept.id), &mira).await.unwrap();
    assert!(tried.ok && tried.walled, "{tried:?}");
    let said = words(&told);
    let slot = rig.dir.join("slots/builder");
    let its_own = slot.join("kept").join(&kept.id).join("home");
    let shown = said.windows(3).find(|three| three[0] == "--bind" && Path::new(&three[2]) == walled.home);
    assert_eq!(shown.map(|three| Path::new(&three[1])), Some(its_own.as_path()), "{said:#?}");
    assert!(!said.iter().any(|word| Path::new(word) == slot.join("home")), "{said:#?}");
    // No work was given.
    assert!(rig.events("agent.turn.**").await.is_empty());

    // Somebody who is not in the channel is told what they are told of
    // one that is not there; an open one is no place to try it in; and
    // an agent that is not in the channel is asked nothing there.
    let refused = rig.agents.try_program_in("builder", Some(&kept.id), &nell).await.unwrap_err();
    let nowhere = rig.agents.try_program_in("builder", Some("ch_nonesuch"), &nell).await.unwrap_err();
    assert_eq!(said_of(&refused, &kept.id), said_of(&nowhere, "ch_nonesuch"));
    let open = rig.agents.try_program_in("builder", Some("shop"), &mira).await.unwrap_err();
    assert_eq!(open.kind, ErrorKind::Invalid);
    assert!(open.message.contains("that channel is open"), "{}", open.message);
    rig.agents.leave_channel(&kept.id, "builder", &mira).await.unwrap();
    let outside = rig.agents.try_program_in("builder", Some(&kept.id), &mira).await.unwrap_err();
    assert!(outside.message.contains("`builder` is not in that channel"), "{}", outside.message);
}

/// What a refusal says, with what was asked for taken out of it.
#[cfg(unix)]
fn said_of(refused: &kernel::Error, asked: &str) -> (ErrorKind, String) {
    said(refused, asked)
}
