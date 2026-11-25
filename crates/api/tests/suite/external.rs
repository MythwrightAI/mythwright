//! An agent this factory does not start (`agents::external`), over HTTP
//! and with keys that hold less: who may invite and what they may give,
//! the door that takes an invitation and no session, the token acting
//! as the agent it names, what it holds changed under it, and the rest
//! of the factory going on around it: a channel kept to its members, a
//! fleet in its project, and a copy.

use std::time::Duration;

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::held::{key, said};

/// Invites `name` as whoever holds `token`: how that was answered.
async fn invite(rig: &Rig, token: &str, asked: Value) -> (StatusCode, Value) {
    rig.call(Method::POST, "/api/agent/invite", Some(token), Some(asked)).await
}

/// Hands an invitation over with no credential at all: how that was answered.
async fn join(rig: &Rig, invitation: &str) -> (StatusCode, Value) {
    rig.call(Method::POST, "/api/agent/join", None, Some(json!({ "invitation": invitation }))).await
}

/// `walls`, invited by the owner with what an agent usually holds, and
/// joined: its token.
async fn joined(rig: &Rig) -> String {
    let (status, invited) = invite(rig, &rig.token, json!({ "name": "walls", "project": "shop" })).await;
    assert_eq!(status, StatusCode::OK, "{invited}");
    let (status, joined) = join(rig, invited["invitation"].as_str().unwrap()).await;
    assert_eq!(status, StatusCode::OK, "{joined}");
    joined["token"].as_str().unwrap().to_string()
}

/// A `shop` project with a board.
async fn shop(rig: &Rig) {
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
}

/// What the list of agents says of `name`.
async fn listed(rig: &Rig, name: &str) -> Value {
    let all = rig.read("/api/agents").await;
    let found = all["agents"].as_array().unwrap().iter().find(|agent| agent["name"] == name);
    found.cloned().unwrap_or(Value::Null)
}

/// Asks, as the owner, whether `actor` holds `scope` now: what was answered.
async fn holds(rig: &Rig, actor: &str, scope: &str) -> Value {
    let source = format!(
        "blueprint: asks\nsteps:\n  ask:\n    do: actor.holds\n    with: {{ actor: \"{actor}\", scope: \"{scope}\" }}\noutputs:\n  holds: ${{steps.ask.out.holds}}\n  known: ${{steps.ask.out.known}}\n"
    );
    let asked = json!({ "source": source, "project": "shop", "wait": true });
    let answer = rig.send("/api/apply", asked).await;
    assert_eq!(answer["finished"]["status"], "ok", "{answer}");
    answer["finished"]["outputs"].clone()
}

