//! Every action of a sandbox is one of three kinds, said where it is
//! registered.

use serde_json::json;

use crate::Rig;

#[tokio::test]
async fn every_action_of_a_sandbox_says_which_of_the_three_kinds_it_is() {
    let rig = Rig::new("blueprint: quiet\nsteps:\n  ready: { do: noop }\n").await;

    let shown = rig.read().await;

    let kinds = shown["actions"].as_object().unwrap();
    for (name, kind) in kinds {
        assert!(["own", "kept", "stood"].contains(&kind.as_str().unwrap()), "`{name}` is {kind}");
    }
    // A word of the language and a block that works something out.
    for own in ["set", "wait", "emit", "pool.turn", "text.put", "work.turn", "review.tally", "rollout.next"] {
        assert_eq!(kinds[own], "own", "{own}");
    }
    // What keeps rows in the tables.
    for kept in ["collection.ensure", "board.add", "board.move", "record.add", "rows.read", "channel.post"] {
        assert_eq!(kinds[kept], "kept", "{kept}");
    }
    // What the device describes and the sandbox has no code for.
    assert_eq!(kinds["outside.run"], "stood");
    assert_eq!(kinds["outside.ask"], "stood");
    assert_eq!(shown["stands_in_for"], json!(["outside.ask", "outside.run"]));
    // And what reaches a project's files is not among what the studio
    // gives a sandbox: described by nobody here, it is not there at all.
    for outside in ["fs.write", "fs.read", "project.create", "project.scan", "library.export"] {
        assert!(!kinds.contains_key(outside), "`{outside}` is in a sandbox's registry as {}", kinds[outside]);
    }
}

#[tokio::test]
async fn a_table_kept_anywhere_but_in_the_sandbox_s_journal_is_refused() {
    // By a store service, which is an address requests are sent to, and
    // as files of a project, which a sandbox has none of.
    for (store, at, says) in [
        ("service", "at: \"http://127.0.0.1:9\"", "a table kept by a store service is not made here"),
        ("files", "", "a table kept as files is not made here"),
        ("documents", "", "a table kept as the files of a folder is not made here"),
        ("mockups", "", "a table kept as the files of a folder is not made here"),
    ] {
        let text = format!(
            "blueprint: stores\nsteps:\n  table:\n    do: collection.ensure\n    with:\n      name: notes\n      store: {store}\n      {at}\n      fields: {{ title: {{ type: text }} }}\n"
        );
        let rig = Rig::new(&text).await;

        rig.apply().await;

        let shown = rig.read().await;
        let error = shown["runs"][0]["error"].as_str().unwrap_or_default();
        assert!(error.contains(says), "{store}: {shown}");
        assert!(error.contains("this is a sandbox"), "{error}");
        assert_eq!(shown["tables"], json!([]), "{store}");
        // Nothing is in its project's folder but what says whose it is.
        let project = rig.folder.join("projects").join("sandbox");
        let there: Vec<_> =
            std::fs::read_dir(project).unwrap().flatten().map(|entry| entry.file_name()).collect();
        assert_eq!(there, ["SANDBOX.md"], "{store}");
    }
}
