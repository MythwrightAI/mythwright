//! Where in the text a name is written, for a refusal to point at; and
//! what was probably meant, where a name is close to one that is there.
//!
//! What a step, a handler or a hook is handed is kept as values, which say
//! nothing of where they were written. So a name read in one of them that
//! is not there was said to be wrong at the start of the step that reads
//! it: in a step of twenty lines that is not where, and an editor marked
//! the whole step. The text is there to be looked in. A name is found
//! where it is written, between the start and the end of what reads it,
//! and that is where the refusal points; where it is written some other
//! way than a name with dots (a place in brackets, a space about a dot)
//! it is not found, and the refusal points where it always did.

use crate::diag::did_you_mean;
use crate::syntax::{Pos, Span};
use crate::validate::Validator;

impl Validator<'_> {
    /// Where the name an expression reads (`path`, as `params.name`) is
    /// first written within `within`; `within` itself, where it is not.
    pub(crate) fn written(&self, path: &[String], within: Span) -> Span {
        let Some(root) = path.first() else { return within };
        let whole = match path.get(1) {
            Some(name) => format!("{root}.{name}"),
            None => root.clone(),
        };
        find(self.source, &whole, within).or_else(|| find(self.source, root, within)).unwrap_or(within)
    }

    /// What follows "there is no step": the step that was probably meant.
    pub(crate) fn close_step(&self, name: &str) -> String {
        fn ids<'a>(flow: &'a crate::model::Flow, found: &mut Vec<&'a str>) {
            for step in &flow.steps {
                found.push(step.id.as_str());
                ids(&step.inner, found);
            }
        }
        let mut found = Vec::new();
        for (_, flow) in self.blueprint.flows() {
            ids(flow, &mut found);
        }
        match did_you_mean(name, found) {
            Some(close) => format!("; did you mean `{close}`?"),
            None => String::new(),
        }
    }

    /// What follows "there is no state key": the one that was probably
    /// meant, or how one is declared.
    pub(crate) fn close_state(&self, name: &str) -> String {
        match did_you_mean(name, self.blueprint.state.keys().map(String::as_str)) {
            Some(close) => format!("; did you mean `{close}`?"),
            None => "; declare it under `state` with an initial value".to_string(),
        }
    }
}

/// Whether a name could go on with this character: a name read in an
/// expression is letters, digits and `_`, joined by dots.
fn of_a_name(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The first place `name` is written as a name of its own within
/// `within`, on a line that is not a comment: not as the end of a longer
/// name (`my_params.x`), or of a longer path (`event.data.params.x`), and
/// not as the start of a longer one (`params.names`).
fn find(source: &str, name: &str, within: Span) -> Option<Span> {
    let (first, last) = (within.start.line as usize, within.end.line as usize);
    for (index, line) in source.lines().enumerate().skip(first.saturating_sub(1)) {
        let number = index + 1;
        if number > last {
            break;
        }
        if line.trim_start().starts_with('#') {
            continue;
        }
        let mut from = 0;
        while let Some(at) = line[from..].find(name).map(|at| at + from) {
            from = at + name.len();
            let before = line[..at].chars().next_back();
            let after = line[from..].chars().next();
            let alone = !before.is_some_and(|c| of_a_name(c) || c == '.') && !after.is_some_and(of_a_name);
            // A column is counted in characters, as the reader counts it.
            let col = line[..at].chars().count() + 1;
            let inside = (number > first || col >= within.start.col as usize)
                && (number < last || col < within.end.col as usize);
            if alone && inside {
                let start = Pos { line: number as u32, col: col as u32 };
                let end = Pos { line: number as u32, col: (col + name.chars().count()) as u32 };
                return Some(Span { start, end });
            }
        }
    }
    None
}
