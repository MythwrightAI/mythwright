//! Writing a blueprint's tree out again as text, for [`crate::format`].
//!
//! The reader gives every node the place it was read from and throws away
//! what is between them: comments, blank lines, how far things were set
//! in. What it throws away is read again here, from the lines themselves,
//! by where the nodes are: the lines between one entry and the next are
//! that next entry's comments, and what follows a value on its own line is
//! a comment on that line. A comment that sits as deep as the entries of a
//! block that has just ended is the end of that block, and stays in it.
//!
//! Nothing here decides what a blueprint may say. A scalar is written as
//! the characters it was (so a quoted string stays quoted its way), a
//! block stays a block, and only a map or list written in braces is ever
//! turned into another form, when it no longer fits its line.
//!
//! One thing is added to what was written: quotes, round a value written
//! plain that YAML does not read as this reader does ([`parts`]). A
//! blueprint is YAML to everything else that reads one, an editor first,
//! and what is laid out here is a text those read too.

use crate::format::{OneLine, Order, Settings, pad};
use crate::syntax::{Entry, Node, Pos, Span, Value};
use crate::vocabulary::{self, Kind, Shape};

/// Whether a value written plain as `text` is one YAML does not read as
/// this reader does, so that an editor marks its line or takes it for
/// something else. This reader ends a plain value at a comment or the end
/// of its line (and inside braces at `,`, `]` and `}`), and takes a
/// `${ }` whole whatever is in it; YAML ends one sooner:
///
/// * at `: `, where it takes what came before for a key, and at a `:` that
///   ends it;
/// * at ` #`, which this reader takes for a comment too, but not inside a
///   `${ }`;
/// * inside braces or brackets (`braced`), at any of `{ } [ ] ,`, which is
///   what a `${ }` written bare there has;
/// * and at its very start, where `- `, `? `, `|`, `>`, `,`, `]` and `}`
///   each begin something else.
///
/// In quotes ([`in_quotes`]) each is the same value to both.
fn parts(text: &str, braced: bool) -> bool {
    text.contains(": ")
        || text.ends_with(':')
        || text.contains(" #")
        || text == "-"
        || text.starts_with("- ")
        || text.starts_with("? ")
        || text.starts_with(['|', '>', ',', ']', '}'])
        || (braced && text.contains(['{', '}', '[', ']', ',']))
}

/// Whether a value written plain as `text` after its key, out of braces,
/// is read as that text. Inside braces a value may begin with what no
/// value of a line can (`%x`, `&x`), and a mapping too long for its line
/// is written out a line to a key: there such a value is put in quotes,
/// which is the same value. The reader is asked, so that what it takes is
/// said in one place.
fn stands(text: &str) -> bool {
    let usual = |first: char| first.is_alphanumeric() || matches!(first, '_' | '$' | '/' | '.');
    let reads = || {
        crate::syntax::parse(&format!("under: {text}\n")).is_ok_and(|read| {
            let under = read.as_map().and_then(|top| top.first());
            under.is_some_and(|entry| matches!(&entry.value.value, Value::Str(read) if read == text))
        })
    };
    text.starts_with(usual) || text.contains('\n') || reads()
}

/// `text` in quotes, so that it is the same text to this reader and to
/// YAML: in double quotes where nothing in it would be an escape there,
/// and otherwise in single ones, whose one rule is that a `'` is written
/// twice.
fn in_quotes(text: &str) -> String {
    if text.contains(['"', '\\']) { format!("'{}'", text.replace('\'', "''")) } else { format!("\"{text}\"") }
}

/// Where in `source`, whose tree is `root`, a value is written plain that
/// YAML does not read as this reader does ([`parts`]): what laying the
/// text out puts in quotes.
pub(crate) fn parted(source: &str, root: &Node) -> Vec<Span> {
    let lines: Vec<&str> = source.split('\n').map(|raw| raw.strip_suffix('\r').unwrap_or(raw)).collect();
    let layout = Layout { lines, settings: &Settings::default() };
    let mut found = Vec::new();
    layout.parted(root, false, &mut found);
    found
}

