//! `record`: the records of a collection a blueprint declared, from the terminal.

use crate::Rig;

/// Sprints, and a board whose cards may each be in one.
const PLAN: &str = r#"blueprint: plan
permissions: [projects, board]
steps:
  sprints:
    do: collection.ensure
    with:
      name: sprints
      fields:
        title: { type: text, required: true }
        size: { type: number }
        open: { type: flag }
        goals: { type: list }
        code: { type: text }
      history:
        record.commented: "{actor} said: {body}"
        record.*: ""
  board:
    do: collection.ensure
    with:
      name: board
      fields:
        title: { type: text, required: true }
        body: { type: text }
        status: { type: choice, of: [todo, done], required: true }
        labels: { type: list }
        sprint: { type: record, of: sprints }
      views:
        board: { kind: board, by: status }
"#;

#[tokio::test]
async fn records_are_listed_added_shown_changed_and_taken_away_from_the_terminal() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    let file = rig.dir.join("plan.bp");
    std::fs::write(&file, PLAN).unwrap();
    let made = rig.run(&["apply", file.to_str().unwrap(), "--project", "shop"]).await.ok();
    assert!(made.contains(" ok in "), "{made}");

    assert_eq!(
        rig.run(&["record", "list", "shop", "sprints"]).await.ok().trim(),
        "`sprints` has no records yet"
    );
    // Each value is made what its field keeps: a number, a flag, a list; and `42` in words stays words.
    let added = rig
        .run(&[
            "record",
            "add",
            "shop",
            "sprints",
            "title=November",
            "size=5",
            "open=true",
            "goals=ship, rest",
            "code=42",
        ])
        .await
        .ok();
    assert_eq!(added.trim(), "added shop-sprints-1 to sprints");
    rig.run(&["record", "add", "shop", "sprints", "title=December"]).await.ok();
    let listed = rig.run(&["record", "list", "shop", "sprints"]).await.ok();
    assert_eq!(
        listed.trim(),
        "shop-sprints-1  code 42  goals ship, rest  open true  size 5  title November\nshop-sprints-2  title December"
    );
    let json = rig.run(&["record", "show", "shop-sprints-1", "--json"]).await.ok();
    let read: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        read["data"],
        serde_json::json!({ "title": "November", "size": 5, "open": true, "goals": ["ship", "rest"], "code": "42" })
    );

    let changed = rig.run(&["record", "set", "shop-sprints-1", "size=8", "code=", "open=false"]).await.ok();
    assert_eq!(changed.trim(), "changed shop-sprints-1");
    let shown = rig.run(&["record", "show", "shop-sprints-1"]).await.ok();
    assert_eq!(
        shown.trim(),
        "shop-sprints-1  of sprints\n  goals: ship, rest\n  open: false\n  size: 8\n  title: November"
    );

    // What is not so is said in the daemon's words, and nothing is changed.
    let wrong = rig.run(&["record", "set", "shop-sprints-1", "size=big"]).await;
    assert!(wrong.code != 0 && wrong.err.contains("`size` must be a number"), "{}", wrong.err);
    let stray = rig.run(&["record", "add", "shop", "sprints", "title=January", "goal=ship"]).await;
    assert!(stray.err.contains("`goal` is not a field of `sprints`"), "{}", stray.err);
    let bare = rig.run(&["record", "add", "shop", "sprints", "January"]).await;
    assert!(bare.err.contains("`January` should be `field=value`, as in `title=November`"), "{}", bare.err);
    let none = rig.run(&["record", "add", "shop", "sprints"]).await;
    assert!(none.err.contains("missing what it holds"), "{}", none.err);
    let card = rig.run(&["record", "add", "shop", "board", "title=Till", "status=todo"]).await;
    assert!(card.err.contains("Use `board.add`, `board.update` and `board.move`"), "{}", card.err);
    let unknown = rig.run(&["record", "rename", "shop-sprints-1"]).await;
    assert!(
        unknown
            .err
            .contains("try list, add, show, set, comment, attach, attachments, detach, title or remove"),
        "{}",
        unknown.err
    );
    assert!(rig.run(&["record"]).await.err.contains("record list <project> <collection>"));

    // Something is said on a sprint, and is kept in its history, which is
    // read as a card's is.
    let mut written =
        rig.node.as_ref().unwrap().app.journal.follow_every(Default::default(), "collection.history");
    let said = rig.run(&["record", "comment", "shop-sprints-1", "Two", "weeks", "is", "too", "short."]).await;
    assert_eq!(said.ok().trim(), "said on shop-sprints-1");
    crate::soon("what was said to be in the sprint's history", async {
        let last = "Ada said: Two weeks is too short.";
        while !rig.run(&["board", "history", "shop-sprints-1"]).await.ok().trim_end().ends_with(last) {
            written.next().await.unwrap();
        }
    })
    .await;
    let nothing = rig.run(&["record", "comment", "shop-sprints-1"]).await;
    assert!(nothing.code != 0 && nothing.err.contains("missing what to say"), "{}", nothing.err);
    // What is said is private unless it is said with `--public`, which
    // marks it and is no word of it; the history says which it was. A word
    // with one dash is a word, as it was, and so is anything after `--`.
    let said = rig.run(&["record", "comment", "shop-sprints-1", "--public", "Three", "it", "is."]).await;
    assert_eq!(said.ok().trim(), "said publicly on shop-sprints-1");
    let said = rig.run(&["record", "comment", "shop-sprints-1", "-n", "is", "no", "flag"]).await;
    assert_eq!(said.ok().trim(), "said on shop-sprints-1");
    let said =
        rig.run(&["record", "comment", "shop-sprints-1", "--", "say", "--public", "to", "mark", "it"]).await;
    assert_eq!(said.ok().trim(), "said on shop-sprints-1");
    let alone = rig.run(&["record", "comment", "shop-sprints-1", "--public"]).await;
    assert!(alone.code != 0 && alone.err.contains("missing what to say"), "{}", alone.err);
    let kept = crate::soon("each of them to be in the sprint's history", async {
        loop {
            let kept = rig.run(&["board", "history", "shop-sprints-1"]).await.ok();
            if kept.trim_end().ends_with("Ada said: say --public to mark it") {
                return kept;
            }
            written.next().await.unwrap();
        }
    })
    .await;
    let lines: Vec<&str> =
        kept.lines().filter_map(|line| line.split_once("  ").map(|(_, said)| said)).collect();
    let last = &lines[lines.len() - 4..];
    assert_eq!(
        last,
        [
            "Ada said: Two weeks is too short.",
            "Ada said: Three it is. (public)",
            "Ada said: -n is no flag",
            "Ada said: say --public to mark it",
        ],
        "{kept}"
    );
    assert!(rig.run(&["help", "record"]).await.ok().contains(
        "record comment <record> [--public] [--on <quote> [--before W] [--after W] [--under H]...] <words...>"
    ));
    // Said with a key somebody made, it is the key that said it: the
    // history names whose key and which, in words.
    let made = rig.run(&["key", "notes", "--scope", "records"]).await.ok();
    let token = made.lines().next().unwrap().to_string();
    let script =
        ["record", "comment", "shop-sprints-1", "--public", "From", "a", "script.", "--token", &token];
    assert_eq!(rig.run(&script).await.ok().trim(), "said publicly on shop-sprints-1");
    crate::soon("what the key said to be in the sprint's history", async {
        let last = "Ada (with the key notes) said: From a script. (public)";
        while !rig.run(&["board", "history", "shop-sprints-1"]).await.ok().trim_end().ends_with(last) {
            written.next().await.unwrap();
        }
    })
    .await;
    // About a few words of it, with those either side and the headings over them: kept with the line.
    let on = [
        "record",
        "comment",
        "shop-sprints-1",
        "--on",
        "too short",
        "--before",
        "is ",
        "--under",
        "Plan",
        "--under",
        "Weeks",
        "Is",
        "it?",
    ];
    assert_eq!(rig.run(&on).await.ok().trim(), "said on shop-sprints-1");
    let told = crate::soon("what was said on a few words to be in the sprint's history", async {
        loop {
            let told = rig.run(&["board", "history", "shop-sprints-1", "--json"]).await.ok();
            if told.contains("Is it?") {
                return told;
            }
            written.next().await.unwrap();
        }
    })
    .await;
    let lines: serde_json::Value = serde_json::from_str(&told).unwrap();
    let line = lines["history"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(
        line["data"]["anchor"],
        serde_json::json!({ "quote": "too short", "before": "is ", "under": ["Plan", "Weeks"] }),
        "{line}"
    );
    let long = "x".repeat(401);
    let refused = rig.run(&["record", "comment", "shop-sprints-1", "--on", &long, "Hm."]).await;
    assert!(refused.code != 0 && refused.err.contains("400 characters at most"), "{}", refused.err);
    // This board's blueprint keeps no history of its cards, so what was
    // said on one would be kept nowhere: it is refused, with what to do.
    rig.run(&["board", "add", "shop", "Till"]).await.ok();
    let unkept = rig.run(&["record", "comment", "shop-1", "When?"]).await;
    assert!(
        unkept.code != 0
            && unkept.err.contains("what is said on a record of `board` would be kept nowhere")
            && unkept.err.contains("apply its blueprint again"),
        "{}",
        unkept.err
    );
    // And there is one way to say something, whatever it is said on.
    let old = rig.run(&["board", "comment", "shop-1", "When?"]).await;
    assert!(old.code != 0 && old.err.contains("record comment <card> <words...>"), "{}", old.err);

    // A card is in the sprint; the sprint is taken away, and the card still moves.
    rig.run(&["board", "add", "shop", "Basket"]).await.ok();
    let gone = rig.run(&["record", "remove", "shop-sprints-2"]).await.ok();
    assert_eq!(gone.trim(), "took shop-sprints-2 away");
    assert_eq!(
        rig.run(&["record", "list", "shop", "sprints"]).await.ok().lines().count(),
        1,
        "the other is as it was"
    );
    assert!(
        rig.run(&["record", "show", "shop-sprints-2"])
            .await
            .err
            .contains("there is no record `shop-sprints-2`")
    );
    assert!(rig.run(&["board", "move", "shop-2", "done"]).await.ok().contains("shop-2 is now in done"));
    assert!(rig.run(&["help"]).await.ok().contains("record list <project> <collection> | add ..."));
    let help = rig.run(&["help", "record"]).await.ok();
    assert!(help.contains("record add shop sprints title=November size=5"), "{help}");
}

