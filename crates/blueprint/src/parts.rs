//! A text put together from parts, each said only where it is to be.
//!
//! What an agent is told is its blueprint's to say, and whoever manages
//! the workflow reads it and changes it without building anything. It is
//! long, and it is prose, so it is not typed out inside the blueprint: it
//! is a file beside the blueprints, in `prompts/`, which the blueprint
//! links. Which of the words are said often rests on whether something
//! was given: what was changed, where there is anything to show; a
//! sentence more, where several look. Written as one expression that is a
//! sum of choices (`a != '' ? 'It was ' + a : ''`), that is a program in
//! a hole in a string.
//!
//! Here it is a list of parts in the order they are said. A part is words
//! written where they are said; or a mapping that says `say:` (words, or
//! parts of its own) or `from:` (a file of `prompts/`, with what fills
//! its blanks under `with:`) or `own:` (a file of the `prompts/` of the
//! project's own blueprints, so that a row of a table can name words a
//! project wrote for itself), and beside it, where the part is only
//! sometimes said, a plain `if:`, and what is said `else`. Putting them
//! together is
//! [`put`], a function of the parts and of the files they link, which a
//! blueprint asks as `text.put` (`blocks::words`; the runner offers what
//! is here as `conductor::words`).
//!
//! A linked file is a text of the library and is found as one
//! ([`linked_as`], then the door a text asked on an instance's behalf
//! comes through): what ships as the program brought it, and a project's
//! own only as it was vouched for. Nothing here reads a file.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::expr::Expr;
use crate::template::{Template, holes};

/// What a part that is not words alone may say.
const SAYS: [&str; 6] = ["if", "say", "from", "own", "with", "else"];

/// What a blueprint asks to have parts put together.
pub const ACTION: &str = "text.put";

/// The folder linked files are in, beside the blueprints of their place.
pub const FOLDER: &str = "prompts";

/// What a linked file's name ends with.
pub const KIND: &str = ".md";

/// Whether `name` is a file a text links (a prompt, in [`FOLDER`] beside
/// the blueprints of its place) and no blueprint: words, which say no
/// version and mean what they say to the letter.
pub fn is_linked(name: &str) -> bool {
    name.ends_with(KIND)
        && name.rsplit_once('/').is_some_and(|(place, _)| place.split('/').any(|part| part == FOLDER))
}

/// What parts came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Put {
    /// The parts that were said, with nothing between them; or what was
    /// to stand for them, where they said nothing but space.
    pub text: String,
    /// Whether any part said anything but space.
    pub said: bool,
}

/// The files parts link, each by what its part calls it (`from:`), with
/// its text, and a file of the project's own (`own:`) by [`own_key`].
/// Those of parts that are not said need not be there.
pub type Files = BTreeMap<String, String>;

/// What a file of the project's own a part names (`own:`) is kept under
/// among the [`Files`]: its name after a `/`, which no name a part gives
/// with `from:` begins with, so that the two are never taken for each
/// other.
pub fn own_key(own: &str) -> String {
    format!("/{own}")
}

/// What a text of the library is called that a part names as a file of
/// the project's own (`own:`): a file of the `prompts/` at the top of where
/// the project keeps its own blueprints, whatever blueprint names it.
pub fn own_as(own: &str) -> String {
    format!("{FOLDER}/{own}")
}

/// The parts put together, with nothing between them; or `otherwise`,
/// where that was given and the parts come to nothing but space.
///
/// A part is words, said as they are; or a mapping. A mapping says `say`
/// (words, or a list of parts, each said where its own `if` says) or
/// `from` (a file, with what fills its blanks under `with`), one of the
/// two; with an `if`, that is said where `if` says something, and `else`
/// (words) where it does not. Words that are nothing (a name that led
/// nowhere) say nothing.
pub fn put(parts: &Value, otherwise: Option<&str>, files: &Files) -> Result<Put, String> {
    let mut text = String::new();
    for (place, part) in listed(parts)?.iter().enumerate() {
        let said = part_of(part, files).map_err(|why| format!("part {} of `parts`: {why}", place + 1))?;
        text.push_str(&said);
    }
    let said = !text.trim().is_empty();
    Ok(match otherwise {
        Some(otherwise) if !said => Put { text: otherwise.to_string(), said },
        _ => Put { text, said },
    })
}

