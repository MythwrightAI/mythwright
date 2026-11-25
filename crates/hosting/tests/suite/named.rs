//! The file a program's name comes to, where the system finds a program
//! by more than its name (Windows), and why one was not started.
//!
//! Functions of a name, folders and endings, played here with folders
//! made for it: nothing of this has been run on the system it is for.
//! One thing differs there and is not played: that system does not tell
//! `TOOL.CMD` from `tool.cmd`, so the endings are written here as the
//! files are.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use hosting::named::{USUAL, endings, file, found, is_script, why_not_on};

/// Folders of programs as an installer leaves them: `one` has a tool as
/// a script of the system's shell, with the script for another shell
/// beside it that is called the bare name; `two` has it as a program.
fn folders() -> PathBuf {
    let top =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("named-{}", &kernel::id::secret()[..12]));
    for file in [
        "one/tool",
        "one/tool.cmd",
        "one/other.EXE",
        "two/tool.exe",
        "two/late.bat",
        "two/whole.exe",
        "two/whole.exe.cmd",
        "three/only",
        "four/both.exe",
        "four/both.cmd",
    ] {
        let file = top.join(file);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, "").unwrap();
    }
    top
}

struct Case {
    what: &'static str,
    name: &'static str,
    /// Folders of the path, each under the top; one that begins with
    /// `/` is said from the top as a whole path.
    folders: &'static [&'static str],
    endings: &'static [&'static str],
    /// The file it comes to, under the top.
    file: Option<&'static str>,
}

const ALL: &[&str] = &[".exe", ".cmd", ".bat"];

const CASES: &[Case] = &[
    Case {
        what: "a script of the system's shell is found by the bare name, and the file of that very name is not it",
        name: "tool",
        folders: &["one", "two"],
        endings: ALL,
        file: Some("one/tool.cmd"),
    },
    Case {
        what: "the first folder that has one says, whatever a later folder holds",
        name: "tool",
        folders: &["two", "one"],
        endings: ALL,
        file: Some("two/tool.exe"),
    },
    Case {
        what: "a file that ends in nothing the list names is no program there",
        name: "tool",
        folders: &["one"],
        endings: &[".exe"],
        file: None,
    },
    Case {
        what: "nor is the file of the bare name where it is alone",
        name: "only",
        folders: &["three"],
        endings: ALL,
        file: None,
    },
    Case {
        what: "a name said with its ending is that file and no other",
        name: "tool.cmd",
        folders: &["two", "one"],
        endings: ALL,
        file: Some("one/tool.cmd"),
    },
    Case {
        what: "and is never given a second ending",
        name: "whole.exe",
        folders: &["two"],
        endings: &[".cmd", ".exe"],
        file: Some("two/whole.exe"),
    },
    Case {
        what: "an ending is the same in either case",
        name: "other.EXE",
        folders: &["one"],
        endings: &[".exe"],
        file: Some("one/other.EXE"),
    },
    Case {
        what: "of two in one folder the list's order says which",
        name: "both",
        folders: &["four"],
        endings: &[".exe", ".cmd"],
        file: Some("four/both.exe"),
    },
    Case {
        what: "and the other where the list is the other way round",
        name: "both",
        folders: &["four"],
        endings: &[".cmd", ".exe"],
        file: Some("four/both.cmd"),
    },
    Case {
        what: "the list's later endings are come to",
        name: "late",
        folders: &["one", "two"],
        endings: ALL,
        file: Some("two/late.bat"),
    },
    Case {
        what: "what is nowhere is nothing",
        name: "missing",
        folders: &["one", "two"],
        endings: ALL,
        file: None,
    },
    Case { what: "no name is nothing", name: "", folders: &["one"], endings: ALL, file: None },
    Case { what: "no endings find nothing", name: "tool", folders: &["one"], endings: &[], file: None },
    Case { what: "no folders find nothing", name: "tool", folders: &[], endings: ALL, file: None },
    Case {
        what: "a folder that is nothing (two marks together in a path) is passed over, not taken for here",
        name: "tool",
        folders: &["", "one"],
        endings: ALL,
        file: Some("one/tool.cmd"),
    },
    Case {
        what: "a name that says where it is from the top is looked for there, with the endings",
        name: "/one/tool",
        folders: &["two"],
        endings: ALL,
        file: Some("one/tool.cmd"),
    },
    Case {
        what: "and is itself where it says its ending",
        name: "/two/whole.exe",
        folders: &[],
        endings: ALL,
        file: Some("two/whole.exe"),
    },
    Case {
        what: "and nothing where no file is there",
        name: "/three/only",
        folders: &["one"],
        endings: ALL,
        file: None,
    },
    Case {
        what: "a name that says where it is from wherever the program runs is not looked for",
        name: "one/tool",
        folders: &["one", "two"],
        endings: ALL,
        file: None,
    },
];

