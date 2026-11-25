//! What is still running here that nobody waits for.
//!
//! A test ends what it starts, however it ends: the daemon leaves its
//! services running on purpose, a stand-in told to wait goes on waiting,
//! and a check of the client has a daemon of its own. What was left held
//! a lock, a port or a processor until somebody came across it, on a
//! machine where the gates run all day beside other work, and nothing
//! looked. This is the looking: what is running whose program is under
//! this checkout's `target/`, whose folder is under the checkout, or that
//! a test program of this checkout's gate started (`FROM`, which whatever
//! such a program starts is handed on with the rest of what it was
//! told).
//!
//! It is asked two ways. `cargo xtask left` lists what there is, and ends
//! nothing. The gates that run the test programs look before and after,
//! and fail for what is new and nobody's (`new`): something that was
//! there before is not theirs, and neither is what somebody who was there
//! before still waits for (another terminal's build, an editor's).
//!
//! Whether a process is nobody's is its parent having gone, which the
//! system says only by who has it now: whoever takes in what is left
//! behind (`Looked::adopts`), which is the system's first process unless
//! something nearer says it does, and nothing says which. So the looking
//! leaves something behind itself and reads who has it (`adopter`): this
//! tooling's own program, started to start itself again and end.
//!
//! What a look found is plain values, so that what is made of them is
//! asked with cases and no process; the reading is Linux's `/proc`, and
//! on a system with none a look says so and finds nothing.

use std::path::Path;

/// What a test program is told by the gate that runs it: whose tests it
/// is, of which checkout (`from`). Nothing reads it but a look.
pub const FROM: &str = "TESTS_OF";

/// What the tests of `krate` in the checkout at `root` are told.
pub fn from(root: &Path, krate: &str) -> String {
    format!("{krate}@{}", root.display())
}

/// Which crate's tests said `told`, where they are of the checkout at `root`.
fn of_here<'a>(told: &'a str, root: &Path) -> Option<&'a str> {
    let (krate, checkout) = told.split_once('@')?;
    (Path::new(checkout) == root).then_some(krate)
}

/// One process, as a look found it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Seen {
    pub pid: u32,
    pub parent: u32,
    /// When it began, in hundredths of a second since the system did.
    /// With its number this is which process it is: a number is given
    /// again.
    pub began: u64,
    /// It leads a session that has a terminal: somebody's shell.
    pub leads: bool,
    /// Whoever looks may read it, which is to say it is theirs.
    pub ours: bool,
    /// Its program's file.
    pub program: Option<String>,
    /// The folder it is in.
    pub folder: Option<String>,
    /// What it was started with, its words with a space between.
    pub told: String,
    /// What it carries of `FROM`.
    pub from: Option<String>,
}

/// What one look found.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Looked {
    pub seen: Vec<Seen>,
    /// Whoever looked.
    pub me: u32,
    /// Who takes in a process whose parent has gone.
    pub adopts: Vec<u32>,
    /// When it looked, as `Seen::began` is counted.
    pub at: u64,
}

impl Looked {
    fn one(&self, pid: u32) -> Option<&Seen> {
        self.seen.iter().find(|seen| seen.pid == pid)
    }

    fn holds(&self, seen: &Seen) -> bool {
        self.seen.iter().any(|one| one.pid == seen.pid && one.began == seen.began)
    }

    /// Whether the parent of `seen` has gone: it is with whoever takes
    /// such in.
    fn orphaned(&self, seen: &Seen) -> bool {
        self.adopts.contains(&seen.parent) || self.one(seen.parent).is_none()
    }

    /// `seen` and what it came from, nearest first. A look is not one
    /// moment, so it goes no further than a tree is ever deep.
    fn line<'a>(&'a self, seen: &'a Seen) -> Vec<&'a Seen> {
        let mut line = vec![seen];
        while let Some(parent) = self.one(line[line.len() - 1].parent) {
            if line.len() == DEEPEST || line.iter().any(|one| one.pid == parent.pid) {
                break;
            }
            line.push(parent);
        }
        line
    }
}

/// How far a process is followed towards what it came from.
const DEEPEST: usize = 64;

/// Why a process is this checkout's.
#[derive(Clone, Debug, PartialEq)]
pub enum Why {
    /// Its program is under the checkout's `target/`.
    Program,
    /// A test program of this checkout's gate started it, or something
    /// that did.
    Told,
    /// It is in a folder of the checkout.
    Folder,
    /// It came from one that is, which has this number.
    Under(u32),
}

