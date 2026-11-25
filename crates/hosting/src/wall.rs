//! A wall: what a program started here sees of the device.
//!
//! Everything the daemon starts runs as whoever runs the daemon, and with
//! no wall sees what they see: their home and the keys in it, the
//! factory's own data, every other project. Behind a wall a program is
//! shown the system's programs, the folder it works in, a home of its own
//! and what its owner added, and inherits nothing: what is not shown is
//! not there to read, and nothing but what it may change can be written.
//!
//! The wall is held by a program of the system's (bubblewrap's `bwrap`, on
//! Linux), which starts the command in a view of the files made for it.
//! Nothing here is a sandbox of its own making: this says what to show and
//! the system does the showing. A wall that cannot be held is a failure,
//! and never the open device instead.
//!
//! It is a wall for files and for what would have been inherited. It does
//! not hold the network: what is behind it reaches the daemon, which is
//! how an agent acts on the factory, and whatever else the device reaches.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

/// Where a system keeps the program that holds a wall. By where it is kept
/// and not by looking along the daemon's path: what holds a wall must not
/// be whatever was put first on somebody's path.
const HOLDER: &str = "/usr/bin/bwrap";

/// What of the system every program is shown, to read: its programs and
/// libraries and how it is set up. Each only where the system has it.
const SYSTEM: &[&str] =
    &["/usr", "/bin", "/sbin", "/lib", "/lib32", "/lib64", "/libx32", "/etc", "/opt", "/nix", "/snap"];

/// Folders on a path that are another system's, mounted into this one: a
/// path there is given nothing behind a wall, where none of it would run.
const FOREIGN: &[&str] = &["/mnt", "/media", "/run", "/proc", "/sys", "/dev", "/tmp"];

/// What a file is called when it holds a credential, by the custom of the
/// programs that keep one. In a folder that was added for a program
/// behind a wall, a file called one of these is not shown: a folder of a
/// toolchain is added so that it builds, not so that it publishes.
const CREDENTIALS: &[&str] = &[
    "credentials",
    "credentials.toml",
    "credentials.json",
    ".credentials.json",
    ".netrc",
    ".npmrc",
    ".pypirc",
    ".git-credentials",
    "auth.json",
    "id_rsa",
    "id_ed25519",
    "id_ecdsa",
];

/// What the environment of a program behind a wall keeps of the daemon's:
/// where things are, how text is written, and how the network is reached.
/// Nothing else is inherited: a key that whoever started the daemon had in
/// their environment is not a program's behind a wall.
pub const INHERITED: &[&str] = &[
    "PATH",
    "HOME",
    "USER",
    "LOGNAME",
    "SHELL",
    "LANG",
    "LANGUAGE",
    "LC_ALL",
    "LC_CTYPE",
    "TERM",
    "COLORTERM",
    "TZ",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "ALL_PROXY",
    "NO_PROXY",
    "http_proxy",
    "https_proxy",
    "all_proxy",
    "no_proxy",
    "SSL_CERT_FILE",
    "SSL_CERT_DIR",
];

/// How walls are held on this device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Walls {
    /// By this program, which is asked to start each command.
    Held(PathBuf),
    /// Not at all, because whoever started the factory said so: every
    /// program sees the device as they do, whatever its own row says.
    Open,
}

impl Walls {
    /// Held by the program the system keeps for it.
    pub fn system() -> Walls {
        Walls::Held(PathBuf::from(HOLDER))
    }

    /// Whether a wall can be held here, and in a sentence why not. It is
    /// tried, since a program that is there may still be refused by the
    /// system; and tried once, since the answer does not change while the
    /// daemon runs.
    pub fn can(&self) -> Result<(), String> {
        let Walls::Held(holder) = self else {
            return Err("this factory was started holding no walls".into());
        };
        static TRIED: Mutex<BTreeMap<PathBuf, Result<(), String>>> = Mutex::new(BTreeMap::new());
        let mut tried = TRIED.lock().expect("walls lock");
        if let Some(known) = tried.get(holder) {
            return known.clone();
        }
        let found = try_one(holder);
        tried.insert(holder.clone(), found.clone());
        found
    }
}

