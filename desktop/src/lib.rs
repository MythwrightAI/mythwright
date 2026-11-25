//! The desktop app's part that is not a window: finding the daemon, and
//! starting it when it is not running.
//!
//! The app is a thin shell. Everything it shows is the daemon's own client,
//! the same pages a browser gets; the app adds a window, and the convenience
//! of not having to start anything first.
//!
//! The window has no title bar of the system's. The client draws the top of
//! the window itself, as part of the page, and asks the window to do what a
//! title bar does ([`Ask`]): move, resize, minimise, maximise, close. The
//! app tells the page how it is framed as the page loads ([`page_setup`]).

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The window's own icon: the pixels of `desktop/icon/icon-window.rgba`,
/// four bytes each (red, green, blue, how solid), a row at a time. Built
/// in, so that the program shows it with no file beside it.
/// `desktop/icon/make.mjs` writes that file from the drawing.
pub const ICON: &[u8] = include_bytes!("../icon/icon-window.rgba");
/// How many pixels each side of [`ICON`] is.
pub const ICON_SIDE: u32 = 128;

/// What the window was asked for on its command line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Asked {
    /// To be told what the window takes, and nothing else.
    pub help: bool,
    /// The daemon's address, where one was said (`--url`).
    pub url: Option<String>,
    /// The system's own title bar (`--system-frame`).
    pub system_frame: bool,
    /// The window opens onto a daemon that is answering already, and starts
    /// none (`--no-start`). An address on this machine may be another's: a
    /// daemon in WSL is at `localhost` from Windows, and a daemon started
    /// in its place on Windows would be another factory.
    pub no_start: bool,
}

impl Asked {
    /// Reads the window's arguments. One it does not take is refused, in a
    /// sentence: a word misspelt would otherwise be passed over, and a
    /// window told `--no-strat` would start a daemon.
    pub fn read(args: &[String]) -> Result<Asked, String> {
        if args.iter().any(|arg| arg == "--help" || arg == "-h") {
            return Ok(Asked { help: true, ..Asked::default() });
        }
        let mut asked = Asked::default();
        let mut args = args.iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--url" => match args.next().filter(|url| !url.starts_with("--") && !url.trim().is_empty()) {
                    Some(url) => asked.url = Some(url.clone()),
                    None => {
                        return Err(format!(
                            "`--url` needs the daemon's address after it, such as `--url {}`.",
                            brand::DEFAULT_URL
                        ));
                    }
                },
                "--system-frame" => asked.system_frame = true,
                "--no-start" => asked.no_start = true,
                other => {
                    return Err(format!(
                        "`{other}` is not something the window takes; `--help` says what it does."
                    ));
                }
            }
        }
        Ok(asked)
    }
}

/// What is said where the window was told to start nothing and no daemon
/// answers at its address.
pub fn unanswered(url: &str) -> String {
    format!(
        "Nothing is answering at {url} as {name}'s daemon, and the window was told to start none (`--no-start`), so it opens nothing. Start the daemon that is to be there, then open the window again.",
        name = brand::DISPLAY
    )
}

/// The daemon's address: what was asked for, else the environment's, else
/// the one this user last signed in to, else the usual one on this machine.
pub fn address(flag: Option<&str>, env: Option<&str>, saved: Option<&str>) -> String {
    [flag, env, saved]
        .into_iter()
        .flatten()
        .map(|url| url.trim().trim_end_matches('/'))
        .find(|url| !url.is_empty())
        .unwrap_or(brand::DEFAULT_URL)
        .to_string()
}

/// The address the user last signed in to from this machine, if any.
pub fn saved_address(config_dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(config_dir.join("session.json")).ok()?;
    let session: serde_json::Value = serde_json::from_str(&text).ok()?;
    session["url"].as_str().map(str::to_string)
}

/// `host:port` of an `http://` address.
fn authority(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("http://")?;
    let authority = rest.split('/').next()?;
    (!authority.is_empty()).then_some(authority)
}

/// Whether the product's daemon is answering at `url`. Something else
/// listening on the port is not an answer.
pub fn answers(url: &str) -> bool {
    let Some(authority) = authority(url) else { return false };
    let host_port = if authority.contains(':') { authority.to_string() } else { format!("{authority}:80") };
    // Each address the name comes to, in turn: `localhost` may come to
    // `::1` before `127.0.0.1`, and a daemon listens at one of them.
    let Ok(addrs) = host_port.to_socket_addrs() else { return false };
    let Some(mut stream) =
        addrs.into_iter().find_map(|addr| TcpStream::connect_timeout(&addr, Duration::from_millis(700)).ok())
    else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let request = format!("GET /api/meta HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n\r\n");
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }
    let mut response = String::new();
    let _ = stream.take(16 * 1024).read_to_string(&mut response);
    response.starts_with("HTTP/1.1 200") && response.contains(&format!("\"id\":\"{}\"", brand::ID))
}

