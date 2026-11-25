//! A stand-in for a program: what the tests start where they need one
//! that says, reads, writes, waits and ends as they tell it.
//!
//! It was a line of a Unix shell (`sh -c 'echo ready; sleep 300'`), which
//! is one system's: where there is no such shell the tests that used one
//! could not be played at all. This is a program, built with the suite
//! that uses it and the same on every system, and what it does is said
//! in its arguments, read by itself: no shell reads anything.
//!
//! It is the standard library and nothing else, on purpose. It is built
//! as a program of each crate whose suite starts it (cargo builds a
//! crate's programs for its tests, and tells them where:
//! `CARGO_BIN_EXE_<name>`), and a crate's program cannot use what only
//! its tests depend on. It is no part of the product: nothing that makes
//! a package or installs copies it, and nothing in `src/` starts it.
//!
//! # What it is told
//!
//! Its arguments are words it does one after another, each with as many
//! things after it as it takes, and it ends well when it has done the
//! last:
//!
//! | word | what it does |
//! |---|---|
//! | `say TEXT` | says a line |
//! | `err TEXT` | says a line where a program complains |
//! | `put TEXT` | says the text and does not end the line |
//! | `var NAME` | says what that variable holds, and does not end the line |
//! | `env PART` | says each variable it was started with that has the part in its name or in what it holds, a line each, as `NAME=what it holds` |
//! | `args ...` | says everything after it as it arrived, with `\|` between, and ends |
//! | `cat` | says back what it is handed to read, to its end |
//! | `swallow` | reads what it is handed to its end and says nothing |
//! | `read` | reads what it is handed to its end and keeps it |
//! | `where PART ( ... )` | where what it read has the part in it, does what is in the brackets and ends |
//! | `pick START END` | keeps what is between the two in the first line of what it read that begins with the one and has the other after it |
//! | `there FILE ( ... )` | where there is such a file, does what is in the brackets and ends |
//! | `says FILE` | where there is such a file, says what is in it and ends well |
//! | `write FILE TEXT` | writes a line into a file, in place of what was there |
//! | `append FILE TEXT` | adds a line to a file |
//! | `has FILE` | goes on where there is such a file, and ends with 1 where not |
//! | `lacks FILE` | goes on where there is no such file, and ends with 1 where there is |
//! | `needs FILE TEXT` | where there is no such file, says the line and ends with 1 |
//! | `any START` | goes on where the folder it is in has a file called something that starts so, and ends with 1 where not |
//! | `holds FILE TEXT` | goes on where the file says that and no more, and ends with 1 where not |
//! | `fewer FILE COUNT TEXT` | where the file has fewer lines than that, says the line and ends with 1 |
//! | `first FILE CODE` | where there is no such file, makes it and ends with the code |
//! | `lacking NAME TEXT` | where no such variable was set at all, says the line and ends well |
//! | `given NAME` | goes on where that variable holds something, and ends with 1 where not |
//! | `held AT WHO` | waits to be told: see below |
//! | `tell AT TEXT` | says a line to whoever listens there, and goes on |
//! | `idle` | does nothing until it is stopped, and for ten minutes at most: one a test left behind ends by itself |
//! | `also ( ... )` | starts a copy of itself that does what is in the brackets and goes on alone |
//! | `run ( PROGRAM ... )` | starts that program with those arguments, where it is, and waits for it, however it ends |
//! | `unset NAME` | what it starts from here on is not told that variable |
//! | `exit CODE` | ends with the code |
//!
//! In a text, `%NAME%` is what that variable holds (nothing where it is
//! not set), `%NAME?words%` the words where it is not set at all, and
//! `%%` a `%`. Eight are its own: `%:pid%` is its number, `%:child%` the
//! number of the copy it last started, `%:cwd%` the folder it is in and
//! `%:dir%` what that is called, `%:told%` what it was last told while
//! it was held, `%:picked%` what it last picked out of what it read,
//! `%:read%` what it read, and `%:sum%` a number that is the same for
//! the same thing read and another for anything else.
//!
//! Where a shell is to start it (a user's own `run:` is read by one),
//! `%NAME%` is the Windows shell's own way of writing a variable, and
//! that shell puts what the variable holds into the line before the
//! stand-in is started: `put` and `var` say the same with nothing in
//! them either shell reads.
//!
//! # Waiting to be told
//!
//! `held AT WHO` connects to an address on this device, says who it is
//! in a line, and waits there until it is told a line or the connection
//! is closed. A test listens there and lets it go when it chooses:
//! nothing waits on a clock, and nothing is looked for again and again.
//! Where nothing listens, nobody holds it and it goes on, so one that
//! outlives its test ends by itself. (The shell's way was a pipe with a
//! name, which only Unix has.)
//!
//! Where it waits is an address (`127.0.0.1:4000`), or a file that
//! holds one: a test that is to hold it makes the file, and where there
//! is no such file nobody holds it. Several may be said with `|`
//! between them, and it waits at the first that is there: one of its
//! own before one for everybody. `tell` says a line at such a place
//! and waits for nothing.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};

