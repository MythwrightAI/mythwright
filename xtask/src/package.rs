//! What a package is: a folder someone unpacks and runs.
//!
//! A package holds everything the product needs and nothing has to be
//! built, installed or fetched to use it: the daemon under the command's
//! name, the window under the product's, the client, the packages that
//! come with it, the licences, the notices of other people's libraries it
//! is built from (`notices.rs`), a line saying which revision it is, and a
//! page of its own on how to start it. A program in it is a copy like an
//! installed one (`crates/folders/src/place.rs`): it reads nothing from any
//! checkout, and what is built or edited afterwards never reaches it.
//!
//! There is one for each system the product is written for, and each is
//! that system's own: a `.tar.gz` for Linux and for macOS, a `.zip` for
//! Windows, whose programs end in `.exe`; and a page whose every command
//! and path is that system's. Only the one for Linux is of a system the
//! product is tested on, and each page says what has been run where.
//!
//! This half is what can be asked as a function: what a package is called
//! on a system, what its programs are called there and where cargo put
//! them, which of the client's files belong in one, what makes the one
//! file of it, and what its page says. The making of one is in the program
//! (`packing.rs`).

/// The profile a package is built with: what is released, not what is
/// worked with.
pub const PROFILE: &str = "release";

/// A system there is a package for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum System {
    Linux,
    Macos,
    Windows,
}

impl System {
    /// The system a word names, as `std::env::consts::OS` and a package's
    /// own name say one.
    pub fn of(word: &str) -> Option<System> {
        match word {
            "linux" => Some(System::Linux),
            "macos" => Some(System::Macos),
            "windows" => Some(System::Windows),
            _ => None,
        }
    }

    /// The system in a package's name.
    pub fn word(self) -> &'static str {
        match self {
            System::Linux => "linux",
            System::Macos => "macos",
            System::Windows => "windows",
        }
    }

    /// The system as people write it.
    pub fn called(self) -> &'static str {
        match self {
            System::Linux => "Linux",
            System::Macos => "macOS",
            System::Windows => "Windows",
        }
    }

    /// What the file of a program called `name` is called on this system.
    pub fn program(self, name: &str) -> String {
        match self {
            System::Windows => format!("{name}.exe"),
            System::Linux | System::Macos => name.to_string(),
        }
    }

    /// How the one file of a package ends: what the system opens by
    /// itself. Windows opens a zip and nothing else.
    pub fn archive(self) -> &'static str {
        match self {
            System::Windows => ".zip",
            System::Linux | System::Macos => ".tar.gz",
        }
    }
}

/// The system a package is made on or for, by the word
/// `std::env::consts::OS` has for one, or that there is no package for it.
pub fn on(system: &str) -> Result<System, String> {
    System::of(system).ok_or_else(|| {
        format!(
            "a package is made on and for Linux, macOS and Windows, and this is {system}. On this machine `cargo xtask install` builds it and puts a copy where it is used"
        )
    })
}

/// What a package for a system and a kind of processor is called. A
/// pinned release's says its version; a nightly's says [`NIGHTLY`] there.
pub fn named(id: &str, version: &str, system: System, processor: &str) -> String {
    format!("{id}-{version}-{}-{processor}", system.word())
}

/// What a nightly's package says in its name where a pinned release's
/// says its version: `sample-nightly-linux-x86_64`. The `nightly` release
/// is one release whose files are replaced each night, so they are
/// called the same every night, and a link to one leads to the latest;
/// which night and which commit it is, its version says inside it.
pub const NIGHTLY: &str = "nightly";

