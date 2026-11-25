//! Workspaces from the terminal: made, filled, called something else and
//! taken away, and what a channel belongs to said and changed. A factory
//! always has one, and a project is always in one.

use serde_json::Value;

use crate::Rig;

async fn listed(rig: &Rig) -> Vec<Value> {
    let found: Value = serde_json::from_str(&rig.run(&["workspace", "list", "--json"]).await.ok()).unwrap();
    found["workspaces"].as_array().unwrap().clone()
}

#[tokio::test]
async fn workspaces_are_made_filled_called_something_else_and_taken_away_from_the_terminal() {
    let rig = Rig::signed_in().await;
    // Its owner's is there from when they registered, and is the usual
    // one: where a project goes when nobody says.
    let first = rig.run(&["workspace", "list"]).await.ok();
    assert!(
        first.starts_with("ada-s-workspace") && first.contains("Ada's workspace (the usual one)"),
        "{first}"
    );
    assert_eq!(first.lines().count(), 1, "{first}");

    let made = rig.run(&["workspace", "create", "Shop", "Front"]).await.ok();
    assert!(made.starts_with("made the workspace Shop Front (shop-front), with nothing in it yet"), "{made}");
    rig.run(&["workspace", "create", "annex"]).await.ok();

    // A project is added in one, and one that is there is put in another.
    let shop = rig.run(&["project", "create", "Shop", "--bare", "--workspace", "shop-front"]).await.ok();
    assert!(shop.starts_with("project `shop` created at "), "{shop}");
    rig.run(&["project", "create", "Depot", "--bare"]).await.ok();
    let all = listed(&rig).await;
    assert_eq!(
        (&all[0]["id"], &all[0]["projects"]),
        (&"ada-s-workspace".into(), &serde_json::json!(["depot"]))
    );
    let put = rig.run(&["workspace", "put", "annex", "depot"]).await.ok();
    assert_eq!(put.trim_end(), "depot is in the workspace annex; nothing else of it changed");
    // A project for a workspace there is not is not made.
    let nowhere = rig.run(&["project", "create", "Other", "--bare", "--workspace", "cellar"]).await;
    assert!(nowhere.code != 0 && nowhere.err.contains("there is no workspace `cellar`"), "{}", nowhere.err);
    assert!(!rig.run(&["project", "list"]).await.ok().contains("other"));

    // A channel is made a workspace's, a project's or nobody's, and what
    // it belongs to is said and changed afterwards.
    rig.run(&["channel", "create", "plans", "--workspace", "shop-front"]).await.ok();
    rig.run(&["channel", "create", "chat"]).await.ok();
    let now = rig.run(&["channel", "belongs", "chat", "--project", "depot"]).await.ok();
    assert_eq!(now.trim_end(), "#chat belongs to the project depot");
    let now = rig.run(&["channel", "belongs", "#chat", "--workspace", "annex"]).await.ok();
    assert_eq!(now.trim_end(), "#chat belongs to the workspace annex");
    let both = rig.run(&["channel", "belongs", "chat", "--project", "depot", "--workspace", "annex"]).await;
    assert!(both.code != 0 && both.err.contains("and one thing"), "{}", both.err);
    let unsaid = rig.run(&["channel", "belongs", "chat"]).await;
    assert!(
        unsaid.code != 0 && unsaid.err.contains("one thing: `--project P` or `--workspace W`"),
        "{}",
        unsaid.err
    );

    // Listed by name, each with its projects and its own channels by
    // what they are called.
    let list = rig.run(&["workspace", "list"]).await.ok();
    let lines: Vec<&str> = list.lines().collect();
    assert_eq!(lines.len(), 3, "{list}");
    assert!(lines[0].starts_with("ada-s-workspace") && lines[0].contains("no projects"), "{list}");
    assert!(
        lines[1].starts_with("annex") && lines[1].contains("depot") && lines[1].contains("#chat"),
        "{list}"
    );
    assert!(
        lines[2].starts_with("shop-front") && lines[2].contains("shop") && lines[2].contains("#plans"),
        "{list}"
    );
    assert!(!lines[1].contains("the usual one") && !lines[2].contains("the usual one"), "{list}");

    // Called something else, its id is as it was.
    let named = rig.run(&["workspace", "rename", "shop-front", "The", "whole", "shop"]).await.ok();
    assert_eq!(named.trim_end(), "shop-front is now called The whole shop; its id is as it was");
    let all = listed(&rig).await;
    assert_eq!((&all[2]["id"], &all[2]["name"]), (&"shop-front".into(), &"The whole shop".into()), "{all:?}");

    // A project is never taken out of every workspace: refused, saying
    // to put it in another.
    let out = rig.run(&["workspace", "take", "depot"]).await;
    assert!(out.code != 0 && out.err.contains("a project is always in a workspace"), "{}", out.err);
    assert!(out.err.contains("workspace put <workspace> depot"), "{}", out.err);
    // Nor is a channel taken out of everything: it is a project's or a
    // workspace's, and one made with neither said is the usual one's own.
    let nothing = rig.run(&["channel", "belongs", "chat", "--nothing"]).await;
    assert!(nothing.code != 0 && nothing.err.contains("and never nobody's"), "{}", nothing.err);
    assert!(nothing.err.contains("channel belongs chat --workspace <workspace>"), "{}", nothing.err);
    rig.run(&["channel", "create", "notes"]).await.ok();
    let all = listed(&rig).await;
    assert_eq!(all[0]["channels"].as_array().unwrap().len(), 1, "{all:?}");
    rig.run(&["channel", "belongs", "notes", "--workspace", "annex"]).await.ok();
    // A workspace that holds something is taken away with where that
    // goes, and nothing in it is removed.
    let held = rig.run(&["workspace", "remove", "shop-front"]).await;
    assert!(
        held.code != 0 && held.err.contains("workspace remove shop-front --to <workspace>"),
        "{}",
        held.err
    );
    let gone = rig.run(&["workspace", "remove", "shop-front", "--to", "annex"]).await.ok();
    assert!(
        gone.starts_with("took the workspace The whole shop away. Nothing that was in it is removed: its 1 project(s) and 1 channel(s) are in annex now"),
        "{gone}"
    );
    let all = listed(&rig).await;
    assert_eq!(all.len(), 2, "{all:?}");
    assert_eq!(
        (&all[1]["id"], &all[1]["projects"]),
        (&"annex".into(), &serde_json::json!(["depot", "shop"]))
    );
    assert_eq!(all[1]["channels"].as_array().unwrap().len(), 3, "{all:?}");
    // One with nothing in it goes as it is, and the last one stays.
    let gone = rig.run(&["workspace", "remove", "ada-s-workspace"]).await.ok();
    assert!(gone.starts_with("took the workspace Ada's workspace away, which had nothing in it"), "{gone}");
    let last = rig.run(&["workspace", "remove", "annex"]).await;
    assert!(last.code != 0 && last.err.contains("is the only workspace"), "{}", last.err);
    assert!(last.err.contains("workspace rename annex <name>"), "{}", last.err);
    let all = listed(&rig).await;
    assert_eq!((all.len(), &all[0]["id"], &all[0]["usual"]), (1, &"annex".into(), &true.into()), "{all:?}");
    let projects = rig.run(&["project", "list"]).await.ok();
    assert!(projects.contains("shop") && projects.contains("depot"), "{projects}");
    let channels = rig.run(&["channel", "list"]).await.ok();
    assert!(channels.contains("#plans") && channels.contains("#chat"), "{channels}");

    // With a key that may read and may not add a project, none is made.
    let token = rig
        .run(&["key", "reader", "--scope", "projects:read", "--scope", "channels:read"])
        .await
        .ok()
        .lines()
        .next()
        .unwrap()
        .to_string();
    let refused = rig.run(&["workspace", "create", "Mine", "--token", &token]).await;
    assert!(refused.code != 0 && refused.err.contains("projects:write"), "{}", refused.err);
    let refused = rig.run(&["channel", "belongs", "chat", "--workspace", "annex", "--token", &token]).await;
    assert!(refused.code != 0 && refused.err.contains("channels:write"), "{}", refused.err);
    assert_eq!(listed(&rig).await.len(), 1);

    // The help says it, and that it is grouping and nothing else.
    let help = rig.run(&["help", "workspace"]).await.ok();
    assert!(help.contains("every project is in exactly one"), "{help}");
    assert!(help.contains("`workspace take` is refused"), "{help}");
    assert!(help.contains("It is grouping and nothing else."), "{help}");
    assert!(help.contains("lets nobody in and keeps nobody out"), "{help}");
    assert!(rig.run(&["help"]).await.ok().contains("(help workspace)"));
}