#[tokio::test]
async fn what_a_card_holds_is_set_from_the_terminal_and_where_it_is_and_whose_it_is_are_not() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    let file = rig.dir.join("plan.bp");
    std::fs::write(&file, PLAN).unwrap();
    rig.run(&["apply", file.to_str().unwrap(), "--project", "shop"]).await.ok();
    rig.run(&["record", "add", "shop", "sprints", "title=November"]).await.ok();
    let added = rig.run(&["board", "add", "shop", "A", "till", "--status", "todo"]).await.ok();
    assert_eq!(added.trim(), "added shop-1 to todo: A till");
    let held = async || {
        let json = rig.run(&["record", "show", "shop-1", "--json"]).await.ok();
        serde_json::from_str::<serde_json::Value>(&json).unwrap()["data"].clone()
    };

    // A card's own, and what its board keeps of a card beyond that, each
    // made what its field keeps.
    let set =
        ["title=A till that opens", "body=With a key.", "labels=front, urgent", "sprint=shop-sprints-1"];
    let changed = rig.run(&[&["board", "set", "shop-1"][..], &set[..]].concat()).await.ok();
    assert_eq!(changed.trim(), "changed shop-1");
    let now = held().await;
    assert_eq!(now["title"], "A till that opens");
    assert_eq!(now["body"], "With a key.");
    assert_eq!(now["labels"], serde_json::json!(["front", "urgent"]));
    assert_eq!(now["sprint"], "shop-sprints-1");
    assert_eq!(now["status"], "todo", "and it is where it was");

    // With nothing after the `=`, a field is cleared.
    rig.run(&["board", "set", "shop-1", "sprint=", "labels=", "body="]).await.ok();
    let now = held().await;
    assert!(now["sprint"].is_null(), "{now}");
    assert!(now["labels"].as_array().is_none_or(Vec::is_empty), "{now}");
    assert!(now["body"].as_str().is_none_or(str::is_empty), "{now}");
    assert_eq!(now["title"], "A till that opens");

    // Where a card is and whose it is are events of their own, and each
    // has its command: neither is set as a field, and nothing is changed.
    let moved = rig.run(&["board", "set", "shop-1", "status=done"]).await;
    assert!(moved.code != 0 && moved.err.contains("board move shop-1 <column>"), "{}", moved.err);
    let given = rig.run(&["board", "set", "shop-1", "title=Another", "assignee=agent:till"]).await;
    assert!(
        given.code != 0 && given.err.contains("board give shop-1 <agent>|<person>|nobody"),
        "{}",
        given.err
    );
    let unnamed = rig.run(&["board", "set", "shop-1", "title="]).await;
    assert!(unnamed.err.contains("a card is called something"), "{}", unnamed.err);
    let stray = rig.run(&["board", "set", "shop-1", "colour=red"]).await;
    assert!(stray.code != 0 && stray.err.contains("colour"), "{}", stray.err);
    let bare = rig.run(&["board", "set", "shop-1", "urgent"]).await;
    assert!(bare.err.contains("`urgent` should be `field=value`"), "{}", bare.err);
    let none = rig.run(&["board", "set", "shop-1"]).await;
    assert!(none.err.contains("missing what it holds"), "{}", none.err);
    let now = held().await;
    assert_eq!(
        (&now["title"], &now["status"]),
        (&serde_json::json!("A till that opens"), &serde_json::json!("todo"))
    );
    assert!(now["assignee"].is_null(), "{now}");
}

