//! A package on each system there is one for: what it is called, what its
//! programs are called and where cargo put them, what makes the one file
//! of it, and what its page says. All of it asked as functions, of all
//! three systems, on whichever this is.

use xtask::package::{self, About, System};

const ALL: [System; 3] = [System::Linux, System::Macos, System::Windows];

fn words(asked: &[String]) -> String {
    asked.join(" ")
}

#[test]
fn a_package_is_called_for_its_system_and_its_processor() {
    let cases = [
        (System::Linux, "x86_64", "sample-1.2.3-linux-x86_64"),
        (System::Linux, "aarch64", "sample-1.2.3-linux-aarch64"),
        (System::Macos, "aarch64", "sample-1.2.3-macos-aarch64"),
        (System::Macos, "x86_64", "sample-1.2.3-macos-x86_64"),
        (System::Windows, "x86_64", "sample-1.2.3-windows-x86_64"),
        (System::Windows, "aarch64", "sample-1.2.3-windows-aarch64"),
    ];
    for (system, processor, called) in cases {
        assert_eq!(package::named("sample", "1.2.3", system, processor), called);
    }
    // The one file of it is what its system opens by itself.
    assert_eq!(ALL.map(System::archive), [".tar.gz", ".tar.gz", ".zip"]);
    assert_eq!(ALL.map(System::called), ["Linux", "macOS", "Windows"]);
}

#[test]
fn a_system_is_known_by_the_word_the_compiler_has_for_it_and_any_other_is_told_what_there_is() {
    assert_eq!(package::on("linux"), Ok(System::Linux));
    assert_eq!(package::on("macos"), Ok(System::Macos));
    assert_eq!(package::on("windows"), Ok(System::Windows));
    for system in ALL {
        assert_eq!(System::of(system.word()), Some(system));
    }
    let refusal = package::on("freebsd").unwrap_err();
    assert!(refusal.contains("made on and for Linux, macOS and Windows, and this is freebsd"), "{refusal}");
    assert!(refusal.contains("cargo xtask install"), "{refusal}");
}

#[test]
fn a_target_says_its_system_and_its_processor_as_cargo_names_one() {
    let cases = [
        ("x86_64-pc-windows-gnu", System::Windows, "x86_64"),
        ("x86_64-pc-windows-msvc", System::Windows, "x86_64"),
        ("aarch64-pc-windows-msvc", System::Windows, "aarch64"),
        ("aarch64-apple-darwin", System::Macos, "aarch64"),
        ("x86_64-apple-darwin", System::Macos, "x86_64"),
        ("x86_64-unknown-linux-gnu", System::Linux, "x86_64"),
        ("aarch64-unknown-linux-musl", System::Linux, "aarch64"),
    ];
    for (triple, system, processor) in cases {
        assert_eq!(package::target(triple), Ok((system, processor.to_string())), "{triple}");
    }
    for nothing in ["wasm32-unknown-unknown", "x86_64-unknown-freebsd", "windows", "", "-pc-windows-gnu"] {
        let refusal = package::target(nothing).unwrap_err();
        assert!(refusal.contains(&format!("`{nothing}` does not say a system")), "{refusal}");
        assert!(refusal.contains("x86_64-pc-windows-gnu"), "it says how one is written: {refusal}");
    }
}

#[test]
fn a_program_is_called_what_its_system_calls_one_and_is_where_cargo_put_it() {
    assert_eq!(ALL.map(|system| system.program("sm")), ["sm", "sm", "sm.exe"]);
    assert_eq!(ALL.map(|system| system.program("factory")), ["factory", "factory", "factory.exe"]);
    let cases = [
        (None, "release", "target/release"),
        (None, "fast", "target/fast"),
        // Cargo calls the profile `dev` and its folder `debug`.
        (None, "dev", "target/debug"),
        (Some("x86_64-pc-windows-gnu"), "release", "target/x86_64-pc-windows-gnu/release"),
        (Some("aarch64-apple-darwin"), "release", "target/aarch64-apple-darwin/release"),
        (Some("x86_64-unknown-linux-gnu"), "dev", "target/x86_64-unknown-linux-gnu/debug"),
    ];
    for (target, profile, folder) in cases {
        assert_eq!(package::built_in(target, profile), folder);
    }
}

