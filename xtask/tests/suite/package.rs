//! Packaging: the folder someone downloads and the one file of it, made
//! by the tooling itself in a tree: this system's, and another system's
//! made here to be tried there. What a package is on each system, asked
//! as functions, is in `systems.rs`.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use xtask::menu;
use xtask::package::{self, About, System};

const XTASK: &str = env!("CARGO_BIN_EXE_xtask");
const NAME: &str = "sample-1.2.3-linux";

/// A tree that looks like the repository to the tooling, with what a
/// release build would have left in it.
struct Tree {
    root: PathBuf,
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

impl Tree {
    fn new(name: &str) -> Tree {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique =
            format!("xtask-package-{name}-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst));
        let tree = Tree { root: PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(unique) };
        let root = &tree.root;
        write(&root.join("brand.toml"), "display = \"Sample\"\nid = \"sample\"\ncli = \"sm\"\n");
        write(&root.join("Cargo.toml"), "[workspace.package]\nversion = \"1.2.3\"\nedition = \"2024\"\n");
        write(&root.join("LICENSE-MIT"), "one licence");
        write(&root.join("LICENSE-APACHE"), "the other licence");
        write(&root.join("THIRD-PARTY-NOTICES.md"), "whose work is in it");
        write(
            &root.join("docs/first-run.md"),
            "# The first run\n\nThe [README](../README.md) has the same path.\n",
        );
        write(&root.join("extensions/first/extension.yaml"), "extension: first\n");
        write(&root.join("extensions/first/notes/more.txt"), "more of it\n");
        write(&root.join("desktop/icon/README.md"), "what an icon has to be\n");
        write(&root.join("target/release/factory"), "the daemon");
        write(&root.join("target/release/desktop"), "the window");
        // The client as its build leaves it: this build's files, and the
        // files of the build before it, for pages still open on that.
        let dist = root.join("ui/dist");
        write(&dist.join("index.html"), "<meta name=\"build\" content=\"two\">");
        write(&dist.join("lib/chart.js"), "not a build's");
        write(&dist.join("assets/app-TWO.js"), "part of two");
        write(&dist.join("assets/app-ONE.js"), "part of one");
        write(
            &dist.join("builds.json"),
            "[{\"id\":\"two\",\"files\":[\"assets/app-TWO.js\"]},{\"id\":\"one\",\"files\":[\"assets/app-ONE.js\"]}]",
        );
        tree
    }

    fn out(&self) -> PathBuf {
        self.root.join("target/package")
    }

    /// The folder of the package, whatever kind of processor this is.
    fn folder(&self) -> PathBuf {
        self.out().join(format!("{NAME}-{}", std::env::consts::ARCH))
    }

    /// Runs `cargo xtask package` in the tree, building nothing.
    fn package(&self, args: &[&str]) -> (bool, String) {
        self.asked(&[&["--no-build"], args].concat(), None)
    }

    /// Runs `cargo xtask package` in the tree as it was asked, with
    /// `first` looked in before anywhere else for the programs it starts.
    fn asked(&self, args: &[&str], first: Option<&Path>) -> (bool, String) {
        let mut command = Command::new(XTASK);
        command.arg("package").args(args).current_dir(&self.root).env("HOME", self.root.join("home"));
        if let Some(first) = first {
            let rest = std::env::var_os("PATH").unwrap_or_default();
            let all = std::iter::once(first.to_path_buf()).chain(std::env::split_paths(&rest));
            command.env("PATH", std::env::join_paths(all).unwrap());
        }
        let output = command.output().unwrap();
        let said =
            format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        (output.status.success(), said)
    }

    /// What a release build for another system would have left: that
    /// target's folder, with the programs as that system calls them.
    fn built_for(&self, target: &str, programs: &[&str]) {
        for program in programs {
            write(
                &self.root.join("target").join(target).join("release").join(program),
                &format!("{program}, for {target}"),
            );
        }
    }
}

/// A target that is another system's than this one, whichever this is,
/// for a test of a package made here to be tried there.
const WINDOWS: &str = "x86_64-pc-windows-gnu";
const MACOS: &str = "aarch64-apple-darwin";

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

fn runnable(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|file| file.permissions().mode() & 0o111 == 0o111)
}

fn said_by(program: &str, args: &[&str], dir: &Path) -> (bool, String) {
    let output = Command::new(program).args(args).current_dir(dir).output().unwrap();
    (output.status.success(), String::from_utf8_lossy(&output.stdout).into_owned())
}

#[test]
fn a_package_holds_the_programs_under_the_names_people_type_and_all_they_need_beside_them() {
    let tree = Tree::new("holds");
    let (ok, said) = tree.package(&[]);
    assert!(ok, "{said}");
    let folder = tree.folder();
    // The daemon under the command's name, the window under the product's,
    // and both something that can be run though what they were copied
    // from was not.
    assert_eq!(read(&folder.join("sm")), "the daemon");
    assert_eq!(read(&folder.join("sample")), "the window");
    assert!(runnable(&folder.join("sm")) && runnable(&folder.join("sample")));
    // The client, of the one build its entry page is of.
    assert!(read(&folder.join("ui/index.html")).contains("content=\"two\""));
    assert_eq!(read(&folder.join("ui/assets/app-TWO.js")), "part of two");
    assert_eq!(read(&folder.join("ui/lib/chart.js")), "not a build's");
    assert!(!folder.join("ui/assets/app-ONE.js").exists(), "the build before it is nobody's in a package");
    assert!(!folder.join("ui/builds.json").exists());
    // The packages that come with it, whole.
    assert_eq!(read(&folder.join("extensions/first/extension.yaml")), "extension: first\n");
    assert_eq!(read(&folder.join("extensions/first/notes/more.txt")), "more of it\n");
    assert_eq!(read(&folder.join("LICENSE-MIT")), "one licence");
    assert_eq!(read(&folder.join("LICENSE-APACHE")), "the other licence");
    // And the notices of what it bundles, as they are committed: nothing
    // makes them here.
    assert_eq!(read(&folder.join("THIRD-PARTY-NOTICES.md")), "whose work is in it");
    // The notes of the first run, as the source has them, with how a
    // command is typed in this folder said at their head and no link to
    // a page the package does not hold.
    let notes = read(&folder.join("FIRST-RUN.md"));
    assert!(notes.starts_with("# The first run\n\n> The notes of the first run"), "{notes}");
    assert!(
        notes.contains("that is `./sm ...`") && notes.ends_with("The README has the same path.\n"),
        "{notes}"
    );
    // Which revision it is, where the program looks for that.
    assert!(read(&folder.join("installed")).starts_with("revision "));
    // And a page of its own, in the names this product has.
    let page = read(&folder.join("README.md"));
    assert!(page.starts_with("# Sample 1.2.3 for Linux"), "{page}");
    assert!(page.contains("./sample\n") && page.contains("./sm up"), "{page}");
    // No icon yet, and the page about one is not it.
    assert!(!folder.join("icon").exists());
    assert!(said.contains("no icon yet"), "{said}");
}

#[test]
fn what_a_build_leaves_beside_the_programs_is_in_no_package() {
    let tree = Tree::new("beside");
    // A build of the command line's crate leaves more than the program:
    // the stand-in its tests start is a program of that crate too, so
    // that cargo builds it for them, and is no part of the product.
    for beside in ["cli-standin", "cli-standin.exe", "factory.d"] {
        write(&tree.root.join("target/release").join(beside), "not the product's");
    }
    let (ok, said) = tree.package(&[]);
    assert!(ok, "{said}");
    // A package is the programs by their names, and nothing a build
    // happened to leave where they are.
    fn names(folder: &Path, into: &mut Vec<String>) {
        for entry in fs::read_dir(folder).unwrap().flatten() {
            into.push(entry.file_name().to_string_lossy().into_owned());
            if entry.path().is_dir() {
                names(&entry.path(), into);
            }
        }
    }
    let mut held = Vec::new();
    names(&tree.folder(), &mut held);
    assert!(held.iter().any(|name| name == "sm"), "{held:?}");
    assert!(!held.iter().any(|name| name.contains("standin") || name == "factory.d"), "{held:?}");
}

#[test]
fn the_one_file_of_a_package_is_the_folder_and_its_sum_is_that_files() {
    let tree = Tree::new("file");
    let (ok, said) = tree.package(&[]);
    assert!(ok, "{said}");
    let name = format!("{NAME}-{}", std::env::consts::ARCH);
    let archive = format!("{name}.tar.gz");
    let (listed, list) = said_by("tar", &["--list", "--verbose", "--gzip", "--file", &archive], &tree.out());
    assert!(listed, "{list}");
    let lines: Vec<&str> = list.lines().collect();
    for wanted in ["/sm", "/sample", "/ui/index.html", "/extensions/first/extension.yaml", "/README.md"] {
        assert!(lines.iter().any(|line| line.ends_with(&format!("{name}{wanted}"))), "{wanted} in\n{list}");
    }
    // Everything is under the one folder, and is nobody's in particular:
    // whoever made the package is not written into it.
    for line in &lines {
        assert!(line.contains(&format!(" {name}/")), "{line}");
        assert!(line.contains(" 0/0 "), "{line}");
    }
    let (checked, check) = said_by("sha256sum", &["--check", &format!("{archive}.sha256")], &tree.out());
    assert!(checked, "{check}");
    assert!(check.contains(&format!("{archive}: OK")), "{check}");
}

#[test]
fn a_package_is_made_new_each_time_and_holds_an_icon_when_there_is_one() {
    let tree = Tree::new("again");
    assert!(tree.package(&[]).0);
    fs::remove_file(tree.root.join("extensions/first/notes/more.txt")).unwrap();
    write(&tree.root.join("desktop/icon/icon-256.png"), "a picture");
    let (ok, said) = tree.package(&[]);
    assert!(ok, "{said}");
    assert!(!tree.folder().join("extensions/first/notes/more.txt").exists());
    assert_eq!(read(&tree.folder().join("icon/icon-256.png")), "a picture");
    assert!(!tree.folder().join("icon/README.md").exists());
}

#[test]
fn a_package_carries_what_puts_it_in_a_desktops_menu_from_wherever_it_is_unpacked() {
    let tree = Tree::new("menu");
    let icon = tree.root.join("desktop/icon");
    write(&icon.join("icon.svg"), "a drawing");
    write(&icon.join("icon-48.png"), "small");
    write(&icon.join("icon-256.png"), "middling");
    write(&icon.join("icon-512.png"), "large");
    // What the pictures are made with and the window's own pixels are the
    // repository's and the program's, and are not a package's.
    write(&icon.join("make.mjs"), "a script");
    write(&icon.join("icon-window.rgba"), "pixels");
    let (ok, said) = tree.package(&[]);
    assert!(ok, "{said}");
    let folder = tree.folder();
    let held: Vec<String> = fs::read_dir(folder.join("icon"))
        .unwrap()
        .map(|file| file.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(held.len(), 4, "the pictures and nothing else: {held:?}");
    assert!(runnable(&folder.join("menu.sh")));
    assert!(read(&folder.join("README.md")).contains("./menu.sh"));

    // Run where the folder is, it files an entry and the icon under the
    // person's own data folder, naming the program by where it is.
    let home = tree.root.join("someone");
    let menu = |args: &[&str]| {
        let output = Command::new("sh")
            .arg(folder.join("menu.sh"))
            .args(args)
            .env("HOME", &home)
            .env_remove("XDG_DATA_HOME")
            .output()
            .unwrap();
        (output.status.success(), String::from_utf8_lossy(&output.stdout).into_owned())
    };
    let (ok, said) = menu(&[]);
    assert!(ok && said.contains("Sample is in the menu"), "{said}");
    let data = home.join(".local/share");
    let entry = read(&data.join("applications/sample.desktop"));
    for wanted in [
        "[Desktop Entry]\n",
        "Name=Sample\n",
        &format!("Exec=\"{}\"\n", folder.join("sample").display()),
        "Icon=sample\n",
        "StartupWMClass=sample\n",
    ] {
        assert!(entry.contains(wanted), "`{wanted}` in\n{entry}");
    }
    assert_eq!(read(&data.join("icons/hicolor/48x48/apps/sample.png")), "small");
    assert_eq!(read(&data.join("icons/hicolor/256x256/apps/sample.png")), "middling");
    assert_eq!(read(&data.join("icons/hicolor/512x512/apps/sample.png")), "large");
    assert_eq!(read(&data.join("icons/hicolor/scalable/apps/sample.svg")), "a drawing");

    // And takes away what it put there, and nothing else.
    write(&data.join("applications/another.desktop"), "somebody else's");
    let (ok, said) = menu(&["remove"]);
    assert!(ok && said.contains("Sample is out of the menu"), "{said}");
    assert!(!data.join("applications/sample.desktop").exists());
    assert!(!data.join("icons/hicolor/48x48/apps/sample.png").exists());
    assert!(!data.join("icons/hicolor/scalable/apps/sample.svg").exists());
    assert!(data.join("applications/another.desktop").exists());
    // Asked for anything else, it does nothing and says what it does.
    assert!(!menu(&["sideways"]).0);
}

#[test]
fn an_entry_names_its_program_in_quotes_with_what_would_mean_something_made_plain() {
    let entry = menu::entry("Sample", "org.sample.app", "/home/some one/a $folder/sample", "sample");
    assert!(entry.contains("Exec=\"/home/some one/a \\$folder/sample\"\n"), "{entry}");
    assert!(entry.contains("Icon=org.sample.app\n") && entry.contains("Type=Application\n"), "{entry}");
    assert_eq!(menu::entry_place("org.sample.app"), "applications/org.sample.app.desktop");
    assert!(menu::is_picture("icon-48.png") && menu::is_picture("icon.svg"));
    assert!(
        !menu::is_picture("README.md")
            && !menu::is_picture("make.mjs")
            && !menu::is_picture("icon-window.rgba")
    );
}

#[test]
fn nothing_is_packaged_until_everything_a_package_holds_is_there() {
    let tree = Tree::new("short");
    fs::remove_file(tree.root.join("LICENSE-APACHE")).unwrap();
    let (ok, said) = tree.package(&[]);
    assert!(!ok && said.contains("LICENSE-APACHE is not at the top of the repository"), "{said}");
    assert!(!tree.out().exists(), "nothing was put anywhere");

    write(&tree.root.join("LICENSE-APACHE"), "the other licence");
    fs::remove_file(tree.root.join("THIRD-PARTY-NOTICES.md")).unwrap();
    let (ok, said) = tree.package(&[]);
    assert!(!ok && said.contains("THIRD-PARTY-NOTICES.md is not at the top of the repository"), "{said}");
    assert!(said.contains("`cargo xtask notices` makes it"), "{said}");
    assert!(!tree.out().exists(), "nothing was put anywhere");

    write(&tree.root.join("THIRD-PARTY-NOTICES.md"), "whose work is in it");
    let walk = fs::read_to_string(tree.root.join("docs/first-run.md")).unwrap();
    fs::remove_file(tree.root.join("docs/first-run.md")).unwrap();
    let (ok, said) = tree.package(&[]);
    assert!(!ok && said.contains("docs/first-run.md is not in the repository"), "{said}");
    assert!(said.contains("carries the notes of the first run as `FIRST-RUN.md`"), "{said}");
    assert!(!tree.out().exists(), "nothing was put anywhere");

    write(&tree.root.join("docs/first-run.md"), &walk);
    fs::remove_file(tree.root.join("target/release/desktop")).unwrap();
    let (ok, said) = tree.package(&[]);
    assert!(!ok && said.contains("has not been built: run this without `--no-build`"), "{said}");
    assert!(!tree.out().exists());
}

#[test]
fn a_nightly_is_called_nightly_and_says_inside_it_its_day_and_its_commit() {
    let tree = Tree::new("nightly");
    let (ok, said) = tree.package(&["--nightly", "20261124"]);
    assert!(ok, "{said}");
    // Whatever commit this tree is in, as git says it there.
    let (_, commit) = said_by("git", &["rev-parse", "--short", "HEAD"], &tree.root);
    let version = format!("1.2.3-nightly.20261124+{}", commit.trim());
    let folder = tree.out().join(format!("sample-nightly-linux-{}", std::env::consts::ARCH));
    assert!(folder.join("sm").is_file(), "{said}");
    assert!(tree.out().join(format!("sample-nightly-linux-{}.tar.gz", std::env::consts::ARCH)).is_file());
    assert!(!tree.folder().exists(), "no folder is called by the version");
    let note = read(&folder.join("installed"));
    assert!(note.starts_with(&format!("{version}, a nightly, revision ")), "{note}");
    let page = read(&folder.join("README.md"));
    assert!(page.starts_with(&format!("# Sample {version} for Linux")), "{page}");
    assert!(said.contains(&format!("packaged Sample {version} for Linux")), "{said}");
}

#[test]
fn a_nightly_is_not_made_without_its_day_or_its_commit() {
    let tree = Tree::new("unnightly");
    for asked in [&["--nightly"][..], &["--nightly", "2026-11-24"], &["--nightly", "--no-window"]] {
        let (ok, said) = tree.package(asked);
        assert!(
            !ok && said.contains("`--nightly` is followed by the day the nightly is of"),
            "{asked:?}: {said}"
        );
        assert!(!tree.out().exists(), "nothing was put anywhere");
    }
    // Where git finds no repository, there is no commit to say.
    let output = Command::new(XTASK)
        .args(["package", "--no-build", "--nightly", "20261124"])
        .current_dir(&tree.root)
        .env("HOME", tree.root.join("home"))
        .env("GIT_CEILING_DIRECTORIES", tree.root.parent().unwrap())
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success() && said.contains("git could not say which commit this is"), "{said}");
    assert!(!tree.out().exists(), "nothing was put anywhere");
}

#[test]
fn making_a_package_touches_nothing_but_where_the_package_is_put() {
    let tree = Tree::new("only");
    let elsewhere = tree.root.join("elsewhere");
    let (ok, said) = tree.package(&["--out", elsewhere.to_str().unwrap()]);
    assert!(ok, "{said}");
    assert!(elsewhere.join(format!("{NAME}-{}", std::env::consts::ARCH)).join("sm").is_file());
    assert!(!tree.root.join("home").exists(), "nobody's home was touched");
    assert!(!tree.out().exists(), "told where to put it, it puts nothing where it otherwise would");
}

#[test]
fn the_version_of_a_package_is_the_one_every_crate_here_has() {
    let manifest = "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"0.4.1\"\nedition = \"2024\"\n\n[workspace.dependencies]\nversion = \"9\"\n";
    assert_eq!(package::version(manifest).as_deref(), Some("0.4.1"));
    assert_eq!(package::version("[package]\nversion = \"1.0.0\"\n"), None);
}

#[test]
fn a_package_takes_the_one_build_of_the_client_its_entry_page_is_of() {
    let all: Vec<String> = [
        "assets/a-NEW.js",
        "assets/a-OLD.js",
        "builds.json",
        "built-from.json",
        "index.html",
        "lib/chart.js",
    ]
    .map(str::to_string)
    .to_vec();
    let entry = "<meta name=\"build\" content=\"new\">";
    let builds =
        "[{\"id\":\"new\",\"files\":[\"assets/a-NEW.js\"]},{\"id\":\"old\",\"files\":[\"assets/a-OLD.js\"]}]";
    assert_eq!(
        package::client_files(&all, entry, Some(builds)),
        ["assets/a-NEW.js", "index.html", "lib/chart.js"]
    );
    // Where it cannot tell which build is which, it takes everything:
    // too much, and never too little.
    assert_eq!(package::client_files(&all, entry, None), all);
    assert_eq!(package::client_files(&all, "<html>", Some(builds)), all);
    assert_eq!(package::client_files(&all, entry, Some("[]")), all);

    // However many earlier builds the client's folder keeps, and whatever
    // else its build says of each, a package has no earlier builds.
    let kept = "[{\"id\":\"newer\",\"at\":3,\"files\":[\"assets/a-NEWER.js\"]},\
                {\"id\":\"new\",\"at\":2,\"files\":[\"assets/a-NEW.js\"]},\
                {\"id\":\"old\",\"at\":1,\"files\":[\"assets/a-OLD.js\"]}]";
    let mut more = all.clone();
    more.insert(0, "assets/a-NEWER.js".to_string());
    assert_eq!(
        package::client_files(&more, entry, Some(kept)),
        ["assets/a-NEW.js", "index.html", "lib/chart.js"]
    );
}

#[test]
fn the_page_in_a_package_says_how_to_start_it_what_the_window_needs_and_how_to_be_rid_of_it() {
    let page = package::readme(&About {
        display: "Sample",
        cli: "sm",
        id: "sample",
        version: "1.2.3",
        system: System::Linux,
        processor: "x86_64",
        revision: Some("revision abc1234"),
        built_on: "Some Linux 1, with glibc 2.35",
        elsewhere: None,
        window: true,
        nightly: false,
        address: "http://127.0.0.1:7700",
    });
    for wanted in [
        "# Sample 1.2.3 for Linux (x86_64)",
        "**A preview.**",
        "./sample\n",
        "./sm up &",
        "./sm open",
        "http://127.0.0.1:7700",
        "libwebkit2gtk-4.1-0",
        "webkit2gtk4.1",
        "webkit2gtk-4.1",
        "ln -sf \"$PWD/sm\" ~/.local/bin/sm",
        "~/.local/share/sample",
        "~/.config/sample",
        "./sm down",
        "./menu.sh remove",
        "Built on Some Linux 1, with glibc 2.35. This is revision abc1234.",
        "LICENSE-MIT",
        "LICENSE-APACHE",
        "`THIRD-PARTY-NOTICES.md` names each",
    ] {
        assert!(page.contains(wanted), "`{wanted}` in\n{page}");
    }
}

#[test]
fn a_package_for_windows_made_here_is_a_zip_of_that_systems_programs_and_says_it_is_for_trying() {
    let tree = Tree::new("windows");
    tree.built_for(WINDOWS, &["factory.exe", "desktop.exe"]);
    // An icon there is, which is a menu's on Linux and no part of this.
    write(&tree.root.join("desktop/icon/icon-256.png"), "a picture");
    let (ok, said) = tree.package(&["--target", WINDOWS]);
    assert!(ok, "{said}");
    assert!(said.contains("packaged Sample 1.2.3 for Windows (x86_64)  (2 programs, "), "{said}");
    let name = "sample-1.2.3-windows-x86_64";
    let folder = tree.out().join(name);
    assert_eq!(read(&folder.join("sm.exe")), format!("factory.exe, for {WINDOWS}"));
    assert_eq!(read(&folder.join("sample.exe")), format!("desktop.exe, for {WINDOWS}"));
    assert!(read(&folder.join("ui/index.html")).contains("content=\"two\""));
    assert_eq!(read(&folder.join("extensions/first/extension.yaml")), "extension: first\n");
    assert_eq!(read(&folder.join("LICENSE-MIT")), "one licence");
    // Nothing of Linux's: no programs under its names, no menu, no icon
    // for one.
    for absent in ["sm", "sample", "menu.sh", "icon"] {
        assert!(!folder.join(absent).exists(), "{absent} is not a Windows package's");
    }
    // It says where it was built and what for, where the daemon reads
    // what it is and on its page, which is Windows's.
    let note = read(&folder.join("installed"));
    assert!(note.starts_with("revision ") && note.contains(", built on "), "{note}");
    assert!(note.trim_end().ends_with(" for Windows, for trying and never a release"), "{note}");
    let page = read(&folder.join("README.md"));
    assert!(page.starts_with("# Sample 1.2.3 for Windows (x86_64)"), "{page}");
    assert!(page.contains("**This one is not a release.** It was built on "), "{page}");
    assert!(page.contains(".\\sm.exe up\n") && page.contains("`sample.exe` is the window"), "{page}");
    assert!(said.contains("it is for trying there, and nothing in it is run here"), "{said}");

    // The one file of it is a zip: every file of the folder under the
    // folder's own name, whole, the programs said to be programs.
    let archive = format!("{name}.zip");
    let unpacked = crate::zip::unpacked(&fs::read(tree.out().join(&archive)).unwrap());
    let names: Vec<&str> = unpacked.iter().map(|file| file.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted, "in the order of their names");
    for file in &unpacked {
        let within = file.name.strip_prefix(&format!("{name}/")).unwrap_or_else(|| panic!("{}", file.name));
        assert_eq!(file.bytes, fs::read(folder.join(within)).unwrap(), "{within}");
        assert_eq!(file.runs, ["sm.exe", "sample.exe"].contains(&within), "{within}");
    }
    for wanted in [
        "sm.exe",
        "sample.exe",
        "ui/index.html",
        "ui/assets/app-TWO.js",
        "extensions/first/notes/more.txt",
        "LICENSE-APACHE",
        "THIRD-PARTY-NOTICES.md",
        "FIRST-RUN.md",
        "installed",
        "README.md",
    ] {
        assert!(names.contains(&format!("{name}/{wanted}").as_str()), "{wanted} in {names:?}");
    }
    assert_eq!(unpacked.len(), 13, "and nothing else: {names:?}");
    // And its sum is that file's, in the form the system's own program reads.
    let (checked, check) = said_by("sha256sum", &["--check", &format!("{archive}.sha256")], &tree.out());
    assert!(checked && check.contains(&format!("{archive}: OK")), "{check}");
    assert!(!tree.out().join(format!("{name}.tar.gz")).exists());
}

#[test]
fn a_package_made_without_the_window_holds_none_and_asks_for_none() {
    let tree = Tree::new("windowless");
    tree.built_for(WINDOWS, &["factory.exe"]);
    // With the window wanted, a package is refused for the lack of it, by
    // where cargo would have put it.
    let (ok, said) = tree.package(&["--target", WINDOWS]);
    assert!(!ok, "{said}");
    assert!(said.contains(&format!("target/{WINDOWS}/release/desktop.exe has not been built")), "{said}");
    assert!(!tree.out().exists(), "nothing was put anywhere");

    let (ok, said) = tree.package(&["--target", WINDOWS, "--no-window"]);
    assert!(ok, "{said}");
    assert!(said.contains("(1 programs, "), "{said}");
    let folder = tree.out().join("sample-1.2.3-windows-x86_64");
    assert!(folder.join("sm.exe").is_file() && !folder.join("sample.exe").exists());
    let page = read(&folder.join("README.md"));
    assert!(
        page.contains("made without the window's program: there is no `sample.exe` in this folder"),
        "{page}"
    );
    assert!(!page.contains("is the window") && !page.contains("WebView2"), "{page}");

    // The same of this system's own package, which then has no menu
    // either: an entry there starts the window.
    fs::remove_file(tree.root.join("target/release/desktop")).unwrap();
    write(&tree.root.join("desktop/icon/icon-256.png"), "a picture");
    let (ok, said) = tree.package(&["--target", "x86_64-unknown-linux-gnu", "--no-window"]);
    assert!(
        !ok && said.contains("target/x86_64-unknown-linux-gnu/release/factory has not been built"),
        "{said}"
    );
    tree.built_for("x86_64-unknown-linux-gnu", &["factory"]);
    let (ok, said) = tree.package(&["--target", "x86_64-unknown-linux-gnu", "--no-window"]);
    assert!(ok, "{said}");
    let folder = tree.out().join("sample-1.2.3-linux-x86_64");
    assert_eq!(read(&folder.join("sm")), "factory, for x86_64-unknown-linux-gnu");
    for absent in ["sample", "menu.sh", "icon"] {
        assert!(!folder.join(absent).exists(), "{absent}");
    }
}

#[test]
fn a_package_for_macos_made_here_is_that_systems_folder_in_one_file() {
    let tree = Tree::new("macos");
    tree.built_for(MACOS, &["factory", "desktop"]);
    write(&tree.root.join("desktop/icon/icon-256.png"), "a picture");
    let (ok, said) = tree.package(&["--target", MACOS]);
    assert!(ok, "{said}");
    let name = "sample-1.2.3-macos-aarch64";
    let folder = tree.out().join(name);
    assert_eq!(read(&folder.join("sm")), format!("factory, for {MACOS}"));
    assert_eq!(read(&folder.join("sample")), format!("desktop, for {MACOS}"));
    assert!(runnable(&folder.join("sm")) && runnable(&folder.join("sample")));
    for absent in ["menu.sh", "icon"] {
        assert!(!folder.join(absent).exists(), "{absent} is Linux's");
    }
    let page = read(&folder.join("README.md"));
    assert!(page.starts_with("# Sample 1.2.3 for macOS (aarch64)"), "{page}");
    assert!(page.contains("./sm up &") && page.contains("xattr -dr com.apple.quarantine ."), "{page}");
    assert!(read(&folder.join("installed")).contains(" for macOS, for trying and never a release"));
    let archive = format!("{name}.tar.gz");
    let (listed, list) = said_by("tar", &["--list", "--gzip", "--file", &archive], &tree.out());
    assert!(listed, "{list}");
    for wanted in ["/sm", "/sample", "/ui/index.html", "/README.md"] {
        assert!(list.lines().any(|line| line == format!("{name}{wanted}")), "{wanted} in\n{list}");
    }
    assert!(!tree.out().join(format!("{name}.files")).exists(), "what named the files in order is gone");
    let (checked, check) = said_by("sha256sum", &["--check", &format!("{archive}.sha256")], &tree.out());
    assert!(checked, "{check}");
}

#[test]
fn building_for_another_system_asks_cargo_for_that_target_by_the_word_said() {
    let tree = Tree::new("asked");
    tree.built_for(WINDOWS, &["factory.exe", "desktop.exe"]);
    // A cargo that writes down what it was asked and builds nothing: what
    // a build would have left is in the tree already.
    let tools = tree.root.join("tools");
    let asked = tree.root.join("asked.txt");
    write(&tools.join("cargo"), &format!("#!/bin/sh\necho \"$@\" >> '{}'\n", asked.display()));
    fs::set_permissions(tools.join("cargo"), fs::Permissions::from_mode(0o755)).unwrap();

    let (ok, said) = tree.asked(&["--target", WINDOWS, "--cargo-build", "zigbuild"], Some(&tools));
    assert!(ok, "{said}");
    assert_eq!(
        read(&asked),
        format!(
            "zigbuild --profile release -p cli --target {WINDOWS}\nzigbuild --profile release -p desktop --target {WINDOWS}\n"
        )
    );
    assert!(said.contains("building Sample 1.2.3 for Windows, to be tried there"), "{said}");

    // Without the window nothing is asked for it; and with nothing said,
    // cargo is asked to build, for this system, as it always was.
    fs::remove_file(&asked).unwrap();
    let (ok, said) = tree.asked(&["--target", WINDOWS, "--no-window"], Some(&tools));
    assert!(ok, "{said}");
    assert_eq!(read(&asked), format!("build --profile release -p cli --target {WINDOWS}\n"));
    fs::remove_file(&asked).unwrap();
    let (ok, said) = tree.asked(&[], Some(&tools));
    assert!(ok, "{said}");
    assert_eq!(read(&asked), "build --profile release -p cli\nbuild --profile release -p desktop\n");
    assert!(said.contains("building Sample 1.2.3 as it is released"), "{said}");
}

#[test]
fn a_target_that_names_no_system_there_is_a_package_for_is_refused_before_anything_is_built() {
    let tree = Tree::new("nowhere");
    for (args, why) in [
        (&["--target", "wasm32-unknown-unknown"][..], "`wasm32-unknown-unknown` does not say a system"),
        (&["--target"][..], "`--target` is followed by a target as cargo names one"),
    ] {
        let (ok, said) = tree.asked(args, None);
        assert!(!ok && said.contains(why), "{said}");
        assert!(!tree.out().exists(), "nothing was put anywhere");
    }
}
