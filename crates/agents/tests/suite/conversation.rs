use agents::SlotSpec;
use serde_json::json;

use crate::{Rig, ada};

#[tokio::test]
async fn a_message_in_a_channel_with_one_agent_gets_an_answer_from_it() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    rig.fake.will_say("The cart needs a database table first.");
    let asked = rig.say("How should we build the cart?").await;
    let finished = rig.turns_finished(1).await;

    assert_eq!(
        rig.transcript().await,
        ["Ada: How should we build the cart?", "builder: The cart needs a database table first."]
    );
    let reply = rig.studio.channels.messages(&crate::own(), "shop", None, 1).await.unwrap().remove(0);
    assert_eq!(
        (reply.author.as_str(), reply.reply_to.as_deref()),
        ("agent:builder", Some(asked.id.as_str()))
    );

    let data = &finished[0].data;
    assert_eq!(
        (data["slot"].as_str(), data["ok"].as_bool(), data["cost_usd"].as_f64()),
        (Some("builder"), Some(true), Some(0.01))
    );
    assert_eq!((data["input_tokens"].as_u64(), data["output_tokens"].as_u64()), (Some(100), Some(20)));
    let turns = rig
        .journal
        .query_metrics(journal::MetricQuery { name: "agent.turn".into(), ..Default::default() })
        .await
        .unwrap();
    assert_eq!((turns[0].total.count, turns[0].total.errors), (1, 0));
    let cost = rig
        .journal
        .query_metrics(journal::MetricQuery { name: "agent.cost_usd".into(), ..Default::default() })
        .await
        .unwrap();
    assert_eq!((cost[0].total.avg, cost[0].total.sum), (0.01, 0.01));
    // What it read and wrote is measured where its program says, each added up as what it is.
    for (name, tokens) in [("agent.input_tokens", 100.0), ("agent.output_tokens", 20.0)] {
        let spent = rig
            .journal
            .query_metrics(journal::MetricQuery { name: name.into(), ..Default::default() })
            .await
            .unwrap();
        assert_eq!((spent[0].total.count, spent[0].total.sum), (1, tokens), "{name}");
        assert_eq!(blueprint::labels::unit(name), Some("tokens"));
    }
}

#[tokio::test]
async fn a_turn_whose_program_says_nothing_of_what_it_cost_is_kept_as_not_reported_and_never_as_nought() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    *rig.fake.says_no_cost.lock().unwrap() = true;
    rig.say("How should we build the cart?").await;
    let finished = rig.turns_finished(1).await;
    // Said: its tokens, and nothing of a cost.
    let data = &finished[0].data;
    assert_eq!(data.get("cost_usd"), None, "{data}");
    assert_eq!((data["input_tokens"].as_u64(), data["output_tokens"].as_u64()), (Some(100), Some(20)));
    // Kept: not reported, which a cost of nought is not.
    let turn = rig.agents.turns("builder", 1).await.unwrap().remove(0);
    assert_eq!((turn.cost_usd, turn.cost_said), (0.0, false));
    // Measured: nothing of a cost, so that what is added up is only what was said.
    let cost = rig
        .journal
        .query_metrics(journal::MetricQuery { name: "agent.cost_usd".into(), ..Default::default() })
        .await
        .unwrap();
    assert!(cost.is_empty(), "{cost:?}");

    // One whose program says is kept as said, and the two apart after a restart.
    *rig.fake.says_no_cost.lock().unwrap() = false;
    rig.say("@builder and the till?").await;
    rig.turns_finished(2).await;
    let mut rig = rig;
    rig.restart().await;
    let turns = rig.agents.turns("builder", 2).await.unwrap();
    let kept: Vec<(f64, bool)> = turns.iter().map(|turn| (turn.cost_usd, turn.cost_said)).collect();
    assert_eq!(kept, [(0.01, true), (0.0, false)]);
}

