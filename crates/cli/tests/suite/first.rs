//! The first things anybody types at a program just unpacked, from a
//! terminal: what `help` says of starting it, opening it and the first
//! account, what `open` says where no browser comes up, and what is said
//! when a project is let go of with its fleet still up. Each is what a new
//! person met with nothing said.

use crate::{Rig, run_in, scratch};

#[tokio::test(flavor = "multi_thread")]
async fn forgetting_a_project_says_what_still_runs_for_it_and_what_stops_each() {
    let rig = Rig::signed_in().await;
    let cli = brand::CLI;
    rig.run(&["project", "create", "Shop"]).await.ok();
    let said = rig.run(&["project", "forget", "shop"]).await.ok();
    let lines: Vec<&str> = said.lines().collect();
    // Into the Trash, saying what gives it back.
    let trashed =
        lines[0].strip_prefix("forgot `shop`, into the Trash as `").and_then(|rest| rest.split_once('`'));
    let (trashed, rest) = trashed.unwrap_or_else(|| panic!("{said}"));
    assert_eq!(
        rest,
        format!("; its files are where they were, and `{cli} trash restore {trashed}` gives it back"),
        "{said}"
    );
    assert!(lines[1].ends_with(&format!("`{cli} stop <id>` stops one:")), "{said}");
    // Each thing applied for it, by the id that stops it.
    let applied: Vec<(&str, &str)> =
        lines.iter().filter_map(|line| line.strip_prefix("  ")?.split_once("  ")).collect();
    let mut names: Vec<&str> = applied.iter().map(|(_, blueprint)| *blueprint).collect();
    names.sort_unstable();
    assert_eq!(names, ["fleet", "std/board", "std/docs"], "{said}");
    assert!(said.contains("its agents are still up: ") && said.contains("shop-worker-1"), "{said}");
    assert!(said.contains(&format!("`{cli} agent stop <name>`")), "{said}");
    assert!(
        said.contains("its channels are still there: #shop.")
            && said.contains(&format!("`{cli} channel remove <name>`")),
        "{said}"
    );
    // Stopped by what it said, nothing of the project is alive.
    for (id, _) in &applied {
        rig.run(&["stop", id]).await.ok();
    }
    let agents = rig.run(&["agent", "list"]).await.ok();
    assert!(
        agents.lines().filter(|line| line.starts_with("shop-")).all(|line| line.contains("stopped")),
        "{agents}"
    );

    // A project with nothing applied for it is forgotten in the one line,
    // and given back from the Trash by what that line says.
    rig.run(&["project", "create", "--bare", "Plain"]).await.ok();
    let forgot = rig.run(&["project", "forget", "plain"]).await.ok();
    assert_eq!(forgot.trim().lines().count(), 1, "{forgot}");
    let (_, trashed) = forgot.split_once("into the Trash as `").unwrap();
    let trashed = trashed.split('`').next().unwrap();
    let listed = rig.run(&["trash"]).await.ok();
    assert!(listed.contains(&format!("{trashed}  project `Plain`")), "{listed}");
    assert!(listed.contains("kept in the Trash for 30 days"), "{listed}");
    let back = rig.run(&["trash", "restore", trashed]).await.ok();
    assert!(back.starts_with("gave back project `Plain`"), "{back}");
    assert!(rig.run(&["project", "list"]).await.ok().contains("plain"));
    // Kept for nought days, a project is forgotten as it always was.
    rig.run(&["trash", "--keep", "0"]).await.ok();
    assert_eq!(
        rig.run(&["project", "forget", "plain"]).await.ok().trim(),
        "forgot `plain`; its files are where they were"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_first_things_anybody_types_each_have_help_of_their_own() {
    let dir = scratch("first-help");
    let cli = brand::CLI;
    let help = |topic: &'static str| {
        let dir = dir.clone();
        async move { run_in(&dir, "http://127.0.0.1:1", &[], &["help", topic]).await.ok() }
    };
    // Starting it: that it stays and starts the daemon again, how it is stopped, and where it listens unless told.
    let up = help("up").await;
    assert!(up.starts_with(&format!("{cli} up [--listen ADDR]")), "{up}");
    for said in [
        format!("`{cli} up &`"),
        "http://127.0.0.1:7700".into(),
        format!("`{cli} status` says"),
        "`down`".into(),
    ] {
        assert!(up.contains(&said), "`{said}` of starting it:\n{up}");
    }
    assert_eq!(help("serve").await, up);
    // Opening it: the address is what to go by where no browser comes up.
    let open = help("open").await;
    assert!(
        open.contains("the address it said is\nwhat to open")
            && open.contains(&format!("`{cli} help register`")),
        "{open}"
    );
    // Nobody registers to begin: the first start makes the owner, and a
    // terminal signed in as nobody is signed in as the owner by this machine.
    let register = help("register").await;
    assert!(
        register.starts_with(&format!("{cli} register [--email E --name N --password P] [--invite CODE]")),
        "{register}"
    );
    assert!(
        register.contains("Nobody registers to begin: the first start of a factory makes its owner")
            && register.contains("`--here`")
            && register.contains(&format!("(`{cli} open` opens the client signed in)")),
        "{register}"
    );
    for same in ["login", "logout", "whoami"] {
        assert_eq!(help(same).await, register, "`help {same}`");
    }
    assert!(help("status").await.contains("makes the first account"));
    // And none of them made the page every agent reads any longer.
    assert!(run_in(&dir, "http://127.0.0.1:1", &[], &["help"]).await.ok().lines().count() < 70);
    let _ = std::fs::remove_dir_all(dir);
}

/// Unix's alone: the program is run where nothing that opens a browser
/// can be found, by a path that holds nothing, and on Windows what opens
/// one is the system's shell, which is found whatever the path says: run
/// there, this would open a browser on whoever runs the tests.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn open_says_where_the_client_is_whether_or_not_a_browser_comes_up() {
    let dir = scratch("open");
    let nothing = dir.join("nothing-to-run");
    std::fs::create_dir_all(&nothing).unwrap();
    let path = [("PATH", nothing.display().to_string())];
    let ran = run_in(&dir, "http://127.0.0.1:1", &path, &["open"]).await;
    assert_eq!(ran.code, 0, "{}", ran.err);
    let said: Vec<&str> = ran.out.lines().collect();
    assert_eq!(
        said,
        [
            "the client is at http://127.0.0.1:1; opening it in your browser",
            "no browser could be started from here: open that address in one"
        ],
        "the address first, and then that nothing could be started"
    );
    let _ = std::fs::remove_dir_all(dir);
}
