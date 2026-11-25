//! The expression language used inside `${ ... }`.
//!
//! Deliberately small and total: literals, paths into the run's data,
//! arithmetic, comparison, boolean logic, `a ? b : c`, `a ?? b` and a handful
//! of pure functions. There are no loops, assignments or user functions, so an
//! expression always terminates and can always be drawn as a label on an edge.
//! Anything that needs more than this belongs in an action.

use std::borrow::Cow;

use serde_json::{Map, Number, Value};

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Lit(Value),
    /// A name at the root of the scope, such as `params`.
    Var(String),
    Member(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    List(Vec<Expr>),
    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BinaryOp {
    Or,
    And,
    Coalesce,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    In,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

/// A problem in an expression. `offset` is in bytes from its start.
#[derive(Clone, Debug, PartialEq)]
pub struct ExprError {
    pub message: String,
    pub offset: usize,
}

impl ExprError {
    fn new(message: impl Into<String>, offset: usize) -> Self {
        Self { message: message.into(), offset }
    }
}

pub const FUNCTIONS: &[(&str, &str)] = &[
    ("len", "len(x): length of a string, list or mapping"),
    ("contains", "contains(haystack, needle): substring, list member or key"),
    ("starts_with", "starts_with(text, prefix): whether the text begins with the prefix"),
    ("ends_with", "ends_with(text, suffix): whether the text ends with the suffix"),
    ("lower", "lower(text): the text in small letters, to compare words however they were written"),
    ("upper", "upper(text): the text in capital letters"),
    ("trim", "trim(text): the text without the spaces and lines at either end"),
    ("string", "string(x): x as text, a number or a list written out"),
    ("int", "int(x): x as a whole number, from text or a number"),
    ("float", "float(x): x as a number that may have a fraction, from text or a number"),
    ("json", "json(x): x written as JSON text, for a program that reads it"),
    ("keys", "keys(mapping): its keys as a list"),
    ("join", "join(list, separator): the elements of a list as one text, the separator between each two"),
    ("split", "split(text, separator): the text cut into a list wherever the separator is"),
    ("replace", "replace(text, from, to): the text with every `from` in it written as `to`"),
    ("matches", "matches(text, pattern): whether the pattern is found in the text; `^` and `$` are its ends"),
    ("slug", "slug(text): lowercase words joined by dashes"),
    ("min", "min(a, b, ...): the least of the numbers it is handed"),
    ("max", "max(a, b, ...): the greatest of the numbers it is handed"),
    (
        "where",
        "where(list, field, value): the mappings of a list whose field is that value; a field within a field is `a.b`",
    ),
    (
        "sort",
        "sort(list) or sort(list, field): smallest first, a list of mappings by a field of each; equals keep their order",
    ),
    (
        "without",
        "without(list, value): the list with that value left out, or with every value of a second list left out",
    ),
    ("range", "range(n): the whole numbers from 1 to n, as a list; none for less than 1"),
    (
        "pluck",
        "pluck(list, field): that field of each mapping of a list, as a list; one that has none is left out",
    ),
    (
        "at",
        "at(list, places): the elements at those places, counted from 1, in the order asked; a place nothing is at is left out",
    ),
    ("from_json", "from_json(text): what the JSON text says, as a value"),
    ("sum", "sum(list): the numbers of a list added up; nothing, for an empty one, is 0"),
];

pub fn parse(source: &str) -> Result<Expr, ExprError> {
    let tokens = lex(source)?;
    let mut parser = Parser { tokens, at: 0, end: source.len() };
    let expr = parser.ternary()?;
    match parser.peek() {
        None => Ok(expr),
        Some(token) => Err(ExprError::new("unexpected text after the expression", token.offset)),
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Ident(String),
    Number(Number),
    Text(String),
    Sym(&'static str),
}

#[derive(Clone, Debug)]
struct Token {
    tok: Tok,
    offset: usize,
}

const SYMBOLS: &[&str] = &[
    "&&", "||", "??", "==", "!=", "<=", ">=", "<", ">", "!", "+", "-", "*", "/", "%", "(", ")", "[", "]",
    ",", ".", "?", ":",
];

fn lex(source: &str) -> Result<Vec<Token>, ExprError> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let byte = bytes[at];
        if byte.is_ascii_whitespace() {
            at += 1;
        } else if byte.is_ascii_alphabetic() || byte == b'_' {
            let start = at;
            while at < bytes.len() && (bytes[at].is_ascii_alphanumeric() || bytes[at] == b'_') {
                at += 1;
            }
            tokens.push(Token { tok: Tok::Ident(source[start..at].to_string()), offset: start });
        } else if byte.is_ascii_digit() {
            let start = at;
            while at < bytes.len() && (bytes[at].is_ascii_digit() || bytes[at] == b'.') {
                // `1.field` is not a number followed by a member access we
                // need to support, but `a[0].x` is: stop at a dot that is not
                // followed by a digit.
                if bytes[at] == b'.' && !bytes.get(at + 1).is_some_and(u8::is_ascii_digit) {
                    break;
                }
                at += 1;
            }
            let text = &source[start..at];
            let number = if text.contains('.') {
                text.parse::<f64>().ok().and_then(Number::from_f64)
            } else {
                text.parse::<i64>().ok().map(Number::from)
            };
            let number = number.ok_or_else(|| ExprError::new("not a valid number", start))?;
            tokens.push(Token { tok: Tok::Number(number), offset: start });
        } else if byte == b'"' || byte == b'\'' {
            let start = at;
            let mut text = String::new();
            let mut chars = source[at + 1..].char_indices();
            let mut closed = false;
            while let Some((index, ch)) = chars.next() {
                if ch as u32 == byte as u32 {
                    at = at + 1 + index + 1;
                    closed = true;
                    break;
                }
                if ch == '\\' {
                    match chars.next() {
                        Some((_, 'n')) => text.push('\n'),
                        Some((_, 't')) => text.push('\t'),
                        Some((_, other)) => text.push(other),
                        None => break,
                    }
                } else {
                    text.push(ch);
                }
            }
            if !closed {
                return Err(ExprError::new("this string is never closed", start));
            }
            tokens.push(Token { tok: Tok::Text(text), offset: start });
        } else if let Some(symbol) = SYMBOLS.iter().find(|s| source[at..].starts_with(**s)) {
            tokens.push(Token { tok: Tok::Sym(symbol), offset: at });
            at += symbol.len();
        } else {
            let ch = source[at..].chars().next().unwrap_or('?');
            let hint = if ch == '=' { "; use `==` to compare" } else { "" };
            return Err(ExprError::new(format!("unexpected `{ch}`{hint}"), at));
        }
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    at: usize,
    end: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at)
    }

    fn offset(&self) -> usize {
        self.peek().map_or(self.end, |token| token.offset)
    }

    fn eat(&mut self, symbol: &str) -> bool {
        let found = matches!(self.peek(), Some(Token { tok: Tok::Sym(s), .. }) if *s == symbol);
        if found {
            self.at += 1;
        }
        found
    }

    fn expect(&mut self, symbol: &str) -> Result<(), ExprError> {
        if self.eat(symbol) {
            Ok(())
        } else {
            Err(ExprError::new(format!("expected `{symbol}`"), self.offset()))
        }
    }

    fn ternary(&mut self) -> Result<Expr, ExprError> {
        let condition = self.binary(0)?;
        if !self.eat("?") {
            return Ok(condition);
        }
        let then = self.ternary()?;
        self.expect(":")?;
        let otherwise = self.ternary()?;
        Ok(Expr::Ternary(Box::new(condition), Box::new(then), Box::new(otherwise)))
    }

    /// Precedence climbing over the binary operators, loosest first.
    fn binary(&mut self, level: usize) -> Result<Expr, ExprError> {
        const LEVELS: &[&[(&str, BinaryOp)]] = &[
            &[("??", BinaryOp::Coalesce)],
            &[("||", BinaryOp::Or)],
            &[("&&", BinaryOp::And)],
            &[("==", BinaryOp::Eq), ("!=", BinaryOp::Ne)],
            &[
                ("<=", BinaryOp::Le),
                (">=", BinaryOp::Ge),
                ("<", BinaryOp::Lt),
                (">", BinaryOp::Gt),
                ("in", BinaryOp::In),
            ],
            &[("+", BinaryOp::Add), ("-", BinaryOp::Sub)],
            &[("*", BinaryOp::Mul), ("/", BinaryOp::Div), ("%", BinaryOp::Rem)],
        ];
        let Some(operators) = LEVELS.get(level) else { return self.unary() };
        let mut left = self.binary(level + 1)?;
        loop {
            let found = operators.iter().find(|(symbol, _)| match self.peek().map(|t| &t.tok) {
                Some(Tok::Sym(s)) => s == symbol,
                Some(Tok::Ident(name)) => name == symbol,
                _ => false,
            });
            let Some((_, op)) = found else { return Ok(left) };
            self.at += 1;
            let right = self.binary(level + 1)?;
            left = Expr::Binary(*op, Box::new(left), Box::new(right));
        }
    }

    fn unary(&mut self) -> Result<Expr, ExprError> {
        if self.eat("!") {
            return Ok(Expr::Unary(UnaryOp::Not, Box::new(self.unary()?)));
        }
        if self.eat("-") {
            return Ok(Expr::Unary(UnaryOp::Neg, Box::new(self.unary()?)));
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Expr, ExprError> {
        let mut expr = self.primary()?;
        loop {
            if self.eat(".") {
                let offset = self.offset();
                match self.tokens.get(self.at).map(|token| token.tok.clone()) {
                    Some(Tok::Ident(name)) => {
                        self.at += 1;
                        expr = Expr::Member(Box::new(expr), name);
                    }
                    _ => return Err(ExprError::new("expected a name after `.`", offset)),
                }
            } else if self.eat("[") {
                let index = self.ternary()?;
                self.expect("]")?;
                expr = Expr::Index(Box::new(expr), Box::new(index));
            } else {
                return Ok(expr);
            }
        }
    }

    fn primary(&mut self) -> Result<Expr, ExprError> {
        let offset = self.offset();
        let Some(token) = self.tokens.get(self.at).cloned() else {
            return Err(ExprError::new("the expression ends too soon", offset));
        };
        self.at += 1;
        match token.tok {
            Tok::Number(number) => Ok(Expr::Lit(Value::Number(number))),
            Tok::Text(text) => Ok(Expr::Lit(Value::String(text))),
            Tok::Ident(name) => match name.as_str() {
                "true" => Ok(Expr::Lit(Value::Bool(true))),
                "false" => Ok(Expr::Lit(Value::Bool(false))),
                "null" => Ok(Expr::Lit(Value::Null)),
                _ if self.eat("(") => {
                    if !FUNCTIONS.iter().any(|(known, _)| *known == name) {
                        let known: Vec<&str> = FUNCTIONS.iter().map(|(n, _)| *n).collect();
                        // The one that was probably meant, before all there are.
                        let said = match crate::diag::did_you_mean(&name, known.iter().copied()) {
                            Some(close) => format!("did you mean `{close}`?"),
                            None => format!("there are: {}", known.join(", ")),
                        };
                        return Err(ExprError::new(format!("unknown function `{name}`; {said}"), offset));
                    }
                    let arguments = self.arguments(")")?;
                    // A pattern written out is read here, so that one that
                    // does not read is a mistake in the file, with where it
                    // is, and not a step that fails when it is reached.
                    if let ("matches", Some(Expr::Lit(Value::String(pattern)))) =
                        (name.as_str(), arguments.get(1))
                    {
                        pattern_of(pattern).map_err(|err| ExprError::new(err, offset))?;
                    }
                    Ok(Expr::Call(name, arguments))
                }
                _ => Ok(Expr::Var(name)),
            },
            Tok::Sym("(") => {
                let inner = self.ternary()?;
                self.expect(")")?;
                Ok(inner)
            }
            Tok::Sym("[") => Ok(Expr::List(self.arguments("]")?)),
            Tok::Sym(symbol) => Err(ExprError::new(format!("unexpected `{symbol}`"), offset)),
        }
    }

    fn arguments(&mut self, close: &str) -> Result<Vec<Expr>, ExprError> {
        let mut items = Vec::new();
        if self.eat(close) {
            return Ok(items);
        }
        loop {
            items.push(self.ternary()?);
            if self.eat(close) {
                return Ok(items);
            }
            self.expect(",")?;
        }
    }
}

impl Expr {
    /// Every path this expression reads, as its leading run of plain names:
    /// `steps.build.out.code > 0` yields `["steps", "build", "out", "code"]`.
    pub fn paths(&self) -> Vec<Vec<String>> {
        let mut found = Vec::new();
        self.collect_paths(&mut found);
        found
    }

    fn static_path(&self) -> Option<Vec<String>> {
        match self {
            Expr::Var(name) => Some(vec![name.clone()]),
            Expr::Member(base, name) => {
                let mut path = base.static_path()?;
                path.push(name.clone());
                Some(path)
            }
            Expr::Index(base, index) => match index.as_ref() {
                Expr::Lit(Value::String(key)) => {
                    let mut path = base.static_path()?;
                    path.push(key.clone());
                    Some(path)
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn collect_paths(&self, found: &mut Vec<Vec<String>>) {
        if let Some(path) = self.static_path() {
            found.push(path);
            return;
        }
        match self {
            Expr::Lit(_) | Expr::Var(_) => {}
            Expr::Member(base, _) | Expr::Unary(_, base) => base.collect_paths(found),
            Expr::Index(base, index) => {
                base.collect_paths(found);
                index.collect_paths(found);
            }
            Expr::List(items) | Expr::Call(_, items) => {
                items.iter().for_each(|item| item.collect_paths(found));
            }
            Expr::Binary(_, left, right) => {
                left.collect_paths(found);
                right.collect_paths(found);
            }
            Expr::Ternary(a, b, c) => {
                a.collect_paths(found);
                b.collect_paths(found);
                c.collect_paths(found);
            }
        }
    }

    /// Evaluates against `scope`, whose keys are the root names.
    ///
    /// A path that leads nowhere is `null`, not an error, so that
    /// `steps.build.out.warnings ?? 0` reads naturally. Type errors (adding a
    /// list to a number) are errors.
    pub fn eval(&self, scope: &Map<String, Value>) -> Result<Value, String> {
        Ok(match self {
            Expr::Lit(value) => value.clone(),
            Expr::Var(name) => scope.get(name).cloned().unwrap_or(Value::Null),
            Expr::Member(..) | Expr::Index(..) => self.lookup(scope)?.map_or(Value::Null, Cow::into_owned),
            Expr::List(items) => Value::Array(items.iter().map(|i| i.eval(scope)).collect::<Result<_, _>>()?),
            Expr::Unary(UnaryOp::Not, inner) => Value::Bool(!truthy(&inner.eval(scope)?)),
            Expr::Unary(UnaryOp::Neg, inner) => match inner.eval(scope)? {
                Value::Number(n) if n.is_i64() => Value::from(-n.as_i64().unwrap_or(0)),
                Value::Number(n) => float(-n.as_f64().unwrap_or(0.0)),
                other => return Err(format!("cannot negate {}", describe(&other))),
            },
            Expr::Binary(BinaryOp::And, left, right) => {
                Value::Bool(truthy(&left.eval(scope)?) && truthy(&right.eval(scope)?))
            }
            Expr::Binary(BinaryOp::Or, left, right) => {
                Value::Bool(truthy(&left.eval(scope)?) || truthy(&right.eval(scope)?))
            }
            Expr::Binary(BinaryOp::Coalesce, left, right) => match left.eval(scope)? {
                Value::Null => right.eval(scope)?,
                value => value,
            },
            Expr::Binary(op, left, right) => binary(*op, left.eval(scope)?, right.eval(scope)?)?,
            Expr::Ternary(condition, then, otherwise) => {
                if truthy(&condition.eval(scope)?) {
                    then.eval(scope)?
                } else {
                    otherwise.eval(scope)?
                }
            }
            Expr::Call(name, arguments) => {
                let values: Vec<Value> = arguments.iter().map(|a| a.eval(scope)).collect::<Result<_, _>>()?;
                call(name, &values)?
            }
        })
    }

    /// Follows a member/index chain by reference where it can, so reading one
    /// field of a large step output does not clone the output.
    fn lookup<'a>(&self, scope: &'a Map<String, Value>) -> Result<Option<Cow<'a, Value>>, String> {
        match self {
            Expr::Var(name) => Ok(scope.get(name).map(Cow::Borrowed)),
            Expr::Member(base, name) => Ok(base.lookup(scope)?.and_then(|value| child(value, name))),
            Expr::Index(base, index) => {
                let Some(target) = base.lookup(scope)? else { return Ok(None) };
                match index.eval(scope)? {
                    Value::String(key) => Ok(child(target, &key)),
                    Value::Number(n) => {
                        let len = target.as_array().map_or(0, Vec::len) as i64;
                        let at = n.as_i64().unwrap_or(-1);
                        let at = if at < 0 { at + len } else { at };
                        let Ok(at) = usize::try_from(at) else { return Ok(None) };
                        Ok(match target {
                            Cow::Borrowed(value) => value.get(at).map(Cow::Borrowed),
                            Cow::Owned(value) => value.get(at).cloned().map(Cow::Owned),
                        })
                    }
                    other => Err(format!("cannot index with {}", describe(&other))),
                }
            }
            other => other.eval(scope).map(|value| Some(Cow::Owned(value))),
        }
    }
}

fn child<'a>(value: Cow<'a, Value>, key: &str) -> Option<Cow<'a, Value>> {
    match value {
        Cow::Borrowed(value) => value.get(key).map(Cow::Borrowed),
        Cow::Owned(value) => value.get(key).cloned().map(Cow::Owned),
    }
}

/// `null`, `false`, `0`, `""`, `[]` and `{}` are false; everything else is true.
pub fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        Value::Number(number) => number.as_f64().is_some_and(|n| n != 0.0),
        Value::String(text) => !text.is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) => !map.is_empty(),
    }
}

