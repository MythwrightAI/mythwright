//! Who of a project's agents is there to be told of: which of them take
//! part in a channel, and how many there are of some roles.
//!
//! Something that has just brought agents up says so to people, and what
//! it says is only of use if it says who is there: who can be talked to
//! where this is said, and how many there are to do the work. Which roles
//! those are is its blueprint's to say (it knows what its roles are for;
//! nothing here does), and it hands them: the roles whose agents are
//! named if they take part in the channel, and the roles whose agents are
//! counted. What comes of it is [`present`], a function of the agents
//! there are and of what was handed, which a blueprint asks as
//! `agent.present` and then says its own sentences by.

use blueprint::{ActionSpec, ValueType as T};
use conductor::Registry;
use serde_json::{Value, json};

use crate::Agents;
use crate::actions::{add, text};

pub use deciding::present::{Came, One, Present, present};

pub(crate) fn register(registry: &mut Registry, agents: &Agents) {
    add(
        registry,
        agents,
        ActionSpec::new(
            "agent.present",
            "Say who of a project's agents is there: which of some roles take part in a channel, by name, and \
             how many there are of some roles.",
        )
        .arg("project", T::String, "whose agents (default: this instance's project)")
        .arg("channel", T::String, "the channel, by its name or id; with none, nobody is heard")
        .arg("here", T::List, "the roles whose agents are named where they take part in the channel")
        .arg("counted", T::List, "the roles whose agents are counted")
        .capability("agents:read")
        .output("came", &format!("{}: somebody is heard and somebody counted; only heard; only counted; or neither", Came::words()))
        .output("heard", "the agents of the roles `here` that take part in the channel, by name")
        .output("first", "the first of them; nothing where there is none")
        .output("alone", "whether the first of them is the one agent in the channel, so that what is said there with nobody named is said to it")
        .output("count", "how many agents there are of the roles `counted`")
        // Stood in, there are no agents to be there: nobody is.
        .stands(serde_json::json!({ "came": "nobody", "heard": [], "first": "", "alone": false, "count": 0 }))
        .idempotent(),
        |agents, cx, args| async move {
            let project = text(&args, "project").or_else(|| cx.project().map(str::to_string));
            let roles = |key: &str| -> Vec<String> {
                let listed = args.get(key).and_then(Value::as_array).into_iter().flatten();
                listed.map(blueprint::expr::display).filter(|role| !role.is_empty()).collect()
            };
            // Who takes part in the channel, as agents are known there.
            let members: Vec<String> = match text(&args, "channel") {
                Some(channel) => {
                    // Who takes part is the channel's to say: of one kept to
                    // its members, to whoever the instance runs as being one.
                    let channels = &agents.inner.studio.channels;
                    let reader = channels.may_find(&channel, cx.principal()).await?;
                    let channel = channels.get(&reader, &channel).await?;
                    // One there to say what it is asked takes no part in what names nobody.
                    let agent = |member: &studio::Member| member.actor.strip_prefix("agent:").map(str::to_string);
                    channel.members.iter().filter(|member| member.listens).filter_map(agent).collect()
                }
                None => Vec::new(),
            };
            let all: Vec<One> = agents
                .slots()
                .await?
                .into_iter()
                .filter(|slot| slot.project == project)
                .map(|slot| One {
                    in_channel: members.contains(&slot.name),
                    // One this factory does not start takes no work,
                    // and nothing said with nobody named is said to it.
                    up: slot.active && !slot.leaving && !slot.external(),
                    name: slot.name,
                    role: slot.role,
                })
                .collect();
            let present = present(&all, &roles("here"), &roles("counted"));
            Ok(json!({
                "came": present.came.word(),
                "heard": present.heard,
                "first": present.first,
                "alone": present.alone,
                "count": present.count,
            }))
        },
    );
}