/// The files that parts which are said link, by what each part calls
/// them, in the order they are said and each once: what has to be found
/// before the parts are put together. Parts handed to a file's blank are
/// looked through with the part that hands them.
pub fn links(parts: &Value) -> Result<Vec<String>, String> {
    let mut linked: Vec<String> = Vec::new();
    for (place, part) in listed(parts)?.iter().enumerate() {
        links_of(part, &mut linked).map_err(|why| format!("part {} of `parts`: {why}", place + 1))?;
    }
    Ok(linked)
}

fn links_of(part: &Value, linked: &mut Vec<String>) -> Result<(), String> {
    let Value::Object(part) = part else { return Ok(()) };
    let shape = shape(part)?;
    if !said(part)? {
        return Ok(());
    }
    // Parts said in the place of words may link files of their own.
    for (place, part) in shape.say.and_then(Value::as_array).into_iter().flatten().enumerate() {
        links_of(part, linked).map_err(|why| format!("part {} of its `say`: {why}", place + 1))?;
    }
    let Some(from) = shape.from else { return Ok(()) };
    if !linked.iter().any(|known| known == from) {
        linked.push(from.to_string());
    }
    for (name, handed) in shape.with.into_iter().flatten() {
        for (place, part) in handed.as_array().into_iter().flatten().enumerate() {
            links_of(part, linked)
                .map_err(|why| format!("part {} of what is handed as `{name}`: {why}", place + 1))?;
        }
    }
    Ok(())
}

fn listed(parts: &Value) -> Result<&Vec<Value>, String> {
    parts.as_array().ok_or_else(|| {
        format!("`parts` is a list of what is said, in the order it is said; it is {}", shown(parts))
    })
}

/// What a part that is a mapping says, once it is known to be written as
/// one is.
struct Shape<'a> {
    say: Option<&'a Value>,
    from: Option<&'a str>,
    own: Option<&'a str>,
    with: Option<&'a Map<String, Value>>,
    otherwise: Option<&'a Value>,
}

/// A mapping read as a part: `say` or `from`, one of the two; `with`
/// beside `from` alone; `else` beside an `if` alone.
fn shape(part: &Map<String, Value>) -> Result<Shape<'_>, String> {
    if let Some(other) = part.keys().find(|key| !SAYS.contains(&key.as_str())) {
        return Err(format!(
            "it says `{other}`; a part that is not words alone says `say` (words), `from` (a file of \
             `{FOLDER}/`, with what fills its blanks under `with`) or `own` (a file of the project's own \
             `{FOLDER}/`), and where it is only sometimes said, `if` and what is said `else`"
        ));
    }
    let from = match part.get("from") {
        None => None,
        Some(Value::String(from)) => Some(named(from)?),
        Some(other) => {
            return Err(format!("`from` is the name of a file of `{FOLDER}/`; it is {}", shown(other)));
        }
    };
    let own = match part.get("own") {
        None => None,
        // Held to being a name only where it is said: a row that names none
        // says nothing here (`if:` beside it).
        Some(Value::String(own)) => Some(own.as_str()),
        Some(other) => {
            return Err(format!(
                "`own` is the name of a file of the `{FOLDER}/` of the project's own blueprints; it is {}",
                shown(other)
            ));
        }
    };
    let with = match part.get("with") {
        None | Some(Value::Null) => None,
        Some(Value::Object(with)) => Some(with),
        Some(other) => {
            return Err(format!(
                "`with` is what fills the file's blanks, each by its name; it is {}",
                shown(other)
            ));
        }
    };
    let ways = [part.contains_key("say"), from.is_some(), own.is_some()];
    if ways.iter().filter(|way| **way).count() > 1 {
        return Err(
            "it says more than one of `say`, `from` and `own`, and a part is said one way".to_string()
        );
    }
    match (part.contains_key("say"), from.is_some() || own.is_some()) {
        (false, false) => {
            return Err(format!(
                "it says neither `say` (words), `from` (a file of `{FOLDER}/`) nor `own` (a file of the project's \
                 own `{FOLDER}/`)"
            ));
        }
        (true, false) if !part.contains_key("if") => {
            return Err(
                "it says `say` with no `if`: words that are always said are written alone".to_string()
            );
        }
        (true, false) if with.is_some() => {
            return Err("it says `with`, which fills the blanks of a file: words said here have their holes"
                .to_string());
        }
        _ => {}
    }
    if part.contains_key("else") && !part.contains_key("if") {
        return Err("it says `else` with no `if` for it to be the other side of".to_string());
    }
    Ok(Shape { say: part.get("say"), from, own, with, otherwise: part.get("else") })
}