/// How a value appears when spliced into text.
/// The whole numbers from 1 to `count`, none for less than 1: what
/// `range()` gives, and what a `for_each` goes over that is handed a number
/// and not a list. An error, as the end of a sentence, where that is not
/// a whole number or is more than can be meant.
pub fn up_to(count: &Value) -> Result<Vec<Value>, String> {
    let count = match count {
        Value::Number(count) => count.as_i64(),
        _ => None,
    };
    let count = count.ok_or_else(|| "takes a whole number".to_string())?;
    // Something to go through one by one, and not a way to make a list of
    // any length: a blueprint that asks for a million of something has
    // made a mistake.
    if count > 1000 {
        return Err(format!("goes to 1000 at most, and was asked for {count}"));
    }
    Ok((1..=count).map(Value::from).collect())
}

pub fn display(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn describe(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "a list",
        Value::Object(_) => "a mapping",
    }
}

fn float(number: f64) -> Value {
    Number::from_f64(number).map_or(Value::Null, Value::Number)
}

fn binary(op: BinaryOp, left: Value, right: Value) -> Result<Value, String> {
    use BinaryOp::*;
    match op {
        Eq => return Ok(Value::Bool(loosely_equal(&left, &right))),
        Ne => return Ok(Value::Bool(!loosely_equal(&left, &right))),
        In => return call("contains", &[right, left]),
        _ => {}
    }
    if let (Add, Value::String(a), b) | (Add, b, Value::String(a)) = (op, &left, &right) {
        let left_is_text = left.is_string();
        let other = display(b);
        return Ok(Value::String(if left_is_text { format!("{a}{other}") } else { format!("{other}{a}") }));
    }
    if let (Add, Value::Array(a), Value::Array(b)) = (op, &left, &right) {
        return Ok(Value::Array(a.iter().chain(b).cloned().collect()));
    }
    if let (Value::String(a), Value::String(b)) = (&left, &right) {
        let ordering = a.cmp(b);
        return Ok(Value::Bool(match op {
            Lt => ordering.is_lt(),
            Le => ordering.is_le(),
            Gt => ordering.is_gt(),
            Ge => ordering.is_ge(),
            _ => return Err("text only supports `+` and comparison".to_string()),
        }));
    }
    let (Value::Number(a), Value::Number(b)) = (&left, &right) else {
        return Err(format!("cannot apply this operator to {} and {}", describe(&left), describe(&right)));
    };
    if let (Some(a), Some(b)) = (a.as_i64(), b.as_i64()) {
        let arithmetic = match op {
            Add => a.checked_add(b),
            Sub => a.checked_sub(b),
            Mul => a.checked_mul(b),
            Div if b != 0 && a % b == 0 => Some(a / b),
            Rem if b != 0 => Some(a % b),
            Rem => return Err("division by zero".to_string()),
            Lt => return Ok(Value::Bool(a < b)),
            Le => return Ok(Value::Bool(a <= b)),
            Gt => return Ok(Value::Bool(a > b)),
            Ge => return Ok(Value::Bool(a >= b)),
            _ => None,
        };
        if let Some(result) = arithmetic {
            return Ok(Value::from(result));
        }
    }
    let (a, b) = (a.as_f64().unwrap_or(0.0), b.as_f64().unwrap_or(0.0));
    Ok(match op {
        Add => float(a + b),
        Sub => float(a - b),
        Mul => float(a * b),
        Div if b == 0.0 => return Err("division by zero".to_string()),
        Div => float(a / b),
        Rem => float(a % b),
        Lt => Value::Bool(a < b),
        Le => Value::Bool(a <= b),
        Gt => Value::Bool(a > b),
        Ge => Value::Bool(a >= b),
        _ => unreachable!("handled above"),
    })
}

