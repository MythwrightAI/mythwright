//! `sandbox`: a blueprint run where nothing is real, from a terminal,
//! with the real program.

use crate::{Rig, STANDIN};

#[tokio::test(flavor = "multi_thread")]
async fn a_blueprint_that_ships_is_run_in_a_sandbox_from_a_terminal_and_nothing_is_applied_in_the_factory() {
    let rig = Rig::signed_in().await;

    let board = rig.run(&["sandbox", "std/board"]).await.ok();

    assert!(
        board.starts_with(
            "A sandbox test of std/board: it is run in a factory of its own, with one made-up project called \
             `sandbox`, and nothing here is real.\n"
        ),
        "{board}"
    );
    for line in [
        // What was started, each run under its own heading, each step by
        // what it says of itself with its key and what it does.
        "  + started std/board (`std/board`)\n",
        // What the table's action said for its step is under the run
        // it was said for, before the step.
        "\nstd/board (`std/board`): its steps\n    said `collection.ensured` ",
        "  ok         The table a card is a row of, and the ways it is laid out; the board is the rows by their \
         status. [board]  (do collection.ensure)\n",
        "  + started std/cards (`cards`)\n",
        "  = std/board (`std/board`) is up: its steps went well and it stays, listening\n",
        "\nstd/cards (`cards`): handler `first`\n  heard `instance.active`\n",
        // What it ended with, and what was stood in: nothing.
        "\ntable `board`: 0 rows\n",
        "\nNothing was stood in: nothing this did reaches outside the factory's tables.\n",
    ] {
        assert!(board.contains(line), "`{line}` in:\n{board}");
    }
    assert!(board.trim_end().ends_with("passing here is not passing for real."), "{board}");
    // Nothing was applied in the factory, and it has no such project.
    assert!(rig.run(&["instances", "--all"]).await.ok().trim().lines().count() <= 1);
    assert!(!rig.run(&["project", "list"]).await.ok().contains("sandbox"));

    // As the daemon hands it: runs as real ones are handed, and the rest.
    let handed = rig.run(&["sandbox", "std/board", "--json"]).await.ok();
    let handed: serde_json::Value = serde_json::from_str(&handed).expect("JSON");
    assert_eq!(handed["runs"][0]["flow"], "main");
    assert_eq!(handed["instances"][0]["project"], "sandbox");
    assert_eq!(handed["tables"][0]["name"], "board");
    assert_eq!(handed["asked"], serde_json::json!([]));
    assert_eq!(handed["inbox"], serde_json::json!([]));

    // The fleet that ships over a board, with a card made ready: nobody
    // is there to take it, since an agent is stood in, and what the
    // fleet attaches to tell somebody of a card left tells whoever
    // applied it, which is said at the foot with the tables.
    let ready = ["--do", "board.add", "title=Try it", "status=ready"];
    let fleet = rig.run(&[&["sandbox", "std/fleet", "--first", "std/board"], &ready[..]].concat()).await.ok();
    for line in [
        "\nstd/fleet (`std/fleet`): its steps\n",
        "  + started std/told (`told`)\n",
        "\ntable `tells`: 1 row\n",
        "\ntold whoever applied it (left): sandbox-1 is left for a person",
        "\nStood in, and so not exercised: `agent.ensure`, `agent.let_go`, `agent.present`, `agent.task`, \
         `place.make`, `system.check`.",
    ] {
        assert!(fleet.contains(line), "`{line}` in:\n{fleet}");
    }
    // And nobody in the factory was told anything.
    assert!(!rig.run(&["inbox", "--all"]).await.ok().contains("left for a person"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_project_s_own_blueprint_that_runs_a_command_is_run_in_a_sandbox_and_the_command_is_not() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "Shop", "--bare"]).await.ok();
    let would = rig.dir.join("would-have-run");
    let text = format!(
        "blueprint: release\ndescription: Build what is there and say so.\npermissions: [shell]\nsteps:\n  build:\n    description: Build it. The rest is not said.\n    do: shell\n    with: {{ cmd: ['{STANDIN}', write, '{}', ran] }}\n  note: {{ do: set, with: {{ code: \"${{steps.build.out.code}}\" }} }}\n  pause: {{ do: wait, with: {{ for: 10m }} }}\n  after: {{ do: set, with: {{ waited: true }} }}\nstate:\n  code: -1\n  waited: false\n  asked: nothing\non:\n  asked:\n    event: release.asked\n    steps:\n      keep: {{ do: set, with: {{ asked: \"${{event.data.version}}\" }} }}\n",
        would.display()
    );
    let file = rig.placed("shop").join("release.bp");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, text).unwrap();

    // By its name, as an apply finds it, with what is done after it in
    // the order it is asked: the clock moved on, and then an event said,
    // which the text hears once its steps have ended.
    let ran = rig
        .run(&[
            "sandbox",
            "release",
            "--project",
            "shop",
            "--advance",
            "10m",
            "--say",
            "release.asked",
            "version=2",
        ])
        .await
        .ok();

    for line in [
        "\nrelease (`release`): its steps\n  ok         Build it. [build]  (do shell)\n             stood in: `shell` was asked",
        "and answered {\"code\":0,\"duration_ms\":0,\"stderr\":\"\",\"stdout\":\"\"} (what a stand-in for it answers)\n",
        "  waits      [pause] for the clock\n",
        "  ok         after  (do set)\n",
        "  = release (`release`) is up: its steps went well and it stays, listening\n",
        "    said `release.asked` {\"version\":2}\n",
        "\nrelease (`release`): handler `asked`\n  heard `release.asked`\n  ok         keep  (do set)\n",
        "\nStood in, and so not exercised: `shell`. Asked once in all, they ran nothing, and answered what a \
         stand-in was told to answer.\n",
    ] {
        assert!(ran.contains(line), "`{line}` in:\n{ran}");
    }
    // In that order: the wait, what the clock let go on, and then what was said.
    let at = |part: &str| ran.find(part).unwrap_or_else(|| panic!("`{part}` in:\n{ran}"));
    assert!(
        at("for the clock") < at("after  (do set)") && at("after  (do set)") < at("said `release.asked`")
    );
    // The command was not run.
    assert!(!would.exists(), "the command was run: {} is there", would.display());

    // Told what a stand-in answers, that is what its step gives; and one
    // that fails for it fails the command, saying so.
    let failing =
        rig.run(&["sandbox", "release", "--project", "shop", "--tell", "shell", "code=3"]).await.ok();
    assert!(failing.contains("and answered {\"code\":3} (as it was told)"), "{failing}");
    let json = rig
        .run(&["sandbox", "release", "--project", "shop", "--tell", "shell", "code=3", "--json"])
        .await
        .ok();
    let json: serde_json::Value = serde_json::from_str(&json).expect("JSON");
    assert_eq!(json["instances"][0]["state"]["code"], 3);
    assert!(!would.exists());
    // Nothing of it was applied in the project.
    assert!(rig.run(&["instances", "--all"]).await.ok().trim().lines().count() <= 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_sandbox_test_of_what_fails_does_not_check_or_is_not_there_fails_saying_which() {
    let rig = Rig::signed_in().await;

    // One whose steps fail is printed, and the command ends with an error.
    let breaks = rig.dir.join("breaks.bp");
    std::fs::write(&breaks, "blueprint: breaks\nsteps:\n  broken: { do: fail, with: { message: it is broken } }\n  never: { do: noop }\n").unwrap();
    let failed = rig.run(&["sandbox", breaks.to_str().unwrap()]).await;
    assert_eq!(failed.code, 1, "{}", failed.out);
    assert!(
        failed.out.contains("  failed     broken  (do fail)\n             why: it is broken\n"),
        "{}",
        failed.out
    );
    assert!(
        failed.out.contains("  ! breaks (`breaks`) failed: step `broken` failed: it is broken\n"),
        "{}",
        failed.out
    );
    assert!(
        failed.err.contains("something failed in the sandbox: what, and why, is said above"),
        "{}",
        failed.err
    );

    // One that does not check is said as `check` says it, and none is made.
    let wrong = rig.dir.join("wrong.bp");
    std::fs::write(&wrong, "blueprint: wrong\nsteps:\n  a: { do: shel }\n").unwrap();
    let wrong = wrong.to_str().unwrap();
    let refused = rig.run(&["sandbox", wrong]).await;
    assert_eq!(refused.code, 1);
    assert!(
        refused.out.contains(&format!("{wrong}:3:3: error: there is no action called `shel`")),
        "{}",
        refused.out
    );
    assert!(refused.err.contains("does not check, so it was not run in a sandbox"), "{}", refused.err);

    // A name nothing has, nothing said, and something that is not done by hand.
    let nobody = rig.run(&["sandbox", "shop/no-such"]).await;
    assert!(nobody.err.contains("there is no blueprint called `shop/no-such`"), "{}", nobody.err);
    let nothing = rig.run(&["sandbox"]).await;
    assert!(nothing.err.contains("missing the blueprint to run in a sandbox"), "{}", nothing.err);
    let other = rig.run(&["sandbox", "std/board", "--do", "fs.write", "path=x", "--advance", "1m"]).await;
    assert_eq!(other.code, 1);
    assert!(
        other.err.contains("`fs.write` is not something done to a sandbox's tables by hand"),
        "{}",
        other.err
    );
    assert!(other.err.contains("What was asked after that was not done"), "{}", other.err);
    // And there is help on it.
    let help = rig.run(&["help", "sandbox"]).await.ok();
    assert!(help.contains("Passing here is not passing for real"), "{help}");
}
