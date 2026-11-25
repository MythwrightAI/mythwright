//! A discussion between two agents of a channel, stopped anywhere in it
//! and started again.
//!
//! The scenario is a channel that offers its agents discussions, where a
//! person asks one agent something, that agent names the other in its
//! answer, and the other agrees. What is new to be stopped in is the
//! discussion: as it is begun, in each step of its stretch and of the
//! turn, as the turn is written down, as it ends and as the person is
//! told. Wherever the daemon goes, the thread says the other agent's
//! answer and how it ended once, and the person has one line of it.

use kernel::Event;
use serde_json::{Value, json};

use crate::Rig;
use crate::found::Grain;
use crate::simulation::{Came, Place, Play, Scenario, simulate};

#[derive(Clone)]
struct Agreed;

impl Scenario for Agreed {
    const ACTS: usize = 1;

    async fn stage(&self) -> Play {
        let mut at = None;
        let rig = Rig::with("api-discussing", |config, dir| {
            let file = |name: &str| dir.join(name).to_string_lossy().into_owned();
            let (began, hold, ended) = (file("began"), file("hold"), file("ended"));
            // It reads what it is asked, says its program is there, waits
            // to be let go, answers and says it has ended.
            let stands_in = |who: &str, does: &[&str]| {
                let first = ["read", "tell", &began, who, "held", &hold, who];
                crate::standin::argv(&[&first[..], does, &["append", &ended, who][..]].concat())
            };
            let asks = [
                "where",
                "keep sessions",
                "(",
                "say",
                "One table. @critic, is that right?",
                "exit",
                "0",
                ")",
            ];
            config
                .adapters
                .push(("writer".into(), stands_in("writer", &[&asks[..], &["say", "Fine."]].concat())));
            config
                .adapters
                .push(("critic".into(), stands_in("critic", &["say", "Yes.", "say", "DECISION: agree"])));
            at = Some(dir.to_path_buf());
        })
        .await;
        let rig = Rig::owned(rig).await;
        rig.send("/api/channels", json!({ "name": "design" })).await;
        for agent in ["writer", "critic"] {
            rig.send("/api/agents", json!({ "name": agent, "adapter": agent, "channel": "design" })).await;
        }
        let params = json!({ "channel": "design", "agents": ["writer", "critic"] });
        let offer =
            json!({ "blueprint": "std/discuss", "key": "discuss-design", "params": params, "wait": true });
        rig.send("/api/apply", offer).await;
        Play::on(rig, &at.unwrap()).await
    }

    async fn act(&self, _: usize, play: &mut Play) {
        // A person looks first: what they said before the daemon went is there.
        let body = "@writer how do we keep sessions?";
        let there = play.rig.read("/api/channels/design/messages").await;
        let said = there["messages"].as_array().into_iter().flatten().any(|one| one["body"] == body);
        if !said {
            play.rig.send("/api/channels/design/messages", json!({ "body": body })).await;
        }
    }

    fn heard(&self, _: usize, play: &Play, _: &Event) -> bool {
        // It ended, and the stretch that held it is over.
        let ended = play.times(|place| place.topic == "channel.discussion.ended") > 0;
        let over =
            play.times(|place| place.topic == "instance.finished" && place.within("std/discussion")) > 0;
        ended && over
    }

    async fn came_to(&self, play: &Play) -> Came {
        let rig = &play.rig;
        let top = rig.read("/api/channels/design/messages").await;
        let top = top["messages"].as_array().cloned().unwrap_or_default();
        let bodies = |said: &[Value]| -> Value { said.iter().map(|one| one["body"].clone()).collect() };
        let root = top.iter().find(|one| one["author"] == "agent:writer").and_then(|one| one["id"].as_str());
        let thread = match root {
            Some(root) => rig.read(&format!("/api/channels/design/messages?thread={root}")).await["messages"]
                .as_array()
                .cloned()
                .unwrap_or_default(),
            None => Vec::new(),
        };
        let lines = rig.read("/api/inbox?all=true").await;
        let told: Value = lines["lines"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|line| json!([line["says"], line["why"], line["count"]]))
            .collect();
        let mut came = Came::default().with("channel", bodies(&top)).with("thread", bodies(&thread));
        came = came.with("told", told);
        came.asked = crate::observed::asked(rig, &["writer", "critic"]).await;
        came
    }
}

/// Where a discussion is under way: what its texts say of their runs,
/// what is said of it, and the line that tells of it.
fn in_the_discussion(place: &Place) -> bool {
    place.within("std/discussion")
        || place.within("std/discussion-turn")
        || place.topic.starts_with("channel.discussion.")
        || place.topic == "inbox.told"
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_discussion_between_two_agents_is_agreed_and_told_once_wherever_the_daemon_is_stopped_in_it() {
    let grain = Grain::asked_or(Grain::Where(in_the_discussion));
    let found = simulate("two agents that discuss and agree", Agreed, grain).await;
    found.holds();
}
