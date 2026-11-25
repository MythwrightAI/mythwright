//! An agent asks for a change it may not make with this program's own
//! command while it answers in a channel, a person approves it from a
//! terminal, and the agent is told in the channel it asked from
//! (`docs/approvals.md`).
//!
//! Unix's alone: the agent is a line of a Unix shell that runs the
//! factory's command, as `threads.rs`'s is.
#![cfg(unix)]

use serde_json::Value;

use crate::fleets::event_after;
use crate::threads::rig_running;
use crate::{BINARY, Rig};

/// A table whose `note` takes `shell`, which an agent added by hand does
/// not hold, and whose history keeps its changes.
const SETTINGS: &str = "blueprint: settings
steps:
  table:
    do: collection.ensure
    with:
      project: shop
      name: settings
      fields:
        title: { type: text }
        note: { type: text, takes: shell }
      history:
        record.updated: \"{actor} changed its {changed}\"
";

async fn json(rig: &Rig, args: &[&str]) -> Value {
    serde_json::from_str(&rig.run(&[args, &["--json"]].concat()).await.ok()).unwrap()
}

#[tokio::test]
async fn an_agent_asks_with_the_command_a_person_approves_it_and_the_agent_is_told_where_it_asked() {
    let asks = format!(
        "cat >/dev/null; \"{BINARY}\" ask shop-settings-1 note=loud --why 'it is too quiet' >/dev/null \
           || echo It could not ask.; \
         echo Asked."
    );
    let rig = rig_running("asked", asks).await;
    rig.run(&["project", "create", "Shop", "--bare"]).await.ok();
    let file = rig.dir.join("settings.bp");
    std::fs::write(&file, SETTINGS).unwrap();
    rig.run(&["apply", file.to_str().unwrap(), "--project", "shop"]).await.ok();
    rig.run(&["record", "add", "shop", "settings", "title=one"]).await.ok();
    rig.run(&["channel", "create", "talk", "--project", "shop"]).await.ok();
    rig.run(&["agent", "add", "helper", "--adapter", "poster", "--project", "shop", "--channel", "talk"])
        .await
        .ok();

    let before = crate::fleets::now(&rig);
    rig.run(&["post", "talk", "@helper", "make", "it", "louder"]).await.ok();
    event_after(&rig, before, "agent.turn.finished", |event| event.data["slot"] == "helper").await;
    let talk = rig.run(&["read", "talk"]).await.ok();
    assert!(talk.contains("[helper] Asked."), "{talk}");

    // Ada is shown what it asked for, each cell with what it takes.
    let listed = rig.run(&["asked"]).await.ok();
    assert!(listed.contains("waiting  shop-settings-1 of settings, asked by helper"), "{listed}");
    assert!(listed.contains("  why: it is too quiet"), "{listed}");
    assert!(listed.contains("  note: null -> \"loud\"; takes shell"), "{listed}");
    let found = json(&rig, &["asked"]).await;
    let id = found["asked"][0]["ask"].as_str().unwrap().to_string();

    // Approved, it is made as the agent, and the agent is told in the channel.
    let before = crate::fleets::now(&rig);
    let approved = rig.run(&["approve", &id]).await.ok();
    assert!(
        approved.contains(&format!("{id}  approved")) && approved.contains("\n  approved by Ada"),
        "{approved}"
    );
    assert_eq!(json(&rig, &["record", "show", "shop-settings-1"]).await["data"]["note"], "loud");
    event_after(&rig, before.clone(), "agent.turn.finished", |event| event.data["slot"] == "helper").await;
    let talk = rig.run(&["read", "talk"]).await.ok();
    assert!(talk.contains("@helper Ada approved your change of shop-settings-1, and it is made."), "{talk}");
    // The line is written by what keeps histories, a moment after the change.
    let approved_line =
        |event: &kernel::Event| event.data["said"].as_str().is_some_and(|said| said.contains("approved by"));
    event_after(&rig, before, "collection.history", approved_line).await;
    let history = rig.run(&["board", "history", "shop-settings-1"]).await.ok();
    assert!(history.contains("helper changed its note · approved by Ada"), "{history}");

    // Asked again of what is so already, nothing is asked; and an ask that
    // is decided is so however often it is pressed.
    let again = rig.run(&["approve", &id]).await.ok();
    assert!(again.contains(&format!("{id}  approved")), "{again}");
    let refused = rig.run(&["decline", "ask_nothing", "--why", "no"]).await;
    assert_ne!(refused.code, 0);
    assert!(refused.err.contains("there is no ask `ask_nothing`"), "{}", refused.err);

    // The help says it, under each of its words.
    let help = rig.run(&["help", "approve"]).await.ok();
    assert!(help.contains("`approve` makes the change in the one write"), "{help}");
}
