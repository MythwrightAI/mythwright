//! `cargo xtask install`: puts a copy of the product where it is used.
//!
//! What is installed is a copy, in a folder of its own: the programs, the
//! client and the packages that come with it. The command names are links
//! to the copies. A copy reads nothing from this checkout (the standard
//! blueprints are built into the program, and it serves the client and
//! offers the packages that are beside it), so a factory somebody is using
//! is not changed by anything built or edited here. It changes when this
//! is run again, and from then on when its daemon is next started.
//!
//! Installing over a copy that is running must not break it:
//!
//! - a program is written beside the one it replaces and renamed over it,
//!   so the one that is running goes on being the file it was started from;
//! - the client's files go in first and its entry page last, in one step,
//!   which is the moment the new client exists; the files of the builds
//!   installed before stay, for pages still on one of them, by the rule the
//!   client's own build keeps its folder by (`ui/build.mjs`,
//!   `xtask::generations`): those replaced within a week, twenty at most.
//!   What is installed is the one build the entry page is of, as a package
//!   takes it; the earlier builds are the copy's own, of what it served;
//! - the packages are copied whole into a folder beside and swapped in.
//!
//! `--link` is the other thing: the command names are links to the programs
//! where cargo builds them, nothing is copied, and what they run follows
//! this checkout as it is built and edited.
//!
//! `--no-build` installs what is built already, and `--profile NAME` builds
//! and installs another profile than `fast`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use xtask::generations::{self, Build};
use xtask::{menu, package, tools};

use crate::repo::Repo;

/// Where the icon is drawn and its pictures are kept (`desktop/icon/README.md`).
pub(crate) const ICON: &str = "desktop/icon";
/// The builds of the client the copy has served and keeps, with their
/// files (`xtask::generations`), kept beside the client and not in it: the
/// client's folder is served as it is.
const CLIENT_KEPT: &str = "client.kept";
/// One line saying what was installed, which the program reads and says.
pub(crate) const NOTE: &str = "installed";

/// Runs a tool and waits for it, saying what was asked. The tool is
/// named as people name it, and started as this system starts it
/// (`xtask::tools`).
pub(crate) fn run(program: &str, args: &[&str], dir: &Path) -> Result<(), String> {
    println!("  {program} {}", args.join(" "));
    let status = Command::new(tools::here(program))
        .args(args)
        .current_dir(dir)
        .status()
        .map_err(|err| format!("could not run `{program}`: {err}"))?;
    if status.success() { Ok(()) } else { Err(format!("`{program} {}` failed", args.join(" "))) }
}

pub(crate) fn could_not(what: &str, path: &Path, err: &std::io::Error) -> String {
    format!("could not {what} {}: {err}", path.display())
}

/// `path` from the top, for a sentence, a menu entry or a command file:
/// the system's own, with links followed, wherever that is a path anybody
/// would write. On Windows the system's own is in a form of its own
/// (`\\?\C:\…`), which a command file may not run and nobody reads;
/// what says such a path plainly again is the application's
/// (`kernel::whole`), and this tooling stands on the standard library
/// alone. So there it is the path as it was written, said from the top,
/// its links not followed. It is there to be found either way.
pub(crate) fn whole(path: &Path) -> std::io::Result<PathBuf> {
    let followed = path.canonicalize()?;
    if cfg!(windows) { std::path::absolute(path) } else { Ok(followed) }
}

/// The folder of whoever runs this: `HOME`, and what a system with no
/// `HOME` says in its place.
fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from)
}

/// A name beside `path` that nothing else is called.
fn beside(path: &Path, word: &str) -> PathBuf {
    let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    path.with_file_name(format!(".{name}.{word}"))
}

/// Puts `content` (a file to copy, or text) at `to` so that whoever reads
/// `to` reads the old file or the new one and never half of either, and so
/// that a program started from the old file goes on running it.
pub(crate) fn replace(to: &Path, write: impl FnOnce(&Path) -> std::io::Result<()>) -> Result<(), String> {
    if let Some(dir) = to.parent() {
        fs::create_dir_all(dir).map_err(|err| could_not("create", dir, &err))?;
    }
    let new = beside(to, "new");
    write(&new).map_err(|err| could_not("write", &new, &err))?;
    if fs::rename(&new, to).is_ok() {
        return Ok(());
    }
    // Where a file in use cannot be written over, it can still be moved
    // aside: the program that is running keeps it, under another name.
    let old = beside(to, "old");
    let _ = fs::remove_file(&old);
    fs::rename(to, &old).map_err(|err| could_not("move aside", to, &err))?;
    fs::rename(&new, to).map_err(|err| could_not("put in place", to, &err))
}

