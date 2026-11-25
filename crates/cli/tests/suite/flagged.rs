//! A flag is taken only by a command that reads it (`crate::args` of the
//! command line). A program from before a switch was there answered as
//! though all were well, having done what it was asked without it.

use crate::Rig;

/// The flags every command takes, as the refusal says them.
fn everywhere() -> &'static str {
    "(`--url`, `--profile`, `--token`, `--as`, `--json`, `--workers` and `--help`)"
}

#[tokio::test(flavor = "multi_thread")]
async fn a_flag_a_command_does_not_read_is_refused_before_anything_is_sent_naming_it_and_the_command() {
    let rig = Rig::signed_in().await;
    let cli = brand::CLI;

    let status = rig.run(&["status", "--bogus"]).await;
    assert_eq!(status.code, 2, "{}", status.out);
    assert_eq!(
        status.err.trim(),
        format!(
            "error: `{cli} status` takes no `--bogus`, so nothing was sent: it takes only the flags every \
             command takes {}. `{cli} help status` says what it does",
            everywhere()
        )
    );
    assert!(status.out.is_empty(), "nothing is printed of a status that was never asked: {}", status.out);

    // A command that sends something sends nothing: the agent is not added.
    let added = rig.run(&["agent", "add", "helper", "--bogus", "--surely-not"]).await;
    assert_eq!(added.code, 2, "{}", added.out);
    assert!(
        added.err.contains(&format!(
            "`{cli} agent add` takes no `--bogus` or `--surely-not`, so nothing was sent"
        )) && added.err.contains("`--web`")
            && added.err.contains(&format!("besides the flags every command takes {}", everywhere())),
        "it says which flags it takes: {}",
        added.err
    );
    let agents = rig.run(&["agent", "list"]).await.ok();
    assert!(!agents.contains("helper"), "{agents}");

    let made = rig.run(&["project", "create", "--bare", "Shop", "--nope"]).await;
    assert_eq!(made.code, 2, "{}", made.err);
    assert!(made.err.contains(&format!("`{cli} project create` takes no `--nope`")), "{}", made.err);
    assert!(!rig.run(&["project", "list"]).await.ok().contains("shop"));

    // A flag another word of the same command reads is no flag of this one.
    let listed = rig.run(&["channel", "list", "--kept"]).await;
    assert_eq!(listed.code, 2, "{}", listed.out);
    assert!(listed.err.contains(&format!("`{cli} channel list` takes no `--kept`")), "{}", listed.err);

    // And one that talks to no daemon refuses before it does anything.
    let laid = rig.run(&["fmt", "--chek", rig.dir.to_str().unwrap()]).await;
    assert_eq!(laid.code, 2, "{}", laid.out);
    assert!(
        laid.err.contains(&format!("`{cli} fmt` takes no `--chek`, so nothing was sent: it takes `--check`"))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_flags_every_command_takes_are_taken_by_each() {
    let rig = Rig::signed_in().await;
    let status = rig.run(&["status", "--json", "--url", &rig.url, "--workers", "2"]).await.ok();
    assert!(status.trim_start().starts_with('{'), "{status}");
    rig.run(&["whoami", "--json", "--workers", "2"]).await.ok();
    rig.run(&["project", "list", "--url", &rig.url]).await.ok();
    rig.run(&["version", "--json"]).await.ok();
    let help = rig.run(&["agent", "--help"]).await.ok();
    assert!(help.contains("agent add"), "{help}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_flag_a_word_after_the_command_reads_is_taken() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    rig.run(&["apply", "std/board", "--project", "shop"]).await.ok();
    // `channel create` reads `--kept`, as `channel list` does not.
    let kept = rig.run(&["channel", "create", "yard", "--kept"]).await.ok();
    assert!(kept.starts_with("created"), "{kept}");
    // `board add` reads `--status` and `--body`.
    let card = rig
        .run(&["board", "add", "shop", "Add", "a", "cart", "--status", "ready", "--body", "One."])
        .await
        .ok();
    assert!(card.contains("to ready: Add a cart"), "{card}");
}
