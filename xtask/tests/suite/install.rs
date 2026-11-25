//! Installing: a copy in a folder of its own, put there so that a copy
//! that is running is not broken by it.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

const XTASK: &str = env!("CARGO_BIN_EXE_xtask");

/// A tree that looks like the repository to the tooling, with what a build
/// would have left in it, and somewhere to install to.
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
        let unique = format!("xtask-{name}-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst));
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(unique);
        let tree = Tree { root };
        write(&tree.root.join("brand.toml"), "display = \"Sample\"\nid = \"sample\"\ncli = \"sm\"\n");
        write(&tree.root.join("Cargo.toml"), "");
        write(&tree.root.join("extensions/first/extension.yaml"), "extension: first\n");
        tree.built("program one");
        tree.client("one", &["assets/app-ONE.js"]);
        tree
    }

    /// What a build leaves: the two programs, saying `text`.
    fn built(&self, text: &str) {
        write(&self.root.join("target/fast/factory"), text);
        write(&self.root.join("target/fast/desktop"), &format!("{text}, in a window"));
    }

    /// What a build of the client leaves: an entry page naming the build,
    /// and that build's files and no others.
    fn client(&self, build: &str, files: &[&str]) {
        let dist = self.root.join("ui/dist");
        let _ = fs::remove_dir_all(&dist);
        write(&dist.join("index.html"), &format!("<meta name=\"build\" content=\"{build}\">"));
        for file in files {
            write(&dist.join(file), &format!("part of {build}"));
        }
    }

    fn to(&self) -> PathBuf {
        self.root.join("installed")
    }

    fn bin(&self) -> PathBuf {
        self.root.join("bin")
    }

    /// Runs `cargo xtask install` in the tree, building nothing.
    fn install(&self, args: &[&str]) -> (bool, String) {
        let output = Command::new(XTASK)
            .args(["install", "--no-build", "--bin"])
            .arg(self.bin())
            .args(args)
            .current_dir(&self.root)
            .env("HOME", self.root.join("home"))
            .output()
            .unwrap();
        let said =
            format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        (output.status.success(), said)
    }

    /// Installs a copy into the tree's own folder, which must work.
    fn copy(&self, more: &[&str]) -> String {
        let to = self.to();
        let mut args = vec!["--to", to.to_str().unwrap()];
        args.extend(more);
        let (ok, said) = self.install(&args);
        assert!(ok, "{said}");
        said
    }
}

