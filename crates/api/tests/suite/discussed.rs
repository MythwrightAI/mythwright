//! Agents of a channel talking something over with each other, in a
//! thread, and bringing a person in only to decide (`std/discuss`).
//!
//! The agents are stand-ins that answer by what they are shown: each is
//! asked with the thread, so what it says comes of what was said there.

use serde_json::{Value, json};

use crate::Rig;

/// A stand-in that, where what it read says `when`, says `then`, and
/// otherwise says `otherwise`; each a list of lines.
fn answers(when: &str, then: &[&str], otherwise: &[&str]) -> Vec<String> {
    let mut does: Vec<String> = vec!["read".into(), "where".into(), when.into(), "(".into()];
    for line in then {
        does.extend(["say".into(), line.to_string()]);
    }
    does.extend(["exit".into(), "0".into(), ")".into()]);
    for line in otherwise {
        does.extend(["say".into(), line.to_string()]);
    }
    does
}

/// A node with two agents' programs, each as `answers` makes it.
async fn rig(name: &str, programs: &[(&str, Vec<String>)]) -> Rig {
    let programs: Vec<(String, Vec<String>)> =
        programs.iter().map(|(name, does)| (name.to_string(), does.clone())).collect();
    let rig = Rig::with(name, move |config, _| {
        for (name, does) in &programs {
            let does: Vec<&str> = does.iter().map(String::as_str).collect();
            config.adapters.push((name.clone(), crate::standin::argv(&does)));
        }
    })
    .await;
    Rig::owned(rig).await
}

/// A channel with the agents in it, each on the program of its name.
async fn channel(rig: &Rig, name: &str, agents: &[&str]) -> String {
    let made = rig.send("/api/channels", json!({ "name": name })).await;
    for agent in agents {
        rig.send("/api/agents", json!({ "name": agent, "adapter": agent, "channel": name })).await;
    }
    made["id"].as_str().unwrap().to_string()
}

async fn offer(rig: &Rig, channel: &str, params: Value) -> String {
    let mut params = params;
    params["channel"] = json!(channel);
    let applied = json!({ "blueprint": "std/discuss", "key": format!("discuss-{channel}"), "params": params, "wait": true });
    rig.send("/api/apply", applied).await["instance"].as_str().unwrap().to_string()
}

async fn say(rig: &Rig, channel: &str, body: Value) -> Value {
    rig.send(&format!("/api/channels/{channel}/messages"), body).await
}

/// What was said under `root`, in order.
async fn thread(rig: &Rig, channel: &str, root: &str) -> Vec<Value> {
    let (_, read) = rig.get(&format!("/api/channels/{channel}/messages?thread={root}")).await;
    read["messages"].as_array().cloned().unwrap_or_default()
}

async fn ended(rig: &Rig, after: u64, came: &'static str) -> std::sync::Arc<kernel::Event> {
    rig.event("channel.discussion.ended", move |event| event.seq > after && event.data["came"] == came).await
}

/// Whoever is asked is asked once for each turn it is given.
async fn turns_of(rig: &Rig, slot: &str) -> usize {
    let (_, turns) = rig.get(&format!("/api/agents/{slot}/turns")).await;
    turns["turns"].as_array().map_or(0, Vec::len)
}

#[tokio::test]
async fn an_agent_naming_another_has_them_discuss_it_in_a_thread_until_they_agree_and_the_person_is_told() {
    let rig = rig(
        "api-discussed-agree",
        &[
            ("ann", answers("keep sessions", &["One table will do. @bo, is that right?"], &["Fine."])),
            ("bo", answers("discussing", &["Yes, one table is right.", "DECISION: agree"], &["Hello."])),
        ],
    )
    .await;
    let id = channel(&rig, "design", &["ann", "bo"]).await;
    offer(&rig, "design", json!({ "agents": ["ann: writes the plan", "bo: checks it"] })).await;
    let after = rig.mark();
    say(&rig, "design", json!({ "body": "@ann how should we keep sessions?" })).await;

    let begun = rig.event("channel.discussion.begun", move |event| event.seq > after).await;
    assert_eq!(begun.data["why"], "tagged", "{}", begun.data);
    assert_eq!(begun.data["agents"], json!(["bo", "ann"]));
    let root = begun.data["root"].as_str().unwrap().to_string();
    let over = ended(&rig, after, "agreed").await;
    assert_eq!(over.data["turns"], 1);

    let told = rig.event("inbox.told", move |event| event.seq > after).await;
    let said = thread(&rig, &id, &root).await;
    let words: Vec<&str> = said.iter().filter_map(|one| one["body"].as_str()).collect();
    assert!(words.iter().any(|one| one.starts_with("Yes, one table is right.")), "{words:?}");
    assert!(words.contains(&"bo and ann agreed, after 1 turn."), "{words:?}");
    // Bo was asked once, in the discussion, and not as somebody named.
    assert_eq!(turns_of(&rig, "bo").await, 1);
    let (_, line) = rig.get(&format!("/api/inbox/{}", told.data["line"].as_str().unwrap_or_default())).await;
    assert!(line.to_string().contains("agreed in a discussion in #design"), "{line}");
}

