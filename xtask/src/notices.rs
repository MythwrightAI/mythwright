//! The notices of what a package bundles: other people's libraries, each
//! handed on under a licence that asks that its notice and its text go
//! with every copy.
//!
//! They are one file at the top of the repository, which a package carries
//! beside the product's own licences (`packing.rs` copies it and needs no
//! tool). `cargo xtask notices` makes it from nothing, when a dependency
//! changes, and it is committed:
//!
//! - the programs' half from `cargo about`, a tool whoever adds a
//!   dependency has and nobody else needs, which reads every crate the
//!   programs are built from for the three systems and the licence text
//!   each carries, held to the licences `about.toml` accepts, and writes
//!   them as records (`about.hbs`);
//! - the client's half from the client's own build, asked which packages
//!   are in what it serves (`node build.mjs --notices`), as the same
//!   records.
//!
//! This reads the records, puts them in an order that is the same on any
//! machine and lays the file out. It says at its head what it was made
//! from, and what it holds can be read back out of it (`read`), so that a
//! test with no tool holds it to the code: the crates the programs can be
//! built from are those it was made from (`shipped`), each library it
//! names is the version that is locked, and each names a licence and has
//! its text. `cargo xtask notices --check` makes it and says whether what
//! is there is that, writing nothing.
//!
//! Without the tool the programs' half is kept as it is where nothing it
//! was made from has changed, so a change to the client alone needs none.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use crate::shipped::{self, Locked};
use crate::{sha256, tools};

/// The file. Markdown by its name: the gates let a library's name be
/// spelled there (a library may be called what the application may not
/// say), and count none of its lines as source.
pub const FILE: &str = "THIRD-PARTY-NOTICES.md";
/// What the tool goes by, and what it writes by.
pub const CONFIG: &str = "about.toml";
pub const TEMPLATE: &str = "about.hbs";
/// The tool's program, and the one version the file is made with: another
/// may tell a licence's text otherwise.
pub const TOOL: &str = "cargo-about";
pub const VERSION: &str = "0.9.3";
/// How the tool is come by. Its program is built only with that feature.
pub const INSTALL: &str = "cargo install cargo-about --locked --version 0.9.3 --features cli";
/// What makes the file, for a sentence.
pub const COMMAND: &str = "cargo xtask notices";

/// Where each half begins.
pub const CRATES: &str = "## The programs: Rust crates";
pub const PACKAGES: &str = "## The client: packages";
/// What the marks at the head are called.
const MARK_CRATES: &str = "crates";

/// The files a crate may say something more in than its licence: a notice
/// its licence asks to be handed on, whose work it holds beside its own.
const BESIDES: [&str; 9] = [
    "NOTICE",
    "NOTICE.txt",
    "NOTICE.md",
    "COPYRIGHT",
    "COPYRIGHT.txt",
    "COPYRIGHT.md",
    "LICENSE-THIRD-PARTY",
    "LICENSE-THIRD-PARTY.txt",
    "LICENSE-THIRD-PARTY.md",
];

/// How a licence's name begins that asks for more than its notice of
/// whoever hands on a built copy: that the source of what is under it is
/// offered too. None is accepted in general (`about.toml`), and a library
/// that is under one by the maintainer's word says where its source is.
const ASKS_FOR_SOURCE: [&str; 9] =
    ["MPL-", "LGPL-", "GPL-", "AGPL-", "EPL-", "CDDL-", "EUPL-", "CPL-", "OSL-"];

/// What a line that says where a library's source is begins with.
pub const SOURCE: &str = "Source: ";

/// Whether a licence asks that the source of what is under it be offered.
pub fn asks_for_source(licence: &str) -> bool {
    ASKS_FOR_SOURCE.iter().any(|begins| licence.starts_with(begins))
}

/// What is built into a program that is no crate of its own, said at the
/// end of the programs' half. No tool finds these: a crate that builds
/// somebody's code from its source, or carries it built, is read for it
/// when the crate is added.
const BUILT_IN: &str = "\
### Built into the programs with them

