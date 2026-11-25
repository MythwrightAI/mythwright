//! How what is said to the factory's bot is read: by how each command is
//! written in the command line's own words (`commands::TAKES`), and by
//! nothing else. A command's words are read from that line (what is fixed,
//! what is one word, what is the rest of them, `name=value` pairs and
//! `--flags`), so that what the bot takes, what it says it takes and what
//! the command line takes are one list and cannot come apart.

use std::collections::HashMap;

/// A part of how a command is written.
#[derive(Debug, PartialEq)]
enum Piece {
    /// A word written as it is: `board`, `move`, and `run` in the middle;
    /// one in square brackets may be left out: `[list]`.
    Fixed { word: String, needed: bool },
    /// One word: `<card>`; `<agent>|nobody`; and `<project>|--none`, which
    /// is left out where the flag after it is said.
    One { name: String, or: Option<String> },
    /// The rest of the words: `<title...>`, and `[<column>...]`, which may be none.
    Rest { name: String, needed: bool },
    /// `name=value` pairs: `<field=value>...`, at least one; `[k=v]...`, any.
    Pairs { needed: bool },
    /// A flag: `[--back]`, `[--note TEXT]`, `[--label L]...`.
    Flag { name: String, takes: bool, again: bool },
}

/// What was said to a command, as its line reads it.
#[derive(Debug, Default)]
pub struct Said {
    /// Each word its line names (`card`, `title`), as it was said.
    pub words: HashMap<String, String>,
    /// Each flag that was said, with what was said after it each time.
    pub flags: HashMap<String, Vec<String>>,
    /// The `name=value` pairs, in order.
    pub pairs: Vec<(String, String)>,
}

impl Said {
    pub fn word(&self, name: &str) -> &str {
        self.words.get(name).map_or("", String::as_str)
    }

    pub fn flag(&self, name: &str) -> Option<&str> {
        self.flags.get(name).and_then(|said| said.last()).map(String::as_str)
    }

    pub fn has(&self, name: &str) -> bool {
        self.flags.contains_key(name)
    }

    pub fn every(&self, name: &str) -> &[String] {
        self.flags.get(name).map_or(&[], Vec::as_slice)
    }
}

/// The parts of how a command is written.
fn pieces(written: &str) -> Vec<Piece> {
    let (mut pieces, mut words) = (Vec::new(), written.split(' ').peekable());
    while let Some(word) = words.next() {
        let piece = if let Some(flag) = word.strip_prefix("[--") {
            match flag.strip_suffix(']') {
                Some(name) => Piece::Flag { name: name.to_string(), takes: false, again: false },
                None => {
                    // A flag's value is one placeholder, or several where they are bracketed
                    // themselves (`[--project P [--environment E]]`): up to the one that closes it.
                    let mut again = false;
                    while let Some(value) = words.next_if(|value| !value.starts_with("[--")) {
                        if value.ends_with("]...") {
                            again = true;
                        }
                        if value.ends_with(']') || value.ends_with("]...") {
                            break;
                        }
                    }
                    Piece::Flag { name: flag.to_string(), takes: true, again }
                }
            }
        } else if word.contains('=') && !word.contains("|--") {
            Piece::Pairs { needed: word.starts_with('<') }
        } else if let Some(inside) = word.strip_prefix("[<") {
            let name = inside.split(['.', '>']).next().unwrap_or_default();
            Piece::Rest { name: name.to_string(), needed: false }
        } else if let Some(inside) = word.strip_prefix('<') {
            let (name, after) = inside.split_once('>').unwrap_or((inside, ""));
            let or = after.strip_prefix("|--").map(str::to_string);
            match name.strip_suffix("...") {
                Some(name) => Piece::Rest { name: name.to_string(), needed: true },
                None => Piece::One { name: name.to_string(), or },
            }
        } else if let Some(word) = word.strip_prefix('[').and_then(|word| word.strip_suffix(']')) {
            Piece::Fixed { word: word.to_string(), needed: false }
        } else {
            Piece::Fixed { word: word.to_string(), needed: true }
        };
        // `<x>|--y`: the flag is one too, which is how the word is left out.
        if let Piece::One { or: Some(flag), .. } = &piece {
            let flag = Piece::Flag { name: flag.clone(), takes: false, again: false };
            pieces.push(piece);
            pieces.push(flag);
            continue;
        }
        pieces.push(piece);
    }
    pieces
}

/// The words a command is written with that are written as they are,
/// wherever they are (`board move`, `project environment run`): what tells
/// one command from another.
pub fn key(written: &str) -> String {
    let fixed = pieces(written).into_iter().filter_map(|piece| match piece {
        Piece::Fixed { word, needed: true } => Some(word),
        _ => None,
    });
    fixed.collect::<Vec<_>>().join(" ")
}