/// Equality that treats `1` and `1.0` as the same number.
fn loosely_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        _ => left == right,
    }
}

fn call(name: &str, args: &[Value]) -> Result<Value, String> {
    let text = |index: usize| -> Result<&str, String> {
        args.get(index)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{name}(): argument {} must be a string", index + 1))
    };
    let arity = |expected: usize| -> Result<(), String> {
        if args.len() == expected {
            Ok(())
        } else {
            Err(format!("{name}() takes {expected} argument(s), got {}", args.len()))
        }
    };
    Ok(match name {
        "len" => {
            arity(1)?;
            Value::from(match &args[0] {
                Value::String(text) => text.chars().count(),
                Value::Array(items) => items.len(),
                Value::Object(map) => map.len(),
                Value::Null => 0,
                other => return Err(format!("len() of {}", describe(other))),
            })
        }
        "contains" => {
            arity(2)?;
            Value::Bool(match &args[0] {
                Value::String(haystack) => haystack.contains(display(&args[1]).as_str()),
                Value::Array(items) => items.iter().any(|item| loosely_equal(item, &args[1])),
                Value::Object(map) => args[1].as_str().is_some_and(|key| map.contains_key(key)),
                Value::Null => false,
                other => return Err(format!("contains() on {}", describe(other))),
            })
        }
        "starts_with" => Value::Bool(text(0)?.starts_with(text(1)?)),
        "ends_with" => Value::Bool(text(0)?.ends_with(text(1)?)),
        "lower" => Value::String(text(0)?.to_lowercase()),
        "upper" => Value::String(text(0)?.to_uppercase()),
        "trim" => Value::String(text(0)?.trim().to_string()),
        "string" => {
            arity(1)?;
            Value::String(display(&args[0]))
        }
        "int" => {
            arity(1)?;
            match &args[0] {
                Value::Number(n) => {
                    Value::from(n.as_i64().unwrap_or_else(|| n.as_f64().unwrap_or(0.0) as i64))
                }
                Value::String(text) => Value::from(
                    text.trim()
                        .parse::<i64>()
                        .map_err(|_| format!("int(): `{text}` is not a whole number"))?,
                ),
                Value::Bool(flag) => Value::from(i64::from(*flag)),
                other => return Err(format!("int() of {}", describe(other))),
            }
        }
        "float" => {
            arity(1)?;
            match &args[0] {
                Value::Number(n) => float(n.as_f64().unwrap_or(0.0)),
                Value::String(text) => float(
                    text.trim().parse::<f64>().map_err(|_| format!("float(): `{text}` is not a number"))?,
                ),
                other => return Err(format!("float() of {}", describe(other))),
            }
        }
        "json" => {
            arity(1)?;
            Value::String(args[0].to_string())
        }
        "keys" => {
            arity(1)?;
            match &args[0] {
                Value::Object(map) => map.keys().cloned().map(Value::String).collect(),
                Value::Null => Value::Array(Vec::new()),
                other => return Err(format!("keys() of {}", describe(other))),
            }
        }
        "join" => {
            let items = args.first().and_then(Value::as_array).ok_or("join(): argument 1 must be a list")?;
            Value::String(items.iter().map(display).collect::<Vec<_>>().join(text(1)?))
        }
        "split" => text(0)?.split(text(1)?).map(|part| Value::String(part.to_string())).collect(),
        "replace" => Value::String(text(0)?.replace(text(1)?, text(2)?)),
        "matches" => {
            arity(2)?;
            Value::Bool(pattern_of(text(1)?)?.is_match(text(0)?))
        }
        "slug" => Value::String(slug(text(0)?)),
        "min" | "max" => {
            let mut best: Option<f64> = None;
            let mut all_whole = true;
            for arg in args {
                let Value::Number(n) = arg else {
                    return Err(format!("{name}() takes numbers, got {}", describe(arg)));
                };
                all_whole &= n.is_i64();
                let value = n.as_f64().unwrap_or(0.0);
                best = Some(match best {
                    Some(current) if name == "min" => current.min(value),
                    Some(current) => current.max(value),
                    None => value,
                });
            }
            let best = best.ok_or_else(|| format!("{name}() needs at least one argument"))?;
            if all_whole { Value::from(best as i64) } else { float(best) }
        }
        "where" => {
            arity(3)?;
            let field = text(1)?;
            let missing = Value::Null;
            list_of(name, &args[0])?
                .iter()
                .filter(|item| loosely_equal(within(item, field).unwrap_or(&missing), &args[2]))
                .cloned()
                .collect()
        }
        "sort" => {
            let field = match args.len() {
                1 => None,
                2 => Some(text(1)?),
                given => {
                    return Err(format!(
                        "sort() takes a list, and a field to sort it by; got {given} arguments"
                    ));
                }
            };
            let missing = Value::Null;
            let by = |item: &'_ Value| -> Value {
                match field {
                    Some(field) => within(item, field).unwrap_or(&missing).clone(),
                    None => item.clone(),
                }
            };
            let mut items = list_of(name, &args[0])?.to_vec();
            // Stable: sorting by one field and then by another orders by
            // the second, and within it by the first.
            items.sort_by(|a, b| order(&by(a), &by(b)));
            Value::Array(items)
        }
        "without" => {
            arity(2)?;
            // One value, or each of a list of them.
            let gone: &[Value] = match &args[1] {
                Value::Array(gone) => gone,
                one => std::slice::from_ref(one),
            };
            list_of(name, &args[0])?
                .iter()
                .filter(|item| !gone.iter().any(|gone| loosely_equal(item, gone)))
                .cloned()
                .collect()
        }
        "range" => {
            arity(1)?;
            Value::Array(up_to(&args[0]).map_err(|why| format!("range() {why}"))?)
        }
        "pluck" => {
            arity(2)?;
            let field = text(1)?;
            list_of(name, &args[0])?
                .iter()
                .filter_map(|item| within(item, field))
                .filter(|value| !value.is_null())
                .cloned()
                .collect()
        }
        "at" => {
            arity(2)?;
            let items = list_of(name, &args[0])?;
            // One place, or each of a list of them.
            let places: &[Value] = match &args[1] {
                Value::Array(places) => places,
                Value::Null => &[],
                one => std::slice::from_ref(one),
            };
            let mut found = Vec::new();
            for place in places {
                let place = place.as_i64().ok_or_else(|| {
                    format!("at(): a place is a whole number, counted from 1, not {}", describe(place))
                })?;
                if let Some(item) = usize::try_from(place - 1).ok().and_then(|index| items.get(index)) {
                    found.push(item.clone());
                }
            }
            Value::Array(found)
        }
        "sum" => {
            arity(1)?;
            let mut total = 0.0;
            let mut all_whole = true;
            for value in list_of(name, &args[0])? {
                let Value::Number(n) = value else {
                    return Err(format!("sum() adds numbers up, and the list has {}", describe(value)));
                };
                all_whole &= n.is_i64();
                total += n.as_f64().unwrap_or(0.0);
            }
            if all_whole { Value::from(total as i64) } else { float(total) }
        }
        "from_json" => {
            arity(1)?;
            serde_json::from_str(text(0)?).map_err(|err| format!("from_json(): this is not JSON: {err}"))?
        }
        _ => return Err(format!("unknown function `{name}`")),
    })
}

