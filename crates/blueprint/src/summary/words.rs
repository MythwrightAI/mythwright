//! What a blueprint writes for the runner, said in words for a person: a
//! name (`params.table`), a plain question of one (`when`), and a value
//! with holes in it.
//!
//! A name is said by what it is a name of: the param, what a step gave,
//! what the event carries. A condition is said as the question it asks
//! where it is plain ([`crate::plain`]); one that works something out is
//! not put into words here, and whoever asks says it as it is written.
//! Nothing here leaves a `${` in what it says.

use serde_json::Value;

use crate::expr::{self, BinaryOp, Expr, UnaryOp};
use crate::plain::plain;
use crate::template::{Cond, Template, has_holes};

/// Something said two ways: that it is so, and that it is not.
pub(crate) struct Asked {
    pub so: String,
    pub not: String,
}

impl Asked {
    fn turned(self) -> Asked {
        Asked { so: self.not, not: self.so }
    }
}

/// What an expression names: a thing, or something that is already
/// either so or not (`steps.build.failed`).
enum Named {
    Thing(String),
    Fact(Asked),
}

impl Named {
    fn thing(self) -> String {
        match self {
            Named::Thing(words) => words,
            Named::Fact(asked) => format!("whether {}", asked.so),
        }
    }

    fn asked(self) -> Asked {
        match self {
            Named::Thing(words) => Asked { so: format!("{words} is so"), not: format!("{words} is not so") },
            Named::Fact(asked) => asked,
        }
    }
}

/// A word or a number as it is written, set apart from the words about it.
fn written(value: &Value) -> String {
    match value {
        Value::Null => "nothing".to_string(),
        Value::String(text) if text.is_empty() => "nothing".to_string(),
        // On one line, whatever it was written on.
        Value::String(text) => format!("`{}`", text.split_whitespace().collect::<Vec<_>>().join(" ")),
        Value::Array(items) if items.is_empty() => "nothing".to_string(),
        other => format!("`{other}`"),
    }
}

/// Whether a literal says nothing: what stands for a name where the name
/// says nothing is then no more than the name.
fn says_nothing(expr: &Expr) -> bool {
    match expr {
        Expr::Lit(Value::Null) => true,
        Expr::Lit(Value::String(text)) => text.is_empty(),
        Expr::Lit(Value::Bool(flag)) => !flag,
        Expr::List(items) => items.is_empty(),
        _ => false,
    }
}

/// The names from the root down, where an expression is a path of names
/// and places written out.
fn path(expr: &Expr) -> Option<(&str, Vec<String>)> {
    match expr {
        Expr::Var(root) => Some((root, Vec::new())),
        Expr::Member(base, name) => {
            let (root, mut parts) = path(base)?;
            parts.push(name.clone());
            Some((root, parts))
        }
        Expr::Index(base, index) => match index.as_ref() {
            Expr::Lit(place) => {
                let (root, mut parts) = path(base)?;
                parts.push(expr::display(place));
                Some((root, parts))
            }
            _ => None,
        },
        _ => None,
    }
}

/// `a.b` of something, or the thing itself where nothing of it is named.
fn of(parts: &[String], whole: &str) -> String {
    if parts.is_empty() { whole.to_string() } else { format!("`{}` of {whole}", parts.join(".")) }
}

/// What a step says of how it went, by the word after its name.
fn of_step(step: &str, parts: &[String]) -> Named {
    let fact =
        |so: &str, not: &str| Named::Fact(Asked { so: format!("{step} {so}"), not: format!("{step} {not}") });
    match parts.split_first().map(|(first, rest)| (first.as_str(), rest)) {
        Some(("failed", [])) => fact("failed", "did not fail"),
        Some(("ok", [])) => fact("went well", "did not go well"),
        Some(("skipped", [])) => fact("was passed over", "was not passed over"),
        Some(("out", [])) => Named::Thing(format!("what {step} gave")),
        Some(("out", given)) => Named::Thing(format!("`{}` from {step}", given.join("."))),
        Some(("reason", [])) => Named::Thing(format!("why {step} failed")),
        Some(("error", [])) => Named::Thing(format!("why {step} failed, in full")),
        Some(("duration_ms", [])) => Named::Thing(format!("how long {step} took")),
        _ => Named::Thing(of(parts, step)),
    }
}