#[test]
fn cargo_is_asked_to_build_for_the_target_said_by_the_word_said() {
    let cases = [
        ("build", None, "cli", "build --profile release -p cli"),
        ("build", None, "desktop", "build --profile release -p desktop"),
        (
            "zigbuild",
            Some("x86_64-pc-windows-gnu"),
            "cli",
            "zigbuild --profile release -p cli --target x86_64-pc-windows-gnu",
        ),
        (
            "build",
            Some("aarch64-apple-darwin"),
            "desktop",
            "build --profile release -p desktop --target aarch64-apple-darwin",
        ),
    ];
    for (word, target, krate, asked) in cases {
        assert_eq!(words(&package::build(word, "release", target, krate)), asked);
    }
}

#[test]
fn the_one_file_is_asked_of_each_systems_own_tar_so_that_it_is_the_same_whoever_makes_it() {
    // Linux's sorts by name itself and is told the owner as it always was.
    assert_eq!(
        words(&package::tar(System::Linux, "/out/sample.tar.gz", "sample", "sample.files").unwrap()),
        "--create --gzip --owner=0 --group=0 --numeric-owner --sort=name --file /out/sample.tar.gz sample"
    );
    // macOS's cannot sort: it goes into no folder by itself and is handed
    // the names in order. Nobody's in particular there too.
    assert_eq!(
        words(&package::tar(System::Macos, "/out/sample.tar.gz", "sample", "sample.files").unwrap()),
        "-c -z --uid 0 --gid 0 --numeric-owner --no-recursion -f /out/sample.tar.gz -T sample.files"
    );
    // On Windows no file says it can be run, so a package whose programs
    // have to say so is not made there.
    let refusal = package::tar(System::Windows, "sample.tar.gz", "sample", "sample.files").unwrap_err();
    assert!(refusal.contains("a package for Linux or macOS is not made on Windows"), "{refusal}");
    assert!(refusal.contains("Make it on the system it is for"), "{refusal}");
}

#[test]
fn everything_in_a_package_is_named_in_order_each_folder_before_what_it_holds() {
    let files: Vec<String> = [
        "LICENSE-MIT",
        "extensions/first/extension.yaml",
        "extensions/first/notes/more.txt",
        "sm",
        "ui/index.html",
    ]
    .map(str::to_string)
    .to_vec();
    assert_eq!(
        package::listed("sample", &files),
        [
            "sample",
            "sample/LICENSE-MIT",
            "sample/extensions",
            "sample/extensions/first",
            "sample/extensions/first/extension.yaml",
            "sample/extensions/first/notes",
            "sample/extensions/first/notes/more.txt",
            "sample/sm",
            "sample/ui",
            "sample/ui/index.html",
        ]
    );
    assert_eq!(package::listed("sample", &[]), ["sample"]);
}

#[test]
fn a_package_built_on_another_system_than_it_is_for_says_so_where_the_daemon_reads_what_it_is() {
    let cases = [
        (Some("revision abc1234"), None, Some("revision abc1234")),
        (
            Some("revision abc1234"),
            Some((System::Linux, System::Windows)),
            Some("revision abc1234, built on Linux for Windows, for trying and never a release"),
        ),
        (
            None,
            Some((System::Linux, System::Macos)),
            Some("built on Linux for macOS, for trying and never a release"),
        ),
        (None, None, None),
    ];
    for (revision, elsewhere, said) in cases {
        assert_eq!(package::note(revision, elsewhere, None).as_deref(), said);
    }
}

