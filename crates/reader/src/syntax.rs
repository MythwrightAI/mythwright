//! A strict subset of YAML, parsed with source positions.
//!
//! Blueprint files look like YAML and any YAML-aware editor will colour them,
//! but the grammar accepted here is deliberately smaller:
//!
//! * block mappings and sequences, flow `[..]` and `{..}`, plain and quoted
//!   scalars, `|` and `>` block scalars, `#` comments;
//! * no anchors, aliases, tags, merge keys, multiple documents or tabs;
//! * only `true`, `false`, `null` and `~` are special words, so `no`, `on`
//!   and `yes` stay strings;
//! * a `${ ... }` expression is one token: it may contain `:`, `#`, `,` and
//!   braces without quoting.
//!
//! Every node carries the span it was read from, which is what lets the editor
//! underline a mistake and the node view jump to a step's source.

use serde::Serialize;

/// A 1-based line and column.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Pos {
    pub line: u32,
    pub col: u32,
}

/// A half-open range in the source text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Span {
    pub start: Pos,
    pub end: Pos,
}

impl Span {
    pub fn to(self, other: Span) -> Span {
        Span { start: self.start, end: other.end }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub value: Value,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Seq(Vec<Node>),
    Map(Vec<Entry>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub key: String,
    pub key_span: Span,
    pub value: Node,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SyntaxError {
    pub message: String,
    pub span: Span,
}

impl Node {
    pub fn as_str(&self) -> Option<&str> {
        match &self.value {
            Value::Str(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&[Entry]> {
        match &self.value {
            Value::Map(entries) => Some(entries),
            _ => None,
        }
    }

    pub fn as_seq(&self) -> Option<&[Node]> {
        match &self.value {
            Value::Seq(items) => Some(items),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self.value, Value::Null)
    }

    /// What kind of thing this is, for error messages.
    pub fn kind(&self) -> &'static str {
        match self.value {
            Value::Null => "nothing",
            Value::Bool(_) => "a boolean",
            Value::Int(_) | Value::Float(_) => "a number",
            Value::Str(_) => "a string",
            Value::Seq(_) => "a list",
            Value::Map(_) => "a mapping",
        }
    }

    /// The same data without positions.
    pub fn to_json(&self) -> serde_json::Value {
        match &self.value {
            Value::Null => serde_json::Value::Null,
            Value::Bool(flag) => (*flag).into(),
            Value::Int(number) => (*number).into(),
            Value::Float(number) => {
                serde_json::Number::from_f64(*number).map_or(serde_json::Value::Null, Into::into)
            }
            Value::Str(text) => text.clone().into(),
            Value::Seq(items) => items.iter().map(Node::to_json).collect(),
            Value::Map(entries) => serde_json::Value::Object(
                entries.iter().map(|entry| (entry.key.clone(), entry.value.to_json())).collect(),
            ),
        }
    }
}

/// Parses a whole document, which must be a mapping.
pub fn parse(source: &str) -> Result<Node, SyntaxError> {
    let mut parser = Parser::new(source)?;
    parser.skip_blank();
    let Some(first) = parser.line() else {
        return Err(SyntaxError { message: "the file is empty".into(), span: Span::default() });
    };
    if first.text.starts_with("---") || first.text.starts_with("...") {
        return Err(parser.error_line("multiple documents (`---`) are not supported"));
    }
    if first.indent != 0 {
        return Err(parser.error_line("the first line must not be indented"));
    }
    if is_seq_item(first.text) {
        return Err(parser.error_line("a blueprint is a mapping, not a list"));
    }
    let node = parser.block_map(0)?;
    parser.skip_blank();
    if parser.line().is_some() {
        return Err(parser.error_line("unexpected content; check the indentation of this line"));
    }
    Ok(node)
}

#[derive(Clone, Copy)]
struct Line<'a> {
    no: u32,
    /// Columns before `text`. For a line rewritten after a `- `, this is the
    /// column the remaining content really starts at.
    indent: usize,
    text: &'a str,
}

impl Line<'_> {
    fn is_blank(&self) -> bool {
        self.text.is_empty() || self.text.starts_with('#')
    }

    fn pos(&self, offset: usize) -> Pos {
        // An offset found by bytes may fall inside a letter that takes
        // several (a mark of byte order written in the middle of a line):
        // the column is the letter's.
        let mut end = offset.min(self.text.len());
        while !self.text.is_char_boundary(end) {
            end -= 1;
        }
        let chars = self.text[..end].chars().count();
        Pos { line: self.no, col: (self.indent + chars + 1) as u32 }
    }

    fn span(&self, from: usize, to: usize) -> Span {
        Span { start: self.pos(from), end: self.pos(to) }
    }
}

/// A position inside a flow collection, which may run across lines.
#[derive(Clone, Copy)]
struct Cursor {
    line: usize,
    offset: usize,
}

struct Parser<'a> {
    lines: Vec<Line<'a>>,
    index: usize,
}

type Parsed<T> = Result<T, SyntaxError>;

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Parsed<Self> {
        let mut lines = Vec::new();
        for (index, raw) in source.split('\n').enumerate() {
            let raw = raw.strip_suffix('\r').unwrap_or(raw);
            let indent = raw.bytes().take_while(|byte| *byte == b' ').count();
            let text = raw[indent..].trim_end();
            let no = index as u32 + 1;
            if text.starts_with('\t') {
                let at = Pos { line: no, col: indent as u32 + 1 };
                return Err(SyntaxError {
                    message: "tabs cannot be used for indentation; use spaces".into(),
                    span: Span { start: at, end: at },
                });
            }
            lines.push(Line { no, indent, text });
        }
        Ok(Self { lines, index: 0 })
    }

    fn line(&self) -> Option<Line<'a>> {
        self.lines.get(self.index).copied()
    }

