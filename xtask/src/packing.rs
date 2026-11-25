//! `cargo xtask package`: makes a package, the folder someone downloads.
//!
//! What a package is, is in `package.rs` (the library half). This builds
//! what a release is built with and puts it together under `target/package`:
//! the folder, the same as one file, and that file's SHA-256 beside it.
//! Nothing is installed and nobody's home is touched; the folder is made
//! new each time.
//!
//! It makes the package of the system it is run on. `--target TRIPLE`
//! builds for another target, as cargo names one, and makes that system's
//! package, which then says of itself that it was built somewhere else and
//! is for trying; `--cargo-build WORD` asks cargo `WORD` in place of
//! `build` (`zigbuild`, where that is what builds for the other system),
//! and `--no-window` leaves out the window's program, which may not build
//! for another system from here.
//!
//! `--no-build` packages what is built already, `--profile NAME` builds and
//! packages another profile than the one a release uses, and `--out DIR`
//! puts the three somewhere else.
//!
//! `--nightly YYYYMMDD` makes a nightly of the commit that is checked out:
//! called `nightly` where a release's name has its version, so that each
//! night's replaces the one before under the same name, and saying inside
//! it which day and which commit it is (`package::nightly_version`), which
//! the daemon then says as its version. The day is said and not read from
//! the clock, so that the packages of one night, made on three machines,
//! are one version whichever of them finished after midnight.

use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use xtask::package::{self, About, System};
use xtask::{menu, notices, sha256, tools, zip};

use crate::install::{ICON, NOTE, could_not, files_under, place_tree, revision, run, whole};
use crate::repo::Repo;

/// The script in a package that puts it in a desktop's menu.
const MENU: &str = "menu.sh";

/// The system this is being built on, for the package to say of itself:
/// with its C library on Linux, where a program does not start on a
/// system older than that.
fn built_on(made_on: System) -> String {
    let said = |program: &str, args: &[&str]| {
        let output = Command::new(program).args(args).output().ok()?;
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        (output.status.success() && !text.is_empty()).then_some(text)
    };
    match made_on {
        System::Linux => {
            let system = fs::read_to_string("/etc/os-release").ok().and_then(|text| {
                text.lines()
                    .find_map(|line| line.strip_prefix("PRETTY_NAME="))
                    .map(|name| name.trim_matches('"').to_string())
            });
            let library = said("getconf", &["GNU_LIBC_VERSION"]);
            match (system, library) {
                (Some(system), Some(library)) => format!("{system}, with {library}"),
                (Some(system), None) => system,
                (None, Some(library)) => format!("a system with {library}"),
                (None, None) => "a system that did not say what it is".to_string(),
            }
        }
        // Asked as the system itself says what it is, and called by its
        // name alone where it does not answer.
        System::Macos => match said("sw_vers", &["-productVersion"]) {
            Some(version) => format!("macOS {version}"),
            None => "macOS".to_string(),
        },
        // Windows says which it is to its shell and to nothing a program
        // may ask without one, and the tooling asks no shell.
        System::Windows => "Windows".to_string(),
    }
}

/// A program put in the package under the name it is run by, and made
/// something that can be run whatever it was copied from.
fn place_program(from: &Path, to: &Path) -> Result<(), String> {
    fs::copy(from, to).map_err(|err| could_not("write", to, &err))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(to, fs::Permissions::from_mode(0o755))
            .map_err(|err| could_not("make runnable", to, &err))?;
    }
    Ok(())
}

fn size(path: &Path) -> String {
    let bytes = fs::metadata(path).map(|file| file.len()).unwrap_or(0);
    format!("{:.1} MB", bytes as f64 / 1_048_576.0)
}

/// The folder as one zip, written here (`xtask::zip`): every file under
/// the folder's own name, in the order of their names, and the programs
/// said to be programs for whoever unpacks it where that is said of a file.
fn zipped(folder: &Path, name: &str, archive: &Path, programs: &[String]) -> Result<(), String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs());
    let file = fs::File::create(archive).map_err(|err| could_not("write", archive, &err))?;
    let mut zip = zip::Zip::new(BufWriter::new(file), now);
    for file in files_under(folder) {
        let from = folder.join(&file);
        let bytes = fs::read(&from).map_err(|err| could_not("read", &from, &err))?;
        zip.file(&format!("{name}/{file}"), &bytes, programs.contains(&file))
            .map_err(|err| could_not("write", archive, &err))?;
    }
    zip.finish().map(|_| ()).map_err(|err| could_not("write", archive, &err))
}