/// A path of names said by what it is a name of.
fn of_path(root: &str, parts: &[String]) -> Named {
    let first = parts.first().map(String::as_str);
    let rest = parts.get(1..).unwrap_or_default();
    let thing = |words: String| Named::Thing(words);
    match (root, first) {
        ("params", Some(param)) => thing(of(rest, &format!("the param `{param}`"))),
        ("params", None) => thing("everything it was told".to_string()),
        ("state", Some(kept)) => thing(of(rest, &format!("what it keeps as `{kept}`"))),
        ("state", None) => thing("everything it keeps".to_string()),
        ("steps", Some(step)) => of_step(&format!("the step `{step}`"), rest),
        ("self", _) => of_step("this step", parts),
        ("event", Some("data")) if rest.is_empty() => thing("what the event carries".to_string()),
        ("event", Some("data")) => thing(format!("the event's `{}`", rest.join("."))),
        ("event", Some("topic")) if rest.is_empty() => thing("the event's topic".to_string()),
        ("event", Some("seq")) if rest.is_empty() => thing("the event's number".to_string()),
        ("event", Some("source")) if rest == ["actor"] => thing("who the event says did it".to_string()),
        ("event", Some("source")) if rest == ["instance"] => {
            thing("the instance that said the event".to_string())
        }
        ("event", _) => thing(of(parts, "the event")),
        ("item", _) => thing(of(parts, "the item")),
        ("index", None) => thing("the item's place in its list".to_string()),
        ("instance", Some("project")) if rest.is_empty() => thing("its project".to_string()),
        ("instance", Some("id")) if rest.is_empty() => thing("its own id".to_string()),
        ("instance", _) => thing(of(parts, "this instance")),
        ("target", _) => thing(of(parts, "the instance it is attached to")),
        ("environment", Some("name")) if rest.is_empty() => thing("the environment's name".to_string()),
        ("environment", Some("vars")) if rest.len() == 1 => {
            thing(format!("the environment's variable `{}`", rest[0]))
        }
        ("environment", _) => thing(of(parts, "the environment it was applied under")),
        ("profile", Some("name")) if rest.is_empty() => thing("the profile's name".to_string()),
        ("profile", Some("vars")) if rest.len() == 1 => {
            thing(format!("the profile's variable `{}`", rest[0]))
        }
        ("profile", _) => thing(of(parts, "the profile")),
        ("was", Some("params")) if rest.len() == 1 => {
            thing(format!("the param `{}` as it was before this text", rest[0]))
        }
        ("was", Some("state")) if rest.len() == 1 => {
            thing(format!("what it kept as `{}` before this text", rest[0]))
        }
        ("was", _) => thing(of(parts, "the instance as it was before this text")),
        ("run", Some("status")) if rest.is_empty() => thing("how its steps ended".to_string()),
        ("run", Some("error")) if rest.is_empty() => thing("why its steps failed".to_string()),
        ("run", _) => thing(of(parts, "the run of its steps")),
        _ => {
            let whole: Vec<&str> = std::iter::once(root).chain(parts.iter().map(String::as_str)).collect();
            thing(format!("`{}`", whole.join(".")))
        }
    }
}

/// A plain thing in words: a name, a word or number written out, or a
/// name with what stands for it where it says nothing.
fn named(expr: &Expr) -> Option<Named> {
    Some(match expr {
        Expr::Lit(value) => Named::Thing(written(value)),
        Expr::Unary(UnaryOp::Neg, inner) => match inner.as_ref() {
            Expr::Lit(value) => Named::Thing(format!("`-{value}`")),
            _ => return None,
        },
        Expr::Binary(BinaryOp::Coalesce, left, right) => {
            let left = named(left)?;
            if says_nothing(right) {
                return Some(left);
            }
            Named::Thing(format!("{} (or {}, where that says nothing)", left.thing(), named(right)?.thing()))
        }
        Expr::Index(base, index) if path(expr).is_none() => {
            Named::Thing(format!("{} at {}", named(base)?.thing(), named(index)?.thing()))
        }
        Expr::List(items) => Named::Thing(listed(items, "and")?),
        Expr::Var(_) | Expr::Member(_, _) | Expr::Index(_, _) => {
            let (root, parts) = path(expr)?;
            of_path(root, &parts)
        }
        _ => return None,
    })
}