    fn skip_blank(&mut self) {
        while self.line().is_some_and(|line| line.is_blank()) {
            self.index += 1;
        }
    }

    fn error_line(&self, message: &str) -> SyntaxError {
        let span = self.line().map(|line| line.span(0, line.text.len())).unwrap_or_default();
        SyntaxError { message: message.into(), span }
    }

    /// A mapping or sequence whose first line is the current one.
    fn block(&mut self, indent: usize) -> Parsed<Node> {
        match self.line() {
            Some(line) if is_seq_item(line.text) => self.block_seq(indent),
            _ => self.block_map(indent),
        }
    }

    fn block_map(&mut self, indent: usize) -> Parsed<Node> {
        let mut entries: Vec<Entry> = Vec::new();
        let start = self.line().map(|line| line.pos(0)).unwrap_or_default();
        let mut end = start;
        loop {
            self.skip_blank();
            let Some(line) = self.line() else { break };
            if line.indent < indent || (line.indent == indent && is_seq_item(line.text)) {
                break;
            }
            if line.indent > indent {
                return Err(self.error_line(
                    "unexpected indentation: this line is further in than the line above it, which has its value \
                     already. Line the two up; or, if this belongs under that one, write nothing after its colon",
                ));
            }
            reject_unsupported(&line)?;
            let Some(key) = split_key(&line)? else {
                return Err(self.error_line(
                    "expected `key: value`: this line has no colon after a key (a colon is followed by a space, \
                     or ends the line)",
                ));
            };
            if entries.iter().any(|entry| entry.key == key.text) {
                return Err(SyntaxError { message: format!("duplicate key `{}`", key.text), span: key.span });
            }
            let value = self.value_after(indent, key.after)?;
            end = value.span.end;
            entries.push(Entry { key: key.text, key_span: key.span, value });
        }
        Ok(Node { value: Value::Map(entries), span: Span { start, end } })
    }