/// Whether a part that is a mapping is said: always, with no `if`.
fn said(part: &Map<String, Value>) -> Result<bool, String> {
    part.get("if").map_or(Ok(true), says)
}

/// The name a part gives the file it links, held to being one: a path
/// within `prompts/`, with no way up out of it, of the kind a prompt is.
fn named(from: &str) -> Result<&str, String> {
    let within = !from.is_empty()
        && !from.starts_with('/')
        && !from.contains('\\')
        && from.split('/').all(|part| !part.is_empty() && part != "." && part != "..");
    match (within, from.ends_with(KIND)) {
        (true, true) => Ok(from),
        (false, _) => Err(format!(
            "`from` names a file within `{FOLDER}/`, by its path from there with no going up out of it; it \
             is `{from}`"
        )),
        (true, false) => Err(format!("`from` names a file that ends `{KIND}`; it is `{from}`")),
    }
}

/// What a text of the library is called that the blueprint called
/// `blueprint` links as `from`: a file of the `prompts/` beside it. A
/// blueprint is beside those of its place (`std/review`, among what
/// ships; `fleet`, at the top of a project).
pub fn linked_as(blueprint: &str, from: &str) -> String {
    match blueprint.rsplit_once('/') {
        Some((place, _)) => format!("{place}/{FOLDER}/{from}"),
        None => format!("{FOLDER}/{from}"),
    }
}

/// The files of the project's own that parts which are said name
/// (`own:`), in the order they are said and each once: found as the
/// project's own texts are, before the parts are put together.
pub fn owned(parts: &Value) -> Result<Vec<String>, String> {
    fn of(part: &Value, owned: &mut Vec<String>) -> Result<(), String> {
        let Value::Object(part) = part else { return Ok(()) };
        let shape = shape(part)?;
        if !said(part)? {
            return Ok(());
        }
        let handed = shape.with.into_iter().flatten().flat_map(|(_, handed)| handed.as_array()).flatten();
        for nested in shape.say.and_then(Value::as_array).into_iter().flatten().chain(handed) {
            of(nested, owned)?;
        }
        if let Some(own) = shape.own.filter(|own| !owned.iter().any(|known| known == own)) {
            named(own).map_err(|why| why.replacen("`from`", "`own`", 1))?;
            owned.push(own.to_string());
        }
        Ok(())
    }
    let mut owned = Vec::new();
    for (place, part) in listed(parts)?.iter().enumerate() {
        of(part, &mut owned).map_err(|why| format!("part {} of `parts`: {why}", place + 1))?;
    }
    Ok(owned)
}

/// What one part says.
fn part_of(part: &Value, files: &Files) -> Result<String, String> {
    let Value::Object(part) = part else { return words(part, "a part") };
    let shape = shape(part)?;
    if !said(part)? {
        return shape.otherwise.map_or(Ok(String::new()), |other| words(other, "`else`"));
    }
    match (shape.say, shape.from, shape.own) {
        (Some(Value::Array(parts)), ..) => together(parts, files, "its `say`"),
        (Some(say), ..) => words(say, "`say`"),
        (None, Some(from), _) => {
            let text = files.get(from).ok_or_else(|| format!("the file `{from}` it links was not found"))?;
            let handed = handed(shape.with, files)?;
            filled(text, Some(&handed)).map_err(|why| format!("the file `{from}`: {why}"))
        }
        (None, None, Some(own)) => {
            let text = files
                .get(&own_key(own))
                .ok_or_else(|| format!("the project's own file `{own}` it names was not found"))?;
            let handed = handed(shape.with, files)?;
            filled(text, Some(&handed)).map_err(|why| format!("the project's own file `{own}`: {why}"))
        }
        (None, None, None) => Ok(String::new()),
    }
}