- SQLite, the database library, which the crate `libsqlite3-sys` builds from
  its source, so that the programs hold it whole. Its authors have dedicated
  it to the public domain, and it asks nothing: https://www.sqlite.org/copyright.html
- On Windows, the window's program is built with Microsoft's loader for its
  WebView2 runtime, as the crate `webview2-com-sys` carries it. It is
  Microsoft's, under the terms of the WebView2 SDK (the package
  `Microsoft.Web.WebView2`), which that crate does not carry the text of.
";

/// A library as a record names it.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Library {
    pub name: String,
    pub version: String,
    /// Where it is kept, as it says of itself.
    pub place: String,
    /// Who wrote it, as its manifest names them.
    pub by: Vec<String>,
    /// Where its manifest is on this machine: read from, and never written.
    pub manifest: String,
}

/// One text and the libraries it is the text for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Group {
    /// The licence as it is written in a manifest (`MIT`), or for something
    /// a library says besides, nothing.
    pub licence: String,
    /// The licence as people call it.
    pub called: String,
    /// The text is the licence's standard one: the libraries carry no file.
    pub standard: bool,
    /// For something said besides the licence, the file it is said in.
    pub file: String,
    pub libraries: Vec<Library>,
    pub text: String,
}

/// A text as it is kept: no line ends in a space or as another system ends
/// one, and no empty line is before it or after it. What is in it is not
/// touched.
fn kept(text: &str) -> String {
    let lines: Vec<&str> = text.lines().map(|line| line.trim_end()).collect();
    let first = lines.iter().position(|line| !line.is_empty()).unwrap_or(lines.len());
    let last = lines.iter().rposition(|line| !line.is_empty()).map_or(first, |at| at + 1);
    lines[first..last].join("\n")
}

/// The records a half is written from (`about.hbs` says each field), as
/// groups: one for each text, with the libraries under it.
pub fn records(written: &str) -> Result<Vec<Group>, String> {
    let mut groups: Vec<Group> = Vec::new();
    let mut text: Option<Vec<&str>> = None;
    for line in written.lines() {
        if let Some(lines) = &mut text {
            if line.trim_end() == "%%end%%" {
                let group = groups.last_mut().ok_or("a text with no licence before it")?;
                group.text = kept(&lines.join("\n"));
                text = None;
            } else {
                lines.push(line);
            }
            continue;
        }
        let Some(rest) = line.strip_prefix("%%") else { continue };
        let Some((field, value)) = rest.split_once("%%") else { continue };
        let value = value.trim();
        if field == "licence" || field == "besides" {
            groups.push(Group::default());
        }
        let group = groups.last_mut().ok_or_else(|| format!("`{line}` before any licence"))?;
        let library = || format!("`{line}` before any library");
        match field {
            "licence" => group.licence = value.to_string(),
            "besides" => group.file = value.to_string(),
            "name" => group.called = value.to_string(),
            "from" => group.standard = value == "standard",
            "library" => {
                let (name, version) =
                    value.split_once(' ').ok_or_else(|| format!("`{line}` says no version"))?;
                group.libraries.push(Library {
                    name: name.to_string(),
                    version: version.to_string(),
                    ..Library::default()
                });
            }
            "where" => group.libraries.last_mut().ok_or_else(library)?.place = value.to_string(),
            "manifest" => group.libraries.last_mut().ok_or_else(library)?.manifest = value.to_string(),
            "by" => group.libraries.last_mut().ok_or_else(library)?.by.push(value.to_string()),
            "text" => text = Some(Vec::new()),
            _ => return Err(format!("`{line}` is no field of a record")),
        }
    }
    if text.is_some() {
        return Err("the last text has no end".into());
    }
    for group in &groups {
        let of = group.libraries.first().map_or("no library", |library| library.name.as_str());
        if group.libraries.is_empty() || group.text.is_empty() {
            return Err(format!("a record of `{}` ({of}) has no library or no text", group.licence));
        }
        if group.licence.is_empty() && group.file.is_empty() {
            return Err(format!("the record of {of} names no licence"));
        }
    }
    Ok(ordered(groups))
}