    fn block_seq(&mut self, indent: usize) -> Parsed<Node> {
        let mut items = Vec::new();
        let start = self.line().map(|line| line.pos(0)).unwrap_or_default();
        let mut end = start;
        loop {
            self.skip_blank();
            let Some(line) = self.line() else { break };
            if line.indent > indent {
                return Err(self.error_line(
                    "unexpected indentation: this line is further in than the `- ` items it is among; line it up \
                     with them",
                ));
            }
            if line.indent < indent || !is_seq_item(line.text) {
                break;
            }
            let after_dash = &line.text[1..];
            let content = after_dash.trim_start();
            let offset = 1 + after_dash.len() - content.len();
            let item = if content.is_empty() || content.starts_with('#') {
                self.index += 1;
                self.nested_or_null(indent, line.span(0, 1), false)?
            } else if is_block_scalar_header(content) {
                self.block_scalar(indent, offset)?
            } else if is_seq_item(content) || starts_map_entry(&line, offset)? {
                // Treat the rest of the line as the first line of a nested
                // block that starts at its real column.
                let column = line.indent + line.text[..offset].chars().count();
                self.lines[self.index] = Line { no: line.no, indent: column, text: &line.text[offset..] };
                self.block(column)?
            } else {
                self.inline(offset)?
            };
            end = item.span.end;
            items.push(item);
        }
        Ok(Node { value: Value::Seq(items), span: Span { start, end } })
    }

    /// The value of a mapping entry, starting just after its colon.
    fn value_after(&mut self, indent: usize, after: usize) -> Parsed<Node> {
        let line = self.line().expect("caller is on a line");
        let rest = &line.text[after..];
        let content = rest.trim_start();
        let offset = after + rest.len() - content.len();
        if content.is_empty() || content.starts_with('#') {
            self.index += 1;
            return self.nested_or_null(indent, line.span(after, after), true);
        }
        if is_block_scalar_header(content) {
            return self.block_scalar(indent, offset);
        }
        self.inline(offset)
    }

    /// What follows a key or dash that had nothing after it on its own line.
    fn nested_or_null(&mut self, indent: usize, at: Span, allow_compact_seq: bool) -> Parsed<Node> {
        self.skip_blank();
        match self.line() {
            Some(next) if next.indent > indent => self.block(next.indent),
            // A list may sit at the same indentation as the key that owns it.
            Some(next) if allow_compact_seq && next.indent == indent && is_seq_item(next.text) => {
                self.block_seq(indent)
            }
            _ => Ok(Node { value: Value::Null, span: at }),
        }
    }

    /// A value that begins on the current line at `offset`.
    fn inline(&mut self, offset: usize) -> Parsed<Node> {
        let line = self.line().expect("caller is on a line");
        let rest = &line.text[offset..];
        match rest.as_bytes()[0] {
            b'[' | b'{' => {
                let mut cursor = Cursor { line: self.index, offset };
                let node = self.flow_value(&mut cursor)?;
                self.expect_line_end(cursor)?;
                self.index = cursor.line + 1;
                Ok(node)
            }
            b'"' | b'\'' => {
                let (text, end) = quoted(&line, offset)?;
                self.expect_line_end(Cursor { line: self.index, offset: end })?;
                self.index += 1;
                Ok(Node { value: Value::Str(text), span: line.span(offset, end) })
            }
            _ => {
                reject_indicator(&line, offset)?;
                let end = offset + plain_end(&line, offset, PlainStop::Block)?;
                let raw = line.text[offset..end].trim_end();
                self.index += 1;
                Ok(Node { value: plain_value(raw), span: line.span(offset, offset + raw.len()) })
            }
        }
    }

    fn expect_line_end(&self, cursor: Cursor) -> Parsed<()> {
        let line = self.lines[cursor.line];
        let rest = line.text[cursor.offset..].trim_start();
        if rest.is_empty() || rest.starts_with('#') {
            return Ok(());
        }
        let from = line.text.len() - rest.len();
        Err(SyntaxError {
            message: "unexpected text after the value".into(),
            span: line.span(from, line.text.len()),
        })
    }

