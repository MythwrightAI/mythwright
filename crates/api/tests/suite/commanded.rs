//! What the factory's bot does when it is spoken to: one of a fixed set of
//! commands, done as whoever asked with what their key holds, or nothing.
//!
//! The bot is a deputy with everything in reach, so each test here that
//! asks with a key that holds less expects to be refused by the name of
//! what it lacks, and looks to see that nothing was done.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// A project with a board and a card on it, and a channel of its own:
/// the card's id.
async fn shop(rig: &Rig) -> String {
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    rig.send("/api/channels", json!({ "name": "floor", "project": "shop" })).await;
    let card = rig.send("/api/board/shop/items", json!({ "title": "Add a cart" })).await;
    card["id"].as_str().unwrap().to_string()
}

/// A key of the rig's owner that holds `scopes` and nothing else: its token.
async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "narrow", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

/// Says `words` on the floor with `token`: what the bot answered to it.
async fn asked(rig: &Rig, token: &str, words: &str) -> String {
    let body = Some(json!({ "body": words }));
    let (status, message) = rig.call(Method::POST, "/api/channels/floor/messages", Some(token), body).await;
    assert_eq!(status, StatusCode::OK, "{message}");
    answer_to(rig, message["id"].as_str().unwrap()).await
}

/// What the bot answered to the message `id`.
async fn answer_to(rig: &Rig, id: &str) -> String {
    let id = id.to_string();
    let answer = rig
        .event("channel.message", move |event| {
            event.data["reply_to"] == id.as_str() && event.data["author"] == "bot:conductor"
        })
        .await;
    answer.data["body"].as_str().unwrap().to_string()
}

async fn column_of(rig: &Rig, card: &str) -> String {
    let board = rig.read("/api/board/shop").await;
    let found = board["items"].as_array().unwrap().iter().find(|item| item["id"] == card).unwrap();
    found["status"].as_str().unwrap().to_string()
}

async fn said_on_the_floor(rig: &Rig) -> Vec<Value> {
    rig.read("/api/channels/floor/messages?limit=200").await["messages"].as_array().unwrap().clone()
}

#[tokio::test]
async fn a_message_that_names_the_bot_is_one_of_the_command_lines_commands_and_is_answered_under_it() {
    let rig = Rig::new().await;
    let card = shop(&rig).await;
    let board = asked(&rig, &rig.token, "@conductor board").await;
    assert!(board.contains(&format!("- draft: `{card}` Add a cart")), "{board}");
    assert!(board.contains("- done: nothing"), "{board}");
    // Wherever in the message its name is, what follows is the command.
    let moved =
        asked(&rig, &rig.token, &format!("please @conductor board move {card} ready --note it is time"))
            .await;
    assert_eq!(moved, format!("`{card}` is in ready."));
    assert_eq!(column_of(&rig, &card).await, "ready");
    // Moved by whoever asked, and with what they said as it was moved.
    let how = rig.event("board.item.moved", |event| event.data["note"] == "it is time").await;
    assert!(how.data["by"].as_str().unwrap().starts_with("user:"), "{}", how.data);
    assert!(asked(&rig, &rig.token, "@conductor status").await.contains("is running. "));
    let alive = asked(&rig, &rig.token, "@conductor instances").await;
    assert!(alive.contains("std/board as ") && alive.contains(": active"), "{alive}");
    assert_eq!(asked(&rig, &rig.token, "@conductor agent list").await, "There are no agents.");
    let help = asked(&rig, &rig.token, "@conductor help").await;
    assert!(help.contains("`board move <card> <column> [--note TEXT]`"), "{help}");
    // It is in the channel from the first thing it says there.
    let floor = rig.read("/api/channels/floor").await;
    assert!(floor["members"].as_array().unwrap().iter().any(|member| member["actor"] == "bot:conductor"));
}