/// Groups put so that they are the same whatever order they came in: one
/// for each licence and text, its libraries by name, the licences by name,
/// and under one licence the texts by the first library of each.
fn ordered(groups: Vec<Group>) -> Vec<Group> {
    let mut by_text: BTreeMap<(String, String, String), Group> = BTreeMap::new();
    for group in groups {
        let key = (group.licence.clone(), group.file.clone(), group.text.clone());
        match by_text.get_mut(&key) {
            Some(there) => {
                there.libraries.extend(group.libraries);
                there.standard &= group.standard;
            }
            None => {
                by_text.insert(key, group);
            }
        }
    }
    let mut groups: Vec<Group> = by_text.into_values().collect();
    for group in &mut groups {
        group.libraries.sort();
        group.libraries.dedup();
    }
    groups.sort_by(|a, b| {
        (&a.licence, &a.file, &a.libraries, &a.text).cmp(&(&b.licence, &b.file, &b.libraries, &b.text))
    });
    groups
}

/// What crates say besides their licence: for each, the files of the
/// usual names beside its manifest, as groups.
fn besides(groups: &[Group]) -> Vec<Group> {
    let mut found = Vec::new();
    let libraries: BTreeSet<&Library> = groups.iter().flat_map(|group| &group.libraries).collect();
    for library in libraries {
        let Some(folder) = Path::new(&library.manifest).parent() else { continue };
        for file in BESIDES {
            let Ok(text) = std::fs::read_to_string(folder.join(file)) else { continue };
            let text = kept(&text);
            if !text.is_empty() {
                found.push(Group {
                    file: file.to_string(),
                    libraries: vec![library.clone()],
                    text,
                    ..Group::default()
                });
            }
        }
    }
    ordered(found)
}

/// Which half of the file a library is of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Of {
    Crates,
    Packages,
}

/// Where the source of a library is to be had, said under it where its
/// licence asks that: the library as it was published, at the address the
/// registry it was fetched from has for that version of it.
fn source_said(library: &Library, of: Of) -> String {
    let Library { name, version, .. } = library;
    let (kind, at) = match of {
        Of::Crates => ("crate", format!("https://crates.io/crates/{name}/{version}")),
        Of::Packages => ("package", format!("https://www.npmjs.com/package/{name}/v/{version}")),
    };
    format!(
        "{SOURCE}this is `{name}` {version} as its authors published it, with nothing in it changed here. \
         Its source is that {kind}: {at}"
    )
}

/// A text between marks no line of it is, so that nothing in it is read as
/// anything but itself.
fn fenced(text: &str) -> String {
    let longest = text.split(|ch| ch != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest.max(3) + 1);
    format!("{fence}text\n{text}\n{fence}\n")
}

/// A half of the file as it is laid out: each licence, under it each text
/// with the libraries that text is for, and then what some say besides.
pub fn half(groups: &[Group], of: Of) -> String {
    let mut out = String::new();
    let mut licence = None;
    for group in groups.iter().filter(|group| !group.licence.is_empty()) {
        if licence != Some(&group.licence) {
            licence = Some(&group.licence);
            let called = if group.called.is_empty() || group.called == group.licence {
                String::new()
            } else {
                format!("{} ", group.called)
            };
            out.push_str(&format!("### {called}(`{}`)\n\n", group.licence));
        }
        if group.standard {
            out.push_str(
                "These carry no file of their licence. What follows them is the standard text of the\n\
                 licence their manifests name, and each is the work of those its manifest names.\n\n",
            );
        }
        for library in &group.libraries {
            let place = if library.place.is_empty() { String::new() } else { format!(", {}", library.place) };
            out.push_str(&format!("- `{}` {}{place}\n", library.name, library.version));
            if group.standard && !library.by.is_empty() {
                out.push_str(&format!("  By: {}\n", library.by.join(", ")));
            }
            if asks_for_source(&group.licence) {
                out.push_str(&format!("  {}\n", source_said(library, of)));
            }
        }
        out.push_str(&format!("\n{}\n", fenced(&group.text)));
    }
    let said: Vec<&Group> = groups.iter().filter(|group| group.licence.is_empty()).collect();
    if !said.is_empty() {
        out.push_str("### What some of them say besides their licence\n\n");
    }
    for group in said {
        let of: Vec<String> =
            group.libraries.iter().map(|library| format!("`{}` {}", library.name, library.version)).collect();
        out.push_str(&format!(
            "In the file `{}` of {}:\n\n{}\n",
            group.file,
            of.join(", "),
            fenced(&group.text)
        ));
    }
    out
}