    /// A `|` or `>` scalar whose header is on the current line at `offset`.
    fn block_scalar(&mut self, parent_indent: usize, offset: usize) -> Parsed<Node> {
        let header = self.line().expect("caller is on a line");
        let spec = header.text[offset..].split('#').next().unwrap_or("").trim_end();
        let folded = spec.starts_with('>');
        let chomp = match &spec[1..] {
            "" => Chomp::Clip,
            "-" => Chomp::Strip,
            "+" => Chomp::Keep,
            _ => {
                return Err(SyntaxError {
                    message: "only `|`, `|-`, `|+`, `>`, `>-` and `>+` are supported here".into(),
                    span: header.span(offset, header.text.len()),
                });
            }
        };
        self.index += 1;

        let mut body: Vec<Line<'a>> = Vec::new();
        while let Some(line) = self.line() {
            if !line.text.is_empty() && line.indent <= parent_indent {
                break;
            }
            body.push(line);
            self.index += 1;
        }
        while body.last().is_some_and(|line| line.text.is_empty()) {
            body.pop();
        }
        let block_indent = body.iter().find(|line| !line.text.is_empty()).map_or(0, |line| line.indent);

        let mut rows: Vec<String> = Vec::with_capacity(body.len());
        for line in &body {
            if line.text.is_empty() {
                rows.push(String::new());
            } else if line.indent < block_indent {
                return Err(SyntaxError {
                    message: "this line is indented less than the first line of the block".into(),
                    span: line.span(0, line.text.len()),
                });
            } else {
                rows.push(format!("{}{}", " ".repeat(line.indent - block_indent), line.text));
            }
        }

        let mut text = if folded { fold(&rows) } else { rows.join("\n") };
        if !text.is_empty() && !matches!(chomp, Chomp::Strip) {
            text.push('\n');
        }
        let end = body.last().map_or(header.pos(header.text.len()), |l| l.pos(l.text.len()));
        Ok(Node { value: Value::Str(text), span: Span { start: header.pos(offset), end } })
    }

    // ---- flow collections ------------------------------------------------