/// Holds a wall around a command that does nothing, to see whether one
/// can be held at all.
fn try_one(holder: &Path) -> Result<(), String> {
    if !holder.is_file() && !cfg!(target_os = "linux") {
        return Err(format!(
            "a wall is held by bubblewrap's `bwrap`, which only Linux has; there is none at {}",
            holder.display()
        ));
    }
    if !holder.is_file() {
        return Err(format!(
            "the program that holds one, bubblewrap's `bwrap`, is not at {}; it is a package of the system's, usually called `bubblewrap`",
            holder.display()
        ));
    }
    let tried = Command::new(holder)
        .args(["--unshare-user", "--unshare-pid", "--ro-bind", "/", "/", "--proc", "/proc", "--dev", "/dev"])
        .args(["--", "/bin/sh", "-c", ":"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output();
    match tried {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => {
            let said = String::from_utf8_lossy(&output.stderr);
            let why = said.lines().find(|line| !line.trim().is_empty()).unwrap_or("it gave no reason").trim();
            Err(format!(
                "`{}` is here and the system would not let it ({why}). Some systems allow it only once they are told to let programs make a namespace of their own",
                holder.display()
            ))
        }
        Err(err) => Err(format!("`{}` could not be started: {err}", holder.display())),
    }
}

/// How one thing is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Access {
    /// It may be changed.
    Change,
    /// It may be read.
    Read,
    /// It is not there: in its place is nothing, which cannot be written.
    Hidden,
}

#[derive(Clone, Debug)]
struct Given {
    /// Where it is, behind the wall.
    at: PathBuf,
    /// What is shown there, when it is not what is at the same place on
    /// the device.
    from: Option<PathBuf>,
    access: Access,
}

/// What a program behind a wall is shown, besides the system.
///
/// What is said of a folder holds for everything in it that nothing
/// nearer speaks of: a folder that may be changed can hold one that may
/// only be read, and that one a file that is hidden.
#[derive(Clone, Debug, Default)]
pub struct Shown {
    /// The folder it is started in.
    work: PathBuf,
    given: Vec<Given>,
    /// Folders that are never shown whole: the home of whoever runs the
    /// daemon, the factory's own data.
    never: Vec<PathBuf>,
}

/// The path as the system has it, links followed; as written, where there
/// is nothing there to follow.
fn real(path: &Path) -> PathBuf {
    kernel::whole::whole(path).unwrap_or_else(|_| path.to_path_buf())
}

impl Shown {
    /// A view for a program started in `work`, which is given nothing yet.
    pub fn starting_in(work: &Path) -> Shown {
        Shown { work: real(work), given: Vec::new(), never: Vec::new() }
    }

    /// A folder that no entry may show whole: one that is it, or holds
    /// it, is refused when the wall is made.
    pub fn never(&mut self, folder: &Path) {
        self.never.push(real(folder));
    }

    /// It may read this folder or file. One that is not there is left out.
    pub fn read(&mut self, path: &Path) {
        self.give(path, Access::Read);
    }

    /// It may change this folder or file.
    pub fn change(&mut self, path: &Path) {
        self.give(path, Access::Change);
    }

    /// This is not shown, though it is in something that is. A folder
    /// that is not there is hidden all the same, so that none can be made
    /// in its place.
    pub fn hide(&mut self, path: &Path) {
        self.given.push(Given { at: real(path), from: None, access: Access::Hidden });
    }

    /// The file at `path` reads as nothing and cannot be written, whether
    /// or not there is one: for a file that something outside the wall
    /// would act on if it were made (a repository's settings for one of
    /// its worktrees), in a folder that may otherwise be changed.
    pub fn shut(&mut self, path: &Path) {
        if !path.is_dir() {
            self.given.push(Given {
                at: real(path),
                from: Some(PathBuf::from("/dev/null")),
                access: Access::Read,
            });
        }
    }

    /// The folder `own` is shown at `at`, to change: a home of the
    /// program's own, where the home of whoever runs the daemon is.
    pub fn put(&mut self, own: &Path, at: &Path) {
        self.given.push(Given { at: real(at), from: Some(real(own)), access: Access::Change });
    }

    fn give(&mut self, path: &Path, access: Access) {
        if path.exists() {
            self.given.push(Given { at: real(path), from: None, access });
        }
    }

    /// The folders of a path, to read: the programs somebody installed
    /// are there behind a wall as they are in front of it. One that would
    /// show a folder that is never shown whole is left out, as is one that
    /// is another system's.
    pub fn path(&mut self, path: &str) {
        for folder in std::env::split_paths(path) {
            let folder = real(&folder);
            let foreign = FOREIGN.iter().any(|foreign| folder.starts_with(foreign));
            if folder.is_dir() && folder.is_absolute() && !foreign && !self.holds_what_is_never(&folder) {
                self.given.push(Given { at: folder, from: None, access: Access::Read });
            }
        }
    }

    /// The program that is run, to read: the file found for `program`
    /// along `path`, and what a link there leads to. `beside` are folders
    /// of the program's own, written from where the file itself is
    /// (`../..`), for a program that is one file among several.
    pub fn program(&mut self, program: &str, path: &str, beside: &[String]) {
        let named = Path::new(program);
        let found = if named.components().count() > 1 {
            Some(if named.is_absolute() { named.to_path_buf() } else { self.work.join(named) })
        } else {
            std::env::split_paths(path).map(|folder| folder.join(named)).find(|file| file.is_file())
        };
        let Some(found) = found else { return };
        let file = real(&found);
        for folder in beside {
            let folder = real(&file.parent().unwrap_or(Path::new("/")).join(folder));
            if folder.is_dir() && !self.holds_what_is_never(&folder) {
                self.given.push(Given { at: folder, from: None, access: Access::Read });
            }
        }
        if file.is_file() {
            self.given.push(Given { at: file, from: None, access: Access::Read });
        }
    }

    /// In a folder that was added, the files that are a credential by
    /// what they are called are not shown: in it, and in the folders in it.
    pub fn without_credentials(&mut self, folder: &Path) {
        let folder = real(folder);
        let within = std::fs::read_dir(&folder).into_iter().flatten().flatten().map(|entry| entry.path());
        for inside in std::iter::once(folder.clone()).chain(within.filter(|inside| inside.is_dir())) {
            for name in CREDENTIALS {
                let file = inside.join(name);
                // A link is left: what it leads to is shown or not by where
                // that is. So is one that was itself named.
                let named = self.given.iter().any(|given| given.at == file);
                if file.is_file() && !file.is_symlink() && !named {
                    self.given.push(Given { at: file, from: None, access: Access::Hidden });
                }
            }
        }
    }

    fn holds_what_is_never(&self, folder: &Path) -> bool {
        folder == Path::new("/") || self.never.iter().any(|never| never.starts_with(folder))
    }

    /// The folder the program is started in.
    pub fn work(&self) -> &Path {
        &self.work
    }
}

impl Walls {
    /// What to start in place of a command so that it runs behind a wall
    /// showing `shown`: the program that holds the wall and what it is
    /// told, after which the command and its arguments follow as they are.
    /// The sentence says why there can be no such wall.
    pub fn behind(&self, shown: &Shown) -> Result<Vec<OsString>, String> {
        let Walls::Held(holder) = self else {
            return Err("this factory was started holding no walls".into());
        };
        self.can()?;
        for given in &shown.given {
            if given.from.is_none() && given.access != Access::Hidden && shown.holds_what_is_never(&given.at)
            {
                return Err(format!(
                    "`{}` cannot be shown behind a wall: it is, or it holds, a folder that is never shown whole (the home of whoever runs the daemon, the factory's own data)",
                    given.at.display()
                ));
            }
        }
        let mut args: Vec<OsString> = vec![holder.into()];
        let mut say = |words: &[&str]| args.extend(words.iter().map(OsString::from));
        // A view of its own of processes and names, and of nothing else the
        // system shares: it cannot see or signal what runs beside it. The
        // network is the device's.
        say(&["--unshare-user", "--unshare-pid", "--unshare-ipc", "--unshare-uts", "--unshare-cgroup-try"]);
        // Somewhere of its own to put what it does not keep, which is gone with it.
        say(&["--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp", "--tmpfs", "/var/tmp"]);
        let mut all: Vec<Given> = SYSTEM
            .iter()
            .map(Path::new)
            .filter(|folder| folder.exists())
            .map(|folder| Given { at: folder.to_path_buf(), from: None, access: Access::Read })
            .collect();
        // How names are looked up is a file of the system's that may be
        // kept somewhere else, and is then a link out of what is shown.
        let lookup = real(Path::new("/etc/resolv.conf"));
        if lookup.is_file() && !lookup.starts_with("/etc") {
            all.push(Given { at: lookup, from: None, access: Access::Read });
        }
        all.extend(shown.given.iter().cloned());
        // The nearer is said later, and so is what holds: within one
        // folder, the stricter.
        all.sort_by_key(|given| (given.at.components().count(), given.access));
        for given in all {
            let from = given.from.as_deref().unwrap_or(&given.at);
            match given.access {
                Access::Change => args.extend(["--bind".into(), from.into(), given.at.into()]),
                Access::Read => args.extend(["--ro-bind".into(), from.into(), given.at.into()]),
                Access::Hidden if given.at.is_file() => {
                    args.extend(["--ro-bind".into(), "/dev/null".into(), given.at.into()]);
                }
                Access::Hidden => {
                    args.extend(["--tmpfs".into(), given.at.clone().into()]);
                    args.extend(["--remount-ro".into(), given.at.into()]);
                }
            }
        }
        args.extend(["--chdir".into(), shown.work.clone().into(), "--".into()]);
        Ok(args)
    }
}
