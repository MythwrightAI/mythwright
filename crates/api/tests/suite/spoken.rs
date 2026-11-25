//! What the factory's blueprints say in a channel: said by one bot, with
//! what said it kept on the message, and a refusal said once.

use reqwest::{Method, StatusCode};
use serde_json::Value;
use serde_json::json;

use crate::Rig;
use crate::fleets::{change, rig, row, shop};

/// Every message of the project's channel, oldest first.
async fn messages(rig: &Rig) -> Vec<Value> {
    rig.read("/api/channels/shop/messages?limit=200").await["messages"].as_array().unwrap().clone()
}

/// Changes a row of one of the fleet's tables and waits for nothing.
async fn write(rig: &Rig, table: &str, title: &str, fields: Value) {
    let id = row(rig, table, title).await;
    let body = Some(json!({ "fields": fields }));
    let (status, answer) =
        rig.call(Method::PATCH, &format!("/api/records/{id}"), Some(&rig.token), body).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
}

#[tokio::test]
async fn what_a_blueprint_says_is_said_by_the_factorys_bot_and_which_blueprint_it_was_is_kept_on_the_message()
{
    let (rig, _at) = rig().await;
    shop(&rig, json!({})).await;
    let said = messages(&rig).await;
    let up = said.iter().find(|message| message["body"].as_str().unwrap().contains("is up")).unwrap();
    assert_eq!(up["author"], "bot:conductor", "{up}");
    assert_eq!(up["author_name"], "conductor", "{up}");
    assert_eq!(up["said_by"]["blueprint"], "std/fleet", "{up}");
    assert_eq!(up["said_by"]["step"], "up", "{up}");
    for part in ["instance", "run"] {
        assert!(up["said_by"][part].as_str().is_some_and(|id| !id.is_empty()), "{up}");
    }
    // It is in the channel like anyone who speaks there, once.
    let channel = rig.read("/api/channels/shop").await;
    let bots: Vec<&Value> = channel["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|member| member["actor"] == "bot:conductor")
        .collect();
    assert_eq!(bots.len(), 1, "{channel}");
    assert_eq!(bots[0]["name"], "conductor", "{channel}");
}

#[tokio::test]
async fn a_row_given_a_cell_the_fleet_refuses_is_one_message_that_names_the_row_and_the_cell() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({})).await;
    // A list written as text in a cell, as somebody did: one path, and not a path.
    write(&rig, "agents", "worker", json!({ "reads": ["[\"~/elsewhere/tools\"]"] })).await;
    let refused = rig
        .event("channel.message", |event| {
            event.data["body"].as_str().unwrap_or_default().contains("cannot be among")
        })
        .await;
    let body = refused.data["body"].as_str().unwrap();
    assert!(body.contains("`worker`") && body.contains("`agents`") && body.contains("`reads`"), "{body}");
    assert!(body.contains("a folder or a file is written from the top"), "{body}");
    // The names of steps inside the blueprints are what it is opened for.
    assert!(!body.contains("step `"), "{body}");
    let detail = refused.data["said_by"]["detail"].as_str().unwrap_or_default();
    assert!(detail.contains("step `"), "{}", refused.data);
    // Put right, the fleet follows; and by then everything the refusal
    // set off has been said.
    change(&rig, "agents", "worker", json!({ "reads": ["~/elsewhere/tools"] })).await;
    let said = messages(&rig).await;
    let of_it: Vec<&str> = said
        .iter()
        .map(|message| message["body"].as_str().unwrap())
        .filter(|body| body.contains("cannot be among") || body.contains("failed"))
        .collect();
    assert_eq!(of_it.len(), 1, "{of_it:#?}");
    // A row added that the fleet cannot take is said the same way, as new.
    let tester = json!({ "title": "tester", "role": "tester", "reads": ["tools/of/ours"] });
    rig.send("/api/collections/shop/agents/records", json!({ "fields": tester })).await;
    let added = rig
        .event("channel.message", |event| {
            event.data["body"].as_str().unwrap_or_default().contains("the new row")
        })
        .await;
    let body = added.data["body"].as_str().unwrap();
    assert!(
        body.starts_with("The fleet did not take the new row `tester` of `agents`: `tools/of/ours` cannot"),
        "{body}"
    );
}

