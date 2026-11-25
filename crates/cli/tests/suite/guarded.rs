//! What the terminal says of what the daemon holds to: a project it does
//! not yet trust, the names it answers to, and listening beyond this
//! machine.

use crate::Rig;

const THEIR_BOARD: &str = "\
blueprint: std/board
description: Somebody else's idea of a board.
steps:
  theirs: { do: emit, with: { topic: theirs.ran } }
";

#[tokio::test(flavor = "multi_thread")]
async fn a_folder_that_was_there_is_said_not_to_be_trusted_until_its_owner_trusts_it_from_the_terminal() {
    let rig = Rig::signed_in().await;
    let repository = rig.dir.join("their-repository");
    std::fs::create_dir_all(repository.join("blueprints/std")).unwrap();
    std::fs::write(repository.join("blueprints/std/board.bp"), THEIR_BOARD).unwrap();

    // Taking it up says what that means and how to change it.
    let adopted =
        rig.run(&["project", "create", "--bare", "Shop", "--path", repository.to_str().unwrap()]).await.ok();
    assert!(adopted.contains("adopted as it is"), "{adopted}");
    assert!(adopted.contains("project trust shop"), "{adopted}");
    // Its repository's are read as its own only where it says a folder of it is.
    rig.run(&["project", "blueprints", "shop", "blueprints"]).await.ok();
    // One the factory made itself says nothing of it, here or in the list.
    let made = rig.run(&["project", "create", "--bare", "Depot"]).await.ok();
    assert!(!made.contains("trust"), "{made}");
    let listed = rig.run(&["project", "list"]).await.ok();
    let line = |listed: &str, id: &str| {
        listed.lines().find(|line| line.starts_with(id)).unwrap_or_else(|| panic!("{listed}")).to_string()
    };
    assert!(line(&listed, "shop").ends_with("not trusted"), "{listed}");
    assert!(!line(&listed, "depot").contains("trusted"), "{listed}");

    // Applying the board there gives the one that ships, and says which
    // file was passed over; checking says the same.
    let applied = rig.run(&["apply", "std/board", "--project", "shop"]).await;
    assert_eq!(applied.code, 0, "{}\n{}", applied.out, applied.err);
    assert!(applied.err.contains("note: `shop` has a file of its own for `std/board`"), "{}", applied.err);
    assert!(applied.err.contains("`blueprints/std/board.bp`"), "{}", applied.err);
    assert!(applied.err.contains("project trust shop"), "{}", applied.err);
    let board = rig.run(&["board", "shop"]).await.ok();
    assert!(board.to_lowercase().contains("backlog"), "the board that ships has its columns: {board}");

    // Trusted, its own is what `std/board` is there; and taken back again.
    let trusted = rig.run(&["project", "trust", "shop"]).await.ok();
    assert!(trusted.contains("`shop` is trusted"), "{trusted}");
    assert!(!line(&rig.run(&["project", "list"]).await.ok(), "shop").contains("not trusted"));
    let theirs = rig.run(&["apply", "std/board", "--project", "shop", "--key", "theirs"]).await;
    assert_eq!(theirs.code, 0, "{}\n{}", theirs.out, theirs.err);
    assert!(!theirs.err.contains("note:"), "{}", theirs.err);
    let events = rig.run(&["events", "--topics", "theirs.ran,project.trusted"]).await.ok();
    assert!(events.contains("theirs.ran") && events.contains("project.trusted"), "{events}");

    let distrusted = rig.run(&["project", "distrust", "shop"]).await.ok();
    assert!(distrusted.contains("`shop` is not trusted"), "{distrusted}");
    assert!(line(&rig.run(&["project", "list"]).await.ok(), "shop").ends_with("not trusted"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_repository_whose_blueprints_folder_is_its_own_source_is_read_for_none_of_them_until_a_folder_is_said()
 {
    let rig = Rig::signed_in().await;
    let repository = rig.dir.join("their-source");
    std::fs::create_dir_all(repository.join("blueprints/std")).unwrap();
    std::fs::write(repository.join("blueprints/std/board.bp"), THEIR_BOARD).unwrap();
    rig.run(&["project", "create", "--bare", "Shop", "--path", repository.to_str().unwrap()]).await.ok();
    rig.run(&["project", "trust", "shop"]).await.ok();

    let usual = rig.run(&["project", "blueprints", "shop"]).await.ok();
    assert!(
        usual.contains(
            "`shop` keeps its own blueprints where the factory keeps them for it (`@blueprints/`), as a project \
             does where it says nothing"
        ),
        "{usual}"
    );
    // What ships is applied, and nothing is said of the file there.
    let applied = rig.run(&["apply", "std/board", "--project", "shop"]).await;
    assert_eq!(applied.code, 0, "{}\n{}", applied.out, applied.err);
    assert!(!applied.err.contains("note:"), "{}", applied.err);
    // A fork is written where the factory keeps them, and nothing in the repository.
    let forked = rig.run(&["own", "std/board", "--project", "shop"]).await.ok();
    assert!(forked.contains("`@blueprints/std/board.bp`"), "{forked}");
    assert!(rig.placed("shop").join("std/board.bp").is_file());
    assert_eq!(std::fs::read_to_string(repository.join("blueprints/std/board.bp")).unwrap(), THEIR_BOARD);
    rig.run(&["own", "std/board", "--project", "shop", "--back"]).await.ok();

    let none = rig.run(&["project", "blueprints", "shop", "--none"]).await.ok();
    assert!(none.contains("`shop` keeps no blueprints of its own"), "{none}");
    assert!(
        rig.run(&["project", "blueprints", "shop", "--none"])
            .await
            .ok()
            .contains("`shop` already keeps no blueprints")
    );
    let events = rig.run(&["events", "--topics", "theirs.ran,project.blueprints"]).await.ok();
    assert!(!events.contains("theirs.ran") && events.contains("project.blueprints"), "{events}");
    // No fork is made there, and the sentence says how to have one.
    let forked = rig.run(&["own", "std/board", "--project", "shop"]).await;
    assert_ne!(forked.code, 0);
    assert!(forked.err.contains("keeps no blueprints of its own"), "{}", forked.err);

    // A folder of its repository, read before the factory's place; and one that is not within it.
    let said = rig.run(&["project", "blueprints", "shop", "blueprints"]).await.ok();
    assert!(said.contains("`shop` keeps its own blueprints under `blueprints/` of its repository"), "{said}");
    let theirs = rig.run(&["apply", "std/board", "--project", "shop", "--key", "theirs"]).await.ok();
    assert!(theirs.contains("started"), "{theirs}");
    let out = rig.run(&["project", "blueprints", "shop", "../out"]).await;
    assert_ne!(out.code, 0);
    assert!(out.err.contains("does not stay within the project"), "{}", out.err);
    let both = rig.run(&["project", "blueprints", "shop", "tools/bp", "--none"]).await;
    assert_ne!(both.code, 0);
    assert!(both.err.contains("only one of them"), "{}", both.err);

    // Moved in: copied where the factory keeps them, the repository's left as it was.
    let moved = rig.run(&["project", "blueprints", "shop", "--move-in"]).await.ok();
    assert!(
        moved.contains("and 1 file moved there:")
            && moved.contains("blueprints/std/board.bp -> @blueprints/std/board.bp"),
        "{moved}"
    );
    assert_eq!(std::fs::read_to_string(rig.placed("shop").join("std/board.bp")).unwrap(), THEIR_BOARD);
    assert!(repository.join("blueprints/std/board.bp").is_file());
    let back = rig.run(&["project", "blueprints", "shop", "--usual"]).await.ok();
    assert!(back.contains("`shop` already keeps its own blueprints where the factory keeps them"), "{back}");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_daemon_is_told_a_name_it_answers_to_from_the_terminal_and_the_name_is_taken_back_there() {
    let rig = Rig::signed_in().await;
    let before = rig.run(&["names"]).await.ok();
    assert!(before.contains("it has been told no other name"), "{before}");
    assert!(before.contains("names add <name>"), "it says how to tell it one: {before}");

    // As it is typed after `https://`, port and capitals and all.
    let told = rig.run(&["names", "add", "Box.Example.net:8443"]).await.ok();
    assert!(told.contains("now answers as `box.example.net`"), "{told}");
    assert!(rig.run(&["names"]).await.ok().contains("  box.example.net"));

    // An address is answered at already, and is not a name.
    let address = rig.run(&["names", "add", "203.0.113.7"]).await;
    assert_ne!(address.code, 0);
    assert!(address.err.contains("a host's name, such as `box.example.net`"), "{}", address.err);

    let gone = rig.run(&["names", "remove", "box.example.net"]).await.ok();
    assert!(gone.contains("no longer answers as `box.example.net`"), "{gone}");
    assert!(rig.run(&["names"]).await.ok().contains("it has been told no other name"));
    let twice = rig.run(&["names", "remove", "box.example.net"]).await;
    assert_ne!(twice.code, 0);
    assert!(twice.err.contains("is not a name this daemon was told"), "{}", twice.err);

    let help = rig.run(&["help", "names"]).await.ok();
    assert!(help.contains("names add box.example.net"), "{help}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_key_made_from_the_terminal_holds_what_its_maker_does_unless_told_less_and_never_more() {
    let rig = Rig::signed_in().await;
    // Told no scope, it is its maker's own: the owner's here.
    let whole = rig.run(&["key", "whole"]).await.ok();
    let holds = whole.lines().find_map(|line| line.strip_prefix("It holds: ")).unwrap().to_string();
    assert!(holds.split(", ").any(|scope| scope == "admin"), "{whole}");

    // Told which, it holds those and no others.
    let narrow = rig.run(&["key", "ci", "--scope", "search", "--scope", "board:read"]).await.ok();
    assert!(narrow.contains("It holds: search, board:read"), "{narrow}");
    let token = narrow.lines().next().unwrap().to_string();

    // With that key, a key that holds more is refused, by the name of what
    // it lacks; one with nothing named holds what the key does.
    let wider = rig.run(&["key", "wider", "--scope", "search", "--scope", "shell", "--token", &token]).await;
    assert_ne!(wider.code, 0, "{}", wider.out);
    assert!(wider.err.contains("you do not hold `shell`"), "{}", wider.err);
    let same = rig.run(&["key", "same", "--token", &token]).await.ok();
    assert!(same.contains("It holds: search, board:read"), "{same}");

    let help = rig.run(&["help", "key"]).await.ok();
    assert!(help.contains("A key holds no more than whoever makes it"), "{help}");
}

#[cfg(unix)]
mod listening {
    use std::process::Stdio;

    use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

    use crate::daemon::signal;
    use crate::{BINARY, scratch, soon};

    /// Starts the daemon listening at `listen`, waits until it says it is
    /// running, stops it, and gives back the line that said so and what
    /// it wrote beside its output.
    async fn said_starting_at(listen: &str) -> (String, String) {
        let dir = scratch("listen");
        let mut child = tokio::process::Command::new(BINARY)
            .args(["serve", "--data", dir.join("data").to_str().unwrap(), "--listen", listen])
            .args(["--adapter", "builtin"])
            .env_clear()
            .envs(crate::kept_on_windows())
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("HOME", &dir)
            // Where the command line keeps its session, as `run_in` says it: the
            // owner its first start makes is signed in there.
            .env("XDG_CONFIG_HOME", dir.join("config"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
        let running = soon("the daemon to say it is running", async {
            while let Some(line) = lines.next_line().await.unwrap() {
                if line.contains("is running at") {
                    return line;
                }
            }
            panic!("the daemon's output ended");
        })
        .await;
        signal(child.id().unwrap(), "-TERM");
        let mut said = String::new();
        let mut errors = child.stderr.take().unwrap();
        soon("the daemon to stop", async {
            errors.read_to_string(&mut said).await.unwrap();
            child.wait().await.unwrap();
        })
        .await;
        let _ = std::fs::remove_dir_all(&dir);
        (running, said)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_daemon_that_listens_beyond_this_machine_says_what_that_means_as_it_starts() {
        let (running, said) = said_starting_at("0.0.0.0:0").await;
        assert!(said.contains("warning: listening at 0.0.0.0:"), "{said}");
        assert!(said.contains("which other machines can reach"), "{said}");
        assert!(said.contains("Nothing here is encrypted"), "{said}");
        // Where it says it is running is somewhere to go: this device's
        // own address, and not the one that means every address, which
        // not every system and not every browser will go to.
        assert!(running.contains("is running at http://127.0.0.1:"), "{running}");

        // One that listens on this machine only, as it does unless told,
        // has nothing to warn of.
        let (running, said) = said_starting_at("127.0.0.1:0").await;
        assert!(!said.contains("warning"), "{said}");
        assert!(running.contains("is running at http://127.0.0.1:"), "{running}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_run_set_off_by_something_kept_from_whoever_asks_says_so_from_a_terminal_and_shows_nothing_its_steps_said()
 {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "Life", "--bare"]).await.ok();
    let file = rig.dir.join("watch.bp");
    let watch = "blueprint: watch\npermissions: [projects, records]\nsteps:\n  moods:\n    do: collection.ensure\n    with: { name: moods, reads: private:journal, fields: { title: { type: text, required: true } } }\non:\n  changed:\n    event: record.updated\n    for: { project: life }\n    steps:\n      read: { do: record.get, with: { id: \"${event.data.id}\" } }\n      said: { do: log, with: { message: \"it is called ${steps.read.out.data.title}\" } }\n";
    std::fs::write(&file, watch).unwrap();
    rig.run(&["apply", file.to_str().unwrap(), "--project", "life"]).await.ok();
    rig.run(&["record", "add", "life", "moods", "title=Saturday"]).await.ok();
    let before = crate::fleets::now(&rig);
    rig.run(&["record", "set", "life-moods-1", "title=quince"]).await.ok();
    let ended =
        crate::fleets::event_after(&rig, before, "run.finished", |event| event.data["flow"] == "on.changed")
            .await;
    let run = ended.source.run.clone().unwrap();

    // Its owner reads what the steps printed.
    let whole = rig.run(&["logs", &run]).await.ok();
    assert!(whole.contains("it is called quince") && !whole.contains("kept from you"), "{whole}");

    // Somebody who may read runs and was not given the journal is told
    // that it ran, by which steps, and that the rest is kept from them.
    let member: Vec<&str> = ["key", "member"]
        .into_iter()
        .chain(kernel::principal::MEMBER_SCOPES.iter().flat_map(|scope| ["--scope", *scope]))
        .collect();
    let token = rig.run(&member).await.ok().lines().next().unwrap().to_string();
    for asked in [&["logs", &run][..], &["run", &run]] {
        let shown = rig.run(&[asked, &["--token", &token]].concat()).await.ok();
        assert!(shown.contains("read") && shown.contains("said"), "{shown}");
        assert!(shown.contains("what set this run off is kept from you"), "{shown}");
        assert!(!shown.contains("quince") && !shown.contains("life-moods-1"), "{shown}");
    }
    // And a step's log, asked for by itself, is that of a run that is not there.
    let refused = rig.run(&["logs", &run, "said", "--token", &token]).await;
    assert_eq!(refused.code, 1, "{}", refused.out);
    assert!(refused.err.contains(&format!("there is no run `{run}`")), "{}", refused.err);
}
