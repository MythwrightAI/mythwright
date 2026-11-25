//! A text written before 0.1.0, told so.
//!
//! Before 0.1.0 `environment` in an expression was the copy of the
//! factory a blueprint runs in (`environment.name`, `environment.vars`),
//! and `environment.set` set its variables. Before the first release that
//! was renamed: the copy of the factory is
//! a **profile** (`profile.name`, `profile.vars`, `profile.set`), and
//! `environment` is the environment of its project's an instance was
//! applied under (dev, staging, prod). A text still written the old way
//! reads, and means something else: nothing in it says which was meant.
//!
//! What can be told is a text that sets the profile's variables and reads
//! `environment.*`: written the new way it would read what it sets as
//! `profile.vars`, so it was almost certainly written before. That is
//! warned of, once, where it first reads `environment`. The rest is said
//! in the notes of the release (`docs/releases.md`).

use crate::diag::Diagnostic;
use crate::syntax::{Pos, Span};

/// What a text that sets the profile's variables and reads `environment.*`
/// is told, where it first reads it.
pub(crate) fn told(source: &str, diagnostics: &mut Vec<Diagnostic>) {
    let sets_profile = source.lines().any(|line| {
        let line = line.trim().trim_start_matches("- ");
        line.strip_prefix("do:").map(str::trim).map(|action| action.trim_matches('"')) == Some("profile.set")
    });
    if !sets_profile {
        return;
    }
    let Some((line, col)) = first_read(source) else { return };
    let at = Pos { line, col };
    diagnostics.push(Diagnostic::warning(
        "environment-renamed",
        "this text sets the profile's variables with `profile.set`, and reads `environment`, which since 0.1.0 is \
         the environment of its project's an instance was applied under and not the profile: where the profile's \
         were meant, write `profile.name` and `profile.vars.<name>`",
        Span { start: at, end: Pos { line, col: col + "environment".len() as u32 } },
    ));
}

/// The first place, 1-based, where an expression reads `environment.`: the
/// word inside a `${ }`, after something that cannot be part of a name.
fn first_read(source: &str) -> Option<(u32, u32)> {
    for (number, line) in source.lines().enumerate() {
        let mut open = 0usize;
        while let Some(found) = line[open..].find("${") {
            let start = open + found + 2;
            let end = line[start..].find('}').map_or(line.len(), |close| start + close);
            let inside = &line[start..end];
            let mut from = 0;
            while let Some(at) = inside[from..].find("environment.") {
                let at = from + at;
                let before = inside[..at].chars().last();
                if !before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') {
                    return Some((number as u32 + 1, (start + at) as u32 + 1));
                }
                from = at + 1;
            }
            open = end;
        }
    }
    None
}
