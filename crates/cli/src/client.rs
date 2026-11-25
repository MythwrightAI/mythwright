//! Talking to the daemon.
//!
//! The client finds the daemon and a credential the same way whether a
//! person or an agent is running it: a flag, then the environment, then the
//! session saved by `login`. An agent's slot sets the environment, with the
//! credential in a file it is pointed at rather than in a variable. An
//! agent a factory does not start, which joined it from this machine,
//! is asked for by its name (`--as`, or the environment's `AGENT`), and
//! its credential is the one kept for it here (`crate::joined`).
//!
//! A daemon is said by its address, or, for a factory that has a
//! profile, by what that is called: `--profile dev1` is the `dev1` factory
//! on this device. Signing in is to one factory, and the session with each
//! is kept apart from the others.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use reqwest::Method;
use serde_json::{Value, json};

use crate::args::{Args, Heed};

pub struct Client {
    pub url: String,
    token: Option<String>,
    /// Print what the daemon said, as JSON, instead of a summary.
    pub json: bool,
    http: reqwest::Client,
    /// The factory that was asked for by its profile's name, when one was.
    named: Option<Named>,
    /// The agent this is asked as, where it is one that joined from this
    /// machine (`crate::joined`).
    pub agent: Option<String>,
    /// Which flags the command asked for: one given and never asked for
    /// is refused before anything is sent.
    heed: Arc<Heed>,
    /// The address of the daemon the session kept here is with, where it
    /// is another than this one's and so was not sent: said where this one
    /// asks for a sign-in, since signing in here puts this one's in its
    /// place.
    kept_with: Option<String>,
}

/// A factory found by what its profile is called.
struct Named {
    name: String,
    /// Its data folder, which is what it is on this device: a session is
    /// kept for that, not for the name, which another factory may be given.
    data: PathBuf,
    /// Whether what answers at its address has been asked what it is.
    asked: AtomicBool,
}

/// Where this user's settings for the product are kept.
pub fn config_dir() -> PathBuf {
    brand::config_dir()
}

/// Where the daemon keeps its data unless told otherwise.
pub fn data_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_default();
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(PathBuf::from));
    base.unwrap_or_else(|| home.join(".local").join("share")).join(brand::ID)
}

/// Where a factory with a profile keeps its data unless told
/// otherwise: beside where one with none does, with the name added, so
/// that starting `dev1` can never land on the data of another.
pub fn data_dir_of(profile: &str) -> PathBuf {
    let unnamed = data_dir();
    let folder = format!("{}-{profile}", brand::ID);
    unnamed.parent().map_or_else(|| PathBuf::from(&folder), |beside| beside.join(&folder))
}

/// Where the session with a factory is kept: one file for the daemon found
/// by its address, as there always was, and one for each factory found by
/// name, so that signing in to `dev1` leaves every other session alone.
fn session_path(profile: Option<&str>) -> PathBuf {
    match profile {
        Some(name) => config_dir().join("sessions").join(format!("{name}.json")),
        None => config_dir().join("session.json"),
    }
}

fn saved_session(profile: Option<&str>) -> Value {
    std::fs::read_to_string(session_path(profile))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(Value::Null)
}

fn env(name: &str) -> Option<String> {
    std::env::var(brand::env_var(name)).ok().filter(|value| !value.is_empty())
}

/// Keeps the session of the owner a daemon made as it first started, for
/// the command line of whoever started it (`serve`): where this home keeps
/// no session for that factory's place, or one with that very factory.
/// One kept with another daemon is never put out by it; what to do
/// instead is said.
pub fn keep_made(url: &str, profile: Option<&str>, data: &Path, token: &str) -> Result<(), String> {
    let saved = saved_session(profile);
    let theirs = match profile {
        Some(_) => saved["data"].as_str().is_some_and(|kept| Path::new(kept) != data),
        None => saved["url"].as_str().is_some_and(|kept| kept.trim_end_matches('/') != url),
    };
    if theirs && saved["token"].is_string() {
        return Err(format!(
            "this terminal is signed in to another factory, and is left so: `{} open` opens this one \
             signed in all the same, and `login --here` signs this terminal in to it",
            brand::CLI
        ));
    }
    let session = match profile {
        Some(_) => json!({ "url": url, "token": token, "data": data }),
        None => json!({ "url": url, "token": token }),
    };
    let path = session_path(profile);
    crate::owned::write(&path, session.to_string().as_bytes())
        .map_err(|err| format!("could not keep the session in {}: {err}", path.display()))
}

