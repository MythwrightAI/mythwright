//! Problems found in a blueprint, with where they are.

use serde::Serialize;

use crate::syntax::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    /// A stable short name for the kind of problem, e.g. `unknown-action`.
    pub code: &'static str,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn error(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Self { severity: Severity::Error, code, message: message.into(), span }
    }

    pub fn warning(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Self { severity: Severity::Warning, code, message: message.into(), span }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// `file:line:col: error: message`, the form editors and terminals link.
    pub fn render(&self, file: &str) -> String {
        let level = if self.is_error() { "error" } else { "warning" };
        format!("{file}:{}:{}: {level}: {}", self.span.start.line, self.span.start.col, self.message)
    }
}

/// The closest of `candidates` to `word`, if any is close enough to be a typo.
pub fn did_you_mean<'a>(word: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let limit = word.len().div_ceil(3).max(1);
    candidates
        .into_iter()
        .map(|candidate| (edit_distance(word, candidate), candidate))
        .filter(|(distance, _)| *distance <= limit)
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, candidate)| candidate)
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let substitute = diagonal + usize::from(ca != *cb);
            diagonal = row[j + 1];
            row[j + 1] = substitute.min(row[j] + 1).min(diagonal + 1);
        }
    }
    row[b.len()]
}