/// The day a nightly is of, as it is said to `--nightly` and written in
/// its version (`20261124`, the day in UTC), or why what was said is not
/// one. The workflow says one day to the packages of every system, so a
/// night's three are one version whichever finished after midnight.
pub fn nightly_day(said: &str) -> Result<&str, String> {
    let refused = || {
        format!(
            "`--nightly` is followed by the day the nightly is of, as YYYYMMDD in UTC (`20261124`), and `{said}` is not one"
        )
    };
    if said.len() != 8 || !said.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(refused());
    }
    let number = |from: usize, to: usize| said[from..to].parse::<u32>().unwrap_or(0);
    let (year, month, day) = (number(0, 4), number(4, 6), number(6, 8));
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return Err(refused()),
    };
    if year < 2000 || day == 0 || day > days {
        return Err(refused());
    }
    Ok(said)
}

/// What a nightly of `version` calls itself: the version, then that it is
/// a nightly of a day, then the commit it was built from, as semantic
/// versioning writes a pre-release and what a build was made of
/// (`0.1.0-nightly.20261124+abc1234`). It is no version of its own: the
/// program is still `version`, and the notes of releases have no heading
/// for a night (the daemon says this where it says its version, read
/// from what the package says of itself, `note`).
pub fn nightly_version(version: &str, day: &str, commit: &str) -> String {
    format!("{version}-nightly.{day}+{commit}")
}

/// The system and the kind of processor a target is, as cargo names
/// targets (`x86_64-pc-windows-gnu`, `aarch64-apple-darwin`): the processor
/// is the first part, and the system one of those after it.
pub fn target(triple: &str) -> Result<(System, String), String> {
    let mut parts = triple.split('-');
    let processor = parts.next().filter(|processor| !processor.is_empty());
    let system = parts.find_map(|part| match part {
        "linux" => Some(System::Linux),
        "darwin" => Some(System::Macos),
        "windows" => Some(System::Windows),
        _ => None,
    });
    match (processor, system) {
        (Some(processor), Some(system)) => Ok((system, processor.to_string())),
        _ => Err(format!(
            "`{triple}` does not say a system there is a package for: a target is written as cargo writes one, such as x86_64-pc-windows-gnu, aarch64-apple-darwin or x86_64-unknown-linux-gnu"
        )),
    }
}

/// Where cargo puts what it builds, from the top of the repository, with
/// `/` between the parts: a folder for the profile, inside one for the
/// target where a target was said. Cargo calls the profile `dev` and its
/// folder `debug`.
pub fn built_in(target: Option<&str>, profile: &str) -> String {
    let folder = if profile == "dev" { "debug" } else { profile };
    match target {
        Some(target) => format!("target/{target}/{folder}"),
        None => format!("target/{folder}"),
    }
}

/// What cargo is asked, to build the package `krate` as it is packaged:
/// `word` is `build`, or what stands in for it where another program does
/// the building for a system this is not (`zigbuild`).
pub fn build(word: &str, profile: &str, target: Option<&str>, krate: &str) -> Vec<String> {
    let mut asked: Vec<String> = [word, "--profile", profile, "-p", krate].map(str::to_string).to_vec();
    if let Some(target) = target {
        asked.extend(["--target".to_string(), target.to_string()]);
    }
    asked
}

/// Everything a `tar` that is not told to go into folders has to be named
/// to hold the folder `name` with `files` in it: the folder, and each
/// folder inside it before the first thing it holds, in the order the
/// files are given. That order is then the archive's, whatever order the
/// system would have listed them in.
pub fn listed(name: &str, files: &[String]) -> Vec<String> {
    let mut all = vec![name.to_string()];
    for file in files {
        let mut folder = name.to_string();
        let mut parts = file.split('/').peekable();
        while let Some(part) = parts.next() {
            folder = format!("{folder}/{part}");
            // A folder is named once; a file is whatever is last.
            if parts.peek().is_none() || !all.contains(&folder) {
                all.push(folder.clone());
            }
        }
    }
    all
}