    fn flow_skip(&self, cursor: &mut Cursor) -> Parsed<()> {
        loop {
            let Some(line) = self.lines.get(cursor.line) else {
                let last = self.lines[cursor.line - 1];
                return Err(SyntaxError {
                    message: "this `[` or `{` is never closed".into(),
                    span: last.span(last.text.len(), last.text.len()),
                });
            };
            let rest = &line.text[cursor.offset..];
            let trimmed = rest.trim_start();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                cursor.line += 1;
                cursor.offset = 0;
                continue;
            }
            cursor.offset += rest.len() - trimmed.len();
            return Ok(());
        }
    }

    fn flow_peek(&self, cursor: &mut Cursor) -> Parsed<u8> {
        self.flow_skip(cursor)?;
        Ok(self.lines[cursor.line].text.as_bytes()[cursor.offset])
    }

    fn flow_value(&mut self, cursor: &mut Cursor) -> Parsed<Node> {
        let first = self.flow_peek(cursor)?;
        let line = self.lines[cursor.line];
        let start = line.pos(cursor.offset);
        match first {
            b'[' => {
                cursor.offset += 1;
                let mut items = Vec::new();
                loop {
                    if self.flow_peek(cursor)? == b']' {
                        break;
                    }
                    items.push(self.flow_value(cursor)?);
                    match self.flow_peek(cursor)? {
                        b',' => cursor.offset += 1,
                        b']' => break,
                        _ => return Err(self.flow_error(*cursor, "expected `,` or `]`")),
                    }
                }
                cursor.offset += 1;
                let end = self.lines[cursor.line].pos(cursor.offset);
                Ok(Node { value: Value::Seq(items), span: Span { start, end } })
            }
            b'{' => {
                cursor.offset += 1;
                let mut entries: Vec<Entry> = Vec::new();
                loop {
                    if self.flow_peek(cursor)? == b'}' {
                        break;
                    }
                    let (key, key_span) = self.flow_key(cursor)?;
                    if entries.iter().any(|entry| entry.key == key) {
                        return Err(SyntaxError {
                            message: format!("duplicate key `{key}`"),
                            span: key_span,
                        });
                    }
                    let value = match self.flow_peek(cursor)? {
                        b',' | b'}' => Node { value: Value::Null, span: key_span },
                        _ => self.flow_value(cursor)?,
                    };
                    entries.push(Entry { key, key_span, value });
                    match self.flow_peek(cursor)? {
                        b',' => cursor.offset += 1,
                        b'}' => break,
                        _ => return Err(self.flow_error(*cursor, "expected `,` or `}`")),
                    }
                }
                cursor.offset += 1;
                let end = self.lines[cursor.line].pos(cursor.offset);
                Ok(Node { value: Value::Map(entries), span: Span { start, end } })
            }
            b'"' | b'\'' => {
                let (text, end) = quoted(&line, cursor.offset)?;
                let span = line.span(cursor.offset, end);
                cursor.offset = end;
                Ok(Node { value: Value::Str(text), span })
            }
            _ => {
                let length = plain_end(&line, cursor.offset, PlainStop::Flow)?;
                let raw = line.text[cursor.offset..cursor.offset + length].trim_end();
                if raw.is_empty() {
                    return Err(self.flow_error(*cursor, "expected a value"));
                }
                let span = line.span(cursor.offset, cursor.offset + raw.len());
                cursor.offset += length;
                Ok(Node { value: plain_value(raw), span })
            }
        }
    }

    fn flow_key(&mut self, cursor: &mut Cursor) -> Parsed<(String, Span)> {
        let line = self.lines[cursor.line];
        let first = line.text.as_bytes()[cursor.offset];
        let (key, span) = if first == b'"' || first == b'\'' {
            let (text, end) = quoted(&line, cursor.offset)?;
            let span = line.span(cursor.offset, end);
            cursor.offset = end;
            (text, span)
        } else {
            let length = plain_end(&line, cursor.offset, PlainStop::FlowKey)?;
            let raw = line.text[cursor.offset..cursor.offset + length].trim_end();
            let span = line.span(cursor.offset, cursor.offset + raw.len());
            cursor.offset += length;
            (raw.to_string(), span)
        };
        if key.is_empty() {
            return Err(self.flow_error(*cursor, "expected a key"));
        }
        if self.flow_peek(cursor)? != b':' {
            return Err(self.flow_error(*cursor, "expected `:` after the key"));
        }
        cursor.offset += 1;
        Ok((key, span))
    }

    fn flow_error(&self, cursor: Cursor, message: &str) -> SyntaxError {
        let line = self.lines[cursor.line];
        let end = (cursor.offset + 1).min(line.text.len());
        SyntaxError { message: message.into(), span: line.span(cursor.offset, end) }
    }
}

enum Chomp {
    Clip,
    Strip,
    Keep,
}

/// Joins the lines of a `>` scalar: neighbouring text lines become one line,
/// a blank line becomes a line break, and more-indented lines are kept as is.
fn fold(rows: &[String]) -> String {
    let mut out = String::new();
    let mut previous_was_text = false;
    for row in rows {
        let literal = row.starts_with(' ');
        if row.is_empty() {
            out.push('\n');
            previous_was_text = false;
        } else {
            if previous_was_text {
                out.push(if literal { '\n' } else { ' ' });
            }
            out.push_str(row);
            if literal {
                out.push('\n');
            }
            previous_was_text = !literal;
        }
    }
    out
}

fn is_seq_item(text: &str) -> bool {
    text == "-" || text.starts_with("- ")
}