/// The factory called `name` on this device, from the list each writes
/// itself into as it starts.
fn find(name: &str) -> Result<api::registry::Factory, String> {
    let cli = brand::CLI;
    api::profile::check_name(name).map_err(|err| err.message)?;
    let mut found = api::registry::Registry::of_this_user().named(name).map_err(|err| err.message)?;
    match found.len() {
        0 => Err(format!(
            "there is no `{name}` factory on this device. `{cli} env` lists the ones there are, and \
             `{cli} up --profile {name}` starts one"
        )),
        1 => Ok(found.remove(0)),
        count => {
            let lines: Vec<Vec<String>> = found
                .iter()
                .map(|factory| {
                    vec![
                        format!("  {}", url_of(factory.address)),
                        format!("data in {}", factory.data.display()),
                    ]
                })
                .collect();
            Err(format!(
                "there are {count} `{name}` factories on this device, and the name does not say which:\n{}\n\
                 Say which by its address: `{cli} --url <address> ...`",
                table(&lines)
            ))
        }
    }
}

/// Where a factory that listens on `address` is reached from this device.
pub fn url_of(address: std::net::SocketAddr) -> String {
    let mut address = address;
    if address.ip().is_unspecified() {
        let own: std::net::IpAddr = match address.ip() {
            std::net::IpAddr::V4(_) => std::net::Ipv4Addr::LOCALHOST.into(),
            std::net::IpAddr::V6(_) => std::net::Ipv6Addr::LOCALHOST.into(),
        };
        address.set_ip(own);
    }
    format!("http://{address}")
}