/// What the `tar` of the system a package is made on is asked, to make
/// `archive` of the folder `name`, or why that system makes no such file.
///
/// What is asked for is one file that is the same whoever makes it: the
/// files are nobody's in particular (owner and group nought, and no
/// names), and in the same order every time. Linux's `tar` is GNU's and
/// sorts by name itself. macOS's is another program (bsdtar), which
/// cannot sort and says the owner otherwise: it is told not to go into
/// folders and is handed `list`, a file naming everything in the order
/// wanted (`listed`).
pub fn tar(made_on: System, archive: &str, name: &str, list: &str) -> Result<Vec<String>, String> {
    let asked: &[&str] = match made_on {
        System::Linux => &[
            "--create",
            "--gzip",
            "--owner=0",
            "--group=0",
            "--numeric-owner",
            "--sort=name",
            "--file",
            archive,
            name,
        ],
        System::Macos => &[
            "-c",
            "-z",
            "--uid",
            "0",
            "--gid",
            "0",
            "--numeric-owner",
            "--no-recursion",
            "-f",
            archive,
            "-T",
            list,
        ],
        System::Windows => {
            return Err(format!(
                "a package for {} or {} is not made on Windows: a program in one has to say that it can be run, and a file on Windows cannot say so. Make it on the system it is for; what is made here is the package for Windows",
                System::Linux.called(),
                System::Macos.called()
            ));
        }
    };
    Ok(asked.iter().map(|word| word.to_string()).collect())
}

/// The one line a package says of itself beside its programs, which the
/// daemon reads and says as it starts: for a nightly first its version
/// and that it is one (`0.1.0-nightly.20261124+abc1234, a nightly`), which
/// is what the daemon then says its version is (`place::version_in` of
/// `folders`, held to this line by a case of the API suite); then the
/// revision, and for a package built on another system than it is for,
/// that it is.
pub fn note(
    revision: Option<&str>,
    elsewhere: Option<(System, System)>,
    nightly: Option<&str>,
) -> Option<String> {
    let nightly = nightly.map(|version| format!("{version}, a nightly"));
    let elsewhere = elsewhere.map(|(on, system)| {
        format!("built on {} for {}, for trying and never a release", on.called(), system.called())
    });
    let said: Vec<String> =
        [nightly, revision.map(str::to_string), elsewhere].into_iter().flatten().collect();
    (!said.is_empty()).then(|| said.join(", "))
}

/// The version every crate here has: `version` under `[workspace.package]`
/// in the root manifest.
pub fn version(manifest: &str) -> Option<String> {
    let mut inside = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line == "[workspace.package]";
        } else if inside && let Some(rest) = line.strip_prefix("version") {
            let value = rest.trim_start().strip_prefix('=')?.trim();
            return Some(value.trim_matches('"').to_string()).filter(|version| !version.is_empty());
        }
    }
    None
}

/// The build an entry page is of, as the client's build writes it there.
pub fn build_of(entry: &str) -> Option<&str> {
    let marker = "name=\"build\" content=\"";
    let rest = &entry[entry.find(marker)? + marker.len()..];
    Some(&rest[..rest.find('"')?])
}

/// Which of the client's files a package holds, of all that its build left.
///
/// The client's folder keeps the files of the builds before the last one,
/// for pages that are still open on one of them. Nobody has a package's client open
/// before it is unpacked, so a package takes the one build its entry page
/// is of: the files `builds.json` lists for it, and whatever is not a
/// build's at all (the entry page, what is under `lib/`). Neither that
/// list nor what the build says it was built from (`built-from.json`,
/// which a check of the client reads in the checkout) is for anybody who
/// unpacks it. Where the list cannot be read, it takes everything, which
/// is too much and never too little.
pub fn client_files(all: &[String], entry: &str, builds: Option<&str>) -> Vec<String> {
    let wanted = || -> Option<Vec<String>> {
        let build = build_of(entry)?;
        let builds = builds?;
        let listed = &builds[builds.find(&format!("\"id\":\"{build}\""))?..];
        let files = &listed[listed.find("\"files\":[")? + "\"files\":[".len()..];
        let files = &files[..files.find(']')?];
        Some(files.split(',').map(|file| file.trim().trim_matches('"').to_string()).collect())
    };
    let Some(wanted) = wanted() else { return all.to_vec() };
    all.iter()
        .filter(|file| {
            !["builds.json", "built-from.json"].contains(&file.as_str())
                && (!file.starts_with("assets/") || wanted.iter().any(|one| one == *file))
        })
        .cloned()
        .collect()
}

