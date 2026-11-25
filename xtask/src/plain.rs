//! The `plain` gate: a blueprint that ships works nothing out for itself.
//!
//! A blueprint is where somebody manages a workflow in plain words: what
//! exists, what is heard, what is done. What is decided (whose turn it is,
//! what a row comes to, whether a card goes round again, the sentence that
//! says so) is a building block's: a function in Rust, tested as one with a
//! table of cases, which the blueprint asks and then does what it is told.
//! A text that decides for itself is a program written in holes in
//! strings, which nothing tests but playing the whole product, and which
//! is where the faults at a restart were.
//!
//! So an expression in a text that ships says what a thing is called, or
//! asks one plain question of it:
//!
//! - a name: `params.column`, `steps.card.out.status`, `event.data.id`;
//! - a name compared with a word, a number or another name, or looked for
//!   among several (`==`, `!=`, `in`);
//! - a name held against a number written out (`<`, `<=`, `>`, `>=`);
//! - a name with what stands for it when it says nothing (`a ?? b`);
//! - `!` of one of those;
//! - at most three of those joined, all by `&&` or all by `||`.
//!
//! A function that is called, arithmetic, `? :`, a sentence put together
//! with `+`, which of two names is the greater, `&&` and `||` in one
//! expression, four things joined: each is something worked out, and is
//! not plain.
//!
//! What is plain is the language's to say, since only it reads an
//! expression as the runner does (`blueprint::plain`), and the tooling is
//! the standard library and nothing else. So the texts are read by a test
//! of the runner's suite ([`READER`]), which says each expression that is
//! not plain on a line of its own; this counts them against what is still
//! let through, and is tested as that (`tests/suite/plain.rs`).
//!
//! What is let through is work still to do, and is to be nothing: the
//! list below can only shrink. A file with more than its number fails; so
//! does one with fewer, until the number is lowered, so that the list is
//! always what is so.

/// The crate whose test program reads the texts, and the test that does.
pub const READER: (&str, &str) =
    ("conductor", "plainly::the_expressions_of_what_ships_are_read_and_said_for_the_gate");

/// Before each line that test says of an expression that is not plain:
/// the file, the line, why, and the expression, with a tab between.
pub const SAID: &str = "::plain ";
/// Before the line that says how many texts it read.
pub const READ: &str = "::plain-read ";

/// What a file is still let: how many of its expressions are not plain,
/// what is to become of it, and where that is written down.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Let {
    pub file: String,
    pub count: usize,
    pub becomes: String,
    pub written: String,
}

/// Where what the standard blueprints are still let is written.
const HERE: &str = "xtask/src/plain.rs";

/// What is still let through of the blueprints that ship with the program.
pub fn still() -> Vec<Let> {
    STILL
        .iter()
        .map(|(file, count, becomes)| Let {
            file: (*file).to_string(),
            count: *count,
            becomes: (*becomes).to_string(),
            written: HERE.to_string(),
        })
        .collect()
}

/// What a package says its blueprints are still let, from the one line it
/// keeps of it (`<number> <what is to become of it>`; `#` begins a note):
/// for the manifest `manifest`, written in `written`. An error where the
/// line cannot be read.
pub fn of_package(manifest: &str, written: &str, text: &str) -> Result<Let, String> {
    let line = text.lines().map(str::trim).find(|line| !line.is_empty() && !line.starts_with('#'));
    let said = line.and_then(|line| {
        let (count, becomes) = line.split_once(' ')?;
        Some((count.parse::<usize>().ok()?, becomes.trim()))
    });
    match said {
        Some((count, becomes)) if count > 0 && !becomes.is_empty() => Ok(Let {
            file: manifest.to_string(),
            count,
            becomes: becomes.to_string(),
            written: written.to_string(),
        }),
        _ => Err(format!(
            "{written} says what the package's blueprints still work out for themselves in one line, the \
             number and then what is to become of it (`3 to be made plain before the first release`); it \
             says nothing this can read. A package whose blueprints are plain has no such file"
        )),
    }
}

/// What is still to be made plain of the blueprints that ship with the
/// program: the file, how many of its expressions are not, and what is to
/// become of it. It is empty, and stays so: a text that ships and works
/// something out for itself fails the gate.
pub const STILL: &[(&str, usize, &str)] = &[];

/// Where a package says what its own blueprints still work out, beside its
/// manifest: one line, the number and what is to become of it. The
/// tooling names no package, so a package's number is the package's to
/// keep ([`of_package`]).
pub const PACKAGES_OWN: &str = "contract/plain.txt";