/// A library the file names, as it is read back out of it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Named {
    pub name: String,
    pub version: String,
    pub licence: String,
    /// What is said under it: who it is by, where its source is.
    pub said: Vec<String>,
    /// How long the text it is under is.
    pub text: usize,
}

/// What the file holds, read back out of it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Read {
    /// What it says at its head it was made from, each by its name.
    pub marks: Vec<(String, String)>,
    pub crates: Vec<Named>,
    pub packages: Vec<Named>,
    /// Each half as it is written, from its heading to the next.
    pub crates_half: String,
    pub packages_half: String,
}

/// Reads a notices file. Nothing between the marks of a text is read as
/// anything: a licence may have a line that looks like a heading.
pub fn read(file: &str) -> Read {
    #[derive(PartialEq)]
    enum Part {
        Head,
        Crates,
        Packages,
    }
    let mut read = Read::default();
    let mut part = Part::Head;
    let mut licence: Option<String> = None;
    let mut fence: Option<String> = None;
    // The libraries named since the last text: the next text is theirs.
    let mut waiting = 0;
    let mut length = 0;
    for line in file.lines() {
        let half = match part {
            Part::Head => None,
            Part::Crates => Some((&mut read.crates_half, &mut read.crates)),
            Part::Packages => Some((&mut read.packages_half, &mut read.packages)),
        };
        if let Some(end) = &fence {
            if let Some((written, named)) = half {
                written.push_str(line);
                written.push('\n');
                if line == end {
                    let from = named.len() - waiting.min(named.len());
                    named[from..].iter_mut().for_each(|one| one.text = length);
                    waiting = 0;
                }
            }
            if line == end {
                fence = None;
            } else {
                length += line.len() + 1;
            }
            continue;
        }
        if line == CRATES || line == PACKAGES {
            part = if line == CRATES { Part::Crates } else { Part::Packages };
            (licence, waiting) = (None, 0);
            continue;
        }
        if line.starts_with("````") && line.ends_with("text") {
            fence = Some(line.trim_end_matches("text").to_string());
            length = 0;
        }
        let Some((written, named)) = half else {
            // At the head: `  <what>  <its mark>`.
            let mut words = line.split_whitespace();
            if let (true, Some(what), Some(mark), None) =
                (line.starts_with("  "), words.next(), words.next(), words.next())
                && mark.len() == 64
                && mark.chars().all(|ch| ch.is_ascii_hexdigit())
            {
                read.marks.push((what.to_string(), mark.to_string()));
            }
            continue;
        };
        written.push_str(line);
        written.push('\n');
        if let Some(heading) = line.strip_prefix("### ") {
            // A licence's heading ends in its name as a manifest writes it.
            licence = heading
                .strip_suffix("`)")
                .and_then(|heading| heading.rsplit_once("(`"))
                .map(|(_, licence)| licence.to_string());
            waiting = 0;
        } else if let (Some(licence), Some(rest)) = (&licence, line.strip_prefix("- `")) {
            let (name, after) = rest.split_once('`').unwrap_or((rest, ""));
            let version = after.split_whitespace().next().unwrap_or_default().trim_end_matches(',');
            named.push(Named {
                name: name.to_string(),
                version: version.to_string(),
                licence: licence.clone(),
                ..Named::default()
            });
            waiting += 1;
        } else if waiting > 0
            && let Some(said) = line.strip_prefix("  ")
            && let Some(last) = named.last_mut()
        {
            last.said.push(said.to_string());
        }
    }
    read
}

