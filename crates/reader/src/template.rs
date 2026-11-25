//! Strings with `${ ... }` holes.
//!
//! A string that is exactly one hole keeps the hole's type, so
//! `port: ${params.port}` is a number. Any other mix is text. `$${` is a
//! literal `${`.
//!
//! Holes are only ever found inside string values of the parsed tree. A
//! param can therefore change a value but never add a key, re-type a node or
//! alter the shape of the blueprint.

use serde_json::{Map, Value};

use crate::expr::{self, Expr, ExprError};

#[derive(Clone, Debug, PartialEq)]
pub struct Template {
    parts: Vec<Part>,
}

#[derive(Clone, Debug, PartialEq)]
enum Part {
    Text(String),
    Hole(Expr),
}

pub fn has_holes(text: &str) -> bool {
    text.contains("${")
}

/// A text taken apart: what is written between its holes, and what is
/// written in each, with where the hole's inside begins.
enum Piece<'a> {
    Text(String),
    Hole { written: &'a str, at: usize },
}

fn pieces(text: &str) -> Result<Vec<Piece<'_>>, ExprError> {
    let bytes = text.as_bytes();
    let mut pieces = Vec::new();
    let mut literal = String::new();
    let mut at = 0;
    while at < bytes.len() {
        let rest = &text[at..];
        if rest.starts_with("$${") {
            literal.push_str("${");
            at += 3;
        } else if rest.starts_with("${") {
            let close = closing_brace(text, at + 1)
                .ok_or_else(|| ExprError { message: "this `${` is never closed".into(), offset: at })?;
            if !literal.is_empty() {
                pieces.push(Piece::Text(std::mem::take(&mut literal)));
            }
            pieces.push(Piece::Hole { written: &text[at + 2..close], at: at + 2 });
            at = close + 1;
        } else {
            let ch = rest.chars().next().expect("at is inside the text");
            literal.push(ch);
            at += ch.len_utf8();
        }
    }
    if !literal.is_empty() {
        pieces.push(Piece::Text(literal));
    }
    Ok(pieces)
}

/// What is written in each hole of a text, in order and as it was written:
/// the holes [`Template::parse`] reads, for whoever has something to say of
/// an expression in its writer's own words.
pub fn holes(text: &str) -> Result<Vec<&str>, ExprError> {
    Ok(pieces(text)?
        .into_iter()
        .filter_map(|piece| match piece {
            Piece::Hole { written, .. } => Some(written),
            Piece::Text(_) => None,
        })
        .collect())
}

impl Template {
    pub fn parse(text: &str) -> Result<Template, ExprError> {
        let mut parts = Vec::new();
        for piece in pieces(text)? {
            parts.push(match piece {
                Piece::Text(text) => Part::Text(text),
                Piece::Hole { written, at } => Part::Hole(
                    expr::parse(written)
                        .map_err(|err| ExprError { message: err.message, offset: at + err.offset })?,
                ),
            });
        }
        Ok(Template { parts })
    }

    /// The expression, when the whole string is one hole.
    pub fn single(&self) -> Option<&Expr> {
        match self.parts.as_slice() {
            [Part::Hole(expr)] => Some(expr),
            _ => None,
        }
    }

    pub fn exprs(&self) -> impl Iterator<Item = &Expr> {
        self.parts.iter().filter_map(|part| match part {
            Part::Hole(expr) => Some(expr),
            Part::Text(_) => None,
        })
    }

    /// The text with each hole said by `hole` where it stands, in place of
    /// being filled: for whoever says in words what a text will come to.
    pub fn said(&self, hole: &dyn Fn(&Expr) -> String) -> String {
        let said = |part: &Part| match part {
            Part::Text(text) => text.clone(),
            Part::Hole(expr) => hole(expr),
        };
        self.parts.iter().map(said).collect()
    }

    pub fn render(&self, scope: &Map<String, Value>) -> Result<Value, String> {
        if let Some(expr) = self.single() {
            return expr.eval(scope);
        }
        let mut out = String::new();
        for part in &self.parts {
            match part {
                Part::Text(text) => out.push_str(text),
                Part::Hole(expr) => out.push_str(&expr::display(&expr.eval(scope)?)),
            }
        }
        Ok(Value::String(out))
    }
}

/// Offset of the `}` matching the `{` at `open`.
pub fn closing_brace(text: &str, open: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut at = open;
    while at < bytes.len() {
        match bytes[at] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(at);
                }
            }
            quote @ (b'"' | b'\'') => {
                at += 1;
                while at < bytes.len() && bytes[at] != quote {
                    at += if bytes[at] == b'\\' { 2 } else { 1 };
                }
            }
            _ => {}
        }
        at += 1;
    }
    None
}

/// The key under which a mapping hands on another mapping's entries:
/// `with: { ...: ${params}, back: true }` is everything in `params`, with
/// `back` said beside it.
pub const HANDED_ON: &str = "...";

/// Fills every hole in `value`, recursively. A mapping that says `...`
/// begins from the mapping that comes to.
pub fn render(value: &Value, scope: &Map<String, Value>) -> Result<Value, String> {
    Ok(match value {
        Value::String(text) if has_holes(text) => {
            Template::parse(text).map_err(|err| err.message)?.render(scope)?
        }
        Value::Array(items) => {
            Value::Array(items.iter().map(|item| render(item, scope)).collect::<Result<_, _>>()?)
        }
        Value::Object(map) => {
            let mut out = Map::with_capacity(map.len());
            // `...: ${ a mapping }` hands on what that mapping holds: its
            // entries come first, and what is written beside it is said
            // over them.
            match map.get(HANDED_ON).map(|handed| render(handed, scope)).transpose()? {
                None | Some(Value::Null) => {}
                Some(Value::Object(handed)) => out.extend(handed),
                Some(other) => {
                    return Err(format!(
                        "`{HANDED_ON}` hands on a mapping, and was given {}",
                        crate::expr::display(&other)
                    ));
                }
            }
            for (key, item) in map.iter().filter(|(key, _)| *key != HANDED_ON) {
                out.insert(key.clone(), render(item, scope)?);
            }
            Value::Object(out)
        }
        other => other.clone(),
    })
}

/// Calls `visit` with every string in `value` that has holes.
pub fn each_template(value: &Value, visit: &mut dyn FnMut(&str)) {
    match value {
        Value::String(text) if has_holes(text) => visit(text),
        Value::Array(items) => items.iter().for_each(|item| each_template(item, visit)),
        Value::Object(map) => map.values().for_each(|item| each_template(item, visit)),
        _ => {}
    }
}

/// A condition: either `${ expr }` or a bare expression.
#[derive(Clone, Debug, PartialEq)]
pub struct Cond {
    pub source: String,
    pub expr: Expr,
}

impl Cond {
    pub fn parse(text: &str) -> Result<Cond, ExprError> {
        let trimmed = text.trim();
        let inner = match trimmed.strip_prefix("${").and_then(|rest| rest.strip_suffix('}')) {
            Some(inner) if closing_brace(trimmed, 1) == Some(trimmed.len() - 1) => inner,
            _ => trimmed,
        };
        Ok(Cond { source: inner.trim().to_string(), expr: expr::parse(inner)? })
    }

    pub fn test(&self, scope: &Map<String, Value>) -> Result<bool, String> {
        self.expr.eval(scope).map(|value| expr::truthy(&value))
    }
}