impl Client {
    /// Finds the daemon to talk to and who to talk to it as.
    ///
    /// By address, as always: `--url`, the environment, the saved session,
    /// the usual place. Or by what its profile is called (`--profile`),
    /// which is looked up among the factories on this device. A flag is
    /// taken over the environment's variables, and of two variables the
    /// address, which is the more exact: it is what an agent's slot is given.
    pub fn new(args: &Args) -> Result<Client, String> {
        let asked = match (args.value("url"), crate::profiled::flagged(args)) {
            (Some(_), Some(_)) => {
                return Err(
                    "`--url` says which daemon by its address and `--profile` by its name: give one".into()
                );
            }
            (None, Some(_)) => crate::profiled::asked(args),
            (Some(_), None) => None,
            (None, None) if env("URL").is_none() => crate::profiled::asked(args),
            (None, None) => None,
        };
        let flag_token = args.value("token").map(str::to_string);
        // Whether an address was said, by a flag or by the environment.
        let said_where = asked.is_some() || args.value("url").is_some() || env("URL").is_some();
        // Asked as an agent that joined from this machine: by a flag,
        // or by the environment where it says no credential itself (an
        // agent a factory starts is given a file, and that is its own).
        // Joining is asked as nobody: the invitation is what lets it in.
        let joining = matches!(args.word(0), Some("agent" | "agents")) && args.word(1) == Some("join");
        let as_agent = match (args.value("as"), &flag_token) {
            _ if joining => None,
            (Some(_), Some(_)) => {
                return Err(
                    "`--token` is a credential and `--as` names an agent whose credential is kept here: give one"
                        .into(),
                );
            }
            (Some(name), None) => Some(name.to_string()),
            (None, Some(_)) => None,
            (None, None) => env("AGENT").filter(|_| env("TOKEN_FILE").is_none() && env("TOKEN").is_none()),
        };
        // A credential said to be in a file is that file's and no other.
        // One that cannot be read is said: going on to whoever signed in
        // on this machine would have an agent whose own credential was
        // withdrawn act as its owner, and nobody told.
        let filed = match env("TOKEN_FILE").filter(|_| flag_token.is_none() && as_agent.is_none()) {
            Some(path) => match std::fs::read_to_string(&path) {
                Ok(token) => Some(token.trim().to_string()),
                Err(err) => {
                    return Err(format!(
                        "the credential this was given (`{}`, the file {path}) cannot be read: {err}. \
                         Nothing else was used in its place",
                        brand::env_var("TOKEN_FILE")
                    ));
                }
            },
            None => None,
        };
        let given_token = || filed.clone().or_else(|| env("TOKEN"));
        let mut kept_with = None;
        let (url, token, named) = match asked {
            Some(name) => {
                let factory = find(&name)?;
                let url = url_of(factory.address);
                let saved = saved_session(Some(&name));
                // A credential in the environment came with an address, if
                // one is there: it is that daemon's, and goes to no other.
                let theirs = env("URL").is_some_and(|other| other.trim_end_matches('/') != url);
                let token = flag_token.or_else(|| given_token().filter(|_| !theirs)).or_else(|| {
                    // Kept for a factory, not for a name: another factory
                    // that is given the name is not given the session.
                    let same = saved["data"].as_str().is_some_and(|data| Path::new(data) == factory.data);
                    saved["token"].as_str().filter(|_| same).map(str::to_string)
                });
                (url, token, Some(Named { name, data: factory.data, asked: AtomicBool::new(false) }))
            }
            None => {
                let saved = saved_session(None);
                let url = args
                    .value("url")
                    .map(str::to_string)
                    .or_else(|| env("URL"))
                    .or_else(|| saved["url"].as_str().map(str::to_string))
                    .unwrap_or_else(|| brand::DEFAULT_URL.to_string());
                // The saved session is with the daemon it was saved for. One
                // that is said by another address may be another factory,
                // and is not handed it.
                let same = |one: &str, other: &str| one.trim_end_matches('/') == other.trim_end_matches('/');
                let kept = saved["url"].as_str().is_some_and(|saved| same(saved, &url));
                let token = flag_token
                    .or_else(given_token)
                    .or_else(|| saved["token"].as_str().filter(|_| kept).map(str::to_string));
                let elsewhere = saved["url"].as_str().filter(|_| !kept && saved["token"].is_string());
                kept_with = elsewhere.map(|other| other.trim_end_matches('/').to_string());
                (url, token, None)
            }
        };
        let url = url.trim_end_matches('/').to_string();
        // An agent's credential is the one kept for it at that address,
        // or where none was said at the one address it joined; never
        // the session of whoever signed in on this machine.
        let (url, token) = match (&as_agent, joining) {
            (_, true) => (url, None),
            (Some(name), false) if said_where => {
                let token = crate::joined::kept_for(name, &url)?;
                (url, Some(token))
            }
            (Some(name), false) => {
                let (url, token) = crate::joined::only(name)?;
                (url, Some(token))
            }
            (None, false) => (url, token),
        };
        Ok(Client {
            url,
            token,
            json: args.has("json"),
            http: reqwest::Client::new(),
            named,
            agent: as_agent,
            heed: args.heed(),
            kept_with,
        })
    }

    /// Remembers who is signed in, readable by this user only.
    pub fn save_session(&self, token: &str) -> Result<(), String> {
        let name = self.named.as_ref().map(|named| named.name.as_str());
        let path = session_path(name);
        let session = match &self.named {
            Some(named) => json!({ "url": self.url, "token": token, "data": named.data }),
            None => json!({ "url": self.url, "token": token }),
        };
        // Whole, or a command reading it in the moment it is written
        // finds no session (`crate::owned`).
        crate::owned::write(&path, session.to_string().as_bytes())
            .map_err(|err| format!("could not save the session to {}: {err}", path.display()))
    }

