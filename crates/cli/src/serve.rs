//! Running the daemon: `serve` is the daemon itself, `up` keeps one running.
//!
//! `up` is deliberately almost nothing: it starts the daemon, starts it again
//! if it exits, and passes on the signal to stop. It is the one piece that is
//! not replaced by restarting the daemon, so the less it does the better.
//! Restarting the daemon is safe by construction: what it runs is not stopped
//! with it, and the next daemon finds it all again.
//!
//! A factory has one daemon and one thing that keeps it running, and its
//! data folder says who they are (`api::folder`). A second `up` for a
//! factory that has both says so and ends; and a daemon that was refused
//! because the factory is being served already, or because its address
//! is something else's, is not started again: it would be refused the
//! same way however often it was asked ([`REFUSED`]).

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use api::{Config, Node};

use crate::args::Args;
use crate::client::{data_dir, data_dir_of};

/// The flags `serve` and `up` share, passed from one to the other.
const DAEMON_FLAGS: &[&str] = &[
    "profile",
    "banner",
    "data",
    "listen",
    "ui",
    "projects",
    "blueprints",
    "packages",
    "device",
    "adapter",
    "walls",
    "workers",
];

/// What a daemon ends with where it was not started and would not be
/// however often it was asked: its factory is being served already, its
/// address is something else's, or its data is not this program's to
/// open. It is how whatever keeps a daemon running tells that from a
/// daemon that died, which it starts again; and a keeper that is not
/// needed ends with it too. A usage error is 2, as it always was, and a
/// daemon a step asked to come back ends with 4
/// (`api::restarting::AGAIN`), which is started again at once.
const REFUSED: u8 = 3;

/// How long what would keep a daemon running waits, told that another
/// keeps it and has no daemon up, before it says what it is waiting
/// for: a daemon asked to stop is gone in less, with what kept it.
const QUIET: Duration = Duration::from_secs(2);

/// The most threads the daemon's work is done on where nobody says.
const MOST_WORKERS: usize = 8;

/// How many threads this program's work is done on: what it was told
/// (`--workers`, or the variable), and otherwise one for each processor
/// up to eight.
///
/// Left to itself the runtime starts one for every processor there is,
/// and each is a stack and a share of the allocator's memory whether or
/// not there is work for it: on thirty-two processors a factory at rest
/// held thirty-four threads it had no use for. Nearly all of what a
/// daemon does is waiting (on its database's one thread, on programs it
/// started, on the network), so a few are as quick as many. Somebody
/// with heavier work says how many they want.
pub fn workers(args: &Args) -> Result<usize, String> {
    let told = args
        .value("workers")
        .map(str::to_string)
        .or_else(|| std::env::var(brand::env_var("WORKERS")).ok().filter(|value| !value.is_empty()));
    match told {
        Some(told) => match told.parse::<usize>() {
            Ok(count) if (1..=1024).contains(&count) => Ok(count),
            _ => Err(format!(
                "`--workers` is how many threads do the work, a number from 1 up, and not `{told}`"
            )),
        },
        None => Ok(std::thread::available_parallelism().map_or(1, usize::from).min(MOST_WORKERS)),
    }
}

