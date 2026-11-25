//! Whether what a blueprint works out for itself is plain.
//!
//! A blueprint that ships is where somebody manages a workflow in plain
//! words: what exists, what is heard, what is done. What is decided is a
//! building block's, written in Rust and tested as a function, and the
//! blueprint asks it. So an expression in such a text says what a thing is
//! called, or asks one plain question of it, and nothing harder:
//!
//! - a name: a path of names and places, `params.column`,
//!   `steps.card.out.status`, `event.data.id`, `item`;
//! - a name compared with a word, a number or another name, or looked for
//!   among several: `==`, `!=`, `in`;
//! - a name held against a number written out: `<`, `<=`, `>`, `>=`;
//! - a name with what stands for it when it says nothing: `a ?? b`;
//! - `!` of one of those;
//! - and at most three of those joined, all by `&&` or all by `||`.
//!
//! Text with such holes in it is plain. A function that is called,
//! arithmetic, `? :`, a sentence put together with `+`, which of two names
//! is the greater, `&&` and `||` in one expression and four things joined
//! are not: each is something worked out, and what is worked out is a
//! block's.
//!
//! This is the language's own reading (`expr::parse`, `Template`, `Cond`),
//! so that what is counted here is what the runner would run. Nothing in
//! the language is refused by it: a project's own blueprint may say what it
//! likes. It is what the texts that ship are held to (`cargo xtask check`,
//! the gate `plain`).

use serde_json::Value as Written;

use crate::expr::{BinaryOp, Expr, UnaryOp};
use crate::syntax::{self, Node, Value};
use crate::template::{Cond, Template, has_holes, holes};

/// An expression of a text that is not plain: where it is, what it says,
/// and what about it is worked out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unplain {
    /// The line of the text its value begins on, from 1.
    pub line: u32,
    /// The expression as it is written, without its `${ }`.
    pub source: String,
    pub why: &'static str,
}

/// What a value is written under where the whole of it is an expression,
/// with or without `${ }` (`lower`'s `cond`).
const CONDITIONS: &[&str] = &["when", "for_each"];

/// Every expression of a blueprint's text that is not plain, in the order
/// they are written; or what is wrong with the text, where it cannot be
/// read at all.
pub fn unplain(source: &str) -> Result<Vec<Unplain>, String> {
    let tree =
        syntax::parse(source).map_err(|err| format!("line {}: {}", err.span.start.line, err.message))?;
    let mut found = Vec::new();
    walk(&tree, None, &mut found)?;
    Ok(found)
}

