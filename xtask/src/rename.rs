//! Renaming the product.
//!
//! Code, configuration and the client never spell the name; they read it from
//! `brand.toml`. So a rename is: rewrite that file, then rewrite the prose in
//! the Markdown docs, which is the one place the name is allowed to appear
//! (all but `TRADEMARKS.md`, which says whose the name is and is left).
//!
//! One other place carries it: the names of the folders the product is kept
//! in, when the application sits beside its site and services. Those are
//! renamed as well, so that it is still one command.

use std::fs;

use crate::repo::Repo;

pub fn run(repo: &Repo, args: &[String]) -> Result<(), String> {
    let option =
        |flag: &str| args.iter().position(|arg| arg == flag).and_then(|at| args.get(at + 1)).cloned();
    let Some(display) = option("--display") else {
        return Err("usage: cargo xtask rename --display \"New Name\" [--id new-name] [--cli nn]".into());
    };
    let old = |key: &str| repo.brand(key).ok_or_else(|| format!("brand.toml has no `{key}`"));
    let (old_display, old_id, old_cli) = (old("display")?, old("id")?, old("cli")?);

    let id = option("--id").unwrap_or_else(|| slug(&display));
    let cli = option("--cli").unwrap_or_else(|| old_cli.clone());
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err(format!(
            "`{id}` is not a usable id; pass --id with lowercase letters, digits and dashes"
        ));
    }
    let app_id = old("app_id")?.replace(&old_id, &id);
    let env_prefix = option("--env-prefix").unwrap_or_else(|| cli.to_uppercase().replace('-', "_"));

    let brand_path = repo.root.join("brand.toml");
    let mut text = fs::read_to_string(&brand_path).map_err(|err| err.to_string())?;
    for (key, value) in
        [("display", &display), ("id", &id), ("cli", &cli), ("env_prefix", &env_prefix), ("app_id", &app_id)]
    {
        text = set_value(&text, key, value);
    }
    fs::write(&brand_path, text).map_err(|err| err.to_string())?;

    // The application may sit beside other folders of the same product (the
    // site, the hosted services), each a repository of its own, in a folder
    // named after it. Then their prose is rewritten too, with the loose
    // notes beside them, and the folders are renamed.
    let folder = |id: &str, part: &str| format!("{id}-{part}");
    let beside = repo
        .root
        .parent()
        .filter(|_| repo.root.file_name().is_some_and(|name| name == folder(&old_id, "app").as_str()));
    let markdown = |repo: &Repo| -> Vec<std::path::PathBuf> {
        // All of the notes but the one that says whose the name is: a copy
        // that is renamed is no longer called that, and what that note says
        // of the name it had is still so.
        let files = repo.files().into_iter().filter(|file| file.ends_with(".md") && file != NAME_HELD);
        files.map(|file| repo.root.join(file)).collect()
    };
    let mut prose = markdown(repo);
    if let Some(beside) = beside {
        for part in FOLDERS.iter().skip(1) {
            prose.extend(markdown(&Repo { root: beside.join(folder(&old_id, part)) }));
        }
        let loose = fs::read_dir(beside).into_iter().flatten().flatten().map(|entry| entry.path());
        prose.extend(loose.filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "md")));
    }

    let mut rewritten = 0;
    for path in prose {
        let Ok(before) = fs::read_to_string(&path) else { continue };
        let mut after = before.replace(&old_display, &display);
        // The folder names first: a dash counts as part of a word below, so
        // they would otherwise be left as they were.
        for part in FOLDERS {
            after = replace_word(&after, &folder(&old_id, part), &folder(&id, part));
        }
        after = replace_word(&after, &old_id, &id);
        if cli != old_cli {
            after = replace_word(&after, &old_cli, &cli);
        }
        if after != before {
            fs::write(&path, after).map_err(|err| err.to_string())?;
            rewritten += 1;
        }
    }
    println!(
        "renamed `{old_display}` to `{display}` (id `{id}`, command `{cli}`); {rewritten} doc(s) rewritten"
    );
    if let Some(beside) = beside.filter(|_| id != old_id) {
        // The application's own folder last: this program is running from
        // inside it, which some systems will not allow to be moved.
        for part in FOLDERS.iter().rev() {
            let (from, to) = (beside.join(folder(&old_id, part)), beside.join(folder(&id, part)));
            if !from.is_dir() {
                continue;
            }
            match fs::rename(&from, &to) {
                Ok(()) => println!("moved {} to {}", from.display(), to.display()),
                Err(err) => {
                    println!("could not move {} ({err}); move it to {} by hand", from.display(), to.display())
                }
            }
        }
        println!("the application has moved: `cargo xtask install` again from its new folder");
    }
    println!("rebuild to pick it up, then run `cargo xtask check`");
    Ok(())
}

/// The note that says whose the product's name is, which a rename leaves
/// as it is. Whoever the name is renames the product for good writes that
/// note again by hand.
const NAME_HELD: &str = "TRADEMARKS.md";

/// The folders of the product that carry its id, as `<id>-<part>`. The
/// application's own is first.
const FOLDERS: &[&str] = &["app", "services"];

fn slug(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

fn set_value(text: &str, key: &str, value: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        match line.split_once('=') {
            Some((name, _)) if name.trim() == key => out.push_str(&format!("{key} = \"{value}\"")),
            _ => out.push_str(line),
        }
        out.push('\n');
    }
    out
}

/// Replaces `from` where it stands alone as a word. Dashes and underscores
/// count as part of a word, so a short command name is not rewritten inside a
/// longer identifier that happens to contain it.
fn replace_word(text: &str, from: &str, to: &str) -> String {
    if from.is_empty() || from == to {
        return text.to_string();
    }
    let is_word = |c: char| c.is_alphanumeric() || c == '-' || c == '_';
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(from) {
        let before = rest[..at].chars().next_back();
        let after = rest[at + from.len()..].chars().next();
        out.push_str(&rest[..at]);
        if before.is_some_and(is_word) || after.is_some_and(is_word) {
            out.push_str(from);
        } else {
            out.push_str(to);
        }
        rest = &rest[at + from.len()..];
    }
    out.push_str(rest);
    out
}
