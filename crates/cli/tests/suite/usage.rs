use crate::{Rig, run_in, scratch};

fn fleet_file() -> String {
    brand::blueprint_file("fleet")
}

#[tokio::test(flavor = "multi_thread")]
async fn help_is_short_names_the_product_and_carries_the_blueprint_reference() {
    let dir = scratch("help");
    let overview = run_in(&dir, "http://127.0.0.1:1", &[], &["help"]).await.ok();
    assert!(overview.starts_with(&format!("{}: a software factory", brand::DISPLAY)), "{overview}");
    assert!(overview.contains(&format!("usage: {} <command>", brand::CLI)));
    assert!(
        overview.lines().count() < 70,
        "help is something an agent reads on every fresh start; keep it short"
    );
    assert_eq!(
        run_in(&dir, "http://127.0.0.1:1", &[], &[]).await.ok(),
        overview,
        "no command at all is help"
    );

    let reference = run_in(&dir, "http://127.0.0.1:1", &[], &["help", "blueprints"]).await.ok();
    assert!(
        reference.starts_with("# Blueprints")
            && reference.contains("## Hooks")
            && reference.contains("## Expressions")
    );

    let unknown = run_in(&dir, "http://127.0.0.1:1", &[], &["frobnicate"]).await;
    assert_eq!(unknown.code, 2);
    assert!(unknown.err.contains("`frobnicate` is not a command"), "{}", unknown.err);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_command_with_no_daemon_to_talk_to_says_how_to_start_one() {
    let dir = scratch("nodaemon");
    let ran = run_in(&dir, "http://127.0.0.1:1", &[], &["status"]).await;
    assert_eq!(ran.code, 1);
    assert_eq!(
        ran.err.trim(),
        format!(
            "error: nothing is answering at http://127.0.0.1:1; start the daemon with `{} up`",
            brand::CLI
        )
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_first_run_story_from_a_terminal() {
    let rig = Rig::new().await;
    let cli = brand::CLI;

    let fresh = rig.run(&["status"]).await.ok();
    assert!(fresh.contains("has no owner yet") && fresh.contains(&format!("`{cli} register`")), "{fresh}");
    let welcome = rig
        .run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
        .await
        .ok();
    assert_eq!(welcome.trim(), "Welcome, Ada. You are signed in as the owner.");
    // The session was saved in this rig's home; later commands just work.
    assert!(rig.run(&["whoami"]).await.ok().starts_with("Ada (user:acct_"));

    let created = rig.run(&["project", "create", "--bare", "Riverside", "Shop"]).await.ok();
    assert!(created.starts_with("project `riverside-shop` created at "), "{created}");
    assert!(created.contains("docs/overview.md"));

    let fleet = rig.run(&["fleet", "init", "riverside-shop"]).await.ok();
    assert!(
        fleet.starts_with(&format!("@blueprints/{} is kept for `riverside-shop`.", fleet_file()))
            && fleet.contains(&format!("{cli} apply fleet --project riverside-shop")),
        "it says what to do next: {fleet}"
    );
    let file = rig.placed("riverside-shop").join(fleet_file());
    let file = file.to_str().unwrap();

    assert_eq!(
        rig.run(&["check", file, "--project", "riverside-shop"]).await.ok().trim(),
        format!("{file}: ok")
    );
    let graph = rig.run(&["graph", file]).await.ok();
    assert!(graph.starts_with("fleet: Runs the Riverside Shop factory."), "{graph}");
    for expected in ["steps", "  fleet  call std/fleet"] {
        assert!(graph.contains(expected), "the graph is missing {expected:?}:\n{graph}");
    }

    let applied = rig.run(&["apply", file, "--project", "riverside-shop"]).await.ok();
    assert!(applied.contains("started; run run_") && applied.contains(" ok in "), "{applied}");
    let again = rig.run(&["apply", file, "--project", "riverside-shop"]).await.ok();
    assert!(again.contains("is already running this; nothing to do"), "{again}");

    // The project's fleet, the fleet that ships under it, and what that
    // keeps: its roles, and the pools that say how many cards are worked
    // and reviewed at once.
    let alive = rig.run(&["instances"]).await.ok();
    for one in ["fleet", "std/fleet", "std/arrangement", "std/role", "std/working", "std/pool"] {
        assert!(
            alive.lines().any(|line| line.contains(&format!("  {one} ")) && line.contains("active")),
            "{alive}"
        );
    }
    let agents = rig.run(&["agent", "list"]).await.ok();
    assert!(agents.contains("riverside-shop-architect-1  builtin  riverside-shop  active"), "{agents}");
    assert!(agents.contains("riverside-shop-worker-1     builtin  riverside-shop  active"), "{agents}");

    rig.run(&["post", "riverside-shop", "Morning", "all"]).await.ok();
    let said = rig.run(&["read", "riverside-shop"]).await.ok();
    assert!(
        said.contains("[conductor] The fleet for riverside-shop is up. riverside-shop-architect-1 is here")
            && said.contains("[Ada] Morning all"),
        "{said}"
    );
    // What the fleet says is said by the factory's bot, and which blueprint
    // it was is on the message for whoever asks for all of it.
    let all = rig.run(&["read", "riverside-shop", "--json"]).await.ok();
    assert!(
        all.contains("\"blueprint\": \"std/fleet\"") && all.contains("\"author\": \"bot:conductor\""),
        "{all}"
    );
    let speaker = rig.run(&["channel", "speaker"]).await.ok();
    assert!(speaker.contains("is said by `conductor`"), "{speaker}");
    // Its name is its own: a new agent is not given it, and is told what to do.
    let taken = rig.run(&["agent", "add", "conductor"]).await;
    assert!(taken.err.contains("an agent cannot be called `conductor`"), "{}", taken.err);
    assert!(taken.err.contains("channel speaker <name>"), "{}", taken.err);
    // Spoken to, the bot does a command of this command line's, as whoever asked.
    rig.run(&["post", "riverside-shop", "@conductor", "agent", "list"]).await.ok();
    let listed = crate::fleets::event(&rig, "channel.message", |event| {
        event.data["author"] == "bot:conductor" && event.data["reply_to"].is_string()
    })
    .await;
    let listed = listed.data["body"].as_str().unwrap();
    assert!(listed.contains("- `riverside-shop-worker-1`, worker"), "{listed}");

    // A file goes with a message, and whoever reads the channel is told it is there.
    let report = rig.dir.join("report.txt");
    std::fs::write(&report, "all is well\n").unwrap();
    rig.run(&["post", "riverside-shop", "Today's", "report", "--attach", report.to_str().unwrap()])
        .await
        .ok();
    let said = rig.run(&["read", "riverside-shop"]).await.ok();
    // Named with its id, by which whoever reads the channel reads it, from anywhere.
    let (_, named) = said.split_once("[Ada] Today's report [attached: report.txt (").expect(&said);
    let id = named.split_once(")]").expect(&said).0;
    assert!(said.contains(&format!("{} attachment <id>", brand::CLI)), "{said}");
    assert_eq!(rig.run(&["attachment", id]).await.ok(), "all is well\n");
    let missing = rig.run(&["post", "riverside-shop", "nothing", "--attach", "/no/such/file"]).await;
    assert!(missing.err.contains("cannot read /no/such/file to attach it"), "{}", missing.err);

    // A project has no board until a blueprint makes one, and is told how.
    let none = rig.run(&["board", "riverside-shop"]).await;
    assert!(none.err.contains("has no board yet") && none.err.contains("apply `std/board`"), "{}", none.err);
    let made = rig.run(&["apply", "std/board", "--project", "riverside-shop"]).await.ok();
    assert!(made.contains(" ok in "), "{made}");

    let added = rig
        .run(&["board", "add", "riverside-shop", "Add", "a", "cart", "--body", "Customers need one."])
        .await
        .ok();
    assert_eq!(added.trim(), "added riverside-shop-1 to backlog: Add a cart");
    let board = rig.run(&["board", "riverside-shop"]).await.ok();
    assert!(board.starts_with("backlog (1)\n  riverside-shop-1  Add a cart\nready (0)"), "{board}");

    // A card in review is sent back by moving it to doing, saying what is
    // wrong; what was said is kept with the move, in the card's history.
    let mut written =
        rig.node.as_ref().unwrap().app.journal.follow_every(Default::default(), "collection.history");
    rig.run(&["board", "move", "riverside-shop-1", "review"]).await.ok();
    let sent = rig
        .run(&[
            "board",
            "move",
            "riverside-shop-1",
            "doing",
            "--note",
            "The total is wrong when the cart is empty.",
        ])
        .await
        .ok();
    assert_eq!(sent.trim(), "riverside-shop-1 is now in doing");
    let last = "Ada moved it from review to doing, saying: The total is wrong when the cart is empty.";
    crate::soon("the move to be in the card's history", async {
        // A line is said to have been written when it has been: that is what
        // is waited for. Not for it to be the last: what else is said of a
        // card as it is moved is said a moment after.
        while !rig.run(&["board", "history", "riverside-shop-1"]).await.ok().contains(last) {
            written.next().await.unwrap();
        }
    })
    .await;
    // Something said on the card is kept there too, in order with the rest.
    let said =
        rig.run(&["record", "comment", "riverside-shop-1", "Empty", "carts", "matter", "most."]).await.ok();
    assert_eq!(said.trim(), "said on riverside-shop-1");
    crate::soon("what was said to be in the card's history", async {
        let last = "Ada said: Empty carts matter most.";
        while !rig.run(&["board", "history", "riverside-shop-1"]).await.ok().contains(last) {
            written.next().await.unwrap();
        }
    })
    .await;
    let nothing = rig.run(&["record", "comment", "riverside-shop-1"]).await;
    assert!(nothing.code != 0 && nothing.err.contains("missing what to say"), "{}", nothing.err);
    rig.run(&["board", "move", "riverside-shop-1", "backlog"]).await.ok();
    // A card that waits its turn, or on an answer, says so where the board is read.
    // (What looks after the board's cards puts those there; here a
    // blueprint of the test's does, as it does.)
    let marks = rig.dir.join(brand::blueprint_file("marks"));
    let text = "blueprint: marks\npermissions: [board]\nsteps:\n  mark:\n    do: board.update\n    with:\n      id: riverside-shop-1\n      fields: { waiting: one card is worked at a time here, question: \"Which total?\" }\n";
    std::fs::write(&marks, text).unwrap();
    rig.run(&["apply", marks.to_str().unwrap(), "--project", "riverside-shop"]).await.ok();
    let listed = rig.run(&["board", "riverside-shop"]).await.ok();
    assert!(
        listed.contains(
            "Add a cart  (asks: Which total?)  (waits its turn: one card is worked at a time here)"
        ),
        "{listed}"
    );

    let measured = rig.run(&["metrics", "step", "--by", "action"]).await.ok();
    assert!(
        measured.contains("action=system.check") && measured.lines().next().unwrap().starts_with("what"),
        "{measured}"
    );
    // One project's numbers are the same question with the project named.
    let ours = rig.run(&["metrics", "step", "--by", "action", "--project", "riverside-shop"]).await.ok();
    assert!(ours.contains("action=system.check"), "{ours}");
    let nobodys = rig.run(&["metrics", "step", "--by", "action", "--project", "nonesuch"]).await.ok();
    assert!(!nobodys.contains("action="), "{nobodys}");
    // What happened on the board, the latest of it: the card added, moved
    // about, and what was written on it as it went.
    let happened = rig.run(&["events", "--topics", "board.**", "--limit", "20"]).await.ok();
    assert!(happened.contains("board.item.created"), "{happened}");
    let json = rig.run(&["board", "riverside-shop", "--json"]).await.ok();
    let parsed: serde_json::Value =
        serde_json::from_str(&json).expect("--json prints exactly what the daemon said");
    assert_eq!(parsed["items"][0]["title"], "Add a cart");

    rig.node.as_ref().unwrap().app.studio.search.catch_up().await.unwrap();
    let found = rig.run(&["search", "cart"]).await.ok();
    assert!(found.starts_with("card  Add a cart"), "{found}");

    rig.run(&["logout"]).await.ok();
    let out = rig.run(&["whoami"]).await;
    assert_eq!(out.code, 1);
    assert!(out.err.contains(&format!("`{cli} login`")), "{}", out.err);
}

#[tokio::test(flavor = "multi_thread")]
async fn measurements_are_split_by_a_label_a_blueprint_gave_and_narrowed_to_another() {
    let rig = Rig::signed_in().await;
    let none = rig.run(&["metrics", "--labels"]).await.ok();
    assert!(none.contains("nothing measured in that window carries a label"), "{none}");

    let file = rig.dir.join("trial.bp");
    std::fs::write(
        &file,
        "blueprint: trial\npermissions: []\nsteps:\n  quick:\n    labels: { variant: a }\n    task:\n      \
         work: { do: noop }\n  slow:\n    labels: { variant: b }\n    task:\n      work: { do: noop }\n      \
         more: { do: noop }\n",
    )
    .unwrap();
    let file = file.to_str().unwrap();
    rig.run(&["apply", file, "--label", "experiment=cart"]).await.ok();
    rig.run(&["apply", file, "--label", "experiment=search", "--key", "other"]).await.ok();

    let labels = rig.run(&["metrics", "--labels"]).await.ok();
    assert_eq!(labels, "experiment  cart, search\nvariant  a, b\n");

    let cart = rig.run(&["metrics", "step", "--by", "variant", "--label", "experiment=cart"]).await.ok();
    let rows: Vec<Vec<&str>> = cart.lines().map(|line| line.split_whitespace().collect()).collect();
    assert_eq!(rows[0][..2], ["what", "count"], "{cart}");
    // The two tasks themselves are steps of the blueprint, under no variant.
    assert_eq!(rows[1][..2], ["step", "2"], "{cart}");
    assert_eq!(rows[2][..2], ["variant=a", "1"], "{cart}");
    assert_eq!(rows[3][..2], ["variant=b", "2"], "{cart}");
    let both = rig.run(&["metrics", "step", "--by", "variant"]).await.ok();
    assert!(both.contains("variant=b  4"), "{both}");

    let refused = rig.run(&["apply", file, "--label", "step=one"]).await;
    assert_eq!(refused.code, 1);
    assert!(refused.err.contains("`step` cannot be a label"), "{}", refused.err);
    let unsaid = rig.run(&["apply", file, "--label", "cart"]).await;
    assert!(unsaid.err.contains("`--label cart` should be `name=value`"), "{}", unsaid.err);
}

#[tokio::test(flavor = "multi_thread")]
async fn check_reports_like_a_compiler_and_fails_the_command() {
    let rig = Rig::signed_in().await;
    let file = rig.dir.join("broken.bp");
    std::fs::write(&file, "blueprint: broken\nsteps:\n  build: { do: shel, with: { run: make } }\n  tell: { do: channel.post }\n").unwrap();
    let file = file.to_str().unwrap();

    let ran = rig.run(&["check", file]).await;
    assert_eq!(ran.code, 1);
    // Problems come in the order they appear in the file.
    let lines: Vec<&str> = ran.out.lines().collect();
    assert!(
        lines[0].starts_with(&format!("{file}:1:1: warning: this blueprint declares no `permissions`")),
        "{}",
        lines[0]
    );
    assert_eq!(
        lines[1],
        format!("{file}:3:3: error: there is no action called `shel`; did you mean `shell`?")
    );
    assert!(
        lines[2].starts_with(&format!("{file}:4:3: error: `channel.post` needs `channel`")),
        "{}",
        lines[2]
    );

    // Applying it is refused, and says the same.
    let applied = rig.run(&["apply", file]).await;
    assert_eq!(applied.code, 1);
    assert!(applied.err.contains("there is no action called `shel`"), "{}", applied.err);

    // With no daemon, what can be known from the file alone is still checked.
    let offline = run_in(&rig.dir, "http://127.0.0.1:1", &[], &["check", file]).await;
    assert_eq!(
        offline.code, 0,
        "the file is well formed; the daemon is needed to know `shel` does not exist"
    );
    assert!(offline.err.contains("actions and permissions were not checked"), "{}", offline.err);
    std::fs::write(rig.dir.join("worse.bp"), "blueprint: worse\nsteps: [\n").unwrap();
    let offline =
        run_in(&rig.dir, "http://127.0.0.1:1", &[], &["check", rig.dir.join("worse.bp").to_str().unwrap()])
            .await;
    assert_eq!(offline.code, 1);
    assert!(offline.out.contains("worse.bp:3:1: error: this `[` or `{` is never closed"), "{}", offline.out);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_run_is_shown_step_by_step_and_fails_the_command() {
    let rig = Rig::signed_in().await;
    let file = rig.dir.join("fails.bp");
    // A script either system's shell reads, and then a program that
    // complains and ends with the code it is told.
    let fails = crate::standin::cmd(&["err", "no such table", "exit", "${params.code}"]);
    std::fs::write(&file, format!("blueprint: fails\npermissions: [shell]\nparams: {{ code: 3 }}\nsteps:\n  first: {{ do: shell, with: {{ run: echo hello }} }}\n  bad: {{ do: shell, with: {{ cmd: {fails} }} }}\n  never: {{ do: noop }}\n")).unwrap();
    let ran = rig.run(&["apply", file.to_str().unwrap(), "--param", "code=7"]).await;
    assert_eq!(ran.code, 1);
    assert!(ran.out.contains(" failed in "), "{}", ran.out);
    for expected in
        ["  first  ok", "  bad    failed", "exited with code 7: no such table", "  never  cancelled"]
    {
        assert!(ran.out.contains(expected), "missing {expected:?} in:\n{}", ran.out);
    }
    assert_eq!(ran.err.trim(), "error: step `bad` failed: exited with code 7: no such table");

    let runs = rig.run(&["runs"]).await.ok();
    let run_id = runs.split_whitespace().next().unwrap().to_string();
    assert_eq!(rig.run(&["logs", &run_id, "first"]).await.ok().trim(), "hello");
    assert!(
        rig.run(&["logs", &run_id, "bad"]).await.ok().contains("! no such table"),
        "standard error is marked"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_uses_the_same_command_signed_in_as_its_slot() {
    let rig = Rig::signed_in().await;
    rig.run(&["channel", "create", "general"]).await.ok();
    rig.run(&["agent", "add", "helper", "--channel", "general"]).await.ok();
    // What the slot gives an agent's process: where the daemon is, and a
    // file holding its credential.
    let token_file = rig.dir.join("data/slots/helper/token");
    let as_agent = [(brand::env_var("TOKEN_FILE"), token_file.to_string_lossy().into_owned())];
    let env: Vec<(&str, String)> =
        as_agent.iter().map(|(name, value)| (name.as_str(), value.clone())).collect();

    let me = rig.run_with(&env, &["whoami"]).await.ok();
    assert!(me.starts_with("helper (agent:helper)"), "{me}");
    rig.run_with(&env, &["post", "general", "Reporting in."]).await.ok();
    assert!(rig.run(&["read", "general"]).await.ok().contains("[helper] Reporting in."));

    let denied = rig.run_with(&env, &["agent", "add", "minion"]).await;
    assert_eq!(denied.code, 1);
    assert_eq!(denied.err.trim(), "error: helper is not allowed to do this (needs `agents:write`)");

    // Its credential gone (the slot was stopped), it is nobody: it does
    // not go on as whoever signed in on this machine, which is its owner.
    let gone = [(brand::env_var("TOKEN_FILE"), rig.dir.join("no-such-token").to_string_lossy().into_owned())];
    let env_gone: Vec<(&str, String)> =
        gone.iter().map(|(name, value)| (name.as_str(), value.clone())).collect();
    let nobody = rig.run_with(&env_gone, &["whoami"]).await;
    assert_eq!(nobody.code, 1, "{}", nobody.out);
    assert!(
        nobody.err.contains("cannot be read") && nobody.err.contains("Nothing else was used in its place"),
        "{}",
        nobody.err
    );

    let catalog = rig.run_with(&env, &["actions", "agent.consult"]).await.ok();
    assert!(catalog.contains("needs: agents:write"), "{catalog}");
    let options: Vec<&str> = catalog
        .lines()
        .find(|line| line.trim_start().starts_with("options"))
        .unwrap()
        .split_whitespace()
        .take(3)
        .collect();
    assert_eq!(options, ["options", "list", "required"]);
}

/// `actions <name>` lists an action's arguments, and under them what an
/// argument says is written inside it: each key and each of its words
/// by where it is written, with its sentence.
#[tokio::test(flavor = "multi_thread")]
async fn an_action_is_listed_with_what_is_written_inside_its_arguments() {
    let rig = Rig::signed_in().await;
    let table = rig.run(&["actions", "collection.ensure"]).await.ok();
    assert!(table.contains("inside `fields` (`*` is a name of your own choosing):"), "{table}");
    let line = |path: &str| -> Vec<String> {
        let found = table.lines().find(|line| line.split_whitespace().next() == Some(path));
        let found = found.unwrap_or_else(|| panic!("a line for `{path}` in:\n{table}"));
        found.split_whitespace().take(3).map(str::to_string).collect()
    };
    assert_eq!(line("*.quiet"), ["*.quiet", "bool", "its"]);
    assert_eq!(line("*.type.list"), ["*.type.list", "word", "several"]);
    assert_eq!(line("*.figures.*.sum"), ["*.figures.*.sum", "word", "what"]);
    // The argument itself says what it is, and no longer lists its keys in a sentence.
    assert_eq!(line("fields")[..3], ["fields", "object", "required"]);
    assert!(!table.contains("its `type` (text, choice"), "{table}");
    let plain = rig.run(&["actions", "agent.consult"]).await.ok();
    assert!(!plain.contains("inside `"), "an action that says nothing of it lists nothing: {plain}");
    // And where the same is read in the client is said where the command is explained.
    let help = rig.run(&["help", "actions"]).await.ok();
    assert!(help.contains("The client's Reference shows the same on one page"), "{help}");
    assert_eq!(rig.run(&["help", "action"]).await.ok(), help);
    assert!(rig.run(&["help", "events"]).await.ok().contains("The client's Reference lists the\nsame kinds"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_second_device_is_invited_on_the_hub_and_joins_with_the_invitation() {
    let hub = Rig::signed_in().await;
    let member = Rig::signed_in().await;
    let cli = brand::CLI;

    // Each says the short name of the key it proves, to be compared with
    // what the other shows of it.
    let alone = hub.run(&["device", "list"]).await.ok();
    let own =
        alone.lines().next().unwrap().strip_prefix("this device is `test-device`, ").unwrap().to_string();
    assert_eq!(own.len(), 24, "{alone}");
    assert!(
        alone.contains(&format!("no other devices in its fleet; `{cli} device invite <name>`")),
        "{alone}"
    );
    assert!(hub.run(&["status"]).await.ok().contains(&format!("it is {own}.")));

    // The hub answers with the invitation on a line of its own, and with
    // the command to run on the other device, which reads it from
    // standard input: it is written into no command.
    let invited = hub.run(&["device", "invite", "builder-1", "--at", &hub.url, "--expires", "3d"]).await.ok();
    let lines: Vec<&str> = invited.lines().collect();
    assert_eq!(lines[0], "`builder-1` is invited. Its invitation, shown this once:");
    let invitation = lines[1].trim();
    assert!(invitation.starts_with("inv_") && !invitation.contains(' '), "{invited}");
    assert_eq!(lines[3], format!("  {cli} device join"), "{invited}");
    assert_eq!(invited.matches("inv_").count(), 1, "the invitation is in no command: {invited}");
    // What the other device has to be first, in the order it is done.
    assert!(lines[2].contains(&format!("(`{cli} up`, then `{cli} register`)")), "{invited}");
    assert!(
        invited.contains("within the next 3 days") && invited.contains("as you would a password"),
        "{invited}"
    );

    // A device nobody has registered at is told the two things to do, in
    // their order, before its invitation is asked for or used.
    let nobody = Rig::new().await;
    let early = nobody.run(&["device", "join", invitation]).await;
    assert_eq!(early.code, 1, "{}", early.out);
    assert_eq!(
        early.err.trim(),
        format!(
            "error: this device's factory has no account yet, and joining is asked by whoever is signed in to it. \
             Make its first account, `{cli} register`, and then `{cli} device join` again: the invitation has not \
             been used"
        )
    );
    let waiting = hub.run(&["device", "list"]).await.ok();
    assert!(waiting.contains("builder-1") && waiting.contains("has yet to use its invitation"), "{waiting}");

    // What was done before says what is done now.
    let before = hub.run(&["device", "add", "builder-2"]).await;
    assert!(before.code != 0 && before.err.contains("device invite <name>"), "{}", before.err);
    let before = member.run(&["device", "join", &hub.url, "dev_0123456789"]).await;
    assert!(before.code != 0 && before.err.contains("no longer joins by an address"), "{}", before.err);

    // Given on standard input, the invitation is in no shell's history.
    let mut command =
        crate::command_in(&member.dir, &member.url, &["device", "join", "--allow", "fleet,shell"]);
    command.stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped());
    let mut child = command.spawn().unwrap();
    {
        use tokio::io::AsyncWriteExt;
        let mut given = child.stdin.take().unwrap();
        given.write_all(format!("{}\n", invitation).as_bytes()).await.unwrap();
    }
    let output = crate::soon("the device to join", child.wait_with_output()).await.unwrap();
    let joined = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(output.status.success(), "{joined}");
    assert_eq!(
        joined.lines().next().unwrap(),
        format!(
            "joined the fleet at {} ({own}); this device now does what that hub's blueprints send it, limited to: fleet, shell",
            hub.url
        )
    );
    assert!(joined.contains("holds the hub to that key"), "{joined}");
    hub.node
        .as_ref()
        .unwrap()
        .app
        .journal
        .follow_every(Default::default(), "device.connected")
        .next()
        .await
        .expect("the member links to the hub");
    // Used, the invitation lets nobody else in.
    let third = Rig::signed_in().await;
    let again = third.run(&["device", "join", invitation]).await;
    assert!(again.code != 0 && again.err.contains("that invitation has been used"), "{}", again.err);

    let fleet = hub.run(&["device", "list"]).await.ok();
    let row: Vec<&str> = fleet.lines().nth(1).unwrap().split_whitespace().collect();
    assert_eq!((row[0], row[2]), ("builder-1", "linked"), "{fleet}");
    let theirs = member.run(&["device", "list"]).await.ok();
    assert!(theirs.starts_with(&format!("this device is `test-device`, {}", row[1])), "{theirs}\n{fleet}");
    assert!(theirs.contains(&format!("({own}), as `builder-1`")), "{theirs}");

    // A step for that device, applied from the hub's terminal.
    let file = hub.dir.join("afar.bp");
    let marker = member.dir.join("made-here");
    std::fs::write(
        &file,
        format!(
            "blueprint: afar\npermissions: [shell]\nsteps:\n  there: {{ do: shell, device: builder-1, with: {{ cmd: {} }} }}\n",
            crate::standin::cmd(&["write", &marker.to_string_lossy(), "made"])
        ),
    )
    .unwrap();
    let applied = hub.run(&["apply", file.to_str().unwrap()]).await.ok();
    assert!(applied.contains("  there  ok"), "{applied}");
    assert!(marker.exists());

    hub.run(&["device", "remove", "builder-1"]).await.ok();
    assert_eq!(
        member.run(&["device", "leave"]).await.ok().trim(),
        "left the fleet; what was started here keeps running"
    );
    let gone = hub.run(&["device", "remove", "builder-1"]).await;
    assert_eq!(gone.err.trim(), "error: there is no device called `builder-1` in this fleet");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_blueprint_that_ships_with_the_product_is_applied_by_its_name() {
    let rig = Rig::signed_in().await;
    let applied = rig
        .run(&["apply", "std/rolling", "--param", "devices=[\"local\"]", "--param", "run=echo rolled"])
        .await
        .ok();
    assert!(applied.contains(" ok in ") && applied.contains("  each  ok"), "{applied}");

    // A name nobody has is the daemon's to refuse; a missing file is ours.
    let unknown = rig.run(&["apply", "std/no-such-thing"]).await;
    assert_eq!(unknown.code, 1);
    assert!(unknown.err.contains("std/no-such-thing"), "{}", unknown.err);
    let missing = rig.run(&["apply", "./nowhere/thing.bp"]).await;
    assert_eq!(missing.err.trim(), "error: cannot read ./nowhere/thing.bp");
    // The same of a file under the name an editor takes for YAML; and one
    // that is there is applied whichever of its names it has.
    let missing = rig.run(&["apply", "./nowhere/thing.bp.yml"]).await;
    assert_eq!(missing.err.trim(), "error: cannot read ./nowhere/thing.bp.yml");
    let file = rig.dir.join("hello.bp.yml");
    std::fs::write(&file, "blueprint: hello\nsteps:\n  said: { do: emit, with: { topic: shop.hello } }\n")
        .unwrap();
    let applied = rig.run(&["apply", file.to_str().unwrap()]).await.ok();
    assert!(applied.contains(" ok in ") && applied.contains("  said  ok"), "{applied}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_project_is_added_with_a_board_and_a_fleet_and_told_bare_with_neither() {
    let rig = Rig::signed_in().await;
    let cli = brand::CLI;

    let created = rig.run(&["project", "create", "Shop"]).await.ok();
    assert!(created.starts_with("project `shop` created at "), "{created}");
    assert!(
        created.contains(&format!("It has a board (`{cli} board shop`) and a fleet")),
        "it says what it was given: {created}"
    );
    // The board is there to be written on, and the fleet's two tables to be changed.
    rig.run(&["board", "add", "shop", "Write", "a", "README"]).await.ok();
    assert!(rig.run(&["board", "shop"]).await.ok().contains("Write a README"));
    let agents = rig.run(&["record", "list", "shop", "agents"]).await.ok();
    assert!(agents.contains("shop-agents-1"), "{agents}");
    let stages = rig.run(&["record", "list", "shop", "stages"]).await.ok();
    assert!(stages.contains("shop-stages-1"), "{stages}");
    // Its fleet blueprint is a file of its own, to be read and changed.
    assert!(rig.placed("shop").join(fleet_file()).is_file());

    // Told `--bare`, the project is added alone, and what it lacks is said with how to give it.
    let bare = rig.run(&["project", "create", "--bare", "Depot"]).await.ok();
    assert!(
        bare.contains("It is bare, as asked")
            && bare.contains(&format!("{cli} apply std/board --project depot"))
            && bare.contains(&format!("{cli} fleet init depot")),
        "{bare}"
    );
    assert!(!rig.placed("depot").join(fleet_file()).exists());
    let none = rig.run(&["board", "depot"]).await;
    assert!(none.err.contains("has no board yet"), "{}", none.err);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_project_is_added_without_what_it_is_told_to_leave_out_and_what_goes_with_that_is_said() {
    let rig = Rig::signed_in().await;
    let cli = brand::CLI;

    // The usual has a library too, and says so with how to read it.
    let created = rig.run(&["project", "create", "Shop"]).await.ok();
    assert!(
        created
            .contains(&format!("It has a library, its documents as pages (`{cli} record list shop docs`)")),
        "{created}"
    );
    assert!(created.contains("docs/overview.md"), "{created}");
    let documents = rig.run(&["record", "list", "shop", "docs"]).await.ok();
    assert!(documents.contains("shop-docs:overview"), "{documents}");

    // Without its library it has no documents either, and the rest as ever.
    let made = rig.run(&["project", "create", "Ledger", "--without", "docs"]).await.ok();
    assert!(made.contains(&format!("It has a board (`{cli} board ledger`) and a fleet")), "{made}");
    // The documents a library starts with, by name: the folder the
    // project is made in may have `docs/` in its own path.
    assert!(!made.contains("It has a library") && !made.contains("docs/overview.md"), "{made}");
    assert!(!rig.project("ledger").join("docs").exists());
    assert!(rig.placed("ledger").join(fleet_file()).is_file());

    // Without a board there is no fleet, which works one: said, with how to have one later.
    let made = rig.run(&["project", "create", "Shelf", "--without", "board"]).await.ok();
    assert!(!made.contains("It has a board"), "{made}");
    assert!(made.contains("`shelf` was not given a fleet: a fleet works a board"), "{made}");
    assert!(made.contains("It has a library"), "{made}");
    assert!(!rig.placed("shelf").join(fleet_file()).exists());

    // Leaving out more than one, and everything, are said the same way.
    let made = rig
        .run(&["project", "create", "Plain", "--without", "board", "--without", "fleet", "--without", "docs"])
        .await
        .ok();
    assert!(!made.contains("It has") && !made.contains("was not given"), "{made}");
    assert!(!rig.project("plain").join("docs").exists());

    // Something no project starts with is not something to leave out, and nothing is added.
    let wrong = rig.run(&["project", "create", "Other", "--without", "garden"]).await;
    assert_ne!(wrong.code, 0);
    assert!(
        wrong.err.contains("`garden` is nothing a project starts with")
            && wrong.err.contains("board, fleet, docs"),
        "{}",
        wrong.err
    );
    let both = rig.run(&["project", "create", "Other", "--bare", "--without", "docs"]).await;
    assert!(both.err.contains("say one or the other"), "{}", both.err);
    assert!(!rig.run(&["project", "list"]).await.ok().contains("other"));

    // What a project is not given unless it is asked for is asked for by name.
    let made = rig.run(&["project", "create", "Works", "--with", "features"]).await.ok();
    assert!(made.contains(&format!("It has a board (`{cli} board works`) and a fleet")), "{made}");
    assert!(
        made.contains(&format!(
            "It has features, for work bigger than a card and for cards put in order together (`{cli} record list works features`)"
        )),
        "{made}"
    );
    // Without what they need they are left out, saying why.
    let made = rig.run(&["project", "create", "Lone", "--with", "features", "--without", "fleet"]).await.ok();
    assert!(
        made.contains("`lone` was not given features: a feature is planned by one of the fleet"),
        "{made}"
    );
    let wrong = rig.run(&["project", "create", "Other", "--with", "garden"]).await;
    assert!(
        wrong.err.contains("`garden` is nothing a project starts with. It may be with:"),
        "{}",
        wrong.err
    );

    // A setup the factory does not keep is refused, saying what it keeps, and nothing is added.
    let wrong = rig.run(&["project", "create", "Other", "--setup", "garden"]).await;
    assert!(wrong.err.contains("there is no setup called `garden`"), "{}", wrong.err);
    let both = rig.run(&["project", "create", "Other", "--bare", "--setup", "garden"]).await;
    assert!(both.err.contains("say one or the other"), "{}", both.err);

    // What the command says of itself names each, and how a project is taken out again.
    let help = rig.run(&["help", "project"]).await.ok();
    for word in [
        "--setup NAME",
        "--without <name>",
        "--with <name>",
        "board ",
        "fleet ",
        "docs ",
        "features ",
        "project forget <project>",
    ] {
        assert!(help.contains(word), "`{word}` in: {help}");
    }
}

#[tokio::test]
async fn a_channel_is_moved_to_the_trash_from_a_terminal_and_one_something_posts_in_only_when_told_all_the_same()
 {
    let rig = Rig::signed_in().await;
    rig.run(&["channel", "create", "scratch"]).await.ok();
    rig.run(&["post", "scratch", "nothing", "to", "keep"]).await.ok();
    let gone = rig.run(&["channel", "remove", "#scratch"]).await.ok();
    assert!(gone.contains("moved #scratch to the Trash as `trash_"), "{gone}");
    assert!(gone.contains("with what was said in it and attached to it; it is kept 30 days"), "{gone}");
    let listed = rig.run(&["channel", "list"]).await.ok();
    assert!(!listed.contains("#scratch"), "{listed}");
    let read = rig.run(&["read", "scratch"]).await;
    assert!(read.err.contains("there is no channel `scratch`"), "{}", read.err);
    // Listed in the Trash, and given back from there with what was said.
    let trashed = rig.run(&["trash"]).await.ok();
    let line = trashed
        .lines()
        .find(|line| line.contains("channel `#scratch`"))
        .unwrap_or_else(|| panic!("{trashed}"));
    let id = line.split_whitespace().next().unwrap();
    rig.run(&["trash", "restore", id]).await.ok();
    assert!(rig.run(&["read", "scratch"]).await.ok().contains("nothing to keep"));
    // Where nothing that is deleted is kept, it goes for good at once.
    rig.run(&["trash", "--keep", "0"]).await.ok();
    let gone = rig.run(&["channel", "remove", "scratch"]).await.ok();
    assert!(gone.contains("removed #scratch, with what was said in it"), "{gone}");
    rig.run(&["trash", "--keep", "30"]).await.ok();

    // A bot says which channel it speaks in, and holds it while it runs.
    rig.run(&["channel", "create", "general"]).await.ok();
    let bot = ["--param", "channel=general", "--param", "name=parrot", "--param", "run=uptime"];
    rig.run(&[&["apply", "std/bot", "--key", "bot-parrot"][..], &bot[..]].concat()).await.ok();
    let held = rig.run(&["channel", "remove", "general"]).await;
    assert!(held.err.contains("#general is where `bot-parrot` (std/bot, "), "{}", held.err);
    assert!(
        held.err.contains(&format!("`{} stop in_", brand::CLI)) && held.err.contains("--force"),
        "{}",
        held.err
    );
    assert!(rig.run(&["channel", "list"]).await.ok().contains("#general"));
    let forced = rig.run(&["channel", "remove", "general", "--force"]).await.ok();
    assert!(forced.contains("moved #general to the Trash"), "{forced}");
    // The bot was left running; stopped, as anything that runs is.
    let alive = rig.run(&["instances", "--json"]).await.ok();
    let alive: serde_json::Value = serde_json::from_str(&alive).unwrap();
    for instance in alive["instances"].as_array().unwrap() {
        rig.run(&["stop", instance["id"].as_str().unwrap()]).await.ok();
    }
}

#[tokio::test]
async fn repack_is_refused_while_the_factory_runs_and_says_what_there_is_once_it_is_stopped() {
    let mut rig = Rig::new().await;
    let data = rig.dir.join("data");
    let data = data.to_str().unwrap();
    let running = rig.run(&["repack", "--data", data]).await;
    assert_eq!(running.code, 1, "{}", running.out);
    assert!(running.err.contains("is running") && running.err.contains("down"), "{}", running.err);

    let node = rig.node.take().unwrap();
    crate::held(&node);
    node.stop().await;
    // A factory made now is as it would be afterwards, and is told so.
    let looked = rig.run(&["repack", "--look", "--data", data]).await;
    assert_eq!(looked.code, 0, "{}", looked.err);
    assert!(looked.out.contains("pages of 2048 bytes and events the journal numbers"), "{}", looked.out);
    assert!(looked.out.contains("there is nothing to do"), "{}", looked.out);
    assert!(rig.run(&["repack", "--data", data]).await.ok().contains("there is nothing to do"));
    let json: serde_json::Value =
        serde_json::from_str(&rig.run(&["repack", "--look", "--json", "--data", data]).await.ok()).unwrap();
    assert_eq!((&json["page"], &json["wanted"]), (&serde_json::json!(2048), &serde_json::json!(false)));

    let none = rig.run(&["repack", "--data", rig.dir.join("none").to_str().unwrap()]).await;
    assert_eq!(none.code, 1);
    assert!(none.err.contains("there is no factory at"), "{}", none.err);
}

#[tokio::test]
async fn apart_is_refused_while_the_factory_runs_and_moves_a_project_from_before_once_it_is_stopped() {
    let mut rig = Rig::signed_in().await;
    let data = rig.dir.join("data");
    rig.run(&["project", "create", "Shop", "--bare"]).await.ok();
    let running = rig.run(&["apart", "--data", data.to_str().unwrap()]).await;
    assert_eq!(running.code, 1, "{}", running.out);
    assert!(running.err.contains("is running") && running.err.contains("down"), "{}", running.err);

    let node = rig.node.take().unwrap();
    crate::held(&node);
    node.stop().await;
    let data = data.to_str().unwrap();
    // Played with every project kept in the factory's one journal, none
    // has a journal of its own to be told of.
    if journal::apart() == Some(false) {
        return;
    }
    // A project this program added is kept apart already, and is told so.
    let looked = rig.run(&["apart", "--look", "--data", data]).await.ok();
    assert!(looked.contains("`shop` is kept in a journal of its own already"), "{looked}");
    assert!(looked.contains("There is nothing to move"), "{looked}");
    // As a factory an earlier program made holds it: no row says it has
    // a journal, and no file is kept for one.
    let left = |file: &str, sql: &str| {
        let connection = journal::copy::Connection::open(rig.dir.join("data").join(file)).unwrap();
        connection.execute_batch(sql).unwrap_or_else(|err| panic!("{sql}: {err}"));
    };
    left("journal.db", "DELETE FROM journals");
    std::fs::remove_dir_all(rig.dir.join("data").join("journals")).unwrap();
    // With something that program kept for it beside its rows: the last
    // text of a blueprint that checked, as the project saw it. And with
    // what it kept of it in tables: one of the project's own with a
    // record and a line of its history, and what an agent was asked.
    let kept = |file: &str| -> i64 {
        let connection = journal::copy::Connection::open(rig.dir.join("data").join(file)).unwrap();
        let fits = "SELECT count(*) FROM kv WHERE namespace = 'fits' AND key = 'shop' || char(10) || 'kid'";
        connection.query_row(fits, [], |row| row.get(0)).unwrap()
    };
    left(
        "journal.db",
        "INSERT INTO kv (namespace, key, value) VALUES ('fits', 'shop' || char(10) || 'kid', '{}');
         INSERT INTO collections (project, name, fields, views, store, created, updated)
             VALUES ('shop', 'notes', '[]', '[]', 'local', 1, 1);
         INSERT INTO records (id, project, collection, number, data, position, created_by, created, updated)
             VALUES ('shop-notes-1', 'shop', 'notes', 1, '{}', 1.0, 'ada', 1, 1);
         INSERT INTO record_history (record, seq, ts, topic, actor, who, said, data)
             VALUES ('shop-notes-1', 1, 1, 'record.created', 'user:ada', 'Ada', 'Ada: created', '{}');
         INSERT INTO slots (name, adapter, project, instructions, scopes, permission_mode, active, created, updated)
             VALUES ('shop-helper', 'builtin', 'shop', '', '[]', 'edit', 1, 1, 1);
         INSERT INTO turns (id, slot, kind, thread, prompt, options, status, created)
             VALUES ('turn_1', 'shop-helper', 'task', 'seat:1', 'do it', '[]', 'done', 2);",
    );
    // It says what would go of each kind, in words.
    let all = "1 tables, 1 records, 1 lines of their history, 1 turns of its agents";
    let looked = rig.run(&["apart", "--look", "--data", data]).await.ok();
    assert!(looked.contains("`shop` would be moved to a journal of its own"), "{looked}");
    assert!(looked.contains(&format!("{all}; and beside them 1 texts that last checked")), "{looked}");
    assert!(looked.contains("Nothing was changed"), "{looked}");
    assert_eq!(kept("journal.db"), 1);
    assert!(!rig.dir.join("data").join("journals").join("shop.db").exists());
    let json: serde_json::Value =
        serde_json::from_str(&rig.run(&["apart", "--look", "--json", "--data", data]).await.ok()).unwrap();
    assert_eq!(json["projects"][0]["project"], "shop", "{json}");
    assert_eq!((&json["projects"][0]["moves"], &json["projects"][0]["apart"]), (&true.into(), &false.into()));
    assert_eq!(json["projects"][0]["beside"]["fits"], 1, "{json}");
    assert_eq!(
        (&json["projects"][0]["rows"]["records"], &json["projects"][0]["rows"]["turns"]),
        (&1.into(), &1.into())
    );

    let unknown = rig.run(&["apart", "yard", "--data", data]).await;
    assert_eq!(unknown.code, 1);
    assert!(
        unknown.err.contains("has no project `yard`") && unknown.err.contains("`shop`"),
        "{}",
        unknown.err
    );
    let moved = rig.run(&["apart", "shop", "--data", data]).await.ok();
    assert!(moved.contains("`shop` was moved") && moved.contains("journals/shop.db"), "{moved}");
    assert!(moved.contains("Start the factory again"), "{moved}");
    assert!(moved.contains(all) && moved.contains("1 texts that last checked"), "{moved}");
    assert!(rig.dir.join("data").join("journals").join("shop.db").is_file());
    assert_eq!((kept("journal.db"), kept("journals/shop.db")), (0, 1));
    assert!(rig.run(&["apart", "--data", data]).await.ok().contains("There was nothing to move"));
    // Moved, it is said to be kept apart, with what its file holds.
    let looked = rig.run(&["apart", "--look", "--data", data]).await.ok();
    assert!(looked.contains("already (journals/shop.db: ") && looked.contains(all), "{looked}");

    // A project whose journal keeps less than one made now is told what
    // it keeps and what is left to move. One that keeps its tables and
    // not what its agents were asked, with a turn in the factory's file:
    left("journals/shop.db", "DELETE FROM turns");
    left(
        "journal.db",
        "UPDATE journals SET keeps = 2;
         INSERT INTO turns (id, slot, kind, thread, prompt, options, status, created, project, nth)
             VALUES ('turn_2', 'shop-helper', 'task', 'seat:1', 'again', '[]', 'done', 3, 'shop', 2);",
    );
    let looked = rig.run(&["apart", "--look", "--data", data]).await.ok();
    let keeps = "keeps what is run for it, what is said of it and its tables with their records";
    assert!(
        looked.contains(&format!("`shop` has a journal of its own that {keeps} (journals/shop.db)")),
        "{looked}"
    );
    assert!(looked.contains("the rest of it would be moved there: 1 turns of its agents."), "{looked}");
    let moved = rig.run(&["apart", "--data", data]).await.ok();
    assert!(
        moved.contains("`shop` was moved") && moved.contains("journals/shop.2.db now (1 turns"),
        "{moved}"
    );
    // And one that keeps what is run and no more, of which the
    // factory's file holds nothing: its journal is made to keep the rest.
    left("journal.db", "UPDATE journals SET keeps = 1");
    let looked = rig.run(&["apart", "--look", "--data", data]).await.ok();
    let keeps = "keeps what is run for it and what is said of it (journals/shop.2.db)";
    assert!(looked.contains(keeps) && looked.contains("holds nothing more of it"), "{looked}");
    let moved = rig.run(&["apart", "--data", data]).await.ok();
    // Into the first file of its name that is free: the one it had at
    // first was put away when it was moved out of it.
    assert!(moved.contains("`shop` is kept whole in journals/shop.db from now on"), "{moved}");
    let looked = rig.run(&["apart", "--look", "--data", data]).await.ok();
    assert!(looked.contains("already (journals/shop.db: ") && looked.contains("There is nothing to move"));

    let none = rig.run(&["apart", "--data", rig.dir.join("none").to_str().unwrap()]).await;
    assert_eq!(none.code, 1);
    assert!(none.err.contains("there is no factory at"), "{}", none.err);
    // And the help says how it is asked.
    let help = rig.run(&["help", "apart"]).await.ok();
    assert!(help.contains("apart [<project>] [--look]") && help.contains("moved at rest"), "{help}");
}

#[tokio::test]
async fn how_many_threads_do_the_work_is_a_number_from_one_up() {
    let rig = Rig::new().await;
    for wrong in ["0", "many", "-2"] {
        let ran = rig.run(&["status", "--workers", wrong]).await;
        assert_eq!(ran.code, 2, "{wrong}: {}", ran.out);
        assert!(ran.err.contains("`--workers` is how many threads do the work"), "{}", ran.err);
    }
    rig.run(&["status", "--workers", "2"]).await.ok();
    let told = rig.run_with(&[(&brand::env_var("WORKERS"), "3".to_string())], &["status"]).await;
    assert_eq!(told.code, 0, "{}", told.err);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_language_is_printed_as_a_schema_for_an_editor_and_with_no_daemon_as_far_as_can_be_known() {
    let offered = |printed: &str| -> Vec<String> {
        let schema: serde_json::Value =
            serde_json::from_str(printed).expect("a schema is printed and nothing else");
        assert_eq!(schema["required"], serde_json::json!(["blueprint"]), "{schema}");
        let named = schema["definitions"]["an-action"]["anyOf"].as_array().unwrap().clone();
        named.iter().filter_map(|one| one["const"].as_str().map(str::to_string)).collect()
    };
    // Asked of the daemon, it has the actions there are on this device.
    let rig = Rig::signed_in().await;
    let here = offered(&rig.run(&["schema"]).await.ok());
    assert!(here.iter().any(|name| name == "shell") && here.iter().any(|name| name == "emit"), "{here:?}");

    // With no daemon to ask it is the language alone, and it says so
    // apart from what it prints, which is still a schema and nothing else.
    let dir = scratch("schema");
    let alone = run_in(&dir, "http://127.0.0.1:1", &[], &["schema"]).await;
    assert_eq!(alone.code, 0, "{}", alone.err);
    let known = offered(&alone.out);
    assert!(
        known.iter().any(|name| name == "emit") && !known.iter().any(|name| name == "shell"),
        "{known:?}"
    );
    assert!(alone.err.contains("no action but those that are part of the language"), "{}", alone.err);

    // What to tell an editor is in the help: the name a file may have
    // here, and the lines themselves where the language is written up.
    let help = run_in(&dir, "http://127.0.0.1:1", &[], &["help", "schema"]).await.ok();
    assert!(help.contains(".bp.yml") && help.contains("help blueprints"), "{help}");
    let written_up = run_in(&dir, "http://127.0.0.1:1", &[], &["help", "blueprints"]).await.ok();
    assert!(written_up.contains("$schema=./blueprint.schema.json"), "the line an editor reads is there");
    let _ = std::fs::remove_dir_all(dir);
}