#[tokio::test]
async fn a_card_added_or_changed_from_the_terminal_with_a_key_is_said_to_be_the_keys_doing() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    rig.run(&["apply", "std/board", "--project", "shop"]).await.ok();
    let made = rig.run(&["key", "cards", "--scope", "board"]).await.ok();
    let token = made.lines().next().unwrap().to_string();

    // With the key, and then as whoever is signed in.
    let mut written =
        rig.node.as_ref().unwrap().app.journal.follow_every(Default::default(), "collection.history");
    rig.run(&["board", "add", "shop", "Sell mugs", "--token", &token]).await.ok();
    rig.run(&["board", "set", "shop-1", "labels=out", "--token", &token]).await.ok();
    rig.run(&["board", "set", "shop-1", "labels=out,soon"]).await.ok();
    let kept = crate::soon("the card's history", async {
        loop {
            let kept = rig.run(&["board", "history", "shop-1"]).await.ok();
            if kept.lines().count() >= 3 {
                return kept;
            }
            written.next().await.unwrap();
        }
    })
    .await;
    let lines: Vec<&str> =
        kept.lines().filter_map(|line| line.split_once("  ").map(|(_, said)| said)).collect();
    assert_eq!(
        lines,
        [
            "Ada (with the key cards) added it to backlog",
            "Ada (with the key cards) changed its labels",
            "Ada changed its labels",
        ],
        "{kept}"
    );
}