/// Several things in a row, the last joined by `by`.
fn listed(items: &[Expr], by: &str) -> Option<String> {
    let said: Vec<String> = items.iter().map(|item| named(item).map(Named::thing)).collect::<Option<_>>()?;
    Some(joined(&said, by))
}

pub(crate) fn joined(said: &[String], by: &str) -> String {
    match said {
        [] => "nothing".to_string(),
        [one] => one.clone(),
        [first @ .., last] => format!("{} {by} {last}", first.join(", ")),
    }
}

fn number(expr: &Expr) -> bool {
    match expr {
        Expr::Lit(Value::Number(_)) => true,
        Expr::Unary(UnaryOp::Neg, inner) => matches!(inner.as_ref(), Expr::Lit(Value::Number(_))),
        _ => false,
    }
}

/// One plain question in words, both ways.
fn question(expr: &Expr) -> Option<Asked> {
    use BinaryOp::*;
    Some(match expr {
        Expr::Unary(UnaryOp::Not, inner) => question(inner)?.turned(),
        Expr::Binary(Eq, left, right) => match (left.as_ref(), right.as_ref()) {
            (other, Expr::Lit(Value::Bool(flag))) | (Expr::Lit(Value::Bool(flag)), other) => {
                let asked = named(other)?.asked();
                if *flag { asked } else { asked.turned() }
            }
            // Held against nothing: whether it says anything at all.
            (other, nothing) | (nothing, other) if says_nothing(nothing) => {
                let thing = named(other)?.thing();
                Asked { so: format!("{thing} says nothing"), not: format!("{thing} says something") }
            }
            _ => {
                let (left, right) = (named(left)?.thing(), named(right)?.thing());
                Asked { so: format!("{left} is {right}"), not: format!("{left} is not {right}") }
            }
        },
        Expr::Binary(Ne, left, right) => question(&Expr::Binary(Eq, left.clone(), right.clone()))?.turned(),
        Expr::Binary(by @ (Lt | Le | Gt | Ge), left, right) => {
            // Against a number written out, whichever side it is on.
            let (thing, bound, by) = if number(right) {
                (left, right, *by)
            } else {
                let turned = match by {
                    Lt => Gt,
                    Le => Ge,
                    Gt => Lt,
                    _ => Le,
                };
                (right, left, turned)
            };
            let (thing, bound) = (named(thing)?.thing(), named(bound)?.thing());
            let (so, not) = match by {
                Lt => ("less than", "at least"),
                Le => ("at most", "more than"),
                Gt => ("more than", "at most"),
                _ => ("at least", "less than"),
            };
            Asked { so: format!("{thing} is {so} {bound}"), not: format!("{thing} is {not} {bound}") }
        }
        Expr::Binary(In, left, right) => {
            let left = named(left)?.thing();
            match right.as_ref() {
                Expr::List(items) => {
                    let among = listed(items, "or")?;
                    Asked {
                        so: format!("{left} is one of {among}"),
                        not: format!("{left} is none of {among}"),
                    }
                }
                other => {
                    let among = named(other)?.thing();
                    Asked {
                        so: format!("{left} is among {among}"),
                        not: format!("{left} is not among {among}"),
                    }
                }
            }
        }
        other => named(other)?.asked(),
    })
}

/// The questions an expression joins by one word.
fn gathered<'a>(expr: &'a Expr, by: BinaryOp, found: &mut Vec<&'a Expr>) {
    match expr {
        Expr::Binary(other, left, right) if *other == by => {
            gathered(left, by, found);
            gathered(right, by, found);
        }
        one => found.push(one),
    }
}