/// The folder as one compressed file, made by the `tar` of the system
/// this is run on, asked what `package::tar` says that one is asked.
fn tarred(made_on: System, out: &Path, folder: &Path, name: &str, archive: &Path) -> Result<(), String> {
    // What names everything in order, for a `tar` that is handed the
    // order: beside the folder, and gone when the file is made.
    let list = out.join(format!("{name}.files"));
    let asked = package::tar(made_on, &archive.to_string_lossy(), name, &format!("{name}.files"))?;
    let mut command = Command::new(tools::here("tar"));
    command.args(&asked).current_dir(out);
    if made_on == System::Macos {
        let mut names = package::listed(name, &files_under(folder)).join("\n");
        names.push('\n');
        fs::write(&list, names).map_err(|err| could_not("write", &list, &err))?;
        // Or that system's `tar` writes a second file beside each file,
        // holding what the system keeps of it apart from what is in it.
        command.env("COPYFILE_DISABLE", "1");
    }
    println!("  tar {}", asked.join(" "));
    let status = command.status();
    let _ = fs::remove_file(&list);
    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => Err(format!(
            "`tar {}` failed. The folder is made; the one file of it needs `tar`",
            asked.join(" ")
        )),
        Err(err) => {
            Err(format!("could not run `tar`: {err}. The folder is made; the one file of it needs `tar`"))
        }
    }
}

/// The commit that is checked out, as git writes one short, or nothing
/// where git cannot say.
fn commit(repo: &Repo) -> Option<String> {
    let output =
        Command::new("git").args(["rev-parse", "--short", "HEAD"]).current_dir(&repo.root).output().ok()?;
    let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (output.status.success() && !commit.is_empty()).then_some(commit)
}