#[tokio::test]
async fn a_turn_a_message_asked_for_is_handed_with_what_the_message_said_apart_from_all_its_agent_was_sent() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    rig.say("How should we build the cart?").await;
    rig.turns_finished(1).await;
    rig.say("@builder and the till?").await;
    rig.turns_finished(2).await;
    // A piece of work no message asked for.
    let work = agents::TurnRequest {
        slot: "builder".into(),
        kind: "task".into(),
        prompt: "tidy the shelf".into(),
        ..Default::default()
    };
    rig.agents.enqueue(work).await.unwrap();
    rig.turns_finished(3).await;

    // Newest first. What a message asked is its own words; what its
    // agent was sent says more: where it is, what was said before, and
    // then the words. A turn no message asked for has none apart.
    let turns = rig.agents.turns_for("builder", 50, &ada()).await.unwrap();
    let asked: Vec<Option<&str>> = turns.iter().map(|turn| turn.asked.as_deref()).collect();
    assert_eq!(asked, [None, Some("@builder and the till?"), Some("How should we build the cart?")]);
    assert_eq!(turns[0].prompt, "tidy the shelf");
    for turn in &turns[1..] {
        let words = turn.asked.as_deref().unwrap();
        assert!(turn.prompt.starts_with("You are in the channel #shop."), "{}", turn.prompt);
        assert!(turn.prompt.ends_with(words) && turn.prompt.len() > words.len() + 60, "{}", turn.prompt);
    }
    assert!(turns[1].prompt.contains("Earlier messages") && turns[1].prompt.contains("build the cart"));
    // Handed on, the word is there for a turn a message asked for, and
    // is not there at all for one that none did.
    let handed = serde_json::to_value(&turns).unwrap();
    assert_eq!(handed[2]["asked"], "How should we build the cart?");
    assert!(handed[0].get("asked").is_none(), "{}", handed[0]);
    // It is the channel's to say: somebody who may not read channels is
    // handed no message's words apart, whatever else they are shown.
    let reads_none = kernel::Principal { scopes: vec!["agents:read".into()], ..ada() };
    let theirs = rig.agents.turns_for("builder", 50, &reads_none).await.unwrap();
    assert!(theirs.iter().all(|turn| turn.asked.is_none()) && theirs.len() == 3);
    // And a turn as it is kept says none: nothing is written for it.
    let kept = rig.agents.turns("builder", 50).await.unwrap();
    assert!(kept.iter().all(|turn| turn.asked.is_none()));
}

#[tokio::test]
async fn an_agents_message_says_which_model_wrote_it_as_far_as_that_is_known() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    let newest =
        async || rig.studio.channels.messages(&crate::own(), "shop", None, 1).await.unwrap().remove(0);

    // Nothing was asked for and the program does not say: it is not known.
    rig.say("one").await;
    rig.turns_finished(1).await;
    assert_eq!(newest().await.model, None);

    // What the slot asks for is the best there is, until the program says.
    let asks = SlotSpec { name: "builder".into(), model: Some("fast".into()), ..SlotSpec::default() };
    rig.agents.ensure(asks, &ada()).await.unwrap();
    rig.say("two").await;
    rig.turns_finished(2).await;
    assert_eq!(newest().await.model.as_deref(), Some("fast"));

    // The program's own word for it is what is put down.
    *rig.fake.names_its_model.lock().unwrap() = Some("fast-2026-01".into());
    let asked = rig.say("three").await;
    rig.turns_finished(3).await;
    let reply = newest().await;
    assert_eq!(reply.model.as_deref(), Some("fast-2026-01"));
    let told = rig.event("channel.message", |event| event.data["reply_to"] == asked.id.as_str()).await;
    assert_eq!(told.data["model"], "fast-2026-01", "and whoever is following is told too");
    assert_eq!(asked.model, None, "a person's message is nobody's model");

    // An agent left to its provider's choice is known by what its program
    // last said it was using, and still is after a restart.
    rig.agent("usual", "fake").await;
    let usual = rig.agents.slot("usual").await.unwrap();
    assert_eq!(rig.agents.model_name(&usual), None, "until it has said");
    rig.say("@usual four").await;
    rig.turns_finished(4).await;
    assert_eq!(rig.agents.model_name(&usual).as_deref(), Some("fast-2026-01"));
    let mut rig = rig;
    rig.restart().await;
    assert_eq!(rig.agents.model_name(&usual).as_deref(), Some("fast-2026-01"));
}