/// A condition as the question it asks, where it is plain; nothing where
/// it works something out.
fn asked(expr: &Expr) -> Option<String> {
    plain(expr).ok()?;
    let (by, word) = match expr {
        Expr::Binary(BinaryOp::And, _, _) => (BinaryOp::And, "and"),
        Expr::Binary(BinaryOp::Or, _, _) => (BinaryOp::Or, "or"),
        other => return Some(question(other)?.so),
    };
    let mut parts = Vec::new();
    gathered(expr, by, &mut parts);
    let said: Vec<String> =
        parts.into_iter().map(|part| question(part).map(|asked| asked.so)).collect::<Option<_>>()?;
    Some(joined(&said, word))
}

/// A condition in words: `only when …` reads after it. One that is not
/// plain is said to be a condition, as it is written.
pub(crate) fn condition(cond: &Cond) -> String {
    asked(&cond.expr).unwrap_or_else(|| format!("a condition holds, written `{}`", cond.source))
}

/// What a `for_each` goes over, in words.
pub(crate) fn each(cond: &Cond) -> String {
    match plain(&cond.expr).ok().and_then(|()| named(&cond.expr)) {
        Some(named) => named.thing(),
        None => format!("what `{}` comes to", cond.source),
    }
}

/// The words of a name without the marks that set a word apart, for
/// saying it inside something that is itself set apart.
fn bare(words: &str) -> String {
    words.replace('`', "")
}

/// What a hole comes to, in words.
fn hole(expr: &Expr) -> String {
    match plain(expr).ok().and_then(|()| named(expr)) {
        Some(named) => named.thing(),
        None => "something worked out when it runs".to_string(),
    }
}

/// A value as a blueprint writes it, in words: a word set apart as it is
/// written, a name where the whole of it is a hole, and a text with holes
/// as the text with each hole said in angle brackets.
pub(crate) fn value(value: &Value) -> String {
    match value {
        Value::String(text) if has_holes(text) => match Template::parse(text) {
            Ok(template) => match template.single() {
                Some(expr) => hole(expr),
                None => {
                    let said = template.said(&|expr| format!("<{}>", bare(&hole(expr))));
                    format!("`{}`", said.split_whitespace().collect::<Vec<_>>().join(" "))
                }
            },
            Err(_) => "something worked out when it runs".to_string(),
        },
        Value::Array(items) if !items.is_empty() => {
            // Each word as it is written, and one that is a hole by what says it.
            let word = |item: &Value| match says_it(item) {
                Some(says) => format!("<{}>", bare(&says)),
                None => bare(&self::value(item)),
            };
            format!("`{}`", items.iter().map(word).collect::<Vec<_>>().join(" "))
        }
        Value::Object(_) => "a mapping".to_string(),
        other => written(other),
    }
}

/// What says a value, where the whole of it is one hole: the name in the
/// hole, in words. Nothing for a value that is written out, or is a text
/// with holes in it.
pub(crate) fn says_it(value: &Value) -> Option<String> {
    let text = value.as_str().filter(|text| has_holes(text))?;
    Template::parse(text).ok()?.single().map(hole)
}

/// `called …`, to go after what a thing is: by its name where that is
/// written out, and by what says its name where it is not.
pub(crate) fn called(name: &Value) -> String {
    match says_it(name) {
        Some(says) => format!("called what {says} says"),
        None => format!("called {}", value(name)),
    }
}

/// A length of time in words: `90000` is `1 minute 30 seconds`.
pub(crate) fn long(millis: u64) -> String {
    let units =
        [("day", 86_400_000), ("hour", 3_600_000), ("minute", 60_000), ("second", 1_000), ("millisecond", 1)];
    let mut left = millis;
    let mut said = Vec::new();
    for (unit, scale) in units {
        let many = left / scale;
        left %= scale;
        match many {
            0 => {}
            1 => said.push(format!("1 {unit}")),
            many => said.push(format!("{many} {unit}s")),
        }
    }
    if said.is_empty() { "no time".to_string() } else { said.join(" ") }
}

/// A length of time as a blueprint writes one (`5m`), in words; what
/// says it, where it is not written out.
pub(crate) fn lasting(value: &Value) -> String {
    match value.as_str().map(crate::duration::parse) {
        Some(Ok(millis)) => long(millis),
        _ => self::value(value),
    }
}
