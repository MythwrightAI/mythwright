//! What a program started from an almost empty environment keeps of the
//! daemon's: a function of which system it is and what is set, played
//! for both. Nothing of it has been run on Windows.

use std::collections::BTreeMap;

use hosting::cleared_env;

fn kept(windows: bool, set: &[(&str, &str)]) -> BTreeMap<String, String> {
    let set: BTreeMap<&str, &str> = set.iter().copied().collect();
    cleared_env(windows, |name| set.get(name).map(|value| value.to_string()))
        .into_iter()
        .map(|(name, value)| (name.to_string(), value))
        .collect()
}

fn names(kept: &BTreeMap<String, String>) -> Vec<&str> {
    kept.keys().map(String::as_str).collect()
}

/// What a daemon on Windows is started with, more or less, and things
/// of its own beside.
const ON_WINDOWS: &[(&str, &str)] = &[
    ("PATH", "C:\\Windows\\system32;C:\\tools"),
    ("PATHEXT", ".COM;.EXE;.BAT;.CMD"),
    ("SystemRoot", "C:\\Windows"),
    ("SystemDrive", "C:"),
    ("windir", "C:\\Windows"),
    ("ComSpec", "C:\\Windows\\system32\\cmd.exe"),
    ("USERPROFILE", "C:\\Users\\somebody"),
    ("USERNAME", "somebody"),
    ("HOMEDRIVE", "C:"),
    ("HOMEPATH", "\\Users\\somebody"),
    ("APPDATA", "C:\\Users\\somebody\\AppData\\Roaming"),
    ("LOCALAPPDATA", "C:\\Users\\somebody\\AppData\\Local"),
    ("ProgramData", "C:\\ProgramData"),
    ("ProgramFiles", "C:\\Program Files"),
    ("TEMP", "C:\\Users\\somebody\\AppData\\Local\\Temp"),
    ("TMP", "C:\\Users\\somebody\\AppData\\Local\\Temp"),
    ("SOMEBODYS_KEY", "never handed on"),
    ("OneDrive", "C:\\Users\\somebody\\OneDrive"),
];

#[test]
fn a_cleared_program_keeps_what_any_program_needs_and_nothing_of_anybodys_own() {
    let kept = kept(false, &[("PATH", "/usr/bin"), ("HOME", "/home/somebody"), ("SOMEBODYS_KEY", "never")]);
    assert_eq!(names(&kept), ["HOME", "PATH"]);
    assert_eq!(kept["PATH"], "/usr/bin");
}

#[test]
fn what_is_not_set_is_not_given() {
    assert!(kept(false, &[]).is_empty());
    assert!(kept(true, &[]).is_empty());
}

#[test]
fn on_windows_it_keeps_where_its_users_folders_and_the_systems_are_as_well() {
    let kept = kept(true, ON_WINDOWS);
    let mut all: Vec<&str> = ON_WINDOWS.iter().map(|(name, _)| *name).collect();
    all.retain(|name| !["SOMEBODYS_KEY", "OneDrive"].contains(name));
    all.sort_unstable();
    assert_eq!(names(&kept), all);
    assert_eq!(kept["ComSpec"], "C:\\Windows\\system32\\cmd.exe");
}

#[test]
fn and_anywhere_else_a_variable_called_as_one_of_windows_is_somebodys_own_and_is_cleared() {
    // `TMP` is set by people on Unix too: a step that asked for an
    // almost empty environment there goes on getting one.
    let kept = kept(false, ON_WINDOWS);
    assert_eq!(names(&kept), ["PATH", "SystemRoot", "TEMP"]);
}