/// What it knows as it goes.
#[derive(Default)]
struct Knows {
    /// What it was last told while it was held.
    told: String,
    /// The number of the copy it last started.
    child: String,
    /// What it read of what it was handed.
    read: String,
    /// What it last picked out of that.
    picked: String,
    /// The variables that what it starts is not told.
    unset: Vec<String>,
}

fn main() {
    let words: Vec<String> = std::env::args().skip(1).collect();
    let code = match Knows::default().does(&words) {
        Ok(code) => code,
        Err(why) => {
            let _ = std::io::stderr().write_all(format!("the stand-in: {why}\n").as_bytes());
            2
        }
    };
    let _ = std::io::stdout().flush();
    std::process::exit(code);
}

/// A line where a program says things. One that nobody reads any more is
/// not a reason to end badly.
fn say(line: &str) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(format!("{line}\n").as_bytes());
    let _ = out.flush();
}

/// The same, with the line not ended.
fn put(text: &str) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(text.as_bytes());
    let _ = out.flush();
}

impl Knows {
    /// Does the words, one after another: the code to end with.
    fn does(&mut self, words: &[String]) -> Result<i32, String> {
        let mut rest = words.iter();
        while let Some(word) = rest.next() {
            let mut next = |what: &str| {
                rest.next().map(|text| self.reads(text)).ok_or(format!("`{word}` takes {what} after it"))
            };
            match word.as_str() {
                "say" => say(&next("a text")?),
                "err" => {
                    let line = next("a text")?;
                    let _ = std::io::stderr().write_all(format!("{line}\n").as_bytes());
                }
                "put" => put(&next("a text")?),
                "var" => put(&std::env::var(next("a name")?).unwrap_or_default()),
                "env" => {
                    let part = next("a part of a name or of what one holds")?;
                    let mut all: Vec<String> = std::env::vars_os()
                        .map(|(name, held)| format!("{}={}", name.to_string_lossy(), held.to_string_lossy()))
                        .filter(|one| one.contains(&part))
                        .collect();
                    all.sort();
                    all.iter().for_each(|one| say(one));
                }
                "args" => {
                    say(&rest.by_ref().map(String::as_str).collect::<Vec<_>>().join("|"));
                    return Ok(0);
                }
                "cat" => {
                    let mut out = std::io::stdout().lock();
                    let _ = std::io::copy(&mut std::io::stdin().lock(), &mut out);
                    let _ = out.flush();
                }
                "swallow" => {
                    let _ = std::io::copy(&mut std::io::stdin().lock(), &mut std::io::sink());
                }
                "read" => {
                    let mut handed = Vec::new();
                    let _ = std::io::stdin().lock().read_to_end(&mut handed);
                    self.read = String::from_utf8_lossy(&handed).into_owned();
                }
                "where" => {
                    let part = next("a part of what it read, and `(`")?;
                    let inner: Vec<String> = bracketed(&mut rest)?.into_iter().cloned().collect();
                    if self.read.contains(&part) {
                        return self.does(&inner);
                    }
                }
                "pick" => {
                    let (start, end) = (next("a start and an end")?, next("a start and an end")?);
                    let between = |line: &str| {
                        let after = line.strip_prefix(start.as_str())?;
                        Some(after[..after.find(end.as_str())?].to_string())
                    };
                    self.picked = self.read.lines().find_map(between).unwrap_or_default();
                }
                "there" => {
                    let file = next("a file, and `(`")?;
                    let inner: Vec<String> = bracketed(&mut rest)?.into_iter().cloned().collect();
                    if std::path::Path::new(&file).exists() {
                        return self.does(&inner);
                    }
                }
                "says" => {
                    if let Ok(said) = std::fs::read(next("a file")?) {
                        let mut out = std::io::stdout().lock();
                        let _ = out.write_all(&said);
                        let _ = out.flush();
                        return Ok(0);
                    }
                }
                "write" => {
                    let (file, text) = (next("a file and a text")?, next("a file and a text")?);
                    within(&file);
                    std::fs::write(&file, format!("{text}\n")).map_err(|err| format!("{file}: {err}"))?;
                }
                "append" => {
                    let (file, text) = (next("a file and a text")?, next("a file and a text")?);
                    within(&file);
                    let opened = std::fs::OpenOptions::new().create(true).append(true).open(&file);
                    opened
                        .and_then(|mut opened| opened.write_all(format!("{text}\n").as_bytes()))
                        .map_err(|err| format!("{file}: {err}"))?;
                }
                "has" => {
                    if !std::path::Path::new(&next("a file")?).exists() {
                        return Ok(1);
                    }
                }
                "lacks" => {
                    if std::path::Path::new(&next("a file")?).exists() {
                        return Ok(1);
                    }
                }
                "needs" => {
                    let (file, text) = (next("a file and a text")?, next("a file and a text")?);
                    if !std::path::Path::new(&file).exists() {
                        say(&text);
                        return Ok(1);
                    }
                }
                "any" => {
                    let start = next("how a file's name starts")?;
                    let called =
                        |entry: std::fs::DirEntry| entry.file_name().to_string_lossy().starts_with(&start);
                    if !std::fs::read_dir(".").into_iter().flatten().flatten().any(called) {
                        return Ok(1);
                    }
                }
                "holds" => {
                    let (file, text) = (next("a file and a text")?, next("a file and a text")?);
                    let held = std::fs::read_to_string(&file).unwrap_or_default();
                    if held.trim_end_matches(['\r', '\n']) != text {
                        return Ok(1);
                    }
                }
                "fewer" => {
                    let file = next("a file, a count and a text")?;
                    let (count, text) = (next("a count and a text")?, next("a text")?);
                    let count: usize =
                        count.parse().map_err(|_| format!("`{count}` is no count of lines"))?;
                    if std::fs::read_to_string(&file).unwrap_or_default().lines().count() < count {
                        say(&text);
                        return Ok(1);
                    }
                }
                "first" => {
                    let (file, code) = (next("a file and a code")?, next("a file and a code")?);
                    let code = code.parse().map_err(|_| format!("`{code}` is no code to end with"))?;
                    if std::fs::OpenOptions::new().write(true).create_new(true).open(&file).is_ok() {
                        return Ok(code);
                    }
                }
                "lacking" => {
                    let (name, text) = (next("a name and a text")?, next("a name and a text")?);
                    if std::env::var_os(&name).is_none() {
                        say(&text);
                        return Ok(0);
                    }
                }
                "given" => {
                    if std::env::var_os(next("a name")?).is_none_or(|held| held.is_empty()) {
                        return Ok(1);
                    }
                }
                "held" => {
                    let (at, who) = (next("a place and a name")?, next("a place and a name")?);
                    self.told = listens(&at).map(|address| held(&address, &who)).unwrap_or_default();
                }
                "tell" => {
                    let (at, text) = (next("a place and a text")?, next("a place and a text")?);
                    if let Some(mut stream) = listens(&at).and_then(|at| TcpStream::connect(at).ok()) {
                        let _ = stream.write_all(format!("{text}\n").as_bytes());
                    }
                }
                "idle" => {
                    // For ten minutes and no longer. A daemon leaves what it
                    // hosts running, so one whose test was cut off before it
                    // stopped its service has nobody to end it: three of
                    // these were found an hour old where the simulation had
                    // played.
                    let until = std::time::Instant::now() + std::time::Duration::from_secs(600);
                    while let Some(left) = until.checked_duration_since(std::time::Instant::now()) {
                        if left.is_zero() {
                            break;
                        }
                        std::thread::park_timeout(left);
                    }
                }
                "also" => {
                    let inner = bracketed(&mut rest)?;
                    let _ = std::io::stdout().flush();
                    let own =
                        std::env::current_exe().map_err(|err| format!("where this program is: {err}"))?;
                    let mut copy = Command::new(own);
                    for name in &self.unset {
                        copy.env_remove(name);
                    }
                    let copy = copy.args(inner).stdin(Stdio::null()).spawn();
                    self.child =
                        copy.map_err(|err| format!("a copy could not be started: {err}"))?.id().to_string();
                }
                "run" => {
                    let inner = bracketed(&mut rest)?;
                    let (program, arguments) =
                        inner.split_first().ok_or("`run` takes a program between its brackets")?;
                    let _ = std::io::stdout().flush();
                    let mut started = Command::new(program);
                    for name in &self.unset {
                        started.env_remove(name);
                    }
                    let ended = started.args(arguments).stdin(Stdio::null()).status();
                    ended.map_err(|err| format!("`{program}` could not be started: {err}"))?;
                }
                "unset" => {
                    let name = next("a name")?;
                    self.unset.push(name);
                }
                "exit" => {
                    let code = next("a code")?;
                    return code.parse().map_err(|_| format!("`{code}` is no code to end with"));
                }
                other => return Err(format!("`{other}` is no word it knows")),
            }
        }
        Ok(0)
    }