/// `source`, whose tree is `root`, laid out as `settings` say.
pub(crate) fn lay_out(source: &str, root: &Node, settings: &Settings) -> String {
    let lines: Vec<&str> = source.split('\n').map(|raw| raw.strip_suffix('\r').unwrap_or(raw)).collect();
    let total = lines.len() as u32;
    let layout = Layout { lines, settings };
    let at = Place {
        lead: Range { from: 1, to: root.span.start.line },
        after: Range { from: root.span.end.line + 1, to: total + 1 },
        indent: 0,
        owner: -1,
    };
    let mut text = layout.block_map(root, Below::Shape(&vocabulary::BLUEPRINT), at, true).join("\n");
    text.push('\n');
    text
}

/// Source lines `from..to`, counted from one.
#[derive(Clone, Copy)]
struct Range {
    from: u32,
    to: u32,
}

/// Where a block is written, and what of the source around it is its.
#[derive(Clone, Copy)]
struct Place {
    /// The lines before its first entry.
    lead: Range,
    /// The lines after its last, as far as whatever comes next.
    after: Range,
    indent: usize,
    /// How far the key or dash that owns it was set in, in the source;
    /// `-1` for the file itself.
    owner: i64,
}

/// What a value is written after, and where what hangs under it goes.
struct Seat {
    /// `  key:` or `  -`.
    head: String,
    dash: bool,
    /// How far a block under it is set in, and the body of a `|` block.
    nest: usize,
    body: usize,
    /// The source line the head is on.
    line: u32,
    after: Range,
    /// How far the head was set in, in the source.
    own: usize,
    /// A comment that followed the head when nothing else was on its line.
    note: Option<String>,
}