#[tokio::test]
async fn somebody_who_may_not_add_agents_invites_nobody_and_whoever_may_gives_nothing_they_do_not_hold() {
    let rig = Rig::new().await;
    shop(&rig).await;
    // Nobody at all, somebody who may only see agents, and an agent.
    let (status, _) =
        rig.call(Method::POST, "/api/agent/invite", None, Some(json!({ "name": "walls" }))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let reads = key(&rig, &["agents:read", "board", "projects"]).await;
    let (status, refused) = invite(&rig, &reads, json!({ "name": "walls" })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("needs `agents:write`"), "{refused}");
    assert!(listed(&rig, "walls").await.is_null());
    // One who may add agents and holds little: what an agent usually
    // holds is more than they have, and what they have is theirs to give.
    let narrow = key(&rig, &["agents", "board:read"]).await;
    let (status, refused) = invite(&rig, &narrow, json!({ "name": "walls" })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).ends_with("cannot give an agent `projects`, not holding it"), "{refused}");
    let (status, refused) =
        invite(&rig, &narrow, json!({ "name": "walls", "scopes": ["board:write"] })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).ends_with("cannot give an agent `board:write`, not holding it"), "{refused}");
    assert!(listed(&rig, "walls").await.is_null());
    let (status, invited) = invite(&rig, &narrow, json!({ "name": "walls", "scopes": ["board:read"] })).await;
    assert_eq!(status, StatusCode::OK, "{invited}");
    assert_eq!(invited["agent"]["scopes"], json!(["board:read"]));
    assert_eq!(invited["agent"]["adapter"], "external");
    // It is listed, as external, invited and not yet joined.
    let is = listed(&rig, "walls").await;
    assert_eq!((&is["adapter"], &is["external"]["joined"]), (&json!("external"), &json!(false)), "{is}");
    assert_eq!(is["external"]["invited_until"], invited["expires"]);
    // Joined, the agent itself invites nobody, and changes what nobody holds: its own least of all.
    let (_, joined) = join(&rig, invited["invitation"].as_str().unwrap()).await;
    let walls = joined["token"].as_str().unwrap();
    let (status, refused) = invite(&rig, walls, json!({ "name": "gate" })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let more = json!({ "scopes": ["board"] });
    let (status, refused) =
        rig.call(Method::PATCH, "/api/agents/walls", Some(walls), Some(more.clone())).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    // Nor does the one who invited it give it more than they hold afterwards.
    let (status, refused) = rig.call(Method::PATCH, "/api/agents/walls", Some(&narrow), Some(more)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert_eq!(listed(&rig, "walls").await["scopes"], json!(["board:read"]));
    // No invitation and no token is in anything that was said.
    let events = rig.said(journal::EventQuery { limit: 1000, ..Default::default() }).await;
    let all: String = events.iter().map(|event| event.data.to_string()).collect();
    let secret = invited["invitation"].as_str().unwrap().trim_start_matches("enrol_");
    assert!(!all.contains(secret) && !all.contains(walls), "a secret was said in an event");
}

#[tokio::test]
async fn an_invitation_is_handed_over_with_no_session_and_a_wrong_a_used_and_an_expired_one_are_answered_alike()
 {
    let clock = kernel::ManualClock::new();
    let theirs = clock.clone();
    let rig = Rig::with("api-external", move |config, _| {
        config.clock = Some(std::sync::Arc::new(theirs));
    });
    let rig = Rig::owned(rig.await).await;
    shop(&rig).await;
    // How long it works is said as a length of time, and is no more than fourteen days.
    for (said_as, why) in
        [("soon", "is not how long an invitation works"), ("15d", "no more than fourteen days")]
    {
        let (status, refused) =
            invite(&rig, &rig.token, json!({ "name": "walls", "expires": said_as })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
        assert!(said(&refused).contains(why), "{refused}");
    }
    assert!(listed(&rig, "walls").await.is_null(), "an invitation that was refused made nobody");
    let asked = json!({ "name": "walls", "project": "shop", "expires": "2h" });
    let (status, invited) = invite(&rig, &rig.token, asked).await;
    assert_eq!(status, StatusCode::OK, "{invited}");
    let invitation = invited["invitation"].as_str().unwrap().to_string();

    // One this factory never made.
    let (status, wrong) = join(&rig, "enrol_nothing-anybody-made").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{wrong}");
    assert!(said(&wrong).starts_with("that invitation lets no agent in"), "{wrong}");
    // The one it made lets its bearer in, with no credential: it answers the token once.
    let (status, let_in) = join(&rig, &invitation).await;
    assert_eq!(status, StatusCode::OK, "{let_in}");
    assert_eq!((&let_in["agent"], &let_in["project"]), (&json!("walls"), &json!("shop")));
    let token = let_in["token"].as_str().unwrap().to_string();
    let (status, me) = rig.call(Method::GET, "/api/me", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK, "{me}");
    assert_eq!((&me["actor"], &me["principal"]["kind"]), (&json!("agent:walls"), &json!("agent")), "{me}");
    // Used, it is answered as the wrong one was.
    assert_eq!(join(&rig, &invitation).await, (StatusCode::FORBIDDEN, wrong.clone()));
    // And so is one that has stopped working.
    let (_, again) = invite(&rig, &rig.token, json!({ "name": "walls", "expires": "1h" })).await;
    clock.advance(Duration::from_secs(3600));
    assert_eq!(
        join(&rig, again["invitation"].as_str().unwrap()).await,
        (StatusCode::FORBIDDEN, wrong.clone())
    );
    // And one whose agent was stopped.
    let (_, again) = invite(&rig, &rig.token, json!({ "name": "walls" })).await;
    let (status, _) = rig.call(Method::DELETE, "/api/agents/walls", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(join(&rig, again["invitation"].as_str().unwrap()).await, (StatusCode::FORBIDDEN, wrong));
    // An invitation is needed to be handed over at all.
    let (status, _) = rig.call(Method::POST, "/api/agent/join", None, Some(json!({}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    // What was measured of the door counts each, by how it went.
    let measured = rig.read("/api/metrics?name=agent.join").await;
    assert!(measured.to_string().contains("agent.join"), "{measured}");
}

#[tokio::test]
async fn what_an_external_agent_does_with_its_token_says_its_name_and_what_it_holds_is_held_to_at_its_next_request()
 {
    let rig = Rig::new().await;
    shop(&rig).await;
    let walls = joined(&rig).await;
    let as_walls = |method: Method, path: String, body: Option<Value>| {
        let (rig, walls) = (&rig, walls.clone());
        async move { rig.call(method, &path, Some(&walls), body).await }
    };
    let card = rig.send("/api/board/shop/items", json!({ "title": "Hang the gate" })).await;
    let id = card["id"].as_str().unwrap().to_string();
    let item = format!("/api/board/items/{id}");

    // It gives the card to itself, moves it and says something on it.
    let (status, given) =
        as_walls(Method::PATCH, item.clone(), Some(json!({ "assignee": "agent:walls" }))).await;
    assert_eq!(status, StatusCode::OK, "{given}");
    let (status, moved) = as_walls(Method::PATCH, item.clone(), Some(json!({ "status": "doing" }))).await;
    assert_eq!(status, StatusCode::OK, "{moved}");
    let said_on = format!("/api/records/{id}/comments");
    let (status, commented) =
        as_walls(Method::POST, said_on, Some(json!({ "body": "The hinges are on." }))).await;
    assert_eq!(status, StatusCode::OK, "{commented}");
    // Each says the agent did it: who an event is from, and who it says did it.
    for topic in ["board.item.updated", "board.item.moved", "record.commented"] {
        let wanted = id.clone();
        let event = rig
            .event(topic, move |event| {
                event.data["id"] == wanted.as_str() || event.data["record"] == wanted.as_str()
            })
            .await;
        assert_eq!(event.source.actor.as_deref(), Some("agent:walls"), "{topic}: {event:?}");
        assert_eq!(event.data["by"], "agent:walls", "{topic}: {event:?}");
    }
    // And so does the card: who it is given to, and its history.
    let board = rig.read("/api/board/shop").await;
    let is = board["items"].as_array().unwrap().iter().find(|card| card["id"] == id.as_str()).unwrap();
    assert_eq!((&is["status"], &is["assignee"]), (&json!("doing"), &json!("agent:walls")), "{is}");
    let history = crate::lanes::history(&rig, &id, "walls said: The hinges are on.").await;
    assert!(history.contains(&"walls moved it from draft to doing".to_string()), "{history:?}");
    // Anybody who may change a card gives it one, too.
    let other = rig.send("/api/board/shop/items", json!({ "title": "Oil the latch" })).await;
    let other = format!("/api/board/items/{}", other["id"].as_str().unwrap());
    let give = Some(json!({ "assignee": "agent:walls" }));
    let (status, given) = rig.call(Method::PATCH, &other, Some(&rig.token), give).await;
    assert_eq!((status, &given["assignee"]), (StatusCode::OK, &json!("agent:walls")), "{given}");

    // What an event's actor holds now is the agent's, as it stands.
    let held = |holds: bool, known: bool| json!({ "holds": holds, "known": known });
    assert_eq!(holds(&rig, "agent:walls", "board:write").await, held(true, true));
    assert_eq!(holds(&rig, "agent:walls", "blueprints:run").await, held(false, true));
    // Changed where any agent's is, the token it has holds the new list
    // at its next request: it is not replaced, and it reads what it may.
    let less = json!({ "scopes": ["board:read", "projects:read"] });
    let (status, changed) = rig.call(Method::PATCH, "/api/agents/walls", Some(&rig.token), Some(less)).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    let (status, refused) = as_walls(Method::PATCH, item.clone(), Some(json!({ "status": "review" }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("needs `board:write`"), "{refused}");
    assert_eq!(as_walls(Method::GET, "/api/board/shop".into(), None).await.0, StatusCode::OK);
    assert_eq!(holds(&rig, "agent:walls", "board:write").await, held(false, true));
    let more = json!({ "scopes": ["board", "projects:read", "records"] });
    rig.call(Method::PATCH, "/api/agents/walls", Some(&rig.token), Some(more)).await;
    let (status, moved) = as_walls(Method::PATCH, item.clone(), Some(json!({ "status": "review" }))).await;
    assert_eq!(status, StatusCode::OK, "{moved}");

    // Nothing was ever started for it: it has no turns, and its key is no file of the factory's.
    assert_eq!(rig.read("/api/agents/walls/turns").await["turns"], json!([]));
    assert!(!rig.dir.join("data/slots/walls").exists());
    // It is not tried, and is not a way to be given a program.
    let (status, refused) = rig.post("/api/agents/walls/try", json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    let program = json!({ "name": "walls", "adapter": "worker" });
    let (status, refused) = rig.post("/api/agents", program).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).contains("so `adapter` says nothing of it"), "{refused}");

    // Stopped, its token is nobody's, and nobody holds anything by its name.
    let (status, _) = rig.call(Method::DELETE, "/api/agents/walls", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(as_walls(Method::GET, "/api/board/shop".into(), None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(holds(&rig, "agent:walls", "board:read").await, held(false, false));
    let is = listed(&rig, "walls").await;
    assert_eq!((&is["active"], &is["external"]["joined"]), (&json!(false), &json!(false)), "{is}");
}

#[tokio::test]
async fn an_external_agent_gives_up_the_token_it_asks_with_and_is_let_in_again_by_an_invitation() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let walls = joined(&rig).await;
    // A person does not leave, nor a key.
    let (status, refused) = rig.post("/api/agent/leave", json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).starts_with("only an external agent leaves"), "{refused}");
    let (status, _) = rig.call(Method::POST, "/api/agent/leave", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, left) = rig.call(Method::POST, "/api/agent/leave", Some(&walls), None).await;
    assert_eq!(status, StatusCode::OK, "{left}");
    assert_eq!(rig.call(Method::GET, "/api/me", Some(&walls), None).await.0, StatusCode::UNAUTHORIZED);
    let is = listed(&rig, "walls").await;
    assert_eq!((&is["active"], &is["external"]["joined"]), (&json!(true), &json!(false)), "{is}");
    let again = joined(&rig).await;
    assert_ne!(again, walls);
    assert_eq!(rig.call(Method::GET, "/api/me", Some(&again), None).await.0, StatusCode::OK);
    // Taken away by a person, it is gone, with what let it in.
    let (status, removed) = rig.post("/api/agents/walls/remove", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{removed}");
    assert!(listed(&rig, "walls").await.is_null());
    assert_eq!(rig.call(Method::GET, "/api/me", Some(&again), None).await.0, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn an_external_agent_reads_a_channel_kept_to_its_members_that_it_is_in_and_no_other() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let walls = joined(&rig).await;
    let kept = rig.send("/api/channels", json!({ "name": "Pay review", "kept": true })).await;
    let kept = kept["id"].as_str().unwrap().to_string();
    // Its token is answered of a kept channel it is not in as of one
    // that is not there.
    let read = |channel: String| {
        let (rig, walls) = (&rig, walls.clone());
        async move {
            let (status, refused) =
                rig.call(Method::GET, &format!("/api/channels/{channel}"), Some(&walls), None).await;
            (status, said(&refused).replace(&channel, "<it>"))
        }
    };
    assert_eq!(read(kept.clone()).await, read("ch_nonesuch".into()).await);
    // Brought in, as a person is added and with nothing asked of a wall
    // it has none of: it reads the channel with its own token, says
    // things there as itself, and is listed it. One is made with it in
    // it, too.
    let (status, brought) =
        rig.post(&format!("/api/channels/{kept}/agents"), json!({ "name": "walls" })).await;
    assert_eq!(status, StatusCode::OK, "{brought}");
    assert_eq!(read(kept.clone()).await.0, StatusCode::OK);
    let post = Some(json!({ "body": "The round is in." }));
    let (status, posted) =
        rig.call(Method::POST, &format!("/api/channels/{kept}/messages"), Some(&walls), post).await;
    assert_eq!((status, &posted["author"]), (StatusCode::OK, &json!("agent:walls")), "{posted}");
    let with = json!({ "name": "Hiring", "kept": true, "members": ["agent:walls"] });
    let (status, made) = rig.post("/api/channels", with).await;
    assert_eq!(status, StatusCode::OK, "{made}");
    let (_, listed) = rig.call(Method::GET, "/api/channels", Some(&walls), None).await;
    let kept_ones = listed["channels"].as_array().unwrap().iter().filter(|one| one["kept"] == true).count();
    assert_eq!(kept_ones, 2, "{listed}");
    // Taken out, it is shut to it again.
    let (status, left) = rig.post(&format!("/api/channels/{kept}/leave"), json!({ "agent": "walls" })).await;
    assert_eq!(status, StatusCode::OK, "{left}");
    assert_eq!(read(kept.clone()).await, read("ch_nonesuch".into()).await);
    // Into an open channel it is brought like any agent, reads it and says things in it as itself.
    let open = rig.send("/api/channels", json!({ "name": "yard" })).await;
    let open = open["id"].as_str().unwrap().to_string();
    let (status, brought) =
        rig.post(&format!("/api/channels/{open}/agents"), json!({ "name": "walls" })).await;
    assert_eq!(status, StatusCode::OK, "{brought}");
    let post = Some(json!({ "body": "The gate is hung." }));
    let (status, posted) =
        rig.call(Method::POST, &format!("/api/channels/{open}/messages"), Some(&walls), post).await;
    assert_eq!(status, StatusCode::OK, "{posted}");
    assert_eq!(posted["author"], "agent:walls");
    // What names it there starts nothing, and nobody answers for it.
    // An agent this factory starts is in the channel too: what is said
    // is heard in the order it was said, so once that one has answered
    // what was said to it afterwards, the message before was looked at.
    let helper = json!({ "name": "helper", "adapter": "worker" });
    let (status, brought) = rig.post(&format!("/api/channels/{open}/agents"), helper).await;
    assert_eq!(status, StatusCode::OK, "{brought}");
    let named =
        rig.send(&format!("/api/channels/{open}/messages"), json!({ "body": "@walls and the latch?" })).await;
    rig.send(&format!("/api/channels/{open}/messages"), json!({ "body": "@helper and you?" })).await;
    rig.event("agent.turn.finished", |event| event.data["slot"] == "helper").await;
    let messages = rig.read(&format!("/api/channels/{open}/messages")).await;
    let said_there = messages["messages"].as_array().unwrap();
    let answered = said_there.iter().any(|message| message["reply_to"] == named["id"]);
    assert!(!answered, "something answered what named it: {messages}");
    assert!(
        !said_there
            .iter()
            .any(|message| message["body"].as_str().unwrap_or_default().contains("was not asked"))
    );
    assert_eq!(rig.read("/api/agents/walls/turns").await["turns"], json!([]));
    assert_eq!(rig.read("/api/agents/helper/turns").await["turns"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn a_fleet_in_its_project_gives_an_external_agent_no_work_and_a_card_it_takes_is_its_own() {
    use crate::fleets::{add, card, given, shop as fleet_shop};
    use crate::lanes::{move_card, moved};
    use crate::rows::{rig_whose_worker, said_of};
    // A worker that leaves a mark each time it is asked anything.
    let (rig, at) = rig_whose_worker(&["swallow", "append", "$DIR/asked", "asked", "say", "I did it."]).await;
    fleet_shop(&rig, json!({ "reviewers": 0 })).await;
    let walls = joined(&rig).await;

    // A card it takes for itself, straight into work in hand: the stage
    // that works hears the move and asks nobody, as of a card a person
    // works themselves.
    let own = add(&rig, "Hang the gate", &[]).await;
    let item = format!("/api/board/items/{own}");
    let take = Some(json!({ "assignee": "agent:walls" }));
    assert_eq!(rig.call(Method::PATCH, &item, Some(&walls), take).await.0, StatusCode::OK);
    let from = rig.mark();
    let to_doing = Some(json!({ "status": "doing" }));
    assert_eq!(rig.call(Method::PATCH, &item, Some(&walls), to_doing).await.0, StatusCode::OK);
    let at_move = moved(&rig, &own, "doing", from).await;
    rig.event("run.finished", move |event| {
        event.seq > at_move && event.data["blueprint"] == "std/working" && event.data["flow"] == "on.back"
    })
    .await;
    assert_eq!(said_of(&rig, "pool.asked", "as", &own).await, 0, "a turn was asked for its card");
    assert!(!at.join("asked").exists(), "an agent was asked something");

    // A card given to it and made ready is nobody else's to work: its
    // round finds nobody to give it to, and leaves it in hand saying so.
    let given_to = add(&rig, "Oil the latch", &[]).await;
    let give = Some(json!({ "assignee": "agent:walls" }));
    let path = format!("/api/board/items/{given_to}");
    assert_eq!(rig.call(Method::PATCH, &path, Some(&rig.token), give).await.0, StatusCode::OK);
    move_card(&rig, &given_to, "ready", None).await;
    let says = format!(
        "{given_to}: it is given to walls, an external agent, which takes up its own cards and is asked nothing \
         here; it stays in doing"
    );
    let wanted = says.clone();
    rig.event("channel.message", move |event| event.data["body"] == wanted.as_str()).await;
    let is = card(&rig, &given_to).await;
    assert_eq!((&is["status"], &is["assignee"]), (&json!("doing"), &json!("agent:walls")), "{is}");
    assert!(!at.join("asked").exists(), "the fleet's worker was asked a card given to somebody else");

    // A card nobody is named on is the fleet's, and its worker does it.
    let theirs = add(&rig, "Sweep the yard", &[]).await;
    move_card(&rig, &theirs, "ready", None).await;
    moved(&rig, &theirs, "review", 0).await;
    assert_eq!(given(&rig, "shop-worker-1").await.len(), 1);

    // Through all of it nothing was asked of the external agent, and it holds no place in any pool.
    assert_eq!(rig.read("/api/agents/walls/turns").await["turns"], json!([]));
    assert_eq!(said_of(&rig, "agent.turn.queued", "slot", "walls").await, 0);
    let is = card(&rig, &own).await;
    assert_eq!((&is["status"], &is["assignee"]), (&json!("doing"), &json!("agent:walls")), "{is}");
    // And the fleet's own blueprints bring up nobody of its name.
    let source =
        "blueprint: brings\nsteps:\n  it: { do: agent.ensure, with: { name: walls, role: worker } }\n";
    let asked = json!({ "source": source, "project": "shop", "wait": true });
    let answer = rig.send("/api/apply", asked).await;
    assert_eq!(answer["finished"]["status"], "failed", "{answer}");
    let why = answer["finished"]["steps"][0]["error"].as_str().unwrap_or_default();
    assert!(why.starts_with("`walls` is an external agent, which this factory does not start"), "{why}");
}

#[tokio::test]
async fn a_copy_of_a_factory_keeps_an_external_agent_and_neither_its_token_nor_its_invitation() {
    use crate::snapshots::{Shared, taken};
    let shared = Shared::new();
    let prod = shared.factory("prod").await;
    shop(&prod).await;
    let token = joined(&prod).await;
    // And one that was invited and has yet to join.
    let (status, waiting) =
        invite(&prod, &prod.token, json!({ "name": "gate", "scopes": ["board:read"] })).await;
    assert_eq!(status, StatusCode::OK, "{waiting}");
    let to = shared.snapshot("prod-copy");
    taken(&prod, &to).await;

    let dev = shared.copy("dev2", &to).await;
    // Each is there as it was set up, and neither has joined this
    // factory or is invited to it.
    for (name, scopes) in [("walls", json!(agents::external::usual())), ("gate", json!(["board:read"]))] {
        let is = listed(&dev, name).await;
        assert_eq!(
            (&is["adapter"], &is["scopes"], &is["active"]),
            (&json!("external"), &scopes, &json!(true)),
            "{is}"
        );
        assert_eq!(is["external"], json!({ "joined": false, "invited_until": null }), "{is}");
    }
    // The token is the other factory's, and the invitation was made there.
    assert_eq!(dev.call(Method::GET, "/api/me", Some(&token), None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(join(&dev, waiting["invitation"].as_str().unwrap()).await.0, StatusCode::FORBIDDEN);
    // Both still work where they were made.
    assert_eq!(prod.call(Method::GET, "/api/me", Some(&token), None).await.0, StatusCode::OK);
    assert_eq!(join(&prod, waiting["invitation"].as_str().unwrap()).await.0, StatusCode::OK);
    // Invited to the new factory, it joins that one, with a token of its own.
    let theirs = joined(&dev).await;
    assert_ne!(theirs, token);
    assert_eq!(prod.call(Method::GET, "/api/me", Some(&theirs), None).await.0, StatusCode::UNAUTHORIZED);
}