fn text(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

#[test]
fn an_install_is_a_copy_in_a_folder_of_its_own_with_the_commands_linked_to_it() {
    let tree = Tree::new("copy");
    let said = tree.copy(&["--desktop"]);
    let to = tree.to();

    assert_eq!(text(&to.join("factory")), "program one");
    assert_eq!(text(&to.join("desktop")), "program one, in a window");
    assert_eq!(text(&to.join("ui/index.html")), "<meta name=\"build\" content=\"one\">");
    assert_eq!(text(&to.join("ui/assets/app-ONE.js")), "part of one");
    assert_eq!(text(&to.join("extensions/first/extension.yaml")), "extension: first\n");

    // The commands are the copies, not what cargo built: what is built
    // here afterwards is not what they run.
    let command = fs::canonicalize(tree.bin().join("sm")).unwrap();
    assert_eq!(command, fs::canonicalize(to.join("factory")).unwrap());
    let app = fs::canonicalize(tree.bin().join("sample")).unwrap();
    assert_eq!(app, fs::canonicalize(to.join("desktop")).unwrap());
    tree.built("program two");
    assert_eq!(text(&tree.bin().join("sm")), "program one", "a build changes nothing that is installed");

    assert!(said.contains("is still the one from before"), "{said}");
    assert!(said.contains("To upgrade, run this again."), "{said}");
}

#[test]
fn what_a_build_leaves_beside_the_programs_is_not_installed() {
    let tree = Tree::new("beside");
    // A build of the command line's crate leaves more than the program:
    // the stand-in its tests start is a program of that crate too, so
    // that cargo builds it for them, and is no part of the product.
    for beside in ["cli-standin", "cli-standin.exe", "factory.d", "libcli.rlib"] {
        write(&tree.root.join("target/fast").join(beside), "not the product's");
    }
    tree.copy(&["--desktop"]);

    // What is installed is the programs by their names, and nothing a
    // build happened to leave where they are.
    let mut copied: Vec<String> = Vec::new();
    for folder in [tree.to(), tree.bin()] {
        for entry in fs::read_dir(folder).unwrap().flatten() {
            copied.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    assert!(copied.iter().any(|name| name == "factory"), "{copied:?}");
    assert!(!copied.iter().any(|name| name.contains("standin") || name.contains("libcli")), "{copied:?}");
    assert!(!copied.iter().any(|name| name == "factory.d"), "{copied:?}");
}

#[test]
fn installing_again_replaces_a_program_without_writing_into_the_one_that_is_running() {
    let tree = Tree::new("running");
    tree.copy(&[]);
    // A program that is running is the file it was started from, held
    // open. Writing into that file would change it under the program.
    let mut running = fs::File::open(tree.to().join("factory")).unwrap();

    tree.built("program two");
    tree.copy(&[]);

    assert_eq!(text(&tree.to().join("factory")), "program two", "the next start is the new one");
    let mut held = String::new();
    running.read_to_string(&mut held).unwrap();
    assert_eq!(held, "program one", "the one that is running is still what it was");
    let left: Vec<String> = fs::read_dir(tree.to())
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".new") || name.ends_with(".old"))
        .collect();
    assert!(left.is_empty(), "nothing half-made is left behind: {left:?}");
}

#[test]
fn installing_again_puts_the_new_client_in_and_leaves_the_ones_before_for_pages_still_on_them() {
    let tree = Tree::new("client");
    tree.copy(&[]);
    let ui = tree.to().join("ui");

    tree.client("two", &["assets/app-TWO.js", "lib/chart.js"]);
    tree.copy(&[]);
    assert_eq!(text(&ui.join("index.html")), "<meta name=\"build\" content=\"two\">");
    assert_eq!(text(&ui.join("assets/app-TWO.js")), "part of two");
    assert_eq!(
        text(&ui.join("assets/app-ONE.js")),
        "part of one",
        "a page still on the client before this one can fetch the parts it has not loaded"
    );

    tree.client("three", &["assets/app-THREE.js", "lib/chart.js"]);
    tree.copy(&[]);
    assert_eq!(text(&ui.join("index.html")), "<meta name=\"build\" content=\"three\">");
    assert!(ui.join("assets/app-TWO.js").is_file(), "the one before stays");
    assert!(ui.join("assets/app-ONE.js").is_file(), "and the one before that: a page is left open for days");
    assert_eq!(
        text(&ui.join("lib/chart.js")),
        "part of three",
        "a file whose name does not change is replaced"
    );

    // The packages are what came with this install, and no more.
    fs::remove_dir_all(tree.root.join("extensions/first")).unwrap();
    write(&tree.root.join("extensions/second/extension.yaml"), "extension: second\n");
    tree.copy(&[]);
    assert!(tree.to().join("extensions/second/extension.yaml").is_file());
    assert!(!tree.to().join("extensions/first").exists());
}

#[test]
fn a_linked_install_copies_nothing_and_follows_the_checkout() {
    let tree = Tree::new("linked");
    let (ok, said) = tree.install(&["--link"]);
    assert!(ok, "{said}");
    assert!(!tree.to().exists());
    assert!(!tree.root.join("home").exists(), "nothing is put anywhere but where the commands go");
    assert_eq!(
        fs::canonicalize(tree.bin().join("sm")).unwrap(),
        fs::canonicalize(tree.root.join("target/fast/factory")).unwrap()
    );
    tree.built("program two");
    assert_eq!(text(&tree.bin().join("sm")), "program two", "what is built is what the command runs");
    assert!(said.contains("These follow this checkout"), "{said}");

    let to = tree.to();
    let (ok, said) = tree.install(&["--link", "--to", to.to_str().unwrap()]);
    assert!(!ok && said.contains("`--link` copies nothing"), "{said}");
}

#[test]
fn a_folder_cargo_builds_into_and_a_client_that_was_never_built_are_refused() {
    let tree = Tree::new("refused");
    // Cargo marks the folders it builds into; a program in one of those
    // reads the checkout, which is what a copy is for not doing.
    let builds = tree.root.join("target/fast");
    fs::create_dir_all(builds.join(".fingerprint")).unwrap();
    let (ok, said) = tree.install(&["--to", builds.to_str().unwrap()]);
    assert!(!ok && said.contains("is a folder cargo builds into"), "{said}");

    fs::remove_dir_all(tree.root.join("ui/dist")).unwrap();
    let to = tree.to();
    let (ok, said) = tree.install(&["--to", to.to_str().unwrap()]);
    assert!(!ok && said.contains("the client has not been built"), "{said}");
    assert!(!to.exists(), "and nothing was put anywhere");

    fs::remove_file(tree.root.join("target/fast/factory")).unwrap();
    let (ok, said) = tree.install(&["--to", to.to_str().unwrap()]);
    assert!(!ok && said.contains("has not been built: run this without `--no-build`"), "{said}");
}

#[cfg(target_os = "linux")]
#[test]
fn an_install_with_the_window_puts_its_menu_entry_and_icon_under_the_folder_it_was_given_and_in_nobodys_home()
{
    let tree = Tree::new("menu");
    let icon = tree.root.join("desktop/icon");
    write(&icon.join("icon.svg"), "a drawing");
    write(&icon.join("icon-48.png"), "small");
    write(&icon.join("README.md"), "what an icon has to be");

    // Without the window there is nothing to put in a menu.
    tree.copy(&[]);
    assert!(!tree.to().join("share").exists());

    let said = tree.copy(&["--desktop"]);
    let share = tree.to().join("share");
    let entry = text(&share.join("applications/sample.desktop"));
    assert!(entry.contains("Name=Sample\n") && entry.contains("Icon=sample\n"), "{entry}");
    assert!(
        entry
            .contains(&format!("Exec=\"{}\"\n", tree.bin().canonicalize().unwrap().join("sample").display())),
        "it starts the command the window is run by: {entry}"
    );
    assert_eq!(text(&share.join("icons/hicolor/48x48/apps/sample.png")), "small");
    assert_eq!(text(&share.join("icons/hicolor/scalable/apps/sample.svg")), "a drawing");
    assert!(!share.join("icons/hicolor/256x256").exists(), "a picture that is not drawn is not made up");
    assert!(said.contains("puts it in the desktop's menu"), "{said}");
    // Told where to install, it wrote nowhere else: not in the home it was run with.
    assert!(!tree.root.join("home").exists(), "nothing was put in anybody's home");
}

#[test]
fn an_install_keeps_the_builds_it_served_in_the_last_week_twenty_at_most_and_carries_over_what_one_from_before_kept()
 {
    let tree = Tree::new("generations");
    let (ui, kept) = (tree.to().join("ui"), tree.to().join("client.kept"));
    // What an install before this kept: the files of the one build it had
    // put in place, a line each.
    write(&ui.join("assets/app-BEFORE.js"), "part of before");
    write(&kept, "assets/app-BEFORE.js\n");
    tree.copy(&[]);
    assert!(ui.join("assets/app-BEFORE.js").is_file(), "what an install from before kept is kept");
    assert!(text(&kept).starts_with("build one "), "{}", text(&kept));

    // Twenty-two more: the twenty before the last are kept, and nothing
    // older, of what the copy served or of what was there before.
    let names: Vec<String> = (0..22).map(|at| format!("n{at}")).collect();
    for name in &names {
        tree.client(name, &[&format!("assets/app-{name}.js")]);
        tree.copy(&[]);
    }
    let have = |name: &str| ui.join(format!("assets/app-{name}.js")).is_file();
    assert!(names[1..].iter().all(|name| have(name)), "{:?}", files_in(&ui));
    assert!(!have("n0") && !have("ONE") && !have("BEFORE"), "{:?}", files_in(&ui));

    // One whose place was taken more than a week ago goes, with every one
    // before it: what is kept says when each was put in place.
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
    let day = 24 * 60 * 60 * 1000;
    let aged = format!(
        "build n21 {}\nassets/app-n21.js\nbuild n20 {}\nassets/app-n20.js\nbuild n19 {}\nassets/app-n19.js\n",
        now - 8 * day,
        now - 9 * day,
        now - 10 * day
    );
    fs::write(&kept, aged).unwrap();
    tree.client("last", &["assets/app-last.js"]);
    tree.copy(&[]);
    assert!(have("last") && have("n21"), "the one it replaces stays however long it was current");
    assert!(!have("n20") && !have("n19"), "{:?}", files_in(&ui));
    assert!(!have("n18"), "what nothing kept says is kept is taken away: {:?}", files_in(&ui));
}

#[test]
fn an_install_takes_the_one_build_its_entry_page_is_of_and_none_of_what_the_checkout_keeps_besides() {
    let tree = Tree::new("one-build");
    let dist = tree.root.join("ui/dist");
    write(&dist.join("index.html"), "<meta name=\"build\" content=\"new\">");
    write(&dist.join("assets/app-NEW.js"), "part of new");
    write(&dist.join("assets/app-OLD.js"), "part of old");
    write(&dist.join("built-from.json"), "{}");
    write(
        &dist.join("builds.json"),
        "[{\"id\":\"new\",\"at\":2,\"files\":[\"assets/app-NEW.js\"]},{\"id\":\"old\",\"at\":1,\"files\":[\"assets/app-OLD.js\"]}]",
    );
    tree.copy(&[]);
    assert_eq!(files_in(&tree.to().join("ui")), ["assets/app-NEW.js", "index.html"]);
}

fn files_in(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, found: &mut Vec<String>) {
        for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
            let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
            if entry.path().is_dir() {
                walk(&entry.path(), &format!("{name}/"), found);
            } else {
                found.push(name);
            }
        }
    }
    let mut found = Vec::new();
    walk(dir, "", &mut found);
    found.sort();
    found
}