/// Whether `url` is on this machine, where the app may start a daemon.
pub fn is_local(url: &str) -> bool {
    let host = authority(url).map(|authority| authority.rsplit_once(':').map_or(authority, |(host, _)| host));
    matches!(host, Some("127.0.0.1" | "localhost" | "[::1]"))
}

/// Where the window is being asked to go, as far as the window is
/// concerned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Going {
    /// A page of the daemon the window was opened for, or something a page
    /// of it made for itself: it goes there.
    Here,
    /// A page somewhere else on the web: not in this window, which holds
    /// the session of whoever is signed in. The system's browser is where
    /// such a page belongs.
    Outside,
    /// Anything else: nowhere.
    Nowhere,
}

/// Judges where the window is asked to go, given the daemon it was opened
/// for (`http://127.0.0.1:7700`).
///
/// The window is the daemon's client and nothing else. Left to itself a web
/// view goes wherever a link or a script sends it, and whatever page it
/// lands on is then inside the window that can move, resize and close
/// itself, beside a session that is still signed in.
pub fn going(daemon: &str, to: &str) -> Going {
    let own = |url: &str| match (authority(daemon), authority(url)) {
        (Some(daemon), Some(to)) => {
            daemon.eq_ignore_ascii_case(to)
                && url.split("://").next().unwrap_or_default().eq_ignore_ascii_case("http")
        }
        _ => false,
    };
    let to = to.trim();
    // What a page makes for itself: an empty frame, a document it was
    // handed as text, a file it built in memory, which is its own only
    // when it says whose it is.
    if to.starts_with("about:") || to.starts_with("data:") {
        return Going::Here;
    }
    if let Some(inner) = to.strip_prefix("blob:") {
        return if own(inner) { Going::Here } else { Going::Nowhere };
    }
    if own(to) {
        Going::Here
    } else if to.starts_with("http://") || to.starts_with("https://") {
        Going::Outside
    } else {
        Going::Nowhere
    }
}

/// Hands a page that is not the daemon's to the system's browser. Only an
/// address of the web is handed over, as one argument and to no shell.
pub fn open_outside(url: &str) {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return;
    }
    let (program, args): (&str, &[&str]) = if cfg!(target_os = "macos") {
        ("open", &[])
    } else if cfg!(windows) {
        ("rundll32", &["url.dll,FileProtocolHandler"])
    } else {
        ("xdg-open", &[])
    };
    let _ = std::process::Command::new(program)
        .args(args)
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

/// The daemon's program: beside this one (as it is installed, or as it is
/// built), else whatever the command is on the PATH.
pub fn daemon_program(beside: Option<&Path>) -> PathBuf {
    let names = [brand::CLI, "factory"];
    beside
        .into_iter()
        .flat_map(|dir| {
            names.iter().map(move |name| dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX)))
        })
        .find(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from(brand::CLI))
}

/// Makes sure a daemon is answering at `url`, starting one if it is on this
/// machine and not running. The daemon is started to outlive the app: it is
/// what runs the fleet, and closing a window must not stop that.
///
/// Answers whether it had to be started.
pub fn ensure_running(url: &str, program: &Path, patience: Duration) -> Result<bool, String> {
    if answers(url) {
        return Ok(false);
    }
    if !is_local(url) {
        return Err(format!(
            "Nothing is answering at {url}, and it is not on this machine to be started from here."
        ));
    }
    let listen = authority(url).unwrap_or_default();
    // Why it was started is kept with its start, in the factory's own log,
    // where it can be found later. It is not news for whoever is looking
    // at the window now: they opened it and it works.
    let note = format!(
        "The daemon was not running, so the window started it. It keeps running when the window is closed; `{} down` stops it.",
        brand::CLI
    );
    let mut command = Command::new(program);
    command
        .args(["up", "--listen", listen, "--note", &note])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        // Its own process group, so that it is not taken down with the app.
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command.spawn().map_err(|err| {
        format!(
            "The daemon is not running, and `{}` could not be started ({err}). Start it from a terminal with `{} up`.",
            program.display(),
            brand::CLI
        )
    })?;
    let started = Instant::now();
    let mut pause = Duration::from_millis(50);
    while started.elapsed() < patience {
        if answers(url) {
            return Ok(true);
        }
        std::thread::sleep(pause);
        pause = (pause * 2).min(Duration::from_millis(400));
    }
    Err(format!(
        "The daemon was started but is still not answering at {url}. `{} status` in a terminal may say why.",
        brand::CLI
    ))
}