    pub fn forget_session(&self) {
        let _ = std::fs::remove_file(session_path(self.named.as_ref().map(|named| named.name.as_str())));
    }

    /// What to run to start the daemon this is looking for.
    fn start_with(&self) -> String {
        match &self.named {
            Some(named) => format!("{} up --profile {}", brand::CLI, named.name),
            None => format!("{} up", brand::CLI),
        }
    }

    /// Makes sure that what answers at the address of a factory found by
    /// name is that factory, before anything is sent to it; and that the
    /// command asked for every flag it was given (`Heed::check`).
    ///
    /// The address is where the factory last listened. If it has stopped,
    /// something else may be listening there by now, and a credential must
    /// not be handed to whatever that is.
    async fn sure(&self) -> Result<(), String> {
        self.heed.check()?;
        let Some(named) = &self.named else { return Ok(()) };
        if named.asked.load(Ordering::Relaxed) {
            return Ok(());
        }
        let nothing = || {
            format!(
                "nothing is answering at {}, where the `{}` factory last listened; start it with `{}`",
                self.url,
                named.name,
                self.start_with()
            )
        };
        let response = self.http.get(format!("{}/api/meta", self.url)).send().await.map_err(|_| nothing())?;
        let meta: Value = response.json().await.unwrap_or(Value::Null);
        // `environment` is what a program before 0.1.0 answers it as.
        let called = meta["profile"].as_str().or(meta["environment"].as_str());
        if called != Some(&named.name) {
            let says = match called {
                Some(other) => format!("a `{other}` factory"),
                None if meta["id"] == brand::ID => "a factory with no profile".to_string(),
                None => "something else".to_string(),
            };
            return Err(format!(
                "the `{name}` factory last listened at {url}, and what answers there now is {says}. Nothing \
                 was sent to it. Start `{name}` again with `{start}`; if its address has been taken, give it \
                 another once with `--listen`",
                name = named.name,
                url = self.url,
                start = self.start_with(),
            ));
        }
        named.asked.store(true, Ordering::Relaxed);
        Ok(())
    }

    /// Asks as whoever `token` is from here on, or as nobody, for this
    /// command alone: nothing is kept by it.
    pub fn ask_as(&mut self, token: Option<&str>) {
        self.token = token.map(str::to_string);
    }

    pub fn signed_in(&self) -> bool {
        self.token.is_some()
    }

    pub async fn call(&self, method: Method, path: &str, body: Option<Value>) -> Result<Value, String> {
        let mut request = self.http.request(method, format!("{}{path}", self.url));
        if let Some(body) = body {
            request = request.json(&body);
        }
        self.answer(request).await
    }

    /// Sends a file as it is, for what takes one: something to attach.
    pub async fn upload(&self, path: &str, bytes: Vec<u8>) -> Result<Value, String> {
        self.answer(self.http.post(format!("{}{path}", self.url)).body(bytes)).await
    }

    /// Writes a file as it is, whatever is in it: a project's file, byte for byte.
    pub async fn put_bytes(&self, path: &str, bytes: Vec<u8>) -> Result<Value, String> {
        let request = self.http.put(format!("{}{path}", self.url));
        self.answer(request.header(reqwest::header::CONTENT_TYPE, "application/octet-stream").body(bytes))
            .await
    }

