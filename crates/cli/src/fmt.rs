//! `fmt`: lays blueprint files out the one way.
//!
//! It asks no daemon and needs nobody signed in: a text and the settings
//! its project keeps beside it are all there is to know. Given a folder it
//! takes every blueprint under it, leaving alone the folders that are
//! other tools' (`target`, `node_modules`, and any whose name begins with
//! a dot, a lane's worktree among them).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::args::Args;

/// Folders that hold what was built or fetched, not what was written.
const NOT_OURS: &[&str] = &["target", "node_modules"];

fn blueprints_under(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
        if path.is_dir() {
            if !name.starts_with('.') && !NOT_OURS.contains(&name.as_str()) {
                blueprints_under(&path, found);
            }
        } else if brand::is_blueprint_path(&name) {
            found.push(path);
        }
    }
}

pub fn fmt(args: &Args) -> ExitCode {
    let check = args.has("check");
    // A flag nobody reads is refused before anything is done (`main` says it).
    if args.heed().check().is_err() {
        return ExitCode::from(2);
    }
    let mut named: Vec<&str> = (1..).map_while(|index| args.word(index)).collect();
    if named.is_empty() {
        named.push(".");
    }
    let mut files = Vec::new();
    for name in &named {
        let path = Path::new(name);
        if path.is_dir() {
            blueprints_under(path, &mut files);
        } else if path.is_file() {
            files.push(path.to_path_buf());
        } else {
            eprintln!("error: there is no file or folder `{name}`");
            return ExitCode::FAILURE;
        }
    }

    let (mut changed, mut unread) = (0usize, 0usize);
    for file in &files {
        let shown = file.display();
        let folder = file.parent().filter(|dir| !dir.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let laid = std::fs::read_to_string(file)
            .map_err(|err| format!("{shown}: could not be read: {err}"))
            .and_then(|text| {
                let settings = api::laying::settings_for(folder, None)?;
                match blueprint::format(&text, &settings) {
                    Ok(laid) => Ok((laid != text).then_some(laid)),
                    Err(left) => Err(format!("{shown}:{}", left.message())),
                }
            });
        match laid {
            Ok(None) => {}
            Ok(Some(_)) if check => {
                changed += 1;
                println!("would change {shown}");
            }
            Ok(Some(laid)) => match std::fs::write(file, laid) {
                Ok(()) => {
                    changed += 1;
                    println!("laid out {shown}");
                }
                Err(err) => {
                    unread += 1;
                    eprintln!("error: {shown}: could not be written: {err}");
                }
            },
            Err(said) => {
                unread += 1;
                eprintln!("error: {said}");
            }
        }
    }

    let count =
        |many: usize| if many == 1 { "1 blueprint".to_string() } else { format!("{many} blueprints") };
    if unread > 0 {
        eprintln!("{} left as it was, for what is said above", count(unread));
    }
    if check && changed > 0 {
        eprintln!(
            "{} of {} not laid out as `{cli} fmt` lays them out; run `{cli} fmt {}` to rewrite {}",
            count(changed),
            files.len(),
            named.join(" "),
            if changed == 1 { "it" } else { "them" },
            cli = brand::CLI,
        );
        return ExitCode::FAILURE;
    }
    if !check {
        println!("{} looked at, {changed} laid out again", count(files.len()));
    }
    if unread > 0 { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}