#[test]
fn a_nightly_is_called_the_same_every_night_and_says_inside_it_which_night_and_commit_it_is() {
    // Each night's files replace the last night's under the same names.
    let cases = [
        (System::Linux, "x86_64", "sample-nightly-linux-x86_64", ".tar.gz"),
        (System::Macos, "aarch64", "sample-nightly-macos-aarch64", ".tar.gz"),
        (System::Windows, "x86_64", "sample-nightly-windows-x86_64", ".zip"),
    ];
    for (system, processor, called, archive) in cases {
        assert_eq!(package::named("sample", package::NIGHTLY, system, processor), called);
        assert_eq!(system.archive(), archive);
    }
    // Its version is the program's with the night and the commit, as
    // semantic versioning writes a pre-release and a build.
    let version = package::nightly_version("1.2.3", "20261124", "abc1234");
    assert_eq!(version, "1.2.3-nightly.20261124+abc1234");
    // Which the daemon reads first of what the package says beside it.
    let cases = [
        (Some("revision abc1234"), None, "1.2.3-nightly.20261124+abc1234, a nightly, revision abc1234"),
        (
            Some("revision abc1234"),
            Some((System::Linux, System::Windows)),
            "1.2.3-nightly.20261124+abc1234, a nightly, revision abc1234, built on Linux for Windows, for trying and never a release",
        ),
        (None, None, "1.2.3-nightly.20261124+abc1234, a nightly"),
    ];
    for (revision, elsewhere, said) in cases {
        assert_eq!(package::note(revision, elsewhere, Some(&version)).as_deref(), Some(said));
    }
    // And its page, on every system, says it is one and what that means.
    for system in ALL {
        let page = package::readme(&About {
            version: "1.2.3-nightly.20261124+abc1234",
            nightly: true,
            ..about(system)
        });
        assert!(
            page.starts_with(&format!("# Sample 1.2.3-nightly.20261124+abc1234 for {}", system.called())),
            "{page}"
        );
        for wanted in [
            "It is a nightly: built from the main line",
            "a pinned release is the other line",
            "what moving back takes",
        ] {
            assert!(page.contains(wanted), "`{wanted}` in\n{page}");
        }
        let pinned = package::readme(&about(system));
        assert!(!pinned.contains("nightly"), "{pinned}");
    }
}

#[test]
fn a_nightly_is_of_a_day_written_as_one_and_nothing_else_is_taken_for_one() {
    for day in ["20261124", "20240229", "20000101", "20261231"] {
        assert_eq!(package::nightly_day(day), Ok(day));
    }
    for not in [
        "",
        "2026-11-24",
        "2026112",
        "202611240",
        "20261324",
        "20261100",
        "20261131",
        "20250229",
        "19991231",
        "2026112x",
        "+0261124",
    ] {
        let refused = package::nightly_day(not).unwrap_err();
        assert!(refused.contains("YYYYMMDD") && refused.contains(&format!("`{not}`")), "{not}: {refused}");
    }
}

fn about(system: System) -> About<'static> {
    About {
        display: "Sample",
        cli: "sm",
        id: "sample",
        version: "1.2.3",
        system,
        processor: "x86_64",
        revision: Some("revision abc1234"),
        built_on: "Some System 1",
        elsewhere: None,
        window: true,
        nightly: false,
        address: "http://127.0.0.1:7700",
    }
}

/// What a page says only of another system than its own: a command, a
/// path or a thing needed that is not there.
fn of_another(system: System) -> Vec<&'static str> {
    let linux = ["WebKitGTK", "libwebkit2gtk", "menu.sh", "C library"];
    let unix = ["./sm", "~/.local/share", "~/.config", "ln -sf", "rm -rf"];
    let macos = ["xattr", "notarised", "xcode-select", "Apple", ".zprofile"];
    let windows =
        [".exe", "%LOCALAPPDATA%", "%APPDATA%", "WebView2", "Windows protected your PC", "Remove-Item"];
    match system {
        System::Linux => [&macos[..], &windows[..], &["--walls off", "--no-wall"]].concat(),
        System::Macos => [&linux[..], &windows[..]].concat(),
        System::Windows => [&linux[..], &unix[..], &macos[..]].concat(),
    }
}