    /// What is answered as it is, whatever is in it: a project's file, byte for byte.
    pub async fn bytes(&self, path: &str) -> Result<Vec<u8>, String> {
        let response = self.sent(self.http.get(format!("{}{path}", self.url))).await?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|err| format!("reading the answer failed: {err}"))?;
        if status.is_success() {
            return Ok(bytes.to_vec());
        }
        let text = String::from_utf8_lossy(&bytes).into_owned();
        Err(self.refused(status, serde_json::from_str(&text).unwrap_or(Value::String(text))).await)
    }

    async fn sent(&self, mut request: reqwest::RequestBuilder) -> Result<reqwest::Response, String> {
        self.sure().await?;
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        request.send().await.map_err(|err| {
            if err.is_connect() {
                format!("nothing is answering at {}; start the daemon with `{}`", self.url, self.start_with())
            } else {
                format!("the request to {} failed: {err}", self.url)
            }
        })
    }

    async fn answer(&self, request: reqwest::RequestBuilder) -> Result<Value, String> {
        let response = self.sent(request).await?;
        let status = response.status();
        let text = response.text().await.map_err(|err| format!("reading the answer failed: {err}"))?;
        let body: Value = serde_json::from_str(&text).unwrap_or(Value::String(text));
        if status.is_success() {
            return Ok(body);
        }
        Err(self.refused(status, body).await)
    }

    /// What is said of an answer that refused, from what the daemon said.
    async fn refused(&self, status: reqwest::StatusCode, body: Value) -> String {
        let message = body["error"]["message"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| format!("the daemon answered {status}"));
        match status.as_u16() {
            // Asked as an agent that joined from here, and not taken:
            // said for what it is, and never as somebody to sign in.
            401 if self.agent.is_some() && self.token.is_some() => {
                let name = self.agent.as_deref().unwrap_or_default();
                format!(
                    "the credential kept on this machine for the agent `{name}` is not taken at {url} any more: \
                     the agent was stopped or taken away, or it joined again from somewhere else. An invitation \
                     lets it in again (`{cli} agent invite {name}` by whoever may add agents there, then \
                     `{cli} agent join --url {url}` here)",
                    url = self.url,
                    cli = brand::CLI
                )
            }
            401 if !self.signed_in() => {
                let to = match &self.named {
                    Some(named) => format!("{} --profile {}", brand::CLI, named.name),
                    None => brand::CLI.to_string(),
                };
                // Where nobody is registered there is no account to sign in
                // to: sent to `login`, somebody on a device they had just
                // started was told their email and password matched none.
                if self.has_no_owner().await {
                    format!("{message} (`{to} register`: nobody is registered here yet)")
                } else if let Some(other) = &self.kept_with {
                    // Said, or a session that worked a moment ago reads as
                    // one that was lost: it was put in place of this one's
                    // by signing in to another daemon from this home.
                    format!(
                        "{message}: the session kept on this machine is with {other} and not {url}, since \
                         signing in to one daemon by its address puts its session in place of the other's \
                         (`{to} login --url {url}`)",
                        url = self.url
                    )
                } else {
                    format!("{message} (`{to} login`)")
                }
            }
            _ => message,
        }
    }

    /// Whether the daemon says nobody is registered at it yet. It is asked
    /// only once something was refused for want of a sign-in, with nothing
    /// sent beside the question; a daemon that does not say is taken to
    /// have an owner.
    async fn has_no_owner(&self) -> bool {
        let Ok(answered) = self.http.get(format!("{}/api/meta", self.url)).send().await else { return false };
        answered.json::<Value>().await.is_ok_and(|meta| meta["needs_setup"] == true)
    }

    pub async fn get(&self, path: &str) -> Result<Value, String> {
        self.call(Method::GET, path, None).await
    }

    pub async fn post(&self, path: &str, body: Value) -> Result<Value, String> {
        self.call(Method::POST, path, Some(body)).await
    }

    /// Opens a long-lived response, such as the event stream.
    pub async fn stream(&self, path: &str) -> Result<reqwest::Response, String> {
        self.stream_or_gone(path).await.map_err(Unanswered::said)
    }

    /// The same, told apart from the daemon being gone, for whoever goes
    /// on asking until it is back.
    pub async fn stream_or_gone(&self, path: &str) -> Result<reqwest::Response, Unanswered> {
        self.reached(path).await
    }

    /// What is read where whoever asks goes on asking until the daemon is
    /// back (`inbox --follow`): told apart from the daemon being gone.
    pub async fn get_or_gone(&self, path: &str) -> Result<Value, Unanswered> {
        let response = self.reached(path).await?;
        response.json().await.map_err(|err| Unanswered::Gone(format!("reading the answer failed: {err}")))
    }

    /// A request answered well, or why not: the daemon not there (nothing
    /// answers, the connection broke, or what stands in front of it says
    /// it is not there), or there and refusing.
    async fn reached(&self, path: &str) -> Result<reqwest::Response, Unanswered> {
        let response = self.sent(self.http.get(format!("{}{path}", self.url))).await.map_err(|err| {
            match Client::is_unreachable(&err) || err.starts_with("the request to ") {
                true => Unanswered::Gone(err),
                false => Unanswered::Refused(err),
            }
        })?;
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        let text = response.text().await.unwrap_or_default();
        let said = self.refused(status, serde_json::from_str(&text).unwrap_or(Value::String(text))).await;
        match status.as_u16() {
            502..=504 => Err(Unanswered::Gone(said)),
            _ => Err(Unanswered::Refused(said)),
        }
    }

    /// Whether the daemon could not be reached at all, as opposed to
    /// answering with an error.
    pub fn is_unreachable(error: &str) -> bool {
        error.starts_with("nothing is answering")
    }
}