/// What the language knows of a mapping, which decides only two things:
/// whether its entries are things with names of the author's own, set
/// apart by blank lines, and what order its fields are listed in.
#[derive(Clone, Copy)]
enum Below {
    Shape(&'static Shape),
    /// A mapping from the author's names to the shape named.
    Named(&'static str),
    /// A list of the shape named, or one of them.
    List(&'static str),
    /// The author's own: nothing is known of it.
    Own,
}

#[derive(Clone, Copy, PartialEq)]
enum Form {
    /// Nothing was written.
    Empty,
    Scalar,
    BlockScalar,
    FlowMap,
    FlowSeq,
    BlockMap,
    BlockSeq,
}

#[derive(Clone, PartialEq)]
enum Trivia {
    Blank,
    Comment(String),
}

/// One entry of a block as it will be written: what comes before it, and
/// its own lines.
struct Part {
    lead: Vec<Trivia>,
    body: Vec<String>,
    rank: usize,
}

/// The comments inside a map or list written in braces, by where they sat.
struct Notes {
    lead: Vec<Vec<String>>,
    trail: Vec<Option<String>>,
    last: Vec<String>,
}

struct Layout<'a> {
    lines: Vec<&'a str>,
    settings: &'a Settings,
}

fn shape(name: &str) -> Below {
    vocabulary::SHAPES.iter().find(|shape| shape.name == name).map_or(Below::Own, Below::Shape)
}

fn below(kind: Kind) -> Below {
    match kind {
        Kind::Shape(name) => shape(name),
        Kind::MapOf(name) => Below::Named(name),
        Kind::ListOf(name) => Below::List(name),
        _ => Below::Own,
    }
}

/// What is known of the value of `key` in a mapping `under` is known as.
fn field_below(under: Below, key: &str) -> Below {
    match under {
        Below::Shape(shape) => {
            shape.fields.iter().find(|field| field.name == key).map_or(Below::Own, |field| below(field.value))
        }
        Below::Named(name) => shape(name),
        Below::List(_) | Below::Own => Below::Own,
    }
}

/// A mapping written where a list of them may be is one of them.
fn one_of(under: Below) -> Below {
    match under {
        Below::List(name) => shape(name),
        other => other,
    }
}

fn each_of(under: Below) -> Below {
    match under {
        Below::List(name) => shape(name),
        _ => Below::Own,
    }
}

fn with(line: String, note: Option<String>) -> String {
    match note {
        Some(note) => format!("{line}  {note}"),
        None => line,
    }
}

fn is_block_header(rest: &str) -> bool {
    let head = rest.split('#').next().unwrap_or("").trim_end();
    (head.starts_with('|') || head.starts_with('>')) && head.len() <= 2
}

fn is_empty(node: &Node) -> bool {
    match &node.value {
        Value::Map(entries) => entries.is_empty(),
        Value::Seq(items) => items.is_empty(),
        _ => false,
    }
}

/// A key written in braces with nothing after its colon.
fn keyed_null(entry: &Entry) -> bool {
    entry.value.is_null() && entry.value.span == entry.key_span
}

fn note_in(segment: &str) -> Option<String> {
    segment.find('#').map(|at| segment[at..].trim_end().to_string())
}

/// Makes the blank lines before an entry that is set apart `apart` of
/// them: where the author left one, there; where none, before its comments.
fn set_apart(lead: &mut Vec<Trivia>, apart: usize) {
    if apart == 0 {
        return;
    }
    match lead.iter().position(|one| *one == Trivia::Blank) {
        Some(at) => (1..apart).for_each(|_| lead.insert(at, Trivia::Blank)),
        None => (0..apart).for_each(|_| lead.insert(0, Trivia::Blank)),
    }
}

/// A block's lines under what owns it. After a dash the block begins on
/// the dash's own line, and what was said before its first entry is said
/// before the dash.
fn under(head: String, note: Option<String>, dash: bool, mut lines: Vec<String>) -> Vec<String> {
    if !dash {
        let mut out = vec![with(head, note)];
        out.extend(lines);
        return out;
    }
    let indent = head.len() - 1;
    let pad = &head[..indent];
    let mut out: Vec<String> = note.into_iter().map(|note| format!("{pad}{note}")).collect();
    let content = |line: &String| !line.trim().is_empty() && !line.trim_start().starts_with('#');
    let first = lines.iter().position(content).unwrap_or(0);
    for line in lines.drain(..first) {
        out.push(if line.trim().is_empty() { String::new() } else { format!("{pad}{}", line.trim_start()) });
    }
    if let Some(start) = lines.first_mut() {
        *start = format!("{head} {}", &start[indent + 2..]);
    }
    out.extend(lines);
    out
}

impl Layout<'_> {
    fn line(&self, no: u32) -> &str {
        self.lines.get((no as usize).wrapping_sub(1)).map_or("", |line| line.trim_end())
    }

    fn indent(&self, no: u32) -> usize {
        self.line(no).bytes().take_while(|byte| *byte == b' ').count()
    }

    /// Where in its line a place is. A place counts characters.
    fn byte(&self, pos: Pos) -> usize {
        let line = self.line(pos.line);
        line.char_indices().nth((pos.col as usize).saturating_sub(1)).map_or(line.len(), |(at, _)| at)
    }

    fn rest(&self, pos: Pos) -> &str {
        &self.line(pos.line)[self.byte(pos)..]
    }

    fn slice(&self, span: Span) -> String {
        self.between(span.start, span.end).join("\n")
    }

    /// The text from one place to another, a piece for each line.
    fn between(&self, from: Pos, to: Pos) -> Vec<&str> {
        (from.line..=to.line)
            .map(|no| {
                let line = self.line(no);
                let start = if no == from.line { self.byte(from) } else { 0 };
                let end = if no == to.line { self.byte(to) } else { line.len() };
                line.get(start..end).unwrap_or("")
            })
            .collect()
    }

    fn comment_at(&self, pos: Pos) -> Option<String> {
        let rest = self.rest(pos).trim_start();
        rest.starts_with('#').then(|| rest.to_string())
    }

    fn comment_after_key(&self, entry: &Entry) -> Option<String> {
        let rest = self.rest(entry.key_span.end).trim_start().strip_prefix(':')?.trim_start();
        rest.starts_with('#').then(|| rest.to_string())
    }

    /// The comments and blank lines in a range, with a run of blank lines
    /// as one.
    fn trivia(&self, range: Range) -> Vec<Trivia> {
        let mut out = Vec::new();
        for no in range.from..range.to {
            let text = self.line(no).trim_start();
            if text.starts_with('#') {
                out.push(Trivia::Comment(text.to_string()));
            } else if text.is_empty() && out.last() != Some(&Trivia::Blank) {
                out.push(Trivia::Blank);
            }
        }
        out
    }

    /// How many lines at the front of `range` end a block whose entries
    /// were set in by `floor`: as far as the last comment set in that deep.
    fn tail(&self, range: Range, floor: usize) -> u32 {
        let mut claimed = 0;
        for no in range.from..range.to {
            let text = self.line(no).trim_start();
            if text.is_empty() {
                continue;
            }
            if !text.starts_with('#') || self.indent(no) < floor {
                break;
            }
            claimed = no + 1 - range.from;
        }
        claimed
    }

    /// How many lines at the front of `after` belong inside `node`: to the
    /// block that is its last entry's value, and then to itself.
    fn claims(&self, node: &Node, after: Range, owner: i64) -> u32 {
        let (last, own) = match (self.form(node), &node.value) {
            (Form::BlockMap, Value::Map(entries)) => {
                (entries.last().map(|entry| &entry.value), self.set_in(node))
            }
            (Form::BlockSeq, Value::Seq(items)) => (items.last(), self.set_in(node)),
            _ => return 0,
        };
        let inner = last.map_or(0, |last| self.claims(last, after, own as i64));
        let floor = own.max((owner + 1).max(0) as usize);
        inner + self.tail(Range { from: after.from + inner, to: after.to }, floor)
    }

    /// How far a block's entries were set in, in the source.
    fn set_in(&self, node: &Node) -> usize {
        (node.span.start.col as usize).saturating_sub(1)
    }

    fn form(&self, node: &Node) -> Form {
        let first = self.rest(node.span.start).chars().next();
        match &node.value {
            Value::Map(entries) => {
                let braces = entries
                    .first()
                    .is_none_or(|entry| first == Some('{') && entry.key_span.start != node.span.start);
                if braces { Form::FlowMap } else { Form::BlockMap }
            }
            Value::Seq(_) if first == Some('[') => Form::FlowSeq,
            Value::Seq(_) => Form::BlockSeq,
            Value::Null if matches!(self.slice(node.span).as_str(), "null" | "~") => Form::Scalar,
            Value::Null => Form::Empty,
            Value::Str(_) if is_block_header(self.rest(node.span.start)) => Form::BlockScalar,
            _ => Form::Scalar,
        }
    }

    /// A key as it was written, where the reader takes that for the same
    /// key at the start of a line. One written inside `{ }` may be what no
    /// line can begin with (`---x`, `&x`, `? x`), and a mapping too long
    /// for its line is written out a line to a key: there it is quoted,
    /// which is the same key. So is one that no first key of a list's
    /// item can begin with (`[x`). The reader is asked, so that what it
    /// takes is said in one place.
    fn key(&self, entry: &Entry) -> String {
        let raw = self.slice(entry.key_span);
        let usual = |first: char| first.is_alphanumeric() || matches!(first, '_' | '"' | '\'');
        // Asked under a key of its own, as a key of a mapping is written:
        // the first line of a text is held to more (`...` hands on
        // another mapping's entries, and is a key anywhere but there).
        // And as the first key of a mapping that is one of a list, which
        // is written after the list's dash: there `[x` and `{x` begin a
        // list and a mapping of their own.
        let reads = |written: &str| {
            let alone = |entries: &[Entry]| entries.len() == 1 && entries[0].key == entry.key;
            let under = crate::syntax::parse(&format!("under:\n  {written}: 0\n")).is_ok_and(|read| {
                let under = read.as_map().and_then(|top| top.first()).and_then(|top| top.value.as_map());
                under.is_some_and(alone)
            });
            let listed = crate::syntax::parse(&format!("under:\n  - {written}: 0\n")).is_ok_and(|read| {
                let under = read.as_map().and_then(|top| top.first());
                let items = under.and_then(|top| match &top.value.value {
                    Value::Seq(items) => Some(items),
                    _ => None,
                });
                items.is_some_and(|items| items.len() == 1 && items[0].as_map().is_some_and(alone))
            });
            under && listed
        };
        if raw.starts_with(usual) || reads(&raw) {
            return raw;
        }
        let quoted = match entry.key.contains(['"', '\\']) {
            true => format!("'{}'", entry.key.replace('\'', "''")),
            false => format!("\"{}\"", entry.key),
        };
        if reads(&quoted) { quoted } else { raw }
    }

    /// A scalar as it was written, with the spaces inside its `${ }` as
    /// the settings say. One whose quoting hides where an expression ends
    /// (an escape in it) is left exactly as it was. One written plain that
    /// YAML does not read as it is here ([`parts`]), inside braces or
    /// brackets (`braced`) or out of them, is put in quotes; and so is one
    /// that was written inside braces and is no value where a line is its
    /// key and its value ([`stands`]).
    fn scalar(&self, node: &Node, braced: bool) -> String {
        let raw = self.slice(node.span);
        let how = self.settings.expressions;
        let inside = || &raw[1..raw.len() - 1];
        match raw.chars().next() {
            Some('"') if raw.len() >= 2 && !raw.contains('\\') => format!("\"{}\"", pad(inside(), how)),
            Some('\'') if raw.len() >= 2 && !inside().contains('\'') => format!("'{}'", pad(inside(), how)),
            Some('"' | '\'') => raw,
            _ => {
                let written = pad(&raw, how);
                let apart = parts(&written, braced) || !(braced || stands(&written));
                if self.is_plain_text(node) && apart { in_quotes(&written) } else { written }
            }
        }
    }

    /// Whether a value is text written plain: not a number, a flag or
    /// nothing, which quotes would make text of, and not in quotes already.
    fn is_plain_text(&self, node: &Node) -> bool {
        matches!(&node.value, Value::Str(text) if *text == self.slice(node.span))
    }

    /// The places under `node` where a value written plain is one YAML
    /// does not read as it is here.
    fn parted(&self, node: &Node, braced: bool, found: &mut Vec<Span>) {
        let form = self.form(node);
        let braced = braced || matches!(form, Form::FlowMap | Form::FlowSeq);
        match &node.value {
            Value::Map(entries) => {
                entries.iter().filter(|entry| !keyed_null(entry)).for_each(|entry| {
                    self.parted(&entry.value, braced, found);
                });
            }
            Value::Seq(items) => items.iter().for_each(|item| self.parted(item, braced, found)),
            _ if form == Form::Scalar
                && self.is_plain_text(node)
                && parts(&self.slice(node.span), braced) =>
            {
                found.push(node.span);
            }
            _ => {}
        }
    }

    fn rank(&self, under: Below, key: &str, index: usize) -> usize {
        match under {
            Below::Shape(shape) if self.settings.order == Order::Vocabulary => {
                shape.fields.iter().position(|field| field.name == key).unwrap_or(usize::MAX)
            }
            _ => index,
        }
    }

    fn block_map(&self, node: &Node, under: Below, at: Place, top: bool) -> Vec<String> {
        let entries = node.as_map().unwrap_or_default();
        let under = one_of(under);
        let own = self.set_in(node);
        let pad = " ".repeat(at.indent);
        let mut parts = Vec::with_capacity(entries.len());
        let mut lead = at.lead;
        for (index, entry) in entries.iter().enumerate() {
            let line = entry.key_span.start.line;
            let to = entries.get(index + 1).map_or(at.after.to, |next| next.key_span.start.line);
            let after = Range { from: entry.value.span.end.line.max(line) + 1, to };
            let seat = Seat {
                head: format!("{pad}{}:", self.key(entry)),
                dash: false,
                nest: at.indent + self.settings.indent,
                body: at.indent + self.settings.indent,
                line,
                after,
                own,
                note: self.comment_after_key(entry),
            };
            let body = self.value(&entry.value, field_below(under, &entry.key), seat);
            parts.push(Part { lead: self.trivia(lead), body, rank: self.rank(under, &entry.key, index) });
            let claimed = self.claims(&entry.value, after, own as i64);
            lead = Range { from: after.from + claimed, to: after.to };
        }
        let mine = self.tail(lead, own.max((at.owner + 1).max(0) as usize));
        let tail = self.trivia(Range { from: lead.from, to: lead.from + mine });
        self.assemble(parts, tail, at.indent, top || matches!(under, Below::Named(_)))
    }

    fn block_seq(&self, node: &Node, under: Below, at: Place) -> Vec<String> {
        let items = node.as_seq().unwrap_or_default();
        let own = self.set_in(node);
        let pad = " ".repeat(at.indent);
        let dashes: Vec<u32> = items.iter().map(|item| self.dash_line(item, own)).collect();
        let mut parts = Vec::with_capacity(items.len());
        let mut lead = at.lead;
        for (index, item) in items.iter().enumerate() {
            let line = dashes[index];
            let to = dashes.get(index + 1).copied().unwrap_or(at.after.to);
            let after = Range { from: item.span.end.line.max(line) + 1, to };
            let rest = self.line(line).get(own + 1..).unwrap_or("").trim_start();
            let seat = Seat {
                head: format!("{pad}-"),
                dash: true,
                nest: at.indent + 2,
                body: at.indent + self.settings.indent,
                line,
                after,
                own,
                note: rest.starts_with('#').then(|| rest.to_string()),
            };
            let body = self.value(item, each_of(under), seat);
            parts.push(Part { lead: self.trivia(lead), body, rank: index });
            let claimed = self.claims(item, after, own as i64);
            lead = Range { from: after.from + claimed, to: after.to };
        }
        let mine = self.tail(lead, own.max((at.owner + 1).max(0) as usize));
        let tail = self.trivia(Range { from: lead.from, to: lead.from + mine });
        self.assemble(parts, tail, at.indent, false)
    }

    /// The line an item's dash is on: its own first line, or, for a block
    /// that begins under its dash, the nearest line above with the dash.
    fn dash_line(&self, item: &Node, own: usize) -> u32 {
        let mut no = item.span.start.line;
        while no > 1 && !(self.indent(no) == own && self.line(no)[own..].starts_with('-')) {
            no -= 1;
        }
        no
    }

    /// The entries of a block one after another, each with what came
    /// before it. In a block of named things (steps, handlers, params, the
    /// sections at the top) one written over several lines is set apart
    /// from its neighbours by blank lines.
    fn assemble(&self, mut parts: Vec<Part>, tail: Vec<Trivia>, indent: usize, named: bool) -> Vec<String> {
        parts.sort_by_key(|part| part.rank);
        let pad = " ".repeat(indent);
        let said = |one: Trivia| match one {
            Trivia::Blank => String::new(),
            Trivia::Comment(text) => format!("{pad}{text}"),
        };
        let mut out = Vec::new();
        let mut before: Option<usize> = None;
        for part in parts {
            let mut lead = part.lead;
            match before {
                None => {
                    let comment = lead.iter().position(|one| *one != Trivia::Blank).unwrap_or(lead.len());
                    lead.drain(..comment);
                }
                Some(lines) if named && (lines > 1 || part.body.len() > 1) => {
                    set_apart(&mut lead, self.settings.blank_lines);
                }
                Some(_) => {}
            }
            out.extend(lead.into_iter().map(said));
            before = Some(part.body.len());
            out.extend(part.body);
        }
        out.extend(tail.into_iter().map(said));
        out
    }

    /// A value after its key or dash.
    fn value(&self, node: &Node, below: Below, seat: Seat) -> Vec<String> {
        let lead = Range { from: seat.line + 1, to: node.span.start.line };
        let at = Place { lead, after: seat.after, indent: seat.nest, owner: seat.own as i64 };
        match self.form(node) {
            Form::Empty => vec![with(seat.head, seat.note)],
            Form::Scalar => {
                let written = self.scalar(node, false);
                vec![with(format!("{} {written}", seat.head), self.comment_at(node.span.end))]
            }
            Form::BlockScalar => self.block_scalar(node, &seat),
            Form::FlowMap | Form::FlowSeq => {
                self.braces(node, below, seat.head, seat.nest, seat.dash, self.comment_at(node.span.end))
            }
            Form::BlockMap => under(seat.head, seat.note, seat.dash, self.block_map(node, below, at, false)),
            Form::BlockSeq => under(seat.head, seat.note, seat.dash, self.block_seq(node, below, at)),
        }
    }

    /// A `|` or `>` block: its header as written, and its lines set in
    /// under what owns it, each as far past the first as it was.
    fn block_scalar(&self, node: &Node, seat: &Seat) -> Vec<String> {
        let rest = self.rest(node.span.start);
        let cut = rest.find('#');
        let header = cut.map_or(rest, |at| &rest[..at]).trim_end();
        let mut out = vec![with(format!("{} {header}", seat.head), cut.map(|at| rest[at..].to_string()))];
        let (first, last) = (node.span.start.line + 1, node.span.end.line);
        let base = (first..=last).find(|no| !self.line(*no).is_empty()).map_or(0, |no| self.indent(no));
        for no in first..=last {
            let text = self.line(no);
            let set_in = self.indent(no);
            out.push(if text.is_empty() {
                String::new()
            } else {
                format!("{}{}", " ".repeat(seat.body + set_in.saturating_sub(base)), &text[set_in..])
            });
        }
        out
    }

    /// A map or list written in braces: on its line while it may stay
    /// there, and as a block otherwise.
    fn braces(
        &self,
        node: &Node,
        below: Below,
        head: String,
        nest: usize,
        dash: bool,
        note: Option<String>,
    ) -> Vec<String> {
        if let Some(text) = self.one_line(node, head.chars().count() + 1) {
            return vec![with(format!("{head} {text}"), note)];
        }
        if is_empty(node) {
            // Nothing in it but comments: it stays, and they follow it, set
            // in as far as it is, which is where they are found again.
            let pad = " ".repeat(head.len() - head.trim_start().len());
            let mut out = vec![with(format!("{head} {}", self.flow(node)), note)];
            out.extend(self.notes(node).last.into_iter().map(|said| format!("{pad}{said}")));
            return out;
        }
        under(head, note, dash, self.opened(node, below, nest))
    }

    /// The one line a map or list in braces is, when it may be one: it
    /// has no comment inside, and it fits, or the settings keep it there.
    fn one_line(&self, node: &Node, used: usize) -> Option<String> {
        if self.has_comments(node) {
            return None;
        }
        let text = self.flow(node);
        let map = matches!(&node.value, Value::Map(entries) if !entries.is_empty());
        match self.settings.one_line {
            _ if is_empty(node) => Some(text),
            OneLine::Always => Some(text),
            OneLine::Never if map => None,
            _ => (used + text.chars().count() <= self.settings.width).then_some(text),
        }
    }

    fn flow(&self, node: &Node) -> String {
        match &node.value {
            Value::Map(entries) if entries.is_empty() => "{}".into(),
            Value::Map(entries) => {
                let inside: Vec<String> = entries
                    .iter()
                    .map(|entry| {
                        let key = self.slice(entry.key_span);
                        if keyed_null(entry) {
                            format!("{key}:")
                        } else {
                            format!("{key}: {}", self.flow(&entry.value))
                        }
                    })
                    .collect();
                format!("{{ {} }}", inside.join(", "))
            }
            Value::Seq(items) if items.is_empty() => "[]".into(),
            Value::Seq(items) => {
                format!("[{}]", items.iter().map(|item| self.flow(item)).collect::<Vec<_>>().join(", "))
            }
            _ => self.scalar(node, true),
        }
    }

    fn has_comments(&self, node: &Node) -> bool {
        let notes = self.notes(node);
        let here = notes.lead.iter().any(|lead| !lead.is_empty())
            || notes.trail.iter().any(Option::is_some)
            || !notes.last.is_empty();
        here || match &node.value {
            Value::Map(entries) => entries.iter().any(|entry| self.has_comments(&entry.value)),
            Value::Seq(items) => items.iter().any(|item| self.has_comments(item)),
            _ => false,
        }
    }

    /// The comments between the things in a map or list written in
    /// braces. One on the line a thing ends on is that thing's; one on a
    /// line of its own is said before what follows it.
    fn notes(&self, node: &Node) -> Notes {
        let places: Vec<(Pos, Pos, Option<Pos>)> = match &node.value {
            Value::Map(entries) => entries
                .iter()
                .map(|entry| {
                    if keyed_null(entry) {
                        (entry.key_span.start, entry.key_span.end, None)
                    } else {
                        (entry.key_span.start, entry.value.span.end, Some(entry.value.span.start))
                    }
                })
                .collect(),
            Value::Seq(items) => items.iter().map(|item| (item.span.start, item.span.end, None)).collect(),
            _ => Vec::new(),
        };
        let count = places.len();
        let mut notes = Notes { lead: vec![Vec::new(); count], trail: vec![None; count], last: Vec::new() };
        if !matches!(node.value, Value::Map(_) | Value::Seq(_)) {
            return notes;
        }
        let close = Pos { line: node.span.end.line, col: node.span.end.col.saturating_sub(1) };
        let mut from = Pos { line: node.span.start.line, col: node.span.start.col + 1 };
        for (index, (start, end, value)) in places.iter().enumerate() {
            for (line, segment) in self.between(from, *start).into_iter().enumerate() {
                match note_in(segment) {
                    Some(found) if line == 0 && index > 0 => notes.trail[index - 1] = Some(found),
                    Some(found) => notes.lead[index].push(found),
                    None => {}
                }
            }
            if let Some(value) = value {
                // Between a key and its value: said before the entry.
                let key = match &node.value {
                    Value::Map(entries) => entries[index].key_span.end,
                    _ => *start,
                };
                notes.lead[index].extend(self.between(key, *value).into_iter().filter_map(note_in));
            }
            from = *end;
        }
        for (line, segment) in self.between(from, close).into_iter().enumerate() {
            match note_in(segment) {
                Some(found) if line == 0 && count > 0 => notes.trail[count - 1] = Some(found),
                Some(found) => notes.last.push(found),
                None => {}
            }
        }
        notes
    }

    /// A map or list that was written in braces, as the block it means.
    fn opened(&self, node: &Node, below: Below, indent: usize) -> Vec<String> {
        let notes = self.notes(node);
        let pad = " ".repeat(indent);
        let comments = |said: &[String]| said.iter().cloned().map(Trivia::Comment).collect::<Vec<_>>();
        let mut parts = Vec::new();
        let named = match &node.value {
            Value::Map(entries) => {
                let under = one_of(below);
                for (index, entry) in entries.iter().enumerate() {
                    let head = format!("{pad}{}:", self.key(entry));
                    let note = notes.trail[index].clone();
                    let body = if keyed_null(entry) {
                        vec![with(head, note)]
                    } else {
                        let nest = indent + self.settings.indent;
                        self.put(&entry.value, field_below(under, &entry.key), head, nest, false, note)
                    };
                    let rank = self.rank(under, &entry.key, index);
                    parts.push(Part { lead: comments(&notes.lead[index]), body, rank });
                }
                matches!(under, Below::Named(_))
            }
            Value::Seq(items) => {
                for (index, item) in items.iter().enumerate() {
                    let note = notes.trail[index].clone();
                    let body = self.put(item, each_of(below), format!("{pad}-"), indent + 2, true, note);
                    parts.push(Part { lead: comments(&notes.lead[index]), body, rank: index });
                }
                false
            }
            _ => false,
        };
        self.assemble(parts, comments(&notes.last), indent, named)
    }

    /// A value that was inside braces, after its key or dash in the block
    /// the braces became.
    fn put(
        &self,
        node: &Node,
        below: Below,
        head: String,
        nest: usize,
        dash: bool,
        note: Option<String>,
    ) -> Vec<String> {
        match &node.value {
            Value::Map(_) | Value::Seq(_) => self.braces(node, below, head, nest, dash, note),
            _ => vec![with(format!("{head} {}", self.scalar(node, false)), note)],
        }
    }
}