/// How many libraries there are, each counted once however often it is
/// named.
pub fn libraries(named: &[Named]) -> usize {
    named.iter().map(|one| (&one.name, &one.version)).collect::<BTreeSet<_>>().len()
}

/// How many libraries are under each licence, for the head of the file.
fn counted(named: &[Named]) -> String {
    let mut under: BTreeMap<&str, BTreeSet<(&str, &str)>> = BTreeMap::new();
    for one in named {
        under.entry(&one.licence).or_default().insert((&one.name, &one.version));
    }
    under.iter().map(|(licence, those)| format!("- `{licence}`: {}\n", those.len())).collect()
}

/// The whole file, from what it was made from and its two halves as they
/// are laid out.
pub fn whole(marks: &[(String, String)], crates: &str, packages: &str) -> String {
    let body = format!("{CRATES}\n{crates}{PACKAGES}\n{packages}");
    let body = body.trim_end();
    let named = read(body);
    let marks: String = marks.iter().map(|(what, mark)| format!("  {what}  {mark}\n")).collect();
    format!(
        "# Third-party notices

What is in this package was not all written for it. Its programs are built
from other people's Rust crates and its client from other people's packages,
each handed on under a licence that asks that its notice and its text go
with every copy. This file is those: every such library by name and version,
under the licence it is taken under, with the text of that licence as the
library itself carries it.

The package's own licences are `LICENSE-MIT` and `LICENSE-APACHE`, beside
this file, and nothing here is about them.

<!--
Made by `{COMMAND}` and never by hand. What it was made from:
{marks}A test of the tooling's fails where these are no longer so, and says to
run that command again.
-->

## What is here

The programs are built from {} crates, for Linux, macOS and Windows
together: a crate that is one system's is here too. By the licence each is
taken under, a crate that is under two of them at once counted under each:

{}
The client is built from {} packages:

{}
{body}
",
        libraries(&named.crates),
        counted(&named.crates),
        libraries(&named.packages),
        counted(&named.packages),
    )
}

/// What the file is made from that is a file read whole, each by the name
/// its mark has at the head.
pub fn marks(reached: &[Locked], config: &str, template: &str) -> Vec<(String, String)> {
    vec![
        (MARK_CRATES.to_string(), shipped::mark(reached)),
        (CONFIG.to_string(), sha256::hex(config.as_bytes())),
        (TEMPLATE.to_string(), sha256::hex(template.as_bytes())),
    ]
}

/// What the file says it was made from that is no longer so, each in a
/// sentence. Nothing, while it was made from what is there now.
pub fn stale(read: &Read, now: &[(String, String)]) -> Vec<String> {
    let mut problems = Vec::new();
    for (what, mark) in now {
        let said = read.marks.iter().find(|(one, _)| one == what).map(|(_, mark)| mark);
        let moved = match what.as_str() {
            MARK_CRATES => {
                "the crates the programs can be built from are not the ones it was made from: one was added, \
                 taken away or is another version (`Cargo.lock`), or one that only tests used is now built \
                 into a program (a manifest)"
                    .to_string()
            }
            file => format!("`{file}` is not as it was when it was made"),
        };
        match said {
            Some(said) if said == mark => {}
            Some(_) => problems.push(format!("{FILE}: {moved}. Run `{COMMAND}` and commit what it writes")),
            None => problems.push(format!(
                "{FILE}: it does not say at its head what `{what}` it was made from. Run `{COMMAND}` and \
                 commit what it writes"
            )),
        }
    }
    problems
}