#[tokio::test]
async fn one_thing_of_a_list_that_holds_a_comma_is_said_whole_with_a_plus() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    let file = rig.dir.join("plan.bp");
    std::fs::write(&file, PLAN).unwrap();
    rig.run(&["apply", file.to_str().unwrap(), "--project", "shop"]).await.ok();
    let goals = async |id: &str| {
        let json = rig.run(&["record", "show", id, "--json"]).await.ok();
        serde_json::from_str::<serde_json::Value>(&json).unwrap()["data"]["goals"].clone()
    };

    // `=` is the things between commas, as it always was; `+=` is one thing more, as it is written.
    let said = ["title=November", "goals=ship, rest", "goals+=Look at it, then say"];
    rig.run(&[&["record", "add", "shop", "sprints"][..], &said[..]].concat()).await.ok();
    assert_eq!(goals("shop-sprints-1").await, serde_json::json!(["ship", "rest", "Look at it, then say"]));
    rig.run(&["record", "add", "shop", "sprints", "title=December", "goals+=a, b", "goals+=c"]).await.ok();
    assert_eq!(goals("shop-sprints-2").await, serde_json::json!(["a, b", "c"]));

    // Said of a record that is there, it adds to what the record holds; after `=`, to what that said.
    rig.run(&["record", "set", "shop-sprints-2", "goals+=d, e"]).await.ok();
    assert_eq!(goals("shop-sprints-2").await, serde_json::json!(["a, b", "c", "d, e"]));
    rig.run(&["record", "set", "shop-sprints-2", "goals=", "goals+=only, this"]).await.ok();
    assert_eq!(goals("shop-sprints-2").await, serde_json::json!(["only, this"]));
    let unlisted = rig.run(&["record", "set", "shop-sprints-2", "title+=, again"]).await;
    assert!(unlisted.code != 0 && unlisted.err.contains("`title` is no list"), "{}", unlisted.err);

    // A card's labels the same way, and what its board keeps beside them.
    rig.run(&["board", "add", "shop", "A", "till", "--status", "todo", "--label", "front"]).await.ok();
    rig.run(&["board", "set", "shop-1", "labels+=back, then front"]).await.ok();
    let json = rig.run(&["record", "show", "shop-1", "--json"]).await.ok();
    let card = serde_json::from_str::<serde_json::Value>(&json).unwrap()["data"].clone();
    assert_eq!(card["labels"], serde_json::json!(["front", "back, then front"]), "{card}");

    let help = rig.run(&["help", "record"]).await.ok();
    assert!(help.contains("goals+="), "{help}");
}