fn walk(node: &Node, under: Option<&str>, found: &mut Vec<Unplain>) -> Result<(), String> {
    let line = node.span.start.line;
    match &node.value {
        Value::Str(text) if under.is_some_and(|key| CONDITIONS.contains(&key)) => {
            // The whole of it is one expression, however it is written. One
            // that does not read as an expression is somebody's own words
            // under a key of that name, and is no concern of this.
            match Cond::parse(text) {
                Ok(cond) => judge(&cond.expr, &cond.source, line, found),
                Err(err) if has_holes(text) => return Err(format!("line {line}: {}", err.message)),
                Err(_) => {}
            }
        }
        Value::Str(text) if has_holes(text) => {
            let unread = |err: crate::expr::ExprError| format!("line {line}: {}", err.message);
            let template = Template::parse(text).map_err(unread)?;
            for (expr, written) in template.exprs().zip(holes(text).map_err(unread)?) {
                judge(expr, written, line, found);
            }
        }
        Value::Seq(items) => {
            for item in items {
                walk(item, None, found)?;
            }
        }
        Value::Map(entries) => {
            for entry in entries {
                walk(&entry.value, Some(&entry.key), found)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn judge(expr: &Expr, written: &str, line: u32, found: &mut Vec<Unplain>) {
    if let Err(why) = plain(expr) {
        found.push(Unplain { line, source: written.trim().to_string(), why });
    }
}

/// Whether an expression is plain; if not, what about it is worked out.
pub fn plain(expr: &Expr) -> Result<(), &'static str> {
    match expr {
        Expr::Binary(by @ (BinaryOp::And | BinaryOp::Or), _, _) => {
            let mut joined = Vec::new();
            gather(expr, *by, &mut joined)?;
            if joined.len() > MOST_JOINED {
                return Err("it joins more than three things");
            }
            joined.into_iter().try_for_each(question)
        }
        other => question(other),
    }
}

/// How many questions one expression may join. Three read as a sentence
/// (`this and that and the other`); a fourth is a rule, and a rule is a
/// block's.
const MOST_JOINED: usize = 3;

/// The questions an expression joins, where one word joins them all. Both
/// words in one expression need to be told how they group, which is working
/// something out however it is bracketed.
fn gather<'a>(expr: &'a Expr, by: BinaryOp, joined: &mut Vec<&'a Expr>) -> Result<(), &'static str> {
    match expr {
        Expr::Binary(other @ (BinaryOp::And | BinaryOp::Or), left, right) => {
            if *other != by {
                return Err("it joins with both `&&` and `||`");
            }
            gather(left, by, joined)?;
            gather(right, by, joined)
        }
        one => {
            joined.push(one);
            Ok(())
        }
    }
}

/// One plain question, or one plain thing: a name, a name with what stands
/// for it, either compared, or `!` of one of those.
fn question(expr: &Expr) -> Result<(), &'static str> {
    match expr {
        Expr::Unary(UnaryOp::Not, inner) => match inner.as_ref() {
            Expr::Unary(UnaryOp::Not, _) => Err("it says `!` of a `!`"),
            inner => question(inner),
        },
        Expr::Binary(BinaryOp::Eq | BinaryOp::Ne, left, right) => {
            thing(left)?;
            thing(right)
        }
        Expr::Binary(BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge, left, right) => {
            // Against a number written out, either way round. Which of two
            // names is the greater is the kind of thing that grows a rule
            // behind it, and is a block's to say.
            match (number(left), number(right)) {
                (false, true) => thing(left),
                (true, false) => thing(right),
                _ => {
                    thing(left)?;
                    thing(right)?;
                    Err("it asks which of two names is the greater")
                }
            }
        }
        Expr::Binary(BinaryOp::In, left, right) => {
            thing(left)?;
            match right.as_ref() {
                Expr::List(items) => items.iter().try_for_each(thing),
                other => thing(other),
            }
        }
        other => thing(other),
    }
}

/// A name, a word or a number, or a name with what stands for it when it
/// says nothing.
fn thing(expr: &Expr) -> Result<(), &'static str> {
    match expr {
        Expr::Binary(BinaryOp::Coalesce, left, right) => {
            // What stands for a thing may itself have something standing
            // for it, written either way round: `a ?? b ?? c`.
            match left.as_ref() {
                Expr::Binary(BinaryOp::Coalesce, _, _) => thing(left)?,
                other => named(other)?,
            }
            match right.as_ref() {
                Expr::Binary(BinaryOp::Coalesce, _, _) => thing(right),
                Expr::List(items) if items.is_empty() => Ok(()),
                other => named(other),
            }
        }
        other => named(other),
    }
}

/// A number written out, below nothing included.
fn number(expr: &Expr) -> bool {
    match expr {
        Expr::Lit(Written::Number(_)) => true,
        Expr::Unary(UnaryOp::Neg, inner) => matches!(inner.as_ref(), Expr::Lit(Written::Number(_))),
        _ => false,
    }
}

/// A name of a thing, or a word or number written out.
fn named(expr: &Expr) -> Result<(), &'static str> {
    match expr {
        Expr::Lit(_) | Expr::Var(_) => Ok(()),
        Expr::Unary(UnaryOp::Neg, inner) if matches!(inner.as_ref(), Expr::Lit(_)) => Ok(()),
        Expr::Member(base, _) => path(base),
        Expr::Index(base, index) => {
            path(base)?;
            match index.as_ref() {
                Expr::Lit(_) => Ok(()),
                index => path(index).map_err(|_| "it looks a thing up by something worked out"),
            }
        }
        other => Err(why(other)),
    }
}

/// A path of names and places, and nothing else: no word written out is
/// asked for a part of itself.
fn path(expr: &Expr) -> Result<(), &'static str> {
    match expr {
        Expr::Var(_) => Ok(()),
        Expr::Member(_, _) | Expr::Index(_, _) => named(expr),
        other => Err(why(other)),
    }
}

/// What about an expression that is no name is worked out.
fn why(expr: &Expr) -> &'static str {
    match expr {
        Expr::Call(_, _) => "it calls a function",
        Expr::Ternary(_, _, _) => "it chooses with `? :`",
        Expr::Binary(BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem, _, _) => {
            "it adds, takes away, multiplies or divides"
        }
        Expr::Binary(BinaryOp::And | BinaryOp::Or, _, _) => "it joins things where one thing is wanted",
        Expr::Binary(
            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::In
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge,
            _,
            _,
        ) => "it compares what was itself compared",
        Expr::Binary(BinaryOp::Coalesce, _, _) => "what stands for a thing is itself worked out",
        Expr::Unary(UnaryOp::Neg, _) => "it takes a thing from nothing",
        Expr::Unary(UnaryOp::Not, _) => "it says `!` where a name is wanted",
        Expr::List(_) => "it makes a list",
        Expr::Lit(_) | Expr::Var(_) | Expr::Member(_, _) | Expr::Index(_, _) => "it is not a name",
    }
}
