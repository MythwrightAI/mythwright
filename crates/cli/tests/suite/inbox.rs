//! One's own inbox from a terminal, with the real program: a person,
//! and an agent the factory does not start asked as with `--as`, each
//! reading what names them, saying it was read and putting it away, and
//! neither reading the other's.

use crate::external::{handed, invitation};
use crate::fleets::{event_after, now};
use crate::{Rig, run_in};

/// The id of the line that says `words`, as a listing printed it.
fn line_of(listed: &str, words: &str) -> String {
    let line = listed.lines().find(|line| line.contains(words));
    line.unwrap_or_else(|| panic!("no line says `{words}`:\n{listed}"))
        .split_whitespace()
        .next()
        .unwrap()
        .to_string()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_person_and_an_agent_from_outside_each_read_their_own_inbox_from_a_terminal() {
    let rig = Rig::signed_in().await;
    let cli = brand::CLI;
    rig.run(&["channel", "create", "general"]).await.ok();
    let empty = rig.run(&["inbox"]).await.ok();
    assert!(empty.starts_with("nothing waits in your inbox"), "{empty}");
    // An agent this factory does not start, joined from this home: it
    // holds what an agent usually does and its own inbox.
    let invited = invitation(&rig.run(&["agent", "invite", "walls"]).await.ok());
    handed(&rig.dir, &rig.url, &["agent", "join"], &invited).await.ok();
    let as_walls =
        async |args: &[&str]| run_in(&rig.dir, &rig.url, &[], &[&["--as", "walls"], args].concat()).await;
    assert!(as_walls(&["inbox"]).await.ok().starts_with("nothing waits in your inbox"));

    // Named by a person, the agent is asked nothing and answered by
    // nobody: the message waits in its box, where it reads it as itself.
    let before = now(&rig);
    rig.run(&["post", "general", "@walls", "hang", "the", "gate"]).await.ok();
    event_after(&rig, before, "inbox.told", |event| event.data["owner"] == "agent:walls").await;
    let theirs = as_walls(&["inbox"]).await.ok();
    assert!(theirs.contains("unread  #general  [Ada] @walls hang the gate"), "{theirs}");
    assert!(theirs.ends_with("1 unread\n"), "{theirs}");
    let gate = line_of(&theirs, "hang the gate");
    // And naming somebody back, it puts a line in theirs.
    as_walls(&["post", "general", "@ada", "the", "gate", "is", "hung"]).await.ok();
    let mine = rig.run(&["inbox"]).await.ok();
    assert!(mine.contains("unread  #general  [walls] @ada the gate is hung"), "{mine}");
    assert!(mine.ends_with("1 unread\n") && !mine.contains("hang the gate"), "{mine}");
    let hung = line_of(&mine, "the gate is hung");

    // Neither reads the other's: a line that is not one's own is one
    // that is not there, to the factory's owner as to the agent.
    for (asked, line) in [
        (rig.run(&["inbox", "read", &gate]).await, &gate),
        (as_walls(&["inbox", "away", &hung]).await, &hung),
    ] {
        assert!(asked.code != 0, "{}", asked.out);
        assert!(asked.err.contains(&format!("there is no line `{line}` in your inbox")), "{}", asked.err);
    }
    assert!(as_walls(&["inbox"]).await.ok().contains("unread  #general"));

    // Read, then put away: listed no longer, and with everything still.
    assert_eq!(rig.run(&["inbox", "read", &hung]).await.ok().trim(), format!("read {hung}; 0 unread"));
    let read = rig.run(&["inbox"]).await.ok();
    assert!(read.contains(&format!("{hung}  read    #general")) && read.ends_with("0 unread\n"), "{read}");
    // Made to wait unread again, it is counted with the rest; and read once more.
    let again = rig.run(&["inbox", "unread", &hung]).await.ok();
    assert_eq!(again.trim(), format!("{hung} waits unread again; 1 unread"));
    assert!(rig.run(&["inbox"]).await.ok().contains(&format!("{hung}  unread  #general")));
    assert_eq!(rig.run(&["inbox", "read", &hung]).await.ok().trim(), format!("read {hung}; 0 unread"));
    assert_eq!(rig.run(&["inbox", "away", &hung]).await.ok().trim(), format!("put {hung} away; 0 unread"));
    assert!(rig.run(&["inbox"]).await.ok().starts_with("nothing waits in your inbox"));
    let all = rig.run(&["inbox", "--all"]).await.ok();
    assert!(all.contains(&format!("{hung}  away    #general  [walls] @ada the gate is hung")), "{all}");
    // Every line at once, and what the daemon answered as it is.
    assert_eq!(as_walls(&["inbox", "read", "--all"]).await.ok().trim(), "1 line read; 0 unread");
    assert_eq!(as_walls(&["inbox", "read", "--all"]).await.ok().trim(), "0 lines read; 0 unread");
    let whole: serde_json::Value = serde_json::from_str(&as_walls(&["inbox", "--json"]).await.ok()).unwrap();
    assert_eq!(
        (&whole["unread"], &whole["lines"][0]["id"]),
        (&serde_json::json!(0), &serde_json::json!(gate))
    );
    assert_eq!(whole["lines"][0]["said"]["body"], "@walls hang the gate");

    // What is asked wrongly says what to ask; a key that was given no
    // box reads none, its maker's included.
    let which = rig.run(&["inbox", "read"]).await;
    assert!(which.code != 0 && which.err.contains("say which line was read"), "{}", which.err);
    let odd = rig.run(&["inbox", "empty"]).await;
    assert!(odd.code != 0 && odd.err.contains(&format!("`{cli} help inbox`")), "{}", odd.err);
    let key = rig.run(&["key", "ci", "--scope", "channels"]).await.ok();
    let key = key.split_whitespace().find(|word| word.starts_with("key_")).unwrap().to_string();
    let refused = rig.run_with(&[(brand::env_var("TOKEN").as_str(), key)], &["inbox", "--all"]).await;
    assert!(refused.code != 0 && refused.err.contains("needs `inbox:read`"), "{}", refused.err);
    assert!(!refused.err.contains("gate") && refused.out.is_empty(), "{}{}", refused.out, refused.err);

    // And the program says what it is.
    let help = rig.run(&["help", "inbox"]).await.ok();
    assert!(help.starts_with(&format!("{cli} inbox [--all]")), "{help}");
    assert!(help.contains(&format!("`{cli} inbox --as <name>`")), "{help}");
    assert!(
        rig.run(&["help"]).await.ok().contains("inbox [--all] | inbox read <line>|--all | inbox away <line>")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_cards_a_told_line_lists_are_passed_and_failed_from_a_terminal() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    rig.run(&["apply", "std/board", "--project", "shop", "--param", r#"columns=["ready","staging","done"]"#])
        .await
        .ok();
    rig.run(&["board", "add", "shop", "Fix", "the", "cart", "--status", "staging"]).await.ok();
    let told = rig.dir.join("told.bp");
    std::fs::write(
        &told,
        "blueprint: told\npermissions: [inbox]\nsteps:\n  told:\n    do: inbox.tell\n    with:\n      to: applier\n      \
         says: The preview is ready.\n      why: preview\n      parts: [{ kind: cards, column: staging }]\n",
    )
    .unwrap();
    rig.run(&["apply", told.to_str().unwrap(), "--project", "shop"]).await.ok();
    let line = line_of(&rig.run(&["inbox"]).await.ok(), "The preview is ready.");
    let parts = rig.run(&["inbox", "parts", &line]).await.ok();
    assert!(
        parts.contains("1. the cards of shop that were in staging as it was told; failed go to ready")
            && parts.contains("shop-1  Fix the cart"),
        "{parts}"
    );
    let why = rig.run(&["inbox", "fail", &line, "shop-1"]).await;
    assert!(why.code != 0 && why.err.contains("is failed here only with why"), "{}", why.err);
    let failed = rig.run(&["inbox", "fail", &line, "shop-1", "--reason", "The total is wrong."]).await.ok();
    assert_eq!(failed.trim(), "shop-1 failed: moved to ready");
    let history = rig.run(&["board", "history", "shop-1"]).await.ok();
    assert!(history.contains("moved it from staging to ready, saying: The total is wrong."), "{history}");
    // Still listed where it is now, and passed or failed there and not from the line.
    let parts = rig.run(&["inbox", "parts", &line]).await.ok();
    assert!(parts.contains("shop-1  Fix the cart  (in ready)"), "{parts}");
    let moved = rig.run(&["inbox", "pass", &line, "shop-1"]).await;
    assert!(moved.code != 0 && moved.err.contains("any more: it is in ready"), "{}", moved.err);
    let none = rig.run(&["inbox", "pass", &line, "shop-9"]).await;
    assert!(none.code != 0 && none.err.contains("no part of"), "{}", none.err);
}