/// What a part hands the file it links, each thing by its name: words and
/// numbers as they are, and, where what is handed is a list, parts put
/// together as the parts of a text are. That is how a sentence that is
/// only sometimes said has its place in a file that is read whole.
fn handed(with: Option<&Map<String, Value>>, files: &Files) -> Result<Map<String, Value>, String> {
    let mut handed = Map::new();
    for (name, value) in with.into_iter().flatten() {
        let value = match value {
            Value::Array(parts) => {
                Value::String(together(parts, files, &format!("what is handed as `{name}`"))?)
            }
            other => other.clone(),
        };
        handed.insert(name.clone(), value);
    }
    Ok(handed)
}

/// Parts that stand where words would, put together as the parts of a
/// text are: `of` is where they stand, for whoever is told one is wrong.
fn together(parts: &[Value], files: &Files, of: &str) -> Result<String, String> {
    let mut text = String::new();
    for (place, part) in parts.iter().enumerate() {
        let said = part_of(part, files).map_err(|why| format!("part {} of {of}: {why}", place + 1))?;
        text.push_str(&said);
    }
    Ok(text)
}

/// A linked file's text with its blanks filled: each `${name}`, or
/// `${name.part}`, by what `with` hands under that name. The file's one
/// last newline is not said, so that what follows it follows its words.
///
/// A file works nothing out: a hole that is not a path of names is
/// refused, and so is a blank nothing was handed for.
pub fn filled(text: &str, with: Option<&Map<String, Value>>) -> Result<String, String> {
    let text = text.strip_suffix('\n').unwrap_or(text);
    let template = Template::parse(text).map_err(|err| format!("it does not read: {}", err.message))?;
    let written = holes(text).map_err(|err| format!("it does not read: {}", err.message))?;
    let none = Map::new();
    let with = with.unwrap_or(&none);
    // Where in the file each blank is, for whoever is told one is wrong:
    // they are written in order, so each is looked for after the last.
    let mut after = 0;
    for (expr, written) in template.exprs().zip(written) {
        let at = text[after..].find(&format!("${{{written}}}")).map_or(after, |found| after + found);
        let line = text[..at].matches('\n').count() + 1;
        after = at + 1;
        let Some(name) = blank(expr) else {
            return Err(format!(
                "line {line}: its blank `${{{written}}}` works something out; a blank is a name that is handed \
                 under `with` (`${{item}}`, `${{item.title}}`)"
            ));
        };
        if !with.contains_key(name) {
            return Err(format!(
                "line {line}: its blank `${{{}}}` is handed nothing: say `{name}` under `with`",
                written.trim()
            ));
        }
    }
    template.render(with).map(|filled| crate::expr::display(&filled))
}

/// The names a linked file's blanks begin with, each once.
fn blanks(text: &str) -> std::collections::BTreeSet<String> {
    let Ok(template) = Template::parse(text.strip_suffix('\n').unwrap_or(text)) else {
        return Default::default();
    };
    template.exprs().filter_map(blank).map(str::to_string).collect()
}

/// What moved between two texts of a linked file, a sentence for each
/// thing, as [`crate::differs`] says it of a blueprint: that its words are
/// others, which is all there is to say of words, and which blanks it has
/// that it had not, or has no longer, which is what whoever links it has
/// to hand. Nothing where the two are the same words.
pub fn differences(was: &str, now: &str) -> Vec<String> {
    if was == now {
        return Vec::new();
    }
    let (before, after) = (blanks(was), blanks(now));
    let mut said = vec!["its words changed".to_string()];
    said.extend(after.difference(&before).map(|name| format!("the blank `${{{name}}}` is new")));
    said.extend(before.difference(&after).map(|name| format!("the blank `${{{name}}}` is gone")));
    said
}