#[tokio::test]
async fn a_discussion_that_needs_a_person_waits_for_their_answer_in_the_thread_and_then_goes_on() {
    let rig = rig(
        "api-discussed-needs",
        &[
            (
                "ann",
                answers(
                    "plan the sessions",
                    &["Here is the plan: one table.", "DECISION: plan"],
                    &["Still one table."],
                ),
            ),
            (
                "bo",
                answers(
                    "Go with two",
                    &["Two it is.", "DECISION: agree"],
                    &["Which is cheaper?", "DECISION: needs"],
                ),
            ),
        ],
    )
    .await;
    let id = channel(&rig, "design", &["ann", "bo"]).await;
    offer(&rig, "design", json!({ "agents": ["ann", "bo"] })).await;
    let after = rig.mark();
    say(&rig, "design", json!({ "body": "@ann plan the sessions" })).await;

    let begun = rig.event("channel.discussion.begun", move |event| event.seq > after).await;
    assert_eq!(begun.data["why"], "plans", "{}", begun.data);
    let root = begun.data["root"].as_str().unwrap().to_string();
    let plan = thread(&rig, &id, &root).await;
    assert_eq!(plan[0]["mark"], "plan", "{plan:?}");
    let needs = ended(&rig, after, "needs").await;
    assert_eq!(needs.data["status"], "waiting");
    let told = rig.event("inbox.told", move |event| event.seq > after).await;
    let line = told.data["line"].as_str().unwrap_or_default().to_string();
    let (_, read) = rig.get(&format!("/api/inbox/{line}")).await;
    assert!(read.to_string().contains("need you to decide, in #design: Which is cheaper?"), "{read}");
    assert_eq!(read["parts"][0]["kind"], "message", "{read}");

    // Ada's answer in the thread lets it go on, from whoever asked her.
    let again = rig.mark();
    say(&rig, "design", json!({ "body": "Go with two.", "thread": root })).await;
    let resumed = rig.event("channel.discussion.resumed", move |event| event.seq > again).await;
    assert_eq!(resumed.data["first"], "bo");
    ended(&rig, again, "agreed").await;
}

#[tokio::test]
async fn agents_that_never_agree_stop_at_the_cap_and_the_person_is_told_so() {
    let rig = rig(
        "api-discussed-cap",
        &[
            ("ann", answers("sessions?", &["Tables. @bo?"], &["No, tables.", "DECISION: reply"])),
            ("bo", answers("never", &[], &["No, files.", "DECISION: reply"])),
        ],
    )
    .await;
    channel(&rig, "design", &["ann", "bo"]).await;
    offer(&rig, "design", json!({ "agents": ["ann", "bo"], "turns": 3 })).await;
    let after = rig.mark();
    say(&rig, "design", json!({ "body": "@ann sessions?" })).await;
    let capped = ended(&rig, after, "capped").await;
    assert_eq!(capped.data["turns"], 3);
    let told = rig.event("inbox.told", move |event| event.seq > after).await;
    let line = told.data["line"].as_str().unwrap_or_default().to_string();
    let (_, read) = rig.get(&format!("/api/inbox/{line}")).await;
    assert!(read.to_string().contains("did not agree in 3 turns"), "{read}");
}