#[tokio::test]
async fn in_a_thread_a_person_goes_on_talking_to_the_agent_they_were_talking_to() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    rig.agent("reviewer", "fake").await;
    let channels = &rig.studio.channels;
    let who = |events: &[kernel::Event]| -> Vec<String> {
        events.iter().map(|event| event.data["slot"].as_str().unwrap().to_string()).collect()
    };

    // With two agents in the channel one has to be named.
    let asked = rig.say("@builder how should we build the cart?").await;
    rig.turns_finished(1).await;
    rig.say("and the checkout?").await;

    // Under that message, it does not: the thread is a conversation with builder.
    let under = studio::Place { thread: Some(&asked.id), ..studio::Place::default() };
    rig.fake.will_say("The same way.");
    let follow = channels.post_at("shop", &ada(), "and the checkout?", under).await.unwrap();
    let finished = rig.turns_finished(2).await;
    assert_eq!(who(&finished), ["builder", "builder"], "the message in the channel itself went to nobody");

    // Its answer is in the thread, to the message that asked.
    let thread = channels.thread(&crate::own(), &asked.id).await.unwrap();
    let said: Vec<(&str, &str)> = thread.iter().map(|m| (m.author_name.as_str(), m.body.as_str())).collect();
    assert_eq!(
        said,
        [
            ("Ada", "@builder how should we build the cart?"),
            ("Ada", "and the checkout?"),
            ("builder", "The same way.")
        ]
    );
    assert_eq!(thread[2].reply_to.as_deref(), Some(follow.id.as_str()));
    assert_eq!(rig.transcript().await.len(), 3, "and not in the channel: {:?}", rig.transcript().await);

    // It was shown the thread, told how to read the channel, and began a
    // conversation of its own for it rather than carrying on the channel's.
    let seen =
        rig.fake.seen.lock().unwrap().last().map(|seen| (seen.prompt.clone(), seen.session.clone())).unwrap();
    assert!(seen.0.starts_with("You are in a thread in the channel #shop."), "{}", seen.0);
    assert!(seen.0.contains(" read shop` shows what was said in the channel itself"), "{}", seen.0);
    // And which thread it is, how something is posted into it, and that
    // a file is attached and never named by where it is.
    assert!(
        seen.0.contains("What you answer is said in this thread, and so is what you post in #shop"),
        "{}",
        seen.0
    );
    assert!(seen.0.contains(" post shop --attach <file> <words>`"), "{}", seen.0);
    assert!(seen.0.contains(&format!("`--thread {}`", asked.id)), "{}", seen.0);
    assert!(seen.0.contains("`--top` to say it in the channel itself"), "{}", seen.0);
    assert!(
        seen.0.contains("where it is on this device means nothing to whoever reads the channel"),
        "{}",
        seen.0
    );
    assert!(seen.0.contains("[Ada] @builder how should we build the cart?\n"), "{}", seen.0);
    assert!(seen.0.ends_with("The newest message, from Ada, is for you:\nand the checkout?"), "{}", seen.0);
    assert_eq!(seen.1, None, "a thread is remembered as a conversation of its own");

    // Naming another agent brings it into the thread, and then it is the one being talked to.
    channels.post_at("shop", &ada(), "@reviewer what do you think?", under).await.unwrap();
    rig.turns_finished(3).await;
    channels.post_at("shop", &ada(), "go on", under).await.unwrap();
    let finished = rig.turns_finished(4).await;
    assert_eq!(who(&finished)[2..], ["reviewer", "reviewer"]);
}

#[tokio::test]
async fn where_an_agent_is_being_asked_is_the_thread_its_turn_in_hand_was_asked_in() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    rig.agent("reviewer", "fake").await;
    let channels = &rig.studio.channels;
    let asked = rig.say("@builder how should we build the cart?").await;
    rig.turns_finished(1).await;
    assert_eq!(
        rig.agents.asked_under(&crate::own(), "builder", "shop").await,
        None,
        "it has nothing in hand"
    );

    // Asked under that message, and still answering: it is being asked in that thread.
    let under = studio::Place { thread: Some(&asked.id), ..studio::Place::default() };
    channels.post_at("shop", &ada(), "and the checkout? [gate:thread]", under).await.unwrap();
    rig.fake.saw(2).await;
    assert_eq!(
        rig.agents.asked_under(&crate::own(), "builder", "shop").await.as_deref(),
        Some(asked.id.as_str())
    );
    // Nobody else is, and it is not in any other channel.
    assert_eq!(rig.agents.asked_under(&crate::own(), "reviewer", "shop").await, None);
    channels.create("elsewhere", Some("shop"), "", &ada()).await.unwrap();
    assert_eq!(rig.agents.asked_under(&crate::own(), "builder", "elsewhere").await, None);
    assert_eq!(rig.agents.asked_under(&crate::own(), "builder", "no-such-channel").await, None);
    rig.fake.open("thread");
    rig.turns_finished(2).await;
    assert_eq!(rig.agents.asked_under(&crate::own(), "builder", "shop").await, None, "it has answered");

    // Asked in the channel itself, it is being asked in no thread.
    rig.say("@builder and payments? [gate:channel]").await;
    rig.fake.saw(3).await;
    assert_eq!(rig.agents.asked_under(&crate::own(), "builder", "shop").await, None);
    rig.fake.open("channel");
    rig.turns_finished(3).await;
}