/// What is wrong with a linked file as a part that hands it `handed`
/// links it, where anything is: a blank that works something out, or one
/// nothing is handed for. Asked when a blueprint is checked, of what its
/// text says it hands, before anything is run.
pub fn unfit(text: &str, handed: &[&str]) -> Option<String> {
    let with: Map<String, Value> = handed.iter().map(|name| (name.to_string(), Value::Null)).collect();
    filled(text, Some(&with)).err()
}

/// A file a step of a blueprint links, as its text says it.
#[derive(Clone, Debug, PartialEq)]
pub struct Link<'a> {
    /// The step that links it, and where that is written.
    pub step: &'a str,
    pub span: crate::Span,
    /// What the part calls the file.
    pub from: &'a str,
    /// What the part hands it, for its blanks: a mapping, or nothing.
    pub with: &'a Value,
}

/// Every file the steps of `blueprint` link, in the order they are
/// written: each part of a `text.put` step that says `from:` in words
/// written out. One whose name is worked out when the step runs is found
/// then, and is not listed here.
pub fn linked(blueprint: &crate::Blueprint) -> Vec<Link<'_>> {
    fn of_part<'a>(step: &'a crate::Step, part: &'a Value, found: &mut Vec<Link<'a>>) {
        // Parts said in the place of words may link files of their own.
        for nested in part["say"].as_array().into_iter().flatten() {
            of_part(step, nested, found);
        }
        let Some(from) = part["from"].as_str().filter(|from| !crate::template::has_holes(from)) else {
            return;
        };
        let with = &part["with"];
        found.push(Link { step: &step.id, span: step.span, from, with });
        // Parts handed to one of its blanks may link files of their own.
        let handed = with.as_object().into_iter().flat_map(|with| with.values());
        for nested in handed.filter_map(Value::as_array).flatten() {
            of_part(step, nested, found);
        }
    }
    fn of<'a>(flow: &'a crate::Flow, found: &mut Vec<Link<'a>>) {
        for step in &flow.steps {
            match &step.kind {
                crate::StepKind::Task => of(&step.inner, found),
                crate::StepKind::Do(action) if action == ACTION => {
                    for part in step.with["parts"].as_array().into_iter().flatten() {
                        of_part(step, part, found);
                    }
                }
                _ => {}
            }
        }
    }
    let mut found = Vec::new();
    for (_, flow) in blueprint.flows() {
        of(flow, &mut found);
    }
    found
}

/// The name a blank begins with, where the blank is a path of names.
fn blank(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Var(name) => Some(name),
        Expr::Member(base, _) => blank(base),
        _ => None,
    }
}

/// Whether what stands under `if` says something: true does and false
/// does not; a text does where it is not empty (as `!= ''` finds it), and
/// a list where it holds anything; nothing does not.
fn says(when: &Value) -> Result<bool, String> {
    match when {
        Value::Bool(said) => Ok(*said),
        Value::Null => Ok(false),
        Value::String(text) => Ok(!text.is_empty()),
        Value::Array(items) => Ok(!items.is_empty()),
        other => Err(format!(
            "`if` is true or false, or a text or a list that may say nothing; it is {}, and what is to be asked \
             of that is a question (`${{ x > 0 }}`)",
            shown(other)
        )),
    }
}

/// Words as they are said: a text, or a number as it is written; nothing
/// says nothing.
fn words(said: &Value, what: &str) -> Result<String, String> {
    match said {
        Value::String(_) | Value::Number(_) | Value::Null => Ok(crate::expr::display(said)),
        other => Err(format!("{what} is words, and this is {}", shown(other))),
    }
}

/// A value as it is said of in a refusal.
fn shown(value: &Value) -> String {
    match value {
        Value::Null => "nothing".to_string(),
        Value::Bool(said) => format!("`{said}`"),
        Value::Number(number) => format!("the number {number}"),
        Value::String(text) => format!("the text `{text}`"),
        Value::Array(_) => "a list".to_string(),
        Value::Object(_) => "a mapping".to_string(),
    }
}
