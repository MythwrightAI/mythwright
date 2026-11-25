//! One way of laying a blueprint out.
//!
//! People and agents write blueprints, and each has a hand of their own.
//! [`format`] writes a text again as anyone would have: the same
//! indentation and spacing, a blank line where one sets things apart, a
//! map in braces opened into a block when it has outgrown its line. What
//! the text means is never touched, and neither is anything that was a
//! choice of words: a comment, how a string is quoted, a sentence in a
//! `description`, the order things are said in.
//!
//! What is laid out is also YAML to anything else that reads YAML. The
//! reader here takes a few things written plain that YAML does not (a
//! value with `: ` in it, a `${ }` bare inside braces), and those are
//! written in quotes, where they are the same to both ([`not_yaml`] says
//! where a text has one). The reader goes on taking them as they were.
//!
//! The editor, the command line and the check that the blueprints which
//! ship are laid out this way all call this, so there is one opinion and
//! it is this file's and [`crate::layout`]'s. A project that wants another
//! says so in a file of its own ([`SETTINGS_FILE`]), in the few ways
//! listed in [`SETTINGS`]; finding that file is the caller's, since this
//! crate reads no files.
//!
//! Because a text can be laid out again without meaning anything else,
//! two texts can also be asked whether they mean the same
//! ([`means_the_same`]): a running instance is not behind a text that
//! differs from its own only in layout and comments.

use serde::Serialize;

use crate::syntax::{self, Node, SyntaxError, Value};
use crate::template;

/// The file a project says its own settings in, looked for in a
/// blueprint's folder and then in each folder above it.
pub const SETTINGS_FILE: &str = "blueprint-format.yaml";

/// When a map written in braces stays on its line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OneLine {
    /// While it fits the width; one that does not becomes a block.
    Fits,
    /// Never: a map in braces is always written as a block.
    Never,
    /// Always, however long the line.
    Always,
}

/// The spaces just inside `${` and `}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expressions {
    /// None around a bare name (`${params.item}`), one each side of
    /// anything else (`${ a == b }`).
    Mixed,
    /// One each side, always.
    Spaced,
    /// None, ever.
    Tight,
    /// Left as they were written.
    Written,
}

/// The order the fields of a step, a handler and the like are written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    /// As the author wrote them.
    Written,
    /// As the language lists them ([`crate::vocabulary`]).
    Vocabulary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub width: usize,
    pub indent: usize,
    pub blank_lines: usize,
    pub one_line: OneLine,
    pub expressions: Expressions,
    pub order: Order,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            width: 100,
            indent: 2,
            blank_lines: 1,
            one_line: OneLine::Fits,
            expressions: Expressions::Mixed,
            order: Order::Written,
        }
    }
}

/// One thing a project can say of how its blueprints are laid out.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Setting {
    pub name: &'static str,
    /// What it is when nothing is said.
    pub default: &'static str,
    /// What may be written after it.
    pub may_be: &'static str,
    pub help: &'static str,
}

/// Everything that can be set, and nothing else can. The documentation's
/// table is this list, and a test says when the two differ.
pub const SETTINGS: &[Setting] = &[
    Setting {
        name: "width",
        default: "100",
        may_be: "a whole number from 40 to 1000",
        help: "How wide a line may be before a map or a list written on it becomes a block. Comments and sentences are never broken to fit.",
    },
    Setting {
        name: "indent",
        default: "2",
        may_be: "a whole number from 1 to 8",
        help: "How many spaces each level is set in by.",
    },
    Setting {
        name: "blank_lines",
        default: "1",
        may_be: "a whole number from 0 to 3",
        help: "How many blank lines set apart a step, a handler, a param or a section at the top that is written over several lines. With 0 none is added, and one the author left stays.",
    },
    Setting {
        name: "one_line",
        default: "fits",
        may_be: "`fits`, `never` or `always`",
        help: "When a map written in braces stays on its line: while it `fits` the width, `never` (it is always written as a block), or `always`, however long. A list in brackets goes by the width unless this is `always`.",
    },
    Setting {
        name: "expressions",
        default: "mixed",
        may_be: "`mixed`, `spaced`, `tight` or `written`",
        help: "The spaces just inside `${` and `}`: `mixed` is none around a bare name and one each side of anything else, `spaced` one each side always, `tight` none, `written` as the author left them.",
    },
    Setting {
        name: "order",
        default: "written",
        may_be: "`written` or `vocabulary`",
        help: "The order of the fields of a step, a handler, a param and the top of the file: as `written`, or as the language lists them (`vocabulary`). Names of the author's own (steps, params, handlers) always stay in the order written.",
    },
];