#[tokio::test]
async fn with_a_lead_a_person_talks_to_the_lead_alone_which_consults_its_advisor_and_answers_with_what_was_agreed()
 {
    let rig = rig(
        "api-discussed-lead",
        &[
            (
                "helper",
                answers(
                    "has ended",
                    &["We will keep sessions in one table."],
                    &["Let me check with @architect: one table?"],
                ),
            ),
            ("architect", answers("discussing", &["One table is right.", "DECISION: agree"], &["Hello."])),
        ],
    )
    .await;
    let id = channel(&rig, "design", &["helper", "architect"]).await;
    offer(
        &rig,
        "design",
        json!({ "lead": "helper", "advisors": ["architect: technical advice, and confirming a plan"] }),
    )
    .await;
    let after = rig.mark();
    let asked = say(&rig, "design", json!({ "body": "How should we keep sessions?" })).await;
    let asked = asked["id"].as_str().unwrap().to_string();

    let begun = rig.event("channel.discussion.begun", move |event| event.seq > after).await;
    assert_eq!(begun.data["root"], asked.as_str(), "it hangs under what the person asked");
    assert_eq!(begun.data["agents"], json!(["architect", "helper"]));
    ended(&rig, after, "agreed").await;
    // The lead answers the person in the channel, beside what they asked.
    let answered = rig
        .event("channel.message", move |event| {
            event.seq > after
                && event.data["author"] == "agent:helper"
                && event.data["body"] == "We will keep sessions in one table."
        })
        .await;
    assert_eq!(answered.data["reply_to"], asked.as_str());
    assert!(answered.data["thread"].is_null(), "{}", answered.data);
    let told = rig.event("inbox.told", move |event| event.seq > after).await;
    assert!(told.data["owner"].as_str().is_some_and(|owner| owner.starts_with("user:")), "{}", told.data);
    // The architect answered the person never: only the lead, and in the thread.
    let said = thread(&rig, &id, &asked).await;
    assert!(said.iter().any(|one| one["author"] == "agent:architect"), "{said:?}");
    assert_eq!(turns_of(&rig, "architect").await, 1);
}

#[tokio::test]
async fn with_a_lead_an_advisor_and_an_agent_that_only_reports_a_person_talks_to_the_lead_names_the_advisor_or_answers_in_the_thread()
 {
    // A setup met in use: a channel whose discussions say a lead and an
    // advisor, and where a fleet's agent is a member only to report. What names nobody is the lead's alone, the agent that reports
    // is asked by nothing said there, the advisor named is asked itself,
    // and what is said in a discussion's thread is the discussion's.
    let rig = rig(
        "api-discussed-reports",
        &[
            (
                "helper",
                answers(
                    "has ended",
                    &["We will keep sessions in two tables."],
                    &["Let me check with @architect: one table?"],
                ),
            ),
            (
                "architect",
                answers(
                    "Go with two",
                    &["Two it is.", "DECISION: agree"],
                    &["Which is cheaper?", "DECISION: needs"],
                ),
            ),
            ("worker", answers("never", &[], &["I was asked."])),
        ],
    )
    .await;
    let id = channel(&rig, "design", &["helper", "architect"]).await;
    // Brought up as a fleet brings up its agents, reporting in the channel.
    let worker = json!({
        "blueprint": "std/agent", "key": "worker", "wait": true,
        "params": { "name": "worker", "adapter": "worker", "role": "worker", "reports": "design" },
    });
    let applied = rig.send("/api/apply", worker).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let channel = rig.read(&format!("/api/channels/{id}")).await;
    let reports = channel["members"].as_array().unwrap().iter().find(|one| one["actor"] == "agent:worker");
    assert!(reports.is_some_and(|one| one["listens"] == false), "{channel}");
    offer(
        &rig,
        "design",
        json!({ "lead": "helper", "advisors": ["architect: technical advice, and confirming a plan"] }),
    )
    .await;

    // Naming nobody, the person talks to the lead alone: its turn is the
    // first, and the discussion it begins hangs under what was asked.
    let after = rig.mark();
    let asked = say(&rig, "design", json!({ "body": "How should we keep sessions?" })).await;
    let asked = asked["id"].as_str().unwrap().to_string();
    let first = rig.event("agent.turn.started", move |event| event.seq > after).await;
    assert_eq!(first.data["slot"], "helper", "{}", first.data);
    let begun = rig.event("channel.discussion.begun", move |event| event.seq > after).await;
    assert_eq!(begun.data["root"], asked.as_str());
    assert_eq!(begun.data["agents"], json!(["architect", "helper"]));
    let needs = ended(&rig, after, "needs").await;
    assert_eq!(needs.data["status"], "waiting");

    // What the person says in its thread, naming nobody, is the discussion's:
    // it goes on from the advisor, and the lead is not asked it directly.
    let again = rig.mark();
    say(&rig, "design", json!({ "body": "Go with two.", "thread": asked })).await;
    let resumed = rig.event("channel.discussion.resumed", move |event| event.seq > again).await;
    assert_eq!(resumed.data["first"], "architect");
    ended(&rig, again, "agreed").await;
    rig.event("channel.message", move |event| {
        event.seq > again
            && event.data["author"] == "agent:helper"
            && event.data["body"] == "We will keep sessions in two tables."
    })
    .await;

    // Naming the advisor directly asks the advisor, and begins nothing.
    let named = rig.mark();
    let architect = turns_of(&rig, "architect").await;
    say(&rig, "design", json!({ "body": "@architect is two tables enough?" })).await;
    rig.event("agent.turn.finished", move |event| event.seq > named && event.data["slot"] == "architect")
        .await;
    assert_eq!(turns_of(&rig, "architect").await, architect + 1);
    let started =
        rig.said(journal::EventQuery { topics: "agent.turn.started".into(), ..Default::default() }).await;
    let since: Vec<&serde_json::Value> =
        started.iter().filter(|event| event.seq > named).map(|event| &event.data["slot"]).collect();
    assert_eq!(since, [&json!("architect")], "only the advisor named was asked");
    let begun = rig
        .said(journal::EventQuery { topics: "channel.discussion.begun".into(), ..Default::default() })
        .await;
    assert_eq!(begun.len(), 1, "{begun:?}");

    // And the agent that only reports was asked by none of it.
    assert_eq!(turns_of(&rig, "worker").await, 0);
}