    /// A text as it is meant: with what each `%...%` in it stands for.
    fn reads(&self, text: &str) -> String {
        let mut read = String::new();
        let mut rest = text;
        while let Some(at) = rest.find('%') {
            read.push_str(&rest[..at]);
            let after = &rest[at + 1..];
            let Some(end) = after.find('%') else {
                // One by itself is a `%`.
                read.push('%');
                rest = after;
                continue;
            };
            read.push_str(&self.stands_for(&after[..end]));
            rest = &after[end + 1..];
        }
        read + rest
    }

    fn stands_for(&self, name: &str) -> String {
        match name {
            "" => "%".to_string(),
            ":pid" => std::process::id().to_string(),
            ":child" => self.child.clone(),
            ":told" => self.told.clone(),
            ":read" => self.read.clone(),
            ":picked" => self.picked.clone(),
            // The same for the same thing read: no more is asked of it.
            ":sum" => self
                .read
                .bytes()
                .fold(0xcbf2_9ce4_8422_2325_u64, |sum, byte| {
                    (sum ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3)
                })
                .to_string(),
            ":cwd" => std::env::current_dir().map(|dir| dir.display().to_string()).unwrap_or_default(),
            ":dir" => std::env::current_dir()
                .ok()
                .and_then(|dir| dir.file_name().map(|name| name.to_string_lossy().into_owned()))
                .unwrap_or_default(),
            name => {
                let (name, unset) = name.split_once('?').unwrap_or((name, ""));
                std::env::var(name).unwrap_or_else(|_| unset.to_string())
            }
        }
    }
}