#[tokio::test]
async fn a_factory_from_before_speaks_as_the_bot_with_nothing_applied_again_and_what_was_said_keeps_its_name()
{
    let (mut rig, _at) = rig().await;
    shop(&rig, json!({})).await;
    // What a fleet said before there was a bot: kept as the system's, under
    // the blueprint's name.
    rig.node()
        .app
        .studio
        .channels
        .post_system(&crate::own(), "shop", "std/fleet", "The fleet is as it was.")
        .await
        .unwrap();
    rig.restart().await;
    // The fleet that was running says something, with nothing applied.
    let before = rig.mark();
    let reviewer = json!({ "title": "reviewer", "role": "reviewer" });
    rig.send("/api/collections/shop/agents/records", json!({ "fields": reviewer })).await;
    let now = rig
        .event("channel.message", move |event| {
            event.seq > before && event.data["body"].as_str().unwrap_or_default().starts_with("Two rows")
        })
        .await;
    assert_eq!(now.data["author"], "bot:conductor", "{}", now.data);
    assert_eq!(now.data["said_by"]["blueprint"], "std/arrangement", "{}", now.data);
    let said = messages(&rig).await;
    let old = said.iter().find(|message| message["body"] == "The fleet is as it was.").unwrap();
    assert_eq!((old["author"].as_str(), old["author_name"].as_str()), (Some("system"), Some("std/fleet")));
    assert!(old.get("said_by").is_none(), "{old}");
}

#[tokio::test]
async fn what_the_bot_is_called_is_the_owners_to_say_and_what_was_said_keeps_the_name_it_was_said_under() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({})).await;
    assert_eq!(rig.read("/api/system/speaker").await["name"], "conductor");
    // Not a member's to change, and not the name of an agent.
    let member =
        rig.send("/api/credentials", json!({ "name": "member", "scopes": ["channels", "agents"] })).await;
    let call = |token: String, name: &'static str| {
        let rig = &rig;
        async move {
            let body = Some(json!({ "name": name }));
            rig.call(Method::POST, "/api/system/speaker", Some(&token), body).await
        }
    };
    let (status, refused) = call(member["token"].as_str().unwrap().to_string(), "Foreman").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let (status, taken) = call(rig.token.clone(), "shop-worker-1").await;
    assert_eq!(status, StatusCode::CONFLICT, "{taken}");
    assert!(
        taken["error"]["message"].as_str().unwrap().contains("an agent is called `shop-worker-1`"),
        "{taken}"
    );
    let (status, called) = call(rig.token.clone(), "Foreman").await;
    assert_eq!((status, called["name"].as_str()), (StatusCode::OK, Some("foreman")), "{called}");
    // Where it was a member it is one under its new name.
    let channel = rig.read("/api/channels/shop").await;
    let bots: Vec<&str> = channel["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|member| member["actor"].as_str())
        .filter(|actor| actor.starts_with("bot:"))
        .collect();
    assert_eq!(bots, ["bot:foreman"], "{channel}");
    // It says what blueprints say from here on, and is what is spoken to.
    let asked = rig.send("/api/channels/shop/messages", json!({ "body": "@foreman help" })).await;
    let answer = rig.event("channel.message", move |event| event.data["reply_to"] == asked["id"]).await;
    assert_eq!(answer.data["author"], "bot:foreman", "{}", answer.data);
    assert!(answer.data["body"].as_str().unwrap().contains("`@foreman` is for the commands above"));
    change(&rig, "agents", "worker", json!({ "count": 2 })).await;
    let said = messages(&rig).await;
    let up = said.iter().find(|message| message["body"].as_str().unwrap().contains("is up")).unwrap();
    assert_eq!(up["author_name"], "conductor", "{up}");
}