/// Something that is running and is this checkout's.
#[derive(Clone, Debug, PartialEq)]
pub struct Left {
    pub seen: Seen,
    pub why: Why,
    /// Which crate's tests it came from, where anything says.
    pub of: Option<String>,
    /// Nobody waits for it: its parent has gone, or the one it came from
    /// has lost its own.
    pub nobodys: bool,
}

/// Why `seen` is of the checkout at `root` by what it says itself.
fn why(seen: &Seen, root: &Path) -> Option<Why> {
    let under = |path: &Option<String>, folder: &Path| {
        path.as_deref().is_some_and(|path| Path::new(path).starts_with(folder))
    };
    if under(&seen.program, &root.join("target")) {
        Some(Why::Program)
    } else if seen.from.as_deref().is_some_and(|told| of_here(told, root).is_some()) {
        Some(Why::Told)
    } else if under(&seen.folder, root) {
        Some(Why::Folder)
    } else {
        None
    }
}

/// Which crate's tests `seen` came from, by what it or the nearest thing
/// it came from carries.
fn of(line: &[&Seen], root: &Path) -> Option<String> {
    line.iter().find_map(|one| of_here(one.from.as_deref()?, root)).map(str::to_string)
}

/// What is running that is this checkout's, the oldest first: every
/// program of its `target/`, whoever waits for it; what is in a folder of
/// it, or was started by its tests, and has lost its parent; and whatever
/// came from one of those. Whoever looks, and what that came from, are
/// left out, and so is somebody's shell that happens to be in the folder.
///
/// `after` is a process that is about to end, where whoever asks is one
/// (a check of the client, as it exits): what came from it has nobody
/// waiting for it by then, and is said so now.
pub fn lying(looked: &Looked, root: &Path, after: Option<u32>) -> Vec<Left> {
    let mine: Vec<u32> = match looked.one(looked.me) {
        Some(me) => looked.line(me).iter().map(|one| one.pid).collect(),
        None => vec![looked.me],
    };
    // Whoever has a process that is not theirs to read took it in: the
    // system's own, which whoever looks did not come from.
    let alone =
        |seen: &Seen| looked.orphaned(seen) || looked.one(seen.parent).is_some_and(|parent| !parent.ours);
    let own = |seen: &Seen| -> Option<Why> {
        if mine.contains(&seen.pid) {
            return None;
        }
        match why(seen, root)? {
            Why::Program => Some(Why::Program),
            other => (alone(seen) && !seen.leads).then_some(other),
        }
    };
    let mut left: Vec<Left> = looked
        .seen
        .iter()
        .filter(|seen| !mine.contains(&seen.pid))
        .filter_map(|seen| {
            let line = looked.line(seen);
            let (why, top) = match own(seen) {
                Some(why) => (why, seen),
                None => {
                    let from = line[1..].iter().find(|one| own(one).is_some())?;
                    (Why::Under(from.pid), *from)
                }
            };
            let nobodys = alone(top)
                || looked.line(top).iter().any(|one| alone(one) && own(one).is_some())
                || after.is_some_and(|after| line.iter().any(|one| one.pid == after));
            Some(Left { seen: seen.clone(), why, of: of(&line, root), nobodys })
        })
        .collect();
    left.sort_by_key(|left| (left.seen.began, left.seen.pid));
    left
}

/// What is running `after` that was not `before`, is this checkout's, and
/// that nobody who was there before waits for: what the test programs
/// run between the two looks left behind. A program of another checkout
/// is not this one's; something there before is not new; and something
/// whose parent, or the parent of what it came from, was there before and
/// still is (another terminal's build, whoever looks itself) is theirs.
pub fn new(before: &Looked, after: &Looked, root: &Path) -> Vec<Left> {
    let mut left: Vec<Left> = after
        .seen
        .iter()
        .filter_map(|seen| {
            // What it came from, as far as that is new.
            let line: Vec<&Seen> =
                after.line(seen).into_iter().take_while(|one| !before.holds(one)).collect();
            let top = line.last()?;
            // A shell somebody opened meanwhile, and what they run in it,
            // is theirs, whoever started the shell for them.
            if !after.orphaned(top) || line.iter().any(|one| one.leads) {
                return None;
            }
            let (index, why) =
                line.iter().enumerate().find_map(|(index, one)| Some((index, why(one, root)?)))?;
            let why = if index == 0 { why } else { Why::Under(line[index].pid) };
            Some(Left { seen: seen.clone(), why, of: of(&line, root), nobodys: true })
        })
        .collect();
    left.sort_by_key(|left| (left.seen.began, left.seen.pid));
    left
}

