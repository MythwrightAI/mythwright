//! The product's name.
//!
//! Every user-visible spelling of the product comes from here, so that a
//! rename is one edit to `brand.toml`. Code never writes the name as a
//! literal; `cargo xtask check` refuses a tree in which it does.

include!(concat!(env!("OUT_DIR"), "/brand.rs"));

/// The version of this build: what a blueprint written for a later one is
/// told it is being read by.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Name of an environment variable in the product's namespace, e.g.
/// `env_var("URL")` is `<PREFIX>_URL`.
pub fn env_var(name: &str) -> String {
    format!("{ENV_PREFIX}_{name}")
}

/// What may follow a blueprint's ending in its file's name. A blueprint is
/// a strict subset of YAML, and a file called `fleet.bp.yml` is one every
/// editor colours and checks as that with nobody having told it what a
/// blueprint is. Never one of these alone: a project is full of YAML that
/// is not a blueprint, and a file is ours only by the ending before it.
const AS_YAML: [&str; 2] = ["yml", "yaml"];

/// Every ending a file may have, without its first dot, to be a blueprint
/// whose usual ending is `ending`: the usual first.
pub fn endings_of(ending: &str) -> Vec<String> {
    let mut all = vec![ending.to_string()];
    all.extend(AS_YAML.iter().map(|yaml| format!("{ending}.{yaml}")));
    all
}

/// Every ending a blueprint's file may have, the usual first.
pub fn blueprint_endings() -> Vec<String> {
    endings_of(BLUEPRINT_EXT)
}

/// A blueprint file name for `stem`, as one is usually called: `fleet`
/// becomes `fleet.bp`.
pub fn blueprint_file(stem: &str) -> String {
    format!("{stem}.{BLUEPRINT_EXT}")
}

/// Every name the file of the blueprint `stem` may have, the usual first.
pub fn blueprint_files(stem: &str) -> Vec<String> {
    blueprint_endings().iter().map(|ending| format!("{stem}.{ending}")).collect()
}

/// What `path` is without `ending` and the dot before it, where that leaves
/// a file something to be called.
pub fn without_ending<'a>(path: &'a str, ending: &str) -> Option<&'a str> {
    let stem = path.strip_suffix(ending)?.strip_suffix('.')?;
    (!stem.is_empty() && !stem.ends_with(['/', '\\'])).then_some(stem)
}

/// What a blueprint is called by its file: `fleet` of `fleet.bp` and of
/// `fleet.bp.yml`, and nothing of a file that is not a blueprint's.
pub fn blueprint_stem(path: &str) -> Option<&str> {
    // The longest first, so that `fleet.bp.yml` is `fleet`.
    blueprint_endings().iter().rev().find_map(|ending| without_ending(path, ending))
}

/// Whether `path` names a blueprint file.
pub fn is_blueprint_path(path: &str) -> bool {
    blueprint_stem(path).is_some()
}

/// Whether `path` names a blueprint by one of the names that say it is
/// YAML (`fleet.bp.yml`): the names an editor reads a file as YAML by,
/// with nobody having told it what a blueprint is.
pub fn is_blueprint_as_yaml(path: &str) -> bool {
    blueprint_endings().iter().skip(1).any(|ending| without_ending(path, ending).is_some())
}

/// Where the daemon listens unless told otherwise, and so where every
/// client looks for it first.
pub const DEFAULT_URL: &str = "http://127.0.0.1:7700";

/// Where this user's settings for the product are kept: the saved session,
/// which every client on the machine (command line, desktop app) shares.
pub fn config_dir() -> std::path::PathBuf {
    use std::path::PathBuf;
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_default();
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from));
    base.unwrap_or_else(|| home.join(".config")).join(ID)
}