/// The words it begins with that are written as they are.
fn leading(pieces: &[Piece]) -> Vec<&str> {
    let fixed = pieces.iter().map_while(|piece| match piece {
        Piece::Fixed { word, needed: true } => Some(word.as_str()),
        _ => None,
    });
    fixed.collect()
}

/// How many words `written` begins with that are written as they are,
/// where `said` begins with them; nothing where it does not.
pub fn begins(written: &str, said: &[&str]) -> Option<usize> {
    let pieces = pieces(written);
    let words = leading(&pieces);
    (said.len() >= words.len() && said[..words.len()] == words[..]).then_some(words.len())
}

/// What `said` says to the command written `written`, or why it says
/// nothing it takes. `project` is the channel's, where it has one: there
/// `<project>` may be left out, and is that project.
pub fn read(written: &str, said: &[&str], project: Option<&str>) -> Result<Said, String> {
    let pieces = pieces(written);
    let fixed = leading(&pieces);
    let named = fixed.join(" ");
    let usage = || format!("it is written `{written}`");
    let mut read = Said::default();
    // What follows the fixed words: the flags, wherever they are, and the rest in order.
    let mut placed: Vec<&str> = Vec::new();
    let mut rest = said[fixed.len()..].iter().peekable();
    while let Some(word) = rest.next() {
        let Some(name) = word.strip_prefix("--") else {
            placed.push(word);
            continue;
        };
        let Some(Piece::Flag { takes, again, .. }) =
            pieces.iter().find(|piece| matches!(piece, Piece::Flag { name: one, .. } if one == name))
        else {
            return Err(format!("`--{name}` is not something `{named}` takes: {}.", usage()));
        };
        let mut value: Vec<&str> = Vec::new();
        while *takes && rest.peek().is_some_and(|next| !next.starts_with("--")) {
            value.extend(rest.next().copied());
        }
        if *takes && value.is_empty() {
            return Err(format!("`--{name}` is followed by what it says: {}.", usage()));
        }
        let kept = read.flags.entry(name.to_string()).or_default();
        if !again && !kept.is_empty() {
            return Err(format!("`--{name}` is said once: {}.", usage()));
        }
        kept.push(value.join(" "));
    }
    let mut placed = placed.into_iter().peekable();
    let after = |at: usize| {
        pieces[at + 1..].iter().any(|piece| matches!(piece, Piece::One { .. } | Piece::Rest { .. }))
    };
    for (at, piece) in pieces.iter().enumerate().skip(fixed.len()) {
        match piece {
            Piece::Fixed { word, needed } => match placed.peek() {
                Some(said) if said == word => drop(placed.next()),
                _ if !needed => {}
                _ => return Err(format!("`{named}` needs `{word}` there: {}.", usage())),
            },
            Piece::One { or: Some(flag), .. } if read.has(flag) => {}
            Piece::One { name, .. } if name == "project" && project.is_some() => {
                // In a project's channel the project may be left out: a word is it where it is the
                // project, or where nothing else is to be said after it (and is then another's).
                let named = placed.peek().is_some_and(|word| Some(*word) == project || !after(at));
                let word =
                    if named { placed.next().unwrap_or_default() } else { project.unwrap_or_default() };
                read.words.insert(name.clone(), word.to_string());
            }
            Piece::One { name, .. } => {
                let word = placed.next().ok_or_else(|| format!("`{named}` needs <{name}>: {}.", usage()))?;
                read.words.insert(name.clone(), word.to_string());
            }
            Piece::Rest { name, needed } => {
                let words: Vec<&str> =
                    std::iter::from_fn(|| placed.next_if(|word| !word.contains('='))).collect();
                if *needed && words.is_empty() {
                    return Err(format!("`{named}` needs <{name}...>: {}.", usage()));
                }
                read.words.insert(name.clone(), words.join(" "));
            }
            Piece::Pairs { needed } => {
                for word in placed.by_ref() {
                    let (name, value) = word
                        .split_once('=')
                        .ok_or_else(|| format!("`{word}` should be `name=value`: {}.", usage()))?;
                    read.pairs.push((name.to_string(), value.to_string()));
                }
                if *needed && read.pairs.is_empty() {
                    return Err(format!("`{named}` needs one or more of `name=value`: {}.", usage()));
                }
            }
            Piece::Flag { .. } => {}
        }
    }
    match placed.next() {
        Some(word) => Err(format!("`{word}` is more than `{named}` takes: {}.", usage())),
        None => Ok(read),
    }
}

/// How many parts a command is written with: of two that begin with the
/// same words, the one with more is tried first.
pub fn parts(written: &str) -> usize {
    pieces(written).len()
}