/// Which factory these arguments speak of: its profile, where it has
/// one, and where its data is.
pub fn place(args: &Args) -> Result<(Option<String>, PathBuf), String> {
    let env = |name: &str| std::env::var(brand::env_var(name)).ok().filter(|value| !value.is_empty());
    let setting = |flag: &str, variable: &str| args.value(flag).map(str::to_string).or_else(|| env(variable));
    // What the factory's profile is called decides where its data is,
    // so it is held to being a name before it is made part of a path.
    let profile = crate::profiled::asked(args);
    if let Some(name) = &profile {
        api::profile::check_name(name).map_err(|err| err.message)?;
    }
    let data = setting("data", "DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| profile.as_deref().map_or_else(data_dir, data_dir_of));
    Ok((profile, data))
}

/// The standard blueprints as files, for a program run from the checkout
/// it was built in. Read from disk they can be edited and applied again
/// without rebuilding anything. A copy of the program anywhere else uses
/// the ones built into it, whatever becomes of the checkout.
fn checkout_blueprints() -> Option<PathBuf> {
    let dir = api::place::checkout()?.join("blueprints");
    dir.join("std").join(brand::blueprint_file("slot")).is_file().then_some(dir)
}

/// Extension packages that came with this program: a folder beside it, or,
/// for one run from the checkout it was built in, the checkout's. They are
/// only offered; nothing is installed from here unasked.
fn packages() -> Option<PathBuf> {
    let beside = api::place::program_dir().map(|dir| dir.join("extensions"));
    let checkout = api::place::checkout().map(|root| root.join("extensions"));
    beside.into_iter().chain(checkout).find(|dir| dir.is_dir())
}

fn config(args: &Args) -> Result<Config, String> {
    let env = |name: &str| std::env::var(brand::env_var(name)).ok().filter(|value| !value.is_empty());
    let setting = |flag: &str, variable: &str| args.value(flag).map(str::to_string).or_else(|| env(variable));
    let path = |flag: &str, variable: &str| setting(flag, variable).map(PathBuf::from);
    let (profile, data) = place(args)?;
    let mut config = Config::new(data);
    config.profile = profile;
    // An owner before anybody is answered, where there is none: nobody
    // registers to begin (`people::opened`).
    config.makes_owner = true;
    match setting("listen", "LISTEN") {
        Some(listen) => {
            config.listen = listen
                .parse::<SocketAddr>()
                .map_err(|_| format!("`{listen}` is not an address like 127.0.0.1:7700"))?;
        }
        // Told nowhere to listen, a factory with a profile listens
        // where it did before; one with none, where they always have.
        None => config.keep_address = true,
    }
    config.registry = Some(api::registry::Registry::of_this_user().path().to_path_buf());
    config.ui_dir = path("ui", "UI_DIR");
    config.projects_dir = path("projects", "PROJECTS_DIR");
    config.blueprints_dir = path("blueprints", "BLUEPRINTS_DIR").or_else(checkout_blueprints);
    config.packages_dir = path("packages", "PACKAGES_DIR").or_else(packages);
    config.device = setting("device", "DEVICE");
    config.default_adapter = setting("adapter", "ADAPTER");
    // Walls are held unless whoever starts the factory says otherwise.
    config.walls = match setting("walls", "WALLS").as_deref() {
        None | Some("on") => None,
        Some("off") => Some(api::Walls::Open),
        Some(other) => return Err(format!("`--walls` is `on` or `off`, and not `{other}`")),
    };
    config.note = args.value("note").map(str::to_string).filter(|note| !note.trim().is_empty());
    // What every page says across its top in place of what it would (`folders::across`), kept from then on.
    config.banner =
        args.value("banner").map(api::across::told).transpose().map_err(|err| err.to_string())?.flatten();
    // Said each time, like why: a factory says what its steps were handed
    // only while somebody is looking at it.
    config.handed = args.has("handed");
    // Said by what keeps a daemon running, of each it starts: one that
    // ends saying it is to come back is started again.
    config.kept = args.has("kept");
    // A snapshot of another factory to start this one from. Said by a
    // person, each time, and never left standing in the environment.
    config.from = args.value("from").map(PathBuf::from);
    // `--command name=program args...` adds an agent program by configuration.
    for spec in args.values("command") {
        let (name, command) = spec.split_once('=').ok_or("`--command` is `name=program args...`")?;
        // What an agent no program here runs is said to run on.
        if name == agents::EXTERNAL {
            return Err(format!(
                "a program cannot be called `{name}`: that is what an agent this factory does not start is said \
                 to run on. Call it something else: `--command <name>=program args...`"
            ));
        }
        config.adapters.push((name.to_string(), command.split_whitespace().map(str::to_string).collect()));
    }
    Ok(config)
}

/// Being asked to stop (the signal a terminal sends, or the one whatever
/// hosts a program sends), listened for from when this is made. A program
/// makes it before it does anything that says it is there: one that
/// first said where it was and then began to listen was ended by the
/// system where it stood when it was asked in between, having stopped
/// nothing it had started, and whatever asked took it for one that had
/// failed.
pub(crate) struct Asked {
    #[cfg(unix)]
    terminate: tokio::signal::unix::Signal,
    #[cfg(unix)]
    interrupt: tokio::signal::unix::Signal,
}

impl Asked {
    pub(crate) fn listening() -> Asked {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};
            let listen = |kind| signal(kind).expect("signal handling is available");
            Asked { terminate: listen(SignalKind::terminate()), interrupt: listen(SignalKind::interrupt()) }
        }
        #[cfg(not(unix))]
        Asked {}
    }

    /// Waits until it has been asked, which may have been before this.
    pub(crate) async fn heard(&mut self) {
        #[cfg(unix)]
        tokio::select! {
            _ = self.interrupt.recv() => {}
            _ = self.terminate.recv() => {}
        }
        #[cfg(not(unix))]
        {
            let _ = tokio::signal::ctrl_c().await;
        }
    }
}