#[tokio::test]
async fn a_channel_that_offers_no_discussions_answers_as_it_always_did() {
    let rig = rig(
        "api-discussed-none",
        &[("ann", answers("hello", &["Ask @bo."], &["Fine."])), ("bo", answers("never", &[], &["Hello."]))],
    )
    .await;
    channel(&rig, "design", &["ann", "bo"]).await;
    let after = rig.mark();
    say(&rig, "design", json!({ "body": "@ann hello" })).await;
    // Bo, named by ann, is asked as anybody named is.
    rig.event("agent.turn.finished", move |event| event.seq > after && event.data["slot"] == "bo").await;
    let said =
        rig.said(journal::EventQuery { topics: "channel.discussion.*".into(), ..Default::default() }).await;
    assert!(said.is_empty(), "{said:?}");
}

#[tokio::test]
async fn a_discussion_whose_turn_is_in_hand_as_the_daemon_stops_goes_on_after_it_and_asks_its_agent_once() {
    let mut at = None;
    let rig = Rig::with("api-discussed-restart", |config, dir| {
        let file = |name: &str| dir.join(name).to_string_lossy().into_owned();
        let (began, hold) = (file("began-bo"), file("hold-bo"));
        let waits =
            ["read", "tell", &began, "began", "held", &hold, "bo", "say", "Yes.", "say", "DECISION: agree"];
        config.adapters.push(("bo".into(), crate::standin::argv(&waits)));
        let tags = answers("keep sessions", &["One table. @bo?"], &["Fine."]);
        config
            .adapters
            .push(("ann".into(), crate::standin::argv(&tags.iter().map(String::as_str).collect::<Vec<_>>())));
        at = Some(dir.to_path_buf());
    })
    .await;
    let mut rig = Rig::owned(rig).await;
    let at = at.unwrap();
    let hold = at.join("hold-bo");
    crate::hold(&hold);
    let mut began = crate::Listening::at(&at.join("began-bo"));
    channel(&rig, "design", &["ann", "bo"]).await;
    offer(&rig, "design", json!({ "agents": ["ann", "bo"] })).await;
    let after = rig.mark();
    say(&rig, "design", json!({ "body": "@ann how do we keep sessions?" })).await;
    crate::soon("bo to be asked", async { began.next_line().await.unwrap() }).await;

    rig.restart().await;

    crate::pass(&hold, 8);
    let over = ended(&rig, after, "agreed").await;
    assert_eq!(over.data["turns"], 1);
    // The turn it had is the one it is given again: it is asked once.
    assert_eq!(turns_of(&rig, "bo").await, 1);
}

