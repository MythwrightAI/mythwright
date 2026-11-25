//! A key withdrawn from the terminal stops what it applied, unless it is
//! told to leave it (`key revoke <id> [--leave]`), and a key rolled is the
//! same key with another secret (`key roll <id>`), held by the real binary.

use serde_json::Value;

use crate::Rig;

/// A blueprint called `name` that stays, listening, once it is applied.
fn stays(name: &str) -> String {
    format!(
        "blueprint: {name}\non:\n  heard:\n    event: channel.message\n    steps:\n      again: {{ do: emit, with: {{ topic: {name}.heard }} }}\n"
    )
}

/// A key that holds less than its maker: its id and its token.
async fn key(rig: &Rig, name: &str) -> (String, String) {
    let made =
        rig.run(&["key", name, "--scope", "blueprints", "--scope", "channels", "--scope", "emit"]).await.ok();
    let token = made.lines().next().unwrap().to_string();
    let listed: Value = serde_json::from_str(&rig.run(&["keys", "--json"]).await.ok()).unwrap();
    let id = listed["credentials"].as_array().unwrap().iter().find(|one| one["name"] == name).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    (id, token)
}

/// The ids of the instances that are alive.
async fn alive(rig: &Rig) -> Vec<String> {
    let listed: Value = serde_json::from_str(&rig.run(&["instances", "--json"]).await.ok()).unwrap();
    let all = listed["instances"].as_array().or(listed.as_array()).cloned().unwrap_or_default();
    all.iter().map(|one| one["id"].as_str().unwrap().to_string()).collect()
}

/// Applies a blueprint called `name` that stays, as `token`: the instance.
async fn applied(rig: &Rig, token: &str, name: &str) -> String {
    let file = rig.dir.join(format!("{name}.bp"));
    std::fs::write(&file, stays(name)).unwrap();
    let before = alive(rig).await;
    rig.run(&["apply", file.to_str().unwrap(), "--token", token]).await.ok();
    let after = alive(rig).await;
    after.into_iter().find(|one| !before.contains(one)).expect("it to be alive")
}

#[tokio::test]
async fn a_key_revoked_from_the_terminal_stops_what_it_applied_unless_told_to_leave_it() {
    let rig = Rig::signed_in().await;
    let (left_id, left_token) = key(&rig, "left").await;
    let (stopped_id, stopped_token) = key(&rig, "stopped").await;
    let by_left = applied(&rig, &left_token, "one").await;
    let by_stopped = applied(&rig, &stopped_token, "two").await;

    let said = rig.run(&["key", "revoke", &left_id, "--leave"]).await.ok();
    assert!(
        said.contains("`left` is withdrawn") && said.contains("still running") && said.contains(&by_left),
        "{said}"
    );
    assert!(alive(&rig).await.contains(&by_left), "it was left as it was told to be");
    assert_ne!(rig.run(&["whoami", "--token", &left_token]).await.code, 0, "the key is somebody's still");

    let said = rig.run(&["key", "revoke", &stopped_id]).await.ok();
    assert!(said.contains("What it applied was stopped") && said.contains(&by_stopped), "{said}");
    let still = alive(&rig).await;
    assert!(!still.contains(&by_stopped) && still.contains(&by_left), "{still:?}");
}

#[tokio::test]
async fn a_key_rolled_from_the_terminal_has_another_secret_and_what_it_applied_goes_on() {
    let rig = Rig::signed_in().await;
    let (id, old) = key(&rig, "rolled").await;
    let by_key = applied(&rig, &old, "kept").await;

    // A key may roll itself.
    let said = rig.run(&["key", "roll", &id, "--token", &old]).await.ok();
    let new = said.lines().next().unwrap().to_string();
    assert!(new.starts_with("key_") && new != old && said.contains("shown once"), "{said}");
    assert_ne!(rig.run(&["whoami", "--token", &old]).await.code, 0, "the old secret still works");
    assert_eq!(rig.run(&["whoami", "--token", &new]).await.code, 0);
    assert!(alive(&rig).await.contains(&by_key), "what it applied was stopped as it was rolled");
    let help = rig.run(&["help", "key"]).await.ok();
    assert!(help.contains("key roll <id>") && help.contains("--leave"), "{help}");
}
