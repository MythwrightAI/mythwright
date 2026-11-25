//! Which of other people's crates the programs can be built from, read from
//! `Cargo.lock` and the manifests as texts.
//!
//! The notices that go with a package (`notices.rs`) are made by a tool
//! that nobody needs who is not adding a dependency. So what holds them to
//! the code has to be asked without it, and without cargo, of files that
//! are in the repository: this says which crates of the lock a program in
//! a package can reach, and a mark of them, which the notices file says at
//! its head. The mark moves when one of those crates is added, taken away
//! or is another version, and when a crate that only the tests used comes
//! to be built into a program. It does not move for a crate only the tests
//! use, or for the product's own version being another.
//!
//! What is reached is more than any one package holds: the lock says what
//! is built for every system there is, and the notices are of the systems
//! a package is made for (`targets` in `about.toml`). So a crate named in
//! the notices is one of these, and not every one of these is named there.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::sha256;

/// The crates that are the programs a package holds (`packing.rs` says
/// what each is called in one): the daemon and command, and the window.
pub const PROGRAMS: [&str; 2] = ["cli", "desktop"];

/// One `[[package]]` of a lock.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Locked {
    pub name: String,
    pub version: String,
    /// Where it was fetched from. The product's own crates say none.
    pub source: Option<String>,
    pub checksum: String,
    /// As the lock writes each: a name, and a version where the lock holds
    /// several of that name.
    pub needs: Vec<String>,
}

fn quoted(value: &str) -> String {
    value.trim().trim_end_matches(',').trim_matches('"').to_string()
}

/// Every package of a lock, as cargo writes one.
pub fn locked(lock: &str) -> Vec<Locked> {
    let mut all: Vec<Locked> = Vec::new();
    let mut needing = false;
    for line in lock.lines().map(str::trim) {
        if line == "[[package]]" {
            all.push(Locked::default());
            needing = false;
            continue;
        }
        let Some(package) = all.last_mut() else { continue };
        if needing {
            if line == "]" {
                needing = false;
            } else {
                package.needs.push(quoted(line));
            }
        } else if let Some((key, value)) = line.split_once(" = ") {
            match key {
                "name" => package.name = quoted(value),
                "version" => package.version = quoted(value),
                "source" => package.source = Some(quoted(value)),
                "checksum" => package.checksum = quoted(value),
                "dependencies" => needing = value.trim() == "[",
                _ => {}
            }
        }
    }
    all
}

/// What a crate of the product's own asks for outside its tests: the names
/// in its manifest under `[dependencies]` and `[build-dependencies]`, and
/// under those of one kind of system (`[target.….dependencies]`).
pub fn asked(manifest: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut inside = false;
    for line in manifest.lines().map(str::trim) {
        if let Some(section) = line.strip_prefix('[') {
            let section = section.trim_end_matches(']');
            inside = section == "dependencies"
                || section == "build-dependencies"
                || (section.starts_with("target.")
                    && (section.ends_with(".dependencies") || section.ends_with(".build-dependencies")));
        } else if inside
            && !line.starts_with('#')
            && let Some((key, _)) = line.split_once('=')
        {
            // `name = { … }`, or `name.workspace = true`.
            let key = key.trim().split('.').next().unwrap_or_default();
            names.insert(key.trim_matches('"').to_string());
        }
    }
    names
}

/// The dependencies the root manifest declares under another name than the
/// crate's own (`short = { package = "its-real-name", … }`), by the name
/// a member writes.
pub fn renamed(root: &str) -> BTreeMap<String, String> {
    let mut names = BTreeMap::new();
    let mut inside = false;
    for line in root.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line == "[workspace.dependencies]";
        } else if inside
            && let Some((key, value)) = line.split_once('=')
            && let Some(at) = value.find("package")
        {
            let real = value[at + "package".len()..].trim_start().trim_start_matches('=');
            let real = real.trim_start().trim_start_matches('"');
            if let Some(end) = real.find('"') {
                names.insert(key.trim().to_string(), real[..end].to_string());
            }
        }
    }
    names
}