pub fn run_package(repo: &Repo, args: &[String]) -> Result<(), String> {
    let has = |flag: &str| args.iter().any(|arg| arg == flag);
    let value = |flag: &str| args.iter().position(|arg| arg == flag).and_then(|at| args.get(at + 1));
    let cli = repo.brand("cli").ok_or("brand.toml has no `cli`")?;
    let display = repo.brand("display").unwrap_or_else(|| cli.clone());
    let id = repo.brand("id").unwrap_or_else(|| cli.clone());
    let version = repo
        .read("Cargo.toml")
        .and_then(|manifest| package::version(&manifest))
        .ok_or("the root Cargo.toml says no `version` under [workspace.package], so there is nothing to call the package")?;
    // Before anything is built: a system there is no package for is told
    // so at once, and what there is for it instead; and so is a system
    // that cannot make the one file of the package asked for.
    let made_on = package::on(std::env::consts::OS)?;
    if has("--target") && value("--target").is_none() {
        return Err(
            "`--target` is followed by a target as cargo names one, such as x86_64-pc-windows-gnu".into()
        );
    }
    let target = value("--target").map(String::as_str);
    let (system, processor) = match target {
        Some(target) => package::target(target)?,
        None => (made_on, std::env::consts::ARCH.to_string()),
    };
    let elsewhere = (system != made_on).then_some(made_on);
    // A nightly says the day it is of and the commit it was made of, or it
    // is not made: its version is the one thing that tells one from another.
    let nightly = match (has("--nightly"), value("--nightly")) {
        (false, _) => None,
        (true, None) => {
            return Err(
                "`--nightly` is followed by the day the nightly is of, as YYYYMMDD in UTC (`20261124`)"
                    .into(),
            );
        }
        (true, Some(day)) => {
            let day = package::nightly_day(day)?;
            let commit = commit(repo).ok_or(
                "a nightly says which commit it was made of, and git could not say which commit this is: make it from a clone of the repository",
            )?;
            Some(package::nightly_version(&version, day, &commit))
        }
    };
    let name =
        package::named(&id, if nightly.is_some() { package::NIGHTLY } else { &version }, system, &processor);
    // What the package says it is from here on: a nightly's own version.
    let version = nightly.clone().unwrap_or(version);
    if system != System::Windows {
        package::tar(made_on, "", &name, "")?;
    }
    let window = !has("--no-window");
    // Each crate that is a program, what cargo calls its program and what
    // the package calls it.
    let programs: Vec<(&str, String, String)> =
        [("cli", "factory", &cli, true), ("desktop", "desktop", &id, window)]
            .into_iter()
            .filter(|(_, _, _, wanted)| *wanted)
            .map(|(krate, program, command, _)| (krate, system.program(program), system.program(command)))
            .collect();

    let profile = value("--profile").map_or(package::PROFILE, String::as_str);
    let build = value("--cargo-build").map_or("build", String::as_str);
    let ui = repo.root.join("ui");
    let built = repo.root.join(package::built_in(target, profile));
    if !has("--no-build") {
        match elsewhere {
            None => println!("building {display} {version} as it is released"),
            Some(_) => println!("building {display} {version} for {}, to be tried there", system.called()),
        }
        if ui.join("node_modules").is_dir() {
            run("npm", &["run", "--silent", "build"], &ui)?;
        }
        for (krate, _, _) in &programs {
            let asked = package::build(build, profile, target, krate);
            run("cargo", &asked.iter().map(String::as_str).collect::<Vec<_>>(), &repo.root)?;
        }
    }
    // Before anything is put anywhere: half a package is worse than none.
    for (_, program, _) in &programs {
        if !built.join(program).is_file() {
            return Err(format!(
                "{} has not been built: run this without `--no-build`",
                built.join(program).display()
            ));
        }
    }
    let dist = ui.join("dist");
    let entry = fs::read_to_string(dist.join("index.html"))
        .map_err(|_| "the client has not been built: run `npm install` in ui/ first, then this again")?;
    for licence in ["LICENSE-MIT", "LICENSE-APACHE"] {
        if !repo.root.join(licence).is_file() {
            return Err(format!(
                "{licence} is not at the top of the repository, and a package says what it is offered under"
            ));
        }
    }
    // The longer walk goes with it: its page names it, and somebody who
    // has a package has no repository to read it in.
    let walk = repo.root.join("docs").join("first-run.md");
    let walked = fs::read_to_string(&walk).map_err(|_| {
        format!(
            "docs/first-run.md is not in the repository, and a package carries the notes of the first run as `{}`",
            package::NOTES
        )
    })?;
    if !repo.root.join(notices::FILE).is_file() {
        return Err(format!(
            "{} is not at the top of the repository, and a package carries the notices of what it bundles: `{}` makes it, and it is committed",
            notices::FILE,
            notices::COMMAND
        ));
    }

    let out = value("--out").map_or_else(|| repo.root.join("target").join("package"), PathBuf::from);
    fs::create_dir_all(&out).map_err(|err| could_not("create", &out, &err))?;
    let out = whole(&out).map_err(|err| could_not("find", &out, &err))?;
    let folder = out.join(&name);
    let file = format!("{name}{}", system.archive());
    let archive = out.join(&file);
    let sum = out.join(format!("{file}.sha256"));
    // New each time: a file a package once held and no longer does must
    // not be in the next one.
    let _ = fs::remove_dir_all(&folder);
    let _ = fs::remove_file(&archive);
    let _ = fs::remove_file(&sum);
    fs::create_dir_all(&folder).map_err(|err| could_not("create", &folder, &err))?;

    for (_, program, command) in &programs {
        place_program(&built.join(program), &folder.join(command))?;
    }
    let builds = fs::read_to_string(dist.join("builds.json")).ok();
    let client = package::client_files(&files_under(&dist), &entry, builds.as_deref());
    for file in &client {
        let to = folder.join("ui").join(file);
        if let Some(dir) = to.parent() {
            fs::create_dir_all(dir).map_err(|err| could_not("create", dir, &err))?;
        }
        fs::copy(dist.join(file), &to).map_err(|err| could_not("write", &to, &err))?;
    }
    let packages = place_tree(&repo.root.join("extensions"), &folder.join("extensions"))?;
    for licence in ["LICENSE-MIT", "LICENSE-APACHE", notices::FILE] {
        let to = folder.join(licence);
        fs::copy(repo.root.join(licence), &to).map_err(|err| could_not("write", &to, &err))?;
    }
    let revision = revision(repo);
    if let Some(said) =
        package::note(revision.as_deref(), elsewhere.map(|on| (on, system)), nightly.as_deref())
    {
        let note = folder.join(NOTE);
        fs::write(&note, format!("{said}\n")).map_err(|err| could_not("write", &note, &err))?;
    }
    // A desktop's menu is Linux's, and an entry there starts the window:
    // a package for another system, or one with no window in it, carries
    // neither the pictures nor what files them.
    let in_a_menu = system == System::Linux && window;
    let mut icons = Vec::new();
    if in_a_menu {
        // The pictures of the icon, when there are any: not the page about
        // them, the script that makes them, or the window's own pixels,
        // which are in the program.
        icons =
            files_under(&repo.root.join(ICON)).into_iter().filter(|file| menu::is_picture(file)).collect();
        for file in &icons {
            let to = folder.join("icon").join(file);
            if let Some(dir) = to.parent() {
                fs::create_dir_all(dir).map_err(|err| could_not("create", dir, &err))?;
            }
            fs::copy(repo.root.join(ICON).join(file), &to).map_err(|err| could_not("write", &to, &err))?;
        }
        // What puts the folder in a desktop's menu, from wherever it is
        // unpacked: an entry names its program by its whole path.
        let filed = repo.brand("app_id").unwrap_or_else(|| id.clone());
        let script = folder.join(MENU);
        fs::write(&script, menu::script(&display, &filed, &id))
            .map_err(|err| could_not("write", &script, &err))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&script, fs::Permissions::from_mode(0o755))
                .map_err(|err| could_not("make runnable", &script, &err))?;
        }
    }
    let built_on = built_on(made_on);
    let about = About {
        display: &display,
        cli: &cli,
        id: &id,
        version: &version,
        system,
        processor: &processor,
        revision: revision.as_deref(),
        built_on: &built_on,
        elsewhere,
        window,
        nightly: nightly.is_some(),
        address: "http://127.0.0.1:7700",
    };
    let readme = folder.join("README.md");
    fs::write(&readme, package::readme(&about)).map_err(|err| could_not("write", &readme, &err))?;
    let notes = folder.join(package::NOTES);
    fs::write(&notes, package::notes(&walked, &about)).map_err(|err| could_not("write", &notes, &err))?;

    // One file of it. Whoever made it is not part of it: the files are
    // nobody's in particular, and in the same order every time.
    if system == System::Windows {
        let commands: Vec<String> = programs.iter().map(|(_, _, command)| command.clone()).collect();
        zipped(&folder, &name, &archive, &commands)?;
    } else {
        tarred(made_on, &out, &folder, &name, &archive)?;
    }
    let digest = fs::File::open(&archive)
        .and_then(sha256::of)
        .map_err(|err| could_not("work out the SHA-256 of", &archive, &err))?;
    fs::write(&sum, sha256::line(&digest, &file)).map_err(|err| could_not("write", &sum, &err))?;

    println!(
        "packaged {display} {version} for {} ({processor})  ({} programs, {} files of the client, {packages} files of packages{})",
        system.called(),
        programs.len(),
        client.len(),
        match (in_a_menu, icons.len()) {
            (false, _) => String::new(),
            (true, 0) => ", no icon yet".to_string(),
            (true, pictures) => format!(", {pictures} of an icon"),
        }
    );
    println!("  {}{}   as it is unpacked", folder.display(), std::path::MAIN_SEPARATOR);
    println!("  {}   {}", archive.display(), size(&archive));
    println!("  {}   {digest}", sum.display());
    let command = system.program(&cli);
    match (elsewhere, system) {
        (Some(on), _) => println!(
            "\nBuilt on {} for {}: it is for trying there, and nothing in it is run here. It is never a release.",
            on.called(),
            system.called()
        ),
        (None, System::Windows) => println!(
            "\nTo try it here, touching nothing that is installed:\n  cd {}\n  .\\{command} up --profile try\nand in a second terminal there:\n  .\\{command} --profile try open",
            folder.display()
        ),
        (None, System::Linux | System::Macos) => println!(
            "\nTo try it here, touching nothing that is installed:\n  cd {}\n  ./{command} up --profile try &\n  ./{command} --profile try open",
            folder.display()
        ),
    }
    Ok(())
}