impl Settings {
    /// Reads a project's settings file. Anything it does not say is as it
    /// is for everyone; anything it says that cannot be set is refused.
    pub fn read(text: &str) -> Result<Settings, String> {
        let mut settings = Settings::default();
        if text.lines().all(|line| line.trim().is_empty() || line.trim_start().starts_with('#')) {
            return Ok(settings);
        }
        let root = syntax::parse(text).map_err(|err| {
            format!(
                "line {}: {}; it is a few `name: value` lines, such as `width: 120`",
                err.span.start.line, err.message
            )
        })?;
        for entry in root.as_map().unwrap_or_default() {
            let Some(setting) = SETTINGS.iter().find(|setting| setting.name == entry.key) else {
                let known: Vec<&str> = SETTINGS.iter().map(|setting| setting.name).collect();
                return Err(format!(
                    "`{}` is not something the layout of a blueprint can be told; what can be set is: {}",
                    entry.key,
                    known.join(", ")
                ));
            };
            let wrong = || format!("`{}` must be {}", setting.name, setting.may_be);
            let number = |low: i64, high: i64| match entry.value.value {
                Value::Int(number) if (low..=high).contains(&number) => Ok(number as usize),
                _ => Err(wrong()),
            };
            let word = entry.value.as_str().unwrap_or_default();
            match setting.name {
                "width" => settings.width = number(40, 1000)?,
                "indent" => settings.indent = number(1, 8)?,
                "blank_lines" => settings.blank_lines = number(0, 3)?,
                "one_line" => {
                    settings.one_line = match word {
                        "fits" => OneLine::Fits,
                        "never" => OneLine::Never,
                        "always" => OneLine::Always,
                        _ => return Err(wrong()),
                    }
                }
                "expressions" => {
                    settings.expressions = match word {
                        "mixed" => Expressions::Mixed,
                        "spaced" => Expressions::Spaced,
                        "tight" => Expressions::Tight,
                        "written" => Expressions::Written,
                        _ => return Err(wrong()),
                    }
                }
                _ => {
                    settings.order = match word {
                        "written" => Order::Written,
                        "vocabulary" => Order::Vocabulary,
                        _ => return Err(wrong()),
                    }
                }
            }
        }
        Ok(settings)
    }
}

/// Why a text was left as it was.
#[derive(Clone, Debug, PartialEq)]
pub enum Unformatted {
    /// It does not read as a blueprint; what the reader said of it.
    Unreadable(SyntaxError),
    /// Laying it out would have changed what it means. That is a fault
    /// here and never the text's, and the text is the one to keep.
    Fault,
}

impl Unformatted {
    pub fn message(&self) -> String {
        match self {
            Unformatted::Unreadable(err) => {
                format!("{}:{}: {}", err.span.start.line, err.span.start.col, err.message)
            }
            Unformatted::Fault => {
                "laying this out would have changed what it means, which is a fault in the \
                                   formatter and not in the text; it was left exactly as it is"
                    .into()
            }
        }
    }
}

/// `source` laid out as `settings` say. The same text comes back when it
/// was laid out so already; laying out what comes back changes nothing.
pub fn format(source: &str, settings: &Settings) -> Result<String, Unformatted> {
    let root = syntax::parse(source).map_err(Unformatted::Unreadable)?;
    let laid = crate::layout::lay_out(source, &root, settings);
    // Held to its promise every time, and not only by the tests: a text
    // that would come to mean something else is not handed back. Where the
    // settings ask for fields in another order, what is held is that the
    // same things are said.
    let in_order = settings.order == Order::Written;
    let kept = syntax::parse(&laid).is_ok_and(|again| said(&again, in_order) == said(&root, in_order));
    if kept { Ok(laid) } else { Err(Unformatted::Fault) }
}

