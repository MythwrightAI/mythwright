//! What a channel's discussion comes to, asked by the texts that hold its
//! turns (`std/discussion`, `std/discussion-turn`): whose turn it is and
//! whether it is over (`discussion.next`), and what is said as a stretch
//! ends (`discussion.ending`). Both are functions of plain values with
//! their cases in the kernel's suite (`kernel::discussed`); these only
//! read what a step hands them.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use kernel::Fields;
use kernel::discussed::{Next, Settings, Stretch, asked, ending, next};
use serde_json::{Value, json};

use crate::add;

fn settings(args: &Fields) -> Result<Settings, ActionError> {
    Settings::read(args.get("settings").unwrap_or(&Value::Null)).map_err(ActionError::new)
}

fn names(said: &Value) -> Vec<String> {
    said.as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_string).collect()
}

fn stretch_of(said: &Value) -> Stretch {
    let text = |key: &str| said[key].as_str().unwrap_or_default().to_string();
    Stretch {
        agents: names(&said["agents"]),
        spoken: names(&said["spoken"]),
        agreed: names(&said["agreed"]),
        nth: said["nth"].as_i64().unwrap_or(0),
        last: text("last"),
        word: text("word"),
        first: text("first"),
    }
}

fn said_of(stretch: &Stretch) -> Value {
    json!({
        "agents": stretch.agents, "spoken": stretch.spoken, "agreed": stretch.agreed, "nth": stretch.nth,
        "last": stretch.last, "word": stretch.word, "first": stretch.first,
    })
}

pub(crate) fn register(registry: &mut Registry) {
    add(
        registry,
        ActionSpec::new(
            "discussion.next",
            "Say whose turn it is in a discussion, or that it is over: agreed, needing a person, or at its cap.",
        )
        .deciding()
        .required("settings", T::Object, "what the channel offers, as `discussion.offer` keeps it")
        .required("stretch", T::Object, "where the stretch has come to: `agents`, `spoken`, `agreed`, `nth`, `first`")
        .arg("slot", T::String, "whose turn was just over; nothing before the first")
        .arg("decision", T::String, "what it ended with: reply, agree or needs")
        .output("came", "`turn`, `agreed`, `needs` or `capped`")
        .output("slot", "whose turn it is, where it is a turn")
        .output("nth", "which turn of the stretch that is")
        .output("asked", "what that agent is asked")
        .output("then", "the stretch as it stands for that turn")
        .output("turns", "how many turns the stretch has had")
        .idempotent(),
        |_, args| async move {
            let settings = settings(&args)?;
            let mut stretch = stretch_of(args.get("stretch").unwrap_or(&Value::Null));
            if stretch.agents.len() < 2 {
                return Err(ActionError::new("a discussion's `stretch` says its `agents`, two at least"));
            }
            if let Some(slot) = args.get("slot").and_then(Value::as_str).filter(|slot| !slot.is_empty()) {
                stretch.last = slot.to_string();
                stretch.word = args.get("decision").and_then(Value::as_str).unwrap_or_default().to_string();
                stretch.nth += 1;
            }
            let came = next(&settings, &stretch);
            let mut out = json!({ "came": came.came(), "turns": stretch.nth });
            if let Next::Turn { slot, then } = &came {
                out["slot"] = json!(slot);
                out["nth"] = json!(then.nth + 1);
                out["asked"] = json!(asked(&settings, then, slot));
                out["then"] = said_of(then);
            }
            Ok(out)
        },
    );
    add(
        registry,
        ActionSpec::new(
            "discussion.ending",
            "Say what is said as a stretch of a discussion ends: a line in its thread, whether the person is told \
             and in what words, and what a lead is asked to answer the person with.",
        )
        .deciding()
        .required("settings", T::Object, "what the channel offers, as `discussion.offer` keeps it")
        .required("came", T::String, "`agreed`, `needs` or `capped`")
        .required("agents", T::List, "who took part")
        .arg("turns", T::Int, "how many turns the stretch had")
        .arg("channel", T::String, "what the channel is called")
        .arg("words", T::String, "what the last turn said: for `needs`, what is to be decided")
        .output("status", "what the discussion is left as: `agreed`, or `waiting` for a person")
        .output("line", "what the factory's bot says in the thread")
        .output("tells", "whether the person is told in their inbox")
        .output("says", "what they are told")
        .output("lead", "where a lead answers the person in the channel, what it is asked; empty otherwise")
        .idempotent(),
        |_, args| async move {
            let settings = settings(&args)?;
            let text = |key: &str| args.get(key).and_then(Value::as_str).unwrap_or_default();
            let came = text("came");
            if !["agreed", "needs", "capped"].contains(&came) {
                return Err(ActionError::new("`came` is `agreed`, `needs` or `capped`"));
            }
            let agents = names(args.get("agents").unwrap_or(&Value::Null));
            let turns = args.get("turns").and_then(Value::as_i64).unwrap_or(0);
            let ended = ending(&settings, came, &agents, turns, text("channel"), text("words"));
            Ok(json!({
                "status": ended.status, "line": ended.line, "tells": ended.tells, "says": ended.says,
                "lead": ended.lead,
            }))
        },
    );
}