#[tokio::test]
async fn a_new_agent_is_not_given_the_bots_name_and_one_that_had_it_before_goes_on_as_it_is() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    // By the route, which is what the command line asks.
    let (status, refused) = rig.post("/api/agents", json!({ "name": "Conductor", "project": "shop" })).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    let why = refused["error"]["message"].as_str().unwrap();
    assert!(
        why.starts_with("an agent cannot be called `conductor`: that is what this factory itself speaks as"),
        "{why}"
    );
    assert!(
        why.contains("Choose another name for the agent") && why.contains("channel speaker <name>"),
        "{why}"
    );
    // And by a blueprint that brings an agent up.
    let brings = "blueprint: brings\npermissions: [agents]\nsteps:\n  up:\n    do: agent.ensure\n    with: { name: conductor }\n";
    let applied = rig.send("/api/apply", json!({ "source": brings, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "failed", "{applied}");
    assert!(
        applied["finished"]["error"].as_str().unwrap().contains("an agent cannot be called `conductor`"),
        "{applied}"
    );
    assert_eq!(rig.read("/api/agents").await["agents"], json!([]));

    // A factory from before: an agent that had the name when the bot was
    // called something else, which it then came to be called.
    let (_, called) = rig
        .call(Method::POST, "/api/system/speaker", Some(&rig.token), Some(json!({ "name": "foreman" })))
        .await;
    assert_eq!(called["name"], "foreman", "{called}");
    rig.send("/api/agents", json!({ "name": "conductor", "project": "shop" })).await;
    let owner = kernel::Principal::system();
    rig.node().app.studio.channels.call_speaker("conductor", &owner).await.unwrap();
    // It is still there, and is changed like any other.
    let changed =
        rig.send("/api/agents", json!({ "name": "conductor", "project": "shop", "role": "worker" })).await;
    assert_eq!(
        (changed["name"].as_str(), changed["role"].as_str()),
        (Some("conductor"), Some("worker")),
        "{changed}"
    );
}

/// What a fleet said when it was first there: the one message of it.
async fn greeting(rig: &Rig) -> String {
    let said = messages(rig).await;
    let up: Vec<&str> = said
        .iter()
        .map(|message| message["body"].as_str().unwrap())
        .filter(|body| body.starts_with("The fleet for shop is up"))
        .collect();
    assert_eq!(up.len(), 1, "{up:#?}");
    up[0].to_string()
}

/// What every such message ends with, the factory's bot being called `bot`.
fn rest(bot: &str) -> String {
    format!(
        "\n\nWho is in the fleet and what they do with a card are two tables of the project, `agents` and \
         `stages`: change a row of either and the fleet follows.\n\nWrite `@{bot} help` here for what I take as \
         a command."
    )
}

#[tokio::test]
async fn a_fleet_that_is_first_there_says_who_is_in_it_what_to_do_next_and_how_its_bot_is_asked_by_its_name()
{
    // One who works, and nobody to talk a change over with.
    let (one, _at) = rig().await;
    shop(&one, json!({ "reviewers": 0 })).await;
    let cards = "\n\nAdd a card to the board and move it to ready: it is worked when its turn comes.";
    assert_eq!(
        greeting(&one).await,
        format!("The fleet for shop is up: one agent works cards.{cards}{}", rest("conductor"))
    );
    // Applied again with its steps run again, it is not said a second time.
    let params = crate::fleets::params(json!({ "reviewers": 0 }));
    let again =
        json!({ "blueprint": "std/fleet", "project": "shop", "params": params, "wait": true, "force": true });
    assert_eq!(one.send("/api/apply", again).await["finished"]["status"], "ok");
    greeting(&one).await;

    // Three who work; and the bot called something else before the fleet
    // is there: it is asked by the name it has.
    let (three, _at) = rig().await;
    three.send("/api/system/speaker", json!({ "name": "foreman" })).await;
    shop(&three, json!({ "workers": 3, "reviewers": 0 })).await;
    assert_eq!(
        greeting(&three).await,
        format!("The fleet for shop is up: 3 agents work cards.{cards}{}", rest("foreman"))
    );

    // Nobody who works: nobody is in it, and where that is changed is said.
    let (none, _at) = rig().await;
    shop(&none, json!({ "workers": 0, "reviewers": 0 })).await;
    assert_eq!(
        greeting(&none).await,
        format!("The fleet for shop is up, and nobody is in it yet.{}", rest("conductor"))
    );
}

#[tokio::test]
async fn how_many_cards_are_worked_at_once_is_said_when_it_changes_and_not_each_time_the_fleet_is_applied() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    // Applied again with its steps run again, as `--force` does and an
    // upgrade may: nothing about the fleet has changed.
    let params = crate::fleets::params(json!({ "reviewers": 0 }));
    let again =
        json!({ "blueprint": "std/fleet", "project": "shop", "params": params, "wait": true, "force": true });
    assert_eq!(rig.send("/api/apply", again).await["finished"]["status"], "ok");
    // Then a second worker: two cards at once from now, which is said.
    change(&rig, "agents", "worker", json!({ "count": 2 })).await;
    rig.event("channel.message", |event| {
        event.data["body"] == "Cards in ready are worked 2 at a time from now."
    })
    .await;
    // And that is the one time it was said: a pool does one thing at a
    // time, so what the applying again set it to say came before this.
    let said = messages(&rig).await;
    let of_it: Vec<&str> = said
        .iter()
        .map(|message| message["body"].as_str().unwrap())
        .filter(|body| body.contains("at a time from now"))
        .collect();
    assert_eq!(of_it, ["Cards in ready are worked 2 at a time from now."]);
}
