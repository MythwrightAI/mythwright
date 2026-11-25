//! `agent add --scope`: what an agent holds, said whole from the terminal,
//! of one being added and of one that is there.

use serde_json::Value;

use crate::Rig;

const USUAL: &[&str] =
    &["projects", "channels", "board", "records", "blueprints:read", "agents:read", "fleet:read", "search"];

/// An agent as `agent list --json` gives it.
async fn agent(rig: &Rig, name: &str) -> Value {
    let listed: Value = serde_json::from_str(&rig.run(&["agent", "list", "--json"]).await.ok()).unwrap();
    let agents = listed["agents"].as_array().or(listed.as_array()).cloned().unwrap_or_default();
    agents.into_iter().find(|agent| agent["name"] == name).unwrap_or_else(|| panic!("no {name}: {listed}"))
}

#[tokio::test]
async fn what_an_agent_holds_is_said_whole_from_the_terminal_of_a_new_one_and_of_one_that_is_there() {
    let rig = Rig::signed_in().await;
    // Added with nothing said, it holds what an agent usually does, and
    // the command says nothing of it.
    let added =
        rig.run(&["agent", "add", "helper", "--instructions", "Be brief.", "--role", "scout"]).await.ok();
    assert!(added.starts_with("helper is in, run by ") && !added.contains("It holds"), "{added}");
    let before = agent(&rig, "helper").await;
    assert_eq!(before["scopes"], serde_json::json!(USUAL), "{before}");

    // Said of one that is there, it changes what it holds and leaves the
    // rest of it as it was.
    let mut said = vec!["agent", "add", "helper"];
    let more: Vec<&str> =
        USUAL.iter().copied().filter(|scope| *scope != "blueprints:read").chain(["blueprints"]).collect();
    for scope in &more {
        said.extend(["--scope", scope]);
    }
    let given = rig.run(&said).await.ok();
    assert!(
        given.starts_with("helper is in, run by ")
            && given.trim_end().ends_with(&format!("It holds: {}", more.join(", "))),
        "{given}"
    );
    let after = agent(&rig, "helper").await;
    assert_eq!(after["scopes"], serde_json::json!(more), "{after}");
    let rest = |agent: &Value| {
        let mut agent = agent.as_object().unwrap().clone();
        agent.retain(|name, _| !["scopes", "updated"].contains(&name.as_str()));
        agent
    };
    assert_eq!(rest(&before), rest(&after), "only what it holds changed");
    // And said again of nothing, it holds what it held.
    rig.run(&["agent", "add", "helper"]).await.ok();
    assert_eq!(agent(&rig, "helper").await["scopes"], serde_json::json!(more));

    // A new one is added holding what is said, and no more.
    let reader =
        rig.run(&["agent", "add", "reader", "--scope", "board:read", "--scope", "search"]).await.ok();
    assert!(reader.trim_end().ends_with("It holds: board:read, search"), "{reader}");

    // With a key that may change agents and apply what an agent is set up
    // by (`std/agent`), and nothing else of blueprints, that is not given,
    // and is refused by name; nothing is changed.
    let mut key = vec!["key", "arranger", "--scope", "agents", "--scope", "blueprints:run"];
    for scope in USUAL {
        key.extend(["--scope", scope]);
    }
    let token = rig.run(&key).await.ok().lines().next().unwrap().to_string();
    let wider =
        ["agent", "add", "reader", "--scope", "board:read", "--scope", "blueprints", "--token", &token];
    let refused = rig.run(&wider).await;
    assert!(
        refused.code != 0 && refused.err.contains("cannot give an agent `blueprints`, not holding it"),
        "{}",
        refused.err
    );
    assert_eq!(agent(&rig, "reader").await["scopes"], serde_json::json!(["board:read", "search"]));
    let narrower = ["agent", "add", "reader", "--scope", "search", "--token", &token];
    assert!(rig.run(&narrower).await.ok().trim_end().ends_with("It holds: search"));

    let help = rig.run(&["help", "agent"]).await.ok();
    assert!(help.contains("`--scope`, said once for") && help.contains("--scope blueprints "), "{help}");
}

/// A factory started holding no walls (`--walls off`, as the tests'
/// own are) shows every program the device whatever its slot asks for:
/// what the terminal says an agent sees is what is held as the daemon
/// runs, never what the slot asked for (a run of the demonstration said
/// "behind a wall" of agents in such a factory).
#[tokio::test]
async fn an_agent_that_asks_for_a_wall_in_a_factory_that_holds_none_is_said_to_have_none() {
    let dir = crate::scratch("walls-off");
    let order = std::sync::Arc::new(journal::Order::default());
    let mut config = api::Config {
        listen: "127.0.0.1:0".parse().unwrap(),
        device: Some("test-device".into()),
        default_adapter: Some("builtin".into()),
        walls: Some(api::Walls::Open),
        stands_between: true,
        disk: journal::Disk::Unwaited,
        order: Some(order.clone()),
        ..api::Config::new(dir.join("data"))
    };
    // A program of its own, which nothing here asks to run.
    config.adapters.push(("program".into(), vec!["program".into()]));
    let node = api::Node::start(config).await.unwrap();
    let rig = Rig { url: node.url(), node: Some(node), dir, order };
    rig.run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
        .await
        .ok();

    let none = "no wall, since this factory holds none";
    let added = rig.run(&["agent", "add", "helper", "--adapter", "program", "--git", "commit"]).await.ok();
    assert!(added.contains(none) && !added.contains("behind a wall"), "{added}");
    let listed = rig.run(&["agent", "list"]).await.ok();
    let line = listed.lines().find(|line| line.contains("helper")).unwrap_or_default();
    assert!(line.contains(none) && !listed.contains("behind a wall"), "{listed}");
    assert!(
        !line.contains("git commit"),
        "nothing of a repository behind a wall that is not there: {listed}"
    );

    let helper = agent(&rig, "helper").await;
    assert_eq!(
        helper["wall"], true,
        "the slot still asks for one, which a factory that holds walls gives it"
    );
    assert_eq!(helper["sees"], serde_json::json!({ "what": "this device", "how": none, "walled": false }));
}
