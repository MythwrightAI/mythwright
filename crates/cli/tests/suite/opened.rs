//! Nobody registers to begin: the first start of a
//! factory makes its owner and signs the command line that started it in;
//! `open` asks for a link that signs a browser in once; and a terminal
//! signed in as nobody is signed in as the owner by proving this machine
//! can read the factory's data folder.
//!
//! Unix's alone: the daemon is the real program as a process of its own
//! (`crate::daemon`).
#![cfg(unix)]

use serde_json::{Value, json};

use crate::daemon::Daemon;
use crate::{run_in, scratch};

/// Spends a link as a browser's page would, saying how it was answered.
async fn spend(url: &str, link: &str) -> (u16, Value) {
    let once = link.split("#once=").nth(1).expect("a link carries its secret after `#once=`");
    let answer = reqwest::Client::new()
        .post(format!("{url}/api/auth/link"))
        .json(&json!({ "once": once, "page": true }))
        .send()
        .await
        .unwrap();
    let status = answer.status().as_u16();
    (status, answer.json().await.unwrap_or_default())
}

#[tokio::test(flavor = "multi_thread")]
async fn the_first_start_makes_the_owner_and_signs_its_command_line_in_with_nothing_to_register() {
    let mut daemon = Daemon::start("serve").await;
    // Nobody has said what they are called: the owner is a placeholder, and
    // so is the workspace made with them.
    daemon.until("owner   Unnamed, made now: nobody registers to begin").await;
    daemon.until(&format!("`{} open` opens the client in your browser, signed in", brand::CLI)).await;
    assert!(daemon.run(&["whoami"]).await.ok().starts_with("Unnamed"), "the command line is the owner's");
    let spaces = daemon.run(&["workspace", "list"]).await.ok();
    assert!(spaces.starts_with("unnamed-workspace") && spaces.contains("Unnamed workspace"), "{spaces}");

    // A link to this daemon, good once.
    let link = daemon.run(&["open", "--link"]).await.ok().trim().to_string();
    assert!(link.starts_with(&format!("{}/#once=once_", daemon.url)), "{link}");
    let (status, signed) = spend(&daemon.url, &link).await;
    assert_eq!(status, 200, "{signed}");
    assert_eq!(signed["account"]["role"], "owner");
    let (status, refused) = spend(&daemon.url, &link).await;
    assert_eq!(status, 401, "a link spent twice: {refused}");

    // A terminal that holds nothing, on this machine and run by whoever runs
    // the factory, gives the owner an email, a name and a password: it
    // proves it can read the data folder the daemon says it keeps.
    let elsewhere = scratch("opened-elsewhere");
    let given = ["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"];
    let welcomed = run_in(&elsewhere, &daemon.url, &[], &given).await.ok();
    assert_eq!(welcomed.trim(), "Welcome, Ada. You are signed in as the owner.");
    assert!(daemon.run(&["whoami"]).await.ok().starts_with("Ada"), "the start's terminal is Ada's");
    // The workspace made with her is called after her, by the id it was made with.
    let spaces = daemon.run(&["workspace", "list"]).await.ok();
    assert!(
        spaces.starts_with("unnamed-workspace") && spaces.contains("Ada's workspace (the usual one)"),
        "{spaces}"
    );

    // Once she has a password nobody registers over her.
    let other = scratch("opened-other");
    let register = ["register", "--email", "eve@example.com", "--name", "Eve", "--password", "correct horse"];
    let taken = run_in(&other, &daemon.url, &[], &register).await;
    assert_ne!(taken.code, 0, "{}", taken.out);
    assert!(taken.err.contains("already has an owner"), "{}", taken.err);
    assert!(
        run_in(&other, &daemon.url, &[], &["whoami"]).await.code != 0,
        "the refused terminal is signed in as nobody"
    );
    // And she signs in by her password from a terminal that holds nothing.
    let login = ["login", "--email", "ada@example.com", "--password", "correct horse"];
    assert_eq!(run_in(&other, &daemon.url, &[], &login).await.ok().trim(), "Signed in as Ada.");
    let _ = std::fs::remove_dir_all(other);
    let _ = std::fs::remove_dir_all(elsewhere);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_terminal_signed_in_as_nobody_is_signed_in_as_the_owner_by_this_machine() {
    let daemon = Daemon::start("serve").await;
    daemon.run(&["logout"]).await.ok();
    let data = daemon.dir.join("data");
    let data = data.to_str().unwrap();
    // Signed in as nobody and told only the address, it proves the folder the
    // daemon says it keeps its data in, and is given the owner's link.
    let link = daemon.run(&["open", "--link"]).await.ok().trim().to_string();
    assert_eq!(spend(&daemon.url, &link).await.0, 200, "{link}");
    // Told a folder that is not the one served here, the same.
    let elsewhere = scratch("opened-not-served");
    let wrong = elsewhere.to_str().unwrap();
    let link = daemon.run(&["open", "--link", "--data", wrong]).await.ok().trim().to_string();
    assert_eq!(spend(&daemon.url, &link).await.0, 200, "{link}");
    let _ = std::fs::remove_dir_all(elsewhere);
    // And the terminal itself is signed in the same way.
    assert_eq!(daemon.run(&["login", "--here", "--data", data]).await.ok().trim(), "Signed in as Unnamed.");
    daemon.run(&["logout"]).await.ok();
    assert_eq!(daemon.run(&["login", "--here"]).await.ok().trim(), "Signed in as Unnamed.");
    assert!(daemon.run(&["whoami"]).await.ok().starts_with("Unnamed"));
}