#[tokio::test]
async fn a_channel_offers_discussions_only_between_its_own_agents_and_only_as_somebody_who_may_write_there() {
    let rig = rig(
        "api-discussed-refused",
        &[("ann", answers("x", &[], &["Fine."])), ("bo", answers("x", &[], &["Hello."]))],
    )
    .await;
    channel(&rig, "design", &["ann", "bo"]).await;
    rig.send("/api/agents", json!({ "name": "carl", "adapter": "ann" })).await;
    let apply = |params: Value| {
        let mut params = params;
        params["channel"] = json!("design");
        json!({ "blueprint": "std/discuss", "key": "discuss-design", "params": params, "wait": true })
    };
    let refused = |said: &Value| said.to_string();
    // Somebody not in the channel takes no part.
    let (_, said) = rig
        .call(
            reqwest::Method::POST,
            "/api/apply",
            Some(&rig.token),
            Some(apply(json!({ "agents": ["ann", "carl"] }))),
        )
        .await;
    assert!(refused(&said).contains("`carl` is not in #design"), "{said}");
    // A cap past twenty is no cap.
    let (_, said) = rig
        .call(
            reqwest::Method::POST,
            "/api/apply",
            Some(&rig.token),
            Some(apply(json!({ "agents": ["ann", "bo"], "turns": 40 }))),
        )
        .await;
    assert!(refused(&said).contains("from 1 to 20"), "{said}");
    // A key that may not write in channels offers nothing there.
    let made = rig
        .send(
            "/api/credentials",
            json!({ "name": "narrow", "scopes": ["blueprints", "agents", "inbox", "channels:read"] }),
        )
        .await;
    let narrow = made["token"].as_str().unwrap().to_string();
    let (_, said) = rig
        .call(
            reqwest::Method::POST,
            "/api/apply",
            Some(&narrow),
            Some(apply(json!({ "agents": ["ann", "bo"] }))),
        )
        .await;
    assert!(refused(&said).contains("channels:write"), "{said}");
}

#[tokio::test]
async fn agents_that_answer_each_other_where_no_discussion_holds_them_wait_for_a_person_who_is_told() {
    let rig = rig(
        "api-discussed-loop",
        &[
            ("ann", answers("never", &[], &["And you, @bo?"])),
            ("bo", answers("never", &[], &["And you, @ann?"])),
        ],
    )
    .await;
    channel(&rig, "design", &["ann", "bo"]).await;
    let after = rig.mark();
    say(&rig, "design", json!({ "body": "@ann hello" })).await;
    let noted = rig
        .event("channel.message", move |event| {
            event.seq > after && event.data["body"].as_str().unwrap_or_default().contains("wait for a person")
        })
        .await;
    let told = rig.event("inbox.told", move |event| event.seq > after).await;
    let (_, line) = rig.get(&format!("/api/inbox/{}", told.data["line"].as_str().unwrap_or_default())).await;
    assert!(line.to_string().contains("Say something in #design"), "{line}");
    assert_eq!(line["parts"][0]["message"], noted.data["id"], "{line}");
    // Counted for the channel: the person's word lets them go on.
    let again = rig.mark();
    say(&rig, "design", json!({ "body": "@ann once more" })).await;
    rig.event("agent.turn.finished", move |event| event.seq > again && event.data["slot"] == "bo").await;
}

/// Ends the discussion under `root` as whoever holds `token`.
async fn end(rig: &Rig, token: &str, channel: &str, root: &str) -> (reqwest::StatusCode, Value) {
    let path = format!("/api/channels/{channel}/discussions/{root}/end");
    rig.call(reqwest::Method::POST, &path, Some(token), Some(json!({}))).await
}