/// What a package's own page is written from.
pub struct About<'a> {
    /// The product's name as people read it.
    pub display: &'a str,
    /// The command people type.
    pub cli: &'a str,
    /// The product's name in folders, and the window's program.
    pub id: &'a str,
    pub version: &'a str,
    /// The system the package is for.
    pub system: System,
    pub processor: &'a str,
    /// What `install` would say of the checkout: `revision abc1234`.
    pub revision: Option<&'a str>,
    /// The system it was built on, and its C library where it says one.
    pub built_on: &'a str,
    /// The system it was built on, where that is another than it is for:
    /// such a package is for trying, and says so.
    pub elsewhere: Option<System>,
    /// Whether the window's program is in it.
    pub window: bool,
    /// Whether it is a nightly, built from the main line: then `version`
    /// is what a nightly calls itself (`nightly_version`).
    pub nightly: bool,
    /// Where the daemon answers unless told otherwise.
    pub address: &'a str,
}

/// What the notes of the first run are called in a package.
pub const NOTES: &str = "FIRST-RUN.md";

/// The notes of the first run as a package carries them: the source's own
/// text (`docs/first-run.md`), under a head that says what it is and how
/// its commands are typed in this folder on this system, and with the
/// links to pages a package does not hold taken out, their words left.
///
/// A package had its own page and nothing of the walk: what a project
/// starts with, a card taken by an agent, two ways compared, a second
/// device were in the repository alone, which somebody who downloaded a
/// package has not got. The text is not rewritten for a system: what it
/// says of paths and of a shell is said as the source says it, and the
/// head says so where the system is not the one it was written on.
pub fn notes(text: &str, about: &About) -> String {
    let About { display, cli, system, .. } = about;
    let command = match system {
        System::Linux | System::Macos => format!("./{cli}"),
        System::Windows => format!(".\\{}", System::Windows.program(cli)),
    };
    let elsewhere = match system {
        System::Linux => String::new(),
        System::Macos => " It was written on Linux, the system it is tested on.".to_string(),
        System::Windows => {
            " It was written on Linux, the system it is tested on: where it writes a path or a \
                            line of a shell, that is Linux's, and `README.md` beside this has this system's."
                .to_string()
        }
    };
    // The block of settings at its head (`public: true`, which the pages
    // the program writes out of itself go by) is about the file, and is
    // no part of what is read.
    let text = text
        .strip_prefix("---\n")
        .and_then(|after| after.split_once("\n---\n"))
        .map_or(text, |(_, rest)| rest.trim_start_matches('\n'));
    let body = text.strip_prefix("# The first run\n").unwrap_or(text).trim_start_matches('\n');
    format!(
        "# The first run\n\n> The notes of the first run from the source this package of {display} was built\n\
         > from (`docs/first-run.md`), as they stand there. A command is written `{cli} ...`: in this\n\
         > folder that is `{command} ...`. The other pages they name are in the source's `docs/` and\n\
         > not in this folder; `{command} help` lists the topics that say most of each.{elsewhere}\n\n{}",
        unlinked(body)
    )
}

/// A text with its links to other files taken out and their words left: a
/// link is `[words](where)`, and one that leads to an address or to a
/// place in the same page is left as it is.
fn unlinked(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(middle) = rest.find("](") {
        let (Some(open), Some(shut)) = (rest[..middle].rfind('['), rest[middle..].find(')')) else {
            out.push_str(&rest[..middle + 2]);
            rest = &rest[middle + 2..];
            continue;
        };
        let (words, to) = (&rest[open + 1..middle], &rest[middle + 2..middle + shut]);
        let stays = to.starts_with('#') || to.contains("://") || to.starts_with("mailto:");
        out.push_str(&rest[..open]);
        out.push_str(if stays { &rest[open..=middle + shut] } else { words });
        rest = &rest[middle + shut + 1..];
    }
    out.push_str(rest);
    out
}