#[tokio::test]
async fn what_the_bot_is_asked_is_done_as_whoever_asked_and_a_key_that_holds_less_is_refused_by_name() {
    let rig = Rig::with("api-commanded", |config, _| {
        config.adapters.push(("helps".into(), crate::standin::argv(&["swallow", "say", "I am here."])));
    })
    .await;
    let rig = Rig::owned(rig).await;
    let card = shop(&rig).await;
    rig.send("/api/agents", json!({ "name": "helper", "adapter": "helps", "project": "shop" })).await;
    // It may speak in a channel, and that is all it may do.
    let speaks = key(&rig, &["channels"]).await;
    for (words, lacks) in [
        ("@conductor board".to_string(), "board:read"),
        (format!("@conductor board move {card} done"), "board:write"),
        ("@conductor instances".to_string(), "blueprints:read"),
        ("@conductor status".to_string(), "blueprints:read"),
        ("@conductor agent list".to_string(), "agents:read"),
        ("@conductor agent stop helper".to_string(), "agents:write"),
        ("@conductor agent stop helper --idle".to_string(), "agents:write"),
        ("@conductor emit shop.deployed version=3".to_string(), "emit:shop.deployed"),
    ] {
        let answer = asked(&rig, &speaks, &words).await;
        assert!(answer.starts_with("That was refused: "), "{words}: {answer}");
        assert!(answer.contains(&format!("(needs `{lacks}`)")), "{words}: {answer}");
    }
    // And nothing came of any of it.
    assert_eq!(column_of(&rig, &card).await, "draft");
    let agents = rig.read("/api/agents").await;
    assert_eq!(agents["agents"][0]["active"], true, "{agents}");
    assert_eq!(rig.count_of("shop.deployed").await, 0);
    // A key that holds the board moves a card, and still stops no agent.
    let boards = key(&rig, &["channels", "board"]).await;
    assert_eq!(
        asked(&rig, &boards, &format!("@conductor board move {card} done")).await,
        format!("`{card}` is in done.")
    );
    assert_eq!(column_of(&rig, &card).await, "done");
    // Moved for a key, it is the key that moved it, as when the key asks
    // the board itself: whatever asks what the mover holds is answered for
    // the key, and not for whoever made it.
    let moved = rig.event("board.item.moved", |event| event.data["to"] == "done").await;
    let by = moved.source.actor.clone().unwrap_or_default();
    assert!(by.starts_with("key:") && moved.data["by"] == by.as_str(), "{:?} {:?}", moved.source, moved.data);
    assert!(asked(&rig, &boards, "@conductor agent stop helper").await.contains("(needs `agents:write`)"));
    // Whoever holds it is let: the owner.
    assert_eq!(asked(&rig, &rig.token, "@conductor agent stop helper").await, "`helper` is stopped.");
    assert_eq!(rig.read("/api/agents").await["agents"][0]["active"], false);
}

#[tokio::test]
async fn an_agent_that_asks_the_bot_gets_what_an_agent_holds_and_no_more() {
    let rig = Rig::with("api-commanded", |config, _| {
        // One that, whatever it is asked, asks the bot to stop the other
        // and to show the board.
        let asks = ["swallow", "say", "@conductor agent stop helper"];
        config.adapters.push(("asks".into(), crate::standin::argv(&asks)));
    })
    .await;
    let rig = Rig::owned(rig).await;
    shop(&rig).await;
    for (name, adapter) in [("helper", "worker"), ("asker", "asks")] {
        let agent = json!({ "name": name, "adapter": adapter, "project": "shop", "channel": "floor" });
        rig.send("/api/agents", agent).await;
    }
    rig.send("/api/channels/floor/messages", json!({ "body": "@asker what now?" })).await;
    let its = rig.event("channel.message", |event| event.data["author"] == "agent:asker").await;
    let answer = answer_to(&rig, its.data["id"].as_str().unwrap()).await;
    assert!(answer.contains("(needs `agents:write`)"), "{answer}");
    let agents = rig.read("/api/agents").await;
    let helper = agents["agents"].as_array().unwrap().iter().find(|agent| agent["name"] == "helper").unwrap();
    assert_eq!(helper["active"], true, "{agents}");
}

#[tokio::test]
async fn what_is_not_a_command_is_answered_once_with_what_is_and_no_agent_takes_it_for_itself() {
    let rig = Rig::new().await;
    shop(&rig).await;
    // The only agent in the channel: what a person says there with nobody
    // named is said to it.
    let agent = json!({ "name": "helper", "adapter": "worker", "project": "shop", "channel": "floor" });
    rig.send("/api/agents", agent).await;
    let answer = asked(&rig, &rig.token, "@conductor how is the board looking today?").await;
    assert!(answer.starts_with("I did not take that as a command."), "{answer}");
    assert!(answer.contains("`board <project>`") && answer.contains("ask an agent"), "{answer}");
    // Said to the agent next, which answers what it is asked in the order
    // it was asked: by its answer to this, it has answered whatever came
    // before.
    let next =
        rig.send("/api/channels/floor/messages", json!({ "body": "and what do you make of it?" })).await;
    rig.event("channel.message", move |event| {
        event.data["author"] == "agent:helper" && event.data["reply_to"] == next["id"]
    })
    .await;
    let said = said_on_the_floor(&rig).await;
    let by = |author: &str| said.iter().filter(|message| message["author"] == author).count();
    assert_eq!((by("bot:conductor"), by("agent:helper")), (1, 1), "{said:#?}");
}