#[tokio::test]
async fn a_file_is_attached_to_a_card_listed_read_back_and_taken_away_from_the_terminal_as_whoever_may() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    let file = rig.dir.join("plan.bp");
    std::fs::write(&file, PLAN).unwrap();
    rig.run(&["apply", file.to_str().unwrap(), "--project", "shop"]).await.ok();
    rig.run(&["record", "add", "shop", "sprints", "title=November"]).await.ok();
    let log = rig.dir.join("tests.log");
    std::fs::write(&log, "12 passed\n").unwrap();

    let attached = rig.run(&["record", "attach", "shop-sprints-1", log.to_str().unwrap()]).await.ok();
    assert!(attached.starts_with("attached tests.log (10 bytes) to shop-sprints-1 as ratt_"), "{attached}");
    let id = attached.trim().rsplit(' ').next().unwrap().to_string();
    let listed = rig.run(&["record", "attachments", "shop-sprints-1"]).await.ok();
    assert!(listed.contains(&id) && listed.contains("tests.log") && listed.contains("10 bytes"), "{listed}");
    // Read back to standard output as it is, and into a file.
    assert_eq!(rig.run(&["attachment", &id]).await.ok(), "12 passed\n");
    let out = rig.dir.join("read.log");
    let wrote = rig.run(&["attachment", &id, "--out", out.to_str().unwrap()]).await.ok();
    assert!(wrote.contains("wrote 10 bytes"), "{wrote}");
    assert_eq!(std::fs::read_to_string(&out).unwrap(), "12 passed\n");

    // A key that may read records reads it, and attaches nothing.
    let key = rig.run(&["key", "ci", "--scope", "records:read"]).await.ok();
    let key = key.split_whitespace().find(|word| word.starts_with("key_")).unwrap().to_string();
    let token = brand::env_var("TOKEN");
    let read = rig.run_with(&[(token.as_str(), key.clone())], &["attachment", &id]).await.ok();
    assert_eq!(read, "12 passed\n");
    let refused = rig
        .run_with(&[(token.as_str(), key)], &["record", "attach", "shop-sprints-1", log.to_str().unwrap()])
        .await;
    assert!(refused.code != 0 && refused.err.contains("records:write"), "{}", refused.err);

    let gone = rig.run(&["record", "detach", "shop-sprints-1", &id]).await.ok();
    assert!(gone.starts_with("moved tests.log of shop-sprints-1 to the Trash as `trash_"), "{gone}");
    let trash = gone.split('`').nth(1).unwrap().to_string();
    let listed = rig.run(&["record", "attachments", "shop-sprints-1"]).await.ok();
    assert_eq!(listed.trim(), "nothing is attached to shop-sprints-1");
    let missing = rig.run(&["attachment", &id]).await;
    assert!(missing.code != 0, "{}", missing.out);
    // Given back from the Trash, whole.
    rig.run(&["trash", "restore", &trash]).await.ok();
    assert_eq!(rig.run(&["attachment", &id]).await.ok(), "12 passed\n");
}

#[tokio::test]
async fn what_is_attached_to_a_message_is_read_through_the_command_as_whoever_may_read_the_channel() {
    let rig = Rig::signed_in().await;
    rig.run(&["channel", "create", "general"]).await.ok();
    let plan = rig.dir.join("plan.txt");
    std::fs::write(&plan, "the plan").unwrap();
    rig.run(&["post", "general", "here", "--attach", plan.to_str().unwrap()]).await.ok();
    let read: serde_json::Value =
        serde_json::from_str(&rig.run(&["read", "general", "--json"]).await.ok()).unwrap();
    let id = read["messages"][0]["attachments"][0]["id"].as_str().unwrap().to_string();
    assert_eq!(rig.run(&["attachment", &id]).await.ok(), "the plan");
    let key = rig.run(&["key", "ci", "--scope", "records:read"]).await.ok();
    let key = key.split_whitespace().find(|word| word.starts_with("key_")).unwrap().to_string();
    let refused = rig.run_with(&[(brand::env_var("TOKEN").as_str(), key)], &["attachment", &id]).await;
    assert!(refused.code != 0 && refused.err.contains("channels:read"), "{}", refused.err);
}