/// A field of a mapping, or of a mapping within it: `id`, or `data.title`
/// for the `title` of what a record holds.
fn within<'a>(item: &'a Value, field: &str) -> Option<&'a Value> {
    field.split('.').try_fold(item, |at, part| at.get(part))
}

/// The list a function was given; nothing is the empty list.
fn list_of<'a>(name: &str, value: &'a Value) -> Result<&'a [Value], String> {
    match value {
        Value::Array(items) => Ok(items),
        Value::Null => Ok(&[]),
        other => Err(format!("{name}(): argument 1 must be a list, not {}", describe(other))),
    }
}

/// Which of two values comes first: nothing, then false and true, then
/// numbers by size, then text by its letters. What cannot be told apart
/// that way stays as it was.
fn order(a: &Value, b: &Value) -> std::cmp::Ordering {
    let rank = |value: &Value| match value {
        Value::Null => 0,
        Value::Bool(_) => 1,
        Value::Number(_) => 2,
        Value::String(_) => 3,
        _ => 4,
    };
    match (a, b) {
        (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
        (Value::Number(a), Value::Number(b)) => {
            a.as_f64().partial_cmp(&b.as_f64()).unwrap_or(std::cmp::Ordering::Equal)
        }
        (Value::String(a), Value::String(b)) => a.cmp(b),
        _ => rank(a).cmp(&rank(b)),
    }
}

/// A pattern as `matches` takes it. It is looked for anywhere in the text,
/// in time that grows only with the text, so an expression still always ends.
fn pattern_of(pattern: &str) -> Result<regex_lite::Regex, String> {
    regex_lite::Regex::new(pattern)
        .map_err(|err| format!("matches(): `{pattern}` does not read as a pattern: {err}"))
}

/// Lowercase ASCII words joined by single dashes: `Corner Shop!` becomes
/// `corner-shop`.
pub fn slug(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}