/// What is wrong with what the file names, each in a sentence: a library
/// with no version, no licence or no text; one of the product's own crates
/// named as somebody else's; and one whose licence asks for its source to
/// be offered that does not say where it is. A library may be named more
/// than once: under each of two licences it is under both of, and under
/// each of two texts it carries of one.
pub fn amiss(read: &Read, own: &[String]) -> Vec<String> {
    let mut problems = Vec::new();
    for (half, named) in [("crate", &read.crates), ("package", &read.packages)] {
        if named.is_empty() {
            problems.push(format!("{FILE}: it names no {half} at all"));
        }
        for one in named {
            let Named { name, version, licence, .. } = one;
            if version.is_empty() || !version.starts_with(|ch: char| ch.is_ascii_digit()) {
                problems.push(format!("{FILE}: the {half} `{name}` says no version"));
            }
            if licence.is_empty() {
                problems.push(format!("{FILE}: the {half} `{name}` {version} is under no licence"));
            }
            // The shortest licence there is runs to a few hundred letters.
            if one.text < 200 {
                problems.push(format!("{FILE}: the {half} `{name}` {version} has no text of its licence"));
            }
            if asks_for_source(licence) && !one.said.iter().any(|said| said.starts_with(SOURCE)) {
                problems.push(format!(
                    "{FILE}: the {half} `{name}` {version} is under `{licence}`, which asks that whoever hands \
                     on a copy says where its source is, and no line under it says so"
                ));
            }
        }
    }
    for one in read.crates.iter().filter(|one| own.contains(&one.name)) {
        problems.push(format!(
            "{FILE}: `{}` is one of the product's own crates, and is named as somebody else's",
            one.name
        ));
    }
    problems
}

/// The crates the file names that the lock does not hold at that version
/// among what the programs can be built from.
pub fn not_locked(read: &Read, reached: &[Locked]) -> Vec<String> {
    let named = read.crates.iter();
    named
        .filter(|one| !reached.iter().any(|locked| locked.name == one.name && locked.version == one.version))
        .map(|one| {
            format!(
                "{FILE}: it names the crate `{}` {}, which `Cargo.lock` does not hold at that version among \
                 what the programs are built from. Run `{COMMAND}` and commit what it writes",
                one.name, one.version
            )
        })
        .collect()
}

/// Every package a lock of the client's holds, by name and version, as
/// `npm` writes one: `"node_modules/<name>": {` and its `"version"` after.
pub fn installed(lock: &str) -> BTreeSet<(String, String)> {
    let mut all = BTreeSet::new();
    let mut name: Option<&str> = None;
    for line in lock.lines().map(str::trim) {
        if let Some(key) = line.strip_suffix("\": {").and_then(|key| key.strip_prefix('"')) {
            name = key.rfind("node_modules/").map(|at| &key[at + "node_modules/".len()..]);
        } else if let (Some(found), Some(version)) = (name, line.strip_prefix("\"version\": \"")) {
            all.insert((found.to_string(), version.trim_end_matches(['"', ',']).to_string()));
            name = None;
        }
    }
    all
}

/// The packages the file names that the client's lock does not hold at
/// that version.
pub fn not_installed(read: &Read, lock: &str) -> Vec<String> {
    let installed = installed(lock);
    let named = read.packages.iter();
    named
        .filter(|one| !installed.contains(&(one.name.clone(), one.version.clone())))
        .map(|one| {
            format!(
                "{FILE}: it names the client's package `{}` {}, which `ui/package-lock.json` does not hold at \
                 that version. Run `{COMMAND}` and commit what it writes",
                one.name, one.version
            )
        })
        .collect()
}

