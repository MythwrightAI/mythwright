//! The credential of an agent that joined a factory from this machine.
//!
//! An agent a factory does not start is let in by an invitation
//! (`agent join`), and is answered its credential once. It is kept here:
//! a file for the agent's name and the factory's address, under this
//! user's settings and apart from the session of whoever signs in on
//! the machine, so that a person's commands and the agent's never use
//! each other's. A command is asked as that agent by name (`--as`, or
//! the environment's `AGENT`), and is then sent that agent's credential
//! for that address or nothing: one that is not kept is said, and
//! nobody else's is used in its place.
//!
//! The file is its owner's alone from the moment it is there on Unix
//! (mode 0600). On Windows nothing is set on it: it is in the profile
//! of whoever ran the command, which that system keeps from other
//! users, and holds a bearer token all the same.

use std::path::PathBuf;

use serde_json::{Value, json};

use crate::client::config_dir;

/// What an address is called in the name of a file.
fn slug(url: &str) -> String {
    let plain = url.trim_end_matches('/').to_lowercase().replacen("://", "-", 1);
    plain.chars().map(|c| if c.is_ascii_alphanumeric() || c == '.' { c } else { '-' }).collect()
}

/// A name that is an agent's: what a factory makes of any, and so
/// nothing that leads out of the folder the credentials are kept in.
fn named(name: &str) -> Result<&str, String> {
    match !name.is_empty() && blueprint::expr::slug(name) == name {
        true => Ok(name),
        false => Err(format!(
            "`{name}` is not an agent's name: one is lowercase letters, digits and dashes, as `{} agent list` \
             says it",
            brand::CLI
        )),
    }
}

fn folder(name: &str) -> PathBuf {
    config_dir().join("agents").join(name)
}

/// Where the credential of the agent `name` at `url` is kept.
pub fn path(name: &str, url: &str) -> PathBuf {
    folder(name).join(format!("{}.json", slug(url)))
}

/// Keeps `token` as the credential of the agent `name` at `url`,
/// readable by this user only: where it is.
pub fn keep(name: &str, url: &str, token: &str) -> Result<PathBuf, String> {
    let path = path(named(name)?, url);
    let kept = json!({ "agent": name, "url": url.trim_end_matches('/'), "token": token });
    crate::owned::write(&path, kept.to_string().as_bytes())
        .map_err(|err| format!("could not keep the agent's credential in {}: {err}", path.display()))?;
    Ok(path)
}

fn read(path: &std::path::Path) -> Option<(String, String)> {
    let kept: Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    Some((kept["url"].as_str()?.to_string(), kept["token"].as_str()?.to_string()))
}

/// How one joins, said wherever a credential is asked for and not kept.
fn join(url: &str) -> String {
    format!(
        "An agent joins with the invitation made for it (`{0} agent invite <name>`, by whoever may add agents \
         there): `{0} agent join --url {url}`",
        brand::CLI
    )
}

/// The credential kept for the agent `name` at `url`. One that is not
/// kept is said: nothing else is used in its place.
pub fn kept_for(name: &str, url: &str) -> Result<String, String> {
    match read(&path(named(name)?, url)) {
        Some((_, token)) => Ok(token),
        None => Err(format!(
            "no credential is kept on this machine for an agent called `{name}` at {url}, and nothing else was \
             used in its place. {}",
            join(url)
        )),
    }
}

/// The one factory the agent `name` joined from this machine, and its
/// credential there, where no address was said: its address is what
/// was joined. Said, where it joined none or several.
pub fn only(name: &str) -> Result<(String, String), String> {
    // Not one being written (`crate::owned`), which is whole a moment
    // before it is moved over the file it is to be, and would be counted
    // as a second factory joined.
    let found = std::fs::read_dir(folder(named(name)?))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|file| !file.file_name().to_string_lossy().starts_with('.'));
    let mut kept: Vec<(String, String)> = found.filter_map(|file| read(&file.path())).collect();
    kept.sort();
    match kept.len() {
        1 => Ok(kept.remove(0)),
        0 => Err(format!(
            "no credential is kept on this machine for an agent called `{name}`, and nothing else was used in \
             its place. {}",
            join("<the factory's address>")
        )),
        _ => Err(format!(
            "an agent called `{name}` has joined {} factories from this machine, and the name does not say \
             which: {}. Say which by its address: `{} --as {name} --url <address> ...`",
            kept.len(),
            kept.iter().map(|(url, _)| url.as_str()).collect::<Vec<_>>().join(", "),
            brand::CLI
        )),
    }
}

/// Forgets the credential kept for the agent `name` at `url`: whether
/// there was one.
pub fn forget(name: &str, url: &str) -> bool {
    let Ok(name) = named(name) else { return false };
    let gone = std::fs::remove_file(path(name, url)).is_ok();
    // The folder too, once nothing is in it.
    let _ = std::fs::remove_dir(folder(name));
    gone
}
