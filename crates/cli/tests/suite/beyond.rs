//! A person given a scope beyond what their role holds, from the terminal.

use crate::Rig;

#[tokio::test]
async fn the_owner_gives_a_member_a_scope_and_takes_it_away_by_her_email() {
    let rig = Rig::signed_in().await;
    rig.run(&["tier", "premium"]).await.ok();
    let identity = &rig.node.as_ref().unwrap().app.studio.identity;
    let ada = identity.accounts().await.unwrap().remove(0).principal();
    let code = identity.invite(&ada, None).await.unwrap();
    let grace =
        identity.register("grace@example.com", "Grace", "correct horse", Some(&code), true).await.unwrap();

    let given = rig.run(&["give", "grace@example.com", "environments:secrets"]).await.ok();
    assert!(given.contains("Grace holds `environments:secrets` from now on"), "{given}");
    assert!(given.contains("beyond what their role holds: environments:secrets"), "{given}");
    assert!(identity.authenticate(&grace.token).await.unwrap().can("environments:secrets"));
    let again = rig.run(&["give", "grace@example.com", "board"]).await.ok();
    assert!(again.contains("holds `board` already"), "{again}");

    let taken = rig.run(&["give", "grace@example.com", "environments:secrets", "--not"]).await.ok();
    assert!(
        taken.contains("holds `environments:secrets` no longer, and neither do 0 of their keys"),
        "{taken}"
    );
    assert!(!identity.authenticate(&grace.token).await.unwrap().can("environments:secrets"));

    // What is refused is said in the daemon's words.
    let nobody = rig.run(&["give", "nobody@example.com", "board"]).await;
    assert_ne!(nobody.code, 0);
    assert!(nobody.err.contains("there is no account for nobody@example.com"), "{}", nobody.err);
    let wrong = rig.run(&["give", "grace@example.com", "secrets:write"]).await;
    assert_ne!(wrong.code, 0);
    assert!(wrong.err.contains("is not a scope this factory checks"), "{}", wrong.err);
}

#[tokio::test]
async fn an_agent_added_by_hand_is_moved_to_no_project_from_the_terminal() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    rig.run(&["agent", "add", "helper", "--project", "shop"]).await.ok();
    let moved = rig.run(&["agent", "move", "helper", "--none"]).await.ok();
    assert!(moved.contains("helper works in no project from its next piece of work"), "{moved}");
    let listed = rig.run(&["agent", "list", "--json"]).await.ok();
    let all: serde_json::Value = serde_json::from_str(&listed).unwrap();
    let helper =
        all["agents"].as_array().unwrap().iter().find(|one| one["name"] == "helper").unwrap().clone();
    assert_eq!(
        (&helper["project"], &helper["active"]),
        (&serde_json::Value::Null, &serde_json::json!(true)),
        "{helper}"
    );
}