/// The programs' half, made by the tool; or, with no tool here, the half
/// that is there where nothing it was made from has changed.
fn crates_half(top: &Path, there: &Read, now: &[(String, String)]) -> Result<String, String> {
    let asked = Command::new("cargo").args(["about", "--version"]).current_dir(top).output();
    // What it says of itself, and not what cargo may say beside it.
    let version = asked
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned());
    let wanted = format!("{TOOL} {VERSION}");
    if version.as_deref().map(str::trim) != Some(wanted.as_str()) {
        let found = match &version {
            Some(other) => {
                format!("the `{TOOL}` here is `{}`, and the file is made with {VERSION}", other.trim())
            }
            None => format!("`{TOOL}` is not installed here"),
        };
        if stale(there, now).is_empty() && !there.crates_half.is_empty() {
            println!(
                "  {found}: the programs' half is kept as it is, since nothing it was made from has changed"
            );
            return Ok(there.crates_half.clone());
        }
        return Err(format!(
            "{found}. The crates the programs are built from, or what their notices are made by, have \
             changed, so their half of {FILE} has to be made again, and the tool is what reads each crate's \
             licence. Install it with\n  {INSTALL}\nand run `{COMMAND}` again. It is needed for this and \
             nothing else: no build, test or package asks for it"
        ));
    }
    // Held to this machine: the lock as it is and nothing fetched or asked
    // of any service. A crate's licence is read where cargo keeps its source.
    let asked =
        ["about", "-c", "never", "generate", "--frozen", "--fail", "--workspace", "-c", CONFIG, TEMPLATE];
    println!("  cargo {}", asked.join(" "));
    let output = Command::new("cargo")
        .args(asked)
        .current_dir(top)
        .output()
        .map_err(|err| format!("could not run `cargo about`: {err}"))?;
    if !output.status.success() {
        let said = String::from_utf8_lossy(&output.stderr);
        let why = if said.contains("failed to satisfy license requirements") {
            format!(
                "A crate is under a licence that `{CONFIG}` does not accept, named above. If the licence \
                 asks no more than that its notice goes with a copy, add it to `accepted` there. If it asks \
                 more (its source offered, the program under the same terms), it is the maintainer's to accept, \
                 one crate at a time, as an exception written there with what it asks; or the crate is not \
                 used"
            )
        } else if said.contains("offline") || said.contains("download") {
            "The tool reads each crate's licence from where cargo keeps the crate's source, and is told to \
             fetch nothing. A crate of another system than this one may never have been fetched here: \
             `cargo fetch` fetches every crate the lock names, once"
                .to_string()
        } else {
            format!("What `{CONFIG}` says of a crate may no longer be so of the version that is locked")
        };
        return Err(format!("`cargo about` could not make the notices:\n{}\n{why}", said.trim_end()));
    }
    let mut groups = records(&String::from_utf8_lossy(&output.stdout))
        .map_err(|why| format!("what `cargo about` wrote by `{TEMPLATE}` could not be read: {why}"))?;
    groups.extend(besides(&groups));
    let standard: Vec<String> = groups
        .iter()
        .filter(|group| group.standard)
        .flat_map(|group| {
            group.libraries.iter().map(|library| format!("{} {}", library.name, library.version))
        })
        .collect();
    if !standard.is_empty() {
        println!(
            "  {} crates carry no file of their licence, and are said by its standard text: {}",
            standard.len(),
            standard.join(", ")
        );
    }
    Ok(format!(
        "\nEvery crate a program is built from, and what a build of one runs: what\nonly the tests use is in \
         no program and is not here.\n\n{}{BUILT_IN}\n",
        half(&groups, Of::Crates)
    ))
}

/// The client's half, made by the client's own build, which says which
/// packages are in what it serves.
fn packages_half(top: &Path) -> Result<String, String> {
    let ui = top.join("ui");
    if !ui.join("node_modules").is_dir() {
        return Err(format!(
            "the client's packages are not here, and their notices are read from them: run `npm install` in \
             ui/, then `{COMMAND}` again"
        ));
    }
    println!("  node build.mjs --notices");
    let output = Command::new(tools::here("node"))
        .args(["build.mjs", "--notices"])
        .current_dir(&ui)
        .output()
        .map_err(|err| format!("could not run `node`: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "the client's build could not say what it bundles:\n{}",
            String::from_utf8_lossy(&output.stderr).trim_end()
        ));
    }
    let groups = records(&String::from_utf8_lossy(&output.stdout)).map_err(|why| {
        format!("what the client's build wrote of what it bundles could not be read: {why}")
    })?;
    Ok(format!(
        "\nEvery package that is in what the client's build writes to be served: what\nonly builds or tests \
         the client is not here.\n\n{}",
        half(&groups, Of::Packages)
    ))
}