/// What a test must do, said where the gate found something left.
pub const MUST: &str = "a test ends what it starts, however it ends: a guard that ends it when the test is over, \
     whether it passed or not, and a program that ends by itself. What is left holds a lock, a port or a \
     processor for whoever works here next. `cargo xtask left` lists what is running here, and ends nothing";

/// A path as it is said: from the checkout, where it is in it.
fn place(path: &str, root: &Path) -> String {
    match Path::new(path).strip_prefix(root) {
        Ok(within) if within.as_os_str().is_empty() => ".".into(),
        Ok(within) => within.display().to_string(),
        Err(_) => path.to_string(),
    }
}

/// How long ago, in the largest unit that says it in a few figures.
pub fn ago(hundredths: u64) -> String {
    let seconds = hundredths / 100;
    match seconds {
        0..120 => format!("{seconds} s"),
        120..7200 => format!("{} min", seconds / 60),
        _ => format!("{} h", seconds / 3600),
    }
}

/// The most of what a process was started with that is said in a line.
const WORDS: usize = 240;

/// One line for something left: which process, how long it has been
/// there, whose tests it came from where that is known, what it was
/// started with and where it is.
pub fn said(left: &Left, at: u64, root: &Path) -> String {
    let seen = &left.seen;
    let from = match (&left.of, &left.why) {
        (Some(krate), _) => format!(", from the tests of `{krate}`"),
        (None, Why::Under(pid)) => format!(", started by {pid}"),
        (None, _) => String::new(),
    };
    // In a line: a word of it may be a script of many.
    let whole = seen.told.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut told: String = whole.chars().take(WORDS).collect();
    if told.len() < whole.len() {
        told.push('…');
    }
    if told.is_empty() {
        told = seen.program.clone().unwrap_or_else(|| "a program that says nothing of itself".into());
    }
    let folder =
        seen.folder.as_deref().map_or(String::new(), |folder| format!(" (in {})", place(folder, root)));
    let waited = if left.nobodys { String::new() } else { format!(", under {}", seen.parent) };
    format!("{} for {}{from}{waited}: {told}{folder}", seen.pid, ago(at.saturating_sub(seen.began)))
}