/// Runs the daemon in this process until it is told to stop.
pub async fn serve(args: &Args) -> ExitCode {
    // Before the node is there to be asked anything.
    let mut asked = Asked::listening();
    let config = match config(args) {
        Ok(config) => config,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(2);
        }
    };
    let open = args.has("open");
    // A flag nobody reads is refused before anything is started (`main`
    // says why).
    if args.heed().check().is_err() {
        return ExitCode::from(2);
    }
    let data = config.data_dir.clone();
    let blueprints = config.blueprints_dir.clone();
    let node = match Node::start(config).await {
        Ok(node) => node,
        Err(err) => {
            eprintln!("error: {}", err.message);
            // What was asked for cannot be: asking again, as whatever keeps
            // the daemon running would, will not change that. Nor will it
            // change what conflicts with how things stand: a factory that
            // is being served, an address that is something else's.
            return match err.kind {
                kernel::ErrorKind::Invalid => ExitCode::from(2),
                kernel::ErrorKind::Conflict => ExitCode::from(REFUSED),
                _ => ExitCode::FAILURE,
            };
        }
    };
    // Listening beyond this machine is somebody's choice, and what it
    // means is said once, where they will see it: before it says it is
    // running, so that whoever is told the one has been told the other.
    if !node.addr.ip().is_loopback() {
        eprintln!(
            "warning: listening at {}, which other machines can reach. Nothing here is encrypted: a \
             password typed into the client, and every credential, crosses the network in the clear. \
             Listen on this machine only (the default) and put something that encrypts in front of it, \
             or keep to a network of your own.",
            node.addr
        );
    }
    // The owner this start made is signed in on this machine's command
    // line before it is said to be running, so that whoever waits for that
    // finds the command line signed in.
    let made = node.made.clone();
    let kept = match &made {
        Some(signed) => crate::client::keep_made(&node.url(), node.profile(), &data, &signed.token),
        None => Ok(()),
    };
    println!("{} is running at {}", brand::DISPLAY, node.url());
    // A factory with a profile is asked for by it: plain commands go
    // to wherever one with none listens.
    let mut status = format!("{} status", brand::CLI);
    if let Some(name) = node.profile() {
        println!("  profile  {name}");
        status = format!("{} --profile {name} status", brand::CLI);
    }
    println!("  data    {}", data.display());
    println!("  client  {}   (or `{status}` from a terminal)", node.url());
    if let Some(dir) = blueprints {
        println!("  blueprints  {}   (read when applied; edit and apply again)", dir.display());
    }
    // Who it is, and that the command line opens the client signed in by a
    // link (`open`): said before which program this is, which is the last
    // line of its start that whoever started it may wait for.
    let at_terminal = std::io::IsTerminal::is_terminal(&std::io::stdout());
    if let Some(signed) = &made {
        println!("  owner   {}, made now: nobody registers to begin", signed.account.name);
        if let Err(why) = &kept {
            println!("  {why}");
        }
        if !open && !at_terminal {
            println!("  open    `{}` opens the client in your browser, signed in", opens(node.profile()));
        }
    }
    // Which program this is: a copy on its own, or one run from the
    // checkout it was built in, which reads that checkout as it changes.
    println!("  program  {}", api::place::said());
    // A factory started from a copy of another says what it is a copy of,
    // what did not come with it, and what to do next.
    for line in node.arrived.iter().flat_map(api::arrival::Arrived::said) {
        println!("  {line}");
    }
    // Opened signed in where it was asked to be, or where this start made
    // the owner and somebody is at a terminal to see a browser come up;
    // anywhere else what opens it is said, and the link is printed nowhere
    // a log would keep it.
    if open || (made.is_some() && at_terminal) {
        match node.link_for_owner().await {
            Ok(link) => open_signed_in(&format!("{}/#once={}", node.url(), link.once)),
            Err(_) => open_in_browser(&node.url()),
        }
    }
    tokio::select! {
        () = asked.heard() => {}
        // `down`, from anywhere that can sign in as the owner.
        () = node.asked_to_stop() => {}
    }
    // Asked by a step of its own to come back: what keeps it running is
    // told so by how it ends.
    let again = node.again();
    match again {
        true => {
            println!("stopping to be started again, as a step asked; what is running is left running for it")
        }
        false => println!("stopping; what is running is left running for the next start"),
    }
    // Nothing is lost by a database that was not put away in time: what
    // was written is in its log, which the next start takes up. It is
    // said, since the next start waits for the file.
    for left in node.stop_saying(|said| println!("  {said}")).await {
        eprintln!("warning: stopped before a database was put away, which the next start waits for: {left}");
    }
    if again { ExitCode::from(api::restarting::AGAIN) } else { ExitCode::SUCCESS }
}