/// What is there now for a notices file to be made from and held to.
pub struct Now {
    /// What a file made now says at its head it was made from.
    pub marks: Vec<(String, String)>,
    /// The crates of somebody else's the programs can be built from.
    pub reached: Vec<Locked>,
    /// The product's own crates, by name.
    pub own: Vec<String>,
}

/// What is there to be made from, read from the top of the repository.
pub fn now(top: &Path) -> Result<Now, String> {
    let text = |file: &str| {
        std::fs::read_to_string(top.join(file)).map_err(|err| format!("could not read {file}: {err}"))
    };
    let root = text("Cargo.toml")?;
    let manifests = shipped::manifests(top, &root);
    let reached = shipped::reached(&text("Cargo.lock")?, &root, &manifests, &shipped::PROGRAMS);
    let marks = marks(&reached, &text(CONFIG)?, &text(TEMPLATE)?);
    Ok(Now { marks, reached, own: manifests.into_iter().map(|(name, _)| name).collect() })
}

/// Everything a notices file is held to that needs no tool, each thing
/// that does not hold in a sentence: what it was made from is what is
/// there, what it names is what is locked, and each thing it names has a
/// licence and its text.
pub fn held(top: &Path) -> Result<Vec<String>, String> {
    let Now { marks, reached, own } = now(top)?;
    let file = std::fs::read_to_string(top.join(FILE)).map_err(|err| {
        format!(
            "{FILE} is not at the top of the repository ({err}): `{COMMAND}` makes it, and it is committed"
        )
    })?;
    let read = read(&file);
    let mut problems = stale(&read, &marks);
    problems.extend(amiss(&read, &own));
    problems.extend(not_locked(&read, &reached));
    if let Ok(lock) = std::fs::read_to_string(top.join("ui/package-lock.json")) {
        problems.extend(not_installed(&read, &lock));
    }
    Ok(problems)
}

/// `cargo xtask notices [--check]`: makes the file, or with `--check`
/// says whether the one that is there is what would be made.
pub fn run(top: &Path, args: &[String]) -> Result<(), String> {
    let check = args.iter().any(|arg| arg == "--check");
    if let Some(other) = args.iter().find(|arg| *arg != "--check") {
        return Err(format!("`{other}` is nothing `{COMMAND}` takes: it takes `--check`, or nothing"));
    }
    let Now { marks, reached, own } = now(top)?;
    let before = std::fs::read_to_string(top.join(FILE)).unwrap_or_default();
    println!("making {FILE}");
    let crates = crates_half(top, &read(&before), &marks)?;
    let packages = packages_half(top)?;
    let made = whole(&marks, &crates, &packages);
    // Nothing is written that the test of it would then refuse.
    let read = read(&made);
    let mut problems = amiss(&read, &own);
    problems.extend(not_locked(&read, &reached));
    if !problems.is_empty() {
        return Err(format!(
            "what was made is not a notices file as it has to be:\n  {}",
            problems.join("\n  ")
        ));
    }
    let licences = |named: &[Named]| {
        let all: BTreeSet<&str> = named.iter().map(|one| one.licence.as_str()).collect();
        all.into_iter().collect::<Vec<_>>().join(", ")
    };
    println!(
        "  {} crates ({})\n  {} packages of the client ({})",
        libraries(&read.crates),
        licences(&read.crates),
        libraries(&read.packages),
        licences(&read.packages)
    );
    match (made == before, check) {
        (true, _) => println!("{FILE} is what is there already: nothing to commit"),
        (false, true) => {
            return Err(format!(
                "{FILE} is not what `{COMMAND}` makes of what is here now: run it without `--check`, and \
                 commit what it writes"
            ));
        }
        (false, false) => {
            let file = top.join(FILE);
            std::fs::write(&file, &made)
                .map_err(|err| format!("could not write {}: {err}", file.display()))?;
            println!("wrote {FILE} ({} lines): commit it", made.lines().count());
        }
    }
    Ok(())
}