#[tokio::test]
async fn a_thread_under_a_message_an_agent_answered_is_a_conversation_with_that_agent() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    // It was the only agent, so it answered without being named.
    let asked = rig.say("how should we build the cart?").await;
    rig.turns_finished(1).await;
    rig.agent("reviewer", "fake").await;

    let under = studio::Place { thread: Some(&asked.id), ..studio::Place::default() };
    rig.studio.channels.post_at("shop", &ada(), "why that way?", under).await.unwrap();
    let finished = rig.turns_finished(2).await;
    assert_eq!(finished[1].data["slot"], "builder", "the one that answered what the thread is under");
}

#[tokio::test]
async fn an_agent_is_shown_the_conversation_and_remembers_it_between_turns() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    rig.fake.will_say("A table called carts.");
    rig.say("How should we build the cart?").await;
    rig.turns_finished(1).await;
    rig.say("And payments?").await;
    rig.turns_finished(2).await;

    let seen = rig.fake.seen.lock().unwrap().clone();
    assert_eq!(seen[0].session, None, "a new conversation");
    assert_eq!(seen[1].session.as_deref(), Some("session-1"), "the same conversation, continued");
    // It is told where it is, and how a file is shown to whoever reads the
    // channel, in the words of this command line.
    assert_eq!(
        seen[1].prompt,
        format!(
            "You are in the channel #shop. To show somebody a file, attach it, since where it is on this \
             device means nothing to whoever reads the channel: `{cli} post shop --attach <file> <words>`. \
             Earlier messages, oldest first:\n\
             [Ada] How should we build the cart?\n\
             [builder] A table called carts.\n\
             \nThe newest message, from Ada, is for you:\nAnd payments?",
            cli = brand::CLI
        )
    );
}

#[tokio::test]
async fn with_several_agents_only_the_one_addressed_answers() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    rig.agent("reviewer", "fake").await;

    rig.say("Good morning, everyone.").await;
    rig.say("@reviewer could you look at the cart?").await;
    let finished = rig.turns_finished(1).await;
    assert_eq!(finished[0].data["slot"], "reviewer");

    // An agent may address another; nobody answers themselves.
    rig.fake.will_say("@builder the cart has no tests.");
    rig.fake.will_say("Adding them.");
    rig.say("@reviewer and tell the builder what you find.").await;
    let finished = rig.turns_finished(3).await;
    assert_eq!(finished[2].data["slot"], "builder");
    assert_eq!(
        rig.transcript().await[3..],
        [
            "Ada: @reviewer and tell the builder what you find.",
            "reviewer: @builder the cart has no tests.",
            "builder: Adding them."
        ]
    );
    assert_eq!(
        rig.fake.seen.lock().unwrap().len(),
        3,
        "the greeting addressed nobody, so nobody took a turn"
    );
}

#[tokio::test]
async fn a_blueprint_posting_in_a_channel_does_not_set_the_agent_off() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    rig.studio.channels.post_system(&crate::own(), "shop", "fleet", "The fleet is up.").await.unwrap();
    rig.say("thanks").await;
    rig.turns_finished(1).await;
    assert_eq!(rig.fake.prompts().len(), 1, "only the person's message started a turn");
}

#[tokio::test]
async fn a_turn_that_fails_says_so_in_the_channel() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    rig.fake.will_fail("usage limit reached; resets at 14:00");
    rig.say("Build the cart.").await;
    let finished = rig.turns_finished(1).await;

    assert_eq!(
        (finished[0].data["ok"].as_bool(), finished[0].data["error"].as_str()),
        (Some(false), Some("usage limit reached; resets at 14:00"))
    );
    // Said of the agent by the factory's bot, which is what says whatever
    // the factory itself has to say in a channel.
    assert_eq!(
        rig.transcript().await[1],
        "conductor: builder could not answer: usage limit reached; resets at 14:00"
    );
    let said = rig.studio.channels.messages(&crate::own(), "shop", None, 100).await.unwrap();
    assert_eq!(said[1].author, "bot:conductor");
    let turn = rig.agents.turns("builder", 1).await.unwrap().remove(0);
    assert_eq!(
        (turn.status.as_str(), turn.error.as_deref()),
        ("failed", Some("usage limit reached; resets at 14:00"))
    );
}