/// Where the window opens: at a link that signs the person in once
/// (`open --link`, asked of the daemon's own program, which holds the
/// owner's session or proves this machine can read the factory's folder),
/// else at the address, whose page says how to be signed in.
pub fn signed_in_address(url: &str, program: &Path) -> String {
    let asked = Command::new(program)
        .args(["open", "--link", "--url", url])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();
    asked
        .ok()
        .and_then(|said| link_in(&String::from_utf8_lossy(&said.stdout), url))
        .unwrap_or_else(|| url.to_string())
}

/// The link `open --link` said, where it is one to the daemon at `url`.
pub fn link_in(said: &str, url: &str) -> Option<String> {
    let start = format!("{}/#once=", url.trim_end_matches('/'));
    said.lines().map(str::trim).find(|line| line.starts_with(&start)).map(str::to_string)
}

/// What is shown in the window when there is no daemon to show.
pub fn trouble_page(message: &str) -> String {
    let escaped = message.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    format!(
        "<!doctype html><meta charset=utf-8><title>{name}</title>\
         <body style=\"margin:0;display:grid;place-items:center;height:100vh;background:#0c0c0c;color:#e9e9e9;font:15px system-ui\">\
         <div style=\"max-width:520px;padding:24px\"><h1 style=\"font-size:20px\">{name} is not running</h1>\
         <p style=\"color:#a3acb9;line-height:1.5\">{escaped}</p></div>",
        name = brand::DISPLAY
    )
}

/// An edge or corner of the window, to resize it by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

/// What the client asks of the window, in place of a title bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    /// Follow the pointer until the button is let go.
    Drag,
    Resize(Edge),
    Minimize,
    /// Or back to its size, if it is maximised already.
    Maximize,
    Close,
}

impl Ask {
    /// Reads what the client sent. It comes from a web page, so anything
    /// that is not exactly one of these is ignored.
    pub fn from_message(message: &str) -> Option<Ask> {
        let value: serde_json::Value = serde_json::from_str(message).ok()?;
        let asked = value.get("window")?;
        if let Some(edge) = asked.get("resize") {
            return Some(Ask::Resize(match edge.as_str()? {
                "n" => Edge::North,
                "s" => Edge::South,
                "e" => Edge::East,
                "w" => Edge::West,
                "ne" => Edge::NorthEast,
                "nw" => Edge::NorthWest,
                "se" => Edge::SouthEast,
                "sw" => Edge::SouthWest,
                _ => return None,
            }));
        }
        match asked.as_str()? {
            "drag" => Some(Ask::Drag),
            "minimize" => Some(Ask::Minimize),
            "maximize" => Some(Ask::Maximize),
            "close" => Some(Ask::Close),
            _ => None,
        }
    }
}

/// Who draws the top of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Framing {
    /// The client: its own bar, its own buttons, its own edges.
    Page,
    /// The client draws the bar, under buttons the system puts in its
    /// corner. This is how macOS windows without a title bar are made.
    Overlay,
    /// The system, title bar and all, and the client draws nothing.
    System,
}

impl Framing {
    /// What this system's windows are given, unless the system's own title
    /// bar was asked for.
    pub fn choose(system_asked_for: bool) -> Framing {
        if system_asked_for {
            Framing::System
        } else if cfg!(target_os = "macos") {
            Framing::Overlay
        } else {
            Framing::Page
        }
    }

    fn name(self) -> &'static str {
        match self {
            Framing::Page => "page",
            Framing::Overlay => "overlay",
            Framing::System => "system",
        }
    }
}

/// The script run in the page before any of its own: how the window is
/// framed.
pub fn page_setup(framing: Framing) -> String {
    let told = serde_json::json!({ "frame": framing.name() });
    format!("window.desktop = Object.freeze({told});")
}

/// Whether the system is set to dark, where the web view cannot find that out
/// for itself. `None` leaves it to the web view, which is right everywhere
/// but one place: a Linux app shown on a Windows desktop (WSL) sees Linux's
/// setting, which nobody set, and not Windows'.
pub fn system_dark() -> Option<bool> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let under_windows = std::env::var_os("WSL_DISTRO_NAME").is_some()
        || Path::new("/proc/sys/fs/binfmt_misc/WSLInterop").exists();
    if !under_windows {
        return None;
    }
    let key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
    ["reg.exe", "/mnt/c/Windows/System32/reg.exe"].iter().find_map(|program| {
        let asked = Command::new(program)
            .args(["query", key, "/v", "AppsUseLightTheme"])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        dark_from_registry(&String::from_utf8_lossy(&asked.stdout))
    })
}

/// Reads Windows' answer to "do apps use the light theme".
pub fn dark_from_registry(answer: &str) -> Option<bool> {
    let line = answer.lines().find(|line| line.contains("AppsUseLightTheme"))?;
    let value = line.split_whitespace().last()?;
    let light = u32::from_str_radix(value.trim_start_matches("0x"), 16).ok()?;
    Some(light == 0)
}