/// The crates of somebody else's that the programs can be built from: what
/// the lock says each program needs, and what those need, leaving out what
/// a crate of the product's own asks for only for its tests. `manifests`
/// is each of the product's crates by name with its manifest, and `root`
/// the manifest at the top. In the order of their names and versions.
pub fn reached(lock: &str, root: &str, manifests: &[(String, String)], programs: &[&str]) -> Vec<Locked> {
    let all = locked(lock);
    let renamed = renamed(root);
    let find = |need: &str| {
        let mut parts = need.split(' ');
        let (name, version) = (parts.next().unwrap_or_default(), parts.next());
        all.iter().position(|one| one.name == name && version.is_none_or(|version| one.version == version))
    };
    let mut seen = BTreeSet::new();
    let mut next: Vec<usize> = all
        .iter()
        .enumerate()
        .filter(|(_, one)| programs.contains(&one.name.as_str()))
        .map(|(at, _)| at)
        .collect();
    while let Some(at) = next.pop() {
        if !seen.insert(at) {
            continue;
        }
        let package = &all[at];
        // One of the product's own says in its manifest which of what the
        // lock lists for it are its tests'; anybody else's crate is in the
        // lock with what it is built from and nothing more.
        let own = package
            .source
            .is_none()
            .then(|| manifests.iter().find(|(name, _)| name == &package.name))
            .flatten()
            .map(|(_, manifest)| {
                asked(manifest)
                    .into_iter()
                    .map(|name| renamed.get(&name).cloned().unwrap_or(name))
                    .collect::<BTreeSet<String>>()
            });
        for need in &package.needs {
            let name = need.split(' ').next().unwrap_or_default();
            if own.as_ref().is_some_and(|asked| !asked.contains(name)) {
                continue;
            }
            next.extend(find(need));
        }
    }
    let mut reached: Vec<Locked> =
        seen.into_iter().map(|at| all[at].clone()).filter(|one| one.source.is_some()).collect();
    reached.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    reached
}

/// One figure for a list of crates: it is another when a crate is added,
/// taken away, or is another version or other bytes.
pub fn mark(crates: &[Locked]) -> String {
    let mut lines = String::new();
    for one in crates {
        lines.push_str(&format!("{} {} {}\n", one.name, one.version, one.checksum));
    }
    sha256::hex(lines.as_bytes())
}

/// Each crate of the product's own, by name with its manifest, read from
/// the folders the root manifest lists as `members` (a folder, or every
/// folder in one: `crates/*`).
pub fn manifests(top: &Path, root: &str) -> Vec<(String, String)> {
    // The line that begins so: `default-members` is another list.
    let at =
        root.lines().find(|line| line.trim_start().starts_with("members")).and_then(|line| root.find(line));
    let listed = at.and_then(|at| {
        let rest = &root[at..];
        Some(&rest[rest.find('[')? + 1..rest.find(']')?])
    });
    let mut folders = Vec::new();
    for member in listed.unwrap_or_default().split(',').map(quoted).filter(|member| !member.is_empty()) {
        match member.strip_suffix("/*") {
            Some(parent) => {
                let entries = std::fs::read_dir(top.join(parent)).into_iter().flatten().flatten();
                folders.extend(entries.map(|entry| entry.path()));
            }
            None => folders.push(top.join(member)),
        }
    }
    folders.sort();
    let mut all = Vec::new();
    for folder in folders {
        let Ok(manifest) = std::fs::read_to_string(folder.join("Cargo.toml")) else { continue };
        let mut inside = false;
        let name = manifest.lines().map(str::trim).find_map(|line| {
            if line.starts_with('[') {
                inside = line == "[package]";
            }
            let (key, value) = line.split_once('=')?;
            (inside && key.trim() == "name").then(|| quoted(value))
        });
        all.extend(name.map(|name| (name, manifest)));
    }
    all
}
