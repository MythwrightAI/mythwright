//! Why a text that was kept no longer checks.
//!
//! What a blueprint can depend on is written down with the code, and so is
//! every time some of it was taken away or changed, with the day and why
//! (`contract/broken.txt`, kept by a test in `api`). A text this conductor
//! kept was checked when it was applied; if it no longer checks, the likely
//! reason is on that list, and saying it is better than a guess.

use blueprint::Diagnostic;

const LIST: &str = include_str!("../../../contract/broken.txt");

/// What the list says of what `errors` complain of, a line for each thing
/// that was broken; or, where it says nothing of them and they are of
/// something that is not here, that the text may be from an earlier
/// version. Empty for a text whose faults are of another kind.
pub(crate) fn why(errors: &[&Diagnostic]) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for line in LIST.lines().filter(|line| !line.starts_with('#')) {
        let mut parts = line.splitn(4, '\t');
        let (Some(day), Some(what), Some(became), Some(why)) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        // `action board.comment`, or `event board.item.moved: `by``.
        let named = what.split_once(' ').map_or(what, |(_, named)| named);
        let (name, part) = match named.split_once(": ") {
            Some((name, part)) => (name, Some(part.trim_matches('`'))),
            None => (named, None),
        };
        let about = |error: &&Diagnostic| {
            let said = &error.message;
            said.contains(&format!("`{name}`"))
                && part.is_none_or(|part| {
                    said.contains(&format!("`{part}`")) || said.contains(&format!(".{part}`"))
                })
        };
        let line = format!("`{named}` {became} on {day}: {why}");
        if errors.iter().any(about) && !found.contains(&line) {
            found.push(line);
        }
    }
    if found.is_empty() && errors.iter().any(|error| MISSING.contains(&error.code)) {
        found.push(format!(
            "The text may have been written for an earlier version of this program than this one, which is {}",
            brand::VERSION
        ));
    }
    found
}

/// The faults that say something a text uses is not here.
const MISSING: &[&str] = &["unknown-action", "unknown-argument", "missing-argument", "wrong-type"];
