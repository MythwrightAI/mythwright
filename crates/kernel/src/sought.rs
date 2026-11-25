//! What a search finds by what something is called, and where in a
//! document a passage it found stands: what the search index (`search`)
//! asks of a name, a heading and a passage, as plain functions.
//!
//! The index finds what is written in things (files, messages, rows); a
//! project, a channel, somebody or a run is found by what it is called,
//! read as it is asked for, so that it is never found by a name it no
//! longer has. Both read what was typed the same way: each word must
//! begin a word of what is found, so results narrow as somebody types.

/// The words of what was typed, in small letters: what each word of what
/// is found is held to. A word is letters, digits and `_`.
pub fn words(typed: &str) -> Vec<String> {
    split(typed).map(str::to_lowercase).collect()
}

fn split(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_alphanumeric() && c != '_').filter(|word| !word.is_empty())
}

/// How well something called by `names` answers what was typed (`typed`,
/// as [`words`] reads it), or nothing where it does not: each word typed
/// begins a word of one of its names. The smaller is the better: 0 where
/// one of its names is what was typed, 1 where one begins with it, 2
/// otherwise.
pub fn answers(typed: &[String], names: &[&str]) -> Option<u8> {
    if typed.is_empty() {
        return None;
    }
    let theirs: Vec<String> = names.iter().flat_map(|name| split(name)).map(str::to_lowercase).collect();
    if !typed.iter().all(|word| theirs.iter().any(|their| their.starts_with(word.as_str()))) {
        return None;
    }
    let whole = typed.join(" ");
    let said: Vec<String> =
        names.iter().map(|name| split(name).map(str::to_lowercase).collect::<Vec<_>>().join(" ")).collect();
    Some(if said.contains(&whole) {
        0
    } else if said.iter().any(|name| name.starts_with(&whole)) {
        1
    } else {
        2
    })
}

/// `name` with the start of each of its words that a word typed begins
/// between `start` and `end`, as the index marks what it matched.
pub fn marked(name: &str, typed: &[String], (start, end): (char, char)) -> String {
    let mut out = String::with_capacity(name.len() + 8);
    let mut rest = name;
    while let Some(at) = rest.find(|c: char| c.is_alphanumeric() || c == '_') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let length = rest.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(rest.len());
        let word = &rest[..length];
        let small = word.to_lowercase();
        // The longest word typed that begins it, counted in characters: a
        // word in small letters may be longer in bytes than as written.
        let begun =
            typed.iter().filter(|one| small.starts_with(one.as_str())).map(|one| one.chars().count()).max();
        match begun {
            Some(count) => {
                let cut = word.char_indices().nth(count).map_or(word.len(), |(at, _)| at);
                out.push(start);
                out.push_str(&word[..cut]);
                out.push(end);
                out.push_str(&word[cut..]);
            }
            None => out.push_str(word),
        }
        rest = &rest[length..];
    }
    out.push_str(rest);
    out
}

/// Where in `text` a passage the index cut from it stands (`passage`, with
/// `start` and `end` around what matched and ` … ` where it was cut): the
/// line, from 1, of the heading nearest above it, with that heading, or
/// of the passage itself where no heading is above it. Nothing where the
/// passage is not found in it.
pub fn heading_at(text: &str, passage: &str, (start, end): (char, char)) -> Option<(usize, Option<String>)> {
    let unmarked = |said: &str| said.chars().filter(|c| *c != start && *c != end).collect::<String>();
    // The piece that matched, between where the passage was cut, and how
    // far into it what matched begins.
    let marked = passage.split(" … ").find(|piece| piece.contains(start))?;
    let piece = unmarked(marked);
    let lead = piece.len() - piece.trim_start().len();
    let into = unmarked(&marked[..marked.find(start)?]).len().checked_sub(lead)?;
    let piece = piece.trim();
    if piece.is_empty() {
        return None;
    }
    let at = text.find(piece)? + into;
    let line = text[..at].matches('\n').count() + 1;
    let heading = text[..at]
        .lines()
        .enumerate()
        .filter(|(_, said)| said.starts_with('#') && said.trim_start_matches('#').starts_with(' '))
        .last()
        .map(|(index, said)| (index + 1, said.trim_start_matches('#').trim().to_string()));
    Some(match heading {
        Some((line, heading)) => (line, Some(heading)),
        None => (line, None),
    })
}