pub(crate) fn copy_to(from: &Path, to: &Path) -> Result<(), String> {
    replace(to, |new| fs::copy(from, new).map(|_| ()))
}

/// Every file under `dir`, as paths relative to it with `/` between parts.
pub(crate) fn files_under(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, found: &mut Vec<String>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = format!("{prefix}{name}");
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                walk(&entry.path(), &format!("{path}/"), found);
            } else {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    walk(dir, "", &mut found);
    found.sort();
    found
}

/// The client: the files of the build its entry page is of first, its
/// entry page last, and the files of the builds installed before this one
/// that are still kept left where they are.
fn place_client(from: &Path, to: &Path, kept: &Path) -> Result<usize, String> {
    const ENTRY: &str = "index.html";
    let entry =
        fs::read_to_string(from.join(ENTRY)).map_err(|err| could_not("read", &from.join(ENTRY), &err))?;
    let builds = fs::read_to_string(from.join("builds.json")).ok();
    // The one build, as a package takes it: nobody's page is on the
    // checkout's earlier builds, which this copy never served.
    let ours = package::client_files(&files_under(from), &entry, builds.as_deref());
    let files: Vec<String> = ours.into_iter().filter(|file| file != ENTRY).collect();
    let made = Build {
        id: package::build_of(&entry).unwrap_or("unnamed").to_string(),
        at: None,
        files: files.iter().cloned().chain([ENTRY.to_string()]).collect(),
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| u64::try_from(since.as_millis()).unwrap_or(u64::MAX));
    let before = generations::read(&fs::read_to_string(kept).unwrap_or_default());
    let keeping = generations::generations(&before, &made, now);
    for file in &files {
        let (source, target) = (from.join(file), to.join(file));
        // A name under `assets/` carries a hash of what is in it: one that
        // is already there is already right.
        if file.starts_with("assets/") && target.is_file() {
            continue;
        }
        copy_to(&source, &target)?;
    }
    replace(kept, |new| fs::write(new, generations::write(&keeping)))?;
    // The moment the new client exists. The daemon is told the page
    // changed and tells the pages that are open.
    copy_to(&from.join(ENTRY), &to.join(ENTRY))?;
    for old in generations::leftovers(&keeping, &files_under(to)) {
        if old != ENTRY {
            let _ = fs::remove_file(to.join(&old));
        }
    }
    Ok(files.len() + 1)
}

/// A folder copied whole: made beside where it goes, then swapped in.
pub(crate) fn place_tree(from: &Path, to: &Path) -> Result<usize, String> {
    let (new, old) = (beside(to, "new"), beside(to, "old"));
    let _ = fs::remove_dir_all(&new);
    let files = files_under(from);
    for file in &files {
        let target = new.join(file);
        if let Some(dir) = target.parent() {
            fs::create_dir_all(dir).map_err(|err| could_not("create", dir, &err))?;
        }
        fs::copy(from.join(file), &target).map_err(|err| could_not("write", &target, &err))?;
    }
    let _ = fs::remove_dir_all(&old);
    if to.exists() {
        fs::rename(to, &old).map_err(|err| could_not("move aside", to, &err))?;
    }
    fs::rename(&new, to).map_err(|err| could_not("put in place", to, &err))?;
    let _ = fs::remove_dir_all(&old);
    Ok(files.len())
}

/// Makes `at` the way to run `target`: a link, or where there are none to
/// be had, a one-line command file that runs it.
fn link(target: &Path, at: &Path) -> Result<PathBuf, String> {
    #[cfg(unix)]
    {
        let _ = fs::remove_file(at);
        std::os::unix::fs::symlink(target, at).map_err(|err| could_not("link", at, &err))?;
        Ok(at.to_path_buf())
    }
    #[cfg(not(unix))]
    {
        let at = at.with_extension("cmd");
        replace(&at, |new| fs::write(new, format!("@\"{}\" %*\r\n", target.display())))?;
        Ok(at)
    }
}

