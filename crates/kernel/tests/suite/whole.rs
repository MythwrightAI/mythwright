//! A path made whole, and the Windows rule for saying one plainly, which
//! is a function of text so that it is held where there are no such paths.

use std::path::{Path, PathBuf};

use kernel::whole::{at_length, beneath, plainly, same, under, within};

/// A name of 100 letters, for a path past the system's old limit.
fn long_name() -> String {
    "n".repeat(100)
}

#[test]
fn a_long_form_is_said_plainly_only_where_the_plain_form_means_the_same() {
    let deep = format!(r"\\?\C:\{0}\{0}\{0}", long_name());
    // The longest plain path the system takes as it is: 246 letters, and
    // one more is past it.
    let longest = format!(r"\\?\C:\{}", "n".repeat(243));
    let past = format!(r"\\?\C:\{}", "n".repeat(244));
    let cases: &[(&str, &str, &str)] = &[
        // (what it is, written, said)
        ("a folder on a drive", r"\\?\C:\Users\ada\bin", r"C:\Users\ada\bin"),
        ("a drive's own top", r"\\?\C:\", r"C:\"),
        ("a drive written small", r"\\?\d:\work", r"d:\work"),
        ("a share on another machine", r"\\?\UNC\server\share\a", r"\\server\share\a"),
        ("a share's own top", r"\\?\UNC\server\share", r"\\server\share"),
        ("a name with a space and a dot inside", r"\\?\C:\My Work\notes.v2.md", r"C:\My Work\notes.v2.md"),
        (
            "a name that only begins as a device's does",
            r"\\?\C:\console\com10\null",
            r"C:\console\com10\null",
        ),
        ("the longest that is taken as it is", &longest, &longest[4..]),
        // Left exactly as they are.
        ("a plain path", r"C:\Users\ada", r"C:\Users\ada"),
        ("a plain share", r"\\server\share\a", r"\\server\share\a"),
        ("a path of another system", "/srv/ada/work", "/srv/ada/work"),
        ("a path from where one is", r"work\notes", r"work\notes"),
        ("nothing", "", ""),
        (
            "a volume by its id",
            r"\\?\Volume{b75e2c83-0000-0000-0000-602f00000000}\a",
            r"\\?\Volume{b75e2c83-0000-0000-0000-602f00000000}\a",
        ),
        ("a device", r"\\?\GLOBALROOT\Device\Harddisk0", r"\\?\GLOBALROOT\Device\Harddisk0"),
        ("a device, plainly", r"\\.\pipe\factory", r"\\.\pipe\factory"),
        ("a drive with no top", r"\\?\C:", r"\\?\C:"),
        ("a drive that is no letter", r"\\?\1:\a", r"\\?\1:\a"),
        ("a machine with no share", r"\\?\UNC\server", r"\\?\UNC\server"),
        ("a share with no name", r"\\?\UNC\server\", r"\\?\UNC\server\"),
        (
            "a share written small, which the system does not write",
            r"\\?\unc\server\share",
            r"\\?\unc\server\share",
        ),
        ("one past the longest", &past, &past),
        ("a path past the limit", &deep, &deep),
        ("a name that ends in a dot", r"\\?\C:\a\notes.", r"\\?\C:\a\notes."),
        ("a name that ends in a space", r"\\?\C:\a \b", r"\\?\C:\a \b"),
        ("a device's name", r"\\?\C:\a\CON", r"\\?\C:\a\CON"),
        ("a device's name written small", r"\\?\C:\a\nul", r"\\?\C:\a\nul"),
        ("a device's name before a dot", r"\\?\C:\a\aux.txt", r"\\?\C:\a\aux.txt"),
        ("a device's name with spaces after it", r"\\?\C:\a\prn  .txt", r"\\?\C:\a\prn  .txt"),
        ("a numbered device", r"\\?\C:\a\COM1", r"\\?\C:\a\COM1"),
        ("another, with a small number", r"\\?\C:\a\lpt²", r"\\?\C:\a\lpt²"),
        ("the console's own two", r"\\?\C:\a\conin$", r"\\?\C:\a\conin$"),
        ("a share on a machine called as a device is", r"\\?\UNC\nul\share\a", r"\\?\UNC\nul\share\a"),
        ("a name that is one dot", r"\\?\C:\a\.\b", r"\\?\C:\a\.\b"),
        ("a name that is two", r"\\?\C:\a\..\b", r"\\?\C:\a\..\b"),
        ("the other stroke in a name", r"\\?\C:\a/b", r"\\?\C:\a/b"),
        ("a wildcard in a name", r"\\?\C:\a\b*", r"\\?\C:\a\b*"),
        ("a colon in a name", r"\\?\C:\a\b:c", r"\\?\C:\a\b:c"),
        ("a name of nothing", r"\\?\C:\a\\b", r"\\?\C:\a\\b"),
        ("a stroke at the end", r"\\?\C:\a\", r"\\?\C:\a\"),
        ("something that cannot be shown in a name", "\\\\?\\C:\\a\u{7}b", "\\\\?\\C:\\a\u{7}b"),
    ];
    for (what, written, said) in cases {
        assert_eq!(plainly(written), *said, "{what}: `{written}`");
    }
}

#[test]
fn a_plain_path_written_at_length_is_what_saying_it_plainly_undoes() {
    let cases: &[(&str, &str, &str)] = &[
        ("a folder on a drive", r"C:\Users\ada", r"\\?\C:\Users\ada"),
        ("a drive written small", r"c:\Users", r"\\?\C:\Users"),
        ("either stroke", "C:/Work/ada", r"\\?\C:\Work\ada"),
        ("a share", r"\\server\share\a", r"\\?\UNC\server\share\a"),
        ("a share with the other stroke", "//server/share/a", r"\\?\UNC\server\share\a"),
        // Left exactly as they are.
        ("a path already long", r"\\?\C:\a/b", r"\\?\C:\a/b"),
        ("a device", r"\\.\pipe\factory", r"\\.\pipe\factory"),
        ("a path from where one is", r"work\notes", r"work\notes"),
        ("a path from where one is on a drive", r"C:work", r"C:work"),
        ("a path from the top of whichever drive", r"\Users\ada", r"\Users\ada"),
        ("a path of another system", "/srv/ada", "/srv/ada"),
    ];
    for (what, written, long) in cases {
        assert_eq!(at_length(written), *long, "{what}: `{written}`");
    }
    for plain in [r"C:\Users\ada\bin", r"C:\", r"\\server\share\a", r"\\server\share"] {
        assert_eq!(plainly(&at_length(plain)), plain);
    }
}

#[test]
fn what_a_path_is_under_is_the_same_whichever_form_either_is_written_in() {
    let deep = format!(r"\\?\C:\work\shop\{0}\{0}\{0}", long_name());
    let rest = format!(r"{0}\{0}\{0}", long_name());
    let cases: &[(&str, &str, &str, Option<&str>)] = &[
        // (what it is, the path, what it may be under, what is left)
        ("both plain", r"C:\work\shop\notes", r"C:\work\shop", Some("notes")),
        ("both long", r"\\?\C:\work\shop\notes", r"\\?\C:\work\shop", Some("notes")),
        ("something deep, in a folder said plainly", &deep, r"C:\work\shop", Some(&rest)),
        ("the other way round", r"C:\work\shop\notes\a.md", r"\\?\C:\work\shop", Some(r"notes\a.md")),
        ("a name the plain form cannot say", r"\\?\C:\work\shop\aux\a", r"C:\work\shop", Some(r"aux\a")),
        ("a folder and itself", r"\\?\C:\work\shop", r"C:\work\shop", Some("")),
        ("a folder and itself with a stroke after it", r"C:\work\shop\", r"\\?\C:\work\shop", Some("")),
        ("a drive's top", r"\\?\C:\work", r"C:\", Some("work")),
        ("a drive written small", r"\\?\C:\work\shop", r"c:\work", Some("shop")),
        ("a share", r"\\?\UNC\server\share\a\b", r"\\server\share\a", Some("b")),
        ("a share written with the other stroke", r"\\?\UNC\server\share\a", "//server/share", Some("a")),
        ("a folder beside it that begins the same", r"\\?\C:\work\shopping", r"C:\work\shop", None),
        ("the folder above", r"\\?\C:\work", r"C:\work\shop", None),
        ("another drive", r"\\?\D:\work\shop", r"C:\work", None),
        ("a name in another case", r"\\?\C:\Work\shop", r"C:\work", None),
        ("a share of another machine", r"\\?\UNC\other\share\a", r"\\server\share", None),
        ("a drive and a share called the same", r"\\?\C:\share", r"\\C:\share", None),
        ("a volume by its id", r"\\?\Volume{b75e2c83}\work", r"C:\", None),
    ];
    for (what, path, root, left) in cases {
        assert_eq!(beneath(path, root).as_deref(), *left, "{what}: `{path}` under `{root}`");
    }
}

/// A folder of the test's own, under the build's target directory.
#[cfg(unix)]
fn scratch(name: &str) -> PathBuf {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}-{}", &kernel::id::secret()[..12]));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[cfg(unix)]
#[test]
fn a_whole_path_is_the_systems_own_with_links_followed_where_there_is_one_form() {
    let dir = scratch("whole");
    let (real, link) = (dir.join("real"), dir.join("link"));
    let whole = kernel::whole::whole;
    std::fs::create_dir_all(real.join("inside")).unwrap();
    std::os::unix::fs::symlink(&real, &link).unwrap();

    let through = whole(&link.join("inside").join("..").join("inside")).unwrap();
    assert_eq!(through, real.join("inside").canonicalize().unwrap(), "links and `..` are gone from it");
    assert_eq!(whole(&real).unwrap(), real.canonicalize().unwrap(), "and nothing else is done to it");
    let missing = whole(&dir.join("nothing-here")).unwrap_err();
    assert_eq!(missing.kind(), std::io::ErrorKind::NotFound, "what is not there has no whole path");

    // A name may begin as a long form does, where there is none.
    let odd = dir.join([r"\\?", "C:", "a"].join(r"\"));
    std::fs::create_dir_all(&odd).unwrap();
    assert_eq!(whole(&odd).unwrap(), odd.canonicalize().unwrap());
    assert_eq!(kernel::whole::said_plainly(odd.clone()), odd);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn one_whole_path_is_asked_whether_it_is_under_another() {
    let root = Path::new("/work/shop");
    assert_eq!(under(Path::new("/work/shop/notes/a.md"), root), Some(PathBuf::from("notes/a.md")));
    assert_eq!(under(root, root), Some(PathBuf::new()));
    assert_eq!(under(Path::new("/work/shopping"), root), None);
    assert!(within(Path::new("/work/shop/notes"), root) && within(root, root));
    assert!(!within(Path::new("/work"), root));
    assert!(same(root, Path::new("/work/shop/")) && !same(Path::new("/work/shop/notes"), root));
    // Where there is one form, what looks like the other is a name.
    if cfg!(not(windows)) {
        assert!(!within(Path::new(r"\\?\C:\work\shop\notes"), Path::new(r"C:\work\shop")));
    }
}

/// Every file of Rust under a `src` folder below `dir`, from the top of
/// the repository.
fn sources(dir: &Path, top: &Path, found: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let (path, name) = (entry.path(), entry.file_name().to_string_lossy().into_owned());
        if path.is_dir() && !["target", "tests", "node_modules"].contains(&name.as_str()) {
            sources(&path, top, found);
        } else if name.ends_with(".rs") && path.components().any(|part| part.as_os_str() == "src") {
            found.push(path.strip_prefix(top).unwrap().to_string_lossy().replace('\\', "/"));
        }
    }
}

#[test]
fn nothing_asks_the_system_for_a_whole_path_but_the_one_function() {
    let top = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // The function, and the tooling's own few lines, which stand on the
    // standard library alone and say why where they are.
    let may = ["crates/kernel/src/whole.rs", "xtask/src/install.rs"];
    let mut files = Vec::new();
    for folder in ["crates", "desktop", "examples", "xtask"] {
        sources(&top.join(folder), &top, &mut files);
    }
    assert!(files.len() > 100 && may.iter().all(|file| files.iter().any(|found| found == file)), "{files:?}");
    let asks: Vec<&String> = files
        .iter()
        .filter(|file| !may.contains(&file.as_str()))
        .filter(|file| std::fs::read_to_string(top.join(file)).unwrap().contains("canonicalize"))
        .collect();
    assert!(
        asks.is_empty(),
        "the system's own whole path is in a form of its own on Windows (`\\\\?\\C:\\…`), which is not equal to \
         the place written plainly, is not under it, and is refused by programs it is handed to. Ask \
         `kernel::whole::whole`, and ask `kernel::whole::within` whether one such path is under another; \
         these ask the system themselves: {asks:?}"
    );
}