/// Makes the folder a file is to be in, where it is not there.
fn within(file: &str) {
    if let Some(folder) = std::path::Path::new(file).parent().filter(|folder| !folder.as_os_str().is_empty())
    {
        let _ = std::fs::create_dir_all(folder);
    }
}

/// The address that is listened at where `at` says: the first of those
/// it names, with `|` between them, that is an address or a file there
/// is that holds one. None where there is none.
fn listens(at: &str) -> Option<String> {
    at.split('|').find_map(|one| {
        if one.parse::<std::net::SocketAddr>().is_ok() {
            return Some(one.to_string());
        }
        let held = std::fs::read_to_string(one).ok()?;
        Some(held.trim().to_string()).filter(|address| !address.is_empty())
    })
}

/// Waits at `address` until it is told a line or let go: what it was told.
fn held(address: &str, who: &str) -> String {
    // Nothing listening is nobody holding.
    let Ok(mut stream) = TcpStream::connect(address) else { return String::new() };
    let _ = stream.write_all(format!("{who}\n").as_bytes());
    let mut told = String::new();
    let _ = BufReader::new(stream).read_line(&mut told);
    told.trim_end().to_string()
}

/// The words between a `(` and the `)` that closes it.
fn bracketed<'a>(rest: &mut std::slice::Iter<'a, String>) -> Result<Vec<&'a String>, String> {
    if rest.next().map(String::as_str) != Some("(") {
        return Err("`also` takes `(`, what the copy does, and `)`".to_string());
    }
    let (mut inner, mut open) = (Vec::new(), 1);
    for word in rest {
        match word.as_str() {
            "(" => open += 1,
            ")" => open -= 1,
            _ => {}
        }
        if open == 0 {
            return Ok(inner);
        }
        inner.push(word);
    }
    Err("a `(` was never closed".to_string())
}