#[test]
fn a_programs_name_comes_to_a_file_by_the_folders_and_the_endings() {
    let top = folders();
    for case in CASES {
        // Said from the top, a name is a whole path there.
        let name = match case.name.strip_prefix('/') {
            Some(under) => top.join(under).to_string_lossy().into_owned(),
            None => case.name.to_string(),
        };
        let folders: Vec<PathBuf> = case
            .folders
            .iter()
            .map(|folder| if folder.is_empty() { PathBuf::new() } else { top.join(folder) })
            .collect();
        let endings: Vec<String> = case.endings.iter().map(|ending| ending.to_string()).collect();
        let came_to = file(&name, &folders, &endings);
        assert_eq!(came_to, case.file.map(|file| top.join(file)), "{}", case.what);
    }
    let _ = std::fs::remove_dir_all(&top);
}

#[test]
fn the_endings_are_the_systems_list_in_its_order_and_the_usual_ones_where_it_names_none() {
    let usual: Vec<String> = USUAL.iter().map(|ending| ending.to_string()).collect();
    let cases: &[(Option<&str>, &[&str])] = &[
        (None, USUAL),
        (Some(""), USUAL),
        (Some(".COM;.EXE;.BAT;.CMD;.VBS"), &[".COM", ".EXE", ".BAT", ".CMD", ".VBS"]),
        (Some(".cmd;.exe"), &[".cmd", ".exe"]),
        (Some(" .exe ; .cmd ;;"), &[".exe", ".cmd"]),
        // An ending is a dot and something after it.
        (Some("exe;.;;"), USUAL),
        (Some("exe;.py"), &[".py"]),
    ];
    for (listed, are) in cases {
        assert_eq!(endings(*listed), *are, "{listed:?}");
    }
    assert_eq!(usual, [".COM", ".EXE", ".BAT", ".CMD"]);
}

#[test]
fn a_program_is_found_from_the_two_lists_as_a_system_writes_them() {
    let top = folders();
    let path = std::env::join_paths([top.join("three"), top.join("one"), top.join("two")]).unwrap();
    let came_to = |name: &str, pathext: Option<&str>| found(name, Some(path.as_os_str()), pathext);
    assert_eq!(came_to("tool", Some(".exe;.cmd")), Some(top.join("one/tool.cmd")));
    assert_eq!(came_to("late", Some(".exe;.cmd;.bat")), Some(top.join("two/late.bat")));
    assert_eq!(came_to("late", Some(".exe;.cmd")), None);
    // The usual endings are written in capitals, and a system that tells
    // them from small letters, as the one this is played on does, finds
    // only a file written so.
    assert_eq!(came_to("other", None), Some(top.join("one/other.EXE")));
    assert_eq!(found("tool", None, Some(".cmd")), None, "a program was found with no path to look along");
    let _ = std::fs::remove_dir_all(&top);
}

#[test]
fn a_script_of_the_systems_shell_is_told_by_how_its_name_ends() {
    for (file, is) in [
        ("C:\\tools\\tool.cmd", true),
        ("tool.CMD", true),
        ("tool.bat", true),
        ("tool.Bat", true),
        ("tool.exe", false),
        ("tool", false),
        ("cmd", false),
        (".cmd", false),
        ("tool.cmd.exe", false),
    ] {
        assert_eq!(is_script(Path::new(file)), is, "{file}");
    }
}

#[test]
fn a_script_refused_its_arguments_is_said_to_be_one_and_everything_else_in_the_systems_words() {
    use std::io::{Error, ErrorKind};
    let refused = || Error::new(ErrorKind::InvalidInput, "batch file arguments are invalid");
    let missing = || Error::new(ErrorKind::NotFound, "program not found");

    let said = why_not_on(true, OsStr::new("C:\\tools\\tool.cmd"), &refused());
    for words in [
        "a script of the system's shell (C:\\tools\\tool.cmd)",
        "cannot be written there so that it means only what it says",
        "batch file arguments are invalid",
        "Nothing was started",
    ] {
        assert!(said.contains(words), "`{words}` is not in: {said}");
    }
    // A program that is no script, a script that was not there, and any
    // system but that one: what the system said, and no more.
    assert_eq!(why_not_on(true, OsStr::new("tool.exe"), &refused()), "batch file arguments are invalid");
    assert_eq!(why_not_on(true, OsStr::new("tool.cmd"), &missing()), "program not found");
    assert_eq!(why_not_on(false, OsStr::new("tool.cmd"), &refused()), "batch file arguments are invalid");
}