/// Where `source` has a value written plain that YAML does not read as
/// this reader does: each is a place an editor marks, or takes for
/// something else, and [`format`] puts each in quotes. Nothing, for a
/// text that does not read.
pub fn not_yaml(source: &str) -> Vec<syntax::Span> {
    syntax::parse(source).map(|root| crate::layout::parted(source, &root)).unwrap_or_default()
}

/// What a text means, in a form two texts can be compared by: everything
/// it says and the order it says it in, without where it says it, its
/// comments, how a value is quoted or laid out, and the spaces just
/// inside a `${ }`. `None` for a text that does not read.
pub fn meaning(source: &str) -> Option<String> {
    syntax::parse(source).ok().map(|root| said(&root, true))
}

/// Whether two texts are the same, or differ only in layout and comments.
pub fn means_the_same(one: &str, other: &str) -> bool {
    one == other || matches!((meaning(one), meaning(other)), (Some(one), Some(other)) if one == other)
}

fn said(root: &Node, in_order: bool) -> String {
    let mut out = String::new();
    put(root, &mut out, in_order);
    out
}

/// A tree written out with nothing of its layout, in the order it was
/// said. Shared with [`crate::outset`], which writes a part of one.
pub(crate) fn write(node: &Node, out: &mut String) {
    put(node, out, true);
}

fn put(node: &Node, out: &mut String, in_order: bool) {
    match &node.value {
        Value::Null => out.push_str("null"),
        Value::Bool(flag) => out.push_str(&flag.to_string()),
        Value::Int(number) => out.push_str(&number.to_string()),
        Value::Float(number) => out.push_str(&format!("{number:?}")),
        Value::Str(text) => out.push_str(&format!("{:?}", pad(text, Expressions::Tight))),
        Value::Seq(items) => {
            out.push('[');
            for item in items {
                put(item, out, in_order);
                out.push(',');
            }
            out.push(']');
        }
        Value::Map(entries) => {
            let mut entries: Vec<_> = entries.iter().collect();
            if !in_order {
                entries.sort_by(|one, other| one.key.cmp(&other.key));
            }
            out.push('{');
            for entry in entries {
                out.push_str(&format!("{:?}:", entry.key));
                put(&entry.value, out, in_order);
                out.push(',');
            }
            out.push('}');
        }
    }
}

/// `text` with the spaces just inside each `${ }` as `how` says. What is
/// between them is the expression's own and is left alone.
pub(crate) fn pad(text: &str, how: Expressions) -> String {
    if how == Expressions::Written || !template::has_holes(text) {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + 8);
    let mut at = 0;
    while at < text.len() {
        let rest = &text[at..];
        if rest.starts_with("$${") {
            out.push_str("$${");
            at += 3;
        } else if rest.starts_with("${") {
            // One that is never closed is the reader's to complain of.
            let Some(close) = template::closing_brace(text, at + 1) else {
                out.push_str(rest);
                break;
            };
            let inside = text[at + 2..close].trim();
            let spaced = match how {
                Expressions::Spaced => true,
                Expressions::Mixed => !is_name(inside),
                Expressions::Tight | Expressions::Written => false,
            };
            out.push_str("${");
            if spaced && !inside.is_empty() {
                out.push(' ');
                out.push_str(inside);
                out.push(' ');
            } else {
                out.push_str(inside);
            }
            out.push('}');
            at = close + 1;
        } else {
            let ch = rest.chars().next().expect("at is inside the text");
            out.push(ch);
            at += ch.len_utf8();
        }
    }
    out
}

/// A bare name and what follows its dots: `params.item`, `steps.build.ok`.
fn is_name(inside: &str) -> bool {
    !inside.is_empty()
        && !inside.starts_with(|first: char| first.is_ascii_digit())
        && inside
            .split('.')
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_'))
}
