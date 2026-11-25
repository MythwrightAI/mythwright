//! Walls: what a program started here is shown of the device.
//!
//! Unix's alone: a wall is held by a program of that system's
//! (`bubblewrap`), told in that system's paths what to show, and what
//! stands in for it here is a file of its shell made a program by its
//! mode. Nothing holds a wall on Windows.
#![cfg(unix)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use hosting::{Shown, Walls};

/// A folder of a test's own, with what the test puts in it.
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("walls-{name}-{}", &kernel::id::secret()[..12]));
    std::fs::create_dir_all(&dir).unwrap();
    dir.canonicalize().unwrap()
}

fn put(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A program to be run, made without this process ever holding it open for
/// writing. These tests share a process with others that start programs,
/// and a program being started holds a copy of every file its parent has
/// open until it is under way: one started in the instant such a file was
/// being written kept it open so, and the system then refused to run it
/// ("Text file busy").
fn program(path: &Path, does: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let made = std::process::Command::new("sh")
        .args(["-c", "printf %s \"$1\" > \"$2\" && chmod 755 \"$2\"", "sh", does])
        .arg(path)
        .status()
        .unwrap();
    assert!(made.success());
}

/// Something that stands in for the program that holds a wall: it is told
/// what the real one is, and starts the command as it is.
fn stand_in(dir: &Path) -> Walls {
    let holder = dir.join("holder");
    program(&holder, "#!/bin/sh\nwhile [ \"$1\" != \"--\" ]; do shift; done\nshift\nexec \"$@\"\n");
    Walls::Held(holder)
}

/// Where in what the holder is told `words` are, one after another.
fn place(told: &[OsString], words: &[&Path]) -> Option<usize> {
    let words: Vec<OsString> = words.iter().map(|word| word.as_os_str().to_os_string()).collect();
    told.windows(words.len()).position(|window| window == words.as_slice())
}

fn told(told: &[OsString], how: &str, what: &Path) -> Option<usize> {
    place(told, &[Path::new(how), what, what])
}

#[test]
fn what_is_said_of_a_folder_holds_for_everything_in_it_that_nothing_nearer_speaks_of() {
    let dir = scratch("nearer");
    let (work, kept, hidden) = (dir.join("work"), dir.join("work/.git"), dir.join("work/.git/key"));
    put(&hidden, "a key");
    let walls = stand_in(&dir);

    // Said in the other order than they hold: the nearer still holds.
    let mut shown = Shown::starting_in(&work);
    shown.hide(&hidden);
    shown.read(&kept);
    shown.change(&work);
    let args = walls.behind(&shown).unwrap();

    let changed = told(&args, "--bind", &work).expect("its work may be changed");
    let read = told(&args, "--ro-bind", &kept).expect("the folder within it may only be read");
    let gone = place(&args, &[Path::new("--ro-bind"), Path::new("/dev/null"), &hidden])
        .expect("the file is not shown");
    assert!(changed < read && read < gone, "the nearer is said later: {args:?}");
    // It is started where it works, and the command follows as it is.
    assert_eq!(
        &args[args.len() - 3..],
        ["--chdir".into(), work.clone().into_os_string(), OsString::from("--")]
    );
    // The network is the device's: nothing takes it away.
    assert!(!args.iter().any(|word| word == "--unshare-net" || word == "--unshare-all"));
    // And what is not there is left out, where it was to be read or changed.
    let mut shown = Shown::starting_in(&work);
    shown.read(&dir.join("nothing"));
    shown.change(&dir.join("nothing-else"));
    let args = walls.behind(&shown).unwrap();
    assert!(!args.iter().any(|word| word.to_string_lossy().contains("nothing")), "{args:?}");
}

#[test]
fn a_folder_that_is_never_shown_whole_is_refused_by_whatever_would_show_it() {
    let dir = scratch("never");
    let (home, data) = (dir.join("home"), dir.join("home/factory"));
    put(&data.join("secret"), "the factory's own");
    put(&home.join("tools/bin/tool"), "a tool");
    let walls = stand_in(&dir);

    for whole in [&home, &dir, &data] {
        let mut shown = Shown::starting_in(&home.join("tools"));
        shown.never(&home);
        shown.never(&data);
        shown.read(whole);
        let refused = walls.behind(&shown).unwrap_err();
        assert!(refused.contains("cannot be shown behind a wall"), "{refused}");
        assert!(refused.contains(&whole.display().to_string()), "{refused}");
    }
    // What is in one of them is shown, where it was itself named.
    let mut shown = Shown::starting_in(&home.join("tools"));
    shown.never(&home);
    shown.read(&home.join("tools"));
    assert!(walls.behind(&shown).is_ok());

    // A folder on the path that would show one of them is left out, and
    // so is one that is another system's; the rest are there to read.
    let mut shown = Shown::starting_in(&home.join("tools"));
    shown.never(&home);
    let path = std::env::join_paths([&home, &home.join("tools/bin"), Path::new("/mnt/elsewhere")]).unwrap();
    shown.path(path.to_str().unwrap());
    let args = walls.behind(&shown).unwrap();
    assert!(told(&args, "--ro-bind", &home.join("tools/bin")).is_some(), "{args:?}");
    assert!(told(&args, "--ro-bind", &home).is_none(), "{args:?}");
    assert!(!args.iter().any(|word| word == "/mnt/elsewhere"), "{args:?}");
}

#[test]
fn a_credential_in_a_folder_that_was_added_is_not_shown_unless_it_was_named() {
    let dir = scratch("credentials");
    let toolchain = dir.join("toolchain");
    put(&toolchain.join("bin/tool"), "a tool");
    put(&toolchain.join("credentials.toml"), "a token to publish with");
    put(&toolchain.join("registry/.netrc"), "another");
    put(&toolchain.join("registry/index"), "what was fetched");
    let walls = stand_in(&dir);
    let null = Path::new("/dev/null");
    let hidden =
        |args: &[OsString], file: &Path| place(args, &[Path::new("--ro-bind"), null, file]).is_some();

    let mut shown = Shown::starting_in(&dir);
    shown.read(&toolchain);
    shown.without_credentials(&toolchain);
    let args = walls.behind(&shown).unwrap();
    assert!(hidden(&args, &toolchain.join("credentials.toml")), "{args:?}");
    assert!(hidden(&args, &toolchain.join("registry/.netrc")), "{args:?}");
    assert!(!hidden(&args, &toolchain.join("registry/index")) && !hidden(&args, &toolchain.join("bin/tool")));

    // Named, it is what was meant.
    let mut shown = Shown::starting_in(&dir);
    shown.read(&toolchain);
    shown.read(&toolchain.join("credentials.toml"));
    shown.without_credentials(&toolchain);
    let args = walls.behind(&shown).unwrap();
    assert!(!hidden(&args, &toolchain.join("credentials.toml")), "{args:?}");
    assert!(hidden(&args, &toolchain.join("registry/.netrc")));
}

#[test]
fn the_program_that_is_run_is_shown_wherever_it_is_and_what_a_link_to_it_leads_to() {
    let dir = scratch("program");
    let (bin, real) = (dir.join("bin"), dir.join("installed/tool/1.2/bin/tool.js"));
    put(&real, "#!/bin/sh\necho ran\n");
    put(&dir.join("installed/tool/1.2/lib/part"), "what it is made of");
    std::fs::create_dir_all(&bin).unwrap();
    std::os::unix::fs::symlink(&real, bin.join("tool")).unwrap();
    let walls = stand_in(&dir);
    let path = bin.to_str().unwrap();

    // Found along the path: the file the link leads to is shown.
    let mut shown = Shown::starting_in(&dir);
    shown.program("tool", path, &[]);
    let args = walls.behind(&shown).unwrap();
    assert!(told(&args, "--ro-bind", &real).is_some(), "{args:?}");
    assert!(told(&args, "--ro-bind", &dir.join("installed/tool/1.2")).is_none());

    // One that is a file among several says how far its own files go.
    let mut shown = Shown::starting_in(&dir);
    shown.program("tool", path, &["..".to_string()]);
    let args = walls.behind(&shown).unwrap();
    assert!(told(&args, "--ro-bind", &dir.join("installed/tool/1.2")).is_some(), "{args:?}");

    // Never so far as to show a folder that is not shown whole.
    let mut shown = Shown::starting_in(&dir);
    shown.never(&dir.join("installed"));
    shown.program("tool", path, &["../../..".to_string()]);
    let args = walls.behind(&shown).unwrap();
    assert!(told(&args, "--ro-bind", &dir.join("installed")).is_none(), "{args:?}");
    assert!(told(&args, "--ro-bind", &real).is_some());

    // And one named by where it is, is that file.
    let mut shown = Shown::starting_in(&dir);
    shown.program(real.to_str().unwrap(), "", &[]);
    assert!(told(&walls.behind(&shown).unwrap(), "--ro-bind", &real).is_some());
}

#[test]
fn a_wall_that_cannot_be_held_says_why_and_is_never_the_open_device_instead() {
    let dir = scratch("cannot");
    let shown = Shown::starting_in(&dir);

    let missing = Walls::Held(dir.join("no-such-program"));
    let why = missing.can().unwrap_err();
    assert!(why.contains("bubblewrap") && why.contains("no-such-program"), "{why}");
    assert_eq!(missing.behind(&shown).unwrap_err(), why);

    // One that is there and is refused says what the system said.
    let refused = dir.join("refused");
    program(&refused, "#!/bin/sh\necho 'setting up uid map: Permission denied' >&2\nexit 1\n");
    let why = Walls::Held(refused).can().unwrap_err();
    assert!(why.contains("setting up uid map: Permission denied"), "{why}");
    assert!(why.contains("the system would not let it"), "{why}");

    let open = Walls::Open;
    assert!(open.can().unwrap_err().contains("holding no walls"));
    assert!(open.behind(&shown).is_err());
}

/// Runs `script` behind a real wall showing `shown`, and gives what it printed.
fn behind(walls: &Walls, shown: &Shown, home: &Path, script: &str) -> String {
    let args = walls.behind(shown).unwrap();
    let output = std::process::Command::new(&args[0])
        .args(&args[1..])
        .args(["/bin/sh", "-c", script])
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .env("HOME", home)
        .output()
        .unwrap();
    format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr))
}

