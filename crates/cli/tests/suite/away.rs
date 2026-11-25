//! An agent taken away from a terminal (`agent remove`), by the real
//! program asking a daemon: what it says would go and what goes, what is
//! in the way said as the daemon says it, and an agent's own credential
//! refused.

use crate::Rig;
use crate::fleets::{event_after, now};

/// The agents listed, by name.
async fn listed(rig: &Rig) -> Vec<String> {
    let list = rig.run(&["agent", "list"]).await.ok();
    list.lines().filter_map(|line| line.split_whitespace().next()).map(str::to_string).collect()
}

#[tokio::test]
async fn an_agent_is_taken_away_from_a_terminal_by_a_person_and_what_is_in_the_way_is_said() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "Shop", "--without", "fleet"]).await.ok();
    rig.run(&["channel", "create", "shop", "--project", "shop"]).await.ok();
    rig.run(&["agent", "add", "helper", "--project", "shop", "--channel", "shop"]).await.ok();
    // One that holds all an agent can be given, to ask as.
    rig.run(&["agent", "add", "other", "--scope", "agents", "--scope", "admin"]).await.ok();
    let before = now(&rig);
    rig.run(&["post", "shop", "@helper what can you do?"]).await.ok();
    event_after(&rig, before, "agent.turn.finished", |event| event.data["slot"] == "helper").await;
    rig.run(&["board", "add", "shop", "Paint", "the", "door"]).await.ok();
    rig.run(&["board", "give", "shop-1", "helper"]).await.ok();

    // Asked about, it says what would go and what would stay, and nothing has.
    let looked = rig.run(&["agent", "remove", "helper", "--look"]).await.ok();
    let would = format!(
        "taking helper away would take
  its slot, its credential, the folder that was its own, and its place in #shop
  what it said and did stays, under its name: what it said in channels, and the one piece of work it was asked
  shop-1 is given to it, and stays so until somebody gives it to another (`{cli} board give <card> <agent>`)
  an agent that is later called `helper` is another agent, and has none of this for its own
nothing was changed\n",
        cli = brand::CLI
    );
    assert_eq!(looked, would);
    assert_eq!(listed(&rig).await, ["helper", "other"]);

    // An agent asks with its own credential, as its program would: refused.
    let own = rig.dir.join("data/slots/other/token").to_string_lossy().into_owned();
    let refused = rig.run_with(&[(&brand::env_var("TOKEN_FILE"), own)], &["agent", "remove", "helper"]).await;
    assert_ne!(refused.code, 0, "{}", refused.out);
    assert!(
        refused.err.contains("taking an agent away is a person's to do, and other is not one"),
        "{}",
        refused.err
    );
    assert_eq!(listed(&rig).await, ["helper", "other"]);

    // Taken away, it is not listed, and what stays is said.
    let gone = rig.run(&["agent", "remove", "helper"]).await.ok();
    assert!(gone.starts_with("helper is taken away, for good, with\n  its slot, its credential"), "{gone}");
    assert!(gone.contains("shop-1 is given to it"), "{gone}");
    assert_eq!(listed(&rig).await, ["other"]);
    let read = rig.run(&["read", "shop"]).await.ok();
    assert!(read.contains("helper"), "what it said is there still:\n{read}");
    // And asked for exactly what the daemon answered, it is that.
    rig.run(&["agent", "add", "third"]).await.ok();
    let answered = rig.run(&["agent", "remove", "third", "--json"]).await.ok();
    let answered: serde_json::Value = serde_json::from_str(&answered).unwrap();
    assert_eq!((answered["name"].as_str(), answered["removed"].as_bool()), (Some("third"), Some(true)));

    // There is no such agent a second time; and one with nothing said of
    // which is told what is missing.
    let again = rig.run(&["agent", "remove", "helper"]).await;
    assert_ne!(again.code, 0);
    assert!(again.err.contains("there is no agent called `helper`"), "{}", again.err);
    let unsaid = rig.run(&["agent", "remove"]).await;
    assert!(unsaid.err.contains("the agent to take away"), "{}", unsaid.err);

    // One that a fleet counts says where it is changed.
    rig.run(&["project", "create", "Yard"]).await.ok();
    let counted = rig.run(&["agent", "remove", "yard-worker-1"]).await;
    assert_ne!(counted.code, 0);
    assert!(counted.err.contains("`yard-worker-1` is brought up by"), "{}", counted.err);
    assert!(
        counted.err.contains("in the `agents` table of the project `yard`, lower the row's count"),
        "{}",
        counted.err
    );
    let looked = rig.run(&["agent", "remove", "yard-worker-1", "--look"]).await.ok();
    assert!(
        looked.starts_with(
            "yard-worker-1 cannot be taken away as things stand: `yard-worker-1` is brought up by"
        ),
        "{looked}"
    );

    // The help says the command where the others are, and what goes and stays.
    let help = rig.run(&["help"]).await.ok();
    assert!(help.contains("| agent remove <name> [--look]"), "{help}");
    let topic = rig.run(&["help", "agent"]).await.ok();
    for said in
        ["takes it\naway for good", "what goes", "what stays", "its row of the project's `agents` table"]
    {
        assert!(topic.contains(said), "`{said}` in:\n{topic}");
    }
}