#[tokio::test]
async fn what_is_written_to_the_bot_is_words_and_never_part_of_an_address_or_a_command_line() {
    let rig = Rig::new().await;
    let card = shop(&rig).await;
    rig.send("/api/agents", json!({ "name": "helper", "project": "shop" })).await;
    for (words, refusal) in [
        ("@conductor board move ../../agents/helper done", "is not a card"),
        ("@conductor board move shop%2F1 done", "is not a card"),
        ("@conductor board ../agents", "is not a project"),
        ("@conductor agent stop helper?idle=true", "is not an agent"),
        ("@conductor agent stop \"helper; rm -rf $HOME\"", "is not an agent"),
        ("@conductor emit ../board/shop/items title=x", "is not a topic"),
        ("@conductor board move", "it is written `board move <card> <column> [--note TEXT]`"),
        ("@conductor board history ../records/x", "is not a card"),
        ("@conductor board give shop-1 nobody --at once", "`--at` is not something `board give` takes"),
    ] {
        let answer = asked(&rig, &rig.token, words).await;
        assert!(answer.contains(refusal), "{words}: {answer}");
    }
    // A column is whatever the board has, and one it has not got is the
    // board's to refuse.
    let nowhere = asked(&rig, &rig.token, &format!("@conductor board move {card} $(reboot)")).await;
    assert!(nowhere.starts_with("That was refused: "), "{nowhere}");
    assert_eq!(column_of(&rig, &card).await, "draft");
    assert_eq!(rig.read("/api/agents").await["agents"][0]["active"], true);
}

#[tokio::test]
async fn an_event_said_through_the_bot_is_said_by_whoever_asked_and_never_one_of_the_systems_own() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let said = asked(&rig, &rig.token, "@conductor emit shop.deployed version=3 by=\"the owner\"").await;
    assert!(said.starts_with("Said `shop.deployed` ("), "{said}");
    let event = rig.event("shop.deployed", |_| true).await;
    // Said in a project's channel, it is that project's, and says so.
    assert_eq!(event.data, json!({ "version": 3, "by": "the owner", "project": "shop" }));
    assert!(
        event.source.actor.as_deref().is_some_and(|actor| actor.starts_with("user:")),
        "{:?}",
        event.source
    );
    let own = asked(&rig, &rig.token, "@conductor emit instance.failed error=none").await;
    assert!(own.starts_with("That was refused: "), "{own}");
    assert_eq!(rig.count_of("instance.failed").await, 0);
}