#[test]
fn behind_a_wall_a_program_reads_what_it_was_shown_changes_what_it_may_and_finds_nothing_else() {
    let walls = Walls::system();
    if let Err(why) = walls.can() {
        // Said, and not counted as held: the tests above hold what is asked
        // of the system wherever they run; this one needs it to answer.
        println!("skipped: this system holds no wall ({why})");
        return;
    }
    let dir = scratch("real");
    let (work, home, own) = (dir.join("project/lane"), dir.join("home"), dir.join("slot/home"));
    put(&dir.join("project/README.md"), "the project");
    put(&dir.join("project/.git/config"), "[core]");
    put(&work.join("notes.txt"), "its work");
    put(&dir.join("data/secret"), "the factory's own");
    put(&home.join(".ssh/id_rsa"), "a private key");
    put(&home.join(".toolchain/bin/tool"), "a tool");
    put(&home.join(".toolchain/credentials.toml"), "a token");
    std::fs::create_dir_all(&own).unwrap();

    let mut shown = Shown::starting_in(&work);
    shown.never(&home);
    shown.never(&dir.join("data"));
    shown.put(&own, &home);
    shown.read(&dir.join("project"));
    shown.change(&work);
    shown.read(&home.join(".toolchain"));
    shown.without_credentials(&home.join(".toolchain"));

    let at = |path: &str| dir.join(path).display().to_string();
    let script = format!(
        "pwd
         cat {readme} && echo ' :read'
         cat {tool} && echo ' :read'
         echo more >> notes.txt && echo 'its work :changed'
         echo made > \"$HOME\"/made && echo 'its home :changed'
         cat {secret} 2>/dev/null || echo 'the secret :not there'
         cat {key} 2>/dev/null || echo 'the key :not there'
         test -s {token} || echo 'the token :not shown'
         echo x 2>/dev/null >> {readme} || echo 'the project :not changed'
         echo x 2>/dev/null >> {config} || echo 'the repository :not changed'
         echo x 2>/dev/null > {planted} || echo 'the toolchain :not changed'
         ls /proc | grep -c '^[0-9]'",
        readme = at("project/README.md"),
        tool = at("home/.toolchain/bin/tool"),
        secret = at("data/secret"),
        key = at("home/.ssh/id_rsa"),
        token = at("home/.toolchain/credentials.toml"),
        config = at("project/.git/config"),
        planted = at("home/.toolchain/planted"),
    );
    let said = behind(&walls, &shown, &home, &script);
    let lines: Vec<&str> = said.lines().map(str::trim).collect();
    assert_eq!(lines[0], work.display().to_string(), "{said}");
    for expected in [
        "the project :read",
        "a tool :read",
        "its work :changed",
        "its home :changed",
        "the secret :not there",
        "the key :not there",
        "the token :not shown",
        "the project :not changed",
        "the repository :not changed",
        "the toolchain :not changed",
    ] {
        assert!(lines.contains(&expected), "expected `{expected}` in:\n{said}");
    }
    // It sees the processes it started and no others.
    let processes: usize = lines.last().unwrap().parse().unwrap_or(usize::MAX);
    assert!(processes < 10, "{said}");
    // What it changed is changed for real, and its home is its own folder.
    assert_eq!(std::fs::read_to_string(work.join("notes.txt")).unwrap(), "its workmore\n");
    assert_eq!(std::fs::read_to_string(own.join("made")).unwrap(), "made\n");
    assert!(!home.join("made").exists() && !home.join(".toolchain/planted").exists());
    let _ = std::fs::remove_dir_all(&dir);
}