/// The page that goes in a package: how to start it, what it needs of the
/// system, where it keeps things, how to be rid of it, and what it was
/// built on. Every command, path and thing needed that it names is the
/// system's the package is for, and it says of each system only what has
/// been run there.
pub fn readme(about: &About) -> String {
    match about.system {
        System::Linux => linux(about),
        System::Macos => macos(about),
        System::Windows => windows(about),
    }
}

/// What a package says of itself where it is not what a release holds:
/// built on another system than it is for, or made without the window's
/// program. Nothing, for a package as it is released.
fn aside(about: &About) -> String {
    let About { display, id, system, elsewhere, window, .. } = about;
    let mut said = Vec::new();
    if let Some(on) = elsewhere {
        said.push(format!(
            "It was built on {} for {}, by a compiler made to build for another system: it is for trying {display} on {}, and is never what is released.",
            on.called(),
            system.called(),
            system.called()
        ));
    }
    if !window {
        let program = system.program(id);
        said.push(match system {
            System::Linux => format!(
                "It was made without the window's program: there is no `{program}` and no `menu.sh` in this folder, and what this page says of the window and of the desktop's menu is not so of it."
            ),
            System::Macos | System::Windows => {
                format!("It was made without the window's program: there is no `{program}` in this folder.")
            }
        });
    }
    if said.is_empty() {
        String::new()
    } else {
        format!("**This one is not a release.** {}\n\n", said.join(" "))
    }
}

/// The sentence that says which revision a package is, or nothing; and
/// for a nightly, what a nightly is.
fn revision_said(about: &About) -> String {
    let revision = about.revision.map(|revision| format!(" This is {revision}.")).unwrap_or_default();
    if !about.nightly {
        return revision;
    }
    format!(
        "{revision} It is a nightly: built from the main line on the day its version says, once the gate passed on that commit. It changes only when you replace it, and a pinned release is the other line; the README of the repository says how to move from one to the other, and what moving back takes."
    )
}

fn linux(about: &About) -> String {
    let notes = NOTES;
    let About { display, cli, id, version, processor, built_on, address, .. } = about;
    let revision = revision_said(about);
    let aside = aside(about);
    format!(
        r#"# {display} {version} for Linux ({processor})

**A preview.** It is tested on Linux, and it is early: what it keeps, and
the blueprints written for it, are not promised to carry over to the next
version unchanged.

{aside}Everything it needs is in this folder. Nothing is built, nothing is
installed, and nothing is fetched: it sends nothing anywhere by itself.

## Start it

In a window of its own:

```sh
./{id}
```

The window starts the daemon if it is not running, and the daemon goes on
running when the window is closed. Or without the window, the daemon and a
browser:

```sh
./{cli} up &      # start the daemon and keep it running
./{cli} open      # the client, in your browser: {address}
./{cli} down      # stop it; what it runs keeps running
```

Nobody is registered at first, so the client opens on **Register**. The
first account owns the installation, and is made on this machine.
`./{cli} help` lists everything the command does; the client's **Setup
Guide** walks through the rest.

`{notes}`, in this folder, is the longer walk: the first hour, a card taken
by an agent, two ways of doing something counted side by side, and a second
device. It writes each command as `{cli} ...`, which here is `./{cli} ...`.

## What the window needs

The system's own web view, WebKitGTK 4.1. Most desktops have it; where the
window does not start, it is one package:

| System | Package |
|---|---|
| Debian, Ubuntu | `libwebkit2gtk-4.1-0` |
| Fedora | `webkit2gtk4.1` |
| Arch | `webkit2gtk-4.1` |

The daemon and the command need nothing beyond the system's C library.

## In the desktop's menu

```sh
./menu.sh           # an entry that starts the window, and its icon
./menu.sh remove    # take both away again
```

Both are files under `~/.local/share` (`$XDG_DATA_HOME`). An entry names
the program by where it is, so run it again after moving the folder.

## The command on your path

Run this in the folder:

```sh
mkdir -p ~/.local/bin && ln -sf "$PWD/{cli}" ~/.local/bin/{cli}
```

`{cli}` is then a link to the program here, and the folder stays where it
is. To move the folder, move it and make the link again.

## Where it keeps things

| What | Where |
|---|---|
| Everything a factory holds: its journal, projects, agents | `~/.local/share/{id}` (`$XDG_DATA_HOME/{id}` where that is set) |
| A factory started with `--profile <name>` | the same with `-<name>` after it |
| Which daemon the command is signed in to | `~/.config/{id}` (`$XDG_CONFIG_HOME/{id}`) |

This folder holds no data: unpacking a newer package beside it and
starting that one is an upgrade, with the data where it was.

## Removing it

```sh
./{cli} down                                  # the daemon
rm -rf ~/.local/share/{id}* ~/.config/{id}    # what it kept (all of it)
rm -f ~/.local/bin/{cli}                      # the link, if you made one
./menu.sh remove                              # the menu entry, if you added one
```

and delete this folder. What the daemon was hosting keeps running when it
stops, by design: stop those first (`./{cli} instances`, `./{cli} stop <id>`).

## What this is

Built on {built_on}.{revision} A program starts on a system whose C library
is at least as new as the one it was built on; on an older one it says so
and does not start.

Offered under either of two licences, as whoever uses it chooses: MIT
(`LICENSE-MIT`) or Apache 2.0 (`LICENSE-APACHE`). Its programs and its
client are built from other people's libraries as well:
`THIRD-PARTY-NOTICES.md` names each, with the licence it is handed on under
and that licence's text.
"#
    )
}