/// Why a request was not answered well, for whoever goes on asking until
/// the daemon is back: it was not there, or it refused.
pub enum Unanswered {
    Gone(String),
    Refused(String),
}

impl Unanswered {
    pub fn said(self) -> String {
        match self {
            Unanswered::Gone(said) | Unanswered::Refused(said) => said,
        }
    }
}

/// Percent-encodes a value for a query string.
pub fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Lines of text in columns, each as wide as its widest cell.
pub fn table(rows: &[Vec<String>]) -> String {
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    let widths: Vec<usize> = (0..columns)
        .map(|column| {
            rows.iter().filter_map(|row| row.get(column)).map(|cell| cell.chars().count()).max().unwrap_or(0)
        })
        .collect();
    rows.iter()
        .map(|row| {
            let cells: Vec<String> = row
                .iter()
                .enumerate()
                .map(|(column, cell)| {
                    // The last column is not padded, so lines have no trailing spaces.
                    if column + 1 == row.len() {
                        cell.clone()
                    } else {
                        format!("{cell:<width$}", width = widths[column])
                    }
                })
                .collect();
            cells.join("  ").trim_end().to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A duration in milliseconds, in the largest units that keep it short.
pub fn took(ms: i64) -> String {
    match ms {
        ..1_000 => format!("{ms}ms"),
        1_000..60_000 => format!("{:.1}s", ms as f64 / 1000.0),
        _ => blueprint::duration::format((ms as u64 / 1000) * 1000),
    }
}

/// A number with its thousands told apart: `1,204,000`.
pub fn grouped(whole: u64) -> String {
    let digits = whole.to_string();
    let mut said = String::new();
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(3) {
            said.push(',');
        }
        said.push(digit);
    }
    said
}

/// An amount of something that is not a length of time, as the daemon
/// says what a measurement is of (`unit`): money with its sign, anything
/// else as a number, whole once it is large enough for the rest not to
/// matter.
pub fn amount(value: f64, unit: &str) -> String {
    let number = match value.abs() {
        size if size >= 100.0 => grouped(value.round() as u64),
        size if size >= 1.0 => format!("{value:.2}"),
        _ => {
            let small = format!("{value:.4}");
            let small = small.trim_end_matches('0');
            // Never less than two places: `0.50`, not `0.5`.
            let places = small.split_once('.').map_or(0, |(_, places)| places.len());
            format!("{small}{}", "0".repeat(2usize.saturating_sub(places)))
        }
    };
    match unit {
        "usd" => format!("${number}"),
        _ => number,
    }
}

/// A timestamp as a short "how long ago".
pub fn ago(now_ms: i64, then_ms: i64) -> String {
    let seconds = ((now_ms - then_ms) / 1000).max(0);
    match seconds {
        0..60 => format!("{seconds}s ago"),
        60..3600 => format!("{}m ago", seconds / 60),
        3600..86_400 => format!("{}h ago", seconds / 3600),
        _ => format!("{}d ago", seconds / 86_400),
    }
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as i64)
}