fn is_block_scalar_header(content: &str) -> bool {
    let head = content.split('#').next().unwrap_or("").trim_end();
    (head.starts_with('|') || head.starts_with('>')) && head.len() <= 2
}

struct Key {
    text: String,
    span: Span,
    /// Offset just past the colon.
    after: usize,
}

/// Finds `key:` at the start of a line, if it has one.
fn split_key(line: &Line<'_>) -> Parsed<Option<Key>> {
    key_at(line, 0)
}

fn starts_map_entry(line: &Line<'_>, offset: usize) -> Parsed<bool> {
    let first = line.text.as_bytes()[offset];
    if first == b'[' || first == b'{' {
        return Ok(false);
    }
    Ok(key_at(line, offset)?.is_some())
}

fn key_at(line: &Line<'_>, offset: usize) -> Parsed<Option<Key>> {
    let bytes = line.text.as_bytes();
    let ends_key = |at: usize| bytes[at] == b':' && bytes.get(at + 1).is_none_or(|b| *b == b' ');
    if bytes[offset] == b'"' || bytes[offset] == b'\'' {
        let (text, end) = quoted(line, offset)?;
        let rest = &line.text[end..];
        let colon = end + rest.len() - rest.trim_start().len();
        if colon < bytes.len() && ends_key(colon) {
            return Ok(Some(Key { text, span: line.span(offset, end), after: colon + 1 }));
        }
        return Ok(None);
    }
    let mut at = offset;
    while at < bytes.len() {
        match bytes[at] {
            b'$' if bytes.get(at + 1) == Some(&b'$') => at += 2,
            b'$' if bytes.get(at + 1) == Some(&b'{') => at = skip_expression(line, at)?,
            b'#' if at > offset && bytes[at - 1] == b' ' => return Ok(None),
            b':' if ends_key(at) => {
                let text = line.text[offset..at].trim_end();
                if text.is_empty() {
                    return Ok(None);
                }
                let span = line.span(offset, offset + text.len());
                return Ok(Some(Key { text: text.to_string(), span, after: at + 1 }));
            }
            _ => at += 1,
        }
    }
    Ok(None)
}

#[derive(Clone, Copy, PartialEq)]
enum PlainStop {
    /// Ends at a comment or the end of the line.
    Block,
    /// Also ends at `,`, `]` and `}`.
    Flow,
    /// Also ends at `:`.
    FlowKey,
}

/// Length of the plain scalar that starts at `offset`.
fn plain_end(line: &Line<'_>, offset: usize, stop: PlainStop) -> Parsed<usize> {
    let bytes = line.text.as_bytes();
    let mut at = offset;
    while at < bytes.len() {
        match bytes[at] {
            b'$' if bytes.get(at + 1) == Some(&b'$') => at += 2,
            b'$' if bytes.get(at + 1) == Some(&b'{') => at = skip_expression(line, at)?,
            b'#' if at > offset && bytes[at - 1] == b' ' => break,
            b',' | b']' | b'}' if stop != PlainStop::Block => break,
            b':' if stop == PlainStop::FlowKey => break,
            _ => at += 1,
        }
    }
    Ok(at - offset)
}