#[tokio::test]
async fn what_the_bot_takes_is_listed_beside_its_name_and_is_what_it_says_of_itself() {
    let rig = Rig::new().await;
    let card = shop(&rig).await;
    // Whoever may read channels may read what can be asked in one.
    let member = key(&rig, &["channels"]).await;
    let (status, about) = rig.call(Method::GET, "/api/system/speaker", Some(&member), None).await;
    assert_eq!(status, StatusCode::OK, "{about}");
    assert_eq!(about["name"], "conductor");
    let listed: Vec<(&str, &str)> = about["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| (one["written"].as_str().unwrap(), one["does"].as_str().unwrap()))
        .collect();
    assert!(
        listed.iter().any(|(written, _)| *written == "board move <card> <column> [--note TEXT]"),
        "{about}"
    );
    assert!(listed.iter().all(|(written, does)| !written.is_empty() && !does.is_empty()), "{about}");
    // One list: every one of them is in what the bot answers when asked what it takes, and nothing else is.
    let help = asked(&rig, &rig.token, "@conductor help").await;
    for (written, does) in &listed {
        assert!(help.contains(&format!("- `{written}`: {does}")), "`{written}` in {help}");
    }
    assert_eq!(help.matches("\n- `").count(), listed.len(), "{help}");
    // And each is taken: written with something in each place it leaves open, and
    // without what it says may be left out, none is answered as no command.
    for (written, _) in &listed {
        let words: Vec<String> = written
            .split(' ')
            .take_while(|word| !word.starts_with('['))
            .map(|word| match word {
                "<card>" => card.clone(),
                "<project>" => "shop".to_string(),
                "<title...>" => "A till".to_string(),
                "<agent>|<person>|nobody" => "nobody".to_string(),
                "<field=value>..." => "body=Sold".to_string(),
                "<column>" => "ready".to_string(),
                "<name>" => "nobody".to_string(),
                "<topic>" => "shop.said".to_string(),
                // Nothing there by these: each is refused for what it names, and nothing is changed.
                "<record>" | "<id>" | "<channel>" | "<line>|--all" | "<line>" | "<file|name>" => {
                    "nothing".into()
                }
                "<collection>" | "<env>" | "<branch>" | "<project>|--none" => "nothing".into(),
                "<message...>" | "<words...>" => "hello".to_string(),
                word => word.to_string(),
            })
            .collect();
        let answer = asked(&rig, &rig.token, &format!("@conductor {}", words.join(" "))).await;
        assert!(!answer.starts_with("I did not take that as a command"), "`{written}`: {answer}");
    }
}

/// Says `words` in `channel` as the owner: what the bot answered to it.
async fn asked_in(rig: &Rig, channel: &str, words: &str) -> String {
    let said = rig.send(&format!("/api/channels/{channel}/messages"), json!({ "body": words })).await;
    answer_to(rig, said["id"].as_str().unwrap()).await
}

#[tokio::test]
async fn in_a_projects_channel_the_bot_is_that_projects_and_does_nothing_to_anothers() {
    let rig = Rig::with("api-commanded", |config, _| {
        config.adapters.push(("helps".into(), crate::standin::argv(&["swallow", "say", "done"])));
    })
    .await;
    let rig = Rig::owned(rig).await;
    // Two projects, each with a board, a card and an agent; the floor is the shop's channel.
    let card = shop(&rig).await;
    rig.send("/api/projects", json!({ "name": "Depot" })).await;
    rig.board("depot").await;
    let theirs = rig.send("/api/board/depot/items", json!({ "title": "Count the pallets" })).await;
    let theirs = theirs["id"].as_str().unwrap().to_string();
    rig.send("/api/agents", json!({ "name": "shop-helper", "adapter": "helps", "project": "shop" })).await;
    rig.send("/api/agents", json!({ "name": "depot-helper", "adapter": "helps", "project": "depot" })).await;
    let active = |agents: &Value, name: &str| {
        let all = agents["agents"].as_array().unwrap();
        all.iter().find(|one| one["name"] == name).unwrap()["active"] == true
    };

    // What is running and who is up are the shop's, though the owner may read all of it.
    let alive_for = |found: Value| match found["instances"].as_array().unwrap().len() {
        1 => "1 instance is alive".to_string(),
        many => format!("{many} instances are alive"),
    };
    let ours = alive_for(rig.read("/api/instances?live=true&project=shop").await);
    let everyones = alive_for(rig.read("/api/instances?live=true").await);
    assert_ne!(ours, everyones);
    let status = asked(&rig, &rig.token, "@conductor status").await;
    assert!(status.contains(&format!("For `shop`: {ours}")) && status.contains("1 agent is up"), "{status}");
    let alive = asked(&rig, &rig.token, "@conductor instances").await;
    let shops = rig.read("/api/instances?live=true&project=shop").await;
    let shops: Vec<&str> =
        shops["instances"].as_array().unwrap().iter().map(|one| one["id"].as_str().unwrap()).collect();
    assert_eq!(alive.lines().count(), shops.len(), "{alive}");
    assert!(shops.iter().all(|id| alive.contains(&format!("`{id}`"))), "{alive}");
    let agents = asked(&rig, &rig.token, "@conductor agent list").await;
    assert!(agents.contains("`shop-helper`") && !agents.contains("depot-helper"), "{agents}");

    // Another project's board is not read here, its card not moved, its agent not stopped, and no event said for it.
    let board = asked(&rig, &rig.token, "@conductor board depot").await;
    assert!(board.starts_with("This is the channel of `shop`, and `depot` is another project."), "{board}");
    assert!(!board.contains("Count the pallets") && !board.contains("I take these"), "{board}");
    let moved = asked(&rig, &rig.token, &format!("@conductor board move {theirs} ready")).await;
    assert!(moved.contains(&format!("`{theirs}` is not a card of `shop`")), "{moved}");
    let stopped = asked(&rig, &rig.token, "@conductor agent stop depot-helper").await;
    assert!(stopped.contains("`depot-helper` is not one of `shop`'s agents"), "{stopped}");
    let said = asked(&rig, &rig.token, "@conductor emit shop.deployed project=depot").await;
    assert!(
        said.starts_with("This is the channel of `shop`, and an event for `depot` is another project's."),
        "{said}"
    );
    let depot = rig.read("/api/board/depot").await;
    assert_eq!(depot["items"][0]["status"], "draft", "{depot}");
    assert!(active(&rig.read("/api/agents").await, "depot-helper"));
    assert_eq!(rig.count_of("shop.deployed").await, 0);

    // Its own are done as they were, and it says whose it is there when asked what it takes.
    assert_eq!(
        asked(&rig, &rig.token, &format!("@conductor board move {card} ready")).await,
        format!("`{card}` is in ready.")
    );
    assert_eq!(
        asked(&rig, &rig.token, "@conductor agent stop shop-helper").await,
        "`shop-helper` is stopped."
    );
    let help = asked(&rig, &rig.token, "@conductor help").await;
    assert!(help.starts_with("This is the channel of `shop`: here I am asked about that project"), "{help}");

    // In a channel that is no project's it is the factory's, and does what whoever asks may do anywhere.
    rig.send("/api/channels", json!({ "name": "lobby" })).await;
    let anywhere = asked_in(&rig, "lobby", "@conductor board depot").await;
    assert!(anywhere.contains(&format!("`{theirs}` Count the pallets")), "{anywhere}");
    let everything = asked_in(&rig, "lobby", "@conductor status").await;
    assert!(everything.contains(&everyones) && !everything.contains("For `"), "{everything}");
    assert_eq!(
        asked_in(&rig, "lobby", "@conductor agent stop depot-helper").await,
        "`depot-helper` is stopped."
    );
    assert!(!asked_in(&rig, "lobby", "@conductor help").await.contains("This is the channel of"));
}

#[tokio::test]
async fn a_bot_narrowed_does_what_it_is_asked_as_whoever_asked_held_to_its_ceiling_and_says_its_limit() {
    let rig = Rig::new().await;
    let card = shop(&rig).await;
    // Nothing said, it may ask everything its commands reach, and says so.
    let about = rig.read("/api/system/speaker").await;
    assert_eq!(about["holds"], Value::Null, "{about}");
    let reaches: Vec<&str> = about["reaches"].as_array().unwrap().iter().filter_map(Value::as_str).collect();
    assert!(reaches.contains(&"board:write") && reaches.contains(&"agents:write"), "{about}");

    // Held to reading the board, it reads it for the owner and moves nothing,
    // saying that its ceiling is what stands in the way and not the asker.
    let narrowed = json!({ "holds": ["board:read"] });
    let (status, held) =
        rig.call(Method::PUT, "/api/system/speaker/holds", Some(&rig.token), Some(narrowed.clone())).await;
    assert_eq!((status, &held["holds"]), (StatusCode::OK, &json!(["board:read"])), "{held}");
    let said = rig.event("channel.speaker_held", |event| event.data["holds"] == json!(["board:read"])).await;
    assert_eq!(said.data["name"], "conductor");
    assert_eq!(rig.read("/api/system/speaker").await["holds"], json!(["board:read"]));
    assert!(asked(&rig, &rig.token, "@conductor board").await.contains("Add a cart"));
    let refused = asked(&rig, &rig.token, &format!("@conductor board move {card} ready")).await;
    assert!(
        refused.starts_with("I may not do that: what `conductor` may ask is held to `board:read`, and that needs `board:write`, which you hold."),
        "{refused}"
    );
    assert_eq!(column_of(&rig, &card).await, "draft", "nothing was moved");
    let stop = asked(&rig, &rig.token, "@conductor agent list").await;
    assert!(stop.contains("`agents:read`"), "{stop}");

    // A key that holds less than the ceiling is refused for what it lacks,
    // in the route's own words: the ceiling never gives anybody more.
    let reader = key(&rig, &["channels", "agents:read"]).await;
    let less = asked(&rig, &reader, "@conductor board").await;
    assert!(less.starts_with("That was refused") && less.contains("`board:read`"), "{less}");

    // Nobody narrows it but whoever manages the factory, and they give it
    // only what they hold.
    let member = key(&rig, &["channels", "board"]).await;
    let (status, refused) = rig
        .call(Method::PUT, "/api/system/speaker/holds", Some(&member), Some(json!({ "holds": null })))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let admin = key(&rig, &["admin", "board"]).await;
    let more = json!({ "holds": ["board", "agents:write"] });
    let (status, refused) =
        rig.call(Method::PUT, "/api/system/speaker/holds", Some(&admin), Some(more)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(refused.to_string().contains("does not hold `agents:write`"), "{refused}");
    let (status, nonsense) = rig
        .call(Method::PUT, "/api/system/speaker/holds", Some(&rig.token), Some(json!({ "holds": ["dance"] })))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{nonsense}");

    // Given the usual again, it moves the card for whoever asks.
    rig.call(Method::PUT, "/api/system/speaker/holds", Some(&rig.token), Some(json!({ "holds": null })))
        .await;
    assert_eq!(
        asked(&rig, &rig.token, &format!("@conductor board move {card} ready")).await,
        format!("`{card}` is in ready.")
    );
}

#[tokio::test]
async fn a_cards_commands_are_the_command_lines_each_done_as_whoever_asked_and_said_briefly() {
    let rig = Rig::new().await;
    let card = shop(&rig).await;
    // Added in the shop's channel, its project left out; and the same words with `mw` before them.
    let added = asked(
        &rig,
        &rig.token,
        "@conductor board add A till --body Where money is kept --label front --label soon",
    )
    .await;
    assert_eq!(added, "`shop-2` is added to draft: A till");
    let till = rig.read("/api/records/shop-2").await;
    assert_eq!(
        (&till["data"]["body"], &till["data"]["labels"]),
        (&json!("Where money is kept"), &json!(["front", "soon"])),
        "{till}"
    );
    let cli = format!("@conductor {} board add shop A shelf --status ready", brand::CLI);
    assert_eq!(asked(&rig, &rig.token, &cli).await, "`shop-3` is added to ready: A shelf");
    // What it holds set, and given to an agent and to nobody.
    let set =
        asked(&rig, &rig.token, &format!("@conductor board set {card} title=\"Add a basket\" labels=a,b"))
            .await;
    assert_eq!(set, format!("`{card}` is changed."));
    let changed = rig.read(&format!("/api/records/{card}")).await;
    assert_eq!(
        (&changed["data"]["title"], &changed["data"]["labels"]),
        (&json!("Add a basket"), &json!(["a", "b"])),
        "{changed}"
    );
    let moved = asked(&rig, &rig.token, &format!("@conductor board set {card} status=done")).await;
    assert!(moved.contains(&format!("`board move {card} <column>`")), "{moved}");
    rig.send("/api/agents", json!({ "name": "helper", "adapter": "worker", "project": "shop" })).await;
    assert_eq!(
        asked(&rig, &rig.token, &format!("@conductor board give {card} helper")).await,
        format!("`{card}` is given to agent:helper.")
    );
    assert_eq!(
        asked(&rig, &rig.token, &format!("@conductor board give {card} nobody")).await,
        format!("`{card}` is given to nobody.")
    );
    // A person by their handle is that person, as the route says; a word that is nobody's is refused.
    let owner = rig.read("/api/accounts").await["accounts"][0].clone();
    let handle = people_handle(owner["name"].as_str().unwrap());
    let given = asked(&rig, &rig.token, &format!("@conductor board give {card} {handle}")).await;
    assert_eq!(given, format!("`{card}` is given to user:{}.", owner["id"].as_str().unwrap()));
    let nobody = asked(&rig, &rig.token, &format!("@conductor board give {card} nemo")).await;
    assert!(nobody.contains("`nemo` is no agent's name and no person's handle or email here"), "{nobody}");
    assert!(nobody.contains("helper"), "who it could be is listed: {nobody}");
    // Its history, each line as the card's history says it; and what it cost, which is nothing yet.
    let history = asked(&rig, &rig.token, &format!("@conductor board history {card}")).await;
    assert!(history.starts_with("- Ada added it to draft"), "{history}");
    assert_eq!(
        asked(&rig, &rig.token, &format!("@conductor board cost {card}")).await,
        format!("No agent has taken a turn on `{card}`.")
    );
    // Every change was the asker's.
    let by = rig.event("board.item.created", |event| event.data["id"] == "shop-3").await;
    assert!(by.data["by"].as_str().is_some_and(|by| by.starts_with("user:")), "{:?}", by.data);

    // A key that reads the board and changes nothing is refused each change, by the route's own words, and reads.
    let reader = key(&rig, &["channels", "board:read", "records:read"]).await;
    for words in [
        "@conductor board add A mop".to_string(),
        format!("@conductor board set {card} title=Mop"),
        format!("@conductor board give {card} helper"),
    ] {
        let refused = asked(&rig, &reader, &words).await;
        assert!(refused.contains("(needs `board:write`)"), "{words}: {refused}");
    }
    assert!(
        asked(&rig, &reader, &format!("@conductor board history {card}")).await.contains("added it to draft")
    );
    assert_eq!(rig.read(&format!("/api/records/{card}")).await["data"]["title"], "Add a basket");

    // Another project's card is not read about in this one's channel, and its board is not added to.
    rig.send("/api/projects", json!({ "name": "Depot" })).await;
    rig.board("depot").await;
    rig.send("/api/board/depot/items", json!({ "title": "Count the pallets" })).await;
    for words in ["@conductor board history depot-1", "@conductor board cost depot-1"] {
        let answer = asked(&rig, &rig.token, words).await;
        assert!(answer.contains("This is the channel of `shop`"), "{words}: {answer}");
    }
    // What changes a card is held to the channel's project by the route, in its own words.
    let set = asked(&rig, &rig.token, "@conductor board set depot-1 title=x").await;
    assert!(set.contains("`depot-1` is not a card of `shop`"), "{set}");
    let elsewhere = asked(&rig, &rig.token, "@conductor board add depot A pallet").await;
    assert_eq!(
        elsewhere, "`shop-4` is added to draft: depot A pallet",
        "a word that is not this project is the title"
    );
}

#[tokio::test]
async fn a_card_an_agent_adds_through_the_bot_is_the_agents_doing() {
    let rig = Rig::with("api-commanded", |config, _| {
        // One that, whatever it is asked, asks the bot to add a card.
        let asks = ["swallow", "say", "@conductor board add The agent's card"];
        config.adapters.push(("asks".into(), crate::standin::argv(&asks)));
    })
    .await;
    let rig = Rig::owned(rig).await;
    shop(&rig).await;
    rig.send(
        "/api/agents",
        json!({ "name": "asker", "adapter": "asks", "project": "shop", "channel": "floor" }),
    )
    .await;
    rig.send("/api/channels/floor/messages", json!({ "body": "@asker add one" })).await;
    let added = rig.event("board.item.created", |event| event.data["title"] == "The agent's card").await;
    assert_eq!(added.data["by"], "agent:asker", "{:?}", added.data);
}

#[tokio::test]
async fn a_projects_records_are_listed_shown_and_changed_through_the_bot_and_none_kept_private_is_said() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let plan = "blueprint: plan\npermissions: [projects, records]\nsteps:\n  sprints:\n    do: collection.ensure\n    with: { name: sprints, fields: { title: { type: text, required: true }, size: { type: number } }, history: [record.*] }\n  notes:\n    do: collection.ensure\n    with: { name: notes, reads: \"private:notes\", fields: { title: { type: text } } }\n";
    let applied = rig.send("/api/apply", json!({ "source": plan, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let added = asked(&rig, &rig.token, "@conductor record add sprints title=November size=5").await;
    assert_eq!(added, "`shop-sprints-1` is added to sprints.");
    assert_eq!(
        asked(&rig, &rig.token, "@conductor record list sprints").await,
        "- `shop-sprints-1` November"
    );
    assert_eq!(
        asked(&rig, &rig.token, "@conductor record set shop-sprints-1 size=8").await,
        "`shop-sprints-1` is changed."
    );
    let shown = asked(&rig, &rig.token, "@conductor record show shop-sprints-1").await;
    assert!(shown.contains("- size: 8") && shown.contains("- title: November"), "{shown}");
    assert_eq!(
        asked(&rig, &rig.token, "@conductor record comment shop-sprints-1 Two weeks is short").await,
        "Said on `shop-sprints-1`."
    );
    let history = rig.read("/api/records/shop-sprints-1/history").await;
    assert!(history.to_string().contains("Two weeks is short"), "{history}");
    // A table kept private is not listed in a channel, whose members may not read it.
    rig.send("/api/collections/shop/notes/records", json!({ "fields": { "title": "Quiet" } })).await;
    for words in ["@conductor record list notes", "@conductor record show shop-notes-1"] {
        assert!(asked(&rig, &rig.token, words).await.contains("`notes` is kept private"), "{words}");
    }
    assert_eq!(
        asked(&rig, &rig.token, "@conductor record remove shop-sprints-1").await,
        "`shop-sprints-1` is moved to the Trash."
    );
    // A key that holds less is refused by the route's own words.
    let reader = key(&rig, &["channels", "records:read"]).await;
    assert!(
        asked(&rig, &reader, "@conductor record add sprints title=Mop")
            .await
            .contains("(needs `records:write`)")
    );
    // Another project's records are not this channel's.
    rig.send("/api/projects", json!({ "name": "Depot" })).await;
    let other = asked(&rig, &rig.token, "@conductor record list depot sprints").await;
    assert!(other.contains("This is the channel of `shop`"), "{other}");
}

#[tokio::test]
async fn a_channels_talk_the_inbox_and_an_agents_project_are_asked_of_the_bot_as_the_command_line_asks_them()
{
    let rig = Rig::new().await;
    shop(&rig).await;
    rig.send("/api/channels", json!({ "name": "lobby" })).await;
    // A channel read, said in and called something else.
    rig.send("/api/channels/lobby/messages", json!({ "body": "the quince is ripe" })).await;
    assert_eq!(
        asked_in(&rig, "lobby", "@conductor read lobby --limit 5").await.lines().next(),
        Some("- Ada: the quince is ripe")
    );
    assert_eq!(asked_in(&rig, "lobby", "@conductor post lobby the medlar too").await, "Said in #lobby.");
    let kept = rig.send("/api/channels", json!({ "name": "Pay review", "kept": true })).await;
    let kept = kept["id"].as_str().unwrap().to_string();
    let refused = asked_in(&rig, "lobby", &format!("@conductor read {kept}")).await;
    assert!(refused.contains("is kept to its members, and what is said there is read there"), "{refused}");
    assert_eq!(
        asked_in(&rig, "lobby", "@conductor channel rename lobby hall").await,
        "#lobby is called hall now."
    );
    // The floor is the shop's: a channel of no project is not read about there.
    let elsewhere = asked(&rig, &rig.token, "@conductor read hall").await;
    assert!(elsewhere.contains("This is the channel of `shop`"), "{elsewhere}");
    // The inbox is counted and not read out, and its lines are read and put away.
    let said = rig.send("/api/channels/hall/messages", json!({ "body": "a word for @ada" })).await;
    assert!(said["id"].is_string());
    let counted = asked_in(&rig, "hall", "@conductor inbox").await;
    assert!(counted.starts_with("You have ") && counted.contains("not said here"), "{counted}");
    assert_eq!(
        asked_in(&rig, "hall", "@conductor inbox read --all").await,
        "Everything in your inbox is read."
    );
    // An agent moved, from a channel that is no project's and not from a project's.
    rig.send("/api/agents", json!({ "name": "helper", "adapter": "worker", "project": "shop" })).await;
    rig.send("/api/projects", json!({ "name": "Depot" })).await;
    let moved = asked_in(&rig, "hall", "@conductor agent move helper depot").await;
    assert_eq!(moved, "`helper` works in `depot` from its next piece of work.");
    let none = asked_in(&rig, "hall", "@conductor agent move helper --none").await;
    assert_eq!(none, "`helper` works in no project from its next piece of work.");
    assert!(
        asked(&rig, &rig.token, "@conductor agent move helper shop")
            .await
            .contains("This is the channel of `shop`")
    );
    // What is running, applied and stopped; a sandbox test; environments, comparisons and search.
    let applied = asked(&rig, &rig.token, "@conductor apply std/docs").await;
    assert!(applied.starts_with("`std/docs` is applied: `"), "{applied}");
    let instance = applied.split('`').nth(3).unwrap().to_string();
    assert!(
        asked(&rig, &rig.token, &format!("@conductor instance {instance}"))
            .await
            .contains("runs std/docs as ")
    );
    assert_eq!(
        asked(&rig, &rig.token, &format!("@conductor stop {instance}")).await,
        format!("`{instance}` is stopped, and what it holds with it.")
    );
    assert!(asked(&rig, &rig.token, "@conductor apply ./mine.bp").await.contains("is a file"));
    let tried = asked(&rig, &rig.token, "@conductor sandbox std/board").await;
    assert!(tried.starts_with("A sandbox test of `std/board`, where nothing is real."), "{tried}");
    assert_eq!(
        asked(&rig, &rig.token, "@conductor project environment").await,
        "`shop` has no environments."
    );
    assert_eq!(asked(&rig, &rig.token, "@conductor compare").await, "Nothing is being compared.");
    assert!(asked(&rig, &rig.token, "@conductor compare show models").await.starts_with("That was refused"));
    assert!(
        asked(&rig, &rig.token, "@conductor search cart --kind message")
            .await
            .contains("not said in a channel")
    );
    let found = asked(&rig, &rig.token, "@conductor search cart").await;
    assert!(!found.contains("message"), "{found}");
    // A board's columns, read and set in the order given.
    assert!(
        asked(&rig, &rig.token, "@conductor board columns").await.starts_with("The board of `shop` has ")
    );
    let set = asked(&rig, &rig.token, "@conductor board columns draft ready doing review done").await;
    assert_eq!(set, "The board of `shop` has 5 columns: draft, ready, doing, review, done.");
    // In a project's channel its help says the project is left out.
    assert!(asked(&rig, &rig.token, "@conductor help").await.contains("it is left out here: it is `shop`"));
    // A key that holds less is refused where it lacks, through the bot as directly.
    let reader = key(&rig, &["channels"]).await;
    assert!(asked(&rig, &reader, "@conductor apply std/docs").await.contains("(needs `blueprints:run`)"));
    assert!(asked(&rig, &reader, "@conductor search cart").await.contains("(needs `search`)"));
}

/// What a person is named by after an `@`, as the daemon has it.
fn people_handle(name: &str) -> String {
    studio::handle::handle(name)
}