#[tokio::test]
async fn one_agent_takes_one_turn_at_a_time_in_the_order_asked() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    rig.say("first [gate:one]").await;
    rig.say("second").await;
    rig.fake.saw(1).await;
    let mut queued = rig.journal.follow_every(journal::Places::default(), "agent.turn.queued");
    for _ in 0..2 {
        crate::soon("both turns to be queued", queued.next()).await.unwrap();
    }
    assert_eq!(rig.fake.prompts().len(), 1, "the second waits for the first");
    let queued: Vec<String> =
        rig.agents.turns("builder", 5).await.unwrap().into_iter().map(|turn| turn.status).collect();
    assert_eq!(queued, ["queued", "running"]);

    rig.fake.open("one");
    rig.turns_finished(2).await;
    assert!(rig.fake.prompts()[1].ends_with("is for you:\nsecond"));
}

#[tokio::test]
async fn work_in_progress_and_messages_missed_are_picked_up_after_a_restart() {
    let mut rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    rig.say("long job [gate:job]").await;
    rig.fake.saw(1).await;

    rig.engine.halt();
    rig.agents.halt();
    rig.say("posted while it was down").await;
    rig.restart().await;
    rig.fake.open("job");
    let finished = rig.turns_finished(2).await;

    assert!(finished.iter().all(|event| event.data["ok"] == json!(true)));
    let prompts = rig.fake.prompts();
    assert_eq!(prompts.len(), 3, "the interrupted turn ran again, then the missed message");
    assert!(prompts[1].contains("long job") && prompts[2].ends_with("posted while it was down"));
    assert_eq!(
        rig.agents.turns("builder", 10).await.unwrap().len(),
        2,
        "two turns, not three: the first was resumed, not duplicated"
    );
}

#[tokio::test]
async fn the_builtin_assistant_sets_up_a_project_and_then_its_fleet() {
    let rig = Rig::new().await;
    rig.agent("helper", "builtin").await;

    rig.say("Please set up a new project Riverside Shop").await;
    rig.turns_finished(1).await;
    let project = rig.studio.projects.get("riverside-shop").await.unwrap();
    assert_eq!(project.name, "Riverside Shop");
    let said = rig.transcript().await[1].clone();
    assert!(said.starts_with("helper: Created the project **Riverside Shop** (`riverside-shop`)"), "{said}");
    assert!(said.contains("`docs/overview.md`") && said.contains("ask me to set up its fleet"), "{said}");
    // Its board comes before its fleet, which works one, and is said first with how it is made.
    let board = format!("`{} apply std/board --project riverside-shop`", brand::CLI);
    assert!(said.find(&board).is_some_and(|at| at < said.find("set up its fleet").unwrap()), "{said}");

    rig.say("Now set up your fleet for Riverside Shop").await;
    rig.turns_finished(2).await;
    // Kept by the factory for the project, and never in its folder.
    let fleet = format!("@blueprints/{}", brand::blueprint_file("fleet"));
    let source = rig.studio.projects.read("riverside-shop", &fleet).await.unwrap();
    assert!(source.contains("blueprint: fleet") && source.contains("default: riverside-shop"));
    let said = rig.transcript().await[3].clone();
    assert!(
        said.contains(&format!("`{fleet}` is kept for the project now")) && said.contains("Node view"),
        "{said}"
    );
    // Asked for where there is no board, it says a fleet works one and how that is given.
    assert!(
        said.contains("has no board yet, and a fleet works one") && said.ends_with(&format!("({board}).")),
        "{said}"
    );

    // Asking twice is harmless, and it is honest about what it cannot do.
    rig.say("set up a new project Riverside Shop").await;
    rig.say("refactor the checkout module").await;
    rig.turns_finished(4).await;
    let transcript = rig.transcript().await;
    assert_eq!(transcript[6], "helper: there is already a project called `riverside-shop`");
    assert!(
        transcript[7].contains("I am the built-in assistant")
            && transcript[7].contains("help extensions")
            && transcript[7].contains(&format!("`{} agent add helper --adapter <package>`", brand::CLI)),
        "{}",
        transcript[7]
    );
    assert_eq!(rig.studio.projects.list().await.unwrap().len(), 2);
}

#[tokio::test]
async fn the_builtin_assistant_acts_only_as_far_as_its_slot_may() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    rig.studio.channels.create("shop", Some("shop"), "", &ada()).await.unwrap();
    let spec = SlotSpec {
        name: "reader".into(),
        adapter: Some("builtin".into()),
        channel: Some("shop".into()),
        scopes: Some(vec!["channels".into(), "projects:read".into()]),
        ..SlotSpec::default()
    };
    rig.agents.ensure(spec, &ada()).await.unwrap();
    rig.say("set up a new project Forbidden").await;
    rig.turns_finished(1).await;
    assert!(rig.transcript().await[1].contains("not allowed to do this"), "{:?}", rig.transcript().await);
    assert_eq!(rig.studio.projects.list().await.unwrap().len(), 1);
}