/// Keeps a daemon running: starts it, and starts it again if it exits.
pub async fn up(args: &Args) -> ExitCode {
    let mut asked = Asked::listening();
    let Ok(exe) = std::env::current_exe() else {
        eprintln!("error: cannot find this program to start the daemon with");
        return ExitCode::FAILURE;
    };
    let mut daemon_args: Vec<String> = vec!["serve".into(), "--kept".into()];
    // The profile in its new spelling, however it was asked for, so that
    // the old spelling is said once, here.
    if args.value("profile").is_none()
        && let Some(name) = crate::profiled::asked(args)
    {
        daemon_args.extend(["--profile".to_string(), name]);
    }
    for flag in DAEMON_FLAGS {
        if let Some(value) = args.value(flag) {
            daemon_args.extend([format!("--{flag}"), value.to_string()]);
        }
    }
    for command in args.values("command") {
        daemon_args.extend(["--command".to_string(), command.to_string()]);
    }
    if args.has("open") {
        daemon_args.push("--open".into());
    }
    if args.has("handed") {
        daemon_args.push("--handed".into());
    }

    // Why it is being started is true of this start, not of the ones that
    // follow if it exits and is started again.
    let mut note = args.value("note").map(|note| ["--note".to_string(), note.to_string()]);
    // A snapshot to start from is for a factory that does not exist yet.
    // The first daemon is always told of it, and says so if it cannot be;
    // one started after that is told only while the data folder still
    // holds no factory, which is when the one before it died part-way
    // through. Once there is a factory, it is started as any is.
    let from = args.value("from").map(|from| ["--from".to_string(), from.to_string()]);
    let data = config(args).ok().map(|config| config.data_dir);
    if args.heed().check().is_err() {
        return ExitCode::from(2);
    }
    let mut begun = false;

    // Known to the factory's folder as what keeps its daemon running,
    // from the first moment a daemon has had the folder.
    let mut keeping = None;
    let mut quick_exits = 0u32;
    loop {
        if let Err(code) = known(&mut keeping, data.as_deref(), &mut asked).await {
            return code;
        }
        let started = Instant::now();
        let first = note.take().into_iter().flatten();
        let arrived = begun && data.as_deref().is_none_or(api::arrival::is_factory);
        let copy = from.clone().filter(|_| !arrived).into_iter().flatten();
        begun = true;
        let mut child =
            match tokio::process::Command::new(&exe).args(&daemon_args).args(first).args(copy).spawn() {
                Ok(child) => child,
                Err(err) => {
                    eprintln!("error: could not start the daemon: {err}");
                    return ExitCode::FAILURE;
                }
            };
        let status = tokio::select! {
            status = child.wait() => status,
            () = asked.heard() => {
                // The daemon got the same signal if it came from the
                // terminal; make sure of it, then wait for it to finish.
                terminate(&child);
                let _ = child.wait().await;
                return ExitCode::SUCCESS;
            }
        };
        // Only ever opened on the first start.
        daemon_args.retain(|arg| arg != "--open");
        match status {
            // It was asked to stop, and did.
            Ok(status) if status.success() => return ExitCode::SUCCESS,
            // A usage error will not get better by trying again.
            Ok(status) if status.code() == Some(2) => return ExitCode::from(2),
            // Nor will a factory that is being served already, or an
            // address that is something else's: the daemon said which,
            // and what to do. It did not die, and is not started again.
            // Asked by a step of its own to come back: it is started again
            // at once, as nothing that went wrong.
            Ok(status) if status.code() == Some(i32::from(api::restarting::AGAIN)) => {
                eprintln!("the daemon stopped to be started again, as a step asked; starting it");
                quick_exits = 0;
                continue;
            }
            Ok(status) if status.code() == Some(i32::from(REFUSED)) => {
                eprintln!(
                    "the daemon was refused, and would be however often it was asked: it is not started again"
                );
                return ExitCode::from(REFUSED);
            }
            _ => {}
        }
        // The daemon that ended may have been the first to have the
        // folder: this is known to it from here, and not only once it
        // has waited to start the next.
        if let Err(code) = known(&mut keeping, data.as_deref(), &mut asked).await {
            return code;
        }
        // A daemon that keeps dying at once is waited on for longer each
        // time; one that ran for a while starts again promptly.
        quick_exits = if started.elapsed() > Duration::from_secs(60) { 0 } else { quick_exits + 1 };
        let wait = Duration::from_millis(250 * (1u64 << quick_exits.min(7)) - 250);
        eprintln!("the daemon exited ({status:?}); starting it again in {:.1}s", wait.as_secs_f64());
        tokio::select! {
            () = tokio::time::sleep(wait) => {}
            () = asked.heard() => return ExitCode::SUCCESS,
        }
    }
}