/// Why an agent's program has to be let work in the open on a system that
/// holds no wall, in that system's own commands.
fn no_wall(system: System, command: &str) -> String {
    let fence = if system == System::Windows { "```" } else { "```sh" };
    format!(
        r#"## An agent's program has no wall here

On Linux an agent's program is started behind a wall, and of the machine it
is shown its work and little else. Nothing holds a wall on {called}, and an
agent that is to have one is not started, and says so. An agent's program
works here once it is let work in the open:

{fence}
{command} up --walls off
```

starts a factory that holds no walls, and `{command} agent add <name> --no-wall`
says it of one agent. Such a program sees this device as whoever runs the
daemon does: your files, your keys and your sign-ins. It is work you would
give a person with your login.
"#,
        called = system.called()
    )
}

fn macos(about: &About) -> String {
    let notes = NOTES;
    let About { display, cli, id, version, processor, built_on, address, window, .. } = about;
    let revision = revision_said(about);
    let aside = aside(about);
    let no_wall = no_wall(System::Macos, &format!("./{cli}"));
    let the_window = if *window {
        format!(
            r#"
`./{id}` is the window: written to open the client in a window of its own,
and to start the daemon if it is not running, with the system's own web
view and nothing more. It is a plain program started from a terminal and
not an application: there is no bundle for the Finder, the Dock or
Launchpad, and no icon of its own. It has not been run.
"#
        )
    } else {
        String::new()
    };
    let made_for = match *processor {
        "aarch64" => {
            " It is for a Mac with Apple silicon, and does not start on one with an Intel processor."
        }
        "x86_64" => " It is for a Mac with an Intel processor.",
        _ => "",
    };
    format!(
        r#"# {display} {version} for macOS ({processor})

**A preview, and on macOS one nobody has run.** Its tests are run on Linux.
On macOS nothing of it has been run: not the daemon, not the command, not
the window, and no test. It is written to work there, and whoever starts it
is the first to find out. What it keeps, and the blueprints written for it,
are not promised to carry over to the next version unchanged.

{aside}Everything it needs is in this folder, and there is no installer and no
application to drag anywhere: nothing is built, nothing is installed, and
nothing is fetched. It sends nothing anywhere by itself.

## macOS will ask first

The programs here are not signed by a developer Apple knows, and are not
notarised. A file that came through a browser is marked by macOS as come
from elsewhere, and a program so marked is refused when it is started. In
Terminal, in this folder, this takes the mark off everything here:

```sh
xattr -dr com.apple.quarantine .
```

Or start a program, let macOS refuse it, and allow it under **System
Settings**, **Privacy & Security**, **Open Anyway**; each program is asked
about once. That is how macOS treats any download of the kind, and it has
not been tried with this one.

## Start it

In Terminal, in this folder:

```sh
./{cli} up &      # start the daemon and keep it running
./{cli} open      # the client, in your browser: {address}
./{cli} down      # stop it; what it runs keeps running
```

Nobody is registered at first, so the client opens on **Register**. The
first account owns the installation, and is made on this machine.
`./{cli} help` lists everything the command does; the client's **Setup
Guide** walks through the rest.

`{notes}`, in this folder, is the longer walk: the first hour, a card taken
by an agent, two ways of doing something counted side by side, and a second
device. It writes each command as `{cli} ...`, which here is `./{cli} ...`.
{the_window}
## What it needs

`git`, for a fleet to work cards: Apple's command line tools bring it
(`xcode-select --install`). Nothing else.

{no_wall}
## The command on your path

Run this in the folder:

```sh
mkdir -p ~/.local/bin && ln -sf "$PWD/{cli}" ~/.local/bin/{cli}
```

`{cli}` is then a link to the program here, and the folder stays where it
is. macOS does not look in `~/.local/bin` for commands until it is told to:
the line `export PATH="$HOME/.local/bin:$PATH"` in `~/.zprofile` tells it,
for every terminal opened afterwards. To move the folder, move it and make
the link again.

## Where it keeps things

| What | Where |
|---|---|
| Everything a factory holds: its journal, projects, agents | `~/.local/share/{id}` (`$XDG_DATA_HOME/{id}` where that is set) |
| A factory started with `--profile <name>` | the same with `-<name>` after it |
| Which daemon the command is signed in to | `~/.config/{id}` (`$XDG_CONFIG_HOME/{id}`) |

This folder holds no data: unpacking a newer package beside it and
starting that one is an upgrade, with the data where it was.

## Removing it

```sh
./{cli} down                                  # the daemon
rm -rf ~/.local/share/{id}* ~/.config/{id}    # what it kept (all of it)
rm -f ~/.local/bin/{cli}                      # the link, if you made one
```

and delete this folder. What the daemon was hosting keeps running when it
stops, by design: stop those first (`./{cli} instances`, `./{cli} stop <id>`).

## What this is

Built on {built_on}.{revision}{made_for} Which older versions of macOS it
starts on has not been tried. Not signed, and not notarised.

Offered under either of two licences, as whoever uses it chooses: MIT
(`LICENSE-MIT`) or Apache 2.0 (`LICENSE-APACHE`). Its programs and its
client are built from other people's libraries as well:
`THIRD-PARTY-NOTICES.md` names each, with the licence it is handed on under
and that licence's text.
"#
    )
}

fn windows(about: &About) -> String {
    let notes = NOTES;
    let About { display, cli, id, version, processor, built_on, address, elsewhere, window, .. } = about;
    let revision = revision_said(about);
    let aside = aside(about);
    let command = format!(".\\{}", System::Windows.program(cli));
    let program = System::Windows.program(id);
    let no_wall = no_wall(System::Windows, &command);
    let the_window = if *window {
        format!(
            r#"
`{program}` is the window: written to open the client in a window of its
own, and to start the daemon if it is not running. On Windows it has been
run only where it opens no window: asked what it takes (`{program} --help`),
and told to start nothing at an address where nothing answered, which it
said before it ended. Started with a double click it opens a console beside
its window, and closing the console closes the window.

For a daemon that runs in WSL, which passes the port it listens at through
to Windows, the window is told its address and to start none on Windows:

```
.\{program} --url {address} --no-start
```
"#
        )
    } else {
        String::new()
    };
    let window_needs = if *window {
        "| The window | Microsoft's WebView2 runtime, which comes with Windows 11 |\n"
    } else {
        ""
    };
    // A program built on Windows asks for the C runtime of the compiler
    // that built it; one built for Windows on another system asks only
    // for what Windows itself has.
    let programs_need = if elsewhere.is_none() {
        "| The programs themselves | Microsoft's Visual C++ runtime, which most machines have: where a program says `VCRUNTIME140.dll` was not found, the *Visual C++ Redistributable* brings it |\n"
    } else {
        ""
    };
    format!(
        r#"# {display} {version} for Windows ({processor})

**A preview, and on Windows an early one.** Its tests are run on Linux on
every change. On Windows they have been run by hand, on one machine, and by
nothing that runs on every change. There it has been started by hand, once, on
one machine: a daemon, a project, and a card worked through a fleet with
stand-ins for agents' programs. The window's program has been run there
only where it opens no window. What it keeps, and the blueprints written for
it, are not promised to carry over to the next version unchanged.

{aside}Everything it needs is in this folder, and there is no installer: nothing
is built, nothing is installed, nothing is put in the Start menu or on your
path, and nothing is fetched. It sends nothing anywhere by itself.

## Windows will ask first

The programs here are not signed. The first time one is started, Windows
may say **Windows protected your PC**: choose **More info**, and then
**Run anyway**.

## Start it

In a terminal (PowerShell or the Command Prompt), in this folder:

```
{command} up
```

That starts the daemon and keeps it running, and it stays in that terminal:
leave it open. In a second terminal, in this folder:

```
{command} open
```

opens the client in your browser, at {address}. Nobody is registered at
first, so the client opens on **Register**. The first account owns the
installation, and is made on this machine. `{command} help` lists
everything the command does; the client's **Setup Guide** walks through the
rest. `{command} down`, from any terminal, stops the daemon; what it hosts
keeps running.

`{notes}`, in this folder, is the longer walk: the first hour, a card taken
by an agent, two ways of doing something counted side by side, and a second
device. It writes each command as `{cli} ...`, which here is `{command} ...`.
{the_window}
## What it needs

| For | What |
|---|---|
| A fleet to work cards | Git for Windows, with `git` on your path |
{window_needs}{programs_need}
{no_wall}
## Where it keeps things

| What | Where |
|---|---|
| Everything a factory holds: its journal, projects, agents | `%LOCALAPPDATA%\{id}` |
| A factory started with `--profile <name>` | the same with `-<name>` after it |
| Which daemon the command is signed in to | `%APPDATA%\{id}` |

Where `XDG_DATA_HOME` is set the first is a folder `{id}` there instead, and
where `XDG_CONFIG_HOME` is set the last is. This folder holds no data:
unpacking a newer package beside it and starting that one is an upgrade,
with the data where it was.

## The command on your path

Nothing puts it there. Run it by where it is, as above, or add this folder
to `Path`: search the Start menu for **Edit environment variables for your
account**, and add this folder to `Path` there. A terminal opened
afterwards finds `{cli}`.

## Removing it

In PowerShell, in this folder:

```
{command} down
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\{id}*", "$env:APPDATA\{id}"
```

The second line takes away what it kept, all of it. Then delete this
folder, and take it out of `Path` if you added it. What the daemon was
hosting keeps running when it stops, by design: stop those first
(`{command} instances`, `{command} stop <id>`).

## What this is

Built on {built_on}.{revision} Not signed.

Offered under either of two licences, as whoever uses it chooses: MIT
(`LICENSE-MIT`) or Apache 2.0 (`LICENSE-APACHE`). Its programs and its
client are built from other people's libraries as well:
`THIRD-PARTY-NOTICES.md` names each, with the licence it is handed on under
and that licence's text.
"#
    )
}