/// An expression that is not plain, as the reader said it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unplain {
    pub file: String,
    pub line: u32,
    pub why: String,
    pub source: String,
}

/// What the reader said: how many texts it read, and each expression that
/// is not plain. An error where it did not say how many it read, which is
/// a reader that did not run to its end.
pub fn heard(printed: &str) -> Result<(usize, Vec<Unplain>), String> {
    let mut found = Vec::new();
    let mut read = None;
    // A line is looked for the marks anywhere in it: what runs the tests
    // writes the test's name first and no end of line, and the first thing
    // the reader says comes after that on the same line.
    let after = |line: &'_ str, mark: &str| line.find(mark).map(|at| line[at + mark.len()..].to_string());
    for line in printed.lines() {
        if let Some(count) = after(line, READ) {
            read = count.trim().parse().ok();
        } else if let Some(said) = after(line, SAID) {
            let mut parts = said.splitn(4, '\t');
            let (Some(file), Some(at), Some(why), Some(source)) =
                (parts.next(), parts.next(), parts.next(), parts.next())
            else {
                return Err(format!("the reader of the blueprints said a line this cannot read: `{line}`"));
            };
            found.push(Unplain {
                file: file.to_string(),
                line: at.parse().unwrap_or(0),
                why: why.to_string(),
                source: source.to_string(),
            });
        }
    }
    match read {
        Some(read) => Ok((read, found)),
        None => Err(format!(
            "the reader of the blueprints that ship (`{}` in the suite of `{}`) did not say how many it read: \
             it did not run to its end.\n{printed}",
            READER.1, READER.0
        )),
    }
}

/// Holds what was found against what is still let through: what the gate
/// says where all is as listed, or everything that is not.
pub fn held(read: usize, found: &[Unplain], still: &[Let]) -> Result<String, Vec<String>> {
    let of = |file: &str| -> Vec<&Unplain> { found.iter().filter(|one| one.file == file).collect() };
    let mut files: Vec<&str> = found.iter().map(|one| one.file.as_str()).collect();
    files.sort_unstable();
    files.dedup();
    let mut problems = Vec::new();
    for file in &files {
        let there = of(file);
        let listed = still.iter().find(|listed| listed.file == *file);
        let every = || there.iter().map(|one| said(one)).collect::<Vec<_>>().join("\n");
        match listed {
            None => problems.push(format!(
                "{file} works something out for itself, in {}:\n{}\nA blueprint that ships asks a building block \
                 what a thing comes to and does what it is told: move what is worked out here into one (a \
                 function, tested as one, in the crate where its matter lives) and leave the name of its answer.",
                counted(there.len()),
                every()
            )),
            Some(listed) if there.len() > listed.count => problems.push(format!(
                "{file} has {} that {} not plain, and {} {} still let through ({}). The number only goes down: \
                 what is new among these is a building block's to work out.\n{}",
                counted(there.len()),
                are(there.len()),
                listed.count,
                are(listed.count),
                listed.written,
                every()
            )),
            Some(listed) if there.len() < listed.count => problems.push(lower(listed, there.len())),
            Some(_) => {}
        }
    }
    for listed in still.iter().filter(|listed| !files.contains(&listed.file.as_str())) {
        problems.push(lower(listed, 0));
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    let total: usize = still.iter().map(|listed| listed.count).sum();
    Ok(match total {
        0 => format!("{read} blueprints that ship work nothing out for themselves"),
        _ => format!(
            "{} in {} {} still to be made plain, of {read} blueprints that ship",
            counted(total),
            match still.len() {
                1 => "1 file".to_string(),
                files => format!("{files} files"),
            },
            are(total)
        ),
    })
}

fn counted(count: usize) -> String {
    match count {
        1 => "1 expression".to_string(),
        count => format!("{count} expressions"),
    }
}

fn are(count: usize) -> &'static str {
    if count == 1 { "is" } else { "are" }
}

fn said(one: &Unplain) -> String {
    format!("  line {}: {}: `{}`", one.line, one.why, one.source)
}

/// What is said of a file that has fewer than it is let: what is written
/// down is to say what is so.
fn lower(listed: &Let, there: usize) -> String {
    let (file, count, written) = (&listed.file, listed.count, &listed.written);
    match there {
        0 => format!("{file} is plain now, and is still let {}: take that out of {written}", counted(count)),
        there => format!(
            "{file} has {} that {} not plain, and is still let {count}: lower its number in {written} to {there}",
            counted(there),
            are(there)
        ),
    }
}
