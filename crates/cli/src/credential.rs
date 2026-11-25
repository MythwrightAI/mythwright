//! What git is answered when it asks who a remote is reached as.
//!
//! A branch can be sent to a remote as a key the factory keeps, a bot's,
//! in place of whoever runs the daemon (`key` of `std/fleet`). The step
//! that runs git is handed the key in a variable of its environment, tells
//! git that the helper it asks for a credential is this program, and says
//! which remote the key is for. Git asks; this answers with the key, when
//! what git is about to speak to is that remote and nothing else.
//!
//! So the key is in the environment of git and of this program for as long
//! as the step runs, and goes from here to git down a pipe. It is in no
//! argument, no file and no line either of them prints. A remote that
//! sends git on to somewhere else is not given it: the address a key is
//! for is the one the repository itself has for the remote.
//!
//! Nothing is kept or forgotten here: `store` and `erase`, which git says
//! after it has used a credential, do nothing.

use std::io::Read;
use std::process::ExitCode;

// The variable the key is handed in, and the one that says which remote it
// is for: said where git is asked, so that both ends go by the same.
use kernel::git::{KEY, REMOTE};

use crate::args::Args;

/// The user the key is given under where the remote's address names none.
/// A service that takes a key takes it under any name, or says which it
/// wants, and that one is written into the address.
const USER: &str = "token";

pub fn credential(args: &Args) -> ExitCode {
    if args.word(1) != Some("get") {
        return ExitCode::SUCCESS;
    }
    // A flag nobody reads is refused before anything is done (`main` says it).
    if args.heed().check().is_err() {
        return ExitCode::from(2);
    }
    let mut asked = String::new();
    if std::io::stdin().read_to_string(&mut asked).is_err() {
        return ExitCode::SUCCESS;
    }
    let said = |name: &str| std::env::var(name).ok().filter(|value| !value.trim().is_empty());
    match answer(&asked, said(KEY).as_deref(), said(REMOTE).as_deref()) {
        Ok(answer) => print!("{answer}"),
        Err(why) => {
            eprintln!("{why}");
            // Nobody else is asked, and nobody is prompted.
            println!("quit=true");
        }
    }
    ExitCode::SUCCESS
}

/// What git is told, for what it `asked`: the key and who it is given as,
/// nothing where no key was handed, or why the key is not given.
fn answer(asked: &str, key: Option<&str>, remote: Option<&str>) -> Result<String, String> {
    let Some(key) = key.map(str::trim) else { return Ok(String::new()) };
    let of = |name: &str| {
        asked
            .lines()
            .filter_map(|line| line.split_once('='))
            .find(|(said, _)| *said == name)
            .map(|(_, value)| value.trim())
            .filter(|value| !value.is_empty())
    };
    let (protocol, host) = (of("protocol").unwrap_or_default(), of("host").unwrap_or_default());
    let Some(place) = remote.and_then(Place::of) else {
        return Err(format!(
            "A key was handed for a remote, and which remote was not said in a way that is understood ({}), \
             so it was not given to {protocol}://{host}.",
            remote.unwrap_or("nothing was said")
        ));
    };
    if !place.is(protocol, host) {
        return Err(format!(
            "The key is for {}://{}, which is where the repository says its remote is, and it was asked for \
             on the way to {protocol}://{host}. It was not given. If the remote has moved, give the \
             repository its new address.",
            place.protocol, place.host
        ));
    }
    if key.contains(['\n', '\r']) {
        return Err("The key that was handed for the remote is more than one line, which no key is.".into());
    }
    let user = of("username").or(place.user.as_deref()).unwrap_or(USER);
    Ok(format!("username={user}\npassword={key}\n"))
}

/// Where a remote is, as far as a credential goes: how it is spoken to,
/// the host (with its port, where it says one), and who as, if it says.
struct Place {
    protocol: String,
    host: String,
    user: Option<String>,
}

impl Place {
    /// Read from a remote's address. One that is not `protocol://host/...`
    /// (a folder, or `user@host:path`, which is reached another way and
    /// asks for no such credential) is nowhere a key is given.
    fn of(address: &str) -> Option<Self> {
        let (protocol, rest) = address.trim().split_once("://")?;
        let authority = rest.split(['/', '?', '#']).next()?;
        let (user, host) = match authority.rsplit_once('@') {
            Some((who, host)) => (who.split(':').next().filter(|user| !user.is_empty()), host),
            None => (None, authority),
        };
        if protocol.is_empty() || host.is_empty() {
            return None;
        }
        Some(Self {
            protocol: protocol.to_ascii_lowercase(),
            host: host.to_ascii_lowercase(),
            user: user.map(str::to_string),
        })
    }

    fn is(&self, protocol: &str, host: &str) -> bool {
        self.protocol == protocol.to_ascii_lowercase() && self.host == host.to_ascii_lowercase()
    }
}