#[test]
fn the_page_of_each_system_names_that_systems_commands_paths_and_needs_and_no_others() {
    let wanted: [(System, &[&str]); 3] = [
        (
            System::Linux,
            &[
                "# Sample 1.2.3 for Linux (x86_64)",
                "./sample\n",
                "./sm up &",
                "libwebkit2gtk-4.1-0",
                "./menu.sh",
                "~/.local/share/sample",
                "~/.config/sample",
                "Built on Some System 1. This is revision abc1234.",
                "LICENSE-MIT",
                "LICENSE-APACHE",
                "`THIRD-PARTY-NOTICES.md` names each",
            ],
        ),
        (
            System::Macos,
            &[
                "# Sample 1.2.3 for macOS (x86_64)",
                "./sm up &",
                "./sm open",
                "./sm down",
                "`./sample` is the window",
                "not an application",
                "xattr -dr com.apple.quarantine .",
                "**Open Anyway**",
                "not signed by a developer Apple knows, and are not\nnotarised",
                "xcode-select --install",
                "Nothing holds a wall on macOS",
                "./sm up --walls off",
                "`./sm agent add <name> --no-wall`",
                "ln -sf \"$PWD/sm\" ~/.local/bin/sm",
                "~/.zprofile",
                "~/.local/share/sample",
                "~/.config/sample",
                "Built on Some System 1. This is revision abc1234. It is for a Mac with an Intel processor.",
                "LICENSE-MIT",
                "LICENSE-APACHE",
                "`THIRD-PARTY-NOTICES.md` names each",
            ],
        ),
        (
            System::Windows,
            &[
                "# Sample 1.2.3 for Windows (x86_64)",
                ".\\sm.exe up\n",
                ".\\sm.exe open\n",
                "`.\\sm.exe down`",
                "`sample.exe` is the window",
                "there is no installer",
                "nothing is put in the Start menu or on your\npath",
                "The programs here are not signed.",
                "**Windows protected your PC**: choose **More info**, and then\n**Run anyway**",
                "Git for Windows, with `git` on your path",
                "WebView2 runtime, which comes with Windows 11",
                "VCRUNTIME140.dll",
                "Nothing holds a wall on Windows",
                ".\\sm.exe up --walls off",
                "`.\\sm.exe agent add <name> --no-wall`",
                "`%LOCALAPPDATA%\\sample`",
                "`%APPDATA%\\sample`",
                "\"$env:LOCALAPPDATA\\sample*\", \"$env:APPDATA\\sample\"",
                "http://127.0.0.1:7700",
                "Built on Some System 1. This is revision abc1234. Not signed.",
                "LICENSE-MIT",
                "LICENSE-APACHE",
                "`THIRD-PARTY-NOTICES.md` names each",
            ],
        ),
    ];
    for (system, wanted) in wanted {
        let page = package::readme(&about(system));
        for wanted in wanted {
            assert!(page.contains(wanted), "`{wanted}` in the page for {}:\n{page}", system.called());
        }
        for another in of_another(system) {
            assert!(!page.contains(another), "`{another}` is not {}'s:\n{page}", system.called());
        }
        // As it is released, it says nothing of being anything else.
        assert!(!page.contains("not a release") && !page.contains("for trying"), "{page}");
        assert!(page.contains("A preview"), "{page}");
    }
}

#[test]
fn each_systems_page_names_the_notes_of_the_first_run_and_they_say_how_a_command_is_typed_there() {
    let walk = "# The first run\n\nThe [README](../README.md) has the same path; see [extensions.md](extensions.md),\n\
                and [the experiment](#9-an-experiment) or [a page](https://example.test/a).\n\n\
                ```sh\nsm example add research --project shop\n```\n\nA list `[a, b]` and a call `f(x)` are not links.\n";
    // The system; how its command is typed in the folder; what its notes say only where they were not written for it.
    let cases: [(System, &str, Option<&str>); 3] = [
        (System::Linux, "./sm", None),
        (System::Macos, "./sm", Some("It was written on Linux, the system it is tested on.")),
        (System::Windows, ".\\sm.exe", Some("`README.md` beside this has this system's.")),
    ];
    for (system, command, elsewhere) in cases {
        let about = about(system);
        // The page names the file, and says how what it writes is typed here.
        let page = package::readme(&about);
        assert!(
            page.contains(&format!("`{}`, in this folder, is the longer walk", package::NOTES)),
            "{page}"
        );
        assert!(page.contains(&format!("as `sm ...`, which here is `{command} ...`")), "{page}");

        let notes = package::notes(walk, &about);
        assert!(notes.starts_with(
            "# The first run\n\n> The notes of the first run from the source this package of Sample"
        ));
        assert_eq!(notes.matches("# The first run").count(), 1, "one heading: {notes}");
        assert!(notes.contains(&format!("in this\n> folder that is `{command} ...`")), "{notes}");
        assert!(notes.contains(&format!("`{command} help` lists the topics")), "{notes}");
        match elsewhere {
            Some(said) => assert!(notes.contains(said), "{notes}"),
            None => assert!(!notes.contains("It was written on"), "{notes}"),
        }
        // A link to a page the package does not hold is its words alone; one within the page or to an address stays.
        assert!(notes.contains("The README has the same path; see extensions.md,"), "{notes}");
        assert!(!notes.contains("../README.md") && !notes.contains("](extensions.md)"), "{notes}");
        assert!(
            notes.contains("[the experiment](#9-an-experiment) or [a page](https://example.test/a)."),
            "{notes}"
        );
        // And nothing else of it is touched: its commands are the source's.
        assert!(notes.contains("```sh\nsm example add research --project shop\n```"), "{notes}");
        assert!(notes.ends_with("A list `[a, b]` and a call `f(x)` are not links.\n"), "{notes}");
    }
}