/// Has this program known to its factory's folder as what keeps its
/// daemon running, where it is not yet and the factory has a folder
/// ([`keeps`]): asked before each daemon is started, and as each ends.
async fn known(
    keeping: &mut Option<api::folder::Keeping>,
    data: Option<&std::path::Path>,
    asked: &mut Asked,
) -> Result<(), ExitCode> {
    if let (None, Some(data)) = (&*keeping, data) {
        *keeping = keeps(data, asked).await?;
    }
    Ok(())
}

/// Has this program be what keeps the daemon of the factory at `data`
/// running, or says why it is not to be: `None` where no daemon has had
/// the folder yet, which is asked again once one has.
///
/// Another that keeps it with its daemon up is why this one ends, saying
/// so. Another whose daemon is not up is waited for: it ends in a moment
/// if its daemon was asked to stop (`down`, and then `up`, is how a
/// factory is started again), and has its daemon up in a moment if it is
/// starting it again. Two that went on side by side were one that had
/// the daemon and one that started a daemon which could not listen for
/// as long as it was left, and brought the factory back when it was
/// stopped on purpose.
async fn keeps(data: &std::path::Path, asked: &mut Asked) -> Result<Option<api::folder::Keeping>, ExitCode> {
    let (mut waited, mut said) = (Duration::ZERO, false);
    loop {
        match api::folder::keep(data) {
            Ok(api::folder::Keep::Kept(keeping)) => return Ok(Some(keeping)),
            Ok(api::folder::Keep::Nowhere) => return Ok(None),
            Ok(api::folder::Keep::Running(why)) => {
                eprintln!("error: {why}");
                return Err(ExitCode::from(REFUSED));
            }
            Ok(api::folder::Keep::Waiting(why)) => {
                // Said once, when it has gone on for longer than a stop does.
                if waited >= QUIET && !said {
                    eprintln!("{why}");
                    said = true;
                }
            }
            Err(err) => {
                eprintln!("error: {}", err.message);
                return Err(ExitCode::FAILURE);
            }
        }
        let pause = Duration::from_millis(50);
        tokio::select! {
            () = tokio::time::sleep(pause) => waited += pause,
            () = asked.heard() => return Err(ExitCode::SUCCESS),
        }
    }
}

fn terminate(child: &tokio::process::Child) {
    let Some(pid) = child.id() else { return };
    // Asked, and only the daemon: it stops what it started itself, and
    // leaves running what is to outlive it.
    if let Err(why) = hosting::group::ask(pid) {
        eprintln!("the daemon could not be asked to stop: {why}");
    }
}

/// Opens the client in the user's browser.
pub fn open_in_browser(url: &str) {
    // Said first, and whatever comes of it: what opens a browser may start
    // and open none (a device with no screen), and said nothing then, so
    // that whoever asked had no address and no word of why.
    println!("{}", opening(url));
    if !start_browser(url) {
        println!("no browser could be started from here: open that address in one");
    }
}

/// Starts the system's way of opening `url` in a browser, and says whether
/// it could be started; what comes of it is the system's.
fn start_browser(url: &str) -> bool {
    let (program, args): (&str, &[&str]) = if cfg!(target_os = "macos") {
        ("open", &[])
    } else if cfg!(windows) {
        ("cmd", &["/C", "start", ""])
    } else {
        ("xdg-open", &[])
    };
    std::process::Command::new(program)
        .args(args)
        .arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_ok()
}

/// The command that opens the client of this factory, signed in.
fn opens(profile: Option<&str>) -> String {
    match profile {
        Some(name) => format!("{} --profile {name} open", brand::CLI),
        None => format!("{} open", brand::CLI),
    }
}

/// Opens the client in the user's browser at a link that signs them in
/// once: said with the link, which is all there is to go by where no
/// browser comes up, and works once, within a minute.
pub fn open_signed_in(link: &str) {
    println!("opening {link} in your browser: it signs you in once, within a minute");
    if !start_browser(link) {
        println!("no browser could be started from here: open that address in one");
    }
}

/// What is said as the client is opened in a browser: where it is, which
/// is all there is to go by where no browser comes up.
fn opening(url: &str) -> String {
    format!("the client is at {url}; opening it in your browser")
}