/// Given the offset of the `$` in `${`, returns the offset after its `}`.
fn skip_expression(line: &Line<'_>, dollar: usize) -> Parsed<usize> {
    let bytes = line.text.as_bytes();
    let mut depth = 0usize;
    let mut at = dollar + 1;
    while at < bytes.len() {
        match bytes[at] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(at + 1);
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
    Err(SyntaxError {
        message: "this `${` is never closed; an expression must end on the line it starts".into(),
        span: line.span(dollar, line.text.len()),
    })
}

/// Reads a quoted string starting at `offset`; returns it and the offset after
/// the closing quote.
fn quoted(line: &Line<'_>, offset: usize) -> Parsed<(String, usize)> {
    let quote = line.text[offset..].chars().next().expect("caller saw a quote");
    let mut out = String::new();
    let mut chars = line.text[offset + 1..].char_indices();
    let unterminated = || SyntaxError {
        message: "this string is never closed (strings cannot span lines; use `|`)".into(),
        span: line.span(offset, line.text.len()),
    };
    while let Some((at, ch)) = chars.next() {
        let here = offset + 1 + at;
        if ch == quote {
            if quote == '\'' && line.text[here + 1..].starts_with('\'') {
                chars.next();
                out.push('\'');
                continue;
            }
            return Ok((out, here + 1));
        }
        if ch != '\\' || quote == '\'' {
            out.push(ch);
            continue;
        }
        let (_, escape) = chars.next().ok_or_else(unterminated)?;
        match escape {
            'n' => out.push('\n'),
            't' => out.push('\t'),
            'r' => out.push('\r'),
            '0' => out.push('\0'),
            '"' | '\\' | '/' | '\'' | '$' => out.push(escape),
            'u' => {
                let digits: String = chars.by_ref().take(4).map(|(_, digit)| digit).collect();
                let decoded = u32::from_str_radix(&digits, 16).ok().and_then(char::from_u32);
                out.push(decoded.ok_or_else(|| SyntaxError {
                    message: "`\\u` must be followed by four hex digits".into(),
                    span: line.span(here, here + 2 + digits.len()),
                })?);
            }
            other => {
                return Err(SyntaxError {
                    // Most often a path of a system that writes its
                    // paths with backslashes, put in double quotes.
                    message: format!(
                        "unknown escape `\\{other}`: inside double quotes a backslash begins an escape. Write the \
                         backslash twice, or put the value in single quotes, where a backslash is itself \
                         ('C:\\Users\\me')"
                    ),
                    span: line.span(here, here + 1 + other.len_utf8()),
                });
            }
        }
    }
    Err(unterminated())
}

/// The typed value of an unquoted scalar.
fn plain_value(raw: &str) -> Value {
    match raw {
        "" | "null" | "~" => return Value::Null,
        "true" => return Value::Bool(true),
        "false" => return Value::Bool(false),
        _ => {}
    }
    let digits = raw.strip_prefix('-').unwrap_or(raw);
    if !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && let Ok(number) = raw.parse::<i64>()
    {
        return Value::Int(number);
    }
    let numeric = digits.starts_with(|c: char| c.is_ascii_digit())
        && digits.bytes().all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'e' | b'E' | b'+' | b'-'));
    if numeric && let Ok(number) = raw.parse::<f64>() {
        return Value::Float(number);
    }
    Value::Str(raw.to_string())
}

fn reject_unsupported(line: &Line<'_>) -> Parsed<()> {
    let message = if line.text.starts_with("<<:") {
        "merge keys (`<<`) are not supported"
    } else if line.text.starts_with("? ") {
        "complex keys (`?`) are not supported"
    } else if line.text.starts_with("---") || line.text.starts_with("...") && !line.text.starts_with("...:") {
        // `...:` is a key, by which a mapping hands on another's entries.
        "multiple documents (`---`) are not supported"
    } else {
        return reject_indicator(line, 0);
    };
    Err(SyntaxError { message: message.into(), span: line.span(0, line.text.len()) })
}

fn reject_indicator(line: &Line<'_>, offset: usize) -> Parsed<()> {
    let message = match line.text.as_bytes()[offset] {
        b'&' => "anchors (`&name`) are not supported",
        b'*' => "aliases (`*name`) are not supported; quote the value if it is text",
        b'!' => "tags (`!type`) are not supported; to negate a condition write `${ !... }`",
        b'%' => "directives (`%`) are not supported",
        b'@' | b'`' => "a value cannot start with this character; quote it",
        _ => return Ok(()),
    };
    Err(SyntaxError { message: message.into(), span: line.span(offset, line.text.len()) })
}