#[test]
fn the_notes_a_package_carries_are_the_sources_own_with_no_link_to_a_page_it_does_not_hold() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/first-run.md"),
    )
    .expect("the notes of the first run are in the repository");
    for system in [System::Linux, System::Macos, System::Windows] {
        let notes = package::notes(&source, &about(system));
        for (at, _) in notes.match_indices("](") {
            let to = &notes[at + 2..];
            assert!(
                to.starts_with('#') || to.starts_with("http"),
                "a link out of the package: {}",
                &to[..to.len().min(60)]
            );
        }
        // Every line of the source's is there as it was, but for the lines a
        // link was on and the block of settings at its head, which says the
        // page is public and is about the file.
        let read = source
            .strip_prefix("---\n")
            .and_then(|after| after.split_once("\n---\n"))
            .map_or(source.as_str(), |(_, rest)| rest);
        assert!(!notes.contains("public: true"), "{notes}");
        let kept = read.lines().filter(|line| !line.contains("](")).all(|line| notes.contains(line));
        assert!(kept, "a line of the source's notes is not in the package's");
        assert!(notes.contains("## 9. An experiment: two ways of asking"), "{notes}");
    }
}

#[test]
fn no_page_says_the_product_was_tested_where_it_was_not() {
    let linux = package::readme(&about(System::Linux));
    assert!(linux.contains("It is tested on Linux"), "{linux}");
    // Windows was started by hand, once, and its suites run by hand; macOS never.
    let windows = package::readme(&about(System::Windows));
    for wanted in [
        "Its tests are run on Linux on\nevery change. On Windows they have been run by hand, on one machine, and by\nnothing that runs on every change",
        "started by hand, once, on\none machine",
        "The window's program has been run there\nonly where it opens no window",
    ] {
        assert!(windows.contains(wanted), "`{wanted}` in\n{windows}");
    }
    let macos = package::readme(&about(System::Macos));
    for wanted in ["one nobody has run", "On macOS nothing of it has been run", "It has not been run."] {
        assert!(macos.contains(wanted), "`{wanted}` in\n{macos}");
    }
    for page in [&windows, &macos] {
        for claim in ["tested on Windows", "tested on macOS", "It is tested", "works on", "supported"] {
            assert!(!page.contains(claim), "`{claim}` in\n{page}");
        }
    }
}

#[test]
fn a_page_says_which_mac_a_package_is_for() {
    let silicon = package::readme(&About { processor: "aarch64", ..about(System::Macos) });
    assert!(silicon.contains("# Sample 1.2.3 for macOS (aarch64)"), "{silicon}");
    assert!(
        silicon.contains(
            "It is for a Mac with Apple silicon, and does not start on one with an Intel processor."
        ),
        "{silicon}"
    );
}

#[test]
fn a_package_built_somewhere_else_or_without_the_window_says_so_on_its_page() {
    // Built on Linux for Windows, without the window: what is made here to
    // be tried there.
    let tried =
        package::readme(&About { elsewhere: Some(System::Linux), window: false, ..about(System::Windows) });
    for wanted in [
        "**This one is not a release.**",
        "It was built on Linux for Windows",
        "it is for trying Sample on Windows, and is never what is released",
        "It was made without the window's program: there is no `sample.exe` in this folder.",
        ".\\sm.exe up\n",
    ] {
        assert!(tried.contains(wanted), "`{wanted}` in\n{tried}");
    }
    // Nothing is said of what the window needs, or of what a program
    // built on Windows itself would need.
    for gone in ["WebView2", "is the window", "VCRUNTIME140.dll"] {
        assert!(!tried.contains(gone), "`{gone}` in\n{tried}");
    }
    for system in [System::Linux, System::Macos] {
        let page = package::readme(&About { elsewhere: Some(System::Windows), ..about(system) });
        assert!(page.contains(&format!("It was built on Windows for {}", system.called())), "{page}");
        let page = package::readme(&About { window: false, ..about(system) });
        assert!(page.contains("It was made without the window's program: there is no `sample`"), "{page}");
    }
    let linux = package::readme(&About { window: false, ..about(System::Linux) });
    assert!(linux.contains("no `sample` and no `menu.sh` in this folder"), "{linux}");
    let macos = package::readme(&About { window: false, ..about(System::Macos) });
    assert!(!macos.contains("is the window"), "{macos}");
}