/// What this checkout is at, for the copy to say of itself: the revision,
/// and whether what was installed is more than that revision.
pub(crate) fn revision(repo: &Repo) -> Option<String> {
    let git = |args: &[&str]| {
        let output = Command::new("git").args(args).current_dir(&repo.root).output().ok()?;
        output.status.success().then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
    };
    let revision = git(&["rev-parse", "--short", "HEAD"]).filter(|revision| !revision.is_empty())?;
    let changed = git(&["status", "--porcelain"]).is_some_and(|changes| !changes.is_empty());
    Some(format!("revision {revision}{}", if changed { " and changes that are not committed" } else { "" }))
}

/// Where a copy goes when nobody says: a folder of the product's own among
/// the user's programs, and never among its data.
fn default_folder(id: &str) -> Result<PathBuf, String> {
    if cfg!(windows) {
        let local =
            std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA is not set; say where with --to DIR")?;
        Ok(PathBuf::from(local).join("Programs").join(id))
    } else {
        let home = std::env::var_os("HOME").ok_or("HOME is not set; say where with --to DIR")?;
        Ok(PathBuf::from(home).join(".local").join("lib").join(id))
    }
}

/// Where a person's own data is kept, which is where a desktop looks for
/// menu entries and icons.
fn data_home() -> Result<PathBuf, String> {
    if let Some(data) = std::env::var_os("XDG_DATA_HOME").filter(|data| !data.is_empty()) {
        return Ok(PathBuf::from(data));
    }
    let home = std::env::var_os("HOME").ok_or("HOME is not set, so there is nowhere to put a menu entry")?;
    Ok(PathBuf::from(home).join(".local").join("share"))
}

/// Puts the product in a desktop's menu (`xtask::menu`): the pictures of
/// the icon there are, each where a desktop looks for it, and the entry,
/// which names `program` by its whole path. Gives back where the entry is.
/// With no icon drawn yet there is still an entry, and the desktop draws
/// what it draws for a program without one.
fn place_menu(
    repo: &Repo,
    data: &Path,
    display: &str,
    name: &str,
    program: &Path,
    class: &str,
) -> Result<PathBuf, String> {
    for (file, place) in menu::icon_places(name) {
        let from = repo.root.join(ICON).join(file);
        if from.is_file() {
            copy_to(&from, &data.join(place))?;
        }
    }
    let entry = data.join(menu::entry_place(name));
    let text = menu::entry(display, name, &program.to_string_lossy(), class);
    replace(&entry, |new| fs::write(new, text))?;
    Ok(entry)
}