#[tokio::test]
async fn a_person_ends_a_discussion_that_is_going_and_its_agent_s_turn_is_called_off() {
    let mut at = None;
    let rig = Rig::with("api-discussed-stopped", |config, dir| {
        let file = |name: &str| dir.join(name).to_string_lossy().into_owned();
        let (began, hold) = (file("began-bo"), file("hold-bo"));
        let waits =
            ["read", "tell", &began, "began", "held", &hold, "bo", "say", "Yes.", "say", "DECISION: agree"];
        config.adapters.push(("bo".into(), crate::standin::argv(&waits)));
        let tags = answers("keep sessions", &["One table. @bo?"], &["Fine."]);
        config
            .adapters
            .push(("ann".into(), crate::standin::argv(&tags.iter().map(String::as_str).collect::<Vec<_>>())));
        at = Some(dir.to_path_buf());
    })
    .await;
    let rig = Rig::owned(rig).await;
    let at = at.unwrap();
    crate::hold(&at.join("hold-bo"));
    let mut began = crate::Listening::at(&at.join("began-bo"));
    let id = channel(&rig, "design", &["ann", "bo"]).await;
    offer(&rig, "design", json!({ "agents": ["ann", "bo"] })).await;
    let after = rig.mark();
    say(&rig, "design", json!({ "body": "@ann how do we keep sessions?" })).await;
    let begun = rig.event("channel.discussion.begun", move |event| event.seq > after).await;
    let root = begun.data["root"].as_str().unwrap().to_string();
    crate::soon("bo to be asked", async { began.next_line().await.unwrap() }).await;

    let (status, ended) = end(&rig, &rig.token, &id, &root).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{ended}");
    let said = ended_by(&rig, after).await;
    assert!(said.data["by"].as_str().is_some_and(|by| by.starts_with("user:")), "{}", said.data);
    // What bo had in hand is called off, and the stretch that asked is stopped.
    rig.event("agent.turn.finished", move |event| event.seq > after && event.data["slot"] == "bo").await;
    rig.event("instance.stopped", move |event| {
        event.seq > after && event.data["blueprint"] == "std/discussion"
    })
    .await;
    crate::pass(&at.join("hold-bo"), 8);
    // It is said to have ended once, by the person, and nothing is said in its thread of how it ended.
    let ends = rig
        .said(journal::EventQuery { topics: "channel.discussion.ended".into(), ..Default::default() })
        .await;
    assert_eq!(ends.len(), 1, "{ends:?}");
    let words: Vec<String> = thread(&rig, &id, &root)
        .await
        .iter()
        .filter_map(|one| one["body"].as_str().map(str::to_string))
        .collect();
    assert!(!words.iter().any(|one| one.contains("agreed")), "{words:?}");
    // Ended, it is ended: asked again it is not there to end.
    let (again, _) = end(&rig, &rig.token, &id, &root).await;
    assert_eq!(again, reqwest::StatusCode::NOT_FOUND);
}

async fn ended_by(rig: &Rig, after: u64) -> std::sync::Arc<kernel::Event> {
    rig.event("channel.discussion.ended", move |event| event.seq > after && event.data["came"] == "stopped")
        .await
}

#[tokio::test]
async fn a_person_ends_a_discussion_that_waits_for_them_without_answering_and_what_they_say_there_after_begins_nothing()
 {
    let rig = rig(
        "api-discussed-ended",
        &[
            (
                "ann",
                answers("plan the sessions", &["Here is the plan: one table.", "DECISION: plan"], &["Fine."]),
            ),
            ("bo", answers("never", &[], &["Which is cheaper?", "DECISION: needs"])),
        ],
    )
    .await;
    let id = channel(&rig, "design", &["ann", "bo"]).await;
    offer(&rig, "design", json!({ "agents": ["ann", "bo"] })).await;
    let after = rig.mark();
    say(&rig, "design", json!({ "body": "@ann plan the sessions" })).await;
    let waits = ended(&rig, after, "needs").await;
    let root = waits.data["root"].as_str().unwrap().to_string();
    let (_, open) = rig.get(&format!("/api/channels/{id}/discussing")).await;
    assert_eq!(open["discussions"][0]["status"], "waiting", "{open}");

    // A key that may not write in the channel ends nothing.
    let made = rig.send("/api/credentials", json!({ "name": "reader", "scopes": ["channels:read"] })).await;
    let (refused, said) = end(&rig, made["token"].as_str().unwrap(), &id, &root).await;
    assert_eq!(refused, reqwest::StatusCode::FORBIDDEN, "{said}");

    let (status, ended) = end(&rig, &rig.token, &id, &root).await;
    assert_eq!((status, ended["status"].as_str()), (reqwest::StatusCode::OK, Some("ended")), "{ended}");
    ended_by(&rig, after).await;
    let (_, open) = rig.get(&format!("/api/channels/{id}/discussing")).await;
    assert_eq!(open["discussions"], json!([]), "{open}");
    // What she says in its thread then is answered as anywhere, and lets nothing go on.
    let again = rig.mark();
    say(&rig, "design", json!({ "body": "Go with two.", "thread": root })).await;
    rig.event("agent.turn.finished", move |event| event.seq > again).await;
    let resumed = rig
        .said(journal::EventQuery { topics: "channel.discussion.resumed".into(), ..Default::default() })
        .await;
    assert!(resumed.is_empty(), "{resumed:?}");
}