/// What a process says of itself in `/proc/<pid>/stat`: its parent, how it
/// stands, its session, its terminal and when it began. Its name is
/// between brackets and may hold anything, a bracket included, so the
/// rest is read from the last one.
pub fn stat(text: &str) -> Option<Stat> {
    let rest = &text[text.rfind(')')? + 1..];
    let words: Vec<&str> = rest.split_whitespace().collect();
    Some(Stat {
        state: words.first()?.chars().next()?,
        parent: words.get(1)?.parse().ok()?,
        session: words.get(3)?.parse().ok()?,
        terminal: words.get(4)?.parse().ok()?,
        began: words.get(19)?.parse().ok()?,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stat {
    pub state: char,
    pub parent: u32,
    pub session: u32,
    pub terminal: i64,
    pub began: u64,
}

impl Stat {
    /// Over, and only not yet taken off the list by whoever has it.
    pub fn over(&self) -> bool {
        matches!(self.state, 'Z' | 'X' | 'x')
    }
}

/// Said where there is nothing to read what is running from.
pub const NOWHERE: &str =
    "what is running is read from /proc, which this system does not have: nothing was looked for";

/// What this tooling's own program is started with to leave something
/// behind (`leaves`), and what it starts itself with to be that (`waits`).
pub const LEAVES: &str = "leaves-one-waiting";
pub const WAITS: &str = "waits-to-be-let-go";

/// Starts this program again to wait, handing it what this one reads
/// from, says which process that is, and is over: what waits is then
/// nobody's.
pub fn leaves() -> std::io::Result<()> {
    let waits = std::process::Command::new(std::env::current_exe()?)
        .arg(WAITS)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    println!("{}", waits.id());
    Ok(())
}

/// Waits until what it reads from is closed, which whoever left it does
/// by letting go or by ending.
pub fn waits() {
    let _ = std::io::copy(&mut std::io::stdin(), &mut std::io::sink());
}

#[cfg(not(target_os = "linux"))]
pub fn look(_: Option<u32>) -> Result<Looked, String> {
    Err(NOWHERE.into())
}

#[cfg(not(target_os = "linux"))]
pub fn adopter(_: &Path) -> Option<u32> {
    None
}

#[cfg(not(target_os = "linux"))]
pub fn still(_: &Seen) -> bool {
    false
}

#[cfg(target_os = "linux")]
pub use linux::{adopter, look, still};

#[cfg(target_os = "linux")]
mod linux {
    use std::fs;
    use std::io::Read;
    use std::path::Path;
    use std::process::{Command, Stdio};

    use super::{FROM, LEAVES, Looked, NOWHERE, Seen, Stat, stat};

    /// A program's file or its folder as the system says it, which adds a
    /// word to one that is no longer there: a program built again since
    /// it was started, a folder taken away under what is in it.
    const DELETED: &str = " (deleted)";

    fn words(path: &Path) -> Option<Vec<String>> {
        let bytes = fs::read(path).ok()?;
        Some(
            bytes
                .split(|byte| *byte == 0)
                .filter(|word| !word.is_empty())
                .map(|word| String::from_utf8_lossy(word).into_owned())
                .collect(),
        )
    }

    fn stat_of(pid: u32) -> Option<Stat> {
        stat(&fs::read_to_string(format!("/proc/{pid}/stat")).ok()?)
    }

    /// Whether the process a look saw is still there, and is that one.
    pub fn still(seen: &Seen) -> bool {
        stat_of(seen.pid).is_some_and(|now| now.began == seen.began && !now.over())
    }

    /// Who takes in what is left behind by whoever looks: something is
    /// left behind (`program`, which is this tooling's own, started with
    /// `LEAVES`: it starts itself again to wait and ends), and its parent
    /// is read once what started it is over, by when the system has
    /// handed it on. It is let go by what it reads from being closed,
    /// here or by this program ending.
    pub fn adopter(program: &Path) -> Option<u32> {
        let mut leaves = Command::new(program)
            .arg(LEAVES)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        // Kept apart: waiting for it would close this.
        let held = leaves.stdin.take();
        let mut said = String::new();
        leaves.stdout.take()?.read_to_string(&mut said).ok()?;
        leaves.wait().ok()?;
        let adopter = stat_of(said.trim().parse().ok()?).map(|stat| stat.parent);
        drop(held);
        adopter
    }

    /// Everything that is running, as far as whoever looks may read it.
    /// `adopter` is who takes in what whoever looks leaves behind, where
    /// that was found out.
    pub fn look(adopter: Option<u32>) -> Result<Looked, String> {
        let listed = fs::read_dir("/proc").map_err(|_| NOWHERE.to_string())?;
        let at = fs::read_to_string("/proc/uptime")
            .ok()
            .and_then(|text| text.split_whitespace().next()?.parse::<f64>().ok())
            .map_or(0, |seconds| (seconds * 100.0) as u64);
        let mut seen = Vec::new();
        for entry in listed.flatten() {
            let Some(pid) = entry.file_name().to_str().and_then(|name| name.parse::<u32>().ok()) else {
                continue;
            };
            let Some(stat) = stat_of(pid) else { continue };
            if stat.over() {
                continue;
            }
            let folder = entry.path();
            let link = |name: &str| {
                let path = fs::read_link(folder.join(name)).ok()?.to_string_lossy().into_owned();
                Some(path.strip_suffix(DELETED).map(str::to_string).unwrap_or(path))
            };
            let program = link("exe");
            let ours = program.is_some();
            let from = ours.then(|| words(&folder.join("environ"))).flatten().and_then(|all| {
                all.iter().find_map(|one| one.strip_prefix(&format!("{FROM}=")).map(str::to_string))
            });
            seen.push(Seen {
                pid,
                parent: stat.parent,
                began: stat.began,
                leads: stat.session == pid && stat.terminal != 0,
                ours,
                program,
                folder: link("cwd"),
                told: if ours {
                    words(&folder.join("cmdline")).unwrap_or_default().join(" ")
                } else {
                    String::new()
                },
                from,
            });
        }
        let mut adopts = vec![1];
        adopts.extend(adopter);
        Ok(Looked { seen, me: std::process::id(), adopts, at })
    }
}