pub fn run_install(repo: &Repo, args: &[String]) -> Result<(), String> {
    let has = |flag: &str| args.iter().any(|arg| arg == flag);
    let value = |flag: &str| args.iter().position(|arg| arg == flag).and_then(|at| args.get(at + 1));
    let (desktop, linked, build) = (has("--desktop"), has("--link"), !has("--no-build"));
    let cli = repo.brand("cli").ok_or("brand.toml has no `cli`")?;
    let display = repo.brand("display").unwrap_or_else(|| cli.clone());
    let id = repo.brand("id").unwrap_or_else(|| cli.clone());
    let bin = match value("--bin") {
        Some(dir) => PathBuf::from(dir),
        // The same place under a person's own folder on every system.
        // Windows keeps none for a person's commands; whether this one is
        // looked in for them is said at the end.
        None => {
            let unsaid = if cfg!(windows) { "neither HOME nor USERPROFILE is" } else { "HOME is not" };
            let home = home().ok_or(format!("{unsaid} set; say where with --bin DIR"))?;
            home.join(".local").join("bin")
        }
    };
    if linked && cfg!(not(unix)) {
        return Err("`--link` needs links, which this system does not give out freely. Run the program where it is built (target/fast), or install a copy".into());
    }
    if linked && value("--to").is_some() {
        return Err("`--link` copies nothing, so there is no `--to`: the commands are links to where the programs are built".into());
    }

    // Optimised, but without the link-time work a release build does, unless
    // another is asked for: this is for using the product, not for shipping
    // it. Cargo calls the profile `dev` and its folder `debug`.
    let profile = value("--profile").map_or("fast", String::as_str);
    let ui = repo.root.join("ui");
    let built = repo.root.join("target").join(if profile == "dev" { "debug" } else { profile });
    let suffix = std::env::consts::EXE_SUFFIX;
    let programs: Vec<(String, String)> = [("factory", &cli, true), ("desktop", &id, desktop)]
        .into_iter()
        .filter(|(_, _, wanted)| *wanted)
        .map(|(program, command, _)| (format!("{program}{suffix}"), format!("{command}{suffix}")))
        .collect();
    if build {
        println!("building {display}");
        if ui.join("node_modules").is_dir() {
            run("npm", &["run", "--silent", "build"], &ui)?;
        }
        run("cargo", &["build", "--profile", profile, "-p", "cli"], &repo.root)?;
        if desktop {
            run("cargo", &["build", "--profile", profile, "-p", "desktop"], &repo.root)?;
        }
    }
    for (program, _) in &programs {
        if !built.join(program).is_file() {
            return Err(format!(
                "{} has not been built: run this without `--no-build`",
                built.join(program).display()
            ));
        }
    }
    // Before anything is put anywhere: half an install is worse than none.
    if !ui.join("dist").join("index.html").is_file() {
        return Err("the client has not been built: run `npm install` in ui/ first, then this again".into());
    }
    fs::create_dir_all(&bin).map_err(|err| could_not("create", &bin, &err))?;

    if linked {
        for (program, command) in &programs {
            let at = link(&built.join(program), &bin.join(command))?;
            println!("linked {} to {}", at.display(), built.join(program).display());
        }
        println!("\nThese follow this checkout: what is built or edited here is what they run.");
    } else {
        let to = match value("--to") {
            Some(dir) => PathBuf::from(dir),
            None => default_folder(&id)?,
        };
        fs::create_dir_all(&to).map_err(|err| could_not("create", &to, &err))?;
        let to = whole(&to).map_err(|err| could_not("find", &to, &err))?;
        if to.join(".fingerprint").is_dir() {
            return Err(format!(
                "{} is a folder cargo builds into, and a program there reads this checkout. Install into a folder of its own",
                to.display()
            ));
        }
        for (program, _) in &programs {
            copy_to(&built.join(program), &to.join(program))?;
        }
        let client = place_client(&ui.join("dist"), &to.join("ui"), &to.join(CLIENT_KEPT))?;
        let packages = place_tree(&repo.root.join("extensions"), &to.join("extensions"))?;
        match revision(repo) {
            Some(revision) => replace(&to.join(NOTE), |new| fs::write(new, format!("{revision}\n")))?,
            None => {
                let _ = fs::remove_file(to.join(NOTE));
            }
        }
        println!(
            "installed {display} in {}  ({} programs, {client} files of the client, {packages} files of packages)",
            to.display(),
            programs.len()
        );
        for (program, command) in &programs {
            let at = link(&to.join(program), &bin.join(command))?;
            println!("  {} runs {}", at.display(), to.join(program).display());
        }
        if desktop && cfg!(target_os = "linux") {
            // Where a desktop looks for it: the person's own data folder.
            // An install told where to go (`--to`, `--bin`) is not one into
            // somebody's home, and keeps its entry under the folder given.
            let told = value("--to").is_some() || value("--bin").is_some();
            let data = if told { to.join("share") } else { data_home()? };
            let name = repo.brand("app_id").unwrap_or_else(|| id.clone());
            // By its whole path, wherever this was run from: an entry is
            // read by a desktop, which is nowhere in particular.
            let window = whole(&bin).unwrap_or_else(|_| bin.clone()).join(format!("{id}{suffix}"));
            let entry = place_menu(repo, &data, &display, &name, &window, &id)?;
            println!(
                "  {} puts it in the desktop's menu{}",
                entry.display(),
                if told { ", once that folder is where a desktop looks" } else { "" }
            );
        }
        println!(
            "\nA daemon that is running is still the one from before: `{cli} down`, then `{cli} up`, to run this one.\nNothing installed changes with what is built or edited here. To upgrade, run this again."
        );
    }

    let on_path =
        std::env::var_os("PATH").is_some_and(|path| std::env::split_paths(&path).any(|dir| dir == bin));
    if !on_path {
        println!(
            "note: {} is not on your PATH; add it, or run the commands by their full path",
            bin.display()
        );
    }
    println!(
        "\nnext:\n  {cli} up          start the daemon and keep it running\n  {cli} open        open the client in a browser{}",
        if desktop { format!("\n  {id}       or open it in a window of its own") } else { String::new() }
    );
    Ok(())
}
